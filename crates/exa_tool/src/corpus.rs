//! `exa-tool corpus meshes`: decode every chunk tree and report coverage.

use std::collections::BTreeMap;

use anyhow::Result;
use exa_formats::FileKind;
use exa_formats::rfc::{NodeBody, Scene};
use exa_rpk::GameData;

#[derive(Default)]
struct PackTally {
    files: usize,
    file_errors: usize,
    meshes: usize,
    invalid: usize,
    instances: usize,
    other_nodes: usize,
    materials: usize,
    bad_materials: usize,
    gi_meshes: usize,
    gi_points: usize,
}

pub fn meshes(game: &GameData) -> Result<()> {
    let mut reasons: BTreeMap<String, (usize, String)> = BTreeMap::new();
    let mut note = |reason: String, example: String| {
        let key = generalise(&reason);
        reasons.entry(key).or_insert((0, example)).0 += 1;
    };
    println!(
        "{:<13} {:>6} {:>6} {:>8} {:>7} {:>6} {:>6} {:>9} {:>6}",
        "pack", "files", "f.err", "meshes", "invalid", "inst", "other", "materials", "m.err"
    );
    let mut total = PackTally::default();
    for (kind, pack) in game.packs() {
        let mut t = PackTally::default();
        for e in pack.entries() {
            let b = pack.bytes(e);
            if !matches!(
                FileKind::detect(b),
                FileKind::ChunkTree | FileKind::ChunkTreeAlt
            ) {
                continue;
            }
            t.files += 1;
            let scene = match Scene::parse(b) {
                Ok(s) => s,
                Err(err) => {
                    t.file_errors += 1;
                    note(
                        format!("file: {err}"),
                        format!("{}/{}", kind.file_stem(), e.name),
                    );
                    continue;
                }
            };
            for node in &scene.nodes {
                match &node.body {
                    NodeBody::Mesh(m) => {
                        t.meshes += 1;
                        if !m.gi_points.is_empty() {
                            t.gi_meshes += 1;
                            t.gi_points += m.gi_points.len();
                        }
                    }
                    NodeBody::Instance(_) => t.instances += 1,
                    NodeBody::NotMesh | NodeBody::Light(_) | NodeBody::Emitter { .. } => {
                        t.other_nodes += 1
                    }
                    NodeBody::Invalid(err) => {
                        t.invalid += 1;
                        note(
                            format!("mesh: {err}"),
                            format!("{}/{} [{}]", kind.file_stem(), e.name, node.name),
                        );
                    }
                }
            }
            for (name, m) in &scene.materials {
                t.materials += 1;
                if let Err(err) = m {
                    t.bad_materials += 1;
                    note(
                        format!("material: {err}"),
                        format!("{}/{} [{name}]", kind.file_stem(), e.name),
                    );
                }
            }
        }
        if t.files == 0 {
            continue;
        }
        print_row(kind.file_stem(), &t);
        total.files += t.files;
        total.file_errors += t.file_errors;
        total.meshes += t.meshes;
        total.invalid += t.invalid;
        total.instances += t.instances;
        total.other_nodes += t.other_nodes;
        total.materials += t.materials;
        total.bad_materials += t.bad_materials;
        total.gi_meshes += t.gi_meshes;
        total.gi_points += t.gi_points;
        println!(
            "  {:<11} GI points in {} meshes ({} points)",
            "", t.gi_meshes, t.gi_points
        );
    }
    print_row("total", &total);
    let decodable = total.meshes + total.invalid;
    if decodable > 0 {
        println!(
            "\nstatic mesh records decoded: {}/{} ({:.2} %)",
            total.meshes,
            decodable,
            100.0 * total.meshes as f64 / decodable as f64
        );
    }
    if !reasons.is_empty() {
        println!("\nunsupported records by reason:");
        for (reason, (n, example)) in &reasons {
            println!("  {n:>5}  {reason}   e.g. {example}");
        }
    }
    Ok(())
}

fn print_row(label: &str, t: &PackTally) {
    println!(
        "{:<13} {:>6} {:>6} {:>8} {:>7} {:>6} {:>6} {:>9} {:>6}",
        label,
        t.files,
        t.file_errors,
        t.meshes,
        t.invalid,
        t.instances,
        t.other_nodes,
        t.materials,
        t.bad_materials
    );
}

