//! The level's cloth: every mesh node the RFC marks as cloth (long hair,
//! banners, the wraith's robe) becomes a simulated cloth over its mesh.
//!
//! The original gives a mesh node a cloth controller when its material is
//! flagged as cloth and the node carries the cloth marker; each position
//! element of the mesh is a particle (the vertices a UV seam splits share
//! one), the edges of its triangles are structural links and the two
//! vertices opposite a shared edge are a bending link. The cloth hangs from
//! the pins its controller lists when it starts
//! ([`exa_physics::cloth::face_flag_pins`]): an element is pinned when
//! every face about it carries a pin bit in the mesh's face flags, firmly
//! (weight 1) when each also carries a firm bit, else loosely (0.1); the
//! rest hangs free. Every shipped cloth marks its hem faces firmly, so the
//! hem stays where it was placed and the rest sways under gravity, wind and
//! collisions. A mesh that comes without its file's data (built in code)
//! pins every vertex loosely instead [Exarust's own: nothing ships such a
//! cloth].
//!
//! Each 83 Hz body tick advances the cloth ([`exa_physics::cloth::step`])
//! with gravity; every cloth's contacts and step run at once (a cloth reads
//! only the tick's shared body parts and the level and writes only itself,
//! so each ends bit for bit where it would stepped alone), and the object
//! list's re-buckets stay in its order. Once a frame, after its ticks, the
//! mesh's positions take the particles' ([`write_cloth_meshes`]; the mesh
//! is in metres, the cloth in centimetres). The simulation is local to the
//! mesh's node, so the node's own transform carries the cloth with the
//! body. Evidence: the original's cloth construction (a particle per
//! position element, triangle edges and bend links), its pin list and its
//! step (REA assembly). The draw's two passes are [`crate::cloth_draw`]'s.

use bevy::math::Affine3A;
use bevy::prelude::*;

use crate::cloth_parts::{self, PartCells};
use exa_physics::Physics;
use exa_physics::body::Ragdoll;
use exa_physics::cloth::{self, Cloth};
use exa_physics::contact::Level;
use exa_physics::sector::{self, ObjectBodyPart, ObjectBounds, ObjectContacts};

/// Marks a mesh part whose RFC node carries the cloth marker.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ClothNode;

/// What the model file says about a cloth part: its node's
/// [`exa_assets::ClothMesh`] (position elements, faces and face flags) and
/// which of the node's parts this is.
#[derive(Component, Debug, Clone)]
pub struct ClothSource {
    pub mesh: std::sync::Arc<exa_assets::ClothMesh>,
    /// The part's index in the node's parts.
    pub part: usize,
}

/// The sleep countdown a level placement gives the cloth of this node (the
/// word of its `0x3DE0ECDB` record), as the original's placement load sets
/// the cloth's countdown and, under 1, puts it to sleep. Every shipped
/// record stores 0, so every placed banner starts asleep: it takes only
/// moving bodies, and the sleeping props it hangs among never push it.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClothStartSleep(pub i32);

/// The parameter record a cloth node's cloth is set up from, when it is not
/// the engine's built-in one: the character build hands a hair style's
/// cloth [`exa_physics::cloth_record::ClothRecord::HAIR`].
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct ClothParameters(pub exa_physics::cloth_record::ClothRecord);

/// A cloth node whose mesh cannot be a cloth (no positions, no triangles),
/// so the builder does not look at it again.
#[derive(Component, Debug, Clone, Copy)]
pub struct ClothSkipped;

/// The pin weight a cloth built without its file's face flags takes on
/// every vertex [Exarust's own: the original's loose pin weight].
pub const LOOSE_PIN: f32 = cloth::LOOSE_PIN;

/// A cloth simulated over a mesh part.
#[derive(Component, Debug)]
pub struct ClothInstance {
    pub cloth: Cloth,
    /// The mesh whose vertices the cloth drives.
    pub mesh: Handle<Mesh>,
    /// The particle each mesh vertex draws (the vertices a seam splits
    /// draw one particle).
    pub vertex_particle: Vec<usize>,
    /// The node's world transform at the cloth's last tick: a sleeping
    /// cloth whose frame moved meets the level again.
    pub frame: Option<Affine3A>,
    /// The cloth stepped since its mesh last took its particles: the
    /// frame's write-back ([`write_cloth_meshes`]) is owed.
    pub stale_mesh: bool,
}

impl ClothInstance {
    /// A cloth over `mesh`, whose vertex `i` draws particle
    /// `vertex_particle[i]`.
    pub fn new(cloth: Cloth, mesh: Handle<Mesh>, vertex_particle: Vec<usize>) -> Self {
        Self {
            cloth,
            mesh,
            vertex_particle,
            frame: None,
            stale_mesh: false,
        }
    }
}

/// The sector's live simulated-object list: every cloth of the level in the
/// original's 600 cm object grid, in a stable entity order (the order the
/// original's object list keeps, oldest first). A cloth migrates to its
/// new cell after each tick.
#[derive(Resource, Debug)]
pub struct ClothObjects {
    /// The cloths, oldest entity first.
    pub order: Vec<Entity>,
    /// Their cells.
    pub grid: sector::ObjectGrid,
}

impl Default for ClothObjects {
    fn default() -> Self {
        Self::new()
    }
}

impl ClothObjects {
    /// Columns of the object grid: wide enough for a level's cells.
    pub const COLUMNS: i32 = 4096;
    /// Rows of the object grid.
    pub const ROWS: i32 = 4096;
    /// The grid's origin cell, added to a point's cell so the world origin
    /// falls in the grid's middle (cell index 2048 · 4096 + 2048). A
    /// negative origin would put every object below the first row, where
    /// the overflow guard parks it.
    pub const ORIGIN: (i32, i32) = (2048, 2048);

    /// An empty list.
    pub fn new() -> Self {
        Self {
            order: Vec::new(),
            grid: sector::ObjectGrid::new(Self::COLUMNS, Self::ROWS, Self::ORIGIN),
        }
    }
}

/// Whether the level's cloths gather body and level contacts each tick.
/// On by default; `EXA_CLOTH_CONTACTS=0` turns them off. The rules are the
/// original's (a cloth never meets its own node's mesh, the level never
/// wakes it and its slide stops it, a sleeping cloth takes only awake
/// bodies whose padded spheres reach its box, a body pushes the cloth's
/// triangles and trades countdowns with it, so a sleeping prop never holds
/// it awake). With the face-flag pins and the placed cloths starting asleep,
/// exanima01's 14 cloths sleep from the load on the RTX 3090 (the stage
/// about 1.6 ms a tick, 59 FPS); [`ClothDebug`] shows where the time goes.
#[derive(Resource, Debug, Clone, Copy)]
pub struct ClothContacts(pub bool);

impl Default for ClothContacts {
    fn default() -> Self {
        Self(std::env::var_os("EXA_CLOTH_CONTACTS").is_none_or(|v| v != "0"))
    }
}

