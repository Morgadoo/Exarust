//! The level's cloths inside the bodies' tick, at the original's points in
//! the grid sector's step.
//!
//! The original's sector step runs its simulated objects inside the
//! bodies' solver: after the bounds pass each object steps (the cloth's
//! integrate slot) and is re-bucketed; after body–body detection each
//! object meets the bodies linked in the nine cells around its own (the
//! cloth's body-contact slot, which pushes the cloth's triangles *and the
//! body*, and trades sleep countdowns with it), then runs its pass after
//! the collisions and its level contact; then the bodies meet the level
//! and rub. A cloth's push therefore reaches the body before the body's own
//! level detection, friction and post-step, and a later cloth (or a later
//! sphere of the same body) meets the body where the earlier push left it.
//!
//! Exarust keeps that order with three pieces around the solver's system
//! ([`exa_physics::body::step_bodies`]):
//!
//! 1. [`stage_cloths`], before the solver: the object list's sequential
//!    plan (each cloth's frame, owner and re-bucket), then every awake
//!    cloth's own step at once (each writes only itself), and the cloths
//!    handed to the solver's cloth stage ([`ClothSolve`]).
//! 2. [`ClothSolve`]'s object-contact hook, inside the solver after
//!    body–body detection: one cloth at a time, in the object list's
//!    order, the body parts in the nine cells around it (re-bucketed where
//!    the solver has moved them), each part's spheres against the cloth's
//!    triangles with the body's share pushed back at once
//!    ([`exa_physics::cloth_body::touch_ragdoll`]). Taking the cloths in a
//!    fixed order makes the bodies' results independent of the threads.
//! 3. [`finish_cloths`], after the solver: every cloth's pass after the
//!    collisions, its level contact and the end of its tick, at once (each
//!    reads only itself and the static level, which nothing in between
//!    changes), and the cloths handed back to their nodes.
//!
//! Steps 1 and 3 read and write only the cloths, which nothing else in the
//! solver touches, so running them outside it gives the same bits as
//! running them at the original's two points inside it.
//!
//! The stage's cost: each awake cloth's step goes to the compute pool as
//! soon as its plan is made, so the pool's threads wake while the plan goes
//! on; the hook resolves each cell's parts to the solver's slots once for
//! every cloth that reads it, and a particle-mode cloth's gather makes its
//! own pre-test; the hook's lists and the staged cloths are kept from tick
//! to tick; a piece with fewer than two cloths to work on runs on the
//! calling thread. The three pieces still meet the pool three times a tick
//! where the stage before the body contacts moved into the solver met it
//! once (each meeting waits for the pool's threads to wake): the order
//! fixes that floor.

use bevy::ecs::entity::EntityHashMap;
use bevy::math::Affine3A;
use bevy::prelude::*;

use crate::cloth::{
    ClothContacts, ClothDebug, ClothInstance, ClothObjects, ClothWind, ClothWork,
    body_meets_objects, carry_into_frame, cloth_bound,
};
use crate::cloth_parts::{ClothGrid, LinkedPart, PartCells};
use exa_physics::Physics;
use exa_physics::body::{Ragdoll, SolverStage, SolverStages};
use exa_physics::cloth::{self, Cloth};
use exa_physics::cloth_body;
use exa_physics::cloth_level;
use exa_physics::contact::Level;
use exa_physics::sector::{self, ObjectBodyPart, ObjectContacts};

/// What the sequential half of the stage settles for one cloth before its
/// step and contacts.
#[derive(Debug, Clone)]
pub(crate) struct ClothPlan {
    /// The cloth's id in the object list.
    pub(crate) id: usize,
    /// Its cell after the object list's pre-step re-bucket.
    pub(crate) cell: (i32, i32),
    /// The world-to-cloth transform.
    pub(crate) to_object: Affine3A,
    /// The cloth's own character: every node above it.
    pub(crate) own: Vec<Entity>,
    /// The scene node whose static mesh the cloth is drawn from.
    pub(crate) own_node: Option<Entity>,
    /// A particle-mode cloth's owner (the nearest body part above it).
    pub(crate) owner: Option<Entity>,
    /// The node's frame this tick, and whether it moved since the last.
    pub(crate) frame: Affine3A,
    pub(crate) moved: bool,
    /// Whether the cloth was asleep at the stage's start.
    pub(crate) asleep: bool,
    /// Whether the cloth's record is the particle mode.
    pub(crate) particle_mode: bool,
    /// The cloth's gravity (cm a tick², its frame).
    pub(crate) gravity: Vec3,
    /// Its node's world transform this tick.
    pub(crate) at: GlobalTransform,
}

/// One cloth handed to the solver for the tick.
#[derive(Debug)]
struct Staged {
    entity: Entity,
    plan: ClothPlan,
    cloth: Cloth,
    /// The cloth's own step's result, once it ran.
    integrated: Option<cloth::Integrated>,
    /// The tick does nothing for it: a sleeper no body reaches in a still
    /// frame, or a sleeper while contacts are off.
    skipped: bool,
    /// Its debug line so far.
    work: ClothWork,
    /// Its node's rustle flag, read before its end of tick.
    rustling: bool,
    /// Its end of tick, for the sequential end of the stage.
    outcome: Option<ClothOutcome>,
    /// The parts offered to it, and the ones that pushed it with their
    /// countdowns (debug only).
    offered: Vec<Entity>,
    pushers: Vec<(Entity, i32)>,
    started: Option<std::time::Instant>,
}

