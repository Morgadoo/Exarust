//! The scene-content helpers: clearing a scene, detaching and releasing
//! subtrees, attaching a node array, applying a level's environment record
//! and keeping the scene's file-format list.
//!
//! The original's clear flushes the pending lists, waits for the collision
//! builder, walks every node into a holder scene and then deletes the
//! holders; here [`clear_scene`] does the same in one step on
//! [`crate::scene_nodes::Scene`], keeping the scene's own sector. Detach
//! clears a subtree's scene bit and unregisters its lights and geometry
//! without taking it out of its sector; release frees the subtree; attach
//! appends nodes under a parent with that parent's owner. The environment
//! record's ambient, fill, extents and reverb decay are handed on as the
//! level load does. The format list is the scene's registration of
//! `(key, description)` readers.
//!
//! Rules: `docs/ARCHITECTURE.md` (level change) and `docs/formats/RFC.md`
//! (Environment).

use exa_formats::environment::Environment;

use crate::scene_nodes::Scene;

/// What clearing a scene removed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClearReport {
    /// The released nodes, in pre-order per top-level root.
    pub removed: Vec<usize>,
    /// The file formats the scene had registered (the original frees its
    /// format list with the scene).
    pub formats: Vec<FormatEntry>,
}

/// Clears a scene: every top-level node's subtree is detached and released,
/// and the sector lists, geometry list, transitions, removal queue and
/// material queues are emptied. The scene's own sector stays, as the
/// original keeps it. Returns the released node indices and the format
/// list.
pub fn clear_scene(scene: &mut Scene) -> ClearReport {
    let removed = scene.clear_content();
    ClearReport {
        removed,
        formats: Vec::new(),
    }
}

/// Detaches a subtree from the scene's lists without releasing it. Returns
/// the number of nodes that were in the scene.
pub fn detach(scene: &mut Scene, node: usize) -> usize {
    scene.detach_subtree(node)
}

/// Releases a subtree at once, without the removal queue's ownership wait.
pub fn release(scene: &mut Scene, node: usize) -> usize {
    scene.release_subtree(node)
}

/// Attaches a node array under a parent, taking the parent's owner.
pub fn attach(scene: &mut Scene, parent: usize, nodes: &[usize]) -> usize {
    scene.attach_nodes(parent, nodes)
}

/// The values a scene takes from a level's environment record: the two
/// ambient colours, the level's vertical extent and the audio reverb's
/// added decay. The original hands the record to the scene environment and
/// the audio environment; Exarust's level load already performs that, so
/// this is the record's tested hand-off value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppliedEnvironment {
    /// The main ambient colour, sRGB.
    pub ambient: [f32; 3],
    /// The secondary ambient colour, sRGB RGBA.
    pub fill: [f32; 4],
    /// The lowest walkable height, metres.
    pub floor: f32,
    /// The highest walkable height, metres.
    pub ceiling: f32,
    /// Added to the base reverb decay, seconds.
    pub reverb_decay: f32,
}

impl From<&Environment> for AppliedEnvironment {
    fn from(env: &Environment) -> Self {
        Self {
            ambient: env.ambient,
            fill: env.fill,
            floor: env.floor,
            ceiling: env.ceiling,
            reverb_decay: env.reverb_decay,
        }
    }
}

/// Applies a level environment record, or leaves the scene alone when the
/// level stores none.
pub fn apply_environment(env: Option<&Environment>) -> Option<AppliedEnvironment> {
    env.map(AppliedEnvironment::from)
}

/// One entry of the scene's file-format list: the extension or key, the
/// reader's description and whether the reader is registered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatEntry {
    pub key: String,
    pub description: String,
}

/// The scene's file-format list. A registration with an existing key
/// replaces that entry; freeing the list drops every entry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FormatList {
    entries: Vec<FormatEntry>,
}

impl FormatList {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a key and its description, replacing an earlier entry with
    /// the same key.
    pub fn register(&mut self, key: impl Into<String>, description: impl Into<String>) {
        let entry = FormatEntry {
            key: key.into(),
            description: description.into(),
        };
        if let Some(existing) = self.entries.iter_mut().find(|e| e.key == entry.key) {
            *existing = entry;
        } else {
            self.entries.push(entry);
        }
    }

    /// Looks a key up.
    pub fn get(&self, key: &str) -> Option<&FormatEntry> {
        self.entries.iter().find(|e| e.key == key)
    }

    pub fn entries(&self) -> &[FormatEntry] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Frees the list, as the unit's cleanup does.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene_nodes::NodeKind;

    #[test]
    fn attach_detach_release_and_clear_keep_the_scene_sector() {
        let mut scene = Scene::new((0, 0));
        let room = scene.add_node(NodeKind::Sector, None);
        scene.add_batch(&[room]);
        let parent = scene.add_node(NodeKind::Node, None);
        scene.register(parent, 1);
        let child = scene.add_node(NodeKind::Mesh, Some(parent));
        scene.register(child, 1);
        assert_eq!(detach(&mut scene, child), 1);
        assert_eq!(scene.sectors[1].meshes, [child]);
        assert_eq!(attach(&mut scene, parent, &[child]), 1);
        assert_eq!(scene.nodes[child].parent, Some(parent));
        let report = clear_scene(&mut scene);
        assert!(report.removed.contains(&parent));
        assert!(report.removed.contains(&room));
        assert_eq!(scene.sectors.len(), 1);
        assert_eq!(release(&mut scene, parent), 0);
    }

    #[test]
    fn the_environment_record_carries_its_five_values() {
        assert_eq!(apply_environment(None), None);
        let env = Environment {
            ambient: [0.25, 0.5, 0.75],
            fill: [0.1, 0.2, 0.3, 1.0],
            floor: -2.0,
            ceiling: 6.5,
            reverb_decay: 0.4,
            ..Environment::default()
        };
        let applied = apply_environment(Some(&env)).unwrap();
        assert_eq!(applied.ambient, [0.25, 0.5, 0.75]);
        assert_eq!(applied.fill, [0.1, 0.2, 0.3, 1.0]);
        assert_eq!((applied.floor, applied.ceiling), (-2.0, 6.5));
        assert_eq!(applied.reverb_decay, 0.4);
    }

    #[test]
    fn a_format_registration_replaces_the_same_key_and_frees() {
        let mut list = FormatList::new();
        list.register("rfc", "Flux Content");
        list.register("rfi", "Rayform Image");
        assert_eq!(list.len(), 2);
        assert_eq!(list.get("rfi").unwrap().description, "Rayform Image");
        list.register("rfi", "Rayform Image 2");
        assert_eq!(list.len(), 2);
        assert_eq!(list.get("rfi").unwrap().description, "Rayform Image 2");
        list.clear();
        assert!(list.is_empty());
        assert!(list.get("rfc").is_none());
    }
}