/// The cloth stage's debug counters (`EXA_DEBUG_CLOTH=1`): once a second
/// (83 ticks) it logs the per-tick means of the stage's work, the stage's
/// split (bucketing the parts, the sequential per-cloth plan, the wall time
/// of the cloths' contacts and steps with their CPU time by part), the
/// frame's mesh write-back and the whole fixed tick's time, then one line
/// per cloth for the last tick, so a cost can be traced to cloths that
/// never sleep, to many bodies, to many level triangles, or to work outside
/// the stage.
#[derive(Resource, Debug, Default)]
pub struct ClothDebug {
    /// Whether the counters run.
    pub on: bool,
    /// Ticks summed since the last log.
    pub ticks: u32,
    /// Sums over those ticks.
    pub sum: ClothWork,
    /// Each cloth's work in the last tick, with the body parts that
    /// pushed it then (and their bodies' countdowns).
    pub cloths: Vec<ClothTick>,
    /// The stage's time in the last tick (µs).
    pub last_micros: u64,
    /// Its parts split in the last tick (µs): bucketing the body parts.
    pub last_parts_micros: u64,
    /// Frames the mesh write-back ran since the last log, the meshes it
    /// wrote and its time (µs).
    pub frames: u32,
    pub meshes_written: u32,
    pub write_micros: u64,
    /// Fixed ticks timed since the last log and their summed time (µs):
    /// the whole fixed schedule, to set the stage against.
    pub fixed_ticks: u32,
    pub fixed_micros: u64,
    /// When the current fixed tick started.
    pub fixed_started: Option<std::time::Instant>,
}

/// One cloth's line of the debug log: the cloth, its work in the last tick
/// and the body parts that pushed it then, with their bodies' countdowns.
pub type ClothTick = (Entity, ClothWork, Vec<(Entity, i32)>);

/// What the cloth stage did in one tick (or one cloth's share of it).
#[derive(Debug, Default, Clone, Copy)]
pub struct ClothWork {
    /// Cloths that stepped awake.
    pub awake: u32,
    /// Cloths asleep at the stage's start.
    pub asleep: u32,
    /// Sleeping cloths that skipped the stage (still frame, no body).
    pub skipped: u32,
    /// Body parts offered after the sleep and own-character filters.
    pub parts: u32,
    /// Of those, the parts handed to the cloth (a padded sphere reaches
    /// its box).
    pub bodies: u32,
    /// Body spheres kept (standing and swept).
    pub spheres: u32,
    /// Of those, the swept ones.
    pub swept: u32,
    /// Cloth-triangle records the body stage's swept query looked at.
    pub sweep_tests: u64,
    /// Static records found around the cloth.
    pub records: u32,
    /// Records skipped as the cloth's own node's.
    pub own_records: u32,
    /// Level pieces handed over.
    pub pieces: u32,
    /// Level triangles handed over.
    pub triangles: u32,
    /// Particle-triangle tests the level stage made.
    pub level_tests: u64,
    /// The cloth's particles.
    pub particles: u32,
    /// The cloth's countdown after its tick.
    pub sleep: i32,
    /// The cloth's pinned particles.
    pub pins: u32,
    /// The squared length of the summed particle velocity its countdown
    /// read this tick (cm² a tick²).
    pub velocity2: f32,
    /// The countdown's thresholds for that sum: under the first it counts
    /// down, over the second up.
    pub thresholds: (f32, f32),
    /// Time in the stage (µs).
    pub micros: u64,
    /// Of that, gathering the body contacts (µs, summed over the cloths,
    /// which run at once).
    pub body_micros: u64,
    /// Gathering the level's triangles (µs, summed over the cloths).
    pub level_micros: u64,
    /// The cloth's step itself (µs, summed over the cloths).
    pub step_micros: u64,
    /// The stage's total only: the bodies the part index re-bucketed (the
    /// changed ones) and the parts it gathered around the cloths.
    pub rebucketed: u32,
    pub gathered: u32,
    /// The stage's sequential parts (the stage's total only): bucketing
    /// the body parts (re-bucketing the changed bodies, then gathering the
    /// parts around the cloths), then each cloth's frame, owner and object-list
    /// re-bucket, then the wall time of every cloth's contacts and step.
    pub parts_micros: u64,
    pub plan_micros: u64,
    pub cloths_micros: u64,
}

impl ClothWork {
    fn add(&mut self, other: &Self) {
        self.awake += other.awake;
        self.asleep += other.asleep;
        self.skipped += other.skipped;
        self.parts += other.parts;
        self.bodies += other.bodies;
        self.spheres += other.spheres;
        self.swept += other.swept;
        self.sweep_tests += other.sweep_tests;
        self.records += other.records;
        self.own_records += other.own_records;
        self.pieces += other.pieces;
        self.triangles += other.triangles;
        self.level_tests += other.level_tests;
        self.particles += other.particles;
        self.micros += other.micros;
        self.body_micros += other.body_micros;
        self.level_micros += other.level_micros;
        self.step_micros += other.step_micros;
        self.rebucketed += other.rebucketed;
        self.gathered += other.gathered;
        self.parts_micros += other.parts_micros;
        self.plan_micros += other.plan_micros;
        self.cloths_micros += other.cloths_micros;
    }
}

impl ClothDebug {
    /// The counters, on when `EXA_DEBUG_CLOTH` is set and not `0`.
    pub fn from_env() -> Self {
        Self {
            on: std::env::var_os("EXA_DEBUG_CLOTH").is_some_and(|v| v != "0"),
            ..Self::default()
        }
    }

    /// Adds one tick and, once a second, logs and resets.
    fn tick(&mut self, total: ClothWork, names: &Query<&Name>, parents: &Query<&ChildOf>) {
        self.sum.add(&total);
        self.ticks += 1;
        if self.ticks < 83 {
            return;
        }
        let n = self.ticks as f64;
        let s = &self.sum;
        let ms = |micros: u64, count: f64| micros as f64 / count.max(1.0) / 1000.0;
        info!(
            "cloth stage split per tick: parts {:.2} ms ({:.1} bodies re-bucketed, {:.1} parts gathered), plan {:.2} ms, cloths {:.2} ms wall (CPU: bodies {:.2} ms, level {:.2} ms, step {:.2} ms); mesh write-back {:.2} ms a frame ({:.1} meshes, {:.2} ticks a frame); fixed schedule {:.2} ms a tick",
            ms(s.parts_micros, n),
            s.rebucketed as f64 / n,
            s.gathered as f64 / n,
            ms(s.plan_micros, n),
            ms(s.cloths_micros, n),
            ms(s.body_micros, n),
            ms(s.level_micros, n),
            ms(s.step_micros, n),
            ms(self.write_micros, f64::from(self.frames)),
            f64::from(self.meshes_written) / f64::from(self.frames.max(1)),
            n / f64::from(self.frames.max(1)),
            ms(self.fixed_micros, f64::from(self.fixed_ticks)),
        );
        info!(
            "cloth stage per tick: {:.2} ms, awake {:.1}, asleep {:.1}, skipped {:.1}, parts {:.1}, bodies {:.1}, spheres {:.1}, swept {:.1}, body tests {:.0}, records {:.1} (own {:.1}), pieces {:.1}, triangles {:.0}, level tests {:.0}",
            s.micros as f64 / n / 1000.0,
            s.awake as f64 / n,
            s.asleep as f64 / n,
            s.skipped as f64 / n,
            s.parts as f64 / n,
            s.bodies as f64 / n,
            s.spheres as f64 / n,
            s.swept as f64 / n,
            s.sweep_tests as f64 / n,
            s.records as f64 / n,
            s.own_records as f64 / n,
            s.pieces as f64 / n,
            s.triangles as f64 / n,
            s.level_tests as f64 / n,
        );
        // The nearest named node over an entity.
        let name_of = |entity: Entity| {
            let mut at = entity;
            for _ in 0..8 {
                if let Ok(n) = names.get(at) {
                    return n.as_str().to_owned();
                }
                let Ok(parent) = parents.get(at) else { break };
                at = parent.parent();
            }
            String::from("?")
        };
        for (entity, w, pushers) in &self.cloths {
            let name = name_of(*entity);
            let pushed: Vec<String> = pushers
                .iter()
                .map(|(part, countdown)| format!("'{}' {part} ({countdown})", name_of(*part)))
                .collect();
            info!(
                "  cloth {entity} '{name}': sleep {}, velocity sum² {:.6} (down < {:.6}, up > {:.6}), particles {}, pins {}, {} us, parts {}, bodies {}, spheres {}, swept {}, body tests {}, records {} (own {}), pieces {}, triangles {}, level tests {}, pushed by [{}]",
                w.sleep,
                w.velocity2,
                w.thresholds.0,
                w.thresholds.1,
                w.particles,
                w.pins,
                w.micros,
                w.parts,
                w.bodies,
                w.spheres,
                w.swept,
                w.sweep_tests,
                w.records,
                w.own_records,
                w.pieces,
                w.triangles,
                w.level_tests,
                pushed.join(", "),
            );
        }
        self.ticks = 0;
        self.sum = ClothWork::default();
        self.frames = 0;
        self.meshes_written = 0;
        self.write_micros = 0;
        self.fixed_ticks = 0;
        self.fixed_micros = 0;
    }
}

