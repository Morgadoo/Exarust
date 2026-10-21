//! Scene-node content: a drawable node's parts and their per-node scales,
//! shared masters and their instances, and removing a node tree.
//!
//! A drawable node (the original's `TGeometry`) owns a list of sub-objects
//! (parts) and their materials, an emissive scale (`+0x244`), a fade scale
//! (`+0x228`) and a cast word. Spawning a model creates one part entity per
//! mesh under its node; this module groups them so a scale can be applied
//! without copying the model.
//!
//! **Instancing** (`0x100050620`, `0x100054410`): a node can share another
//! node's parts as an *instance*. The master's instance list names every
//! instance; deleting a master hands the role to its last live instance
//! (or frees the instance) before the master goes. Syncing an instance
//! copies its master's flags, parts and scales (the original can also
//! deep-copy the parts instead of sharing them, which Exarust never needs:
//! parts are shared assets already).
//!
//! **Removal** (`0x100050a60`): deleting a node tree unlinks the node from
//! its parent and sector, hands its master role on, then either frees it or
//! queues it for the scene's deferred-removal ring. The ring holds 256
//! nodes; a full ring reports "Node removal queue is full." and the delete
//! falls back to freeing at once. A node with children moves its master
//! role to the last live child and frees itself, then recurses.
//!
//! Rules: `docs/ARCHITECTURE.md` (nodes and parts).

use bevy::prelude::*;

/// The removal ring's capacity.
pub const REMOVAL_RING: usize = 256;

/// A node's part entities and the scales the original keeps on the node.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct NodeParts {
    /// One entity per sub-object, in the model's part order.
    pub parts: Vec<Entity>,
    /// The node's emissive scale (`+0x244`); 1 is the stored material.
    pub emissive_scale: f32,
    /// The node's fade scale (`+0x228`): 1 solid, 0 fully faded.
    pub fade_scale: f32,
    /// The cast flags (`+0xf0` of each sub-object is the material's; the
    /// node's word is kept here): bit `0x80000` means the parts draw
    /// themselves, which clears their cast byte.
    pub cast_flags: u32,
}

impl NodeParts {
    /// A node's parts with the original's default scales (both 1: the
    /// constructor sets emissive and fade to 1).
    pub fn new(parts: Vec<Entity>) -> Self {
        Self {
            parts,
            emissive_scale: 1.0,
            fade_scale: 1.0,
            cast_flags: 0,
        }
    }

    /// Whether the node draws at all at its fade scale: a fully faded node
    /// is dropped.
    pub fn visible(&self) -> bool {
        self.fade_scale > 0.0
    }

    /// The share of the parts' colour the fade keeps.
    pub fn solid(&self) -> f32 {
        self.fade_scale.clamp(0.0, 1.0)
    }

    /// An instance's parts from its master: the same part entities and
    /// scales, unless the instance keeps its own parts (the original's
    /// deep copy, which Exarust's shared mesh assets never need).
    pub fn shared_from(master: &Self, own_parts: bool) -> Self {
        if own_parts {
            Self::new(Vec::new())
        } else {
            Self {
                parts: master.parts.clone(),
                ..master.clone()
            }
        }
    }
}

/// A node whose parts another node owns: the shared master.
#[derive(Component, Debug, Default, Clone, PartialEq)]
pub struct NodeInstances {
    /// The instance entities, in registration order.
    pub instances: Vec<Entity>,
    /// The node's own master when it is itself an instance.
    pub master: Option<Entity>,
    /// The original's "deleting" flag (`+8` bit 31).
    pub deleting: bool,
    /// The node is owned by another system (`+0xC` bit 0): a delete does
    /// not unregister it from its parent list.
    pub owned: bool,
}

impl NodeInstances {
    /// Registers `instance` under this master (the original's instance
    /// list push and the `0x20000` flag).
    pub fn add(&mut self, instance: Entity) {
        if !self.instances.contains(&instance) {
            self.instances.push(instance);
        }
    }

