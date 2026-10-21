//! `exa-tool`: inspect and extract Exanima data from your own installation.
//!
//! ```text
//! exa-tool info                       packs, entry counts, project file
//! exa-tool ls Resource [--inner common.rml]
//! exa-tool stats Objlib               formats by magic
//! exa-tool chunks Resource arena.rfc  chunk tree of an .rfc
//! exa-tool rfi Textures afcrnflg_01d  image header
//! exa-tool png Textures afcrnflg_01d out.png
//! exa-tool extract Sound out/ [--filter msc_]
//! exa-tool corpus meshes              decode every chunk tree, report coverage
//! exa-tool corpus images              images by format; RGB8 normals z in blue
//! exa-tool corpus winding             which way level faces point, floors up?
//! exa-tool rsq                        skeleton, actions and the moveset tree of humanoid.rsq
//! exa-tool clips Ogre 0x1             a character's clip table: base moveset, held-item config
//! exa-tool races                      races with their flags, body parameters, stats, gaits
//! exa-tool characters exanima01       a level's character records, read whole, and placements
//! exa-tool characters actors          the arena's actors in actors.rpk
//! exa-tool bodies Objects "lever a01"  rigid bodies, links and cloth markers
//! exa-tool mechanisms [exanima01]     door and operative records
//! exa-tool morphs [--records]         physique morph sets; weights of every placed record
//! exa-tool corpus bodies              bodies, links and cloth markers per pack
//! exa-tool corpus tracks              every node keyframe track of every chunk tree
//! exa-tool export-gltf Objlib afr_cornflag_a01 out.glb
//! exa-tool nav exanima01 [--png nav.png]  the level's navigation grid
//! exa-tool shader common.rml mtliron04   the original GLSL for a material
//! ```

