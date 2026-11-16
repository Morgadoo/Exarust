//! The sector step's first pass: the simulated objects that meet the bodies
//! before any character ticks.
//!
//! # What the original does
//!
//! The grid sector's step opens, before its character walk, with a pass
//! over its simulated-object list in list order. An object takes part when
//! its class carries the pre-pass bit and its state has none of its
//! asleep, disabled and removed bits. For each one the step walks the nine
//! grid cells around the object's own cell (its own first, in the grid's
//! offset order) and, in each, the bodies linked there in the cell's order.
//! A body is handed to the object's body-contact slot when its bound sphere
//! is not apart from the object's (the sign of `(r + r')² − d²`, a NaN
//! passing), the object's centre lies within the object's radius of the
//! body's mesh box (`r² > d²`, a NaN passing) and the body is not disabled.
//!
//! Two classes carry the bit, and only once thrown: the force blast (set at
//! its release) and the force wave (set at its throw, which shares the
//! blast's class and slots). Their slot is the blast's part contact: the
//! caster's own parts are skipped, the chunks are taken into the part's
//! frame and pushed out of its node mesh, and a struck character takes the
//! knock ([`super::blast_live::strike_part`], `knock_character`). Neither
//! class has the solver's body-contact bit, so this pass is their only
//! meeting with the bodies; their level and cloth contacts stay in their
//! own tick's static and object stages.
//!
//! The pass therefore meets the bodies where the last tick's solver and
//! post-step left them, with the chunks where the object's last step left
//! them, a tick after that step.
//!
//! # Exarust
//!
//! [`meet_bodies_first`] runs in [`exa_physics::body::BodyPreStep`], before
//! the character ticks. The object list is the thrown blasts and waves in
//! entity order (the cloth stage's list order, oldest first). The cells
//! are the cloth stage's: the scene grid's flat cells when the scene has
//! one, the stand-in cells otherwise ([`ClothGrid`]), and the body parts
//! linked in them where their mesh box's world centre falls, each cell's
//! parts in the bodies' walk order ([`PartCells`]). The body gates are the
//! sector's ([`sector::object_meets_part`]) and the disabled bit. The pass
//! keeps its own index and builds it only while an object is thrown.
//!
//! Evidence: the sector step's first pass, the blast's and wave's release
//! (the class bit) and their shared body-contact slot, read from
//! `Exanima.exe`.

use bevy::prelude::*;
use exa_ai::brain::Npc;
use exa_ai::combatant::Combatant;
use exa_physics::body::Ragdoll;
use exa_physics::sector::{self, ObjectBodyPart, ObjectBounds};
use exa_world::cloth_parts::{ClothGrid, LinkedPart, PartCells};

use super::blast_live::{BODY_WAKE, ChunkSet, LiveBlast, body_mass, knock_character, strike_part};
use super::wave_live::{LiveWave, take_back, wave_chunks};
use crate::effects::blast::{BlastChunk, THROWN};
use crate::effects::force_wave::WaveState;

/// The pass's lists, kept from tick to tick.
#[derive(Default)]
pub(crate) struct PrePass {
    /// The body parts' cells (built only while an object takes part).
    index: PartCells,
    /// One cell's parts.
    linked: Vec<LinkedPart>,
    /// The objects of this tick, in list order: the entity and whether it
    /// is a wave.
    objects: Vec<(Entity, bool)>,
    /// A wave's chunks as the blast's, and every object's live flags.
    chunks: Vec<BlastChunk>,
    alive: Vec<bool>,
}

/// What one object's walk did: the parts handed to its slot and the parts
/// it struck.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Walked {
    pub offered: u32,
    pub struck: u32,
}

