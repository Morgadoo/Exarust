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
                    NodeBody::Mesh(_) => t.meshes += 1,
                    NodeBody::Instance(_) => t.instances += 1,
                    NodeBody::NotMesh | NodeBody::Light(_) => t.other_nodes += 1,
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
        "\n{levels} levels, {grids} grids, {props} props, {resolved} resolved in Objlib, {errors} section errors"
    );
    if !missing.is_empty() {
        println!("missing objects:");
        for (name, n) in missing.iter().take(20) {
            println!("  {n:>5}  {name}");
        }
    }
    Ok(())
}