mod bodies;
mod characters;
mod corpus;
mod gltf;
mod morphs;
mod nav;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use exa_formats::FileKind;
use exa_formats::chunk::{self, Chunk, Chunks};
use exa_formats::rfi::Rfi;
use exa_rpk::{Archive, GameData, PackKind};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Exanima installation; defaults to EXANIMA_DIR (environment or .env).
    #[arg(long, global = true)]
    game: Option<PathBuf>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Project file and pack inventory.
    Info,
    /// List entries of a pack, or of an archive nested inside it.
    Ls {
        pack: String,
        #[arg(long)]
        inner: Option<String>,
    },
    /// Count entries by detected format.
    Stats { pack: String },
    /// Print the chunk tree of an .rfc entry.
    Chunks {
        pack: String,
        entry: String,
        #[arg(long, default_value_t = 4)]
        depth: usize,
    },
    /// Print an .rfi header.
    Rfi { pack: String, entry: String },
    /// Decode the base level of an .rfi to PNG.
    Png {
        pack: String,
        entry: String,
        out: PathBuf,
    },
    /// Paint an apparel texture set (e.g. `doublet_aa`) with the appearance
    /// of the first item worn as `model` (e.g. `doublet_a`), as the game
    /// does.
    Paint {
        model: String,
        stem: String,
        out: PathBuf,
    },
    /// Summarise the skeleton and motion set of a `.rsq`.
    Rsq {
        #[arg(default_value = "humanoid.rsq")]
        entry: String,
    },
    /// The clip table of `humanoid.rsq` a character plays with this base
    /// moveset (its race's own, `Default` for a human) and held-item config
    /// (`0x1` One Handed, `0x10010001` with the torch, …): every action with
    /// the moveset its clip comes from.
    Clips {
        base: String,
        #[arg(default_value = "0", value_parser = parse_config)]
        config: u32,
    },
    /// Every race of `races.rdb` with its flags, body parameters, stats and
    /// gait table.
    Races,
    /// Every character record of a level (`exanima01`), with where it
    /// places them, or of the arena's actors (`actors`): looks, feral level,
    /// saved pools, skills with their techniques, perks, roles and powers.
    Characters { source: String },
    /// The rigid bodies (class, mobility, rest points, spheres, links,
    /// sound slots) and cloth markers of a chunk-tree entry's nodes.
    Bodies { pack: String, entry: String },
    /// Door and operative records of `objdb.rdb`, or of a level's own
    /// database (e.g. `exanima01`).
    Mechanisms { level: Option<String> },
    /// The physique morph sets of the races' `*morph.rfc` (landmarks, each
    /// target's reach); with `--records`, the weights every placed
    /// character's record gives them.
    Morphs {
        #[arg(long)]
        records: bool,
    },
    /// Decode every entry of a format and report coverage.
    Corpus {
        #[arg(value_enum)]
        what: CorpusKind,
    },
    /// Build a level's navigation grid as the game does at load, print its
    /// counts, and with `--png` draw its two layers side by side.
    Nav {
        level: String,
        #[arg(long)]
        png: Option<PathBuf>,
    },
    /// Export a chunk tree's meshes to binary glTF with resolved textures.
    ExportGltf {
        pack: String,
        entry: String,
        out: PathBuf,
    },
    /// Print the original GLSL a material's program expands to, from
    /// `Resource.rpk`'s `shading.flb`; with `--warmup`, the 29 warmed
    /// variants, and with `--list`, the library's entries.
    Shader {
        /// Material library inside `Resource.rpk` (e.g. `common.rml`).
        #[arg(default_value = "common.rml")]
        library: String,
        /// Shader library to read (`shading.flb` for the material programs,
        /// `textures.flb` for the procedural textures).
        #[arg(long, default_value = "shading.flb")]
        shader_library: String,
        /// Material name in that library.
        material: Option<String>,
        /// Lights on the object, 0 through 6.
        #[arg(long, default_value_t = 0)]
        lights: u32,
        /// Shading quality, 1 through 6.
        #[arg(long, default_value_t = 5)]
        quality: u32,
        /// Shadow quality, 1 through 6 (the taps before supersampling).
        #[arg(long, default_value_t = 4)]
        shadow_quality: u32,
        /// Supersampling level: 0 off, 1, or 3 for the original's 4x.
        #[arg(long, default_value_t = 3)]
        supersample: u32,
        /// The mesh has a vertex-colour stream (`Obj.VrtCol`).
        #[arg(long)]
        vertex_colors: bool,
        /// The mesh has deformed vertices (`Obj.VrtDlt`).
        #[arg(long)]
        vertex_delta: bool,
        /// Fog mode: 0 off, 1 distance fog, 2 fade to black.
        #[arg(long, default_value_t = 1)]
        fog: u32,
        #[arg(long)]
        outdoor: bool,
        #[arg(long)]
        wet_floor: bool,
        #[arg(long)]
        vxgi_spec: bool,
        #[arg(long)]
        tone_map: bool,
        #[arg(long)]
        sharpen: bool,
        /// Leave the voxel GI out (`Env.VxlGI` clear).
        #[arg(long)]
        no_gi: bool,
        /// The first light is the sun, with its 2D shadow map.
        #[arg(long)]
        sun_first: bool,
        /// Print only the vertex stage.
        #[arg(long)]
        vertex_only: bool,
        /// Print only the fragment stage.
        #[arg(long)]
        fragment_only: bool,
        /// List the warm-up variants for these settings.
        #[arg(long)]
        warmup: bool,
        /// List the shader library's entries.
        #[arg(long)]
        list: bool,
    },
    /// Write entries to a directory, with an extension guessed from the magic.
    Extract {
        pack: String,
        out: PathBuf,
        #[arg(long)]
        filter: Option<String>,
        #[arg(long)]
        inner: Option<String>,
    },
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum CorpusKind {
    Meshes,
    Levels,
    Terrain,
    Factories,
    /// Images by format word and storage; RGB8 normal maps hold z in blue.
    Images,
    /// Near-horizontal level faces up or down, and mirrored nodes.
    Winding,
    /// Rigid bodies, links, sliders and cloth markers per model pack.
    Bodies,
    /// Every keyframe track of every chunk tree, nested packs included.
    Tracks,
    /// The mesh render word: shadow-bias levels and flags, with examples.
    RenderWords,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = locate(cli.game.as_deref())?;
    let game = GameData::open(&root).with_context(|| format!("opening {}", root.display()))?;
    match cli.cmd {
        Cmd::Info => info(&game),
        Cmd::Shader {
            library,
            shader_library,
            material,
            lights,
            quality,
            shadow_quality,
            supersample,
            vertex_colors,
            vertex_delta,
            fog,
            outdoor,
            wet_floor,
            vxgi_spec,
            tone_map,
            sharpen,
            no_gi,
            sun_first,
            vertex_only,
            fragment_only,
            warmup,
            list,
        } => shader(
            &game,
            &ShaderRequest {
                library,
                shader_library,
                material,
                lights,
                quality,
                shadow_quality,
                supersample,
                vertex_colors,
                vertex_delta,
                fog,
                outdoor,
                wet_floor,
                vxgi_spec,
                tone_map,
                sharpen,
                no_gi,
                sun_first,
                vertex_only,
                fragment_only,
                warmup,
                list,
            },
        ),
        Cmd::Ls { pack, inner } => with_archive(&game, &pack, inner.as_deref(), |a| {
            for e in a.entries() {
                let kind = FileKind::detect(a.bytes(e));
                println!("{:<18} {:>12} {:?}", e.name, e.size, kind);
            }
            Ok(())
        }),
        Cmd::Stats { pack } => with_archive(&game, &pack, None, |a| {
            let mut counts: BTreeMap<FileKind, (usize, u64)> = BTreeMap::new();
            for e in a.entries() {
                let c = counts.entry(FileKind::detect(a.bytes(e))).or_default();
                c.0 += 1;
                c.1 += u64::from(e.size);
            }
            for (k, (n, bytes)) in counts {
                println!(
                    "{:<28} {:>6} entries {:>12} bytes",
                    format!("{k:?}"),
                    n,
                    bytes
                );
            }
            Ok(())
        }),
        Cmd::Chunks { pack, entry, depth } => {
            let b = entry_bytes(&game, &pack, &entry)?;
            for c in chunk::rfc_chunks(b)? {
                print_chunk(&c?, 0, depth);
            }
            Ok(())
        }
        Cmd::Rfi { pack, entry } => {
            let rfi = Rfi::parse(entry_bytes(&game, &pack, &entry)?)?;
            println!("{:#?}", rfi.header);
            println!(
                "format: {:?}  mips: {}  packed: {}",
                rfi.format,
                rfi.mip_count,
                rfi.is_packed()
            );
            Ok(())
        }
        Cmd::Paint { model, stem, out } => paint_item(&game, &model, &stem, &out),
        Cmd::Png { pack, entry, out } => {
            let rfi = Rfi::parse(entry_bytes(&game, &pack, &entry)?)?;
            let rgba = rfi.decode_rgba8()?;
            write_png(&out, rfi.header.width, rfi.header.height, &rgba)?;
            println!(
                "{} {}x{} {:?} -> {}",
                entry,
                rfi.header.width,
                rfi.header.height,
                rfi.format,
                out.display()
            );
            Ok(())
        }
        Cmd::Rsq { entry } => {
            let seq = exa_formats::rsq::Sequences::parse(entry_bytes(&game, "Resource", &entry)?)?;
            println!("bones:");
            for (i, n) in seq.skeleton.nodes.iter().enumerate() {
                let body = seq.bodies.get(i).cloned().unwrap_or_default();
                println!(
                    "  {:<16} parent {:?}  links {:?}  contacts {}  pins {}",
                    n.name,
                    n.parent,
                    body.links
                        .iter()
                        .map(|l| (l.slack * 100.0).round() / 100.0)
                        .collect::<Vec<_>>(),
                    body.contacts.len(),
                    body.pins.len()
                );
            }
            let m = &seq.motions;
            println!("{} actions, {} movesets", m.actions.len(), m.movesets.len());
            for (i, a) in m.actions.iter().enumerate() {
                println!("  action {i:#04x} {:<20} params {:?}", a.name, a.params);
            }
            for (i, ms) in m.movesets.iter().enumerate() {
                let defined = ms.clips.iter().filter(|c| c.is_some()).count();
                let keys: usize = ms.clips.iter().flatten().map(|c| c.keys.len()).sum();
                let parent = m
                    .parent_of(i)
                    .map_or(String::new(), |p| format!("{} #{p}", ms.parent));
                println!(
                    "  #{i:<2} {:<14} <- {parent:<16} {defined:>2} actions, {keys:>3} keys",
                    ms.name
                );
            }
            Ok(())
        }
        Cmd::Clips { base, config } => {
            let seq = exa_formats::rsq::Sequences::parse(entry_bytes(
                &game,
                "Resource",
                "humanoid.rsq",
            )?)?;
            let m = &seq.motions;
            let config = exa_formats::rsq::MovesetConfig(config);
            let table = m
                .assemble(&base, config)
                .ok_or_else(|| anyhow!("no moveset named {base:?}"))?;
            println!("{base} with {config} ({:#x}):", config.0);
            for (a, (action, from)) in m.actions.iter().zip(&table.0).enumerate() {
                let from = from.map_or("-".into(), |i| format!("{} #{i}", m.movesets[i].name));
                println!("  action {a:#04x} {:<20} {from}", action.name);
            }
            Ok(())
        }
        Cmd::Races => races(&game),
        Cmd::Characters { source } => characters::characters(&game, &source),
        Cmd::Bodies { pack, entry } => bodies::bodies(&game, &pack, &entry),
        Cmd::Mechanisms { level } => bodies::mechanisms(&game, level.as_deref()),
        Cmd::Morphs { records } => morphs::morphs(&game, records),
        Cmd::Corpus {
            what: CorpusKind::Meshes,
        } => corpus::meshes(&game),
        Cmd::Corpus {
            what: CorpusKind::Levels,
        } => corpus::levels(&game),
        Cmd::Corpus {
            what: CorpusKind::Terrain,
        } => corpus::terrain(&game),
        Cmd::Corpus {
            what: CorpusKind::Factories,
        } => corpus::factories(&game),
        Cmd::Corpus {
            what: CorpusKind::Images,
        } => corpus::images(&game),
        Cmd::Corpus {
            what: CorpusKind::Winding,
        } => corpus::winding(&game),
        Cmd::Corpus {
            what: CorpusKind::Bodies,
        } => bodies::corpus(&game),
        Cmd::Corpus {
            what: CorpusKind::Tracks,
        } => corpus::tracks(&game),
        Cmd::Corpus {
            what: CorpusKind::RenderWords,
        } => corpus::render_words(&game),
        Cmd::Nav { level, png } => nav::nav(&game, &level, png.as_deref()),
        Cmd::ExportGltf { pack, entry, out } => {
            let scene = exa_formats::rfc::Scene::parse(entry_bytes(&game, &pack, &entry)?)
                .with_context(|| format!("decoding {pack}/{entry}"))?;
            let warnings = gltf::export(&game, &scene, &entry, &out)?;
            let meshes = scene.meshes().count();
            println!(
                "{pack}/{entry}: {} nodes, {meshes} meshes -> {}",
                scene.nodes.len(),
                out.display()
            );
            for w in warnings {
                eprintln!("warning: {w}");
            }
            Ok(())
        }
        Cmd::Extract {
            pack,
            out,
            filter,
            inner,
        } => with_archive(&game, &pack, inner.as_deref(), |a| {
            extract(a, &out, filter.as_deref())
        }),
    }
}

