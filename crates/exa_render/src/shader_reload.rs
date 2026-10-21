//! The `cshd` debug command's shader reload.
//!
//! The original's shader-program unit registers `cshd` with the debug
//! registry; running it re-reads the GLSL sources from disk and rebuilds the
//! GPU programs. Exarust's shaders are embedded in the binary (WESL compiled
//! by Bevy), so there is no file to re-read: asking the asset server to
//! reload every embedded shader asset re-reads the embedded bytes and makes
//! Bevy rebuild the pipelines. That only changes anything if the embedded
//! sources do (a build with Bevy's embedded-asset watcher); in the shipped
//! build the command is a pipeline rebuild.
//!
//! `shading.wesl` is a shader library imported by the others, not a loaded
//! asset, so it is not listed.

use bevy::asset::AssetServer;
use bevy::prelude::*;

/// Every shader asset the render crate embeds and loads.
pub const SHADERS: &[&str] = &[
    "arc.wesl",
    "cube_shadow.wesl",
    "exa.wesl",
    "exa_prepass.wesl",
    "flame.wesl",
    "gi.wesl",
    "glow.wesl",
    "post.wesl",
    "resolve.wesl",
    "shell.wesl",
    "spray.wesl",
    "stain.wesl",
];

/// The embedded asset path of a render shader.
pub fn path(name: &str) -> String {
    format!("embedded://exa_render/{name}")
}

/// Asks the asset server to reload every render shader, in the crate's
/// order. Returns the number of assets asked for.
pub fn reload(server: &AssetServer) -> usize {
    for name in SHADERS {
        server.reload(path(name));
    }
    SHADERS.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_embedded_shader_asset_is_listed() {
        for name in SHADERS {
            assert!(name.ends_with(".wesl"));
            assert_eq!(path(name), format!("embedded://exa_render/{name}"));
        }
        assert_eq!(SHADERS.len(), 12);
        assert!(!SHADERS.contains(&"shading.wesl"));
    }
}
