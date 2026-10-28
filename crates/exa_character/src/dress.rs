//! The dressed character mesh: the order the original appends the body and
//! its garments into one skinned mesh, the cull bits each slot's append
//! takes (tested against the vertices' slot masks), and the overlay meshes
//! past the eighteen body slots.
//!
//! The builder assembles each garment into a chosen node per slot
//! ([`crate::layers`]); this module turns that choice into the merge the
//! skinning builder runs: the chosen nodes are appended in slot order (1 to
//! 18) where the slot's select mask meets the garment's visible mask, and a garment's slots take the cull bits of
//! [`crate::layers::cull_bits`]. The body's own visible slots follow, without
//! cull bits, and the overlays (hair and other meshes numbered past the
//! slots) come last. [`crate::rebuild::dress_plan`] gives the assembly; this
//! module is the merge list a caller with model meshes turns into
//! `exa_assets::model::DressedPart`s.
//!
//! The running game drives it: [`assemble`] builds the character's runtime
//! body tree and plan from its built models, and `exa_character`'s dressing
//! pass merges the parts and draws one sub-mesh per material run. A body
//! model whose nodes do not number slots (the demons) or a character
//! without its body model keeps the per-node draw.

use bevy::prelude::Transform;
use exa_assets::Model;
use exa_formats::morph::Frame;

use crate::layers::{cull_bits, slot_selected};
use crate::rebuild::WornGarment;
use crate::runtime::{Body, Node, overlay_nodes};

/// One node of the merged mesh: the node whose meshes are appended, the
/// cull bits its append takes (`exa_assets::merge::MergePart::cull`), and
/// whether it is an overlay (merged after the body, past the slot tree).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MergeStep {
    pub node: usize,
    pub cull: u32,
    pub overlay: bool,
}

/// The dressed-mesh plan: the garment assembly the builder computed, then
/// the merge list in the order the skin builder appends it.
#[derive(Clone, Debug, PartialEq)]
pub struct DressedPlan {
    pub garments: Vec<crate::rebuild::GarmentMerge>,
    /// The body's visible mask ([`crate::layers::visible_body_slots`]).
    pub body_visible: u32,
    /// The body slots still merged, as a mask.
    pub body_slots: u32,
    /// The impact material each body slot takes from its least-covering
    /// garment ([`crate::rebuild::DressPlan`]).
    pub slot_material: [u8; 18],
    /// The nodes merged, in order: each garment's chosen nodes in slot
    /// order, then the body's visible slots, then the overlays.
    pub steps: Vec<MergeStep>,
}

/// A garment's merge list: its chosen nodes in slot order (1 to 18, the
/// order the skin builder appends them; the slot-tree walk before it only
/// builds the skin weights), each gated by the slot's select mask against
/// `visible` and carrying the slot's cull bits.
pub fn garment_steps(chosen: &[Option<usize>; 19], visible: u32) -> Vec<MergeStep> {
    (1..=18)
        .filter_map(|slot| {
            let node = chosen[slot]?;
            slot_selected(slot, visible).then_some(MergeStep {
                node,
                cull: cull_bits(slot, visible),
                overlay: false,
            })
        })
        .collect()
}

/// The body's merge list: the nodes its slot table holds, in slot order,
/// where the slot's select mask meets the body's visible mask. The body's
/// own parts take no cull bits.
pub fn body_steps(slots: &[Option<usize>; 19], visible: u32) -> Vec<MergeStep> {
    (1..=18)
        .filter(|&slot| slot_selected(slot, visible))
        .filter_map(|slot| {
            Some(MergeStep {
                node: slots[slot]?,
                cull: 0,
                overlay: false,
            })
        })
        .collect()
}

/// The whole dressed mesh: the garments in the order given (the builder's
/// layer-level order), the body's visible slots, then every overlay mesh of
/// the body's tree in depth-first order.
pub fn dress(
    body: &Body,
    root: usize,
    garments: &[WornGarment],
    body_slots: &[Option<usize>; 19],
) -> DressedPlan {
    let plan = crate::rebuild::dress_plan(body, garments);
    with_steps(body, root, plan, body_slots)
}

