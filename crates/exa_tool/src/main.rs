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
//! exa-tool export-gltf Objlib afr_cornflag_a01 out.glb
//! ```

mod corpus;
mod gltf;

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
    /// of the item showing `model` (e.g. `doublet_a`), as the game does.
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
    /// Decode every entry of a format and report coverage.
    Corpus {
        #[arg(value_enum)]
        what: CorpusKind,
    },
    /// Export a chunk tree's meshes to binary glTF with resolved textures.
    ExportGltf {
        pack: String,
        entry: String,
        out: PathBuf,
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
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = locate(cli.game.as_deref())?;
    let game = GameData::open(&root).with_context(|| format!("opening {}", root.display()))?;
    match cli.cmd {
        Cmd::Info => info(&game),
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
            for ms in &m.movesets {
                let defined = ms.clips.iter().filter(|c| c.is_some()).count();
                let keys: usize = ms.clips.iter().flatten().map(|c| c.keys.len()).sum();
                println!(
                    "  {:<14} <- {:<12} {defined:>2} actions, {keys:>3} keys",
                    ms.name, ms.parent
                );
            }
            Ok(())
        }
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

fn locate(explicit: Option<&Path>) -> Result<PathBuf> {
    exa_rpk::locate_install(explicit).ok_or_else(|| {
        anyhow!("no installation: pass --game or set EXANIMA_DIR (see .env.example)")
    })
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
    use exa_assets::paint::{PaintLayer, Texels, layer_material, library, normal_library, paint};
    let db_bytes = game
        .pack(exa_rpk::PackKind::Resource)
        .and_then(|p| p.get("objdb.rdb"))
        .context("objdb.rdb not found")?;
    let db = exa_formats::rdb::Database::parse(db_bytes)?;
    let item = exa_formats::objdb::items(&db)
        .into_iter()
        .find(|i| i.model.eq_ignore_ascii_case(model) && i.appearance.is_some())
        .with_context(|| format!("no item shows {model}"))?;
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
        let normals: Vec<Option<Texels>> =
            normal_library(material).iter().map(|n| texels(n)).collect();
        println!(
            "  mask {} material {material} colour {:#04x} library {}/{}",
            layer.mask,
            layer.colour,
            lib.iter().filter(|t| t.is_some()).count(),
            lib.len()
        );
        owned.push((mask, material, layer.colour, lib, normals));
    }
    let layers: Vec<PaintLayer<'_>> = owned
        .iter()
        .map(|(mask, material, colour, lib, normals)| PaintLayer {
            mask,
            material: *material,
            colour: *colour,
            library: lib.iter().map(Option::as_ref).collect(),
            normals: normals.iter().map(Option::as_ref).collect(),
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

fn write_png(path: &Path, w: u32, h: u32, rgba: &[u8]) -> Result<()> {
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