/// A cloth's world bound in centimetres, from its particles' local bounds
/// and its node's transform: the centre and the sphere around the box.
fn cloth_bound(cloth: &Cloth, at: &GlobalTransform) -> (ObjectBounds, Affine3A) {
    let (centre, half) = cloth::bounds(&cloth.particles);
    let scale = at.scale().abs().max_element();
    let world = at.transform_point(centre * 0.01);
    (
        ObjectBounds {
            centre: world * 100.0,
            // The particles are already in centimetres.
            radius: half.length() * scale,
        },
        at.affine().inverse(),
    )
}

/// One vertex's pin index and weight from its skin weights: a vertex with a
/// zero weight slot (or with no skin at all) is pinned, at the loose weight.
pub fn pins_from_skin(vertices: usize, skin: Option<&[[f32; 4]]>) -> Vec<(usize, f32)> {
    match skin {
        None => (0..vertices).map(|i| (i, LOOSE_PIN)).collect(),
        Some(weights) => weights
            .iter()
            .take(vertices)
            .enumerate()
            .filter(|(_, w)| w.contains(&0.0))
            .map(|(i, _)| (i, LOOSE_PIN))
            .collect(),
    }
}

/// Builds the cloth of a marked mesh from its positions (metres) and
/// triangles, pinned as [`pins_from_skin`] says.
pub fn cloth_of(vertices: &[Vec3], triangles: &[[usize; 3]], pins: &[(usize, f32)]) -> Cloth {
    let cm: Vec<Vec3> = vertices.iter().map(|v| *v * 100.0).collect();
    let indices: Vec<usize> = pins.iter().map(|(i, _)| *i).collect();
    let mut cloth = Cloth::from_mesh(&cm, triangles, &indices);
    for (pin, (_, weight)) in cloth.pins.iter_mut().zip(pins) {
        pin.weight = *weight;
    }
    cloth
}

/// Builds the cloth of a marked mesh part from its file's data, as the
/// original's controller does: one particle per position element the part
/// uses (at its first vertex, `vertices` in metres), the part's triangles
/// over those particles, and the pins `pins` lists by element (from
/// [`cloth::face_flag_pins`] over the node's faces; an element the part
/// does not use is skipped). `elements` is each vertex's position element.
/// Returns the cloth and the particle each vertex draws.
pub fn cloth_of_elements(
    vertices: &[Vec3],
    triangles: &[[usize; 3]],
    elements: &[u32],
    pins: &[(usize, f32)],
) -> (Cloth, Vec<usize>) {
    let mut particle_of = std::collections::HashMap::<u32, usize>::new();
    let mut points = Vec::new();
    let vertex_particle: Vec<usize> = vertices
        .iter()
        .enumerate()
        .map(|(vertex, &position)| {
            let element = elements
                .get(vertex)
                .copied()
                .unwrap_or(u32::MAX - vertex as u32);
            *particle_of.entry(element).or_insert_with(|| {
                points.push(position);
                points.len() - 1
            })
        })
        .collect();
    let welded: Vec<[usize; 3]> = triangles
        .iter()
        .filter(|t| t.iter().all(|&i| i < vertex_particle.len()))
        .map(|t| t.map(|i| vertex_particle[i]))
        .collect();
    let held: Vec<(usize, f32)> = pins
        .iter()
        .filter_map(|&(element, weight)| {
            let element = u32::try_from(element).ok()?;
            particle_of.get(&element).map(|&p| (p, weight))
        })
        .collect();
    (cloth_of(&points, &welded, &held), vertex_particle)
}

/// The countdown a placement's word gives a cloth: the word itself, or
/// asleep (0) under 1, as the original sets the countdown and marks the
/// cloth asleep under 1.
pub fn start_countdown(word: i32) -> i32 {
    if word < 1 { 0 } else { word }
}

/// Cloth nodes still waiting for their cloth: not built, not given up on.
type PendingCloth = (
    With<ClothNode>,
    Without<ClothInstance>,
    Without<ClothSkipped>,
);

/// A pending cloth node's mesh, file data and parameter record.
type PendingClothPart<'a> = (
    Entity,
    &'a Mesh3d,
    Option<&'a ClothSource>,
    Option<&'a ClothParameters>,
);

/// Gives each cloth node its cloth, once its mesh is in place (a skinned
/// part gets its mesh when it is bound to its skeleton, after it spawns).
///
/// The cloth moves its mesh's vertices, and one mesh asset is shared by
/// every copy of a model, so each cloth takes a copy of the mesh for its
/// node alone.
fn build_cloth_nodes(
    mut commands: Commands,
    nodes: Query<PendingClothPart, PendingCloth>,
    starts: Query<&ClothStartSleep>,
    parents: Query<&ChildOf>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (entity, mesh, source, parameters) in &nodes {
        let Some(mesh_data) = meshes.get(&mesh.0) else {
            continue;
        };
        let Some(positions) = mesh_data
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|a| a.as_float3())
        else {
            commands.entity(entity).insert(ClothSkipped);
            continue;
        };
        let Some(indices) = mesh_data.indices() else {
            commands.entity(entity).insert(ClothSkipped);
            continue;
        };
        let indices: Vec<usize> = indices.iter().collect();
        let vertices: Vec<Vec3> = positions.iter().map(|p| Vec3::from_array(*p)).collect();
        let triangles: Vec<[usize; 3]> = indices
            .as_chunks::<3>()
            .0
            .iter()
            .filter(|c| c.iter().all(|&i| i < vertices.len()))
            .copied()
            .collect();
        if triangles.is_empty() {
            commands.entity(entity).insert(ClothSkipped);
            continue;
        }
        // The file's position elements and face flags, when the part's
        // vertices are the ones they describe.
        let elements = source.and_then(|source| {
            let part = source.mesh.part_elements.get(source.part)?;
            (part.len() == vertices.len()).then_some((part, &source.mesh))
        });
        let (cloth, vertex_particle) = match elements {
            Some((part, file)) => {
                let pins = cloth::face_flag_pins(file.elements, &file.faces, &file.face_flags);
                cloth_of_elements(&vertices, &triangles, part, &pins)
            }
            None => {
                let pins = pins_from_skin(vertices.len(), None);
                (
                    cloth_of(&vertices, &triangles, &pins),
                    (0..vertices.len()).collect(),
                )
            }
        };
        // A placed cloth starts with the countdown its placement gives it
        // (on the part's node).
        let start = starts.get(entity).ok().or_else(|| {
            let node = parents.get(entity).ok()?.parent();
            starts.get(node).ok()
        });
        let mut cloth = cloth;
        if let Some(&ClothParameters(record)) = parameters {
            cloth.record = record;
        }
        if let Some(&ClothStartSleep(count)) = start {
            cloth.sleep = start_countdown(count);
        }
        let copy = mesh_data.clone();
        let own = meshes.add(copy);
        commands.entity(entity).insert((
            Mesh3d(own.clone()),
            ClothInstance::new(cloth, own, vertex_particle),
        ));
    }
}