    /// Removes an instance; the list drops its `0x20000` flag when empty
    /// (the original clears it).
    pub fn remove(&mut self, instance: Entity) -> bool {
        let len = self.instances.len();
        self.instances.retain(|&e| e != instance);
        self.instances.len() != len
    }

    /// Hands the master role to `heir`, the original's slot 13
    /// (`0x100050620`): the heir takes the instance list, every instance
    /// is repointed, and the old master's list empties. With `drop_last`
    /// the heir's own last slot goes: a list of one or none leaves the
    /// heir with no instances, otherwise every slot but the last is kept.
    /// Returns the instances that now hang from the heir.
    pub fn hand_off(&mut self, heir: Entity, drop_last: bool) -> Vec<Entity> {
        let mut list = std::mem::take(&mut self.instances);
        if drop_last {
            // The heir's count is the old count less one: the last slot
            // goes, and a list of one or none leaves nothing.
            if list.len() >= 2 {
                list.pop();
            } else {
                list.clear();
            }
        } else {
            list.retain(|&e| e != heir);
        }
        list
    }

    /// The instance a delete hands the master role to: the last live one,
    /// as the original scans its list backwards for a node not deleting.
    pub fn last_live(&self, deleting: impl Fn(Entity) -> bool) -> Option<Entity> {
        self.instances.iter().rev().copied().find(|&e| !deleting(e))
    }
}

/// A node sharing another node's parts: the master it follows and whether
/// its parts are its own deep copy (the original's flag `0x4000` on the
/// instance; Exarust shares the mesh assets, so a copy only means the
/// instance keeps its own part entities).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeInstance {
    pub master: Entity,
    pub own_parts: bool,
}

/// The scene's deferred-removal ring: 256 nodes queued for the frame's
/// removal pass. A full ring is an error (the original reports "Node
/// removal queue is full." and frees the node at once).
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct RemovalQueue {
    slots: [Option<Entity>; REMOVAL_RING],
    head: usize,
    count: usize,
    /// The number of times a node could not be queued because the ring was
    /// full.
    pub full_errors: u32,
}

impl Default for RemovalQueue {
    fn default() -> Self {
        Self {
            slots: [None; REMOVAL_RING],
            head: 0,
            count: 0,
            full_errors: 0,
        }
    }
}

impl RemovalQueue {
    /// Queues a node. A full ring counts the error and returns `false`; the
    /// caller then frees the node at once.
    pub fn queue(&mut self, node: Entity) -> bool {
        if self.count == REMOVAL_RING {
            self.full_errors = self.full_errors.saturating_add(1);
            return false;
        }
        let at = (self.head + self.count) % REMOVAL_RING;
        self.slots[at] = Some(node);
        self.count += 1;
        true
    }

    /// Takes the oldest queued node.
    pub fn pop(&mut self) -> Option<Entity> {
        if self.count == 0 {
            return None;
        }
        let node = self.slots[self.head].take();
        self.head = (self.head + 1) % REMOVAL_RING;
        self.count -= 1;
        node
    }

    /// The number of queued nodes.
    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Empties the ring (a level change).
    pub fn clear(&mut self) {
        self.slots = [None; REMOVAL_RING];
        self.head = 0;
        self.count = 0;
    }
}

pub struct NodeLifePlugin;

impl Plugin for NodeLifePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RemovalQueue>().add_systems(
            PostUpdate,
            (
                record_part_emissive,
                sync_instances,
                apply_emissive_scales,
                apply_fade_scales,
                drain_removals,
            )
                .chain(),
        );
    }
}

/// Records a part's material emissive once, so a node's scale multiplies
/// the stored value rather than its own previous result.
fn record_part_emissive(
    mut commands: Commands,
    added: Query<
        (Entity, &MeshMaterial3d<exa_render::material::ExaMaterial>),
        Added<MeshMaterial3d<exa_render::material::ExaMaterial>>,
    >,
    materials: Res<Assets<exa_render::material::ExaMaterial>>,
) {
    for (entity, handle) in &added {
        if let Some(material) = materials.get(handle) {
            commands
                .entity(entity)
                .insert(PartEmissive(material.params.emissive));
        }
    }
}

