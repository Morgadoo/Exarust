//! The scene extension base (`TScenePlugin`): a scene pointer and two
//! hooks, one for the scene's attachment and one for a batch of nodes that
//! arrived. The original's own two functions are empty 4-byte stubs that
//! subclasses reuse; here they are trait defaults with a registry that calls
//! them in attach order.
//!
//! Exarust's world plugins are Bevy `Plugin`s built at start-up. This list
//! is the original's runtime extension point, kept for a plugin that has to
//! see every node batch (the skinning manager and the terrain grass are the
//! original's users).

/// A per-scene extension.
pub trait ScenePlugin {
    /// The scene this extension attached to, when it did.
    fn attach(&mut self, _scene: u32) {}

    /// A batch of nodes was added to the scene.
    fn nodes_added(&mut self, _nodes: &[usize]) {}
}

/// The scene's extension list.
#[derive(Default)]
pub struct ScenePlugins {
    plugins: Vec<Box<dyn ScenePlugin>>,
}

impl ScenePlugins {
    pub fn new() -> Self {
        Self::default()
    }

    /// Attaches `plugin` last and tells it the scene.
    pub fn attach(&mut self, scene: u32, mut plugin: Box<dyn ScenePlugin>) {
        plugin.attach(scene);
        self.plugins.push(plugin);
    }

    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }

    /// Tells every extension about a node batch, in attach order.
    pub fn nodes_added(&mut self, nodes: &[usize]) {
        for plugin in &mut self.plugins {
            plugin.nodes_added(nodes);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct Recorder {
        log: Arc<Mutex<Vec<String>>>,
    }

    impl ScenePlugin for Recorder {
        fn attach(&mut self, scene: u32) {
            self.log.lock().unwrap().push(format!("attach {scene}"));
        }
        fn nodes_added(&mut self, nodes: &[usize]) {
            self.log.lock().unwrap().push(format!("nodes {nodes:?}"));
        }
    }

    #[test]
    fn plugins_attach_and_see_node_batches_in_order() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut plugins = ScenePlugins::new();
        plugins.attach(7, Box::new(Recorder { log: log.clone() }));
        plugins.attach(7, Box::new(Recorder { log: log.clone() }));
        assert_eq!(plugins.len(), 2);
        plugins.nodes_added(&[1, 2]);
        assert_eq!(
            *log.lock().unwrap(),
            ["attach 7", "attach 7", "nodes [1, 2]", "nodes [1, 2]"]
        );
    }
}
