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

    /// Deactivates every extension in attach order, as the static terrain's
    /// deactivation walks its list (first to last).
    pub fn deactivate(&mut self) {
        for plugin in self.plugins.iter_mut() {
            plugin.deactivate();
        }
    }
}

/// The hooks the static terrain registers when it activates, and what its
/// deactivation removes: the scene's frame hook, the manager's scene hook,
/// and per renderer the lit terrain draw and the wireframe debug hook.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TerrainHooks {
    /// The scene's per-frame hook.
    pub frame: bool,
    /// The manager's scene hook.
    pub scene: bool,
    /// Lit terrain draw hooks, one per renderer.
    pub draws: usize,
    /// Wireframe debug hooks, one per renderer.
    pub wireframes: usize,
}

impl TerrainHooks {
    /// The complete set for `renderers` renderers.
    pub fn for_renderers(renderers: usize) -> Self {
        Self {
            frame: true,
            scene: true,
            draws: renderers,
            wireframes: renderers,
        }
    }
}

/// What activating the static terrain did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ManagerActivation {
    /// The terrain was inactive (its inactive bit set) and is now active:
    /// a second activation does nothing.
    pub activated: bool,
    /// The hooks inserted: the frame hook, the scene hook, and per renderer
    /// the lit draw and the wireframe debug hook.
    pub hooks: TerrainHooks,
    /// The background worker's thread was resumed (a second thread helper,
    /// run only when a manager flag is set, is not modelled).
    pub worker_resumed: bool,
    /// The composite-cache release was registered with the 100 ms timer:
    /// the first activation in the process only (a global flag).
    pub release_registered: bool,
    /// Extensions activated, in attach order.
    pub extensions: usize,
}

/// What deactivating the static terrain did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ManagerDeactivation {
    /// The terrain was active and is now inactive: a second deactivation
    /// does nothing.
    pub deactivated: bool,
    /// The hooks removed, as the activation inserted them.
    pub hooks: TerrainHooks,
    /// The background worker's thread was suspended.
    pub worker_suspended: bool,
    /// Extensions deactivated, in attach order.
    pub extensions: usize,
    /// The composite sets' textures were deleted, the pools and the
    /// selection list emptied, and every tile's block nodes freed.
    pub composites_freed: bool,
}

/// The composite caches the release slot frees (`RF_Terrain_Static`'s
/// release): the per-view program dictionary's programs (an entry whose
/// program was never built is skipped), the shader object the terrain
/// programs share, and the dictionary's array. The original keeps them for
/// the whole run and frees them from a 100 ms timer after the terrain
/// deactivates; the release then calls one more teardown outside this
/// component, not modelled here.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TerrainCaches {
    /// Program-dictionary entries: the state words the terrain programs were
    /// built for.
    pub programs: Vec<u32>,
    /// The shared shader object exists.
    pub shader: bool,
}

impl TerrainCaches {
    /// Frees everything and reports how many program entries went with it.
    /// The original's free runs once and leaves the cache empty.
    pub fn release(&mut self) -> usize {
        let freed = self.programs.len();
        self.programs.clear();
        self.shader = false;
        freed
    }
}

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
    /// The room-mode pool's two factors (two words of the manager).
    pub room_pool: (usize, usize),
    /// Renderers the terrain registers its draw and debug hooks with.
    pub renderers: usize,
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
    /// The hooks its activation registered.
    pub hooks: TerrainHooks,
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
    /// The terrain is active: its hooks are registered.
    active: bool,
    hooks: TerrainHooks,
    /// The composite caches, freed by the release slot.
    caches: TerrainCaches,
    /// The release slot has been registered (it is a global, once).
    release_registered: bool,
}