fn races(game: &GameData) -> Result<()> {
    let races = exa_formats::races::parse(entry_bytes(game, "Resource", "races.rdb")?)?;
    println!(
        "{} races (factors as stored; gait rows per second)",
        races.len()
    );
    for r in &races {
        let (b, s) = (&r.body, &r.stats);
        println!(
            "{:>3} {:<16} {:<18} {} / {:<10} health {}",
            r.id,
            r.name,
            r.model(),
            r.movesets[0],
            r.movesets[1],
            r.health
        );
        println!(
            "    flags {:#x} / {:#x}  kind {:#x}{}{}",
            r.flags,
            s.flags2,
            s.kind,
            if r.clawed() { "  clawed" } else { "" },
            if r.regenerates_red() {
                "  regenerates red"
            } else {
                ""
            }
        );
        println!(
            "    clip rate {}  size {}  scale {}  mass divisor {}",
            b.clip_rate, b.size, b.scale, b.mass_divisor
        );
        println!(
            "    pins {}  balance bonus {}  stance {}  arm relaxation {}  damping ratio {}",
            b.pin_factor, b.balance_bonus, b.stance, b.arm_relaxation, b.damping_ratio
        );
        println!(
            "    unarmed reach {} m  swing turn {}  recovery {}  AI attack bonus {}  perception {}  held offset {:.3?} m",
            s.unarmed_reach, s.swing_turn, s.recovery, s.attack_bonus, s.perception, s.held_offset
        );
        println!(
            "    footsteps {:?}  voice volume {}  attack voices {}",
            s.footsteps, s.voice_volume, s.attack_voices
        );
        for (name, g) in exa_formats::races::GAIT_NAMES.iter().zip(&b.gaits) {
            println!(
                "    {name:<9} thrust {:>8.3}  push {:>6}  brake {:>3} m  clip rate {:>5}  boost below {}",
                g.thrust, g.push_scale, g.brake_distance, g.clip_rate, g.boost_below
            );
        }
    }
    Ok(())
}

