//! The thaumaturgy effects in the running game: arcs, barricades and
//! displacement wisps as entities whose simulations ([`super::arcs`],
//! [`super::barricade`], [`super::wisps`]) run at the body tick against the
//! physical bodies and the level, and whose meshes are rebuilt each frame.
//!
//! - **Each tick** (after the bodies): an arc follows its hand, ticks,
//!   ends at the level (the static colliders) and tests every other body's
//!   parts, sending [`ArcShocked`] for a struck character. A shock reaches
//!   a fighter once the game clock is within 150 ms of its next shock time
//!   ([`ShockReady`]), which then moves to 500 ms plus a draw below 200 ms
//!   of the engine's generator ahead; a fighter that is not collapsing or
//!   dead loses the strength in yellow health (floored at 0) and takes the
//!   shock-stagger flag, which the next body tick spends
//!   ([`exa_physics::shock::shock_stagger_tick`]). A barricade
//!   ticks on the game clock, ends once its caster is 20 m away, and
//!   shoves every body part it holds: each sphere's push moves the part's
//!   particles ([`exa_physics::body::Ragdoll::push_sphere`]), the body
//!   wakes for 32 ticks, and each part of a character struck in front
//!   knocks it back a step (doubled, at most its step length), sets its
//!   instability counter to 100 and, while its energy is above 0.25,
//!   lowers it, so a body struck by several parts loses that much for
//!   each. While it stands its line is stamped on the navigation grid's
//!   penalty (`exa_world::nav::stamp_line`, from the first tick a grid is
//!   there) and taken back when its fall ends. Wisps follow their body
//!   parts.
//! - **Each frame**: the arcs' and wisps' ribbons and the barricade's slab
//!   are drawn with [`exa_render::glow::GlowMaterial`], a stand-in for the
//!   original's effect programs, which are not ported (the arcs' ribbon
//!   carries its texture coordinates). An effect that ends leaves the
//!   scene.
//! - **Debug** ([`ThaumDebug`], `--debug-arcs`, `--debug-barricade`,
//!   `--debug-wisps`): the Shock power's hand arcs on the player's right
//!   hand (respawned while they last), a barricade raised 2 m in front of
//!   the player, and the constant displacement's wisps on the player's
//!   body. The player cannot cast yet: Exarust has no power bar.
//!
//! Not wired: the hum and impact sounds, the lights, the AI's sight of a
//! barricade, the attack a barricade interrupts.
//! Rules: `docs/THAUMATURGY.md`.

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use exa_combat::swing::SwingState;
use exa_combat::{Condition, Fighter, StateFlags};
use exa_physics::Physics;
use exa_physics::body::locomotion::Locomotor;
use exa_physics::body::{BodySystems, BodyTicks, Ragdoll, step_length};
use exa_render::glow::GlowMaterial;
use exa_world::nav::{NavGrid, NavPatch, stamp_line, unstamp_line};

use super::arcs::{ARC_COLOUR, ArcEmitter, ArcLimb, HAND_ARCS_OFFSET, hand_arcs};
use super::barricade::{
    Barricade, GridFrame, WAKE_TICKS, WallBody, WallBodyKind, WallFrame, barricade_mesh, nav_stamp,
    power_ends, slide_mesh,
};
use super::displacement::body_wisps;
use super::engine::{Roll, normalize};
use super::force::{GroundHit, place_barricade};
use super::wisps::{Strand, Wisps};
use crate::arena::Mt19937;
use crate::player::PlayerBody;

/// Centimetres a metre: the original's effects run in cm.
const CM: f32 = 100.0;
/// A shock reaches a fighter this many ms before its next shock time.
pub const SHOCK_EARLY_MS: u64 = 150;
/// After a shock the next one is due this many ms later ...
pub const SHOCK_SPACING_MS: u64 = 500;
/// ... plus a draw below this many ms of the engine's generator.
pub const SHOCK_SPREAD_MS: u64 = 200;
/// The chest's bone and the right hand's.
const CHEST: usize = 9;
const RIGHT_HAND: usize = 17;

pub struct ThaumEffectsPlugin;

impl Plugin for ThaumEffectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ArcShocked>()
            .insert_resource(EffectDraws(Mt19937::new(5489)))
            .init_resource::<EffectRoll>()
            .init_resource::<super::effect_draw::ForceNoise>()
            .add_systems(
                FixedUpdate,
                (tick_arcs, tick_barricades, tick_wisps, shock_fighters)
                    .chain()
                    .after(BodySystems),
            )
            .add_systems(
                Update,
                spawn_debug_effects.run_if(resource_exists::<ThaumDebug>),
            )
            .add_systems(
                PostUpdate,
                (
                    draw_arcs,
                    draw_barricades,
                    draw_wisps,
                    (
                        super::effect_draw::tick_force_noise
                            .run_if(super::effect_draw::force_effects_alive),
                        (
                            super::blast_live::draw_blasts,
                            super::torrent_live::draw_torrents,
                            super::effect_draw::draw_shell_auras,
                            super::effect_draw::drop_shell_copies,
                        ),
                    )
                        .chain(),
                )
                    .after(TransformSystems::Propagate),
            );
    }
}