/// The solver's cloth stage: the cloths of this tick, between
/// [`stage_cloths`] and [`finish_cloths`], and the body parts' cell index
/// the object stage reads.
#[derive(Default)]
pub struct ClothSolve {
    staged: Vec<Staged>,
    /// Whether the object-contact hook ran for the staged cloths.
    contacted: bool,
    /// Whether the hook's sleeping bodies come in the solver's walk order
    /// (false for the stand-in run without a solver).
    sorted: bool,
    /// The body parts' cells.
    pub(crate) index: PartCells,
    /// Whether contacts run this tick (on, with a level and its grid).
    live: bool,
    /// The object grid's keys this tick.
    grid: ClothGrid,
    on: bool,
    /// Bodies re-bucketed by the hook (debug).
    rebucketed: u32,
    /// The stage's split this tick (µs): the part index before the solver,
    /// the plan, the cloths' own steps, the body contacts in the solver.
    index_micros: u64,
    plan_micros: u64,
    step_micros: u64,
    contact_micros: u64,
    /// The part of the contacts spent bucketing the parts around the
    /// cloths (µs), before any cloth meets them.
    bucket_micros: u64,
    /// The hook's lists, kept from tick to tick so a tick allocates
    /// nothing: each body's slot, the cells around the cloths, and (without
    /// a solver) the sleepers by place.
    slots: EntityHashMap<Slot>,
    cells: AroundCells,
    scanned: bevy::platform::collections::HashMap<u64, usize>,
}

/// The cells around the tick's cloths, each with its linked parts sorted
/// as a full re-bucket links them; the lists are kept for the next tick.
#[derive(Default)]
struct AroundCells {
    /// Each cell's list in `lists`.
    at: bevy::platform::collections::HashMap<(i32, i32), usize>,
    lists: Vec<Vec<LinkedPart>>,
    /// Each list's parts whose body the solver holds, by its slot.
    ready: Vec<Vec<(Slot, usize)>>,
}

impl AroundCells {
    fn clear(&mut self) {
        self.at.clear();
    }

    /// Reads `cell` from `index` unless it was read this tick.
    fn read(&mut self, cell: (i32, i32), index: &PartCells) {
        let next = self.at.len();
        if let bevy::platform::collections::hash_map::Entry::Vacant(v) = self.at.entry(cell) {
            v.insert(next);
            if self.lists.len() <= next {
                self.lists.push(Vec::new());
            }
            index.sorted_in(cell, &mut self.lists[next]);
        }
    }

    /// Resolves every list's bodies into their slots (a body the solver
    /// does not hold is left out), once the slots are known.
    fn resolve(&mut self, slots: &EntityHashMap<Slot>) {
        let used = self.at.len();
        if self.ready.len() < used {
            self.ready.resize_with(used, Vec::new);
        }
        for (linked, ready) in self.lists[..used].iter().zip(&mut self.ready) {
            ready.clear();
            ready.extend(
                linked
                    .iter()
                    .filter_map(|at| slots.get(&at.body).map(|&slot| (slot, at.part))),
            );
        }
    }

    /// A cell's parts with their bodies' slots, in the cell's order.
    fn get(&self, cell: &(i32, i32)) -> Option<&[(Slot, usize)]> {
        self.at.get(cell).map(|&i| self.ready[i].as_slice())
    }

    /// Every part read this tick.
    fn parts(&self) -> impl Iterator<Item = &LinkedPart> {
        self.lists[..self.at.len()].iter().flatten()
    }
}

impl std::fmt::Debug for ClothSolve {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClothSolve")
            .field("staged", &self.staged.len())
            .field("contacted", &self.contacted)
            .finish()
    }
}

impl SolverStage for ClothSolve {
    /// The cloths take no part in the iterations or the friction passes.
    fn coupled(&self, _bodies: &[&mut Ragdoll], _coupled: &mut [bool]) {}

    fn sim_object_contacts(&mut self, bodies: &mut [&mut Ragdoll], sleeping: &mut [&mut Ragdoll]) {
        self.sorted = true;
        self.contact(bodies, sleeping);
    }
}

/// Where a body sits in the solver's two lists.
#[derive(Debug, Clone, Copy)]
enum Slot {
    Awake(usize),
    Sleeping(usize),
}

