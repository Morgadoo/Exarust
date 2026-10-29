//! Which rigid bodies of a level the original registers in its solver, and
//! their registration: every rigid body of a prop or placed object, from
//! the top body of each node tree down through its body children (a body
//! under a plain node under a body never registers), unless the prop is
//! placed static (`Prop::STATIC`) or the node is marked as level geometry
//! (`Node::STATIC`, whose body the model loader drops). Exarust registers
//! free bodies and hung ones: a body
//! whose root carries links (lamps, chains, wall rings) registers with its
//! stored links anchored to its node ([`exa_physics::body::WorldLink`]) and
//! no level detection. A mechanism's hung body registers the same way and
//! also gets its rules, as the original creates its mechanism body and runs
//! its set-up when it is added to the world: a hinged door's leaf its door
//! rules ([`exa_physics::body::PendingDoor`]), a lever's arm its lever rules
//! ([`exa_physics::body::PendingLever`]), a push surface its plate rules
//! ([`exa_physics::body::PendingPlate`]), and, when the loader is asked for
//! them ([`MechanismBodies::sliding`]), a sliding door's leaf its sliding
//! rules ([`exa_physics::body::PendingSlide`]). Otherwise a mechanism's body
//! stays static level geometry. Their rules are in `exa_physics`'s `door`,
//! `lever`, `plate` and `mechanism` modules.
//! Evidence: `docs/formats/RFC.md` (Rigid body, Props).

use bevy::prelude::*;
use exa_assets::{Model, NodeRigidBody};
use exa_formats::rigid_body::RigidBody;
use exa_physics::body::{
    PendingDoor, PendingLever, PendingPlate, PendingSlide, RigidLink, RigidPart,
};
use exa_physics::prop::{BodyNode, BodyScrape, BodySounds, PendingProp, PendingScrapes};
use exa_physics::sector::Bounds;
use exa_physics::{door, mechanism};

/// What a body tree (a top body and its body children) is to the solver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeKind {
    /// No links on its top body: it registers, asleep.
    Free,
    /// Its top body carries links: it registers hung from the world, its
    /// root's stored links anchored to its node.
    Hung,
}

/// A top body and its body children, as node indices, parents first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BodyTree {
    pub kind: TreeKind,
    pub nodes: Vec<usize>,
}

/// The body trees of a model whose nodes are given as (parent, body):
/// each body with no body over it heads one, with its direct body
/// children, recursively. A body under a plain node under a body is in no
/// tree, as the original never registers it.
pub fn body_trees(nodes: &[(Option<usize>, Option<&RigidBody>)]) -> Vec<BodyTree> {
    let has_body = |i: usize| nodes.get(i).is_some_and(|n| n.1.is_some());
    let under_a_body = |mut at: Option<usize>| {
        // Bounded by the node count, so a cycle in bad input ends.
        for _ in 0..nodes.len() {
            match at {
                Some(i) if has_body(i) => return true,
                Some(i) => at = nodes.get(i).and_then(|n| n.0),
                None => return false,
            }
        }
        false
    };
    let mut trees = Vec::new();
    for (i, &(parent, body)) in nodes.iter().enumerate() {
        let Some(body) = body else { continue };
        if under_a_body(parent) {
            continue;
        }
        let mut tree = vec![i];
        let mut k = 0;
        while k < tree.len() {
            let at = tree[k];
            let children: Vec<usize> = (0..nodes.len())
                .filter(|&j| nodes[j].0 == Some(at) && has_body(j) && !tree.contains(&j))
                .collect();
            tree.extend(children);
            k += 1;
        }
        trees.push(BodyTree {
            kind: if body.links.is_empty() {
                TreeKind::Free
            } else {
                TreeKind::Hung
            },
            nodes: tree,
        });
    }
    trees
}

/// How a level's rigid bodies came out.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BodyCounts {
    /// Body trees registered in the solver, and their bodies.
    pub registered: usize,
    pub parts: usize,
    /// Of the registered trees, those hung from the world by their root's
    /// links.
    pub hung: usize,
    /// Of the registered trees, the hinged doors' leaves, the levers' arms,
    /// the push surfaces and the sliding doors' leaves.
    pub doors: usize,
    pub levers: usize,
    pub plates: usize,
    pub sliding: usize,
    /// Trees left static: of a mechanism, of a prop placed static, or with
    /// a flat rest tetrahedron.
    pub mechanisms: usize,
    pub static_props: usize,
    pub flat: usize,
}

impl BodyCounts {
    pub fn add(&mut self, other: BodyCounts) {
        self.registered += other.registered;
        self.parts += other.parts;
        self.hung += other.hung;
        self.doors += other.doors;
        self.levers += other.levers;
        self.plates += other.plates;
        self.sliding += other.sliding;
        self.mechanisms += other.mechanisms;
        self.static_props += other.static_props;
        self.flat += other.flat;
    }
}

/// How a spawned model's bodies are to be treated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// A prop or object whose free bodies register.
    Loose,
    /// A prop placed static: none of its bodies registers.
    Static,
    /// A mechanism whose bodies stay static (its kind is not asked for).
    Mechanism,
    /// A door: its hinged leaves register hung, with their door rules, and
    /// its sliding leaves too when asked for; any other tree stays static.
    Door(DoorPlacement),
    /// A lever: its hung arm registers with its lever rules.
    Lever(OperativePlacement),
    /// A push surface: its hung plate registers with its plate rules.
    Plate(OperativePlacement),
}

/// What a door's leaves take from its placement: the object database's
/// door record, the placement's stored word, and whether a sliding leaf
/// registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DoorPlacement {
    pub record: door::Record,
    pub saved: i32,
    pub sliding: bool,
}

/// What a lever or push surface takes from its placement: its operative
/// record, the placement's stored word and its placed id (tag bits
/// included), which its signals come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperativePlacement {
    pub record: mechanism::Record,
    pub saved: i32,
    pub id: u32,
}

/// Which mechanisms' bodies a level load registers with their rules.
/// Hinged doors, levers and push surfaces by default; sliding doors only
/// when asked (`--sliding-doors`: their lift is not reproduced, see
/// `exa_physics::body::slide_body`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MechanismBodies {
    pub doors: bool,
    pub levers: bool,
    pub plates: bool,
    pub sliding: bool,
}