/// What the sequential half of the stage settles for one cloth before its
/// contacts and step, which run for every cloth at once.
struct ClothPlan {
    /// The cloth's id in the object list.
    id: usize,
    /// Its cell after the object list's pre-step re-bucket.
    cell: (i32, i32),
    /// Its world bound and the world-to-cloth transform.
    object: ObjectBounds,
    to_object: Affine3A,
    /// The cloth's own character: every node above it.
    own: Vec<Entity>,
    /// The scene node whose static mesh the cloth is drawn from.
    own_node: Option<Entity>,
    /// A particle-mode cloth's owner (the nearest body part above it).
    owner: Option<Entity>,
    /// The node's frame this tick, and whether it moved since the last.
    frame: Affine3A,
    moved: bool,
    /// Whether the cloth was asleep at the stage's start.
    asleep: bool,
    /// Whether the cloth's record is the particle mode.
    particle_mode: bool,
}

/// What a cloth's contacts and step leave for the sequential end of the
/// stage: its centre after the step (when it stepped) and its debug line.
struct ClothOutcome {
    index: usize,
    after: Option<Vec3>,
    line: Option<ClothTick>,
}

/// A body part offered to the cloths this tick: its entity, whether its
/// body sleeps, whether its body takes object contacts at all (a disabled
/// body, an item on the cursor, does not), and its contact data.
type OfferedPart<'a> = (Entity, bool, bool, ObjectBodyPart<'a>);

/// Whether a body takes the cloths' body contacts this tick, as the
/// original's object-body stage masks it: a disabled body (state bit 4) is
/// skipped, and its sleep decides between the two masks. The original also
/// skips a body whose state carries the held-static-only bit (0x800);
/// Exarust keeps that word on a held item's parts rather than its body, so
/// only the disabled bit is read here [guess].
pub fn body_meets_objects(body: &Ragdoll) -> bool {
    !body.disabled()
}

#[allow(clippy::too_many_arguments)]
fn step_cloths(
    mut instances: Query<(Entity, &mut ClothInstance, &GlobalTransform)>,
    bodies: Query<(Entity, Ref<Ragdoll>)>,
    parents: Query<&ChildOf>,
    scene_nodes: Query<(), With<exa_physics::StaticNode>>,
    physics: Option<Res<Physics>>,
    statics: Option<Res<exa_physics::StaticColliders>>,
    mut objects: ResMut<ClothObjects>,
    live_contacts: Res<ClothContacts>,
    mut debug: ResMut<ClothDebug>,
    names: Query<&Name>,
    mut part_cells: Local<sector::CellGrid>,
    mut index: Local<PartCells>,
) {
    let on = debug.on;
    let started = on.then(std::time::Instant::now);
    let mut total = ClothWork::default();
    debug.cloths.clear();
    let gravity = cloth::gravity_cm(1.0);
    // The object list's stable order: the entity id, oldest first. A new or
    // removed cloth rebuilds the grid; otherwise each object migrates after
    // its tick, as the original's list does.
    let mut order: Vec<Entity> = instances.iter().map(|(entity, ..)| entity).collect();
    order.sort_unstable();
    let rebuilt = objects.order != order;
    if rebuilt {
        objects.order = order;
        objects.grid = sector::ObjectGrid::new(
            ClothObjects::COLUMNS,
            ClothObjects::ROWS,
            ClothObjects::ORIGIN,
        );
    }
    // Every part of every body (asleep too) that can meet an object, linked
    // in the grid cell its mesh box's world centre falls in, as the
    // sector's body grid links it: an object is offered only the parts in
    // the nine cells around its own ([`sector::CellGrid::around`]). The
    // index re-buckets only the bodies changed since the last tick (a deep
    // sleeper's parts stay where they were), and nothing while no cloth is
    // simulated ([`PartCells`]).
    if objects.order.is_empty() {
        index.clear();
    } else {
        index.refresh(
            bodies
                .iter()
                .map(|(entity, body)| (entity, body.is_changed(), body.into_inner())),
            ClothObjects::ORIGIN,
        );
    }
    let indexed = on.then(std::time::Instant::now);
    // The sequential half, in the object list's order: each cloth's frame,
    // owner and own character, its carry into the frame, and the object
    // list's re-bucket before its step. It touches the shared object list,
    // so it keeps the list's order.
    let mut plans: bevy::ecs::entity::EntityHashMap<(usize, ClothPlan)> =
        bevy::ecs::entity::EntityHashMap::default();
    let order = std::mem::take(&mut objects.order);
    for (id, &entity) in order.iter().enumerate() {
        let Ok((_, mut instance, at)) = instances.get_mut(entity) else {
            continue;
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
            .unwrap_or_else(|| sector::cell_of(object.centre, ClothObjects::ORIGIN));
        let asleep = instance.cloth.sleep <= 0;
        plans.insert(
            entity,
            (
                id,
                ClothPlan {
                    id,
                    cell,
                    object,
                    to_object,
                    own,
                    own_node,
                    owner: owner.map(|(e, _)| e),
                    frame,
                    moved,
                    asleep,
                    particle_mode,
                },
            ),
        );
    }
    objects.order = order;
    let planned = on.then(std::time::Instant::now);
    // The tick's parts the cloths can be offered: those linked in the nine
    // cells around some cloth, each cell's in the order a full re-bucket
    // links them, with their bodies' sleep and contact data.
    let mut all_parts: Vec<OfferedPart> = Vec::new();
    part_cells.clear();
    if live_contacts.0 && !plans.is_empty() {
        let mut cells = Vec::new();
        cloth_parts::cells_around(plans.values().map(|(_, plan)| plan.cell), &mut cells);
        let mut linked = Vec::new();
        for &cell in &cells {
            index.sorted_in(cell, &mut linked);
            for at in &linked {
                let Ok((_, body)) = bodies.get(at.body) else {
                    continue;
                };
                let body = body.into_inner();
                let Some(part) = body.parts.get(at.part) else {
                    continue;
                };
                let now = part.transform();
                part_cells.insert(cell, all_parts.len());
                all_parts.push((
                    part.entity,
                    body.asleep(),
                    body_meets_objects(body),
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
                    },
                ));
            }
        }
    }
    let gathered = on.then(std::time::Instant::now);
    // Each cloth's contacts and step, every cloth at once: a cloth reads
    // only the tick's shared body parts, the static grid and the level, and
    // writes only itself, so the order they run in changes nothing.
    let outcomes = std::sync::Mutex::new(Vec::with_capacity(plans.len()));
    let shared = StepShared {
        all_parts: &all_parts,
        part_cells: &part_cells,
        physics: physics.as_deref(),
        statics: statics.as_deref(),
        live_contacts: live_contacts.0,
        gravity,
        on,
    };
    instances
        .par_iter_mut()
        .batching_strategy(bevy::ecs::batching::BatchingStrategy::fixed(1))
        .for_each(|(entity, mut instance, at)| {
            let Some((index, plan)) = plans.get(&entity) else {
                return;
            };
            let outcome = step_one(&shared, *index, entity, plan, &mut instance, at);
            if let Ok(mut outcomes) = outcomes.lock() {
                outcomes.push(outcome);
            }
        });
    let stepped = on.then(std::time::Instant::now);
    let mut outcomes = outcomes.into_inner().unwrap_or_default();
    outcomes.sort_unstable_by_key(|o| o.index);
    // The object list's re-bucket after each step, as the original's, in
    // its order.
    for outcome in outcomes {
        if let Some(after) = outcome.after {
            objects.grid.migrate(outcome.index, after);
        }
        if let Some(line) = outcome.line {
            total.add(&line.1);
            debug.cloths.push(line);
        }
    }
    if let (Some(t), Some(indexed), Some(planned), Some(gathered), Some(stepped)) =
        (started, indexed, planned, gathered, stepped)
    {
        let micros = |a: std::time::Instant, b: std::time::Instant| {
            b.saturating_duration_since(a).as_micros() as u64
        };
        total.parts_micros = micros(t, indexed) + micros(planned, gathered);
        total.rebucketed = index.rebucketed as u32;
        total.gathered = all_parts.len() as u32;
        total.plan_micros = micros(indexed, planned);
        total.cloths_micros = micros(gathered, stepped);
        total.micros = t.elapsed().as_micros() as u64;
        debug.last_micros = total.micros;
        debug.last_parts_micros = total.parts_micros;
        debug.tick(total, &names, &parents);
    }
}