impl ClothSolve {
    /// The object stage's body contacts for every staged cloth, one at a
    /// time in the object list's order (see the module).
    fn contact(&mut self, awake: &mut [&mut Ragdoll], sleeping: &mut [&mut Ragdoll]) {
        if self.contacted {
            return;
        }
        self.contacted = true;
        if self.staged.is_empty() {
            return;
        }
        let started = self.on.then(std::time::Instant::now);
        if self.live {
            // The bodies the solver moved this tick are re-bucketed where
            // they are now (the original's bounds pass buckets them before
            // the objects meet them); a body still asleep has not moved.
            let mut slots = std::mem::take(&mut self.slots);
            slots.clear();
            let grid = self.grid;
            self.rebucketed = 0;
            for (i, body) in awake.iter().enumerate() {
                if let Some(key) = self.index.body_of(body) {
                    slots.insert(key, Slot::Awake(i));
                    self.index.rebucket(key, body, grid);
                    self.rebucketed += 1;
                }
            }
            // A sleeper a contact woke this tick may have moved.
            for (i, body) in sleeping.iter().enumerate() {
                if body.asleep() {
                    continue;
                }
                if let Some(key) = self.index.body_of(body) {
                    slots.insert(key, Slot::Sleeping(i));
                    self.index.rebucket(key, body, grid);
                    self.rebucketed += 1;
                }
            }
            // The cells around the cloths, each read once in the order a
            // full re-bucket links it (the original buckets the bodies in
            // its bounds pass; an object's push does not move a body to
            // another cell within the tick).
            let mut cells = std::mem::take(&mut self.cells);
            cells.clear();
            for one in &self.staged {
                for cell in grid.around(one.plan.cell) {
                    cells.read(cell, &self.index);
                }
            }
            // The sleepers linked there, found in the solver's list by
            // their place (the list is in walk order: the place from the
            // largest down) or, without a solver, by a scan.
            let mut scanned = std::mem::take(&mut self.scanned);
            scanned.clear();
            if !self.sorted {
                scanned.extend(
                    sleeping
                        .iter()
                        .enumerate()
                        .map(|(i, b)| (b.list_place(), i)),
                );
            }
            for at in cells.parts() {
                if slots.contains_key(&at.body) {
                    continue;
                }
                let place = self.index.place_of(at.body);
                let found = if place == 0 {
                    // A body never placed (built by hand): by its part.
                    sleeping
                        .iter()
                        .position(|b| b.list_place() == 0 && self.index.body_of(b) == Some(at.body))
                } else if self.sorted {
                    let i = sleeping.partition_point(|b| {
                        let p = b.list_place();
                        p != 0 && p > place
                    });
                    (i < sleeping.len() && sleeping[i].list_place() == place).then_some(i)
                } else {
                    scanned.get(&place).copied()
                };
                if let Some(i) = found {
                    slots.insert(at.body, Slot::Sleeping(i));
                }
            }
            cells.resolve(&slots);
            if let Some(t) = started {
                self.bucket_micros = t.elapsed().as_micros() as u64;
            }
            // One cloth at a time in the list's order where it may push a
            // body; a run of particle-mode cloths (the hair: they push no
            // body, only read them) runs at once, which reads the same
            // bodies as one at a time.
            let mut staged = std::mem::take(&mut self.staged);
            let mut i = 0;
            while i < staged.len() {
                if staged[i].plan.particle_mode {
                    let end = staged[i..]
                        .iter()
                        .position(|s| !s.plan.particle_mode)
                        .map_or(staged.len(), |n| i + n);
                    let (awake, sleeping): (&[&mut Ragdoll], &[&mut Ragdoll]) = (awake, sleeping);
                    let mut run = &mut staged[i..end];
                    if run.len() < 2 {
                        for one in run.iter_mut() {
                            self.prepare(one, &cells, awake, sleeping);
                        }
                    } else {
                        let pool = bevy::tasks::ComputeTaskPool::get_or_init(
                            bevy::tasks::TaskPool::default,
                        );
                        use bevy::tasks::ParallelSliceMut;
                        run.par_splat_map_mut(pool, None, |_, chunk| {
                            for one in chunk.iter_mut() {
                                self.prepare(one, &cells, awake, sleeping);
                            }
                        });
                    }
                    i = end;
                } else {
                    let meeting = self.prepare(&mut staged[i], &cells, awake, sleeping);
                    self.push(&mut staged[i], &meeting, awake, sleeping);
                    i += 1;
                }
            }
            self.staged = staged;
            self.slots = slots;
            self.cells = cells;
            self.scanned = scanned;
        } else {
            for one in &mut self.staged {
                if one.plan.asleep {
                    // A sleeping cloth with no contacts does not step.
                    one.skipped = true;
                }
            }
        }
        if let Some(t) = started {
            self.contact_micros = t.elapsed().as_micros() as u64;
        }
    }