impl StaticTerrain {
    /// Creates or reuses the singleton and performs the set-up. A new
    /// manager is built with the static configuration record, attached to
    /// the scene and marks it; a reused one does neither. Then, outside room
    /// mode, one receiver per tile and block (column in the low bits of the
    /// block number, row above); in room mode the pool is sized instead.
    /// Last, the terrain is activated ([`Self::activate`]).
    pub fn bring_up(
        &mut self,
        scene: SceneInfo,
        extensions: &mut TerrainExtensions,
    ) -> TerrainSetup {
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
        let activation = self.activate(scene.renderers, extensions);
        TerrainSetup {
            created,
            scene_marked: created,
            stream_created,
            receivers,
            room_pool,
            activated: activation.activated,
            hooks: self.hooks,
        }
    }

    /// The manager's configuration once created.
    pub fn config(&self) -> Option<&ManagerConfig> {
        self.config.as_ref()
    }

    /// The composite caches, for building program-dictionary entries and
    /// the shared shader.
    pub fn caches_mut(&mut self) -> &mut TerrainCaches {
        &mut self.caches
    }

    /// The terrain's activation: when inactive, it becomes active, inserts
    /// the frame hook (priority 4), the scene hook (5) and per renderer the
    /// lit draw and the wireframe hook (10), registers the composite-cache
    /// release with its 100 ms timer (once in the process), resumes the
    /// worker and activates every extension in attach order. When already
    /// active it does nothing.
    pub fn activate(
        &mut self,
        renderers: usize,
        extensions: &mut TerrainExtensions,
    ) -> ManagerActivation {
        if self.active {
            return ManagerActivation::default();
        }
        self.active = true;
        self.hooks = TerrainHooks::for_renderers(renderers);
        let release_registered = !self.release_registered;
        self.release_registered = true;
        extensions.activate();
        ManagerActivation {
            activated: true,
            hooks: self.hooks,
            worker_resumed: true,
            release_registered,
            extensions: extensions.len(),
        }
    }

    /// The terrain's deactivation: when active, it becomes inactive, removes
    /// the frame, scene, draw and wireframe hooks, suspends the worker,
    /// deactivates every extension in attach order, then deletes the
    /// composite sets' textures, empties the pools and the selection list
    /// and frees every tile's block nodes before unloading the geometry.
    /// When already inactive it does nothing.
    pub fn deactivate(&mut self, extensions: &mut TerrainExtensions) -> ManagerDeactivation {
        if !self.active {
            return ManagerDeactivation::default();
        }
        self.active = false;
        let hooks = std::mem::take(&mut self.hooks);
        extensions.deactivate();
        ManagerDeactivation {
            deactivated: true,
            hooks,
            worker_suspended: true,
            extensions: extensions.len(),
            composites_freed: true,
        }
    }

