//! The `cshd` debug command's shader reload.
//!
//! The original's shader-program unit registers `cshd` with the debug
//! registry; running it re-reads the GLSL sources and rebuilds the GPU
//! programs. Exarust's shaders are embedded in the binary (WESL compiled by
//! Bevy), so there is no file to re-read, but asking the asset server to
//! reload every embedded shader rebuilds the pipelines from the current
//! sources, which is the same observable effect when a shader source has
//! changed in a development build.
//!
//! The paths are the crate's embedded shader assets.

use bevy::asset::AssetServer;
use bevy::prelude::*;

/// Every shader asset the render crate embeds.
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
    "shading.wesl",
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
    fn every_wesl_file_is_listed() {
        for name in SHADERS {
            assert!(name.ends_with(".wesl"));
            assert_eq!(path(name), format!("embedded://exa_render/{name}"));
        }
        assert_eq!(SHADERS.len(), 13);
        assert!(SHADERS.contains(&"shading.wesl"));
    }
}