    /// One cloth's side of its body contacts that only reads the bodies
    /// (see [`ClothSolve::contact`]): the parts offered, the pre-test, the
    /// sleeper's skip or own step, and a particle-mode cloth's whole
    /// contact. Returns the parts a triangle-mode cloth then meets
    /// ([`ClothSolve::push`]).
    fn prepare(
        &self,
        one: &mut Staged,
        cells: &AroundCells,
        awake: &[&mut Ragdoll],
        sleeping: &[&mut Ragdoll],
    ) -> Vec<(Slot, usize)> {
        let on = self.on;
        let bodies_started = on.then(std::time::Instant::now);
        let plan = &one.plan;
        // The parts linked in the nine cells around the cloth's own, in the
        // original's scan order, each cell's parts in the order a full
        // re-bucket links them, passing the sleep, disabled and
        // own-character filters.
        let mut offered: Vec<(Slot, usize)> = Vec::new();
        for cell in self.grid.around(plan.cell) {
            let Some(linked) = cells.get(&cell) else {
                continue;
            };
            for &(slot, index) in linked {
                let body = body_ref(slot, awake, sleeping);
                let Some(part) = body.parts.get(index) else {
                    continue;
                };
                if !body_meets_objects(body)
                    || !sector::object_takes_body(plan.asleep, body.asleep())
                    || (!plan.particle_mode && plan.own.contains(&part.entity))
                {
                    continue;
                }
                offered.push((slot, index));
            }
        }
        one.work.parts = offered.len() as u32;
        if on {
            one.offered = offered
                .iter()
                .map(|&(slot, part)| body_ref(slot, awake, sleeping).parts[part].entity)
                .collect();
        }
        // The pre-test is against the cloth where its own step left it.
        let (object, _) = cloth_bound(&one.cloth, &plan.at);
        let mut contacts = ObjectContacts::default();
        let meeting: Vec<(Slot, usize)>;
        if plan.particle_mode {
            // The particle mode's gather makes the pre-test itself, and
            // numbers the parts as offered (the debug tags).
            meeting = Vec::new();
            let parts = offered.iter().map(|&(slot, part)| {
                let body = body_ref(slot, awake, sleeping);
                let view = part_view(body, part);
                let owner = plan.owner == Some(body.parts[part].entity);
                (view, owner)
            });
            contacts.gather_particle_bodies(&plan.to_object, object, parts);
            one.work.bodies = contacts.particle_bodies.len() as u32;
        } else {
            meeting = offered
                .iter()
                .copied()
                .filter(|&(slot, part)| {
                    let body = body_ref(slot, awake, sleeping);
                    sector::object_meets_part(object, &part_view(body, part))
                })
                .collect();
            one.work.bodies = meeting.len() as u32;
        }
        let met = if plan.particle_mode {
            !contacts.particle_bodies.is_empty()
        } else {
            !meeting.is_empty()
        };
        if let Some(t) = bodies_started {
            one.work.body_micros = t.elapsed().as_micros() as u64;
        }
        // A sleeping cloth that no body reaches, in a frame that did not
        // move, is the level stage's fixed point: nothing to do.
        if plan.asleep && !plan.moved && !met {
            one.work.skipped = 1;
            one.skipped = true;
            return Vec::new();
        }
        if plan.asleep {
            // The sleeper's own step (it only keeps its countdown) runs now
            // that a body or its frame calls for its tick.
            one.integrated = cloth::integrate(&mut one.cloth, plan.gravity, None);
        }
        let Some(integrated) = one.integrated.as_mut() else {
            return Vec::new();
        };
        let cloth = &mut one.cloth;
        cloth.level.clear();
        cloth.level_pieces.clear();
        if plan.particle_mode {
            cloth.particle_bodies = std::mem::take(&mut contacts.particle_bodies);
            cloth::touch_listed_bodies(cloth, &mut integrated.awake);
            if on {
                for &index in &cloth.pushed_by {
                    if let Some(b) = cloth.particle_bodies.get(index)
                        && let Some(&entity) = one.offered.get(b.part)
                    {
                        one.pushers.push((entity, b.countdown));
                    }
                }
            }
            return Vec::new();
        }
        meeting
    }

    /// A triangle-mode cloth's contacts with the parts it meets (from
    /// [`ClothSolve::prepare`]): each part's spheres push the cloth's
    /// triangles, and the part takes its share back at once.
    fn push(
        &self,
        one: &mut Staged,
        meeting: &[(Slot, usize)],
        awake: &mut [&mut Ragdoll],
        sleeping: &mut [&mut Ragdoll],
    ) {
        if meeting.is_empty() {
            return;
        }
        let on = self.on;
        let plan = &one.plan;
        let Some(integrated) = one.integrated.as_mut() else {
            return;
        };
        let cloth = &mut one.cloth;
        let object_box = cloth::bounds(&cloth.particles);
        let bounds = cloth_body::triangle_bounds(cloth);
        let contact = cloth_body::LiveContact {
            to_object: plan.to_object,
            cloth_box: object_box,
            padding: cloth.record.padding_cm,
            held_chain: false,
        };
        for &(slot, part) in meeting {
            let body: &mut Ragdoll = match slot {
                Slot::Awake(i) => &mut *awake[i],
                Slot::Sleeping(i) => &mut *sleeping[i],
            };
            let touch = cloth_body::touch_ragdoll(
                &cloth.particles,
                &cloth.triangles,
                &bounds,
                &mut cloth.pushes,
                &mut cloth.rustle_tail,
                body,
                part,
                &contact,
            );
            one.work.spheres += touch.spheres as u32;
            one.work.swept += touch.swept as u32;
            cloth.body_tests += touch.tests;
            if touch.pushed {
                if on {
                    one.pushers
                        .push((body.parts[part].entity, body.countdown()));
                }
                integrated.awake |= cloth_body::trade_body_countdowns(&mut cloth.sleep, body);
            }
        }
    }
}

