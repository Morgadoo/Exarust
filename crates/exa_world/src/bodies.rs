//! Which rigid bodies of a level the original registers in its solver, and
//! their registration: every rigid body of a prop or placed object, from
//! the top body of each node tree down through its body children (a body
//! under a plain node under a body never registers), unless the prop is
//! placed static (`Prop::STATIC`) or the node is marked as level geometry
//! (`Node::STATIC`, whose body the model loader drops). Exarust registers
//! only free bodies for
//! now: a body hung from the world by its links (lamps, chains, wall
//! rings) needs the world-link laws, and doors, levers and push surfaces
//! need their mechanisms and those same world links, so they stay static
//! level geometry until both land. Their rules are in `exa_physics`'s
//! `door`, `lever`, `plate` and `mechanism` modules. Evidence:
//! `docs/formats/RFC.md` (Rigid body, Props).

use bevy::prelude::*;
use exa_assets::{Model, NodeRigidBody};
use exa_formats::rigid_body::RigidBody;
use exa_physics::body::{RigidLink, RigidPart};
use exa_physics::prop::{BodyNode, PendingProp};

/// What a body tree (a top body and its body children) is to the solver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeKind {
    /// No links on its top body: it registers, asleep.
    Free,
    /// Its top body carries links: it hangs from the world (static until
    /// the world links land).
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
    /// Trees left static: hung from the world, of a mechanism, of a prop
    /// placed static, or with a flat rest tetrahedron.
    pub hung: usize,
    pub mechanisms: usize,
    pub static_props: usize,
    pub flat: usize,
}

impl BodyCounts {
    pub fn add(&mut self, other: BodyCounts) {
        self.registered += other.registered;
        self.parts += other.parts;
        self.hung += other.hung;
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
    /// A door, lever or push surface (static until its mechanism lands).
    Mechanism,
}

/// A stored link of an articulated child in the solver's terms. The
/// original keeps only flag 8 of a child's links, so a slider's stored
/// slack (its travel squared, in cm²) becomes plain slack in cm.
fn child_link(l: &exa_formats::rigid_body::BodyLink) -> RigidLink {
    let slack = if l.is_slider() {
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
    }
}

/// The solver part of a model node's body (its world transform is set when
/// it registers).
fn rigid_part(
    entity: Entity,
    parent: Option<usize>,
    rigid: &NodeRigidBody,
    origin: Vec3,
) -> RigidPart {
    let b = &rigid.body;
    RigidPart {
        entity,
        parent,
        world: bevy::math::Affine3A::IDENTITY,
        rest: b.rest.map(Vec3::from),
        point_masses: b.point_masses,
        mobility: rigid.mobility(),
        spheres: b
            .spheres
            .iter()
            .map(|s| (Vec3::from(s.centre), s.radius))
            .collect(),
        origin,
        links: if parent.is_some() {
            b.links.iter().map(child_link).collect()
        } else {
            Vec::new()
        },
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
        match (placement, tree.kind) {
            (Placement::Static, _) => counts.static_props += 1,
            (Placement::Mechanism, _) => counts.mechanisms += 1,
            (Placement::Loose, TreeKind::Hung) => counts.hung += 1,
            (Placement::Loose, TreeKind::Free) => {
                if !tree.nodes.iter().all(|&i| nodes[i].1.is_some_and(solid)) {
                    counts.flat += 1;
                    continue;
                }
                counts.registered += 1;
                counts.parts += tree.nodes.len();
                registered.push(tree);
            }
        }
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
        .map(|n| (n.parent, n.rigid.as_ref().map(|r| r.body.as_ref())))
        .collect();
    let (counts, trees) = plan(&shape, placement);
    for tree in trees {
        let parts: Vec<RigidPart> = tree
            .nodes
            .iter()
            .filter_map(|&i| {
                let node = &model.nodes[i];
                let parent = node
                    .parent
                    .and_then(|p| tree.nodes.iter().position(|&j| j == p));
                Some(rigid_part(
                    *nodes.get(i)?,
                    parent,
                    node.rigid.as_ref()?,
                    node.transform.translation,
                ))
            })
            .collect();
        let Some(root) = parts.first().map(|p| p.entity) else {
            continue;
        };
        for (k, part) in parts.iter().enumerate() {
            commands.entity(part.entity).insert(BodyNode {
                body: root,
                part: k,
            });
        }
        commands.entity(root).insert(PendingProp(parts));
    }
    counts
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
    fn level_counts(game: &exa_assets::GameData, level: &str) -> BodyCounts {
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
                let placement = if item.mechanism().is_some() {
                    Placement::Mechanism
                } else {
                    Placement::Loose
                };
                counts.add(count(&m, placement));
            }
        }
        counts
    }

    /// The installed levels' bodies (skipped without the game): what
    /// registers asleep and what stays static. `exanima01`: 4 591 trees of
    /// 5 000 bodies register (props' sacks, buckets and carts with their
    /// children among them); 122 hang from the world (wall chains, lamps),
    /// 198 are doors, levers and plates, 492 belong to props placed static.
    /// `arenao1`: 68 of its props are placed static (the barrels, the cart),
    /// 3 lanterns hang, its own model's tavern table is a node marked static;
    /// 17 bodies register (its tableware, clothes and a basket).
    #[test]
    fn shipped_level_bodies() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(dir).unwrap();
        let counts = |registered, parts, hung, mechanisms, static_props| BodyCounts {
            registered,
            parts,
            hung,
            mechanisms,
            static_props,
            flat: 0,
        };
        assert_eq!(
            level_counts(&game, "exanima01"),
            counts(4591, 5000, 122, 198, 492)
        );
        assert_eq!(level_counts(&game, "arenao1"), counts(17, 17, 3, 0, 68));
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

    /// A child's slider link keeps its stored slack as plain slack: a 7 cm
    /// travel (stored 49) becomes 49 cm.
    #[test]
    fn a_childs_slider_slack_is_its_stored_value() {
        let mut l = body(1).links[0];
        l.flags = 4;
        l.slack = 0.07;
        l.axis = Some([0.0, 0.0, -0.07]);
        assert!((child_link(&l).slack - 0.49).abs() < 1e-6);
        l.flags = 2;
        assert_eq!(child_link(&l).slack, 0.07);
    }
}