/// Multiplies each part's base emissive by its node's scale. Parts keep
/// their material's base emissive in [`PartEmissive`], so repeated changes
/// do not compound.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct PartEmissive(pub Vec4);

/// Gives an instance its master's parts and scales (the original's sync,
/// `0x1000511c0`/`0x100054410`).
fn sync_instances(
    mut commands: Commands,
    new: Query<(Entity, &NodeInstance), Added<NodeInstance>>,
    masters: Query<&NodeParts>,
) {
    for (entity, instance) in &new {
        let Ok(master) = masters.get(instance.master) else {
            continue;
        };
        commands
            .entity(entity)
            .insert(NodeParts::shared_from(master, instance.own_parts));
    }
}

/// Applies a node's emissive scale to its parts.
fn apply_emissive_scales(
    nodes: Query<&NodeParts, Changed<NodeParts>>,
    bases: Query<&PartEmissive>,
    handles: Query<&MeshMaterial3d<exa_render::material::ExaMaterial>>,
    mut materials: ResMut<Assets<exa_render::material::ExaMaterial>>,
) {
    for node in &nodes {
        for &part in &node.parts {
            let (Ok(base), Ok(handle)) = (bases.get(part), handles.get(part)) else {
                continue;
            };
            if let Some(mut material) = materials.get_mut(handle) {
                material.params.emissive =
                    (base.0.truncate() * node.emissive_scale).extend(base.0.w);
            }
        }
    }
}