/// A body part as the sector's object stage reads it.
fn part_view(body: &Ragdoll, index: usize) -> ObjectBodyPart<'_> {
    let part = &body.parts[index];
    let now = part.transform();
    ObjectBodyPart {
        now,
        previous: part.settled_transform(),
        spheres: &part.spheres,
        centre: now.transform_point3(part.pairing.sphere.centre),
        radius: part.pairing.sphere.radius,
        mesh: &part.pairing.mesh,
        countdown: body.countdown(),
        points: part.positions,
        previous_points: part.previous,
    }
}

/// A body of the solver's lists, to read.
fn body_ref<'a>(
    slot: Slot,
    awake: &'a [&mut Ragdoll],
    sleeping: &'a [&mut Ragdoll],
) -> &'a Ragdoll {
    match slot {
        Slot::Awake(i) => awake[i],
        Slot::Sleeping(i) => sleeping[i],
    }
}

/// The level the stage reads: its collision world, static colliders and
/// scene grid.
type LevelResources<'w> = (
    Option<Res<'w, Physics>>,
    Option<Res<'w, exa_physics::StaticColliders>>,
    Option<Res<'w, exa_physics::SceneGrid>>,
);

/// The stage before the solver (see the module): the object list's
/// sequential plan, every awake cloth's own step, and the cloths handed to
/// the solver's [`ClothSolve`].
#[allow(clippy::too_many_arguments)]
pub(crate) fn stage_cloths(
    mut instances: Query<(Entity, &mut ClothInstance, &GlobalTransform)>,
    bodies: Query<(Entity, Ref<Ragdoll>)>,
    parents: Query<&ChildOf>,
    scene_nodes: Query<(), With<exa_physics::StaticNode>>,
    (physics, statics, scene): LevelResources,
    mut objects: ResMut<ClothObjects>,
    live_contacts: Res<ClothContacts>,
    debug: Res<ClothDebug>,
    wind: Res<ClothWind>,
    mut stages: ResMut<SolverStages>,
) {
    let Some(stage) = stages.get_mut::<ClothSolve>() else {
        return;
    };
    let on = debug.on;
    let started = on.then(std::time::Instant::now);
    stage.staged.clear();
    stage.contacted = false;
    stage.contact_micros = 0;
    stage.bucket_micros = 0;
    stage.rebucketed = 0;
    stage.on = on;
    stage.live = live_contacts.0 && physics.is_some() && statics.is_some();
    let gravity_world = cloth::gravity_cm(1.0);
    // The object grid takes the scene's shape (a change rebuilds it).
    let grid = ClothGrid::of_scene(scene.as_deref().copied());
    objects.shape(grid);
    stage.grid = grid;
    // The object list's stable order: the entity id, oldest first. A new or
    // removed cloth rebuilds the grid; otherwise each object migrates after
    // its tick, as the original's list does.
    let mut order: Vec<Entity> = instances.iter().map(|(entity, ..)| entity).collect();
    order.sort_unstable();
    let rebuilt = objects.order != order;
    if rebuilt {
        objects.order = order;
        objects.reset();
    }
    // Every part of every body (asleep too) that can meet an object, linked
    // in the grid cell its mesh box's world centre falls in, as the
    // sector's body grid links it. The index re-buckets only the bodies
    // changed since the last tick (a deep sleeper's parts stay where they
    // were), and nothing while no cloth is simulated ([`PartCells`]).
    if objects.order.is_empty() {
        stage.index.clear();
    } else {
        stage.index.refresh(
            bodies
                .iter()
                .map(|(entity, body)| (entity, body.is_changed(), body.into_inner())),
            grid,
        );
    }
    let indexed = on.then(std::time::Instant::now);
    // The sequential half, in the object list's order: each cloth's frame,
    // owner and own character, its carry into the frame, and the object
    // list's re-bucket before its step.
    let mut staged = std::mem::take(&mut stage.staged);
    staged.clear();
    let order = std::mem::take(&mut objects.order);
    // Whether the steps go to the pool: two awake cloths or more (the plan
    // leaves every cloth's countdown as it found it).
    let awake = instances
        .iter()
        .filter(|(_, instance, _)| instance.cloth.sleep > 0)
        .count();
    let index = &stage.index;
    let objects = &mut *objects;
    let mut plan_one = |id: usize, entity: Entity| -> Option<Staged> {
        let Ok((_, mut instance, at)) = instances.get_mut(entity) else {
            return None;
        };
        let particle_mode = instance.cloth.record.particle_mode();
        let frame = at.affine();
        let last_frame = instance.frame;
        let moved = last_frame != Some(frame);
        instance.frame = Some(frame);
        // The cloth's own character, and the scene node whose static mesh
        // the cloth is drawn from: neither pushes it. The original's static
        // stage skips the mesh of the cloth's own node; the body skip is a
        // guess [the cloth's body-contact call is taken to skip the parts
        // above its node].
        let mut own = Vec::new();
        let mut own_node = scene_nodes.contains(entity).then_some(entity);
        let mut above = entity;
        while let Ok(parent) = parents.get(above) {
            above = parent.parent();
            own.push(above);
            if own_node.is_none() && scene_nodes.contains(above) {
                own_node = Some(above);
            }
            if own.len() > 64 {
                break;
            }
        }
        // A particle-mode cloth's owner: the nearest body part above it
        // (the original's cloth takes the body its node's parent carries).
        // The cloth never sleeps below the owner's countdown, and its
        // particles keep their world motion when the owner moves its node.
        let owner = if particle_mode {
            own.iter().copied().find_map(|e| {
                let body = index.owner_of(e)?;
                let (_, body) = bodies.get(body).ok()?;
                Some((e, body.countdown()))
            })
        } else {
            None
        };
        if let Some((_, countdown)) = owner {
            instance.cloth.owner_sleep = countdown;
            if let Some(last) = last_frame
                && last != frame
            {
                carry_into_frame(&mut instance.cloth, last, frame);
            }
        }
        let (object, to_object) = cloth_bound(&instance.cloth, at);
        if rebuilt {
            objects.grid.insert(object.centre);
        } else {
            objects.grid.migrate(id, object.centre);
        }
        let cell = objects
            .grid
            .cell_of_id(id)
            .unwrap_or_else(|| grid.cell_of(object.centre));
        let asleep = instance.cloth.sleep <= 0;
        // A cloth that follows an owner falls along the world's gravity in
        // its node's frame.
        let gravity = if owner.is_some() {
            frame.inverse().transform_vector3(gravity_world)
        } else {
            gravity_world
        };
        // The cloth leaves its node for the tick (the solver's cloth stage
        // holds it).
        Some(Staged {
            entity,
            plan: ClothPlan {
                id,
                cell: grid.key_of_cell(cell),
                to_object,
                own,
                own_node,
                owner: owner.map(|(e, _)| e),
                frame,
                moved,
                asleep,
                particle_mode,
                gravity,
                at: *at,
            },
            cloth: std::mem::take(&mut instance.cloth),
            integrated: None,
            skipped: false,
            work: ClothWork::default(),
            offered: Vec::new(),
            pushers: Vec::new(),
            started: None,
            rustling: false,
            outcome: None,
        })
    };
    // Every awake cloth's own step at once: each reads only itself and the
    // wind, and writes only itself.
    let wind = &wind.0;
    let step = |one: &mut Staged| {
        let plan = &one.plan;
        let step_started = on.then(std::time::Instant::now);
        one.work.asleep = u32::from(plan.asleep);
        // The wind meets the cloth in the world: its particles are in
        // the node's frame, in centimetres.
        if !plan.asleep {
            let to_world = Affine3A::from_scale(Vec3::splat(100.0))
                * plan.frame
                * Affine3A::from_scale(Vec3::splat(0.01));
            let at = cloth::WindAt { wind, to_world };
            one.integrated = cloth::integrate(&mut one.cloth, plan.gravity, Some(at));
        }
        if let Some(t) = step_started {
            one.work.step_micros = t.elapsed().as_micros() as u64;
        }
        one.started = step_started;
    };
    // Fewer than two awake cloths (a level whose banners sleep) step on
    // this thread: the pool's dispatch would cost more than the work.
    // Otherwise each awake cloth's step goes to the pool as soon as it is
    // planned, so the pool's threads wake while the plan goes on.
    let mut planned = None;
    if awake < 2 {
        for (id, &entity) in order.iter().enumerate() {
            if let Some(mut one) = plan_one(id, entity) {
                step(&mut one);
                staged.push(one);
            }
        }
        planned = on.then(std::time::Instant::now);
    } else {
        let step = &step;
        let pool = bevy::tasks::ComputeTaskPool::get_or_init(bevy::tasks::TaskPool::default);
        let stepped = pool.scope(|s| {
            for (id, &entity) in order.iter().enumerate() {
                let Some(mut one) = plan_one(id, entity) else {
                    continue;
                };
                if one.plan.asleep {
                    step(&mut one);
                    staged.push(one);
                } else {
                    s.spawn(async move {
                        step(&mut one);
                        one
                    });
                }
            }
            planned = on.then(std::time::Instant::now);
        });
        staged.extend(stepped);
        staged.sort_unstable_by_key(|one| one.plan.id);
    }
    objects.order = order;
    stage.staged = staged;
    if let (Some(t), Some(indexed), Some(planned)) = (started, indexed, planned) {
        let micros = |a: std::time::Instant, b: std::time::Instant| {
            b.saturating_duration_since(a).as_micros() as u64
        };
        stage.index_micros = micros(t, indexed);
        stage.plan_micros = micros(indexed, planned);
        stage.step_micros = t.elapsed().as_micros() as u64 - micros(t, planned);
    }
}