    /// Whether the terrain is active (its hooks registered).
    pub fn is_active(&self) -> bool {
        self.active
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

    struct Named {
        name: &'static str,
        log: Arc<Mutex<Vec<&'static str>>>,
    }

    impl TerrainExtension for Named {
        fn deactivate(&mut self) {
            self.log.lock().unwrap().push(self.name);
        }
    }

    fn scene(id: u32, tiles: usize, room_mode: bool) -> SceneInfo {
        SceneInfo {
            id,
            room_mode,
            tiles,
            room_pool: (3, 5),
            renderers: 2,
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
        let first = terrain.bring_up(scene(7, 2, false), &mut TerrainExtensions::new());
        assert!(first.created && first.scene_marked && first.stream_created);
        assert!(first.activated);
        assert_eq!(first.receivers.len(), 2 * 64);
        assert_eq!(first.receivers[0].block, (0, 0));
        assert_eq!(first.receivers[1].block, (1, 0));
        assert_eq!(first.receivers[8].block, (0, 1));
        assert_eq!(first.receivers[63].block, (7, 7));
        assert_eq!(first.receivers[64].tile, 1);
        assert_eq!(first.room_pool, None);
        assert_eq!(first.hooks, TerrainHooks::for_renderers(2));
        assert_eq!(
            first.hooks,
            TerrainHooks {
                frame: true,
                scene: true,
                draws: 2,
                wireframes: 2,
            }
        );
        assert_eq!(terrain.scene, Some(7));
        // The static record gives the manager its palette scale of 4.
        assert_eq!(terrain.config().unwrap().palette_scale, 4.0);
        // A second scene reuses the manager: no mark, no new stream, and the
        // manager keeps the first scene.
        let second = terrain.bring_up(scene(8, 1, false), &mut TerrainExtensions::new());
        assert!(!second.created && !second.scene_marked && !second.stream_created);
        assert_eq!(second.receivers.len(), 64);
        assert_eq!(terrain.scene, Some(7));
    }

    #[test]
    fn room_mode_sizes_a_pool_and_builds_no_receivers() {
        let mut terrain = StaticTerrain::default();
        let setup = terrain.bring_up(scene(1, 4, true), &mut TerrainExtensions::new());
        assert!(setup.receivers.is_empty());
        assert_eq!(setup.room_pool, Some(15));
        assert!(setup.created && setup.scene_marked && setup.activated);
    }

    #[test]
    fn activation_inserts_the_hooks_and_registers_the_release_once() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut list = TerrainExtensions::new();
        list.attach(Box::new(Recorder { log: log.clone() }));
        let mut terrain = StaticTerrain::default();
        // Bringing the terrain up activates it: the hooks go in, the worker
        // resumes, the release is registered and the extension activates.
        let setup = terrain.bring_up(scene(1, 3, false), &mut list);
        assert!(setup.activated && terrain.is_active());
        assert_eq!(setup.hooks, TerrainHooks::for_renderers(2));
        assert_eq!(*log.lock().unwrap(), ["on"]);
        // A second activation while active does nothing.
        assert_eq!(terrain.activate(2, &mut list), ManagerActivation::default());
        assert_eq!(*log.lock().unwrap(), ["on"]);
        // The caches the release slot frees: two program entries and the
        // shared shader.
        terrain.caches_mut().programs.extend([0xff4, 0x111]);
        terrain.caches_mut().shader = true;
        let down = terrain.deactivate(&mut list);
        assert!(down.deactivated && down.worker_suspended && down.composites_freed);
        assert_eq!(down.extensions, 1);
        assert_eq!(
            down.hooks,
            TerrainHooks {
                frame: true,
                scene: true,
                draws: 2,
                wireframes: 2,
            }
        );
        assert!(!terrain.is_active());
        assert_eq!(*log.lock().unwrap(), ["on", "off"]);
        // A second deactivation does nothing.
        assert_eq!(
            terrain.deactivate(&mut list),
            ManagerDeactivation::default()
        );
        // Activating again inserts the hooks again, but the release was
        // registered by the first activation only.
        let again = terrain.activate(1, &mut list);
        assert!(again.activated && again.worker_resumed);
        assert!(!again.release_registered);
        assert_eq!(again.hooks, TerrainHooks::for_renderers(1));
        assert_eq!(*log.lock().unwrap(), ["on", "off", "on"]);
        let freed = terrain.caches_mut().release();
        assert_eq!(freed, 2);
        assert!(terrain.caches_mut().programs.is_empty());
        assert!(!terrain.caches_mut().shader);
    }

    #[test]
    fn a_list_deactivates_first_to_last() {
        let order = Arc::new(Mutex::new(Vec::new()));
        let mut pair = TerrainExtensions::new();
        pair.attach(Box::new(Named {
            name: "grass",
            log: order.clone(),
        }));
        pair.attach(Box::new(Named {
            name: "second",
            log: order.clone(),
        }));
        let mut other = StaticTerrain::default();
        let first = other.activate(1, &mut pair);
        assert!(first.release_registered);
        other.deactivate(&mut pair);
        assert_eq!(*order.lock().unwrap(), ["grass", "second"]);
    }
}
