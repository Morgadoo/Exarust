//! The surface shader (`exa.wesl`) composed with Bevy's PBR modules and
//! validated by naga, as `gi.wesl`'s test does for the self-contained
//! propagation shader. Bevy's own shader cache does the composition (the
//! same WESL compile, features and constants the renderer uses); naga
//! parses and validates the result instead of a GPU device. Bevy's shader
//! sources are read from the crates cargo resolved.

use std::path::{Path, PathBuf};

use bevy::asset::AssetId;
use bevy::asset::uuid::Uuid;
use bevy::shader::{
    Shader, ShaderCache, ShaderCacheError, ShaderCacheSource, ShaderDefVal, ValidateShader,
};

/// Parses and validates a composed module, as a device would.
fn validate(
    _device: &(),
    source: ShaderCacheSource,
    _checks: &ValidateShader,
) -> Result<naga::Module, ShaderCacheError> {
    let ShaderCacheSource::Wgsl(wgsl) = source else {
        return Err(ShaderCacheError::ProcessShaderError("not WGSL".to_string()));
    };
    let module = naga::front::wgsl::parse_str(&wgsl)
        .map_err(|error| ShaderCacheError::ProcessShaderError(error.emit_to_string(&wgsl)))?;
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .map_err(|error| ShaderCacheError::ProcessShaderError(error.emit_to_string(&wgsl)))?;
    Ok(module)
}

/// The triple cargo builds for by default (`host:` in `cargo -vV`).
fn host() -> String {
    let output = std::process::Command::new(env!("CARGO"))
        .arg("-vV")
        .output()
        .expect("cargo -vV runs");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .expect("cargo names its host")
        .to_string()
}

/// The source directory of every Bevy crate in the build, by crate name:
/// cargo's metadata for this platform only, whose packages are all on disk.
fn bevy_sources() -> Vec<(String, PathBuf)> {
    let output = std::process::Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1", "--offline"])
        .args(["--filter-platform", &host()])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("cargo metadata runs");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("cargo metadata is JSON");
    metadata["packages"]
        .as_array()
        .expect("a package list")
        .iter()
        .filter_map(|package| {
            let name = package["name"].as_str()?;
            if !name.starts_with("bevy_") {
                return None;
            }
            let manifest = Path::new(package["manifest_path"].as_str()?);
            Some((name.to_string(), manifest.parent()?.join("src")))
        })
        .collect()
}

/// Every `.wesl` file under `dir`, with its path relative to `dir`.
fn wesl_files(dir: &Path, relative: &Path, out: &mut Vec<(PathBuf, PathBuf)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = relative.join(entry.file_name());
        if path.is_dir() {
            wesl_files(&path, &name, out);
        } else if path.extension().is_some_and(|e| e == "wesl") {
            out.push((path, name));
        }
    }
}

/// A shader cache holding every Bevy shader and exa_render's own, each at
/// the embedded path its crate's plugin loads it from; returns the id of
/// `exa.wesl`.
fn cache_with_every_shader() -> (ShaderCache<naga::Module, ()>, AssetId<Shader>) {
    let mut cache = ShaderCache::new((), validate);
    let own = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut crates = bevy_sources();
    crates.push(("exa_render".to_string(), own));
    let mut next = 1u128;
    let mut surface = None;
    for (name, src) in crates {
        let mut files = Vec::new();
        wesl_files(&src, Path::new(""), &mut files);
        for (path, relative) in files {
            let source = std::fs::read_to_string(&path).expect("a shader reads");
            let embedded = format!(
                "embedded://{name}/{}",
                relative.to_string_lossy().replace('\\', "/")
            );
            let id = AssetId::Uuid {
                uuid: Uuid::from_u128(next),
            };
            next += 1;
            if embedded == "embedded://exa_render/exa.wesl" {
                surface = Some(id);
            }
            cache.set_shader(id, Shader::from_wesl(source, embedded));
        }
    }
    (
        cache,
        surface.expect("exa.wesl is among exa_render's shaders"),
    )
}

/// The definitions every mesh pipeline gets (light and cascade limits,
/// storage buffers, the material's bind group, the instance index, the
/// perspective view and the shadow filter) plus `extra`.
fn defs(extra: &[&'static str]) -> Vec<ShaderDefVal> {
    let mut defs = vec![
        ShaderDefVal::UInt("MAX_DIRECTIONAL_LIGHTS".into(), 10),
        ShaderDefVal::UInt("MAX_CASCADES_PER_LIGHT".into(), 4),
        ShaderDefVal::UInt("MAX_RECT_LIGHTS".into(), 8),
        ShaderDefVal::UInt("AVAILABLE_STORAGE_BUFFER_BINDINGS".into(), 8),
        ShaderDefVal::Bool("AVAILABLE_STORAGE_BUFFER_BINDINGS__GE_3".into(), true),
        ShaderDefVal::UInt("MATERIAL_BIND_GROUP".into(), 3),
        "MESH_PIPELINE".into(),
        "VERTEX_OUTPUT_INSTANCE_INDEX".into(),
        "VIEW_PROJECTION_PERSPECTIVE".into(),
        "SHADOW_FILTER_METHOD_GAUSSIAN".into(),
        "VERTEX_POSITIONS".into(),
        "VERTEX_NORMALS".into(),
    ];
    defs.extend(extra.iter().map(|&def| ShaderDefVal::from(def)));
    defs
}

#[test]
fn the_surface_shader_composes_and_validates() {
    let (mut cache, surface) = cache_with_every_shader();
    // A textured, tangent-mapped, vertex-coloured mesh, and a bare one.
    let variants: [&[&'static str]; 2] = [
        &[
            "VERTEX_UVS",
            "VERTEX_UVS_A",
            "VERTEX_TANGENTS",
            "VERTEX_COLORS",
        ],
        &[],
    ];
    for (pipeline, extra) in variants.iter().enumerate() {
        let module = cache
            .get(pipeline, surface, &defs(extra))
            .unwrap_or_else(|error| panic!("exa.wesl with {extra:?}: {error}"));
        assert!(
            module
                .entry_points
                .iter()
                .any(|entry| entry.stage == naga::ShaderStage::Fragment),
            "exa.wesl has a fragment entry point"
        );
    }
}