impl Default for MechanismBodies {
    fn default() -> Self {
        Self {
            doors: true,
            levers: true,
            plates: true,
            sliding: false,
        }
    }
}

impl MechanismBodies {
    /// Every mechanism static level geometry.
    pub const STATIC: Self = Self {
        doors: false,
        levers: false,
        plates: false,
        sliding: false,
    };
}

/// How a placed object's bodies are treated: a door's as a door (else
/// static), a lever's as a lever and a push surface's as a plate when their
/// kinds are asked for (else static), every other object's loose. A
/// mechanism takes the placement's word as its stored word, as the
/// original's object spawn stores it on the node.
pub fn placement_of(
    item: &exa_formats::objdb::Item,
    object: &exa_formats::rfc::PlacedObject,
    bodies: MechanismBodies,
) -> Placement {
    use exa_formats::objdb::Mechanism;
    let saved = object.flags as i32;
    let operative = |record: &exa_formats::objdb::Operative| OperativePlacement {
        record: crate::mechanisms::operative_record(record),
        saved,
        id: object.id,
    };
    match (
        item.mechanism(),
        item.door.as_ref(),
        item.operative.as_ref(),
    ) {
        (Some(Mechanism::Door), Some(section), _) if bodies.doors => {
            Placement::Door(DoorPlacement {
                record: crate::mechanisms::door_record(section),
                saved,
                sliding: bodies.sliding,
            })
        }
        (Some(Mechanism::Lever), _, Some(record)) if bodies.levers => {
            Placement::Lever(operative(record))
        }
        (Some(Mechanism::PushSurface), _, Some(record)) if bodies.plates => {
            Placement::Plate(operative(record))
        }
        (Some(_), _, _) => Placement::Mechanism,
        (None, _, _) => Placement::Loose,
    }
}

/// Whether a body tree is a hinged leaf: its top body carries links and
/// none of them slides.
fn hinged(tree: &BodyTree, nodes: &[(Option<usize>, Option<&RigidBody>)]) -> bool {
    tree.kind == TreeKind::Hung
        && tree
            .nodes
            .first()
            .and_then(|&i| nodes[i].1)
            .is_some_and(|b| !b.links.iter().any(|l| l.is_slider()))
}

/// Whether a body tree is a sliding leaf: its top body carries a slider
/// link.
fn sliding(tree: &BodyTree, nodes: &[(Option<usize>, Option<&RigidBody>)]) -> bool {
    tree.kind == TreeKind::Hung
        && tree
            .nodes
            .first()
            .and_then(|&i| nodes[i].1)
            .is_some_and(|b| b.links.iter().any(|l| l.is_slider()))
}

/// What a registered tree of a mechanism is, by its placement and links.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rules {
    None,
    Hinged,
    Sliding,
    Lever,
    Plate,
}

/// The rules a tree of `placement` takes, or `None` when the tree stays
/// static (`Some(Rules::None)` registers it plainly).
fn rules_of(
    placement: Placement,
    tree: &BodyTree,
    nodes: &[(Option<usize>, Option<&RigidBody>)],
) -> Option<Rules> {
    match placement {
        Placement::Static | Placement::Mechanism => None,
        Placement::Loose => Some(Rules::None),
        Placement::Door(_) if hinged(tree, nodes) => Some(Rules::Hinged),
        Placement::Door(d) if d.sliding && sliding(tree, nodes) => Some(Rules::Sliding),
        Placement::Door(_) => None,
        Placement::Lever(_) if tree.kind == TreeKind::Hung => Some(Rules::Lever),
        Placement::Plate(_) if sliding(tree, nodes) => Some(Rules::Plate),
        Placement::Lever(_) | Placement::Plate(_) => None,
    }
}

/// A stored link in the solver's terms. A root body's links to the world
/// keep a slider's travel as its slack (the slider law divides by its
/// square, the stored value). The original keeps only flag 8 of an
/// articulated child's links, so a child slider's stored slack (its travel
/// squared, in cm²) becomes plain slack in cm.
fn body_link(l: &exa_formats::rigid_body::BodyLink, child: bool) -> RigidLink {
    let slack = if child && l.is_slider() {
        let travel_cm = l.slack * 100.0;
        travel_cm * travel_cm / 100.0
    } else {
        l.slack
    };
    RigidLink {
        on_body: Vec3::from(l.on_body),
        on_parent: Vec3::from(l.on_parent),
        slack,
        strength: l.strength,
        smoothing: l.smoothing,
        flags: l.flags,
        axis: l.axis.map(Vec3::from).unwrap_or(Vec3::ZERO),
    }
}

/// The solver part of a model node's body (its world transform is set when
/// it registers).
fn rigid_part(
    entity: Entity,
    parent: Option<usize>,
    rigid: &NodeRigidBody,
    origin: Vec3,
    mesh: Bounds,
) -> RigidPart {
    let b = &rigid.body;
    RigidPart {
        entity,
        parent,
        world: bevy::math::Affine3A::IDENTITY,
        rest: b.rest.map(Vec3::from),
        point_masses: b.point_masses,
        parent_world: bevy::math::Affine3A::IDENTITY,
        mobility: rigid.mobility(),
        spheres: b
            .spheres
            .iter()
            .map(|s| (Vec3::from(s.centre), s.radius))
            .collect(),
        origin,
        links: b
            .links
            .iter()
            .map(|l| body_link(l, parent.is_some()))
            .collect(),
        mesh: Some(mesh),
    }
}

/// Whether a body's rest points span a tetrahedron (the solver inverts
/// it); every shipped one does.
fn solid(body: &RigidBody) -> bool {
    let r = body.rest.map(Vec3::from);
    let det = bevy::math::Mat3::from_cols(r[1] - r[0], r[2] - r[0], r[3] - r[0]).determinant();
    det.abs() > 1e-12 && det.is_finite()
}