fn locate(explicit: Option<&Path>) -> Result<PathBuf> {
    exa_rpk::locate_install(explicit).ok_or_else(|| {
        anyhow!("no installation: pass --game or set EXANIMA_DIR (see .env.example)")
    })
}

/// A moveset config word, in hex (`0x10010001`) or decimal.
fn parse_config(s: &str) -> Result<u32, String> {
    let parsed = match s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        Some(hex) => u32::from_str_radix(&hex.replace('_', ""), 16),
        None => s.parse(),
    };
    parsed.map_err(|e| format!("{s:?}: {e}"))
}

fn pack_kind(name: &str) -> Result<PackKind> {
    PackKind::from_name(name).ok_or_else(|| anyhow!("unknown pack {name:?}"))
}

fn with_archive(
    game: &GameData,
    pack: &str,
    inner: Option<&str>,
    f: impl FnOnce(&Archive<'_>) -> Result<()>,
) -> Result<()> {
    let mapped = game
        .pack(pack_kind(pack)?)
        .ok_or_else(|| anyhow!("{pack}.rpk is not installed"))?;
    match inner {
        None => f(&Archive::parse(mapped.data())?),
        Some(inner) => {
            let b = mapped
                .get(inner)
                .ok_or_else(|| anyhow!("{inner:?} not found in {pack}"))?;
            f(&Archive::parse(b)?)
        }
    }
}

fn entry_bytes<'g>(game: &'g GameData, pack: &str, entry: &str) -> Result<&'g [u8]> {
    let mapped = game
        .pack(pack_kind(pack)?)
        .ok_or_else(|| anyhow!("{pack}.rpk is not installed"))?;
    mapped
        .get(entry)
        .ok_or_else(|| anyhow!("{entry:?} not found in {pack}"))
}

