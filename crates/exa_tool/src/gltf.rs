//! Binary glTF (GLB) export of a chunk tree's scene nodes and static meshes.
//!
//! Positions are already metres, right-handed and Y-up with counter-clockwise
//! fronts, which is glTF's convention, so geometry is written unchanged.
//! Images are written with their rows in stored order (bottom row first) and
//! texture coordinates unchanged: the two flips cancel against glTF's
//! top-left texture origin.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use exa_formats::material::{Material, TextureSlot};
use exa_formats::rfc::{Mesh, NodeBody, Scene};
use exa_formats::rfi::{PixelFormat, Rfi};
use exa_rpk::{Archive, GameData};
use serde_json::{Value, json};

const ARRAY_BUFFER: u32 = 34962;
const ELEMENT_ARRAY_BUFFER: u32 = 34963;
const FLOAT: u32 = 5126;
const UNSIGNED_INT: u32 = 5125;

#[derive(Default)]
struct Builder {
    bin: Vec<u8>,
    views: Vec<Value>,
    accessors: Vec<Value>,
    images: Vec<Value>,
    textures: Vec<Value>,
    materials: Vec<Value>,
    meshes: Vec<Value>,
    nodes: Vec<Value>,
    material_index: HashMap<String, usize>,
    texture_index: HashMap<String, Option<(usize, bool)>>,
    warnings: Vec<String>,
}

impl Builder {
    fn view(&mut self, bytes: &[u8], target: Option<u32>) -> usize {
        while !self.bin.len().is_multiple_of(4) {
            self.bin.push(0);
        }
        let mut v = json!({
            "buffer": 0,
            "byteOffset": self.bin.len(),
            "byteLength": bytes.len(),
        });
        if let Some(t) = target {
            v["target"] = json!(t);
        }
        self.bin.extend_from_slice(bytes);
        self.views.push(v);
        self.views.len() - 1
    }

    fn accessor(
        &mut self,
        view: usize,
        offset: usize,
        kind: &str,
        count: usize,
        comp: u32,
    ) -> usize {
        self.accessors.push(json!({
            "bufferView": view,
            "byteOffset": offset,
            "componentType": comp,
            "count": count,
            "type": kind,
        }));
        self.accessors.len() - 1
    }

    fn floats<const N: usize>(&mut self, data: &[[f32; N]], kind: &str) -> usize {
        let bytes: Vec<u8> = data
            .iter()
            .flatten()
            .flat_map(|f| f.to_le_bytes())
            .collect();
        let view = self.view(&bytes, Some(ARRAY_BUFFER));
        self.accessor(view, 0, kind, data.len(), FLOAT)
    }

    /// Index of the glTF texture and whether its image carries alpha.
    fn texture(&mut self, game: &GameData, name: &str, srgb_hint: &str) -> Option<(usize, bool)> {
        let key = name.to_ascii_lowercase();
        if let Some(&cached) = self.texture_index.get(&key) {
            return cached;
        }
        let result = (|| -> Result<(usize, bool)> {
            let bytes = game
                .texture(name)
                .with_context(|| format!("texture {name:?} not found"))?;
            let rfi = Rfi::parse(bytes)?;
            let rgba = rfi.decode_rgba8()?;
            let png = encode_png(rfi.header.width, rfi.header.height, &rgba)?;
            let view = self.view(&png, None);
            self.images
                .push(json!({ "bufferView": view, "mimeType": "image/png", "name": name }));
            self.textures
                .push(json!({ "source": self.images.len() - 1, "name": srgb_hint }));
            let alpha = matches!(
                rfi.format,
                PixelFormat::Bc3 | PixelFormat::Rgba8 | PixelFormat::La8 | PixelFormat::A8
            );
            Ok((self.textures.len() - 1, alpha))
        })();
        let idx = match result {
            Ok(i) => Some(i),
            Err(e) => {
                self.warnings.push(format!("{e:#}"));
                None
            }
        };
        self.texture_index.insert(key, idx);
        idx
    }

    fn material(&mut self, game: &GameData, name: &str, mat: Option<&Material>) -> usize {
        let key = name.to_ascii_lowercase();
        if let Some(&i) = self.material_index.get(&key) {
            return i;
        }
        let mut m = json!({
            "name": name,
            "pbrMetallicRoughness": { "metallicFactor": 0.0, "roughnessFactor": 1.0 },
        });
        match mat {
            Some(mat) => {
                if let Some(t) = mat.texture(TextureSlot::Diffuse)
                    && let Some((i, alpha)) = self.texture(game, t, "diffuse")
                {
                    m["pbrMetallicRoughness"]["baseColorTexture"] = json!({ "index": i });
                    if alpha {
                        m["alphaMode"] = json!("MASK");
                        m["doubleSided"] = json!(true);
                    }
                }
                if let Some(t) = mat.texture(TextureSlot::Normal)
                    && let Some((i, _)) = self.texture(game, t, "normal")
                {
                    m["normalTexture"] = json!({ "index": i });
                }
            }
            None => self.warnings.push(format!("material {name:?} not found")),
        }
        self.materials.push(m);
        self.material_index.insert(key, self.materials.len() - 1);
        self.materials.len() - 1
    }

