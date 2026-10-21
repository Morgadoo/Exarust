//! Scene content: clearing a scene, detaching, releasing and attaching
//! node subtrees, applying a level's environment record, and the scene's
//! file-format list.
//!
//! The original's helper unit (before `RF_Dynamics`):
//!
//! - **clear** (`0x10007d4a0`): flushes the pending lists, stops and waits
//!   for the collision kd-tree builder, drops characters that are not in
//!   the scene, moves the remaining nodes to a holder scene (the iterative
//!   walk of `0x10007d350` resets their owner pointers and a flag), clears
//!   the sector list, deletes the root nodes and frees the scene's format
//!   list. Exarust despawns the level root's subtree, empties the removal
//!   ring and the format list, and lets the collider builder skip entities
//!   whose mesh has gone (it already checks).
//! - **detach** (`0x10007d000`): unregisters a subtree's lights and voxel
//!   lights from the scene lists, drops a geometry node's parts and a
//!   light's alias list, recurses. Here it marks the subtree detached and
//!   takes its lights out of the scene's registration.
//! - **release** (`0x10007d170`, `0x10007d1e0`, `0x10007d450`): frees a
//!   node and its children; an already-fresh node recurses without a
//!   transform refresh.
//! - **attach** (`0x10007d270`): appends an array of nodes as children,
//!   setting their parent and sector to the target.
//! - **environment** (`0x10007d840`): hands the level record's fog,
//!   ambient, floor and ceiling to the scene environment, the top and
//!   bottom to the GI request, and its reverb index to the audio
//!   environment.
//! - **format list** (`0x10007d8f0`, `0x10007d910`): registers a file
//!   format (key, description, loader) and frees the list.
//!
//! Rules: `docs/formats/RFC.md` (environment), `docs/RENDERING.md` (GI
//! bounds).

use bevy::prelude::*;
use exa_formats::environment::Environment;

use crate::node_life::RemovalQueue;

/// One entry of the scene's file-format list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneFormat {
    /// The format's numeric key.
    pub key: u32,
    /// The description shown to the user (`"Flux Content"`).
    pub description: String,
    /// The loader the key names.
    pub loader: u32,
}

/// The scene's file-format list.
#[derive(Resource, Debug, Default, Clone, PartialEq, Eq)]
pub struct SceneFormatList {
    formats: Vec<SceneFormat>,
}

impl SceneFormatList {
    /// Registers a format under its key; a later registration of the same
    /// key replaces the old one (the original's registry call overwrites).
    pub fn register(&mut self, format: SceneFormat) {
        match self.formats.iter_mut().find(|f| f.key == format.key) {
            Some(slot) => *slot = format,
            None => self.formats.push(format),
        }
    }

    /// The format under a key.
    pub fn get(&self, key: u32) -> Option<&SceneFormat> {
        self.formats.iter().find(|f| f.key == key)
    }

    /// Frees the list.
    pub fn free(&mut self) {
        self.formats.clear();
    }

    pub fn len(&self) -> usize {
        self.formats.len()
    }

    pub fn is_empty(&self) -> bool {
        self.formats.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &SceneFormat> + '_ {
        self.formats.iter()
    }
}

/// The key of the level-scene file format the original registers at start
/// up ("Flux Content"). Exarust's key stands in for the original's numeric
/// key, which no file stores.
pub const FLUX_CONTENT_KEY: u32 = u32::from_le_bytes(*b"FCNT");

/// The standard list: the level scene's own format.
pub fn standard_formats() -> SceneFormatList {
    let mut list = SceneFormatList::default();
    list.register(SceneFormat {
        key: FLUX_CONTENT_KEY,
        description: "Flux Content".into(),
        loader: 0,
    });
    list
}

/// What a level's environment record hands to the scene, the GI request and
/// the audio environment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppliedEnvironment {
    /// The GI volume's floor and ceiling, metres.
    pub bottom: f32,
    pub top: f32,
    /// The main ambient (sRGB) and the secondary fill colour.
    pub ambient: [f32; 3],
    pub fill: [f32; 4],
    /// Added reverb decay, seconds.
    pub reverb_decay: f32,
}

/// Applies a record as the clear/finish helper does.
pub fn apply_environment(env: &Environment) -> AppliedEnvironment {
    AppliedEnvironment {
        bottom: env.floor,
        top: env.ceiling,
        ambient: env.ambient,
        fill: env.fill,
        reverb_decay: env.reverb_decay,
    }
}

/// Requests a node subtree's attachment under another node.
#[derive(Message, Debug, Clone, Copy)]
pub struct AttachNode {
    pub parent: Entity,
    pub node: Entity,
}

/// Requests that a subtree be detached from the scene.
#[derive(Message, Debug, Clone, Copy)]
pub struct DetachNode(pub Entity);

/// Requests that a subtree be freed.
#[derive(Message, Debug, Clone, Copy)]
pub struct ReleaseNode(pub Entity);