/// What every cloth's contacts and step read: the tick's body parts and
/// their cells, the level and its static grid.
struct StepShared<'a, 'p> {
    all_parts: &'a [OfferedPart<'p>],
    part_cells: &'a sector::CellGrid,
    physics: Option<&'a Physics>,
    statics: Option<&'a exa_physics::StaticColliders>,
    live_contacts: bool,
    gravity: Vec3,
    on: bool,
}

/// One cloth's contacts and step: the body parts in the nine cells around
/// its own and the level's triangles about it, then the cloth's tick.
fn step_one(
    shared: &StepShared,
    index: usize,
    entity: Entity,
    plan: &ClothPlan,
    instance: &mut ClothInstance,
    at: &GlobalTransform,
) -> ClothOutcome {
    let on = shared.on;
    let cloth_started = on.then(std::time::Instant::now);
    let mut work = ClothWork::default();
    let mut offered_parts: Vec<Entity> = Vec::new();
    let mut pushers = Vec::new();
    let mut after = None;
    let ClothPlan {
        id,
        cell,
        object,
        to_object,
        ref own,
        own_node,
        owner,
        frame,
        moved,
        asleep,
        particle_mode,
    } = *plan;
    debug_assert_eq!(id, index);
    work.asleep = u32::from(asleep);
    'cloth: {
        let mut contacts = ObjectContacts::default();
        if let (true, Some(physics), Some(statics)) =
            (shared.live_contacts, shared.physics, shared.statics)
        {
            let bodies_started = on.then(std::time::Instant::now);
            // Objects against bodies: the parts the grid sector's object
            // stage walks (those bucketed in the nine cells around the
            // object's own), passing its pre-test
            // ([`sector::object_body_meet`]); a sleeping cloth takes awake
            // bodies only. The particle mode takes its owner's character
            // too (its owner's spheres without padding); any other cloth
            // skips its own character.
            let object_box = cloth::bounds(&instance.cloth.particles);
            let all_parts = shared.all_parts;
            let offered: Vec<usize> = shared
                .part_cells
                .around(cell)
                .filter(|&index| {
                    let (part_entity, body_asleep, meets, _) = &all_parts[index];
                    *meets
                        && sector::object_takes_body(asleep, *body_asleep)
                        && (particle_mode || !own.contains(part_entity))
                })
                .collect();
            if on {
                offered_parts = offered.iter().map(|&index| all_parts[index].0).collect();
            }
            work.parts = offered.len() as u32;
            if particle_mode {
                let parts = offered.iter().map(|&index| {
                    let (part_entity, _, _, part) = all_parts[index];
                    (part, owner == Some(part_entity))
                });
                contacts.gather_particle_bodies(&to_object, object, parts);
                work.bodies = contacts.particle_bodies.len() as u32;
            } else {
                let parts = offered.iter().map(|&index| all_parts[index].3);
                contacts.gather_bodies(&to_object, object, object_box, parts);
                work.bodies = contacts.bodies.len() as u32;
            }
            for body in &contacts.bodies {
                work.spheres += body.spheres.len() as u32;
                work.swept += body.spheres.iter().filter(|s| s.moving).count() as u32;
            }
            if let Some(t) = bodies_started {
                work.body_micros = t.elapsed().as_micros() as u64;
            }
            // A sleeping cloth that no body reaches, in a frame that did
            // not move, is the level stage's fixed point: nothing to do.
            if asleep && !moved && contacts.bodies.is_empty() && contacts.particle_bodies.is_empty()
            {
                work.skipped = 1;
                break 'cloth;
            }
            let level_started = on.then(std::time::Instant::now);
            // Objects against the level: the static grid's records around
            // the object's cell that pass its pre-test, less the cloth's own
            // node's meshes, each one's triangles within the object's bound
            // box as one piece.
            let centre_m = object.centre * 0.01;
            let radius_m = object.radius * 0.01;
            let (lo, hi) = (
                centre_m - Vec3::splat(radius_m),
                centre_m + Vec3::splat(radius_m),
            );
            let own_index = own_node.map(|n| n.index().index());
            let mut near = Vec::new();
            let mut triangles = Vec::new();
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
                contacts.gather_level(&to_object, triangles.iter().copied());
            }
            work.pieces = contacts.pieces.len() as u32;
            work.triangles = contacts.level.len() as u32;
            if let Some(t) = level_started {
                work.level_micros = t.elapsed().as_micros() as u64;
            }
        } else if asleep {
            // A sleeping cloth with no contacts does not step.
            break 'cloth;
        }
        instance.cloth.bodies = std::mem::take(&mut contacts.bodies);
        instance.cloth.particle_bodies = std::mem::take(&mut contacts.particle_bodies);
        instance.cloth.level = std::mem::take(&mut contacts.level);
        instance.cloth.level_pieces = std::mem::take(&mut contacts.pieces);
        work.awake = u32::from(!asleep);
        // A cloth that follows an owner falls along the world's gravity in
        // its node's frame.
        let gravity = if owner.is_some() {
            frame.inverse().transform_vector3(shared.gravity)
        } else {
            shared.gravity
        };
        let step_started = on.then(std::time::Instant::now);
        cloth::step(&mut instance.cloth, gravity);
        if let Some(t) = step_started {
            work.step_micros = t.elapsed().as_micros() as u64;
        }
        work.level_tests = instance.cloth.level_tests as u64;
        work.sweep_tests = instance.cloth.body_tests as u64;
        if on {
            let count = instance.cloth.particles.len() as f32;
            let sum = instance.cloth.mean_velocity * count;
            work.velocity2 = sum.length_squared();
            let threshold = (count * 1e-6).sqrt();
            work.thresholds = (threshold, (2.0 * threshold) * (2.0 * threshold));
            for &index in &instance.cloth.pushed_by {
                let tagged = if particle_mode {
                    instance
                        .cloth
                        .particle_bodies
                        .get(index)
                        .map(|b| (b.part, b.countdown))
                } else {
                    instance
                        .cloth
                        .bodies
                        .get(index)
                        .map(|b| (b.part, b.countdown))
                };
                let Some((part, countdown)) = tagged else {
                    continue;
                };
                if let Some(&part) = offered_parts.get(part) {
                    pushers.push((part, countdown));
                }
            }
        }
        // The object list's re-bucket after its step (made in the list's
        // order once every cloth has stepped).
        let (moved_to, _) = cloth_bound(&instance.cloth, at);
        after = Some(moved_to.centre);
        // The frame's mesh write-back takes the particles once, after the
        // frame's last tick ([`write_cloth_meshes`]).
        instance.stale_mesh = true;
    }
    let line = cloth_started.map(|t| {
        work.micros = t.elapsed().as_micros() as u64;
        work.particles = instance.cloth.particles.len() as u32;
        work.sleep = instance.cloth.sleep;
        work.pins = instance.cloth.pins.len() as u32;
        (entity, work, pushers)
    });
    ClothOutcome { index, after, line }
}