fn info(game: &GameData) -> Result<()> {
    let p = game.project();
    println!("install:   {}", game.root().display());
    println!("project:   {}", p.name);
    println!("resource:  {}", p.resource_pack);
    println!("objlibs:   {:?}", p.object_libraries);
    println!("texlibs:   {:?}", p.texture_libraries);
    println!("matlibs:   {:?}", p.material_libraries);
    println!();
    for (kind, pack) in game.packs() {
        let bytes: u64 = pack.entries().iter().map(|e| u64::from(e.size)).sum();
        println!(
            "{:<14} {:>6} entries {:>8.1} MiB",
            kind.file_stem(),
            pack.len(),
            bytes as f64 / 1048576.0
        );
    }
    Ok(())
}

fn print_chunk(c: &Chunk<'_>, depth: usize, max_depth: usize) {
    let preview: String = c
        .data
        .iter()
        .take(24)
        .map(|&b| {
            if b.is_ascii_graphic() || b == b' ' {
                b as char
            } else {
                '.'
            }
        })
        .collect();
    println!(
        "{:indent$}{:08x} {:>9}  {}",
        "",
        c.tag,
        c.data.len(),
        preview,
        indent = depth * 2
    );
    if depth + 1 >= max_depth {
        return;
    }
    if let Some((_, kids)) = chunk::list_children(c) {
        for k in &kids {
            print_chunk(k, depth + 1, max_depth);
        }
    } else if let Ok(kids) = Chunks::new(c.data, 0).collect::<exa_formats::Result<Vec<_>>>()
        && !kids.is_empty()
        && kids.iter().all(|k| k.tag != 0)
    {
        for k in &kids {
            print_chunk(k, depth + 1, max_depth);
        }
    }
}