/// The runtime's random stream the effects draw from.
#[derive(Resource)]
pub struct EffectDraws(pub Mt19937);

impl Default for EffectDraws {
    /// The stream the effects start from.
    fn default() -> Self {
        Self(Mt19937::new(5489))
    }
}

/// The engine's own generator, for the effects that use it.
#[derive(Resource, Default)]
pub struct EffectRoll(pub Roll);

/// Which test effects to put on the player (the debug flags).
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ThaumDebug {
    pub arcs: bool,
    pub barricade: bool,
    pub wisps: bool,
}

/// A character an arc struck.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct ArcShocked {
    pub body: Entity,
    pub direction: Vec3,
    pub strength: f32,
}

/// When a fighter's next shock is due (ms of game time); a fighter without
/// one has never been shocked.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShockReady(pub u64);

/// The shock reaction's clock and health rule, as the original's character
/// takes a shock: `None` while the shock comes too early, else the next
/// shock time and the yellow health left (`dying` keeps it). A released
/// race (`released`, and not dying) is not burnt: its next shock is 500 ms
/// on with no draw and its health is kept.
pub fn shock_reaction(
    ready: Option<u64>,
    now: u64,
    yellow: f32,
    strength: f32,
    dying: bool,
    released: bool,
    roll: &mut Roll,
) -> Option<(u64, f32)> {
    if ready.is_some_and(|next| next > now + SHOCK_EARLY_MS) {
        return None;
    }
    if released && !dying {
        return Some((now + SHOCK_SPACING_MS, yellow));
    }
    let spread = (u64::from(roll.step()) * SHOCK_SPREAD_MS) >> 32;
    let next = now + SHOCK_SPACING_MS + spread;
    let left = if dying {
        yellow
    } else {
        let left = yellow - strength;
        if left < 0.0 { 0.0 } else { left }
    };
    Some((next, left))
}

/// Arcs in the scene: the body and bone whose part they ride (its place is
/// their frame's anchor, the hand arcs' node offset turned with it), and
/// the body that casts them.
#[derive(Component)]
pub struct LiveArcs {
    pub arcs: ArcEmitter,
    pub hand: Option<(Entity, usize)>,
    pub caster: Option<Entity>,
    /// Kept in the scene when its tendrils are gone (the test arcs).
    pub keep: bool,
}

/// A barricade in the scene and who cast it.
#[derive(Component)]
pub struct LiveBarricade {
    pub wall: Barricade,
    pub caster: Option<Entity>,
}

/// Wisps on a body: each strand rides the part of the body at its slot.
#[derive(Component)]
pub struct LiveWisps {
    pub wisps: Wisps,
    pub body: Entity,
    pub parts: Vec<usize>,
}

/// The mesh an effect draws.
#[derive(Component)]
pub struct EffectMesh(pub Handle<Mesh>);

/// The patch a barricade's line stamped on the navigation grid, taken back
/// when its fall ends.
#[derive(Component)]
pub struct NavStamped(pub NavPatch);

/// The world place (cm) of a point given in a part's space (cm).
fn part_point(body: &Ragdoll, part: usize, local_cm: Vec3) -> Option<Vec3> {
    let p = body.parts.get(part)?;
    Some(p.transform().transform_point3(local_cm / CM) * CM)
}

fn bone_part(body: &Ragdoll, bone: usize) -> Option<usize> {
    body.parts.iter().position(|p| p.bone == bone)
}