/// Groups messages that differ only in numbers.
fn generalise(reason: &str) -> String {
    let mut out = String::with_capacity(reason.len());
    let mut in_number = false;
    for c in reason.chars() {
        if c.is_ascii_hexdigit() && (c.is_ascii_digit() || in_number) || (in_number && c == 'x') {
            if !in_number {
                out.push('N');
                in_number = true;
            }
        } else {
            in_number = false;
            out.push(c);
        }
    }
    out
}

/// `exa-tool corpus levels`: grids and props of every level file.
pub fn levels(game: &GameData) -> Result<()> {
    let Some(resource) = game.pack(exa_rpk::PackKind::Resource) else {
        return Ok(());
    };
    let objlib = game.pack(exa_rpk::PackKind::Objlib);
    let (mut levels, mut grids, mut props, mut resolved, mut errors) = (0, 0, 0, 0, 0);
    let mut environments = 0;
    let (mut placed, mut with_record, mut worn, mut worn_found) = (0, 0, 0, 0);
    let mut races: std::collections::BTreeMap<(u8, bool), usize> = Default::default();
    let mut missing: BTreeMap<String, usize> = BTreeMap::new();
    println!(
        "{:<18} {:>9} {:>8} {:>7} {:>9}",
        "level", "grid", "sets w/f", "props", "resolved"
    );
    for e in resource.entries() {
        let b = resource.bytes(e);
        if !matches!(FileKind::detect(b), FileKind::ChunkTree) {
            continue;
        }
        let Ok(scene) = Scene::parse(b) else { continue };
        if scene.grid.is_none() && scene.props.is_empty() {
            continue;
        }
        levels += 1;
        environments += usize::from(scene.environment.is_some());
        if let Some(chars) = &scene.characters {
            for p in &chars.placements {
                placed += 1;
                let Some(rec) = chars.record(p.id) else {
                    continue;
                };
                with_record += 1;
                *races
                    .entry((rec.race, p.condition == exa_formats::characters::DEAD))
                    .or_default() += 1;
                for &item in &rec.worn {
                    worn += 1;
                    let local = item & exa_formats::characters::LOCAL_ITEM != 0;
                    let id = item & exa_formats::characters::ID_MASK;
                    if local && scene.local_items.iter().any(|i| i.id == id) {
                        worn_found += 1;
                    }
                }
            }
        }
        errors += scene.level_errors.len();
        for err in &scene.level_errors {
            println!("  {}: {err}", e.name);
        }
        let here = scene
            .props
            .iter()
            .filter(|p| {
                let found = objlib.is_some_and(|o| o.find(&p.object).is_some());
                if !found {
                    *missing.entry(p.object.clone()).or_default() += 1;
                }
                found
            })
            .count();
        props += scene.props.len();
        resolved += here;
        let (grid, sets) = match &scene.grid {
            Some(g) => {
                grids += 1;
                (
                    format!("{}x{}", g.width, g.height),
                    format!("{}/{}", g.wall_sets.len(), g.floor_sets.len()),
                )
            }
            None => ("-".into(), "-".into()),
        };
        println!(
            "{:<18} {:>9} {:>8} {:>7} {:>9}",
            e.name,
            grid,
            sets,
            scene.props.len(),
            here
        );
    }
    println!(
        "\n{levels} levels, {grids} grids, {environments} environments, {props} props, {resolved} resolved in Objlib, {errors} section errors"
    );
    println!(
        "{placed} placed characters, {with_record} with a local record, {worn_found}/{worn} worn items found in local databases"
    );
    println!("placed characters by (race, dead): {races:?}");
    if !missing.is_empty() {
        println!("missing objects:");
        for (name, n) in missing.iter().take(20) {
            println!("  {n:>5}  {name}");
        }
    }
    Ok(())
}