/// The bodies split as the solver splits them, for [`finish_cloths`] when
/// no solver ran this tick. Their order does not matter: the contacts take
/// the parts in the cell index's order and only look the bodies up.
fn walk_bodies<'a>(
    bodies: &'a mut Query<(Entity, &mut Ragdoll)>,
) -> (Vec<Mut<'a, Ragdoll>>, Vec<Mut<'a, Ragdoll>>) {
    let mut awake = Vec::new();
    let mut sleeping = Vec::new();
    for (_, body) in bodies.iter_mut() {
        if body.asleep() {
            sleeping.push(body);
        } else {
            awake.push(body);
        }
    }
    (awake, sleeping)
}

/// What a cloth's end of tick leaves for the sequential end of the stage.
#[derive(Debug)]
struct ClothOutcome {
    after: Option<Vec3>,
    line: Option<crate::cloth::ClothTick>,
    rustle: Option<cloth::ClothRustled>,
    /// The node's rustle flag after the tick, when the tick set it.
    rustling: Option<bool>,
    /// Whether the tick moved the cloth (the frame's write-back takes it).
    stale: bool,
}

/// Runs `f` over the staged cloths: on this thread when fewer than two of
/// them have work (`working`), as the pool's dispatch would cost more than
/// the work, otherwise at once on the compute pool.
fn each_cloth(mut staged: &mut [Staged], working: usize, f: impl Fn(&mut Staged) + Send + Sync) {
    if working < 2 {
        for one in staged.iter_mut() {
            f(one);
        }
        return;
    }
    let pool = bevy::tasks::ComputeTaskPool::get_or_init(bevy::tasks::TaskPool::default);
    use bevy::tasks::ParallelSliceMut;
    staged.par_splat_map_mut(pool, None, |_, chunk| {
        for one in chunk.iter_mut() {
            f(one);
        }
    });
}

