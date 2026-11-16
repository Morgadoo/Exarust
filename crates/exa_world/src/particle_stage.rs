//! The particle systems' body pass inside the bodies' solver, at the
//! original's point in the grid sector's step.
//!
//! # What the original does
//!
//! A particle system is one of the sector's simulated objects. Its own step
//! (the ageing, integration and spawn of [`crate::particles::Emitter::tick`])
//! runs in the solver's object step, after the bounds pass; its class
//! carries the solver's body-contact bit (every particle system's
//! constructor sets it; the fire presets clear it on their second system),
//! so after body–body detection the sector walks the nine grid cells around
//! the system's cell (from its bound's centre) and hands it every body there
//! whose bound sphere is not apart from the system's bound sphere, whose
//! mesh box the system's centre lies within its radius of, and that is not
//! disabled (an awake system takes the sleeping bodies too). It does not
//! take the sector's first pass before the character ticks: no particle
//! system has that class bit (the fire presets' and the portal's writes of
//! it are on their light sources, and the displacement image's on its
//! geometry node).
//!
//! The system's body slot skips the body whose scene node is its own node's
//! parent (a torch's flame and the torch), points its frame at the body's
//! matrix and runs [`Emitter::body_pass`]: each live particle taken into the
//! body's frame, its radius its age times the collision growth rate, tested
//! against the node's bound sphere and box, and pushed out of every face of
//! the node mesh's two-sided sphere query that it reaches; a particle met
//! goes back to the world, its velocity takes −0.98 of the push and it
//! loses a tick of life. The body then takes `hits × dose` through its dose
//! slot: a character's part whose skin bone index (the 1-based bone less
//! one) is above zero adds it to the character's burn strength
//! (`exa_combat::burn::Burn::add_dose`, which the character's next upkeep
//! burns); any other body's slot hands the value back and keeps nothing.
//!
//! # Exarust
//!
//! [`ParticleSolve`] is the solver stage ([`SolverStage::sim_object_contacts`])
//! for the emitters staged into it: each part linked in the grid cell its
//! mesh box's world centre falls in (the awake bodies, then the sleeping,
//! each body's parts in order), the emitters in staging order, each
//! walking the nine cells around its own; the doses are kept in call order
//! for the burn's system (`exa_combat::burn::take_particle_doses`) after the
//! solver. A part is met through [`PartContact`]: its frame in centimetres
//! (the original's transpose product and its row sum), its node mesh's
//! bound and the shared two-sided sphere query
//! ([`exa_physics::mesh_tree::two_sided_contacts`]).
//!
//! Open: the fires' emitters still step once a frame in the frame's world
//! ([`crate::flame`]), so none is staged in the running game; the
//! particles' own step in the simulation (the sector's object step) is the
//! missing caller.
//!
//! Evidence: the sector step's object stages, the particle system's
//! constructor, body slot, contact walk and push-out callback, and the
//! dose slots of the rigid body, the door and the motile part, read from
//! `Exanima.exe`.

use bevy::math::Affine3A;
use bevy::prelude::*;
use exa_physics::body::{Part, Ragdoll, SolverStage};
use exa_physics::engine_math::{point_into_frame, point_out_of_frame};
use exa_physics::sector::{self, ObjectBodyPart, ObjectBounds};

use crate::cloth_parts::ClothGrid;
use crate::particles::{BodyContact, Emitter};

/// Centimetres a metre.
const CM: f32 = 100.0;

/// A body part as the particles meet it, in centimetres (see the module).
pub struct PartContact<'a> {
    part: &'a Part,
    /// The part's frame with its translation in centimetres.
    frame: Affine3A,
}

impl<'a> PartContact<'a> {
    /// The part where the solver has it now.
    pub fn new(part: &'a Part) -> Self {
        let mut frame = part.transform();
        frame.translation *= CM;
        Self { part, frame }
    }
}

impl BodyContact for PartContact<'_> {
    fn to_local(&self, point: Vec3) -> Vec3 {
        point_into_frame(point, &self.frame)
    }

    fn to_world(&self, point: Vec3) -> Vec3 {
        point_out_of_frame(point, &self.frame)
    }

    fn bound(&self) -> (Vec3, Vec3, f32) {
        let mesh = &self.part.pairing.mesh;
        (mesh.centre * CM, mesh.half * CM, mesh.radius * CM)
    }

    fn query(
        &self,
        sphere: &mut (Vec3, f32),
        on_point: &mut dyn FnMut(&mut (Vec3, f32), Vec3),
    ) -> bool {
        // A part without a node mesh has no faces to meet.
        let Some(triangles) = self.part.triangles.as_deref() else {
            return false;
        };
        exa_physics::mesh_tree::two_sided_contacts(
            triangles,
            self.part.tree.as_deref(),
            sphere,
            on_point,
        )
    }
}