fn tick_arcs(
    mut arcs: Query<&mut LiveArcs>,
    bodies: Query<(Entity, &Ragdoll)>,
    physics: Option<Res<Physics>>,
    mut draws: ResMut<EffectDraws>,
    mut shocked: MessageWriter<ArcShocked>,
) {
    for mut live in &mut arcs {
        let live = &mut *live;
        if let Some((body, bone)) = live.hand {
            let anchor = bodies
                .get(body)
                .ok()
                .and_then(|(_, b)| part_point(b, bone_part(b, bone)?, HAND_ARCS_OFFSET));
            live.arcs.anchor = anchor.or(live.arcs.anchor);
        }
        live.arcs.tick(&mut draws.0);
        if let Some(physics) = physics.as_deref() {
            live.arcs.clip_to_level(|start, dir, len| {
                physics
                    .fixed_hit(start / CM, dir, len / CM)
                    .map(|hit| hit * CM)
            });
        }
        for (entity, body) in &bodies {
            if Some(entity) == live.caster || body.drive.is_none() {
                continue;
            }
            let chest = bone_part(body, CHEST)
                .and_then(|k| part_point(body, k, Vec3::ZERO))
                .unwrap_or(Vec3::ZERO);
            for part in &body.parts {
                let world = part.transform();
                let spheres: Vec<(Vec3, f32)> = part
                    .spheres
                    .iter()
                    .map(|&(c, r)| (world.transform_point3(c) * CM, r * CM))
                    .collect();
                let limb = ArcLimb {
                    character: entity.to_bits(),
                    caster: false,
                    at: Vec3::from(world.translation) * CM,
                    chest,
                };
                let hit = live.arcs.strike_limb(
                    &limb,
                    |probe| {
                        spheres
                            .iter()
                            .any(|&(c, r)| segment_meets(probe.start, probe.end, c, r))
                    },
                    &mut draws.0,
                );
                if let Some(shock) = hit {
                    shocked.write(ArcShocked {
                        body: entity,
                        direction: shock.direction,
                        strength: shock.strength,
                    });
                }
            }
        }
    }
}