/// The stage after the solver (see the module): every cloth's pass after
/// the collisions, its level contact and the end of its tick, the cloths
/// handed back, then the object list's re-buckets, rustles and debug lines
/// in the list's order. A tick with no solver (no body plugin) runs the
/// object contacts here first, against the bodies as they stand.
#[allow(clippy::too_many_arguments)]
pub(crate) fn finish_cloths(
    mut instances: Query<(Entity, &mut ClothInstance, &GlobalTransform)>,
    mut bodies: Query<(Entity, &mut Ragdoll)>,
    (physics, statics): (
        Option<Res<Physics>>,
        Option<Res<exa_physics::StaticColliders>>,
    ),
    mut objects: ResMut<ClothObjects>,
    mut debug: ResMut<ClothDebug>,
    mut stages: ResMut<SolverStages>,
    mut rustles: MessageWriter<cloth::ClothRustled>,
    names: Query<&Name>,
    parents: Query<&ChildOf>,
) {
    let Some(stage) = stages.get_mut::<ClothSolve>() else {
        return;
    };
    let on = stage.on;
    if !stage.contacted && !stage.staged.is_empty() {
        let (mut awake, mut sleeping) = walk_bodies(&mut bodies);
        let mut awake_refs: Vec<&mut Ragdoll> = awake.iter_mut().map(|b| &mut **b).collect();
        // Untouched, as the solver leaves them, so a deep sleeper is not
        // marked changed; one the contact woke is.
        let mut sleeping_refs: Vec<&mut Ragdoll> = sleeping
            .iter_mut()
            .map(|b| b.bypass_change_detection())
            .collect();
        stage.sorted = false;
        stage.contact(&mut awake_refs, &mut sleeping_refs);
        drop((awake_refs, sleeping_refs));
        for body in &mut sleeping {
            if !body.asleep() {
                body.set_changed();
            }
        }
    }
    // The contacts' time is counted apart: the end's own starts here.
    let started = on.then(std::time::Instant::now);
    let live = stage.live;
    let mut staged = std::mem::take(&mut stage.staged);
    // Each cloth's node as it stands now, and its rustle flag; a cloth
    // whose node is gone is dropped.
    staged.retain_mut(|one| {
        let Ok((_, instance, at)) = instances.get(one.entity) else {
            return false;
        };
        one.plan.at = *at;
        one.rustling = instance.rustling;
        true
    });
    let (physics, statics) = (physics.as_deref(), statics.as_deref());
    // As the own steps: fewer than two cloths with a tick to end end here.
    let ticking = staged
        .iter()
        .filter(|s| !s.skipped && s.integrated.is_some())
        .count();
    each_cloth(&mut staged, ticking, |one| {
        one.outcome = Some(finish_one(one, live, physics, statics));
    });
    let mut total = ClothWork::default();
    debug.cloths.clear();
    // The cloths back on their nodes, then the object list's re-bucket
    // after each step, as the original's, in its order.
    for one in &mut staged {
        let Some(outcome) = one.outcome.take() else {
            continue;
        };
        if let Ok((_, mut instance, _)) = instances.get_mut(one.entity) {
            instance.cloth = std::mem::take(&mut one.cloth);
            if outcome.stale {
                // The frame's mesh write-back takes the particles once,
                // after the frame's last tick
                // ([`crate::cloth::write_cloth_meshes`]).
                instance.stale_mesh = true;
            }
            if let Some(rustling) = outcome.rustling {
                instance.rustling = rustling;
            }
        }
        if let Some(after) = outcome.after {
            objects.grid.migrate(one.plan.id, after);
        }
        if let Some(rustle) = outcome.rustle {
            rustles.write(rustle);
        }
        if let Some(line) = outcome.line {
            total.add(&line.1);
            debug.cloths.push(line);
        }
    }
    // The list is kept (emptied) for the next tick.
    staged.clear();
    stage.staged = staged;
    if let Some(t) = started {
        // The parts split is the bucketing (the index and the hook's cells
        // around the cloths), as the cloths' own contacts count with their
        // steps.
        let finish_micros = t.elapsed().as_micros() as u64;
        let touch_micros = stage.contact_micros.saturating_sub(stage.bucket_micros);
        total.rebucketed = stage.rebucketed;
        total.parts_micros = stage.index_micros + stage.bucket_micros;
        total.plan_micros = stage.plan_micros;
        total.cloths_micros = stage.step_micros + touch_micros + finish_micros;
        total.micros = total.parts_micros + total.plan_micros + total.cloths_micros;
        debug.last_micros = total.micros;
        debug.last_parts_micros = total.parts_micros;
        debug.last_split = [
            stage.index_micros,
            stage.plan_micros,
            stage.step_micros,
            stage.bucket_micros,
            touch_micros,
            finish_micros,
        ];
        debug.tick(total, &names, &parents);
    }
}