/// The body trees of a model (nodes as (parent, body)) that register, and
/// what became of all of them.
pub fn plan(
    nodes: &[(Option<usize>, Option<&RigidBody>)],
    placement: Placement,
) -> (BodyCounts, Vec<BodyTree>) {
    let mut counts = BodyCounts::default();
    let mut registered = Vec::new();
    for tree in body_trees(nodes) {
        if placement == Placement::Static {
            counts.static_props += 1;
            continue;
        }
        let Some(rules) = rules_of(placement, &tree, nodes) else {
            counts.mechanisms += 1;
            continue;
        };
        if !tree.nodes.iter().all(|&i| nodes[i].1.is_some_and(solid)) {
            counts.flat += 1;
            continue;
        }
        if tree.kind == TreeKind::Hung {
            counts.hung += 1;
        }
        match rules {
            Rules::Hinged => counts.doors += 1,
            Rules::Sliding => counts.sliding += 1,
            Rules::Lever => counts.levers += 1,
            Rules::Plate => counts.plates += 1,
            Rules::None => {}
        }
        counts.registered += 1;
        counts.parts += tree.nodes.len();
        registered.push(tree);
    }
    (counts, registered)
}

/// Registers the free bodies of a spawned model (`nodes`: its node
/// entities, as [`crate::spawn_model_nodes`] returns them): each tree's
/// top node gets what registration needs, each of its nodes the body it
/// belongs to.
pub fn register_bodies(
    commands: &mut Commands,
    model: &Model,
    nodes: &[Entity],
    placement: Placement,
) -> BodyCounts {
    let shape: Vec<_> = model
        .nodes
        .iter()
        .map(|n| {
            (
                n.parent,
                (!n.placeholder)
                    .then_some(n.rigid.as_ref())
                    .flatten()
                    .map(|r| r.body.as_ref()),
            )
        })
        .collect();
    let (counts, trees) = plan(&shape, placement);
    for tree in trees {
        let mut scrapes = Vec::new();
        let mut sounds = BodySounds::default();
        let mut part_index = 0;
        let parts: Vec<RigidPart> = tree
            .nodes
            .iter()
            .filter_map(|&i| {
                let node = &model.nodes[i];
                let rigid = node.rigid.as_ref()?;
                let entity = *nodes.get(i)?;
                let parent = node
                    .parent
                    .and_then(|p| tree.nodes.iter().position(|&j| j == p));
                let part = rigid_part(
                    entity,
                    parent,
                    rigid,
                    node.transform.translation,
                    // The node's box and radius, as the original copies
                    // them into the body's mesh bound.
                    Bounds {
                        centre: node.bounds.0,
                        half: node.half_extents,
                        radius: node.bounds.1,
                    },
                );
                for sound in &rigid.body.sounds {
                    // The original drops a slot whose sound the audio
                    // environment cannot find; with no audio lookup here,
                    // only an unnamed slot is dropped.
                    if !sound.sound.is_empty() {
                        sounds.scrapes.push(sound.sound.clone());
                        scrapes.push(BodyScrape {
                            part: part_index,
                            point: Vec3::from(sound.point),
                            volume: sound.volume,
                            pitch: sound.pitch,
                        });
                    }
                }
                sounds.impact.push(rigid.collision_sound.clone());
                part_index += 1;
                Some(part)
            })
            .collect();
        let Some(root) = parts.first().map(|p| p.entity) else {
            continue;
        };
        if !scrapes.is_empty() {
            commands.entity(root).insert(PendingScrapes(scrapes));
        }
        // The names the audio side resolves: each part's collision sound
        // and each scrape source's sound.
        commands.entity(root).insert(sounds);
        for (k, part) in parts.iter().enumerate() {
            commands.entity(part.entity).insert(BodyNode {
                body: root,
                part: k,
            });
        }
        commands.entity(root).insert(PendingProp(parts));
        // A mechanism's rules, built once its body is registered.
        let Some(&top) = tree.nodes.first() else {
            continue;
        };
        let handle_height = model.nodes[top].bounds.0.y;
        match (placement, rules_of(placement, &tree, &shape)) {
            (Placement::Door(door), Some(Rules::Hinged)) => {
                commands.entity(root).insert(PendingDoor {
                    record: door.record,
                    saved: door.saved,
                    handle_height,
                });
            }
            (Placement::Door(door), Some(Rules::Sliding)) => {
                commands.entity(root).insert(PendingSlide {
                    record: door.record,
                    saved: door.saved,
                    handle_height,
                });
            }
            (Placement::Lever(lever), Some(Rules::Lever)) => {
                commands.entity(root).insert(PendingLever {
                    record: crate::mechanisms::lever_record(&lever.record),
                    saved: lever.saved,
                    id: lever.id,
                });
            }
            (Placement::Plate(plate), Some(Rules::Plate)) => {
                commands.entity(root).insert(PendingPlate {
                    record: crate::mechanisms::plate_record(&plate.record),
                    saved: plate.saved,
                    id: plate.id,
                });
            }
            _ => {}
        }
    }
    counts
}

/// Which of a spawned model's nodes its placement marks as level geometry
/// (the original's node flag `0x80000`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelGeometry {
    /// None: a loose prop, a placed object without the static bits, the
    /// level's own model.
    None,
    /// The model's root alone: a placed object whose placement carries the
    /// static bits (the object reader does not spread the flag).
    Root,
    /// Every node: a prop placed static (the prop reader spreads the flag).
    All,
}

/// The mesh nodes of a model that carry a rigid body, as the original's grid
/// build sees them, each with whether its points are still stamped: the
/// body's attach sets the node's second flag word's `0x10` (whether the body
/// was read with the node or copied with an object's instance), and the
/// stamp skips such a mesh unless the node is level geometry (`0x80000`);
/// every one of them, level geometry or not, raises the floors under it
/// once the layers are written. Whether the body registers in the solver
/// does not matter (doors, levers and bodies under a plain node under a
/// body count too). A node marked static in its file has no body
/// ([`exa_assets::ModelNode::rigid`]), nor has an object-library
/// placeholder; a mesh without GI points takes no part.
pub fn moving_meshes(model: &Model, geometry: LevelGeometry) -> Vec<(usize, bool)> {
    let root = model.roots().next();
    model
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.rigid.is_some() && !n.placeholder && !n.gi_points.is_empty())
        .map(|(i, _)| {
            let level_geometry = match geometry {
                LevelGeometry::None => false,
                LevelGeometry::Root => Some(i) == root,
                LevelGeometry::All => true,
            };
            (i, level_geometry)
        })
        .collect()
}