/// Sets a part's visibility from its node's fade scale.
fn apply_fade_scales(
    nodes: Query<&NodeParts, Changed<NodeParts>>,
    mut visibility: Query<&mut Visibility>,
) {
    for node in &nodes {
        for &part in &node.parts {
            if let Ok(mut v) = visibility.get_mut(part) {
                *v = if node.visible() {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
            }
        }
    }
}

/// Removes the nodes the ring has queued, one tree at a time. A queued
/// node's children are removed with it; its master role goes to its last
/// live instance first.
fn drain_removals(
    mut commands: Commands,
    mut queue: ResMut<RemovalQueue>,
    mut links: Query<&mut NodeInstances>,
    children: Query<&Children>,
    instances: Query<&NodeInstance>,
) {
    while let Some(root) = queue.pop() {
        let mut stack = vec![root];
        let mut first = true;
        while let Some(node) = stack.pop() {
            if first {
                // The master role passes to the last live instance; the
                // instance stops being an instance of a node that is going.
                let hand_off = links.get(node).ok().and_then(|link| {
                    let heir = link
                        .instances
                        .iter()
                        .rev()
                        .copied()
                        .find(|&e| instances.get(e).is_ok_and(|i| i.master == node))?;
                    let mut copy = link.clone();
                    let moved = copy.hand_off(heir, true);
                    Some((heir, moved))
                });
                if let Some((heir, moved)) = hand_off {
                    if let Ok(mut heir_link) = links.get_mut(heir) {
                        heir_link.instances = moved;
                    }
                    commands.entity(heir).remove::<NodeInstance>();
                }
                if let Ok(mut link) = links.get_mut(node) {
                    link.instances.clear();
                }
                first = false;
            }
            if let Ok(list) = children.get(node) {
                stack.extend(list.iter());
            }
            commands.entity(node).remove::<NodeInstance>();
            commands.entity(node).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entity(raw: u32) -> Entity {
        Entity::from_raw_u32(raw).unwrap()
    }

    #[test]
    fn an_instance_shares_its_masters_parts_and_scales() {
        let mut master = NodeParts::new(vec![entity(1), entity(2)]);
        master.emissive_scale = 0.5;
        master.fade_scale = 0.25;
        master.cast_flags = 0x80000;
        let shared = NodeParts::shared_from(&master, false);
        assert_eq!(shared.parts, master.parts);
        assert_eq!(shared.emissive_scale, 0.5);
        assert_eq!(shared.fade_scale, 0.25);
        assert_eq!(shared.cast_flags, 0x80000);
        let own = NodeParts::shared_from(&master, true);
        assert!(own.parts.is_empty());
        assert_eq!(own.emissive_scale, 1.0);
    }

    #[test]
    fn a_node_starts_solid_with_unit_scales() {
        let node = NodeParts::new(vec![entity(1), entity(2)]);
        assert_eq!(node.emissive_scale, 1.0);
        assert_eq!(node.fade_scale, 1.0);
        assert_eq!(node.solid(), 1.0);
        assert!(node.visible());
    }

    #[test]
    fn the_fade_scale_drops_the_parts_at_one() {
        let mut node = NodeParts::new(Vec::new());
        node.fade_scale = 0.25;
        assert!(node.visible());
        assert!((node.solid() - 0.25).abs() < 1e-6);
        node.fade_scale = 1.0;
        assert!(node.visible());
        node.fade_scale = 0.0;
        assert!(!node.visible());
        assert_eq!(node.solid(), 0.0);
        node.fade_scale = 2.0;
        assert_eq!(node.solid(), 1.0);
    }

    #[test]
    fn the_instance_list_registers_and_unregisters() {
        let mut link = NodeInstances::default();
        link.add(entity(1));
        link.add(entity(2));
        link.add(entity(1));
        assert_eq!(link.instances.len(), 2);
        assert!(link.remove(entity(1)));
        assert!(!link.remove(entity(3)));
        assert_eq!(link.instances, [entity(2)]);
    }

    #[test]
    fn a_master_hands_its_role_to_an_instance() {
        let mut link = NodeInstances::default();
        link.add(entity(1));
        link.add(entity(2));
        link.add(entity(3));
        // Without dropping a slot the heir takes all other instances.
        let moved = link.hand_off(entity(3), false);
        assert_eq!(moved, [entity(1), entity(2)]);
        assert!(link.instances.is_empty());
        // With a single instance and the last slot dropped, the heir has
        // nothing left to follow.
        let mut one = NodeInstances::default();
        one.add(entity(4));
        assert!(one.hand_off(entity(4), true).is_empty());
        // Two instances: the heir keeps the other one.
        let mut two = NodeInstances::default();
        two.add(entity(5));
        two.add(entity(6));
        assert_eq!(two.hand_off(entity(6), true), [entity(5)]);
    }

    #[test]
    fn the_last_live_instance_skips_deleting_nodes() {
        let mut link = NodeInstances::default();
        for id in [1, 2, 3] {
            link.add(entity(id));
        }
        assert_eq!(link.last_live(|_| false), Some(entity(3)));
        assert_eq!(link.last_live(|e| e == entity(3)), Some(entity(2)));
        assert_eq!(link.last_live(|e| e != entity(1)), Some(entity(1)));
        let empty = NodeInstances::default();
        assert_eq!(empty.last_live(|_| false), None);
    }

    #[test]
    fn the_removal_ring_holds_256_nodes_in_order() {
        let mut queue = RemovalQueue::default();
        for i in 0..REMOVAL_RING as u32 {
            assert!(queue.queue(entity(i)));
        }
        assert_eq!(queue.len(), REMOVAL_RING);
        assert!(!queue.queue(entity(999)));
        assert_eq!(queue.full_errors, 1);
        assert_eq!(queue.pop(), Some(entity(0)));
        assert!(queue.queue(entity(999)));
        for i in 1..REMOVAL_RING as u32 {
            assert_eq!(queue.pop(), Some(entity(i)));
        }
        assert_eq!(queue.pop(), Some(entity(999)));
        assert!(queue.pop().is_none());
        queue.clear();
        assert!(queue.is_empty());
    }
}