/// Adds the merge list to an already computed plan ([`dress_steps`]).
fn with_steps(
    body: &Body,
    root: usize,
    plan: crate::rebuild::DressPlan,
    body_slots: &[Option<usize>; 19],
) -> DressedPlan {
    let steps = dress_steps(body, root, &plan.garments, body_slots, plan.body_visible);
    DressedPlan {
        garments: plan.garments,
        body_visible: plan.body_visible,
        body_slots: plan.body_slots,
        slot_material: plan.slot_material,
        steps,
    }
}

/// The merge list of a computed plan: each garment's chosen nodes in slot
/// order, the body's visible slots, then the overlays. `body_slots` is the
/// body's slot table.
fn dress_steps(
    body: &Body,
    root: usize,
    garments: &[crate::rebuild::GarmentMerge],
    body_slots: &[Option<usize>; 19],
    body_visible: u32,
) -> Vec<MergeStep> {
    let mut steps = Vec::new();
    for garment in garments {
        steps.extend(garment_steps(&garment.chosen, garment.visible));
    }
    steps.extend(body_steps(body_slots, body_visible));
    for node in overlay_nodes(body, root) {
        steps.push(MergeStep {
            node,
            cull: 0,
            overlay: true,
        });
    }
    steps
}

/// The body-only merge the builder runs when the race's look word lacks
/// [`exa_formats::races::LOOK_MORPH_PASS`] (bit `0x4`): every body slot the
/// table holds is merged in slot order, ungated by the visible mask and
/// without cull bits, then the overlays. (The other
/// branch, which builds the garments too, is [`dress`].)
pub fn body_only_steps(
    body: &Body,
    root: usize,
    body_slots: &[Option<usize>; 19],
) -> Vec<MergeStep> {
    let mut steps: Vec<MergeStep> = (1..=18)
        .filter_map(|slot| {
            Some(MergeStep {
                node: body_slots[slot]?,
                cull: 0,
                overlay: false,
            })
        })
        .collect();
    for node in overlay_nodes(body, root) {
        steps.push(MergeStep {
            node,
            cull: 0,
            overlay: true,
        });
    }
    steps
}

/// The first run-time material (`&…`) of the merged nodes in merge order:
/// the texture the skin job carries as its first apparel layer. `materials`
/// gives a node's part material names.
pub fn first_apparel(
    steps: &[MergeStep],
    mut materials: impl FnMut(usize) -> Vec<String>,
) -> Option<String> {
    for step in steps {
        if let Some(name) = materials(step.node)
            .into_iter()
            .find(|name| crate::spawn::is_runtime_material(name))
        {
            return Some(name);
        }
    }
    None
}

impl DressedPlan {
    /// The plan's first apparel texture ([`first_apparel`]).
    pub fn first_apparel_of(&self, materials: impl FnMut(usize) -> Vec<String>) -> Option<String> {
        first_apparel(&self.steps, materials)
    }
}

/// A worn garment as the dressed-mesh pass takes it: its built model (the
/// character's physique already applied) and the item's occupancy, coverage
/// and hit-sound material.
pub struct WornModel<'a> {
    pub model: &'a Model,
    /// The item's occupied sub-parts per body slot (slot 1 first).
    pub occupancy: [u64; 18],
    /// The item's eight-byte coverage word per body slot.
    pub coverage: [u64; 18],
    /// The item's hit-sound material byte.
    pub material: u8,
}

/// A character assembled for the merge: the runtime body tree with its
/// garments appended (body nodes first, then each garment's), the merge
/// plan and list, and where each garment's nodes start.
pub struct Assembled {
    pub body: Body,
    /// The body model's first root in [`Assembled::body`].
    pub root: usize,
    pub plan: DressedPlan,
    /// For each garment, its first node in the body tree.
    pub garment_bases: Vec<usize>,
}

impl Assembled {
    /// The model and node a merge step names: `0` is the body's model, `i +
    /// 1` the `i`th garment's.
    pub fn model_of(&self, step: MergeStep) -> (usize, usize) {
        for (g, &base) in self.garment_bases.iter().enumerate().rev() {
            if step.node >= base {
                return (g + 1, step.node - base);
            }
        }
        (0, step.node)
    }
}

