//! `exa-tool shader --variants`: every surface-program variant the shipped
//! data builds, next to the `exa.wesl` branch Exarust draws it with.
//!
//! The original keys a surface program by its material and shader words
//! (plus the frame's object and environment words and the light count).
//! This walks every material record a draw can reach, as the model loader
//! resolves it (a library record first, then the chunk tree's own), groups
//! the records by program set and key words, and prints for each group the
//! template flags its key sets, the object streams its meshes carry, the
//! number of distinct expanded fragment programs over the level settings
//! the game ships (fog modes, outdoor, wet floor), and the loader's
//! [`exa_assets::RecordPath`]: the shading flags and maps the WESL branches
//! on. Rules: `docs/RENDERING.md` (Shader builder, Shipped variants).

use std::collections::{BTreeMap, BTreeSet, HashSet};

use anyhow::{Result, anyhow};
use exa_formats::FileKind;
use exa_formats::flb::ShaderLibrary;
use exa_formats::material::Material;
use exa_formats::rfc::{NodeBody, Scene};
use exa_render::material::{flags, flags2};
use exa_render::shader_program::{
    GeometryOptions, SceneOptions, key_words, program_for_shader_word, program_keys,
};
use exa_rpk::GameData;

/// One program group: the records sharing a set and key words.
#[derive(Default)]
struct Group {
    records: BTreeSet<String>,
    uses: usize,
    vertex_colors: bool,
    plain: bool,
    vertex_delta: bool,
    paths: BTreeSet<String>,
    race_skin: bool,
    sample: Option<Material>,
}

/// The loader's branch names for a record's path.
fn path_names(m: &Material) -> String {
    let path = exa_assets::record_path(m);
    let mut out = Vec::new();
    let named = [
        (flags::LIT, "lit"),
        (flags::TWO_SIDED, "two-sided"),
        (flags::PACKED_SPECULAR, "packed"),
        (flags::HAIR, "hair"),
        (flags::NO_SPECULAR, "no-specular"),
    ];
    for (bit, name) in named {
        if path.flags & bit != 0 {
            out.push(name);
        }
    }
    if path.flags & flags::LIT != 0 && path.flags & flags::PACKED_SPECULAR == 0 {
        out.push("legacy");
    }
    if path.flags2 & flags2::SMOKE != 0 {
        out.push("smoke");
    }
    if path.flags2 & flags2::LOCAL_REFLECTION != 0 {
        out.push("local-reflection");
    }
    if path.flags2 & flags2::FAST_LIGHT != 0 {
        out.push("fast-light");
    }
    if path.flags2 & flags2::TRANSLUCENT != 0 {
        out.push("translucent");
    }
    let maps = path.maps;
    for (on, name) in [
        (maps.diffuse, "diffuse"),
        (maps.opacity, "opacity"),
        (maps.emissive, "emissive"),
        (maps.specular, "specular"),
        (maps.normal, "normal"),
        (maps.gloss, "gloss"),
        (maps.height, "height"),
    ] {
        if on {
            out.push(name);
        }
    }
    out.join(" ")
}

