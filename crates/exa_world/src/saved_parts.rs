//! The owned nodes of a placed object or prop, as a save's part records
//! read them back from the running scene.
//!
//! The original's exporter writes, for a prop or a placed object, one part
//! record per owned child node that carries an attachment, nested under its
//! parent's record: a rigid body's node writes `0x3DE0ECAC` (its name, the
//! attachment's word, its live world matrix) and a cloth's node `0x3DE0ECDB`
//! (its name, the word, the live cloth particles); a child without an
//! attachment is passed over with everything under it, and a child whose
//! attachment is neither a body nor a cloth (an emitter) ends its siblings'
//! records. Every node under a placed object's or prop's root is owned.
//!
//! Exarust keeps the model's tree on the spawned root ([`PartTree`]): each
//! node's entity, parent, name, the attachment its model node carries and
//! its local transform as the load placed it, so the save can tell which
//! owned nodes moved against their parents since (`exa_game::level_save`).

use bevy::math::Affine3A;
use bevy::prelude::*;

use exa_assets::Model;

/// What a model node carries in the original's attachment slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartKind {
    /// No attachment: the part walk passes over the node and its subtree.
    None,
    /// A rigid body (attachment class `0x10…`): a `0x3DE0ECAC` record.
    Body,
    /// A cloth (class `0x60…`): a `0x3DE0ECDB` record.
    Cloth,
    /// An emitter (fire or portal, class `0xA0…`): ends the siblings'
    /// records.
    Emitter,
}

/// One node of a spawned model, as the save's part walk reads it.
#[derive(Debug, Clone, PartialEq)]
pub struct PartNode {
    pub entity: Entity,
    /// Its parent's index in [`PartTree::nodes`]; `None` for a root.
    pub parent: Option<usize>,
    pub name: String,
    pub kind: PartKind,
    /// Its local transform as the load spawned it (the placement's records
    /// applied).
    pub spawned: Affine3A,
}

/// On a placed object's or prop's spawned root: its model's nodes in model
/// order. The first root stands for the original's object node.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct PartTree {
    pub nodes: Vec<PartNode>,
}

impl PartTree {
    /// The tree of `model` spawned as `entities` ([`crate::spawn_model_nodes_at`]).
    /// `None` when no node carries a body or a cloth, so nothing can write a
    /// part record.
    pub fn of(model: &Model, entities: &[Entity]) -> Option<Self> {
        if entities.len() != model.nodes.len() {
            return None;
        }
        let nodes: Vec<PartNode> = model
            .nodes
            .iter()
            .zip(entities)
            .map(|(node, &entity)| PartNode {
                entity,
                parent: node.parent,
                name: node.name.clone(),
                kind: kind_of(node),
                spawned: Affine3A::from_mat4(node.transform.to_matrix()),
            })
            .collect();
        nodes
            .iter()
            .any(|n| matches!(n.kind, PartKind::Body | PartKind::Cloth))
            .then_some(Self { nodes })
    }

    /// The node standing for the original's object node: the first root.
    pub fn root(&self) -> Option<usize> {
        self.nodes.iter().position(|n| n.parent.is_none())
    }

    /// The direct children of `at`, in model order.
    pub fn children(&self, at: usize) -> impl Iterator<Item = usize> + '_ {
        (0..self.nodes.len()).filter(move |&i| i != at && self.nodes[i].parent == Some(at))
    }
}

/// A model node's attachment, in the reader's order: an emitter is the
/// node's body, else a rigid body, else the cloth marker.
pub fn kind_of(node: &exa_assets::ModelNode) -> PartKind {
    if node.fire.is_some() || node.portal.is_some() {
        PartKind::Emitter
    } else if node.rigid.is_some() {
        PartKind::Body
    } else if node.cloth {
        PartKind::Cloth
    } else {
        PartKind::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(name: &str, parent: Option<usize>) -> exa_assets::ModelNode {
        crate::tests::node(name, parent, None)
    }

    #[test]
    fn a_model_without_bodies_or_cloth_has_no_tree() {
        let mut model = Model {
            nodes: vec![node("root", None), node("lamp", Some(0))],
        };
        model.nodes[1].fire = Some(1.0);
        let entities = [
            Entity::from_raw_u32(1).unwrap(),
            Entity::from_raw_u32(2).unwrap(),
        ];
        assert!(PartTree::of(&model, &entities).is_none());
        model.nodes[1].cloth = true;
        // A fire is the node's attachment before a cloth marker.
        assert!(PartTree::of(&model, &entities).is_none());
        model.nodes[1].fire = None;
        let tree = PartTree::of(&model, &entities).unwrap();
        assert_eq!(tree.root(), Some(0));
        assert_eq!(tree.children(0).collect::<Vec<_>>(), vec![1]);
        assert_eq!(tree.nodes[1].kind, PartKind::Cloth);
    }
}