/// Builds a character's runtime body tree and the dressed-mesh plan from
/// its built models, as the original assembles a body and the garments
/// worn over it: the body's nodes with their frames in its first root's
/// space, each garment appended as its own subtree, re-framed on the body's
/// slot frames ([`Body::reframe_from_slots`]), then the merge plan.
///
/// The body's slots come from [`Body::chain_slots`]; the garments are taken
/// in the order given (the caller's layer-level order). `None` when the
/// body model has no root.
pub fn assemble(body_model: &Model, garments: &[WornModel<'_>]) -> Option<Assembled> {
    let root = body_model.roots().next()?;
    let frames = model_frames(body_model);
    let nodes: Vec<Node> = body_model
        .nodes
        .iter()
        .zip(&frames)
        .map(|(node, frame)| node_of(node, *frame))
        .collect();
    let mut body = Body::new(nodes);
    // The body's eighteen slots, indexed by slot (the slot table `dress`
    // takes; index 0 is unused).
    let filled = body.chain_slots(root, true, 18);
    let mut slots = [None; 19];
    slots[1..=18].copy_from_slice(&filled[..18]);
    let slot_world: [Option<Frame>; 19] = std::array::from_fn(|slot| {
        if slot == 0 {
            None
        } else {
            slots[slot].map(|i| body.nodes[i].world)
        }
    });
    let mut worn = Vec::with_capacity(garments.len());
    let mut garment_bases = Vec::with_capacity(garments.len());
    for garment in garments {
        let base = body.nodes.len();
        garment_bases.push(base);
        if garment.model.nodes.is_empty() {
            // An empty model merges nothing; the entry keeps the caller's
            // garment indices aligned.
            worn.push(WornGarment {
                roots: Vec::new(),
                occupancy: garment.occupancy,
                coverage: garment.coverage,
                slots: 0,
                material: garment.material,
            });
            continue;
        }
        let garment_frames = model_frames(garment.model);
        for (node, frame) in garment.model.nodes.iter().zip(&garment_frames) {
            let mut n = node_of(node, *frame);
            n.parent = node.parent.map(|p| p + base);
            body.nodes.push(n);
        }
        body.refresh_links();
        // The item's model hangs under the character's root node, so its
        // tree's own root takes the body root's frame.
        body.nodes[base].world = body.nodes[root].world;
        let roots: Vec<usize> = (0..garment.model.nodes.len())
            .filter(|&i| garment.model.nodes[i].parent.is_none())
            .map(|i| i + base)
            .collect();
        let mut seen = 0;
        for &r in &roots {
            seen |= body.reframe_from_slots(r, &slot_world);
            // A one-node garment's mesh is the tree's own root: the file's
            // re-framing pass always has a parent to read, so its root
            // takes the slot frame directly here.
            let slot = crate::runtime::frame_slot(&body.nodes[r].name);
            if body.nodes[r].parent.is_none()
                && slot <= 18
                && let Some(frame) = slot_world[slot]
            {
                body.nodes[r].world = frame;
                seen |= 1 << (slot - 1);
            }
        }
        worn.push(WornGarment {
            roots,
            occupancy: garment.occupancy,
            coverage: garment.coverage,
            slots: seen,
            material: garment.material,
        });
    }
    let plan = crate::rebuild::dress_plan(&body, &worn);
    let steps = dress_steps(&body, root, &plan.garments, &slots, plan.body_visible);
    Some(Assembled {
        body,
        root,
        plan: DressedPlan {
            garments: plan.garments,
            body_visible: plan.body_visible,
            body_slots: plan.body_slots,
            slot_material: plan.slot_material,
            steps,
        },
        garment_bases,
    })
}

/// A built model node as a runtime body node: its frame, its mesh kind and
/// render word, and the slot and layer its name carries.
fn node_of(node: &exa_assets::ModelNode, world: Frame) -> Node {
    Node {
        name: node.name.clone(),
        parent: node.parent,
        local: file_frame(&node.transform),
        world,
        chunk_kind: if node.mesh { crate::runtime::MESH } else { 0 },
        render_flags: node.render_flags,
        layer: crate::layers::layer_of(&node.name),
        ..Node::default()
    }
}

/// A node transform as the file's row-vector frame (the inverse of
/// [`exa_assets::transform_from_rows`]): its first row is where the x axis
/// goes, its last the translation, as [`exa_formats::morph::compose`] and
/// the merge's placement take them. (`pose::rows_of` stores the rotation's
/// matrix rows instead, the transpose, for the pose module's own maps.)
pub fn file_frame(t: &Transform) -> Frame {
    let m = t.compute_affine();
    [
        m.matrix3.x_axis.to_array(),
        m.matrix3.y_axis.to_array(),
        m.matrix3.z_axis.to_array(),
        m.translation.to_array(),
    ]
}

/// The frame of every node of a model in its first root's space: each
/// node's transform composed with every ancestor's, as the file's
/// row-vector frames compose.
pub fn model_frames(model: &Model) -> Vec<Frame> {
    let mut out = Vec::with_capacity(model.nodes.len());
    for index in 0..model.nodes.len() {
        let mut chain = vec![index];
        let mut at = index;
        let mut steps = 0;
        while let Some(parent) = model.nodes[at].parent {
            if parent >= model.nodes.len() || chain.contains(&parent) || steps > model.nodes.len() {
                break;
            }
            chain.push(parent);
            at = parent;
            steps += 1;
        }
        let mut frame = exa_assets::merge::IDENTITY;
        for &at in chain.iter().rev() {
            frame = exa_formats::morph::compose(&frame, &file_frame(&model.nodes[at].transform));
        }
        out.push(frame);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layers::slot_bit;
    use crate::rebuild::WornGarment;
    use crate::runtime::{MESH, Node};

    fn node(name: &str, parent: Option<usize>) -> Node {
        Node {
            name: name.into(),
            parent,
            chunk_kind: MESH,
            slot: crate::layers::slot_of(name).unwrap_or(0),
            ..Node::default()
        }
    }

    /// The pelvis, its leg chain, the belly, chest, head and both arms: the
    /// body model's slot names.
    fn body() -> Body {
        let mut nodes = vec![
            node("01Pelvis", None),
            node("02RULeg", Some(0)),
            node("03RLLeg", Some(1)),
            node("08Belly", Some(0)),
            node("09Chest", Some(3)),
            node("10Head", Some(4)),
            node("11RShld", Some(4)),
            node("12RUArm", Some(6)),
        ];
        nodes[0].chunk_kind = 0;
        Body::new(nodes)
    }

    /// A garment with a chest node on layer bits `06` and a shoulder node on
    /// `06`: its group chooses the chest, then the shoulder, with the cull
    /// bits the slot masks give.
    #[test]
    fn a_garment_merges_its_chosen_slots_in_slot_order() {
        let mut chosen = [None; 19];
        chosen[9] = Some(4);
        chosen[11] = Some(6);
        let steps = garment_steps(&chosen, !0);
        assert_eq!(
            steps,
            [
                MergeStep {
                    node: 4,
                    cull: 0,
                    overlay: false
                },
                // The shoulder's cull masks are both zero: the slot always
                // takes the second bit.
                MergeStep {
                    node: 6,
                    cull: 0x40_0000,
                    overlay: false
                },
            ]
        );
        // The shoulder's second cull mask misses a visible mask without its
        // partial bit: its node takes `0x400000`.
        let visible = !(slot_bit(11) | 0x40_0000);
        let steps = garment_steps(&chosen, visible);
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].node, 4);
        // Hiding every bit of the chest's select mask drops the chest; the
        // shoulder stays.
        let steps = garment_steps(&chosen, !0x20_2500);
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].node, 6);
    }

    /// The append order is the slot number, not the slot tree: the right
    /// hand (17) follows the left arm (14 to 16), as the skin builder's
    /// slot loop appends them.
    #[test]
    fn the_merge_follows_the_slot_numbers() {
        let mut chosen = [None; 19];
        for slot in [13, 14, 17, 18] {
            chosen[slot] = Some(100 + slot);
        }
        let nodes = |steps: Vec<MergeStep>| steps.iter().map(|s| s.node).collect::<Vec<_>>();
        assert_eq!(nodes(garment_steps(&chosen, !0)), [113, 114, 117, 118]);
        assert_eq!(nodes(body_steps(&chosen, !0)), [113, 114, 117, 118]);
    }

    /// The body's slots merge in the same slot order, gated by the body's
    /// visible mask, with no cull bits; the overlays follow last.
    #[test]
    fn the_body_and_the_overlays_follow_the_garments() {
        let mut b = body();
        b.nodes[7].slot = 19;
        b.nodes[7].render_flags = 0x40_0040;
        let mut slots = [None; 19];
        for (slot, entry) in slots.iter_mut().enumerate().skip(1) {
            if let Some(i) = (0..b.nodes.len()).find(|&i| b.nodes[i].slot == slot) {
                *entry = Some(i);
            }
        }
        let steps = body_steps(&slots, !0);
        assert_eq!(
            steps.iter().map(|s| s.node).collect::<Vec<_>>(),
            [0, 1, 2, 3, 4, 5, 6]
        );
        assert!(steps.iter().all(|s| s.cull == 0 && !s.overlay));
        // Hiding the head's slot drops it.
        let steps = body_steps(&slots, !(slot_bit(10) | 0x1000_0000));
        assert!(!steps.iter().any(|s| s.node == 5));
        // The pelvis garment and the whole plan.
        let mut occupancy = [0u64; 18];
        occupancy[0] = 0x01;
        let garment = WornGarment {
            roots: vec![0],
            occupancy,
            coverage: occupancy,
            slots: slot_bit(1),
            material: 0,
        };
        let plan = dress(&b, 0, &[garment], &slots);
        assert_eq!(plan.steps.last().unwrap().node, 7);
        assert!(plan.steps.last().unwrap().overlay);
        assert_eq!(plan.steps[0].node, 0);
        assert_eq!(plan.steps.len(), 1 + 7 + 1);
        assert_eq!(plan.first_apparel_of(|_| vec!["skinhuman".into()]), None);
        // The body-only branch merges every slot the table holds, ungated.
        let only = body_only_steps(&b, 0, &[None; 19]);
        assert_eq!(only.len(), 1);
        assert!(only[0].overlay);
        b.nodes[7].render_flags = 0;
        assert!(body_only_steps(&b, 0, &[None; 19]).is_empty());
    }

    /// The first run-time material in merge order wins; a body material
    /// (`skinhuman`) is not an apparel texture.
    #[test]
    fn the_first_runtime_material_is_the_apparel_layer() {
        let steps = [
            MergeStep {
                node: 3,
                cull: 0,
                overlay: false,
            },
            MergeStep {
                node: 5,
                cull: 0,
                overlay: false,
            },
        ];
        let names = |node: usize| match node {
            3 => vec!["skinhuman".to_string()],
            _ => vec!["&doublet_ba1".to_string(), "&other".to_string()],
        };
        assert_eq!(
            first_apparel(&steps, names).as_deref(),
            Some("&doublet_ba1")
        );
        assert_eq!(first_apparel(&steps, |_| vec![]), None);
    }

    /// The shipped human body and a helm (item 3, `helm_b`), assembled as
    /// the builder does: the helm's head node is chosen and merged first,
    /// then the body's slots in slot order (skipped when the game is not
    /// installed).
    #[test]
    fn the_shipped_human_dresses_in_a_helm() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let resource = game.pack(exa_assets::PackKind::Resource).unwrap();
        let human = exa_formats::rfc::Scene::parse(resource.get("humanbase.rfc").unwrap()).unwrap();
        let from_scene = |nodes: &[exa_formats::rfc::Node]| -> Vec<Node> {
            nodes
                .iter()
                .map(|n| Node {
                    name: n.name.clone(),
                    // The scene's parents are 1-based.
                    parent: n.parent.map(|p| p - 1),
                    chunk_kind: if n.kind == exa_formats::rfc::NODE_MESH {
                        MESH
                    } else {
                        n.kind
                    },
                    slot: crate::layers::slot_of(&n.name).unwrap_or(0),
                    ..Node::default()
                })
                .collect()
        };
        let mut body = Body::new(from_scene(&human.nodes));
        let filled = body.chain_slots(0, true, 18);
        // The chain's table is indexed by slot − 1; the slot table the
        // merge takes is indexed by slot.
        let mut table = [None; 19];
        table[1..=18].copy_from_slice(&filled[..18]);
        assert_eq!(
            table[9].map(|i| body.nodes[i].name.clone()).as_deref(),
            Some("09Chest")
        );
        let mut slot_world = [None; 19];
        for slot in 1..=18 {
            slot_world[slot] = table[slot].map(|i| body.nodes[i].world);
        }
        let db = exa_formats::rdb::Database::parse(resource.get("objdb.rdb").unwrap()).unwrap();
        let items = exa_formats::objdb::items(&db);
        let helm = items.iter().find(|i| i.id == 3).unwrap();
        assert_eq!(helm.worn_model.to_ascii_lowercase(), "helm_b");
        let chars = game.pack(exa_assets::PackKind::Characters).unwrap();
        let garment = exa_formats::rfc::Scene::parse(chars.get(&helm.worn_model).unwrap()).unwrap();
        let base = body.nodes.len();
        for (i, n) in from_scene(&garment.nodes).into_iter().enumerate() {
            let mut n = n;
            n.parent = garment.nodes[i].parent.map(|p| p - 1 + base);
            body.nodes.push(n);
        }
        body.refresh_links();
        assert_ne!(body.reframe_from_slots(base, &slot_world), 0);
        let slots = (1..=18)
            .filter(|&slot| {
                body.descendants(base)
                    .iter()
                    .any(|&i| body.nodes[i].slot == slot)
            })
            .fold(0, |m, slot| m | crate::layers::slot_bit(slot));
        let mut words = [0u64; 18];
        let mut occupancy = [0u64; 18];
        for slot in 1..=18 {
            words[slot - 1] = helm.slot_word(slot);
            occupancy[slot - 1] = helm.occupancy[slot - 1];
        }
        let worn = WornGarment {
            roots: vec![base],
            occupancy,
            coverage: words,
            slots,
            material: helm.armour.map_or(0, |a| a.material),
        };
        let plan = dress(&body, 0, &[worn], &table);
        // The helm's head word is 0x83 on every layer: the body's own head
        // slot bit is hidden, but the head's select mask keeps its partial
        // bits, so the body's head still merges, after the helm.
        assert_eq!(words[9] & 0xFF, 0x83);
        assert_eq!(plan.body_visible & slot_bit(10), 0);
        assert_ne!(plan.body_slots & slot_bit(10), 0);
        assert_eq!(plan.garments[0].chosen[10].map(|i| i >= base), Some(true));
        assert!(plan.steps[0].node >= base);
        assert_eq!(plan.steps.len(), 1 + 18);
        // The body follows in slot order.
        let body_order: Vec<usize> = plan.steps[1..]
            .iter()
            .map(|s| body.nodes[s.node].slot)
            .collect();
        assert_eq!(body_order, (1..=18).collect::<Vec<_>>());
        // The head's impact material is the helm's own (plate, 4).
        assert_eq!(helm.armour.unwrap().material, 4);
        assert_eq!(plan.slot_material[9], helm.armour.unwrap().material);
    }

    /// A built body and garment assemble: the garment's node re-frames on
    /// the body's slot frame, the merge list puts it before the body's
    /// visible slots, and the frame the merge places it by is the body
    /// part's own (the pelvis at 1 m and the belly 10 cm above it).
    #[test]
    fn a_built_body_and_garment_assemble_on_the_slot_frames() {
        use bevy::prelude::Transform;

        fn built(
            name: &str,
            parent: Option<usize>,
            translation: [f32; 3],
            mesh: bool,
        ) -> exa_assets::ModelNode {
            exa_assets::ModelNode {
                name: name.into(),
                parent,
                transform: Transform::from_translation(translation.into()),
                parts: Vec::new(),
                part_materials: Vec::new(),
                collision: Vec::new(),
                skin_bone: Some(1),
                light: None,
                voxel_light: None,
                fire: None,
                portal: None,
                bounds: (bevy::prelude::Vec3::ZERO, 0.0),
                half_extents: bevy::prelude::Vec3::ZERO,
                gi_points: Default::default(),
                rigid: None,
                tracks: Vec::new(),
                shadow_bias: 0,
                is_skin: false,
                placeholder: false,
                cloth: false,
                mesh,
                render_flags: 0,
            }
        }
        let body = Model {
            nodes: vec![
                built("01Pelvis", None, [0.0, 1.0, 0.0], true),
                built("08Belly", Some(0), [0.0, 0.1, 0.0], true),
            ],
        };
        let garment = Model {
            nodes: vec![built("0800Belly", None, [0.0, 0.0, 0.0], true)],
        };
        let mut occupancy = [0u64; 18];
        occupancy[7] = 1;
        let mut coverage = [0u64; 18];
        coverage[7] = 1;
        let worn = WornModel {
            model: &garment,
            occupancy,
            coverage,
            material: 2,
        };
        let assembled = assemble(&body, &[worn]).expect("a body with a root");
        let nodes: Vec<usize> = assembled.plan.steps.iter().map(|s| s.node).collect();
        // The garment's belly node, then the body's pelvis and belly.
        assert_eq!(nodes, [2, 0, 1]);
        assert_eq!(assembled.model_of(assembled.plan.steps[0]), (1, 0));
        assert_eq!(assembled.model_of(assembled.plan.steps[1]), (0, 0));
        // The garment node sits on the belly's world frame: 1.1 m up.
        let frame = assembled.body.nodes[2].world;
        assert!((frame[3][1] - 1.1).abs() < 1e-5, "{frame:?}");
        assert_eq!(assembled.plan.slot_material[7], 2);
        // The frames compose down the body's tree.
        let frames = model_frames(&body);
        assert_eq!(frames[0][3], [0.0, 1.0, 0.0]);
        assert_eq!(frames[1][3], [0.0, 1.1, 0.0]);
    }

    /// One of `exanima01`'s living characters, dressed from its own record
    /// and items, merges into one skinned mesh: its body's visible slots,
    /// its garments' chosen nodes and the body's overlays, drawn as the
    /// original's material runs (skipped without the game).
    #[test]
    fn a_shipped_character_merges_into_one_skinned_mesh() {
        use exa_assets::{ModelCache, ModelLoader, PackKind};
        use exa_formats::morph::Physique;

        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let resource = game.pack(PackKind::Resource).unwrap();
        let scene = exa_formats::rfc::Scene::parse(resource.get("exanima01.rfc").unwrap()).unwrap();
        let db = exa_formats::rdb::Database::parse(resource.get("objdb.rdb").unwrap()).unwrap();
        let items = exa_formats::objdb::items(&db);
        let locale =
            exa_formats::roles::locale_id(resource.get("locales.rdb").unwrap(), "exanima01.rfc");
        let characters = exa_world::placed_characters(&scene, &items, locale);
        let races = exa_formats::races::parse(resource.get("races.rdb").unwrap()).unwrap();
        let Some(character) = characters.iter().find(|c| c.name == "Kade") else {
            return;
        };
        let race = exa_formats::races::of_character(&races, character.race);
        let model_name = race.map_or_else(|| "humanbase.rfc".to_string(), |r| r.model());
        let morph =
            exa_assets::BodyMorph::of_model(&model_name, Physique::of_record(&character.record));
        let mut cache = ModelCache::default();
        let mut meshes = bevy::prelude::Assets::<bevy::prelude::Mesh>::default();
        let mut materials = bevy::prelude::Assets::<exa_render::material::ExaMaterial>::default();
        let mut images = bevy::prelude::Assets::<bevy::prelude::Image>::default();
        let mut loader = ModelLoader {
            game: &game,
            cache: &mut cache,
            meshes: &mut meshes,
            materials: &mut materials,
            images: &mut images,
            bc_supported: false,
        };
        let body = loader
            .morphed_model(PackKind::Resource, &model_name, Some(&morph))
            .unwrap();
        let infos = crate::item_info(&game);
        let mut models = Vec::new();
        let mut worn = Vec::new();
        for item in &character.worn {
            let Some(model) = loader.morphed_model(PackKind::Characters, &item.model, Some(&morph))
            else {
                continue;
            };
            let info = infos
                .get(&item.model.to_ascii_lowercase())
                .and_then(|i| i.apparel.clone());
            models.push(model);
            worn.push((
                item.model.to_ascii_lowercase(),
                info.as_ref().map_or([0; 18], |i| i.occupancy),
                info.as_ref().map_or([0; 18], |i| i.coverage),
                info.as_ref().map_or(0, |i| i.material),
            ));
        }
        let garments: Vec<WornModel<'_>> = worn
            .iter()
            .zip(&models)
            .map(|((_, occupancy, coverage, material), model)| WornModel {
                model,
                occupancy: *occupancy,
                coverage: *coverage,
                material: *material,
            })
            .collect();
        let assembled = assemble(&body, &garments).expect("a body with a root");
        let mut pieces = Vec::new();
        for step in &assembled.plan.steps {
            let (which, node) = assembled.model_of(*step);
            let model = if which == 0 {
                &body
            } else {
                &models[which - 1]
            };
            let node = &model.nodes[node];
            for ((mesh, handle), name) in node.parts.iter().zip(&node.part_materials) {
                pieces.push((
                    mesh.clone(),
                    handle.clone(),
                    name.clone(),
                    assembled.body.nodes[step.node].world,
                    node.skin_bone.map_or(0, u32::from),
                    step.cull,
                ));
            }
        }
        let parts: Vec<exa_assets::model::DressedPart<'_>> = pieces
            .iter()
            .map(
                |(mesh, handle, material, frame, bone, cull)| exa_assets::model::DressedPart {
                    mesh: meshes.get(mesh).unwrap(),
                    frame: *frame,
                    bone: *bone,
                    material,
                    handle: handle.clone(),
                    cull: *cull,
                },
            )
            .collect();
        let merged = exa_assets::model::merge_dressed(&parts).expect("a merged mesh");
        let names: Vec<&str> = merged
            .merged
            .subobjects
            .iter()
            .map(|s| s.material.as_str())
            .collect();
        // Kade's five garments' chosen nodes, the body's eighteen slots and
        // its two overlays; eight materials, the body's skin and eyes, the
        // dark under-layer and the five painted garments.
        assert_eq!(assembled.plan.steps.len(), 30);
        assert_eq!(pieces.len(), 35);
        // The runs take their materials' first-seen order: trousers first
        // (the innermost layer), then the shirt, the tunic's dark lining,
        // the sandals, the long shirt and the waistcoat, then the body's
        // skin and eyes.
        assert_eq!(
            names,
            [
                "&trouser_aa1",
                "&shirty__aa1",
                "_dark",
                "&sandals_da0",
                "&tunicls_aa1",
                "&wstcoat_aa1",
                "skinhuman",
                "eyehuman",
            ]
        );
        assert_eq!(merged.merged.mesh.positions.len(), 5624);
        assert_eq!(merged.merged.mesh.indices.len(), 25884);
        assert_eq!(merged.materials.len(), merged.merged.subobjects.len());
        // Every run's indices stay inside its own vertices, and the runs
        // together hold every merged vertex.
        let mut total = 0;
        for run in &merged.merged.subobjects {
            let first = run.first_vertex;
            let end = first + run.vertex_count;
            let slice = &merged.merged.mesh.indices[run.first_index..][..run.index_count];
            assert!(slice.iter().all(|i| (first..end).contains(&(*i as usize))));
            total += run.vertex_count;
        }
        assert_eq!(total, merged.merged.mesh.positions.len());
        assert!(
            merged
                .merged
                .mesh
                .skin
                .as_ref()
                .is_some_and(|s| s.len() == merged.merged.mesh.positions.len())
        );
    }

    /// A transform turned into the file's frame and back is the same
    /// transform, and composing frames places a child as Bevy's hierarchy
    /// does: a child one metre along x under a parent turned a quarter
    /// about z sits one metre along y.
    #[test]
    fn file_frames_compose_as_the_hierarchy() {
        use bevy::math::{Quat, Vec3};
        let parent = Transform::from_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2))
            .with_translation(Vec3::new(0.0, 1.0, 0.0));
        let child = Transform::from_xyz(1.0, 0.0, 0.0).with_rotation(Quat::from_rotation_x(0.3));
        let back = exa_assets::transform_from_rows(&file_frame(&parent));
        assert!(back.rotation.angle_between(parent.rotation) < 1e-5);
        assert!(back.translation.distance(parent.translation) < 1e-6);
        let world = exa_formats::morph::compose(&file_frame(&parent), &file_frame(&child));
        let expected = parent * child;
        assert!(Vec3::from(world[3]).distance(Vec3::new(0.0, 2.0, 0.0)) < 1e-6);
        assert!(Vec3::from(world[3]).distance(expected.translation) < 1e-6);
        let x = Vec3::from(world[0]);
        assert!(x.distance(expected.rotation * Vec3::X) < 1e-6);
        let y = Vec3::from(world[1]);
        assert!(y.distance(expected.rotation * Vec3::Y) < 1e-6);
    }
}