/// An emitter handed to the solver for a tick.
#[derive(Debug)]
pub struct StagedEmitter {
    /// The emitter's entity.
    pub entity: Entity,
    pub emitter: Emitter,
    /// The part whose node is the emitter's node's parent (its own body's
    /// part, which it never meets).
    pub parent: Option<Entity>,
    /// The class's body-contact bit (cleared on a fire's second system).
    pub contacts: bool,
}

/// One body part's dose, in the order the body pass made them: the part's
/// entity, its skin bone index (the bone less one) and the dose.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dose {
    pub part: Entity,
    pub skin_bone: i32,
    pub dose: f32,
}

/// The solver's particle stage (see the module).
#[derive(Debug, Default)]
pub struct ParticleSolve {
    /// The emitters of this tick, in the object list's order.
    pub staged: Vec<StagedEmitter>,
    /// The cells the parts and the emitters are keyed in.
    pub grid: ClothGrid,
    /// The doses of this tick's passes.
    doses: Vec<Dose>,
    /// Each cell's parts: the list (sleeping or awake), the body's place in
    /// it and the part's index, kept from tick to tick.
    cells: bevy::platform::collections::HashMap<(i32, i32), Vec<LinkedSlot>>,
}

/// A part linked in a cell: whether its body is in the sleeping list, the
/// body's place in that list and the part's index.
type LinkedSlot = (bool, usize, usize);

impl ParticleSolve {
    /// A stage with `staged` emitters keyed in `grid`'s cells.
    pub fn new(staged: Vec<StagedEmitter>, grid: ClothGrid) -> Self {
        Self {
            staged,
            grid,
            ..Self::default()
        }
    }

    /// Takes the doses the passes made since the last call, in call order.
    pub fn take_doses(&mut self) -> Vec<Dose> {
        std::mem::take(&mut self.doses)
    }

    /// The body passes of every staged emitter (see the module).
    pub fn contact(&mut self, awake: &[&mut Ragdoll], sleeping: &[&mut Ragdoll]) {
        if !self.staged.iter().any(|s| s.contacts) {
            return;
        }
        let grid = self.grid;
        for list in self.cells.values_mut() {
            list.clear();
        }
        for (asleep, bodies) in [(false, awake), (true, sleeping)] {
            for (b, body) in bodies.iter().enumerate() {
                for (k, part) in body.parts.iter().enumerate() {
                    if part.pairing.out || part.spheres.is_empty() {
                        continue;
                    }
                    let centre = part.pairing.mesh.world_centre(&part.transform()) * CM;
                    self.cells
                        .entry(grid.key_of(centre))
                        .or_default()
                        .push((asleep, b, k));
                }
            }
        }
        for one in &mut self.staged {
            if !one.contacts {
                continue;
            }
            let object = ObjectBounds {
                centre: one.emitter.bounds.centre,
                radius: one.emitter.bounds.radius,
            };
            for cell in grid.around(grid.key_of(object.centre)) {
                let Some(linked) = self.cells.get(&cell) else {
                    continue;
                };
                for &(asleep, b, k) in linked {
                    let body: &Ragdoll = if asleep { &*sleeping[b] } else { &*awake[b] };
                    let part = &body.parts[k];
                    if body.disabled() || !meets(object, body, part) {
                        continue;
                    }
                    if one.parent == Some(part.entity) {
                        continue;
                    }
                    let dose = one.emitter.body_pass(&PartContact::new(part));
                    self.doses.push(Dose {
                        part: part.entity,
                        skin_bone: part.bone as i32 - 1,
                        dose,
                    });
                }
            }
        }
    }
}

/// The sector's gates between an object and a part: the part's bound sphere
/// not apart from the object's, and the object's centre within its radius
/// of the part's mesh box.
fn meets(object: ObjectBounds, body: &Ragdoll, part: &Part) -> bool {
    let now = part.transform();
    sector::object_meets_part(
        object,
        &ObjectBodyPart {
            now,
            previous: now,
            spheres: &part.spheres,
            centre: now.transform_point3(part.pairing.sphere.centre),
            radius: part.pairing.sphere.radius,
            mesh: &part.pairing.mesh,
            countdown: body.countdown(),
            points: part.positions,
            previous_points: part.previous,
        },
    )
}

impl SolverStage for ParticleSolve {
    /// The particles take no part in the iterations or the friction passes.
    fn coupled(&self, _bodies: &[&mut Ragdoll], _coupled: &mut [bool]) {}

    fn sim_object_contacts(&mut self, bodies: &mut [&mut Ragdoll], sleeping: &mut [&mut Ragdoll]) {
        self.contact(bodies, sleeping);
    }
}

#[cfg(test)]
#[path = "particle_stage_test.rs"]
mod tests;