/// Writes each cloth that stepped since the last frame into its mesh's
/// positions (the mesh in metres, the cloth in centimetres), once a frame
/// after the fixed ticks: the renderer takes one copy of the mesh a frame
/// whatever the ticks, so writing it every tick only repeated the work.
pub fn write_cloth_meshes(
    mut instances: Query<&mut ClothInstance>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut debug: ResMut<ClothDebug>,
) {
    let started = debug.on.then(std::time::Instant::now);
    let mut written = 0u32;
    for mut instance in &mut instances {
        if !instance.stale_mesh {
            continue;
        }
        instance.stale_mesh = false;
        let Some(mut mesh) = meshes.get_mut(&instance.mesh) else {
            continue;
        };
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(values)) =
            mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
        else {
            continue;
        };
        let particles = &instance.cloth.particles;
        for (slot, &p) in values.iter_mut().zip(&instance.vertex_particle) {
            *slot = particles
                .get(p)
                .map_or(Vec3::ZERO, |p| p.position * 0.01)
                .to_array();
        }
        written += 1;
    }
    if let Some(t) = started {
        debug.frames += 1;
        debug.meshes_written += written;
        debug.write_micros += t.elapsed().as_micros() as u64;
    }
}

/// Re-expresses a cloth's particles (positions and last positions, cm)
/// from the node frame `last` in the node frame `now` (both world, metres),
/// so they keep their world places while the node moves under them; the
/// pins' rest points stay in the node's frame and move with it.
/// [Exarust's frame bookkeeping: the original keeps a cloth's particles in
/// the world and moves its pinned rest points with the node; Exarust keeps
/// both in the node's frame, which is the same motion.]
pub fn carry_into_frame(cloth: &mut Cloth, last: Affine3A, now: Affine3A) {
    let change = now.inverse() * last;
    let carry = Affine3A {
        matrix3: change.matrix3,
        translation: change.translation * 100.0,
    };
    for particle in &mut cloth.particles {
        particle.position = carry.transform_point3(particle.position);
        particle.previous = carry.transform_point3(particle.previous);
    }
}

/// Starts the debug counters' timer on the whole fixed tick.
fn start_fixed_timer(mut debug: ResMut<ClothDebug>) {
    if debug.on {
        debug.fixed_started = Some(std::time::Instant::now());
    }
}

/// Adds the whole fixed tick's time to the debug counters, so the log
/// sets the cloth stage against the schedule it runs in.
fn stop_fixed_timer(mut debug: ResMut<ClothDebug>) {
    if let Some(t) = debug.fixed_started.take() {
        debug.fixed_ticks += 1;
        debug.fixed_micros += t.elapsed().as_micros() as u64;
    }
}

/// Simulates the cloth of every cloth-marked mesh part.
pub struct ClothPlugin;

impl Plugin for ClothPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ClothObjects>()
            .init_resource::<ClothContacts>()
            .insert_resource(ClothDebug::from_env())
            .add_systems(Update, build_cloth_nodes)
            // The sector runs its simulated objects after the bodies' tick
            // (the original's object step and contacts are inside it).
            .add_systems(
                FixedUpdate,
                step_cloths.after(exa_physics::body::BodyPostStep),
            )
            // The meshes take the particles once a frame, after its ticks.
            .add_systems(
                RunFixedMainLoop,
                write_cloth_meshes.in_set(RunFixedMainLoopSystems::AfterFixedMainLoop),
            )
            .add_systems(FixedFirst, start_fixed_timer)
            .add_systems(FixedLast, stop_fixed_timer);
    }
}