/// Marks a spawned model's meshes that carry a rigid body
/// ([`moving_meshes`]) for the navigation grid ([`crate::nav::MovingMesh`]).
pub fn mark_moving_meshes(
    commands: &mut Commands,
    model: &Model,
    nodes: &[Entity],
    geometry: LevelGeometry,
) {
    for (i, stamped) in moving_meshes(model, geometry) {
        if let Some(&e) = nodes.get(i) {
            commands
                .entity(e)
                .insert(crate::nav::MovingMesh { stamped });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_formats::rigid_body::{BodyLink, Sphere};

    fn body(links: usize) -> RigidBody {
        RigidBody {
            class: 0x1000_0000,
            mobility: 1.0,
            rest: [
                [0.0, 0.0, 0.0],
                [0.1, 0.0, 0.0],
                [0.0, 0.1, 0.0],
                [0.0, 0.0, 0.1],
            ],
            point_masses: [4.0; 4],
            spheres: vec![Sphere {
                centre: [0.0; 3],
                radius: 0.05,
            }],
            links: (0..links)
                .map(|_| BodyLink {
                    flags: 2,
                    on_body: [0.0; 3],
                    on_parent: [0.0; 3],
                    slack: 0.01,
                    strength: 1.0,
                    smoothing: 0.2,
                    axis: None,
                })
                .collect(),
            sounds: Vec::new(),
        }
    }

    /// The meshes with bodies the grid build sees: a loose sack's body
    /// meshes (not its plain root, nor a placeholder or a mesh without
    /// points), stamped only where the placement makes the node level
    /// geometry: the root alone for a placed object's static bits, every
    /// node for a prop placed static.
    #[test]
    fn meshes_with_bodies_raise_floors_and_level_geometry_stamps() {
        use exa_assets::{ModelNode, NodeRigidBody};
        let model_node = |parent: Option<usize>, rigid: bool, points: bool| {
            let mut n = crate::tests::node("n", parent, None);
            n.rigid = rigid.then(|| NodeRigidBody {
                body: std::sync::Arc::new(body(0)),
                mobility_divisor: 1.0,
                collision_sound: None,
            });
            if points {
                n.gi_points = std::sync::Arc::new(vec![Vec3::ZERO]);
            }
            n
        };
        let mut placeholder: ModelNode = model_node(Some(0), true, true);
        placeholder.placeholder = true;
        let model = Model {
            nodes: vec![
                model_node(None, true, true),
                model_node(Some(0), true, true),
                model_node(Some(0), false, true),
                model_node(Some(0), true, false),
                placeholder,
            ],
        };
        assert_eq!(
            moving_meshes(&model, LevelGeometry::None),
            [(0, false), (1, false)]
        );
        assert_eq!(
            moving_meshes(&model, LevelGeometry::Root),
            [(0, true), (1, false)]
        );
        assert_eq!(
            moving_meshes(&model, LevelGeometry::All),
            [(0, true), (1, true)]
        );
    }

    /// A sack (a body with three body children), a chain hung from a wall
    /// ring (a linked top body), a plain root over a body, and a body under
    /// a plain node under a body, which never registers.
    #[test]
    fn trees_start_at_top_bodies() {
        let (free, linked, child) = (body(0), body(1), body(2));
        let sack = [
            (None, Some(&free)),
            (Some(0), Some(&child)),
            (Some(0), Some(&child)),
            (Some(1), Some(&child)),
        ];
        assert_eq!(
            body_trees(&sack),
            [BodyTree {
                kind: TreeKind::Free,
                nodes: vec![0, 1, 2, 3]
            }]
        );
        let chain = [(None, Some(&linked)), (Some(0), Some(&child))];
        assert_eq!(body_trees(&chain)[0].kind, TreeKind::Hung);
        let wrapped = [
            (None, None),
            (Some(0), Some(&free)),
            (Some(1), None),
            (Some(2), Some(&free)),
        ];
        let trees = body_trees(&wrapped);
        assert_eq!(trees.len(), 1);
        assert_eq!(trees[0].nodes, [1]);
    }

    /// The bodies of a level as its load registers them, from its scenes
    /// alone: its own model, its props (static ones not), its placed
    /// objects (mechanisms not).
    fn level_counts(
        game: &exa_assets::GameData,
        level: &str,
        mechanisms: MechanismBodies,
    ) -> BodyCounts {
        use exa_assets::PackKind;
        use exa_formats::rdb::Database;
        use exa_formats::rfc::{PlacedObject, Scene};
        let res = game.pack(PackKind::Resource).unwrap();
        let global =
            exa_formats::objdb::items(&Database::parse(res.get("objdb.rdb").unwrap()).unwrap());
        let scene = Scene::parse(res.get(&format!("{level}.rfc")).unwrap()).unwrap();
        let shape = |s: &Scene| -> Vec<(Option<usize>, Option<RigidBody>)> {
            s.nodes
                .iter()
                .map(|n| {
                    let body = n.rigid().filter(|_| !n.is_static()).cloned();
                    (n.parent.and_then(|p| p.checked_sub(1)), body)
                })
                .collect()
        };
        let count = |s: &Scene, placement| {
            let nodes = shape(s);
            let refs: Vec<_> = nodes.iter().map(|(p, b)| (*p, b.as_ref())).collect();
            plan(&refs, placement).0
        };
        let model = |kinds: &[PackKind], name: &str| {
            kinds.iter().find_map(|&k| {
                game.pack(k)
                    .and_then(|p| p.get(name))
                    .and_then(|b| Scene::parse(b).ok())
            })
        };
        let mut counts = count(&scene, Placement::Loose);
        for p in &scene.props {
            if let Some(m) = model(&[PackKind::Objlib], &p.object) {
                let placement = if p.is_static() {
                    Placement::Static
                } else {
                    Placement::Loose
                };
                counts.add(count(&m, placement));
            }
        }
        for o in scene.placed_objects.iter().filter(|o| o.slot == 0) {
            let items = if o.is_local() {
                &scene.local_items
            } else {
                &global
            };
            let id = o.id & PlacedObject::ID_MASK;
            let Some(item) = items
                .iter()
                .find(|i| i.id == id && !i.model.is_empty() && !i.model.starts_with('@'))
            else {
                continue;
            };
            if let Some(m) = model(&[PackKind::Objects, PackKind::Objlib], &item.model) {
                counts.add(count(&m, placement_of(item, o, mechanisms)));
            }
        }
        counts
    }

    /// The installed levels' bodies (skipped without the game): what
    /// registers and what stays static. `exanima01`, with hinged doors
    /// alone: 4 822 trees of 6 146 bodies register (props' sacks, buckets
    /// and carts with their children among them); 231 of the trees hang
    /// from the world (wall chains, lamps, and the 109 hinged door leaves,
    /// 123 bodies with their knockers), 89 are levers, plates and sliding
    /// doors left static, 492 belong to props placed static. By default the
    /// 40 levers and 12 push surfaces register too (37 left static), and
    /// with sliding doors asked for, the 37 sliding leaves as well. `arenao1`: 68 of its props are placed static (the
    /// barrels, the cart), 3 lanterns hang, its own model's tavern table is a
    /// node marked static; 20 bodies register (its tableware, clothes, a
    /// basket and the lanterns).
    #[test]
    fn shipped_level_bodies() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(dir).unwrap();
        let counts = |registered, parts, hung, doors, mechanisms, static_props| BodyCounts {
            registered,
            parts,
            hung,
            doors,
            mechanisms,
            static_props,
            ..BodyCounts::default()
        };
        let doors_only = MechanismBodies {
            levers: false,
            plates: false,
            ..MechanismBodies::default()
        };
        assert_eq!(
            level_counts(&game, "exanima01", doors_only),
            counts(4822, 6146, 231, 109, 89, 492)
        );
        let live = level_counts(&game, "exanima01", MechanismBodies::default());
        let all = level_counts(
            &game,
            "exanima01",
            MechanismBodies {
                sliding: true,
                ..MechanismBodies::default()
            },
        );
        // The 40 levers and 12 push surfaces register with their rules; the
        // 37 sliding leaves (portcullises, secret doors) only when asked.
        assert_eq!(
            live,
            BodyCounts {
                levers: 40,
                plates: 12,
                ..counts(4874, 6198, 283, 109, 37, 492)
            }
        );
        assert_eq!(
            all,
            BodyCounts {
                levers: 40,
                plates: 12,
                sliding: 37,
                ..counts(4911, 6235, 320, 109, 0, 492)
            }
        );
        assert_eq!(
            level_counts(&game, "arenao1", MechanismBodies::default()),
            counts(20, 20, 3, 0, 0, 68)
        );
        // The levels' own bodies marked as level geometry: only the tavern
        // table of arenao1 (the six of exanima01 register).
        let res = game.pack(exa_assets::PackKind::Resource).unwrap();
        for (level, marked, bodies) in [("arenao1", 1, 1), ("exanima01", 0, 6)] {
            let rfc = res.get(&format!("{level}.rfc")).unwrap();
            let scene = exa_formats::rfc::Scene::parse(rfc).unwrap();
            let with_body = || scene.nodes.iter().filter(|n| n.rigid().is_some());
            assert_eq!(with_body().filter(|n| n.is_static()).count(), marked);
            assert_eq!(with_body().count(), bodies, "{level}");
        }
    }

    /// The installed `exanima01`'s hinged doors as bodies (skipped without
    /// the game): every leaf registers with its door rules; woken and left
    /// alone for two seconds of ticks, every shut leaf (67 of the 109) stays
    /// within a centimetre and shut (but for the one whose double-door
    /// partner stands open, which the original lets out of the shut state), and an open one, free on a hinge that is
    /// not quite plumb, settles a few centimetres inside its swing; a shut, unlatched leaf pulled by the hand's
    /// link toward its open side unlatches and swings more than 0.5 rad.
    /// The other bodies are taken out after registration (there is no level
    /// collision here, so a prop a leaf woke would fall).
    #[test]
    fn shipped_doors_rest_shut_and_open_under_a_pull() {
        use bevy::ecs::world::CommandQueue;
        use bevy::time::TimeUpdateStrategy;
        use exa_physics::body::{DoorBody, Ragdoll};
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let Ok(game) = exa_assets::GameData::open(&dir) else {
            return;
        };
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::transform::TransformPlugin,
            exa_physics::PhysicsPlugin,
        ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f64(1.0 / exa_physics::STEP_HZ),
        ));
        let mut cache = exa_assets::ModelCache::default();
        let mut meshes = Assets::<Mesh>::default();
        let mut materials = Assets::<exa_render::material::ExaMaterial>::default();
        let mut images = Assets::<Image>::default();
        let mut queue = CommandQueue::default();
        {
            let mut loader = exa_assets::ModelLoader {
                game: &game,
                cache: &mut cache,
                meshes: &mut meshes,
                materials: &mut materials,
                images: &mut images,
                bc_supported: false,
            };
            let mut commands = Commands::new(&mut queue, app.world());
            crate::spawn_level(&mut commands, &mut loader, "exanima01").expect("exanima01");
        }
        queue.apply(app.world_mut());
        app.world_mut().insert_resource(meshes);
        for _ in 0..4 {
            app.update();
        }
        let mut doors = app.world_mut().query_filtered::<Entity, With<DoorBody>>();
        let leaves: Vec<Entity> = doors.iter(app.world()).collect();
        assert_eq!(leaves.len(), 109);
        let mut others = app
            .world_mut()
            .query_filtered::<Entity, (With<Ragdoll>, Without<DoorBody>)>();
        let others: Vec<Entity> = others.iter(app.world()).collect();
        for e in others {
            app.world_mut().entity_mut(e).remove::<Ragdoll>();
        }
        let snapshot = |app: &mut App| -> Vec<(Vec3, bool, f32, f32)> {
            leaves
                .iter()
                .map(|&e| {
                    let body = app.world().get::<Ragdoll>(e).unwrap();
                    let glue = app.world().get::<DoorBody>(e).unwrap();
                    (
                        body.parts[0].positions[3],
                        glue.door.closed,
                        glue.heading(body) * glue.door.shape.sign,
                        glue.door.shape.closed(),
                    )
                })
                .collect()
        };
        let before = snapshot(&mut app);
        let shut = before.iter().filter(|s| s.1).count();
        assert!(shut > 50, "{shut} of 109 leaves shut at load");
        for _ in 0..2 {
            for &e in &leaves {
                app.world_mut().get_mut::<Ragdoll>(e).unwrap().wake();
            }
            for _ in 0..83 {
                app.update();
            }
        }
        let after = snapshot(&mut app);
        for (i, (b, a)) in before.iter().zip(&after).enumerate() {
            assert!(a.0.is_finite(), "leaf {i}");
            let moved = b.0.distance(a.0);
            if b.1 {
                // A shut leaf is held by its drive. The original lets a
                // double door's leaf whose partner stands open out of the
                // shut state as soon as it reads under its shut angle
                // (without a hold); it still does not move.
                let glue = app.world().get::<DoorBody>(leaves[i]).unwrap();
                let partner_open = glue.partner.is_some_and(|p| {
                    leaves
                        .iter()
                        .position(|&q| q == p)
                        .is_some_and(|k| !before[k].1)
                });
                assert!(a.1 || partner_open, "leaf {i} left the shut state");
                assert!(moved < 0.01, "shut leaf {i} moved {moved}");
            } else {
                // An open leaf is free: its hinge is not quite plumb, so it
                // settles a little under gravity, inside its swing.
                assert!(moved < 0.3, "open leaf {i} moved {moved}");
                let shape = app.world().get::<DoorBody>(leaves[i]).unwrap().door.shape;
                // Fully open is the shut angle less twice the half swing.
                let (lo, hi) = (shape.closed() - 2.0 * shape.half_swing, shape.closed());
                let (lo, hi) = (lo.min(hi) - 0.05, lo.max(hi) + 0.05);
                assert!(
                    (lo..=hi).contains(&a.2),
                    "open leaf {i} at {} past its stop",
                    a.2
                );
            }
        }
        // Pull a shut, unlatched leaf open.
        let (k, &leaf) = leaves
            .iter()
            .enumerate()
            .find(|&(k, &e)| before[k].1 && !app.world().get::<DoorBody>(e).unwrap().latched())
            .expect("a shut unlatched leaf");
        // 1 m from the hinge at 1 m up, toward the stop swung `share` of
        // the range from shut.
        let anchor = |app: &App, share: f32| {
            let body = app.world().get::<Ragdoll>(leaf).unwrap();
            let glue = app.world().get::<DoorBody>(leaf).unwrap();
            let shape = glue.door.shape;
            let origin = glue.origin(body);
            let toward =
                glue.stop_at(body, shape.closed() - 2.0 * shape.half_swing * share) - origin;
            origin + Vec3::Y + Vec3::new(toward.x, 0.0, toward.z).normalize()
        };
        let grip = anchor(&app, 0.0);
        {
            let world = app.world_mut();
            let mut entity = world.entity_mut(leaf);
            let mut glue = entity.take::<DoorBody>().unwrap();
            let mut body = entity.get_mut::<Ragdoll>().unwrap();
            glue.take_hold(&mut body, grip);
            entity.insert(glue);
        }
        let start = before[k].2;
        for _ in 0..400 {
            let to = anchor(&app, 0.9);
            let world = app.world_mut();
            let mut entity = world.entity_mut(leaf);
            let mut glue = entity.take::<DoorBody>().unwrap();
            let mut body = entity.get_mut::<Ragdoll>().unwrap();
            glue.pull_to(&mut body, Some(to));
            entity.insert(glue);
            app.update();
        }
        let glue = app.world().get::<DoorBody>(leaf).unwrap();
        assert!(!glue.door.closed, "the pulled leaf is still shut");
        assert!(
            (glue.door.angle - start).abs() > 0.5,
            "the leaf swung {} rad",
            glue.door.angle - start
        );
    }

    /// The installed `exanima01`'s levers and push surfaces as bodies, with
    /// its sliding doors asked for (skipped without the game): the 40 arms
    /// and 12 plates register with their rules; woken and left alone for
    /// two seconds of ticks, no arm or plate moves more than a centimetre
    /// and none signals; a lever turned by the wheel onto its other detent
    /// signals its target, and the router runs the order on the door bodies
    /// under it. The other bodies are taken out after registration (there
    /// is no level collision here). The 37 sliding leaves all read fraction
    /// 0 at load (their slider points on their anchors), every rail lies
    /// level, and at rest they creep up to about 2 cm in two seconds without
    /// sleeping; the test prints them.
    #[test]
    fn shipped_levers_and_plates_rest_and_signal() {
        use bevy::ecs::world::CommandQueue;
        use bevy::time::TimeUpdateStrategy;
        use exa_physics::body::{DoorBody, LeverBody, PlateBody, Ragdoll, SlideBody};
        use exa_physics::mechanism::MechanismSignal;
        #[derive(Resource, Default)]
        struct Sent(Vec<MechanismSignal>);
        fn capture(mut read: MessageReader<MechanismSignal>, mut sent: ResMut<Sent>) {
            sent.0.extend(read.read().copied());
        }
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let Ok(game) = exa_assets::GameData::open(&dir) else {
            return;
        };
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::transform::TransformPlugin,
            exa_physics::PhysicsPlugin,
            crate::mechanisms::MechanismPlugin,
        ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f64(1.0 / exa_physics::STEP_HZ),
        ))
        .init_resource::<Sent>()
        .add_systems(FixedUpdate, capture.after(exa_physics::body::BodySystems));
        let mut cache = exa_assets::ModelCache::default();
        let mut meshes = Assets::<Mesh>::default();
        let mut materials = Assets::<exa_render::material::ExaMaterial>::default();
        let mut images = Assets::<Image>::default();
        let mut queue = CommandQueue::default();
        {
            let mut loader = exa_assets::ModelLoader {
                game: &game,
                cache: &mut cache,
                meshes: &mut meshes,
                materials: &mut materials,
                images: &mut images,
                bc_supported: false,
            };
            let mut commands = Commands::new(&mut queue, app.world());
            let mut load = crate::LevelLoad::begin(&mut commands, &mut loader, "exanima01")
                .expect("exanima01");
            load.mechanism_bodies.sliding = true;
            while !matches!(
                load.step(&mut commands, &mut loader, std::time::Duration::MAX),
                crate::LoadProgress::Finished(..)
            ) {}
        }
        queue.apply(app.world_mut());
        app.world_mut().insert_resource(meshes);
        for _ in 0..4 {
            app.update();
        }
        let mut q = app.world_mut().query_filtered::<Entity, With<LeverBody>>();
        let levers: Vec<Entity> = q.iter(app.world()).collect();
        let mut q = app.world_mut().query_filtered::<Entity, With<PlateBody>>();
        let plates: Vec<Entity> = q.iter(app.world()).collect();
        let mut q = app.world_mut().query_filtered::<Entity, With<SlideBody>>();
        let slides: Vec<Entity> = q.iter(app.world()).collect();
        assert_eq!((levers.len(), plates.len()), (40, 12));
        let mut q = app.world_mut().query_filtered::<Entity, (
            With<Ragdoll>,
            Without<LeverBody>,
            Without<PlateBody>,
            Without<SlideBody>,
            Without<DoorBody>,
        )>();
        let others: Vec<Entity> = q.iter(app.world()).collect();
        for e in others {
            app.world_mut().entity_mut(e).remove::<Ragdoll>();
        }
        for &e in &slides {
            let body = app.world().get::<Ragdoll>(e).unwrap();
            let glue = app.world().get::<SlideBody>(e).unwrap();
            let rail = body
                .world
                .iter()
                .find(|l| l.flags & exa_physics::body::world_link::SLIDER != 0)
                .map(|l| l.axis.normalize_or_zero())
                .unwrap_or_default();
            eprintln!(
                "slide {e:?}: fraction {:.3} closed {} motorised {} mode {:?} rail {rail:.2}",
                glue.fraction(body),
                glue.door.closed,
                glue.door.motorised,
                glue.door.slide.mode,
            );
        }
        let arm = |app: &App, e: Entity| -> Vec3 {
            let body = app.world().get::<Ragdoll>(e).unwrap();
            app.world().get::<LeverBody>(e).unwrap().arm_point(body)
        };
        let plate_at = |app: &App, e: Entity| -> Vec3 {
            app.world().get::<Ragdoll>(e).unwrap().parts[0]
                .transform()
                .translation
                .into()
        };
        let before: Vec<Vec3> = levers.iter().map(|&e| arm(&app, e)).collect();
        let plates_before: Vec<Vec3> = plates.iter().map(|&e| plate_at(&app, e)).collect();
        let slides_before: Vec<Vec3> = slides.iter().map(|&e| plate_at(&app, e)).collect();
        for _ in 0..2 {
            for &e in levers.iter().chain(&plates).chain(&slides) {
                app.world_mut().get_mut::<Ragdoll>(e).unwrap().wake();
            }
            for _ in 0..83 {
                app.update();
            }
        }
        for (i, &e) in levers.iter().enumerate() {
            let moved = arm(&app, e).distance(before[i]);
            assert!(moved < 0.01, "lever {i} moved {moved} m");
            let lever = app.world().get::<LeverBody>(e).unwrap().lever;
            let near = lever.detent_angle(lever.detent_index());
            assert!(
                (lever.angle - near).abs() < 0.03,
                "lever {i} off its detent"
            );
        }
        for (i, &e) in plates.iter().enumerate() {
            let moved = plate_at(&app, e).distance(plates_before[i]);
            assert!(moved < 0.01, "plate {i} moved {moved} m");
        }
        for (i, &e) in slides.iter().enumerate() {
            let moved = plate_at(&app, e).distance(slides_before[i]);
            let body = app.world().get::<Ragdoll>(e).unwrap();
            let glue = app.world().get::<SlideBody>(e).unwrap();
            // The sliding leaves creep along their rails (up to about 2 cm
            // in two seconds, still awake): why they stay behind the flag.
            assert!(moved < 0.05, "sliding leaf {i} moved {moved} m");
            assert!(!glue.door.closed && glue.fraction(body) < 0.05);
            if moved > 0.01 {
                eprintln!("sliding leaf {i} crept {moved:.3} m at rest");
            }
        }
        assert!(
            app.world().resource::<Sent>().0.is_empty(),
            "{:?}",
            app.world().resource::<Sent>().0
        );
        // Turn a lever with a target onto its other detent.
        let lever = *levers
            .iter()
            .find(|&&e| app.world().get::<LeverBody>(e).unwrap().record.target != 0)
            .expect("a lever with a target");
        let (from, target, other) = {
            let glue = app.world().get::<LeverBody>(lever).unwrap();
            let index = glue.lever.detent_index();
            let other = glue.lever.detent_angle(1 - index.clamp(0, 1));
            (index, glue.record.target, glue.arm_point_at(other))
        };
        for _ in 0..300 {
            let world = app.world_mut();
            let mut entity = world.entity_mut(lever);
            let mut glue = entity.take::<LeverBody>().unwrap();
            let mut body = entity.get_mut::<Ragdoll>().unwrap();
            if !glue.lever.held {
                let at = glue.arm_point(&body);
                let grabbed = glue.take_hold(&mut body, at);
                assert!(grabbed.abs() < 0.05, "{grabbed}");
            }
            glue.turn_to(&mut body, other, other, true, 0.0);
            entity.insert(glue);
            app.update();
        }
        let glue = app.world().get::<LeverBody>(lever).unwrap();
        assert_eq!(glue.lever.detent_index(), 1 - from.clamp(0, 1));
        let sent = &app.world().resource::<Sent>().0;
        let value = if from == 0 { 1 } else { 0 };
        assert!(
            sent.iter().any(|s| s.target == target && s.value == value),
            "{sent:?}"
        );
        eprintln!("lever signals: {sent:?}");
    }

    /// A child's slider link keeps its stored slack as plain slack: a 7 cm
    /// travel (stored 49) becomes 49 cm. A root body's slider keeps its
    /// travel, which the slider law divides by.
    #[test]
    fn a_childs_slider_slack_is_its_stored_value() {
        let mut l = body(1).links[0];
        l.flags = 4;
        l.slack = 0.07;
        l.axis = Some([0.0, 0.0, -0.07]);
        assert!((body_link(&l, true).slack - 0.49).abs() < 1e-6);
        assert_eq!(body_link(&l, false).slack, 0.07);
        l.flags = 2;
        assert_eq!(body_link(&l, true).slack, 0.07);
    }
}