    fn mesh(
        &mut self,
        game: &GameData,
        name: &str,
        mesh: &Mesh,
        lookup: &dyn Fn(&str) -> Option<Material>,
    ) -> usize {
        let (min, max) = bounds(&mesh.positions);
        let pos = self.floats(&mesh.positions, "VEC3");
        self.accessors[pos]["min"] = json!(min);
        self.accessors[pos]["max"] = json!(max);
        let nrm = self.floats(&mesh.normals, "VEC3");
        let uv = mesh.uvs.as_ref().map(|u| self.floats(u, "VEC2"));

        let index_bytes: Vec<u8> = mesh.indices.iter().flat_map(|i| i.to_le_bytes()).collect();
        let index_view = self.view(&index_bytes, Some(ELEMENT_ARRAY_BUFFER));
        let mut primitives = Vec::new();
        for g in &mesh.groups {
            let acc = self.accessor(
                index_view,
                g.first_index as usize * 4,
                "SCALAR",
                g.index_count as usize,
                UNSIGNED_INT,
            );
            let mat = lookup(&g.material);
            let material = self.material(game, &g.material, mat.as_ref());
            let mut attrs = json!({ "POSITION": pos, "NORMAL": nrm });
            if let Some(uv) = uv {
                attrs["TEXCOORD_0"] = json!(uv);
            }
            primitives.push(json!({ "attributes": attrs, "indices": acc, "material": material }));
        }
        self.meshes
            .push(json!({ "name": name, "primitives": primitives }));
        self.meshes.len() - 1
    }
}

/// Writes `scene` to `out` and returns warnings (missing materials/textures).
pub fn export(game: &GameData, scene: &Scene, title: &str, out: &Path) -> Result<Vec<String>> {
    let libraries = material_libraries(game);
    let lookup = |name: &str| -> Option<Material> {
        if let Some(m) = scene.material(name) {
            return Some(m.clone());
        }
        libraries.iter().find_map(|lib| {
            let archive = Archive::parse(lib).ok()?;
            Material::parse(archive.get(name)?).ok()
        })
    };

    let mut b = Builder::default();
    for node in &scene.nodes {
        let t = node.transform;
        let matrix = [
            t[0][0], t[0][1], t[0][2], 0.0, t[1][0], t[1][1], t[1][2], 0.0, t[2][0], t[2][1],
            t[2][2], 0.0, t[3][0], t[3][1], t[3][2], 1.0,
        ];
        let mut n = json!({ "name": node.name, "matrix": matrix });
        if let NodeBody::Mesh(mesh) = &node.body
            && !mesh.groups.is_empty()
        {
            n["mesh"] = json!(b.mesh(game, &node.name, mesh, &lookup));
        }
        b.nodes.push(n);
    }
    let mut roots = Vec::new();
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); scene.nodes.len()];
    for (i, node) in scene.nodes.iter().enumerate() {
        match node.parent {
            Some(p) if p >= 1 && p <= scene.nodes.len() && p - 1 != i => children[p - 1].push(i),
            _ => roots.push(i),
        }
    }
    for (i, kids) in children.into_iter().enumerate() {
        if !kids.is_empty() {
            b.nodes[i]["children"] = json!(kids);
        }
    }

    let mut doc = json!({
        "asset": { "version": "2.0", "generator": "exa-tool (Exarust)" },
        "scene": 0,
        "scenes": [{ "name": title, "nodes": roots }],
        "nodes": b.nodes,
        "buffers": [{ "byteLength": b.bin.len() }],
        "bufferViews": b.views,
        "accessors": b.accessors,
    });
    for (key, list) in [
        ("meshes", b.meshes),
        ("materials", b.materials),
        ("textures", b.textures),
        ("images", b.images),
    ] {
        if !list.is_empty() {
            doc[key] = Value::Array(list);
        }
    }
    write_glb(out, &serde_json::to_vec(&doc)?, &b.bin)?;
    Ok(b.warnings)
}

fn material_libraries(game: &GameData) -> Vec<&[u8]> {
    let mut libs: Vec<&[u8]> = game
        .project()
        .material_libraries
        .iter()
        .filter_map(|p| game.resource_file(p))
        .collect();
    if let Some(human) = game.resource_file("humanbase.rml") {
        libs.push(human);
    }
    libs
}

fn bounds(p: &[[f32; 3]]) -> ([f32; 3], [f32; 3]) {
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    for v in p {
        for k in 0..3 {
            min[k] = min[k].min(v[k]);
            max[k] = max[k].max(v[k]);
        }
    }
    if p.is_empty() {
        return ([0.0; 3], [0.0; 3]);
    }
    (min, max)
}

fn encode_png(w: u32, h: u32, rgba: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header()?.write_image_data(rgba)?;
    }
    Ok(out)
}

fn write_glb(path: &Path, json: &[u8], bin: &[u8]) -> Result<()> {
    let pad = |n: usize| (4 - n % 4) % 4;
    let json_len = json.len() + pad(json.len());
    let bin_len = bin.len() + pad(bin.len());
    let total = 12 + 8 + json_len + if bin.is_empty() { 0 } else { 8 + bin_len };
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(b"glTF");
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(json_len as u32).to_le_bytes());
    out.extend_from_slice(b"JSON");
    out.extend_from_slice(json);
    out.extend(std::iter::repeat_n(b' ', pad(json.len())));
    if !bin.is_empty() {
        out.extend_from_slice(&(bin_len as u32).to_le_bytes());
        out.extend_from_slice(b"BIN\0");
        out.extend_from_slice(bin);
        out.extend(std::iter::repeat_n(0u8, pad(bin.len())));
    }
    std::fs::write(path, out).with_context(|| path.display().to_string())
}