/// `exa-tool corpus terrain`: every terrain and palette, and whether each
/// painted layer resolves in its palette.
pub fn terrain(game: &GameData) -> Result<()> {
    use exa_formats::terrain::{Palette, Terrain};
    let Some(resource) = game.pack(exa_rpk::PackKind::Resource) else {
        return Ok(());
    };
    let (mut palettes, mut palette_errors) = (0, 0);
    for e in resource.entries() {
        if matches!(FileKind::detect(resource.bytes(e)), FileKind::TerrainTable) {
            match Palette::parse(resource.bytes(e)) {
                Ok(_) => palettes += 1,
                Err(err) => {
                    palette_errors += 1;
                    println!("  {}: {err}", e.name);
                }
            }
        }
    }
    println!(
        "{:<20} {:<12} {:>5} {:>6} {:>7} {:>10}",
        "terrain", "palette", "grid", "tiles", "holes", "unresolved"
    );
    let (mut terrains, mut errors, mut tiles, mut unresolved) = (0, 0, 0, 0);
    for e in resource.entries() {
        let b = resource.bytes(e);
        if !matches!(FileKind::detect(b), FileKind::Terrain) {
            continue;
        }
        let t = match Terrain::parse(b) {
            Ok(t) => t,
            Err(err) => {
                errors += 1;
                println!("  {}: {err}", e.name);
                continue;
            }
        };
        terrains += 1;
        tiles += t.tiles.len();
        let palette = resource
            .get(&format!("{}.ftb", t.palette))
            .and_then(|b| Palette::parse(b).ok());
        let mut missing: Vec<&str> = t
            .tiles
            .iter()
            .flat_map(|tile| &tile.layers)
            .filter(|l| palette.as_ref().is_none_or(|p| p.entry(&l.name).is_none()))
            .map(|l| l.name.as_str())
            .collect();
        missing.sort_unstable();
        missing.dedup();
        unresolved += missing.len();
        println!(
            "{:<20} {:<12} {:>5} {:>6} {:>7} {:>10}  {}",
            e.name,
            if palette.is_some() {
                t.palette.clone()
            } else {
                format!("{}?", t.palette)
            },
            format!("{}x{}", t.width, t.height),
            t.tiles.len(),
            t.tiles.iter().filter(|t| t.flags & 0x200 != 0).count(),
            missing.len(),
            missing.join(", ")
        );
    }
    println!(
        "\n{terrains} terrains ({errors} errors), {tiles} tiles, {palettes} palettes ({palette_errors} errors), {unresolved} unresolved layer names"
    );
    Ok(())
}

/// `exa-tool corpus factories`: every weapon factory, plus two generated
/// examples checked against the original's choices.
pub fn factories(game: &GameData) -> Result<()> {
    use exa_formats::factory::Factory;
    let Some(pack) = game.pack(exa_rpk::PackKind::Factories) else {
        return Ok(());
    };
    let (mut ok, mut failed) = (0, 0);
    let mut versions = [0usize; 3];
    for e in pack.entries() {
        match Factory::parse(pack.bytes(e)) {
            Ok(f) => {
                ok += 1;
                versions[f.version as usize] += 1;
            }
            Err(err) => {
                failed += 1;
                println!("  {}: {err}", e.name);
            }
        }
    }
    println!(
        "{ok} factories parsed (v0 {}, v1 {}, v2 {}), {failed} failed",
        versions[0], versions[1], versions[2]
    );
    for (name, seed, quality) in [
        ("AxeTool2H_A01", 0x0D31_0870u32, 107u8),
        ("sword_lnga1", 0x1234_5678, 128),
    ] {
        let Some(f) = pack
            .get(name)
            .or_else(|| pack.get(&format!("{name}.fty")))
            .and_then(|b| Factory::parse(b).ok())
        else {
            println!("{name}: not found");
            continue;
        };
        let g = f.generate(seed, seed, quality, None);
        let chosen: Vec<String> = f
            .parts
            .iter()
            .zip(&g.selection)
            .filter(|(_, s)| **s > 0)
            .map(|(p, s)| format!("{} = {}", p.name, p.variants[*s as usize - 1].model))
            .collect();
        println!(
            "{name} seed {seed:#x} q {quality}: {:?} -> {}",
            g.selection,
            chosen.join(", ")
        );
        for placed in f.place(&g) {
            println!(
                "    {:<10} at {:?}",
                f.parts[placed.part].name,
                placed.position.map(|v| (v * 1000.0).round() / 1000.0)
            );
        }
    }
    Ok(())
}
