//! The dressed character mesh: the order the original appends the body and
//! its garments into one skinned mesh, the cull bits each slot writes on its
//! triangles, and the overlay meshes past the eighteen body slots.
//!
//! The builder assembles each garment into a chosen node per slot
//! ([`crate::layers`]); this module turns that choice into the merge the
//! skinning builder runs: the chosen nodes are appended in slot order (1 to
//! 18) where the slot's select mask meets the garment's visible mask, and a garment's slots take the cull bits of
//! [`crate::layers::cull_bits`]. The body's own visible slots follow, without
//! cull bits, and the overlays (hair and other meshes numbered past the
//! slots) come last. [`crate::rebuild::dress_plan`] gives the assembly; this
//! module is the merge list a caller with model meshes turns into
//! `exa_assets::merge::merge_parts` parts.
//!
//! Exarust still draws one mesh per node, so nothing in the running game
//! calls this yet; it is the tested list the original merges by.

use crate::layers::{cull_bits, slot_selected};
use crate::rebuild::WornGarment;
use crate::runtime::{Body, overlay_nodes};

/// One node of the merged mesh: the node whose meshes are appended, the
/// cull bits its triangles take, and whether it is an overlay (merged after
/// the body, past the slot tree).
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
    let mut steps = Vec::new();
    for garment in &plan.garments {
        steps.extend(garment_steps(&garment.chosen, garment.visible));
    }
    steps.extend(body_steps(body_slots, plan.body_visible));
    for node in overlay_nodes(body, root) {
        steps.push(MergeStep {
            node,
            cull: 0,
            overlay: true,
        });
    }
    DressedPlan {
        garments: plan.garments,
        body_visible: plan.body_visible,
        body_slots: plan.body_slots,
        slot_material: plan.slot_material,
        steps,
    }
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
}