/// Requests that a scene root's contents be cleared.
#[derive(Message, Debug, Clone, Copy)]
pub struct ClearScene(pub Entity);

/// Marks a node taken out of the scene's lists.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Detached;

pub struct SceneContentPlugin;

impl Plugin for SceneContentPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SceneFormatList>()
            .insert_resource(standard_formats())
            .add_message::<AttachNode>()
            .add_message::<DetachNode>()
            .add_message::<ReleaseNode>()
            .add_message::<ClearScene>()
            .add_systems(
                PostUpdate,
                (attach_nodes, detach_nodes, release_nodes, clear_scenes).chain(),
            );
    }
}

/// Attaches queued nodes: the child list (Bevy's `ChildOf`) and the
/// parent pointer, as the original's attach array sets both.
fn attach_nodes(
    mut commands: Commands,
    mut requests: MessageReader<AttachNode>,
    mut links: Query<&mut crate::node_life::NodeInstances>,
) {
    for request in requests.read() {
        commands
            .entity(request.node)
            .insert(ChildOf(request.parent));
        commands.entity(request.node).remove::<Detached>();
        if let Ok(mut link) = links.get_mut(request.parent) {
            link.add(request.node);
        }
    }
}

/// Detaches queued subtrees: every node is taken off its parent, its master
/// list and the lights' registration. The original also unregisters voxel
/// lights; both are Bevy components here, so the marker is what keeps them
/// out of a future scene registration.
fn detach_nodes(
    mut commands: Commands,
    mut requests: MessageReader<DetachNode>,
    children: Query<&Children>,
    mut links: Query<&mut crate::node_life::NodeInstances>,
) {
    for request in requests.read() {
        let mut stack = vec![request.0];
        while let Some(node) = stack.pop() {
            commands.entity(node).insert(Detached);
            commands.entity(node).remove::<ChildOf>();
            if let Ok(mut link) = links.get_mut(node) {
                link.instances.clear();
                link.master = None;
            }
            if let Ok(list) = children.get(node) {
                stack.extend(list.iter());
            }
        }
    }
}

/// Frees queued subtrees (the original's recursive release).
fn release_nodes(
    mut commands: Commands,
    mut requests: MessageReader<ReleaseNode>,
    children: Query<&Children>,
) {
    for request in requests.read() {
        let mut stack = vec![request.0];
        while let Some(node) = stack.pop() {
            if let Ok(list) = children.get(node) {
                stack.extend(list.iter());
            }
            commands.entity(node).despawn();
        }
    }
}

/// Clears a scene root: its subtree goes, and with it the pending removal
/// ring and the format list (the original's clear flushes the pending lists
/// and frees the format list).
fn clear_scenes(
    mut commands: Commands,
    mut requests: MessageReader<ClearScene>,
    children: Query<&Children>,
    mut queue: ResMut<RemovalQueue>,
    mut formats: ResMut<SceneFormatList>,
    mut static_build: ResMut<exa_physics::StaticBuild>,
) {
    for request in requests.read() {
        let mut stack = vec![request.0];
        while let Some(node) = stack.pop() {
            if let Ok(list) = children.get(node) {
                stack.extend(list.iter());
            }
            commands.entity(node).despawn();
        }
        queue.clear();
        static_build.clear();
        formats.free();
        commands.insert_resource(standard_formats());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_format_list_registers_and_frees() {
        let mut list = standard_formats();
        assert_eq!(list.len(), 1);
        assert_eq!(
            list.get(FLUX_CONTENT_KEY).unwrap().description,
            "Flux Content"
        );
        list.register(SceneFormat {
            key: 7,
            description: "Test".into(),
            loader: 3,
        });
        assert_eq!(list.len(), 2);
        assert_eq!(list.get(7).unwrap().loader, 3);
        // A second registration of the key replaces it.
        list.register(SceneFormat {
            key: 7,
            description: "Test 2".into(),
            loader: 4,
        });
        assert_eq!(list.len(), 2);
        assert_eq!(list.get(7).unwrap().description, "Test 2");
        list.free();
        assert!(list.is_empty());
    }

    #[test]
    fn the_environment_record_carries_its_extent_ambient_and_reverb() {
        let env = Environment {
            floor: -1.25,
            ceiling: 7.5,
            ambient: [0.6275, 0.5, 0.25],
            fill: [0.04, 0.05, 0.06, 1.0],
            reverb_decay: 0.25,
            ..Environment::default()
        };
        let applied = apply_environment(&env);
        assert_eq!(applied.bottom, -1.25);
        assert_eq!(applied.top, 7.5);
        assert_eq!(applied.ambient, [0.6275, 0.5, 0.25]);
        assert_eq!(applied.fill, [0.04, 0.05, 0.06, 1.0]);
        assert_eq!(applied.reverb_decay, 0.25);
        // The default record is the level-without-one fallback.
        let default = apply_environment(&Environment::default());
        assert_eq!((default.bottom, default.top), (0.0, 3.4));
    }
}
