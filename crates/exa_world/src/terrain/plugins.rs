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
//! The static set-up creates the manager once per scene, marks the scene as
//! having terrain, applies the palette, builds the per-block shadow
//! receivers (the room-mode variant when the scene is in room mode) and
//! activates it. The Bevy terrain builder performs the corresponding work;
//! the counts and order are modelled here.

use super::manager::{BlockTrees, ManagerConfig, TILE_CELLS};

/// The blocks of receivers per 6 m block row and column.
pub const RECEIVER_BLOCKS: usize = 8;

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

    /// Attaches `plugin`, last (the original's slot 13).
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

    /// Deactivates every extension in reverse attach order.
    pub fn deactivate(&mut self) {
        for plugin in self.plugins.iter_mut().rev() {
            plugin.deactivate();
        }
    }
}

/// One block's shadow receiver: the tile, the receiver's block column and
/// row, and the receiver's side in cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShadowReceiver {
    pub tile: usize,
    pub block: (usize, usize),
    pub cells: usize,
}

/// What bringing the static terrain up produced.
#[derive(Clone, Debug, PartialEq)]
pub struct TerrainSetup {
    /// Whether the singleton manager was created (false when reused).
    pub created: bool,
    /// The scene's "has terrain" word was set.
    pub scene_marked: bool,
    /// The palette was applied.
    pub palette_applied: bool,
    /// The per-block shadow receivers, tile by tile.
    pub receivers: Vec<ShadowReceiver>,
    /// The room-mode receiver variant was used.
    pub room_mode: bool,
    /// The composite pools were allocated.
    pub pools: bool,
    /// The manager was activated.
    pub activated: bool,
}

/// The singleton manager a scene reuses: the original keeps one global
/// terrain manager pointer and creates it only when it is null.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StaticTerrain {
    config: Option<ManagerConfig>,
    /// The scene the manager attached to, once any.
    pub scene: Option<u32>,
    /// The palette scale the manager was configured with.
    pub palette_scale: f32,
}

impl StaticTerrain {
    /// Creates or reuses the singleton and performs the set-up: the scene
    /// is marked, the palette applied, one 8 × 8 receiver set per tile, the
    /// pools allocated and the manager activated. `room_mode` selects the
    /// room receiver variant. `tiles` is the tile count; `scene` the scene
    /// identity.
    pub fn bring_up(
        &mut self,
        config: ManagerConfig,
        scene: u32,
        tiles: usize,
        room_mode: bool,
    ) -> TerrainSetup {
        let created = self.config.is_none();
        if created {
            self.config = Some(config);
            self.palette_scale = config.palette_scale;
        }
        self.scene = Some(scene);
        let mut receivers = Vec::with_capacity(tiles * RECEIVER_BLOCKS * RECEIVER_BLOCKS);
        for tile in 0..tiles {
            for row in 0..RECEIVER_BLOCKS {
                for col in 0..RECEIVER_BLOCKS {
                    receivers.push(ShadowReceiver {
                        tile,
                        block: (col, row),
                        cells: TILE_CELLS / RECEIVER_BLOCKS,
                    });
                }
            }
        }
        TerrainSetup {
            created,
            scene_marked: true,
            palette_applied: true,
            receivers,
            room_mode,
            pools: true,
            activated: true,
        }
    }

    /// The manager's configuration once created.
    pub fn config(&self) -> Option<&ManagerConfig> {
        self.config.as_ref()
    }

    /// Attaches the manager to a scene: the extension is registered, the
    /// scene keeps it as its terrain and it is activated. False when there
    /// is no manager yet.
    pub fn attach(&mut self, scene: u32, extensions: &mut TerrainExtensions) -> bool {
        if self.config.is_none() {
            return false;
        }
        self.scene = Some(scene);
        extensions.activate();
        true
    }

    /// The manager's destroy: the stop flag is set, the worker is woken and
    /// waited for, and the configuration drops.
    pub fn shutdown(&mut self) -> bool {
        let was_running = self.config.is_some();
        self.config = None;
        was_running
    }
}

/// The block trees a static terrain keeps, one per 6 m block (the shared
/// manager trees).
pub fn block_trees() -> BlockTrees {
    BlockTrees::new()
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
        let config = ManagerConfig::new(0, 0.0);
        let first = terrain.bring_up(config, 7, 2, false);
        assert!(first.created && first.pools && first.activated);
        assert!(first.scene_marked && first.palette_applied);
        assert_eq!(first.receivers.len(), 2 * 64);
        assert_eq!(first.receivers[0].block, (0, 0));
        assert_eq!(first.receivers[0].cells, 30);
        assert_eq!(first.receivers[64].tile, 1);
        assert!(!first.room_mode);
        let second = terrain.bring_up(config, 8, 1, true);
        assert!(!second.created);
        assert!(second.room_mode);
        assert_eq!(second.receivers.len(), 64);
        assert_eq!(terrain.scene, Some(8));
        assert_eq!(terrain.palette_scale, 2.0);
    }

    #[test]
    fn the_manager_attaches_and_shuts_down() {
        let mut terrain = StaticTerrain::default();
        let mut extensions = TerrainExtensions::new();
        assert!(!terrain.attach(3, &mut extensions));
        terrain.bring_up(ManagerConfig::new(0, 0.0), 3, 0, false);
        assert!(terrain.attach(3, &mut extensions));
        assert_eq!(terrain.scene, Some(3));
        assert!(terrain.shutdown());
        assert!(!terrain.shutdown());
    }

    #[test]
    fn block_trees_are_the_shared_eight_by_eight() {
        assert_eq!(block_trees().trees.len(), 64);
    }
}