#[cfg(test)]
mod volume_fit {
    use bevy::math::Vec3;
    use exa_assets::PackKind;
    use exa_formats::rfc::{Node, NodeBody, Scene};
    use exa_physics::volume::{
        COARSE_RESOLUTION, FINE_RESOLUTION, FittedVolume, MIN_DEPTH, Mesh, PRECISE_MIN_DEPTH,
        VolumeOptions, fit,
    };
    use exa_physics::voxel::Bounds;

    /// The file's centimetre words behind `metres` (the parser multiplies
    /// each by 0.01 in single precision, which can map two neighbouring
    /// words to one value): the first run of little-endian words in
    /// `bytes` that maps onto `metres` value by value.
    fn raw_words(bytes: &[u8], metres: &[f32]) -> Option<Vec<f32>> {
        let first = *metres.first()?;
        let len = metres.len() * 4;
        let word =
            |o: usize| f32::from_le_bytes([bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]]);
        (0..bytes.len().checked_sub(len)? + 1).find_map(|at| {
            if word(at) * 0.01 != first {
                return None;
            }
            let words: Vec<f32> = (0..metres.len()).map(|i| word(at + i * 4)).collect();
            words
                .iter()
                .zip(metres)
                .all(|(w, m)| w * 0.01 == *m)
                .then_some(words)
        })
    }

    /// A body node's mesh in the file's centimetres over its stored box.
    fn node_mesh(bytes: &[u8], node: &Node) -> Option<Mesh> {
        let NodeBody::Mesh(mesh) = &node.body else {
            return None;
        };
        let mut elements = vec![None; mesh.position_count];
        for (v, &e) in mesh.position_ids.iter().enumerate() {
            *elements.get_mut(e as usize)? = Some(*mesh.positions.get(v)?);
        }
        let all = elements.into_iter().collect::<Option<Vec<[f32; 3]>>>()?;
        let stream = raw_words(bytes, &all.concat())?;
        let at = |v: u32| {
            let o = *mesh.position_ids.get(v as usize)? as usize * 3;
            Some(Vec3::new(
                *stream.get(o)?,
                *stream.get(o + 1)?,
                *stream.get(o + 2)?,
            ))
        };
        let triangles = mesh
            .indices
            .chunks(3)
            .map(|t| Some([at(*t.first()?)?, at(*t.get(1)?)?, at(*t.get(2)?)?]))
            .collect::<Option<Vec<_>>>()?;
        let mut header = node.bounds_half_extents.to_vec();
        header.extend(node.bounds_centre);
        let raw = raw_words(bytes, &header)?;
        Some(Mesh::with_bounds(
            &triangles,
            Bounds {
                centre: Vec3::new(raw[3], raw[4], raw[5]),
                half_extents: Vec3::new(raw[0], raw[1], raw[2]),
            },
        ))
    }

    /// A fit's spheres as the file stores them (metres, bits), sorted. With
    /// `earlier`, the candidate spheres are `r` cells across, as an earlier
    /// version of the fit made them, instead of `r/(1 + 0.01 r)`.
    fn stored_form(fitted: &FittedVolume, bounds: Bounds, earlier: bool) -> Vec<[u32; 4]> {
        let h = bounds.half_extents;
        let mut axis = 0;
        for a in 1..3 {
            if h[axis] < h[a] {
                axis = a;
            }
        }
        let cell = h[axis] / fitted.counts[axis] as f32 * 2.0;
        let mut out: Vec<[u32; 4]> = fitted
            .spheres
            .iter()
            .map(|s| {
                let radius = if earlier && s.radius != cell {
                    (1..40_000u32)
                        .map(|n| f64::from(n).sqrt() as f32)
                        .find(|&r| {
                            let r = f64::from(r);
                            (r / (r * 0.01 + 1.0) * f64::from(cell)) as f32 == s.radius
                        })
                        .map_or(s.radius, |r| r * cell)
                } else {
                    s.radius
                };
                let c = s.centre;
                [c.x * 0.01, c.y * 0.01, c.z * 0.01, radius * 0.01].map(f32::to_bits)
            })
            .collect();
        out.sort();
        out
    }

    /// The installed game's prop bodies (with `EXANIMA_DIR`): every body
    /// node of the `Objlib` models is fitted from its mesh, in the file's
    /// centimetres over its stored box, and its spheres compared bit for
    /// bit with its stored list. The stored lists come from two versions of
    /// the fit (one making candidate spheres `r` cells across) and were made
    /// with or without the voxeliser's wide-query flag, at either
    /// resolution; a body matches when one of those reproduces it. On the
    /// shipped build 1060 of the 1503 bodies match, 526 of them with the
    /// builders' defaults.
    #[test]
    fn shipped_prop_meshes_fit_their_stored_bodies() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let pack = game.pack(PackKind::Objlib).unwrap();
        let (mut bodies, mut by_default, mut matched) = (0usize, 0usize, 0usize);
        let mut time = std::time::Duration::ZERO;
        for entry in pack.entries() {
            let Some(bytes) = pack.get(&entry.name) else {
                continue;
            };
            let Ok(scene) = Scene::parse(bytes) else {
                continue;
            };
            for node in &scene.nodes {
                let Some(Ok(body)) = &node.rigid_body else {
                    continue;
                };
                let Some(mesh) = node_mesh(bytes, node) else {
                    continue;
                };
                bodies += 1;
                let mut stored: Vec<[u32; 4]> = body
                    .spheres
                    .iter()
                    .map(|s| [s.centre[0], s.centre[1], s.centre[2], s.radius].map(f32::to_bits))
                    .collect();
                stored.sort();
                let defaults = VolumeOptions::for_bounds(&mesh.bounds(), false);
                let begun = std::time::Instant::now();
                let fitted = fit(&mesh, &defaults);
                time += begun.elapsed();
                let same = |f: &FittedVolume| {
                    [false, true]
                        .iter()
                        .any(|&earlier| stored_form(f, mesh.bounds(), earlier) == stored)
                };
                if same(&fitted) {
                    by_default += 1;
                    matched += 1;
                    continue;
                }
                let other = if defaults.resolution == FINE_RESOLUTION {
                    COARSE_RESOLUTION
                } else {
                    FINE_RESOLUTION
                };
                let found = [
                    (defaults.resolution, MIN_DEPTH, true),
                    (other, MIN_DEPTH, true),
                    (other, MIN_DEPTH, false),
                    (FINE_RESOLUTION, PRECISE_MIN_DEPTH, true),
                ]
                .into_iter()
                .any(|(resolution, min_depth, wide_queries)| {
                    same(&fit(
                        &mesh,
                        &VolumeOptions {
                            resolution,
                            min_depth,
                            wide_queries,
                            ..defaults
                        },
                    ))
                });
                matched += usize::from(found);
            }
        }
        eprintln!(
            "shipped bodies: {bodies}, {matched} fitted bit for bit ({by_default} with the \
             builders' defaults); default fits took {time:?}"
        );
        assert!(bodies > 1000, "too few shipped bodies found: {bodies}");
        assert!(
            matched * 10 >= bodies * 6,
            "{matched} of {bodies} shipped bodies fitted"
        );
    }
}