pub fn variants(game: &GameData, request_lights: u32) -> Result<()> {
    let shaders = ShaderLibrary::parse(
        game.resource_file("shading.flb")
            .ok_or_else(|| anyhow!("shading.flb is not installed"))?,
    )?;
    let libraries: Vec<(String, exa_rpk::Archive)> = game
        .project()
        .material_libraries
        .iter()
        .map(String::as_str)
        .chain(["humanbase.rml"])
        .filter_map(|lib| {
            Some((
                lib.to_string(),
                exa_rpk::Archive::parse(game.resource_file(lib)?).ok()?,
            ))
        })
        .collect();
    let library = |name: &str| {
        libraries.iter().find_map(|(lib, archive)| {
            Some((lib.clone(), Material::parse(archive.get(name)?).ok()?))
        })
    };
    let mut groups: BTreeMap<(&'static str, u32, u32), Group> = BTreeMap::new();
    let mut nothing = BTreeSet::new();
    let mut add = |groups: &mut BTreeMap<(&'static str, u32, u32), Group>,
                   name: String,
                   m: &Material,
                   used: Option<(bool, bool, bool)>| {
        let Some((mat, shd)) = key_words(m, 5) else {
            nothing.insert(name);
            return;
        };
        let set = program_for_shader_word(shd).set();
        let group = groups.entry((set, mat, shd)).or_default();
        group.records.insert(name);
        group.paths.insert(path_names(m));
        if group.sample.is_none() {
            group.sample = Some(m.clone());
        }
        if let Some((colours, delta, skin)) = used {
            group.uses += 1;
            group.vertex_colors |= colours;
            group.plain |= !colours;
            group.vertex_delta |= delta;
            group.race_skin |= skin;
        }
    };
    // Every library record, used or not.
    for (lib, archive) in &libraries {
        for entry in archive.entries() {
            if let Ok(m) = Material::parse(archive.bytes(entry)) {
                add(&mut groups, format!("{lib}:{}", entry.name), &m, None);
            }
        }
    }
    // Every mesh group of every chunk tree, resolved as the loader does.
    for (kind, pack) in game.packs() {
        for e in pack.entries() {
            let b = pack.bytes(e);
            if !matches!(
                FileKind::detect(b),
                FileKind::ChunkTree | FileKind::ChunkTreeAlt
            ) {
                continue;
            }
            let Ok(scene) = Scene::parse(b) else { continue };
            for node in &scene.nodes {
                let NodeBody::Mesh(mesh) = &node.body else {
                    continue;
                };
                let colours = mesh.colors.is_some();
                let delta = node.flags & exa_render::shader_program::NODE_VERTEX_DELTA != 0;
                for g in &mesh.groups {
                    if exa_assets::model::apparel_texture_key(&g.material).is_some() {
                        continue;
                    }
                    let skin = exa_assets::human_skin::is_race_skin(&e.name, &g.material);
                    let (name, m) = match library(&g.material) {
                        Some((lib, m)) => (format!("{lib}:{}", g.material), m),
                        None => match scene.material(&g.material) {
                            Some(m) => (
                                format!("{}/{}:{}", kind.file_stem(), e.name, g.material),
                                m.clone(),
                            ),
                            None => continue,
                        },
                    };
                    add(&mut groups, name, &m, Some((colours, delta, skin)));
                }
            }
        }
    }

    // The level settings the shipped levels take: fog off, distance fog,
    // fade to black; indoors and outdoors; dry and wet.
    let mut scenes = Vec::new();
    for fog_mode in 0..3 {
        for outdoor in [false, true] {
            for wet_floor in [false, true] {
                scenes.push(SceneOptions {
                    fog_mode,
                    outdoor,
                    wet_floor,
                    ..SceneOptions::default()
                });
            }
        }
    }
    let mut all_programs = HashSet::new();
    println!(
        "{} program groups (set, material word, shader word) at shading quality 5, \
         shadow quality 4, super-sampling 3; {} records build nothing",
        groups.len(),
        nothing.len()
    );
    for ((set, mat, shd), group) in &groups {
        let m = group.sample.as_ref().expect("a group has a record");
        let mut programs = HashSet::new();
        let streams: Vec<GeometryOptions> =
            [(false, false), (true, false), (false, true), (true, true)]
                .into_iter()
                .filter(|&(c, d)| {
                    (if c {
                        group.vertex_colors
                    } else {
                        group.plain || group.uses == 0
                    }) && (!d || group.vertex_delta)
                })
                .map(|(vertex_colors, vertex_delta)| GeometryOptions {
                    vertex_colors,
                    vertex_delta,
                })
                .collect();
        let mut features = BTreeSet::new();
        for scene in &scenes {
            for geometry in &streams {
                for key in program_keys(m, *geometry, scene) {
                    if key.numbers.lights > request_lights {
                        continue;
                    }
                    for name in exa_render::shader_template::flag_names(&key.flags) {
                        if !name.starts_with("Env.") {
                            features.insert(name);
                        }
                    }
                    let program = program_for_shader_word(key.flags.shader);
                    if let Ok(text) = program.expand(&shaders, &key.context()) {
                        programs.insert(text.fragment.clone());
                        all_programs.insert((text.vertex, text.fragment));
                    }
                }
            }
        }
        let names: Vec<&String> = group.records.iter().take(4).collect();
        println!(
            "\n{set}_f mat {mat:#010x} shd {shd:#010x}: {} records, {} draws{}{}",
            group.records.len(),
            group.uses,
            if group.race_skin { ", race skin" } else { "" },
            if group.uses == 0 { ", unused" } else { "" },
        );
        println!(
            "  e.g. {}",
            names
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
        println!(
            "  template: {}",
            features.into_iter().collect::<Vec<_>>().join(" ")
        );
        println!(
            "  distinct fragment programs (0-{request_lights} lights, 12 level settings): {}",
            programs.len()
        );
        for path in &group.paths {
            println!("  wesl: {path}");
        }
    }
    println!(
        "\n{} distinct expanded programs (vertex + fragment) in all",
        all_programs.len()
    );
    Ok(())
}