/// One cloth's end of tick: its pass after the collisions, the level
/// contact and the tick's end (its node's transform is `one.plan.at`, read
/// after the solver).
fn finish_one(
    one: &mut Staged,
    live: bool,
    physics: Option<&Physics>,
    statics: Option<&exa_physics::StaticColliders>,
) -> ClothOutcome {
    let mut after = None;
    let mut rustle = None;
    let mut rustling = None;
    let mut stale = false;
    let plan = &one.plan;
    let at = &plan.at;
    let mut work = one.work;
    if !one.skipped
        && let Some(integrated) = one.integrated.take()
    {
        let cloth = &mut one.cloth;
        work.awake = u32::from(!plan.asleep);
        cloth::after_bodies(cloth, integrated.awake);
        if live && let (Some(physics), Some(statics)) = (physics, statics) {
            let level_started = one.started.map(|_| std::time::Instant::now());
            // Objects against the level, after their step: the static
            // grid's records around the stepped object's cell that pass
            // its pre-test, less the cloth's own node's meshes, each one's
            // triangles within the object's bound box as one piece.
            let (object, _) = cloth_bound(cloth, at);
            let centre_m = object.centre * 0.01;
            let radius_m = object.radius * 0.01;
            let (lo, hi) = (
                centre_m - Vec3::splat(radius_m),
                centre_m + Vec3::splat(radius_m),
            );
            let own_index = plan.own_node.map(|n| n.index().index());
            let mut near = Vec::new();
            let mut triangles = Vec::new();
            let mut contacts = ObjectContacts::default();
            statics.near_object(object.centre, object.radius, &mut near);
            for &record_id in &near {
                let Some(record) = statics.grid().get(record_id) else {
                    continue;
                };
                work.records += 1;
                if own_index.is_some_and(|n| n != 0 && n == record.node) {
                    work.own_records += 1;
                    continue;
                }
                triangles.clear();
                physics.0.triangles(record.piece, lo, hi, &mut triangles);
                // The node mesh's own kd-tree record, else the collider's
                // own triangle count.
                let tree = physics
                    .0
                    .colliders
                    .get_unknown_gen(record.piece)
                    .and_then(|(_, handle)| statics.has_tree(handle))
                    .unwrap_or_else(|| physics.0.has_tree(record.piece));
                contacts.gather_level(&plan.to_object, triangles.iter().copied(), tree);
            }
            work.pieces = contacts.pieces.len() as u32;
            work.triangles = contacts.level.len() as u32;
            cloth.level = std::mem::take(&mut contacts.level);
            cloth.level_pieces = std::mem::take(&mut contacts.pieces);
            cloth_level::touch_level(cloth);
            if let Some(t) = level_started {
                work.level_micros = t.elapsed().as_micros() as u64;
            }
        }
        cloth::finish_step(cloth, &integrated.before, integrated.awake);
        work.level_tests = cloth.level_tests as u64;
        work.sweep_tests = cloth.body_tests as u64;
        if one.started.is_some() {
            let count = cloth.particles.len() as f32;
            let sum = cloth.mean_velocity * count;
            work.velocity2 = sum.length_squared();
            let threshold = (count * 1e-6).sqrt();
            work.thresholds = (threshold, (2.0 * threshold) * (2.0 * threshold));
        }
        // The object list's re-bucket after its step (made in the list's
        // order once every cloth has stepped).
        let (moved_to, _) = cloth_bound(cloth, at);
        after = Some(moved_to.centre);
        stale = true;
        // The rustle loop at the cloth's centre while it plays, and once
        // when it stops.
        if let Some(sound) = cloth.record.rustle
            && (cloth.rustle || one.rustling)
        {
            let playing = cloth.rustle;
            rustling = Some(playing);
            rustle = Some(cloth::ClothRustled {
                cloth: one.entity,
                sound,
                position: moved_to.centre * 0.01,
                volume: cloth.rustle_volume,
                pitch: cloth.rustle_pitch,
                playing,
            });
        }
    }
    let line = one.started.map(|t| {
        work.micros = t.elapsed().as_micros() as u64;
        work.particles = one.cloth.particles.len() as u32;
        work.sleep = one.cloth.sleep;
        work.pins = one.cloth.pins.len() as u32;
        (one.entity, work, std::mem::take(&mut one.pushers))
    });
    ClothOutcome {
        after,
        line,
        rustle,
        rustling,
        stale,
    }
}

#[cfg(test)]
#[path = "cloth_stage_test.rs"]
mod tests;