fn paint_item(game: &GameData, model: &str, stem: &str, out: &Path) -> Result<()> {
    use exa_assets::paint::{
        PaintLayer, Texels, apparel_tint, layer_material, library, normal_library, paint,
        specular_library,
    };
    let db_bytes = game
        .pack(exa_rpk::PackKind::Resource)
        .and_then(|p| p.get("objdb.rdb"))
        .context("objdb.rdb not found")?;
    let db = exa_formats::rdb::Database::parse(db_bytes)?;
    let item = exa_formats::objdb::items(&db)
        .into_iter()
        .find(|i| i.worn_model.eq_ignore_ascii_case(model) && i.appearance.is_some())
        .with_context(|| format!("no item is worn as {model}"))?;
    let appearance = item.appearance.expect("filtered");
    let texels = |name: &str| -> Option<Texels> {
        let rfi = Rfi::parse(game.texture(name)?).ok()?;
        Some(Texels {
            width: rfi.header.width as usize,
            height: rfi.header.height as usize,
            rgba: rfi.decode_rgba8().ok()?,
        })
    };
    let shade = texels(&format!("{stem}d")).context("no shade map")?;
    let mut owned = Vec::new();
    for layer in appearance.used_layers() {
        let material = layer_material(layer);
        let Some(mask) = texels(&format!("{stem}m{}", layer.mask)) else {
            println!("  mask {} missing", layer.mask);
            continue;
        };
        let lib: Vec<Option<Texels>> = library(material).iter().map(|n| texels(n)).collect();
        let specular: Vec<Option<Texels>> = specular_library(material)
            .iter()
            .map(|n| texels(n))
            .collect();
        let normals: Vec<Option<Texels>> =
            normal_library(material).iter().map(|n| texels(n)).collect();
        println!(
            "  mask {} material {material} colour {:#04x} library {}/{}",
            layer.mask,
            layer.colour,
            lib.iter().filter(|t| t.is_some()).count(),
            lib.len()
        );
        owned.push((mask, material, layer.colour, lib, specular, normals));
    }
    let layers: Vec<PaintLayer<'_>> = owned
        .iter()
        .map(|(mask, material, colour, lib, specular, normals)| {
            let (tint, wear) = apparel_tint(*material, *colour);
            PaintLayer {
                mask,
                material: *material,
                tint,
                wear,
                library: lib.iter().map(Option::as_ref).collect(),
                specular: specular.iter().map(Option::as_ref).collect(),
                normals: normals.iter().map(Option::as_ref).collect(),
            }
        })
        .collect();
    let item_normal = texels(&format!("{stem}n"));
    let painted = paint(&shade, item_normal.as_ref(), &layers, &appearance);
    write_png(
        out,
        shade.width as u32,
        shade.height as u32,
        &painted.colour,
    )?;
    let surface = out.with_extension("surface.png");
    write_png(
        &surface,
        shade.width as u32,
        shade.height as u32,
        &painted.surface,
    )?;
    if let Some(normal) = &painted.normal {
        let path = out.with_extension("normal.png");
        write_png(&path, shade.width as u32, shade.height as u32, normal)?;
    }
    println!("{} ({}) -> {}", item.name, model, out.display());
    Ok(())
}

pub(crate) fn write_png(path: &Path, w: u32, h: u32, rgba: &[u8]) -> Result<()> {
    let file = fs::File::create(path).with_context(|| path.display().to_string())?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()?.write_image_data(rgba)?;
    Ok(())
}

fn extract(a: &Archive<'_>, out: &Path, filter: Option<&str>) -> Result<()> {
    fs::create_dir_all(out)?;
    let mut n = 0;
    for e in a.entries() {
        if filter.is_some_and(|f| !e.name.contains(f)) {
            continue;
        }
        let bytes = a.bytes(e);
        let has_ext = e
            .name
            .rsplit_once('.')
            .is_some_and(|(_, x)| (2..=3).contains(&x.len()));
        let name = if has_ext {
            e.name.clone()
        } else {
            format!("{}.{}", e.name, FileKind::detect(bytes).extension())
        };
        fs::write(out.join(name.replace(['/', '\\'], "_")), bytes)?;
        n += 1;
    }
    println!("extracted {n} entries to {}", out.display());
    Ok(())
}