/// Whether the segment from `a` to `b` comes within `r` of `c`.
fn segment_meets(a: Vec3, b: Vec3, c: Vec3, r: f32) -> bool {
    let d = b - a;
    let len2 = d.length_squared();
    let t = if len2 > 0.0 {
        ((c - a).dot(d) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (a + d * t).distance_squared(c) <= r * r
}

#[allow(clippy::type_complexity)]
fn shock_fighters(
    mut shocks: MessageReader<ArcShocked>,
    mut fighters: Query<(
        &mut Fighter,
        Option<&mut ShockReady>,
        Option<&mut Condition>,
        Option<&mut StateFlags>,
        Option<&mut Ragdoll>,
    )>,
    ticks: Option<Res<BodyTicks>>,
    mut roll: ResMut<EffectRoll>,
    mut commands: Commands,
) {
    let now = ticks.map_or(0, |t| t.millis());
    // Next shock times set this tick on fighters that had none yet (their
    // component arrives with the commands).
    let mut set: Vec<(Entity, u64)> = Vec::new();
    for shock in shocks.read() {
        let Ok((mut fighter, ready, mut condition, flags, body)) = fighters.get_mut(shock.body)
        else {
            continue;
        };
        let dying = condition.as_deref().is_some_and(|c| c.dying());
        let known = ready.as_deref().map(|r| r.0).or_else(|| {
            set.iter()
                .rev()
                .find(|(e, _)| *e == shock.body)
                .map(|&(_, next)| next)
        });
        let Some((next, yellow)) = shock_reaction(
            known,
            now,
            fighter.health.yellow,
            shock.strength,
            dying,
            fighter.stats.shocks_release,
            &mut roll.0,
        ) else {
            continue;
        };
        if fighter.stats.shocks_release && !dying {
            // A release race is not burnt: it is put into the shocked
            // state, its step ends and its clip rate stops.
            if let Some(mut body) = body {
                body.force_release(now as u32);
            }
            if let Some(condition) = condition.as_deref_mut() {
                *condition = Condition::SHOCKED;
            }
        } else {
            fighter.health.yellow = yellow;
            if !dying && let Some(mut flags) = flags {
                flags.0 |= StateFlags::SHOCK_STAGGER;
            }
        }
        match ready {
            Some(mut ready) => ready.0 = next,
            None => {
                set.push((shock.body, next));
                commands.entity(shock.body).insert(ShockReady(next));
            }
        }
    }
}

/// A character's energy after the knocks of the parts a barricade struck in
/// front this tick: each lowers it by its loss while it is above 0.25.
pub fn energy_after_knocks(energy: f32, losses: &[f32]) -> f32 {
    losses
        .iter()
        .fold(energy, |e, &loss| if 0.25 < e { e - loss } else { e })
}

#[allow(clippy::too_many_arguments)]
fn tick_barricades(
    mut commands: Commands,
    mut walls: Query<(Entity, &mut LiveBarricade, Option<&NavStamped>)>,
    mut bodies: Query<(&mut Ragdoll, Option<&mut SwingState>)>,
    ticks: Option<Res<BodyTicks>>,
    mut roll: ResMut<EffectRoll>,
    mut nav: Option<ResMut<NavGrid>>,
) {
    let now = ticks.map_or(0, |t| t.millis()) as u32;
    for (wall_entity, mut live, stamped) in &mut walls {
        let live = &mut *live;
        if let Some(caster) = live.caster
            && let Ok(body) = bodies.get(caster)
            && let Some(at) = body.0.parts.first().map(|p| p.transform().translation * CM)
            && power_ends(0, live.wall.centre, at.into())
        {
            live.wall.end();
        }
        if live.wall.tick(now) {
            debug!("barricade {wall_entity}: gone");
        }
        for (mut body, mut swing) in &mut bodies {
            let character = body.drive.as_ref().map(|d| {
                let chest = bone_part(&body, CHEST)
                    .and_then(|k| part_point(&body, k, Vec3::ZERO))
                    .unwrap_or(Vec3::ZERO);
                let step = step_length(d.stats.scale, d.balance().extension());
                (chest, d.stats.scale, step)
            });
            let mut touched = false;
            // Each part struck in front knocks the character back and
            // lowers its energy again.
            let mut knocks: Vec<(Vec3, f32)> = Vec::new();
            for k in 0..body.parts.len() {
                let part = &body.parts[k];
                let world = part.transform();
                let spheres: Vec<(Vec3, f32)> = part
                    .spheres
                    .iter()
                    .map(|&(c, r)| (world.transform_point3(c) * CM, r * CM))
                    .collect();
                let reach = spheres
                    .iter()
                    .map(|&(c, r)| c.distance(Vec3::from(world.translation) * CM) + r)
                    .fold(0.0, f32::max);
                let tested = WallBody {
                    centre: Vec3::from(world.translation) * CM,
                    radius: reach,
                    mobility: part.inverse_mass,
                    step: character.map_or(80.0, |c| c.2),
                    spheres,
                    kind: match character {
                        Some((chest, scale, _)) => WallBodyKind::Limb { chest, scale },
                        None => WallBodyKind::Other,
                    },
                };
                let shove = live.wall.shove(&tested, now, &mut roll.0);
                // The original's Force walls interrupt the holder's attack
                // when they strike a weapon part; the wall does not
                // classify a held item's body yet, so this stays
                // unexercised.
                if shove.interrupt
                    && let Some(state) = swing.as_mut()
                {
                    state.force_interrupt(u64::from(now));
                }
                touched |= shove.touched;
                for (sphere, push) in &shove.pushes {
                    body.push_sphere(k, *sphere, *push / CM);
                }
                if let Some(step) = shove.knockback {
                    knocks.push((step, shove.energy_loss));
                }
            }
            if touched {
                body.touch(WAKE_TICKS);
            }
            if let Some(&(step, _)) = knocks.last() {
                // The original's knockback sets the step's target and does
                // not wake the body, so no blow is queued; every part's
                // knockback aims at the same target.
                body.begin_knockback(step / CM);
                if let Some(drive) = body.drive.as_mut() {
                    drive.balance_mut().set_counter(100);
                    let losses: Vec<f32> = knocks.iter().map(|k| k.1).collect();
                    drive.energy = energy_after_knocks(drive.energy, &losses);
                }
            }
        }
        if live.wall.state == super::barricade::state::GONE {
            // The finished fall takes the stamp back off the grid.
            if let (Some(NavStamped(patch)), Some(grid)) = (stamped, nav.as_mut()) {
                unstamp_line(grid, patch);
            }
            commands.entity(wall_entity).despawn();
        } else if stamped.is_none()
            && let Some(grid) = nav.as_mut()
        {
            // The original stamps the wall's line as it enters the world; the
            // running game does so on the first tick a grid is there.
            let (width, depth) = grid.size();
            let frame = GridFrame {
                width: width as u32,
                height: depth as u32,
                scale: 1.0 / 30.0,
            };
            let (a, b) = live.wall.ends();
            let line = nav_stamp(&frame, a, b);
            let size = (line.size.0.max(0) as u32, line.size.1.max(0) as u32);
            let patch = stamp_line(grid, line.centre, size, &line.points);
            live.wall.stamp = Some(line);
            commands.entity(wall_entity).insert(NavStamped(patch));
        }
    }
}

fn tick_wisps(
    mut commands: Commands,
    mut sets: Query<(Entity, &mut LiveWisps)>,
    bodies: Query<&Ragdoll>,
    mut roll: ResMut<EffectRoll>,
) {
    for (entity, mut live) in &mut sets {
        let live = &mut *live;
        let Ok(body) = bodies.get(live.body) else {
            commands.entity(entity).despawn();
            continue;
        };
        let parts = &live.parts;
        let world = |s: &Strand| {
            s.body
                .and_then(|slot| parts.get(slot))
                .and_then(|&k| part_point(body, k, s.local))
                .unwrap_or(s.local)
        };
        live.wisps.tick(world, &mut roll.0);
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_debug_effects(
    mut commands: Commands,
    debug: Res<ThaumDebug>,
    players: Query<(Entity, &Ragdoll, &Locomotor), With<PlayerBody>>,
    physics: Option<Res<Physics>>,
    ticks: Option<Res<BodyTicks>>,
    mut draws: ResMut<EffectDraws>,
    mut roll: ResMut<EffectRoll>,
    mut done: Local<ThaumDebug>,
) {
    let Some((player, body, facing)) = players.iter().next() else {
        return;
    };
    if body.parts.is_empty() {
        return;
    }
    if debug.arcs && !done.arcs {
        done.arcs = true;
        let mut arcs = ArcEmitter::new(&hand_arcs(), &mut draws.0);
        arcs.anchor =
            bone_part(body, RIGHT_HAND).and_then(|k| part_point(body, k, HAND_ARCS_OFFSET));
        commands.spawn((
            Name::new("_DebugArcs"),
            LiveArcs {
                arcs,
                hand: Some((player, RIGHT_HAND)),
                caster: Some(player),
                keep: true,
            },
        ));
        info!("debug arcs on the player's right hand");
    }
    if debug.barricade && !done.barricade {
        done.barricade = true;
        let root = body.parts[0].transform();
        let at = Vec3::from(root.translation) * CM;
        // The model faces +Z at yaw 0.
        let forward = normalize(Vec3::new(facing.yaw.sin(), 0.0, facing.yaw.cos()));
        let feet = at + forward * 200.0;
        let hit = GroundHit {
            point: Vec3::new(feet.x, feet.y - 80.0, feet.z),
            normal: Vec3::Y,
            exact: true,
        };
        let floor = |from: Vec3, reach: f32| {
            physics
                .as_deref()
                .and_then(|p| p.fixed_hit(from / CM, Vec3::NEG_Y, reach / CM))
                .map(|d| d * CM)
        };
        let place = place_barricade(hit, at, floor, roll.0.unit());
        let frame = WallFrame::facing(place.centre, place.facing);
        let now = ticks.as_deref().map_or(0, |t| t.millis()) as u32;
        let trim = |start: Vec3, dir: Vec3, len: f32| {
            physics
                .as_deref()
                .and_then(|p| p.fixed_hit(start / CM, dir, len / CM))
                .map(|d| d * CM)
        };
        let wall = Barricade::raise(frame, 0.25, now, trim, None, &mut draws.0);
        commands.spawn((
            Name::new("_DebugBarricade"),
            LiveBarricade {
                wall,
                caster: Some(player),
            },
        ));
        info!("debug barricade at {:?}", place.centre);
    }
    if debug.wisps && !done.wisps {
        done.wisps = true;
        let bones: Vec<usize> = body.parts.iter().map(|p| p.bone).collect();
        let mut parts = Vec::new();
        let mut strands = Vec::new();
        for (bone, anchor) in body_wisps(&bones) {
            if let Some(k) = bone_part(body, bone) {
                strands.push(Strand::new(anchor, Some(parts.len())));
                parts.push(k);
            }
        }
        let world = |s: &Strand| {
            s.body
                .and_then(|slot| parts.get(slot))
                .and_then(|&k| part_point(body, k, s.local))
                .unwrap_or(s.local)
        };
        let mut wisps = Wisps::new(strands, 12.0, world, || draws.0.unit() as f32);
        wisps.place(12, world, &mut roll.0);
        commands.spawn((
            Name::new("_DebugWisps"),
            LiveWisps {
                wisps,
                body: player,
                parts,
            },
        ));
        info!("debug wisps on the player's body");
    }
}

/// The glow tint each effect draws with.
const ARC_TINT: Vec4 = Vec4::new(1.0, 1.0, 1.0, 1.0);
const WALL_TINT: Vec4 = Vec4::new(0.45, 0.33, 0.95, 0.25);
const WISP_TINT: Vec4 = Vec4::new(1.0, 1.0, 1.0, 0.6);

fn mesh_of(positions: Vec<[f32; 3]>, colours: Vec<[f32; 4]>, indices: Vec<u32>) -> Mesh {
    let uvs = vec![[0.0, 0.0]; positions.len()];
    mesh_of_uv(positions, uvs, colours, indices)
}

/// A mesh with its texture coordinates (the arcs' ribbon scrolls them).
pub(super) fn mesh_of_uv(
    positions: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    colours: Vec<[f32; 4]>,
    indices: Vec<u32>,
) -> Mesh {
    let normals = vec![[0.0, 1.0, 0.0]; positions.len()];
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colours);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// A force ribbon (cm, colour bytes) as a Bevy mesh (m, linear colours).
pub(super) fn mesh_of_ribbon(ribbon: &crate::effects::ribbon::RibbonMesh) -> Mesh {
    let positions = ribbon
        .positions
        .iter()
        .map(|p| (*p / CM).to_array())
        .collect();
    let uvs = ribbon.uvs.clone();
    let colours = ribbon
        .colours
        .iter()
        .map(|c| {
            [
                f32::from(c[0]) / 255.0,
                f32::from(c[1]) / 255.0,
                f32::from(c[2]) / 255.0,
                f32::from(c[3]) / 255.0,
            ]
        })
        .collect();
    let indices = ribbon.indices.iter().map(|&i| u32::from(i)).collect();
    let mut mesh = mesh_of_uv(positions, uvs, colours, indices);
    // The second coordinate set (`vTx2`): the effect's texture row.
    if ribbon.uvs2.len() == ribbon.positions.len() {
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, ribbon.uvs2.clone());
    }
    mesh
}

#[allow(clippy::too_many_arguments)]
pub(super) fn show(
    commands: &mut Commands,
    entity: Entity,
    drawn: Option<&EffectMesh>,
    mesh: Mesh,
    tint: Vec4,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<GlowMaterial>,
) {
    match drawn {
        Some(EffectMesh(handle)) => {
            if let Some(mut m) = meshes.get_mut(handle) {
                *m = mesh;
            }
        }
        None => {
            let handle = meshes.add(mesh);
            commands.entity(entity).insert((
                EffectMesh(handle.clone()),
                Mesh3d(handle),
                MeshMaterial3d(materials.add(GlowMaterial { tint })),
                Transform::IDENTITY,
                NoFrustumCulling,
                bevy::light::NotShadowCaster,
                bevy::light::NotShadowReceiver,
            ));
        }
    }
}

fn camera_axis(cameras: &Query<&GlobalTransform, exa_render::supersample::SceneCamera>) -> Vec3 {
    cameras
        .iter()
        .next()
        .map_or(Vec3::NEG_Z, |c| c.forward().as_vec3())
}

#[allow(clippy::too_many_arguments)]
fn draw_arcs(
    mut commands: Commands,
    mut arcs: Query<(Entity, &mut LiveArcs, Option<&EffectMesh>)>,
    cameras: Query<&GlobalTransform, exa_render::supersample::SceneCamera>,
    time: Res<Time<Fixed>>,
    mut draws: ResMut<EffectDraws>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GlowMaterial>>,
) {
    let eye = camera_axis(&cameras);
    let alpha = time.overstep_fraction();
    for (entity, mut live, drawn) in &mut arcs {
        let frame = live.arcs.frame(alpha, eye, 0.0, |p| p, &mut draws.0);
        if frame.removed && !live.keep {
            commands.entity(entity).despawn();
            continue;
        }
        let r = &frame.ribbon;
        let positions = r.positions.iter().map(|p| (*p / CM).to_array()).collect();
        let uvs = r.uvs.to_vec();
        let [red, green, blue] = ARC_COLOUR.map(|c| f32::from(c) / 255.0);
        let colours = r
            .alphas
            .iter()
            .map(|&a| [red, green, blue, f32::from(a) / 255.0])
            .collect();
        let indices = r.drawn().iter().map(|&i| u32::from(i)).collect();
        let mesh = mesh_of_uv(positions, uvs, colours, indices);
        show(
            &mut commands,
            entity,
            drawn,
            mesh,
            ARC_TINT,
            &mut meshes,
            &mut materials,
        );
    }
}

fn draw_barricades(
    mut commands: Commands,
    mut walls: Query<(Entity, &mut LiveBarricade, Option<&EffectMesh>)>,
    time: Res<Time<Fixed>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GlowMaterial>>,
    mut slab: Local<Option<super::barricade::WallMesh>>,
) {
    let alpha = time.overstep_fraction();
    let slab = slab.get_or_insert_with(barricade_mesh);
    for (entity, mut live, drawn) in &mut walls {
        let Some(out) = live.wall.frame(alpha) else {
            continue;
        };
        let at = if out.mode == 1 {
            out.at
        } else {
            live.wall.centre
        };
        let frame = WallFrame {
            at,
            ..live.wall.frame
        };
        let (local, _) = slide_mesh(slab, out.slide);
        let height = (out.half_height / super::barricade::HALF_HEIGHT).max(0.0);
        let positions = local
            .iter()
            .map(|p| (frame.world(Vec3::new(p.x, p.y * height, p.z)) / CM).to_array())
            .collect();
        let colours = vec![[1.0; 4]; local.len()];
        let indices = slab
            .triangles
            .iter()
            .flatten()
            .map(|&i| u32::from(i))
            .collect();
        let mesh = mesh_of(positions, colours, indices);
        show(
            &mut commands,
            entity,
            drawn,
            mesh,
            WALL_TINT,
            &mut meshes,
            &mut materials,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_wisps(
    mut commands: Commands,
    mut sets: Query<(Entity, &mut LiveWisps, Option<&EffectMesh>)>,
    cameras: Query<&GlobalTransform, exa_render::supersample::SceneCamera>,
    time: Res<Time<Fixed>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GlowMaterial>>,
) {
    let eye = camera_axis(&cameras);
    let alpha = time.overstep_fraction();
    for (entity, mut live, drawn) in &mut sets {
        let r = live.wisps.ribbon(alpha, eye);
        if r.removed {
            commands.entity(entity).despawn();
            continue;
        }
        let positions = r.positions.iter().map(|p| (*p / CM).to_array()).collect();
        let colours = r
            .colours
            .iter()
            .zip(&r.fade)
            .map(|(c, f)| {
                [
                    f32::from(c[0]) / 255.0,
                    f32::from(c[1]) / 255.0,
                    f32::from(c[2]) / 255.0,
                    f32::from(c[3]) / 255.0 * f[0],
                ]
            })
            .collect();
        let mesh = mesh_of(positions, colours, r.indices);
        show(
            &mut commands,
            entity,
            drawn,
            mesh,
            WISP_TINT,
            &mut meshes,
            &mut materials,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The plugin's systems run with a barricade standing on a navigation
    /// grid (their parameters resolve, the stamp goes on and comes off).
    #[test]
    fn the_effects_run_with_a_barricade_on_a_grid() {
        use bevy::asset::AssetPlugin;
        use exa_world::nav::{Layer, NavCell};
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<GlowMaterial>()
            .init_asset::<Image>()
            .init_asset::<exa_render::force_effects::ForceRibbonMaterial>()
            .init_asset::<exa_render::force_effects::ForceShellMaterial>()
            .add_plugins(ThaumEffectsPlugin);
        let mut grid = NavGrid::empty(40, 40, 0.0);
        for z in 0..40 {
            for x in 0..40 {
                let floor = NavCell {
                    floor: 10,
                    ..NavCell::default()
                };
                grid.set_cell(x, z, Layer::Lower, floor);
            }
        }
        app.insert_resource(grid);
        let frame = WallFrame::facing(Vec3::new(0.0, 50.0, 0.0), Vec3::Z);
        let mut draws = Mt19937::new(5489);
        let wall = Barricade::raise(frame, 0.25, 0, |_, _, _| None, None, &mut draws);
        let wall = app
            .world_mut()
            .spawn(LiveBarricade { wall, caster: None })
            .id();
        app.world_mut().run_schedule(FixedUpdate);
        app.update();
        assert!(app.world().get::<NavStamped>(wall).is_some());
        let stamped: u32 = {
            let grid = app.world().resource::<NavGrid>();
            (0..40)
                .flat_map(|z| (0..40).map(move |x| (x, z)))
                .map(|(x, z)| u32::from(grid.cell(x, z, Layer::Lower).penalty))
                .sum()
        };
        assert!(stamped > 0, "the line is on the grid");
        app.world_mut()
            .get_mut::<LiveBarricade>(wall)
            .unwrap()
            .wall
            .state = super::super::barricade::state::GONE;
        app.world_mut().run_schedule(FixedUpdate);
        let grid = app.world().resource::<NavGrid>();
        assert!(
            (0..40).all(|z| (0..40).all(|x| grid.cell(x, z, Layer::Lower).penalty == 0)),
            "the fall takes it back"
        );
    }

    #[test]
    fn a_segment_meets_a_sphere_beside_it() {
        assert!(segment_meets(
            Vec3::ZERO,
            Vec3::X * 10.0,
            Vec3::new(5.0, 2.0, 0.0),
            2.0
        ));
        assert!(!segment_meets(
            Vec3::ZERO,
            Vec3::X * 10.0,
            Vec3::new(5.0, 2.1, 0.0),
            2.0
        ));
        assert!(segment_meets(
            Vec3::ZERO,
            Vec3::ZERO,
            Vec3::new(0.0, 1.0, 0.0),
            1.0
        ));
        assert!(!segment_meets(
            Vec3::ZERO,
            Vec3::X,
            Vec3::new(3.0, 0.0, 0.0),
            1.0
        ));
    }

    #[test]
    fn a_shock_comes_within_150_ms_of_its_time() {
        let mut roll = Roll(0);
        // Never shocked: it lands, the next is due 500 ms on plus a draw of
        // the generator (state 1: 0 ms).
        let (next, yellow) = shock_reaction(None, 0, 0.25, 0.05, false, false, &mut roll).unwrap();
        assert_eq!(next, 500);
        assert!((yellow - 0.2).abs() < 1e-6);
        // 499 + 150 is short of 500 by a millisecond.
        assert_eq!(
            shock_reaction(Some(500), 349, 0.2, 0.05, false, false, &mut roll),
            None
        );
        assert_eq!(roll, Roll(1), "a refused shock draws nothing");
        // State 0x0808_8406 times 200, high half: 6 ms.
        let (next, yellow) =
            shock_reaction(Some(500), 350, 0.2, 0.05, false, false, &mut roll).unwrap();
        assert_eq!(next, 856);
        assert!((yellow - 0.15).abs() < 1e-6);
        // Collapsing or dead: the clock moves, the health stays.
        let (_, kept) = shock_reaction(None, 0, 0.1, 0.05, true, false, &mut roll).unwrap();
        assert_eq!(kept, 0.1);
        assert_eq!(
            shock_reaction(None, 0, 0.03, 0.05, false, false, &mut roll).map(|s| s.1),
            Some(0.0)
        );
        // A released race: 500 ms on, no draw, its health kept.
        let before = roll;
        assert_eq!(
            shock_reaction(None, 1_000, 0.2, 0.05, false, true, &mut roll),
            Some((1_500, 0.2))
        );
        assert_eq!(roll, before, "a release draws nothing");
    }

    #[test]
    fn a_shocked_fighter_waits_for_its_next_shock() {
        let mut app = App::new();
        app.add_message::<ArcShocked>()
            .insert_resource(BodyTicks(0))
            .insert_resource(EffectRoll(Roll(0)))
            .add_systems(Update, shock_fighters);
        let fighter = app.world_mut().spawn(Fighter::default()).id();
        let shock = ArcShocked {
            body: fighter,
            direction: Vec3::Y,
            strength: 0.05,
        };
        let yellow = |app: &App| app.world().get::<Fighter>(fighter).unwrap().health.yellow;
        // Two arcs in one tick: the second comes too early.
        app.world_mut().write_message(shock);
        app.world_mut().write_message(shock);
        app.update();
        assert!((yellow(&app) - 0.20).abs() < 1e-6);
        assert_eq!(
            app.world().get::<ShockReady>(fighter),
            Some(&ShockReady(500))
        );
        // 29 ticks are 348 ms: 348 + 150 is short of 500.
        app.insert_resource(BodyTicks(29));
        app.world_mut().write_message(shock);
        app.update();
        assert!((yellow(&app) - 0.20).abs() < 1e-6);
        // 30 ticks are 360 ms.
        app.insert_resource(BodyTicks(30));
        app.world_mut().write_message(shock);
        app.update();
        assert!((yellow(&app) - 0.15).abs() < 1e-6);
        assert_eq!(
            app.world().get::<ShockReady>(fighter),
            Some(&ShockReady(866))
        );
        assert_eq!(
            app.world().get::<StateFlags>(fighter).map(|f| f.0),
            Some(StateFlags::SHOCK_STAGGER)
        );
    }

    /// A collapsing fighter keeps its yellow and does not take the stagger.
    #[test]
    fn a_collapsing_fighter_is_not_staggered() {
        let mut app = App::new();
        app.add_message::<ArcShocked>()
            .insert_resource(BodyTicks(0))
            .insert_resource(EffectRoll(Roll(0)))
            .add_systems(Update, shock_fighters);
        let fighter = app
            .world_mut()
            .spawn((Fighter::default(), Condition::COLLAPSING))
            .id();
        app.world_mut().write_message(ArcShocked {
            body: fighter,
            direction: Vec3::Y,
            strength: 0.05,
        });
        app.update();
        let yellow = app.world().get::<Fighter>(fighter).unwrap().health.yellow;
        assert!((yellow - 0.25).abs() < 1e-6);
        assert_eq!(app.world().get::<StateFlags>(fighter).map(|f| f.0), Some(0));
    }

    #[test]
    fn each_struck_part_lowers_the_energy_above_a_quarter() {
        assert!((energy_after_knocks(0.26, &[0.004; 3]) - 0.248).abs() < 1e-6);
        assert_eq!(energy_after_knocks(0.25, &[0.004]), 0.25);
        assert_eq!(energy_after_knocks(1.0, &[]), 1.0);
    }

    /// The plugin builds and ticks with no game and no barricade: the
    /// interrupt path's query must resolve after the plugin's messages and
    /// assets are in place.
    #[test]
    fn the_effects_plugin_ticks_without_a_game() {
        use bevy::asset::AssetPlugin;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<GlowMaterial>()
            .init_asset::<Image>()
            .init_asset::<exa_render::force_effects::ForceRibbonMaterial>()
            .init_asset::<exa_render::force_effects::ForceShellMaterial>()
            .add_plugins(ThaumEffectsPlugin);
        for _ in 0..3 {
            app.update();
        }
    }
}
