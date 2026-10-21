//! Terrain extensions (`TTerrainPlugin`) and the static terrain's scene
//! set-up (`RF_Terrain_Static`).
//!
//! A terrain extension attaches to a manager, which hands it every tile;
//! its activate and deactivate hooks are empty in the base and overridden by
//! the grass. Exarust currently wires the grass directly in
//! [`super::TerrainBuild`] (`Plants::load`/`Plants::spawn`), which is the
//! same outcome as the original's new-terrain hook; this list is the
//! original's registry, kept so a second extension can attach in order.
//!
//! The static set-up creates the manager once (a global), attaches it to the
//! scene and marks the scene as having terrain only when it creates it, then
//! builds one shadow receiver per tile and block (none in room mode, which
//! sizes a pool instead) and activates the manager. The Bevy terrain builder
//! performs the corresponding work; the order and counts are modelled here.

use super::manager::{BLOCK_SIDE, ManagerConfig, ManagerSettings};

/// A terrain extension: the original's empty hooks.
pub trait TerrainExtension {
    /// Called when the manager activates.
    fn activate(&mut self) {}

    /// Called when the manager deactivates.
    fn deactivate(&mut self) {}
}

/// The manager's extension list, in attach order.
#[derive(Default)]
pub struct TerrainExtensions {
    plugins: Vec<Box<dyn TerrainExtension>>,
}

impl TerrainExtensions {
    pub fn new() -> Self {
        Self::default()
    }

    /// Attaches `plugin`: the manager's count goes up by one and the plugin
    /// takes the new last slot.
    pub fn attach(&mut self, plugin: Box<dyn TerrainExtension>) {
        self.plugins.push(plugin);
    }

    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }

    /// Activates every extension in attach order.
    pub fn activate(&mut self) {
        for plugin in &mut self.plugins {
            plugin.activate();
        }
    }

    /// Deactivates every extension in reverse attach order (Exarust's choice:
    /// the base hooks are empty, so the order is not read from the original).
    pub fn deactivate(&mut self) {
        for plugin in self.plugins.iter_mut().rev() {
            plugin.deactivate();
        }
    }
}

/// One block's shadow receiver: the tile and the block's column and row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShadowReceiver {
    pub tile: usize,
    pub block: (usize, usize),
}

/// The scene the static terrain is brought up in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SceneInfo {
    /// Scene identity.
    pub id: u32,
    /// The scene is in room mode (flag bit 0x2000000 of its second word).
    pub room_mode: bool,
    /// Tiles the manager has loaded.
    pub tiles: usize,
    /// The room-mode pool's two factors (the manager's words at +0x170 and
    /// +0x174).
    pub room_pool: (usize, usize),
}

/// What bringing the static terrain up produced.
#[derive(Clone, Debug, PartialEq)]
pub struct TerrainSetup {
    /// Whether the singleton manager was created (false when reused).
    pub created: bool,
    /// The scene's "has terrain" word was set: only when the manager was
    /// created.
    pub scene_marked: bool,
    /// The shared data stream was created (the first time only).
    pub stream_created: bool,
    /// The per-block shadow receivers, tile by tile (empty in room mode).
    pub receivers: Vec<ShadowReceiver>,
    /// The room-mode pool length, in room mode.
    pub room_pool: Option<usize>,
    /// The manager was activated.
    pub activated: bool,
}

/// The singleton manager a scene reuses: the original keeps one global
/// terrain manager pointer and one global data stream, each created only
/// when null.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StaticTerrain {
    config: Option<ManagerConfig>,
    stream: bool,
    /// The scene the manager attached to, once any.
    pub scene: Option<u32>,
}

impl StaticTerrain {
    /// Creates or reuses the singleton and performs the set-up. A new
    /// manager is built with the static configuration record, attached to
    /// the scene and marks it; a reused one does neither. Then, outside room
    /// mode, one receiver per tile and block (column in the low bits of the
    /// block number, row above); in room mode the pool is sized instead.
    pub fn bring_up(&mut self, scene: SceneInfo) -> TerrainSetup {
        let created = self.config.is_none();
        if created {
            self.config = Some(ManagerConfig::new(&ManagerSettings::STATIC));
            self.scene = Some(scene.id);
        }
        let stream_created = !self.stream;
        self.stream = true;
        let mut receivers = Vec::new();
        let mut room_pool = None;
        if scene.room_mode {
            room_pool = Some(scene.room_pool.0 * scene.room_pool.1);
        } else {
            receivers.reserve(scene.tiles * BLOCK_SIDE * BLOCK_SIDE);
            for tile in 0..scene.tiles {
                for block in 0..BLOCK_SIDE * BLOCK_SIDE {
                    receivers.push(ShadowReceiver {
                        tile,
                        block: (block & 7, block >> 3),
                    });
                }
            }
        }
        TerrainSetup {
            created,
            scene_marked: created,
            stream_created,
            receivers,
            room_pool,
            activated: true,
        }
    }

    /// The manager's configuration once created.
    pub fn config(&self) -> Option<&ManagerConfig> {
        self.config.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct Recorder {
        log: Arc<Mutex<Vec<&'static str>>>,
    }

    impl TerrainExtension for Recorder {
        fn activate(&mut self) {
            self.log.lock().unwrap().push("on");
        }
        fn deactivate(&mut self) {
            self.log.lock().unwrap().push("off");
        }
    }

    fn scene(id: u32, tiles: usize, room_mode: bool) -> SceneInfo {
        SceneInfo {
            id,
            room_mode,
            tiles,
            room_pool: (3, 5),
        }
    }

    #[test]
    fn extensions_attach_and_activate_in_order() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut list = TerrainExtensions::new();
        list.attach(Box::new(Recorder { log: log.clone() }));
        list.attach(Box::new(Recorder { log: log.clone() }));
        assert_eq!(list.len(), 2);
        list.activate();
        list.deactivate();
        assert_eq!(*log.lock().unwrap(), ["on", "on", "off", "off"]);
    }

    #[test]
    fn the_static_terrain_is_created_once_and_reused() {
        let mut terrain = StaticTerrain::default();
        assert!(terrain.config().is_none());
        let first = terrain.bring_up(scene(7, 2, false));
        assert!(first.created && first.scene_marked && first.stream_created);
        assert!(first.activated);
        assert_eq!(first.receivers.len(), 2 * 64);
        assert_eq!(first.receivers[0].block, (0, 0));
        assert_eq!(first.receivers[1].block, (1, 0));
        assert_eq!(first.receivers[8].block, (0, 1));
        assert_eq!(first.receivers[63].block, (7, 7));
        assert_eq!(first.receivers[64].tile, 1);
        assert_eq!(first.room_pool, None);
        assert_eq!(terrain.scene, Some(7));
        // The static record gives the manager its palette scale of 4.
        assert_eq!(terrain.config().unwrap().palette_scale, 4.0);
        // A second scene reuses the manager: no mark, no new stream, and the
        // manager keeps the first scene.
        let second = terrain.bring_up(scene(8, 1, false));
        assert!(!second.created && !second.scene_marked && !second.stream_created);
        assert_eq!(second.receivers.len(), 64);
        assert_eq!(terrain.scene, Some(7));
    }

    #[test]
    fn room_mode_sizes_a_pool_and_builds_no_receivers() {
        let mut terrain = StaticTerrain::default();
        let setup = terrain.bring_up(scene(1, 4, true));
        assert!(setup.receivers.is_empty());
        assert_eq!(setup.room_pool, Some(15));
        assert!(setup.created && setup.scene_marked && setup.activated);
    }
}