/// The sector step's first pass (see the module).
#[allow(clippy::too_many_arguments)]
pub(crate) fn meet_bodies_first(
    mut blasts: Query<(Entity, &mut LiveBlast)>,
    mut waves: Query<(Entity, &mut LiveWave)>,
    mut bodies: ParamSet<(Query<(Entity, Ref<Ragdoll>)>, Query<(Entity, &mut Ragdoll)>)>,
    scene: Option<Res<exa_physics::SceneGrid>>,
    mut combatants: Query<&mut Combatant>,
    npcs: Query<&Npc>,
    mut pass: Local<PrePass>,
) {
    let pass = &mut *pass;
    pass.objects.clear();
    pass.objects.extend(
        blasts
            .iter()
            .filter(|(_, b)| b.blast.state == THROWN)
            .map(|(e, _)| (e, false)),
    );
    pass.objects.extend(
        waves
            .iter()
            .filter(|(_, w)| w.wave.state == WaveState::Thrown)
            .map(|(e, _)| (e, true)),
    );
    if pass.objects.is_empty() {
        pass.index.clear();
        return;
    }
    pass.objects.sort_unstable();
    let grid = ClothGrid::of_scene(scene.as_deref().copied());
    pass.index.refresh(
        bodies
            .p0()
            .iter()
            .map(|(entity, body)| (entity, body.is_changed(), body.into_inner())),
        grid,
    );
    let mut bodies = bodies.p1();
    let objects = std::mem::take(&mut pass.objects);
    for &(entity, wave) in &objects {
        if wave {
            let Ok((_, mut live)) = waves.get_mut(entity) else {
                continue;
            };
            let live = &mut *live;
            let mut chunks = std::mem::take(&mut pass.chunks);
            let mut alive = std::mem::take(&mut pass.alive);
            wave_chunks(live, &mut chunks, &mut alive);
            let mut set = ChunkSet {
                chunks: &mut chunks,
                alive: &alive,
                setup: live.setup,
                centre: live.wave.centre,
                radius: live.wave.bound_radius,
            };
            let walked = walk_cells(
                pass,
                grid,
                &mut set,
                live.caster,
                &mut bodies,
                &mut combatants,
                &npcs,
                &mut |_| {},
            );
            if 0 < walked.struck {
                debug!(
                    "wave {entity:?} meets the bodies first: {} of {} parts struck",
                    walked.struck, walked.offered
                );
            }
            take_back(live, &chunks);
            pass.chunks = chunks;
            pass.alive = alive;
        } else {
            let Ok((_, mut live)) = blasts.get_mut(entity) else {
                continue;
            };
            let live = &mut *live;
            let caster = live.caster;
            let mut alive = std::mem::take(&mut pass.alive);
            let mut scale = live.contact_scale;
            let mut set = ChunkSet::of_blast(&mut live.blast, &mut alive);
            let walked = walk_cells(
                pass,
                grid,
                &mut set,
                caster,
                &mut bodies,
                &mut combatants,
                &npcs,
                &mut |s| scale = s,
            );
            live.contact_scale = scale;
            if 0 < walked.struck {
                debug!(
                    "blast {entity:?} meets the bodies first: {} of {} parts struck",
                    walked.struck, walked.offered
                );
            }
            pass.alive = alive;
        }
    }
    pass.objects = objects;
}

/// One object's walk of the nine cells around its own: every linked part
/// that passes the sector's gates is handed to the blast's part contact,
/// the caster's own skipped; `scaled` gets each contact's reach scale.
#[allow(clippy::too_many_arguments)]
fn walk_cells(
    pass: &mut PrePass,
    grid: ClothGrid,
    set: &mut ChunkSet<'_>,
    caster: Entity,
    bodies: &mut Query<(Entity, &mut Ragdoll)>,
    combatants: &mut Query<&mut Combatant>,
    npcs: &Query<&Npc>,
    scaled: &mut dyn FnMut(f32),
) -> Walked {
    let object = ObjectBounds {
        centre: set.centre,
        radius: set.radius,
    };
    let mut walked = Walked::default();
    let mut linked = std::mem::take(&mut pass.linked);
    for cell in grid.around(grid.key_of(object.centre)) {
        pass.index.sorted_in(cell, &mut linked);
        for at in &linked {
            let Ok((other, mut body)) = bodies.get_mut(at.body) else {
                continue;
            };
            if !offered(object, &body, at.part) {
                continue;
            }
            walked.offered += 1;
            // The slot's own skip: the caster's parts.
            if other == caster {
                continue;
            }
            let character = body.drive.is_some();
            let mass = body_mass(&body);
            // Only a struck part changes the body.
            let strike = strike_part(
                set,
                body.bypass_change_detection(),
                at.part,
                character,
                mass,
            );
            scaled(strike.scale);
            if !strike.struck {
                continue;
            }
            walked.struck += 1;
            body.set_changed();
            debug!(
                "pre-pass strikes {other:?} part {}: impulse {:.3?} cm",
                at.part, strike.impulse
            );
            if character {
                knock_character(
                    &mut body,
                    mass,
                    strike.impulse,
                    caster,
                    other,
                    combatants,
                    npcs,
                );
            }
            body.touch(BODY_WAKE);
        }
    }
    pass.linked = linked;
    walked
}

/// Whether the sector hands the object a body part: its bound sphere not
/// apart from the object's, the object's centre within its radius of the
/// part's mesh box, and the body not disabled.
fn offered(object: ObjectBounds, body: &Ragdoll, part: usize) -> bool {
    let Some(p) = body.parts.get(part) else {
        return false;
    };
    let now = p.transform();
    let meeting = ObjectBodyPart {
        now,
        previous: now,
        spheres: &p.spheres,
        centre: now.transform_point3(p.pairing.sphere.centre),
        radius: p.pairing.sphere.radius,
        mesh: &p.pairing.mesh,
        countdown: body.countdown(),
        points: p.positions,
        previous_points: p.previous,
    };
    sector::object_meets_part(object, &meeting) && !body.disabled()
}

#[cfg(test)]
#[path = "pre_pass_test.rs"]
mod tests;