/// The `shader` command's arguments.
struct ShaderRequest {
    library: String,
    shader_library: String,
    material: Option<String>,
    lights: u32,
    quality: u32,
    shadow_quality: u32,
    supersample: u32,
    vertex_colors: bool,
    vertex_delta: bool,
    fog: u32,
    outdoor: bool,
    wet_floor: bool,
    vxgi_spec: bool,
    tone_map: bool,
    sharpen: bool,
    no_gi: bool,
    sun_first: bool,
    vertex_only: bool,
    fragment_only: bool,
    warmup: bool,
    list: bool,
}

fn shader(game: &GameData, request: &ShaderRequest) -> Result<()> {
    use exa_formats::flb::ShaderLibrary;
    use exa_formats::material::Material;
    use exa_render::shader_program::{
        GeometryOptions, SceneOptions, SurfaceProgram, program_for_shader_word, program_keys,
        warmup_keys,
    };
    use exa_render::shader_template::LightKind;

    let bytes = game
        .resource_file(&request.shader_library)
        .ok_or_else(|| anyhow!("{} is not installed", request.shader_library))?;
    let shaders = ShaderLibrary::parse(bytes)?;
    let scene = SceneOptions {
        quality: request.quality,
        shadow_quality: request.shadow_quality,
        supersample_level: request.supersample,
        fog_mode: request.fog,
        voxel_gi: !request.no_gi,
        voxel_gi_specular: request.vxgi_spec,
        outdoor: request.outdoor,
        wet_floor: request.wet_floor,
        tone_map: request.tone_map,
        sharpen: request.sharpen,
    };

    if request.list {
        for entry in shaders.entries() {
            println!("{}", entry.name);
        }
        return Ok(());
    }
    if request.warmup {
        for key in warmup_keys(
            request.quality,
            request.shadow_quality,
            request.supersample,
            request.vxgi_spec,
        ) {
            let program = program_for_shader_word(key.key.flags.shader);
            println!(
                "row {:2} lights {} mat {:#010x} shd {:#010x} obj {:#010x} env {:#010x} {}_f",
                key.row + 1,
                key.key.numbers.lights,
                key.key.flags.material,
                key.key.flags.shader,
                key.key.flags.object,
                key.key.flags.environment,
                program.set(),
            );
        }
        return Ok(());
    }

    let name = request
        .material
        .as_deref()
        .ok_or_else(|| anyhow!("name a material, or pass --list or --warmup"))?;
    let library_bytes = game
        .resource_file(&request.library)
        .ok_or_else(|| anyhow!("{} is not installed", request.library))?;
    let library = Archive::parse(library_bytes)?;
    let payload = library
        .get(name)
        .ok_or_else(|| anyhow!("material {name:?} not found in {}", request.library))?;
    let material = Material::parse(payload)?;
    let geometry = GeometryOptions {
        vertex_colors: request.vertex_colors,
        vertex_delta: request.vertex_delta,
    };
    let keys = program_keys(&material, geometry, &scene);
    let wanted = request.lights.min(6);
    let mut key = keys
        .into_iter()
        .find(|key| key.numbers.lights == wanted)
        .ok_or_else(|| anyhow!("no program variant for {wanted} lights"))?;
    if request.sun_first && !key.lights.is_empty() {
        key.lights[0] = LightKind::Directional;
    }
    let program: SurfaceProgram = program_for_shader_word(key.flags.shader);
    let text = program.expand(&shaders, &key.context())?;
    println!(
        "// {name} from {}: {}_f, {} lights, SdwSmps {}, ShadeQual {}, DspSdws {}",
        request.library,
        program.set(),
        key.numbers.lights,
        key.numbers.shadow_samples,
        key.numbers.shade_quality,
        key.numbers.displacement_samples,
    );
    if !request.fragment_only {
        println!("// ---- vertex: shading_v");
        print!("{}", text.vertex);
    }
    if !request.vertex_only {
        println!("// ---- fragment: {}_f", program.set());
        print!("{}", text.fragment);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn configs_parse_in_hex_or_decimal() {
        assert_eq!(super::parse_config("0x10010001"), Ok(0x1001_0001));
        assert_eq!(super::parse_config("0X1001_0100"), Ok(0x1001_0100));
        assert_eq!(super::parse_config("64"), Ok(0x40));
        assert!(super::parse_config("0xz").is_err());
    }
}