#[cfg(test)]
#[path = "cloth_cost_test.rs"]
mod cost_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unskinned_cloth_pins_every_vertex_loosely() {
        let pins = pins_from_skin(4, None);
        assert_eq!(pins.len(), 4);
        assert!(pins.iter().all(|(_, w)| *w == LOOSE_PIN));
        let weights = [
            [1.0, 0.0, 0.0, 0.0],
            [0.5, 0.5, 0.0, 0.0],
            [0.4, 0.3, 0.2, 0.1],
            [1.0, 0.0, 0.0, 0.0],
        ];
        let pins = pins_from_skin(4, Some(&weights));
        assert_eq!(pins, vec![(0, LOOSE_PIN), (1, LOOSE_PIN), (3, LOOSE_PIN)]);
    }

    /// The cloth of a small square pins at its pins and keeps the mesh's
    /// edges as structural links.
    #[test]
    fn a_mesh_becomes_a_pinned_cloth() {
        let vertices = [
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
        ];
        let triangles = [[0, 1, 2], [0, 2, 3]];
        let pins = pins_from_skin(4, None);
        let cloth = cloth_of(&vertices, &triangles, &pins);
        assert_eq!(cloth.particles.len(), 4);
        assert_eq!(cloth.triangles.len(), 2);
        // Four edges and the diagonal, plus one bend link.
        assert_eq!(cloth.edges.len(), 6);
        assert!(
            cloth.particles[0]
                .position
                .abs_diff_eq(Vec3::new(0.0, 100.0, 0.0), 1e-4)
        );
        assert!(cloth.pins.iter().all(|p| p.weight == LOOSE_PIN));
    }

    /// A part whose seam splits a position element draws one particle for
    /// both vertices, and the file's face flags pin only the hem.
    #[test]
    fn a_seam_split_vertex_is_one_particle_and_the_hem_holds() {
        // A square of two triangles whose diagonal is a UV seam: vertices
        // 4 and 5 repeat elements 0 and 2.
        let vertices = [
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
        ];
        let elements = [0, 1, 2, 3, 0, 2];
        let triangles = [[0, 1, 2], [4, 5, 3]];
        // The faces over the elements: the first marked firmly.
        let faces = [[0, 1, 2], [0, 2, 3]];
        let pins = cloth::face_flag_pins(4, &faces, &[0x8000_0000, 0]);
        assert_eq!(pins, vec![(1, cloth::FIRM_PIN)]);
        let (cloth, drawn) = cloth_of_elements(&vertices, &triangles, &elements, &pins);
        assert_eq!(cloth.particles.len(), 4);
        assert_eq!(drawn, vec![0, 1, 2, 3, 0, 2]);
        assert_eq!(cloth.pins.len(), 1);
        assert_eq!((cloth.pins[0].index, cloth.pins[0].weight), (1, 1.0));
        // Four sides and the diagonal, plus the bend link 1-3.
        assert_eq!(cloth.edges.len(), 6);
    }

    /// A placement's word is the cloth's starting countdown, asleep under 1
    /// (every shipped record stores 0).
    #[test]
    fn a_placed_cloth_starts_with_its_records_countdown() {
        assert_eq!(start_countdown(0), 0);
        assert_eq!(start_countdown(-3), 0);
        assert_eq!(start_countdown(1), 1);
        assert_eq!(start_countdown(40), 40);
        // The builder reads it from the part's node.
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .add_systems(Update, build_cloth_nodes);
        let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(
            Mesh::new(
                bevy::mesh::PrimitiveTopology::TriangleList,
                bevy::asset::RenderAssetUsages::default(),
            )
            .with_inserted_attribute(
                Mesh::ATTRIBUTE_POSITION,
                vec![[0.0f32, 1.0, 0.0], [1.0, 1.0, 0.0], [1.0, 0.0, 0.0]],
            )
            .with_inserted_indices(bevy::mesh::Indices::U32(vec![0, 1, 2])),
        );
        let node = app.world_mut().spawn(ClothStartSleep(0)).id();
        let part = app
            .world_mut()
            .spawn((ClothNode, Mesh3d(mesh), ChildOf(node)))
            .id();
        app.update();
        let instance = app.world().entity(part).get::<ClothInstance>().unwrap();
        assert_eq!(instance.cloth.sleep, 0);
    }

    /// A part carrying [`ClothParameters`] sets its cloth up from that
    /// record (the hair's), any other from the built-in one.
    #[test]
    fn a_cloth_takes_its_nodes_parameter_record() {
        use exa_physics::cloth_record::ClothRecord;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .add_systems(Update, build_cloth_nodes);
        let triangle = || {
            Mesh::new(
                bevy::mesh::PrimitiveTopology::TriangleList,
                bevy::asset::RenderAssetUsages::default(),
            )
            .with_inserted_attribute(
                Mesh::ATTRIBUTE_POSITION,
                vec![[0.0f32, 1.0, 0.0], [1.0, 1.0, 0.0], [1.0, 0.0, 0.0]],
            )
            .with_inserted_indices(bevy::mesh::Indices::U32(vec![0, 1, 2]))
        };
        let hair_mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(triangle());
        let banner_mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(triangle());
        let hair = app
            .world_mut()
            .spawn((
                ClothNode,
                Mesh3d(hair_mesh),
                ClothParameters(ClothRecord::HAIR),
            ))
            .id();
        let banner = app.world_mut().spawn((ClothNode, Mesh3d(banner_mesh))).id();
        app.update();
        let record = |e: Entity| {
            app.world()
                .entity(e)
                .get::<ClothInstance>()
                .unwrap()
                .cloth
                .record
        };
        assert_eq!(record(hair), ClothRecord::HAIR);
        assert_eq!(record(banner), ClothRecord::LEVEL);
    }

    /// Carried into a node frame turned a quarter about y and moved 1 m, a
    /// particle keeps its world place (and its last place keeps its own).
    #[test]
    fn a_carried_cloth_keeps_its_world_places() {
        let mut cloth = Cloth::new(vec![exa_physics::cloth::Particle {
            position: Vec3::new(10.0, 0.0, 0.0),
            previous: Vec3::new(9.0, 0.0, 0.0),
        }]);
        let last = Affine3A::IDENTITY;
        let now = Affine3A::from_rotation_translation(
            Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            Vec3::new(1.0, 0.0, 0.0),
        );
        carry_into_frame(&mut cloth, last, now);
        let world = |p: Vec3| now.transform_point3(p * 0.01) * 100.0;
        let p = cloth.particles[0];
        assert!(world(p.position).abs_diff_eq(Vec3::new(10.0, 0.0, 0.0), 1e-3));
        assert!(world(p.previous).abs_diff_eq(Vec3::new(9.0, 0.0, 0.0), 1e-3));
    }

    /// A free cloth falls through the air and comes to rest on the level's
    /// triangle: the sector's static grid finds the floor, the object's
    /// level contacts are in its own centimetres, and the object list knows
    /// the cloth's cell.
    #[test]
    fn a_free_cloth_lands_on_the_static_grids_level() {
        let (mut app, entity) = falling_cloth(false);
        let instance = app.world().entity(entity).get::<ClothInstance>().unwrap();
        assert!(!instance.cloth.level.is_empty());
        let low = lowest_m(&app, entity);
        assert!(low > 0.5, "the cloth fell through the level: {low} m");
        let objects = app.world().resource::<ClothObjects>();
        assert_eq!(objects.order, [entity]);
        assert_eq!(objects.grid.objects().len(), 1);
        // Landed and still, it falls asleep: the level never wakes it.
        for _ in 0..200 {
            app.world_mut().run_schedule(FixedUpdate);
        }
        let instance = app.world().entity(entity).get::<ClothInstance>().unwrap();
        assert_eq!(instance.cloth.sleep, 0, "the landed cloth never slept");
    }

    /// The level mesh of the cloth's own scene node does not push it, as
    /// the original's static stage skips the cloth's own node.
    #[test]
    fn a_cloth_passes_through_its_own_nodes_mesh() {
        let (app, entity) = falling_cloth(true);
        let low = lowest_m(&app, entity);
        assert!(low < 0.5, "the cloth's own mesh held it: {low} m");
        // The debug counters: every record the cloth found was its own, so
        // it took no triangle (by now it has fallen out of the floor's
        // cells).
        let debug = app.world().resource::<ClothDebug>();
        let (counted, work, _) = debug.cloths[0];
        assert_eq!(counted, entity);
        assert_eq!(work.records, work.own_records);
        assert_eq!((work.triangles, work.level_tests), (0, 0));
        assert_eq!((work.awake, work.particles), (1, 4));
    }

    /// The grid sector's object stage offers a cloth only the body parts
    /// bucketed in the nine 6 m cells around its own: a part in the
    /// cloth's cell and one in the next cell are offered, one three cells
    /// away is not, however many there are.
    #[test]
    fn a_cloth_is_offered_only_the_parts_in_the_nine_cells_around_it() {
        use exa_physics::body::PartSetup;
        use exa_physics::{Physics, StaticColliders};

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, ClothPlugin))
            .init_resource::<Assets<Mesh>>()
            .insert_resource(Physics::default())
            .insert_resource(StaticColliders::default())
            .insert_resource(ClothContacts(true))
            .insert_resource(ClothDebug {
                on: true,
                ..ClothDebug::default()
            });
        let body_at = |x: f32| {
            Ragdoll::new(&[PartSetup {
                bone: 1,
                entity: Entity::PLACEHOLDER,
                parent: None,
                world: Affine3A::from_translation(Vec3::new(x, 1.5, 0.0)),
                centre: Vec3::ZERO,
                half: Vec3::splat(0.1),
                origin: Vec3::ZERO,
                links: vec![],
                foot: None,
                pins: Vec::new(),
                spheres: Some(vec![(Vec3::ZERO, 0.1)]),
            }])
        };
        // The cloth's cell (x 0..6 m), the next cell, and twenty parts
        // three cells away.
        app.world_mut().spawn(body_at(0.5));
        app.world_mut().spawn(body_at(7.0));
        for _ in 0..20 {
            app.world_mut().spawn(body_at(20.0));
        }
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Mesh::new(
                bevy::mesh::PrimitiveTopology::TriangleList,
                bevy::asset::RenderAssetUsages::default(),
            ));
        let corners = [
            Vec3::new(0.0, 2.0, 0.0),
            Vec3::new(1.0, 2.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let cloth = cloth_of(&corners, &[[0, 1, 2], [0, 2, 3]], &[]);
        let drawn = (0..cloth.particles.len()).collect();
        let entity = app
            .world_mut()
            .spawn((
                ClothInstance::new(cloth, mesh, drawn),
                GlobalTransform::default(),
            ))
            .id();
        app.world_mut().run_schedule(FixedUpdate);
        let debug = app.world().resource::<ClothDebug>();
        let (counted, work, _) = debug.cloths[0];
        assert_eq!(counted, entity);
        assert_eq!(work.awake, 1);
        assert_eq!(work.parts, 2, "offered {} parts", work.parts);
        // The object list knows the cloth's own cell, not the park cell.
        let objects = app.world().resource::<ClothObjects>();
        let origin = ClothObjects::ORIGIN;
        assert_eq!(objects.grid.cell_of_id(0), Some(origin));
    }

    /// The original's object-body stage masks a body by its state: a
    /// disabled body (bit 4) takes no cloth body contacts, while its sleep
    /// decides between the stage's two masks.
    #[test]
    fn a_disabled_body_is_never_offered_to_a_cloth() {
        use exa_physics::body::PartSetup;
        use exa_physics::{Physics, StaticColliders};

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, ClothPlugin))
            .init_resource::<Assets<Mesh>>()
            .insert_resource(Physics::default())
            .insert_resource(StaticColliders::default())
            .insert_resource(ClothContacts(true))
            .insert_resource(ClothDebug {
                on: true,
                ..ClothDebug::default()
            });
        let body = Ragdoll::new(&[PartSetup {
            bone: 1,
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: Affine3A::from_translation(Vec3::new(0.5, 1.5, 0.0)),
            centre: Vec3::ZERO,
            half: Vec3::splat(0.1),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: Vec::new(),
            spheres: Some(vec![(Vec3::ZERO, 0.1)]),
        }]);
        let mut disabled = body.clone();
        disabled.set_disabled(true);
        app.world_mut().spawn(body);
        app.world_mut().spawn(disabled);
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Mesh::new(
                bevy::mesh::PrimitiveTopology::TriangleList,
                bevy::asset::RenderAssetUsages::default(),
            ));
        let corners = [
            Vec3::new(0.0, 2.0, 0.0),
            Vec3::new(1.0, 2.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let cloth = cloth_of(&corners, &[[0, 1, 2], [0, 2, 3]], &[]);
        let drawn = (0..cloth.particles.len()).collect();
        app.world_mut().spawn((
            ClothInstance::new(cloth, mesh, drawn),
            GlobalTransform::default(),
        ));
        app.world_mut().run_schedule(FixedUpdate);
        let debug = app.world().resource::<ClothDebug>();
        let (_, work, _) = debug.cloths[0];
        // Both bodies are in the cloth's cell; only the enabled one is
        // offered.
        assert_eq!(work.parts, 1, "offered {} parts", work.parts);
        assert_eq!(work.bodies, 1);
    }

    /// The lowest vertex of a cloth, in metres.
    fn lowest_m(app: &App, entity: Entity) -> f32 {
        let instance = app.world().entity(entity).get::<ClothInstance>().unwrap();
        instance
            .cloth
            .vertices()
            .iter()
            .map(|p| p.y)
            .fold(f32::MAX, f32::min)
            * 0.01
    }

    /// A 1 m cloth dropped from 1-2 m onto a floor quad at y = 0.5 m, in
    /// the world and in the sector's grid, after 150 ticks. With `own`, the
    /// floor belongs to the scene node the cloth hangs under.
    fn falling_cloth(own: bool) -> (App, Entity) {
        use exa_physics::Physics;
        use exa_physics::StaticColliders;
        use rapier3d::prelude::ColliderBuilder;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, ClothPlugin))
            .init_resource::<Assets<Mesh>>()
            .insert_resource(Physics::default())
            .insert_resource(StaticColliders::default())
            .insert_resource(ClothContacts(true))
            .insert_resource(ClothDebug {
                on: true,
                ..ClothDebug::default()
            });
        let node = app
            .world_mut()
            .spawn((exa_physics::StaticNode, GlobalTransform::default()))
            .id();
        let vertices = vec![
            Vec3::new(-2.0, 0.5, -2.0),
            Vec3::new(2.0, 0.5, -2.0),
            Vec3::new(2.0, 0.5, 2.0),
            Vec3::new(-2.0, 0.5, 2.0),
        ];
        let indices = vec![[0u32, 2, 1], [0, 3, 2]];
        let collider = ColliderBuilder::trimesh(vertices, indices).unwrap().build();
        let handle = app
            .world_mut()
            .resource_mut::<Physics>()
            .0
            .insert_collider(collider, None);
        let floor_node = if own { node.index().index() } else { 0 };
        app.world_mut()
            .resource_mut::<StaticColliders>()
            .insert_aabb(
                handle.into_raw_parts().0,
                Vec3::new(-2.0, 0.5, -2.0),
                Vec3::new(2.0, 0.5, 2.0),
                floor_node,
                exa_physics::sector::MASK_SMALL,
            );
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Mesh::new(
                bevy::mesh::PrimitiveTopology::TriangleList,
                bevy::asset::RenderAssetUsages::default(),
            ));
        let corners = [
            Vec3::new(0.0, 2.0, 0.0),
            Vec3::new(1.0, 2.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let cloth = cloth_of(&corners, &[[0, 1, 2], [0, 2, 3]], &[]);
        let drawn = (0..cloth.particles.len()).collect();
        let entity = app
            .world_mut()
            .spawn((
                ClothInstance::new(cloth, mesh, drawn),
                GlobalTransform::default(),
                ChildOf(node),
            ))
            .id();
        for _ in 0..150 {
            app.world_mut().run_schedule(FixedUpdate);
        }
        (app, entity)
    }
}
