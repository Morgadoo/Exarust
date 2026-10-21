//! `exa-tool corpus …`: decode every entry of a kind and report coverage;
//! `corpus images` checks the channel order of uncompressed images,
//! `corpus winding` which way the level geometry faces, `corpus tracks`
//! lists the nodes' keyframe tracks and `corpus render-words` tallies the
//! meshes' render words.

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
    voxel_lights: usize,
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
                    NodeBody::VoxelLight(_) => {
                        t.other_nodes += 1;
                        t.voxel_lights += 1;
                    }
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
        total.voxel_lights += t.voxel_lights;
        println!(
            "  {:<11} GI points in {} meshes ({} points), {} voxel lights",
            "", t.gi_meshes, t.gi_points, t.voxel_lights
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

/// `exa-tool corpus levels`: grids and props of every level file, and what
/// its placed humans wear.
pub fn levels(game: &GameData) -> Result<()> {
    let Some(resource) = game.pack(exa_rpk::PackKind::Resource) else {
        return Ok(());
    };
    let objlib = game.pack(exa_rpk::PackKind::Objlib);
    let characters = game.pack(exa_rpk::PackKind::Characters);
    let global = resource
        .get("objdb.rdb")
        .and_then(|b| exa_formats::rdb::Database::parse(b).ok())
        .map(|db| exa_formats::objdb::items(&db))
        .unwrap_or_default();
    // Placed humans, those covered at the groin, the items they wear and
    // those worn as a `Characters.rpk` model.
    let (mut humans, mut covered, mut human_worn, mut with_model) = (0, 0, 0, 0);
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
                // Race byte 0: humans.
                let (human, mut groin) = (rec.race == 0, false);
                for item in &rec.worn {
                    worn += 1;
                    let local = item.is_local();
                    let id = item.index();
                    if local && scene.local_items.iter().any(|i| i.id == id) {
                        worn_found += 1;
                    }
                    if !human {
                        continue;
                    }
                    human_worn += 1;
                    let items = if local { &scene.local_items } else { &global };
                    let Some(found) = items.iter().find(|i| i.id == id) else {
                        continue;
                    };
                    // Only an item drawn on the body covers it.
                    let model = &found.worn_model;
                    if !model.is_empty() && characters.is_some_and(|c| c.find(model).is_some()) {
                        with_model += 1;
                        groin |= found.covers_groin();
                    }
                }
                if human {
                    humans += 1;
                    covered += usize::from(groin);
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
    println!(
        "placed humans {humans}, covered at the groin {covered}; items they wear {human_worn}, with a Characters model {with_model}"
    );
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
        let built = f.build(&exa_formats::factory::WeaponSpec {
            seeds: [seed, seed],
            quality,
            ..Default::default()
        });
        let g = &built.generated;
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
        let boxes = vec![exa_formats::factory::ComponentBox::default(); f.parts.len()];
        for placed in f.place(g, &boxes) {
            println!(
                "    {:<10} at {:?}",
                f.parts[placed.part].name,
                placed.position.map(|v| (v * 1000.0).round() / 1000.0)
            );
        }
    }
    Ok(())
}

// `exa-tool corpus images` --------------------------------------------------

/// Share of an RGB8 normal map's texels whose blue (z) must be its largest
/// channel. Shipped maps reach 0.81 or more read red first and 0.13 or less
/// with red and blue swapped.
const Z_IN_BLUE: f64 = 0.5;

/// `exa-tool corpus images`: every image in every pack, tallied by format
/// word and storage. Uncompressed images are decoded; uncompressed RGB8
/// normal maps (names ending in `n`) must hold z in blue, which pins the
/// channel order of uncompressed images.
pub fn images(game: &GameData) -> Result<()> {
    use exa_formats::rfi::{PixelFormat, Rfi};
    #[derive(Default)]
    struct Tally {
        raw: usize,
        packed: usize,
        failed: usize,
    }
    let mut words: BTreeMap<u32, Tally> = BTreeMap::new();
    let (mut normals, mut sideways) = (0, 0);
    let mut visit = |name: &str, b: &[u8]| {
        let Ok(rfi) = Rfi::parse(b) else { return };
        let tally = words.entry(rfi.header.format_word).or_default();
        if rfi.is_packed() {
            tally.packed += 1;
        } else {
            tally.raw += 1;
        }
        if rfi.format.bytes_per_pixel().is_none() {
            return;
        }
        let rgba = match rfi.decode_rgba8() {
            Ok(rgba) => rgba,
            Err(err) => {
                tally.failed += 1;
                println!("  {name}: {err}");
                return;
            }
        };
        let stem = name.to_ascii_lowercase();
        if rfi.format == PixelFormat::Rgb8 && stem.trim_end_matches(".rfi").ends_with('n') {
            normals += 1;
            let texels = rgba.as_chunks::<4>().0;
            let blue = texels
                .iter()
                .filter(|p| p[2] >= p[0] && p[2] >= p[1])
                .count();
            let share = blue as f64 / texels.len().max(1) as f64;
            if share < Z_IN_BLUE {
                sideways += 1;
                println!("  {name}: z in blue on {:.0}% of texels", share * 100.0);
            }
        }
    };
    for (kind, pack) in game.packs() {
        if kind == exa_rpk::PackKind::Sound {
            continue;
        }
        for e in pack.entries() {
            let b = pack.bytes(e);
            match FileKind::detect(b) {
                FileKind::Image => visit(&e.name, b),
                FileKind::Pack => {
                    if let Ok(inner) = exa_rpk::Archive::parse(b) {
                        for ie in inner.entries() {
                            visit(&format!("{}/{}", e.name, ie.name), inner.bytes(ie));
                        }
                    }
                }
                _ => {}
            }
        }
    }
    println!(
        "{:<12} {:<9} {:>6} {:>6} {:>6}",
        "word", "format", "raw", "packed", "failed"
    );
    for (word, t) in &words {
        println!(
            "{word:#010x}   {:<9} {:>6} {:>6} {:>6}",
            format!("{:?}", PixelFormat::from_word(*word)),
            t.raw,
            t.packed,
            t.failed
        );
    }
    println!("\n{normals} RGB8 normal maps, {sideways} without z in blue");
    Ok(())
}

// `exa-tool corpus winding` -------------------------------------------------

/// A face is near-horizontal when its unit front normal's |y| exceeds this.
const HORIZONTAL: f32 = 0.7;
/// Faces centred below this height (metres, the floor at y = 0) are at
/// floor height, within reach of a body that falls; above it, overhead.
const OVERHEAD: f32 = 2.0;
/// A face this far below a downward face's centre still counts as above it
/// (a two-sided sheet's other side), metres.
const TWIN: f32 = 0.005;

type V3 = [f32; 3];

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Unit front normal (zero when degenerate) and area of a triangle.
fn normal(t: &[V3; 3]) -> (V3, f32) {
    let n = cross(sub(t[1], t[0]), sub(t[2], t[0]));
    let len = dot(n, n).sqrt();
    if len > 1e-9 {
        (n.map(|v| v / len), len / 2.0)
    } else {
        ([0.0; 3], 0.0)
    }
}

/// A node transform: the images of the three axes, then the translation
/// (the stored row-vector 3×4, read as columns, as `transform_from_rows`).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Affine {
    axes: [V3; 3],
    t: V3,
}

impl Affine {
    const IDENTITY: Self = Self {
        axes: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        t: [0.0; 3],
    };

    fn from_rows(r: &[[f32; 3]; 4]) -> Self {
        Self {
            axes: [r[0], r[1], r[2]],
            t: r[3],
        }
    }

    fn linear(&self, v: V3) -> V3 {
        let [x, y, z] = self.axes;
        std::array::from_fn(|i| x[i] * v[0] + y[i] * v[1] + z[i] * v[2])
    }

    fn point(&self, p: V3) -> V3 {
        let l = self.linear(p);
        [l[0] + self.t[0], l[1] + self.t[1], l[2] + self.t[2]]
    }

    /// `self` applied after `child`.
    fn then(&self, child: &Affine) -> Affine {
        Affine {
            axes: child.axes.map(|a| self.linear(a)),
            t: self.point(child.t),
        }
    }

    fn det(&self) -> f32 {
        dot(self.axes[0], cross(self.axes[1], self.axes[2]))
    }

    fn inverse(&self) -> Option<Affine> {
        let d = self.det();
        if !d.is_normal() {
            return None;
        }
        let [x, y, z] = self.axes;
        // Rows of the inverse.
        let rows = [cross(y, z), cross(z, x), cross(x, y)].map(|r| r.map(|v| v / d));
        let axes = std::array::from_fn(|j| [rows[0][j], rows[1][j], rows[2][j]]);
        let t = rows.map(|r| -dot(r, self.t));
        Some(Affine { axes, t })
    }
}

/// Global transforms of `n` linked nodes: `root(i)` for a node without a
/// parent, the parent's global after `local(i)` otherwise. A parent chain
/// that loops is cut where it loops.
fn globals(
    n: usize,
    parent: impl Fn(usize) -> Option<usize>,
    local: impl Fn(usize) -> Affine,
    root: impl Fn(usize) -> Affine,
) -> Vec<Affine> {
    let mut out: Vec<Option<Affine>> = vec![None; n];
    let mut chain = Vec::new();
    for start in 0..n {
        chain.clear();
        let mut at = Some(start);
        while let Some(i) = at {
            if out[i].is_some() || chain.contains(&i) {
                break;
            }
            chain.push(i);
            at = parent(i).filter(|&p| p < n);
        }
        let mut above = at.and_then(|i| out[i]);
        for &i in chain.iter().rev() {
            let g = match above {
                Some(p) => p.then(&local(i)),
                None => root(i),
            };
            out[i] = Some(g);
            above = Some(g);
        }
    }
    out.into_iter()
        .map(|g| g.unwrap_or(Affine::IDENTITY))
        .collect()
}

/// A chunk tree reduced to what Exarust builds level colliders from.
struct Shape {
    nodes: Vec<ShapeNode>,
}

struct ShapeNode {
    name: String,
    parent: Option<usize>,
    local: Affine,
    /// Triangles of the drawn material groups, in the node's frame.
    tris: Vec<[V3; 3]>,
    /// Triangles of `_null` groups, which are neither drawn nor built.
    null: Vec<[V3; 3]>,
    /// The node reuses another node's data; Exarust builds nothing for it.
    instance: bool,
}

impl Shape {
    fn of(scene: &Scene) -> Self {
        let len = scene.nodes.len();
        let nodes = scene
            .nodes
            .iter()
            .map(|node| {
                let (mut tris, mut null) = (Vec::new(), Vec::new());
                if let NodeBody::Mesh(m) = &node.body {
                    for g in &m.groups {
                        let start = g.first_index as usize;
                        let end = start.saturating_add(g.index_count as usize);
                        let Some(indices) = m.indices.get(start..end) else {
                            continue;
                        };
                        let into = if g.material.eq_ignore_ascii_case("_null") {
                            &mut null
                        } else {
                            &mut tris
                        };
                        for t in indices.as_chunks::<3>().0 {
                            let corner = |i: u32| m.positions.get(i as usize).copied();
                            if let [Some(a), Some(b), Some(c)] = t.map(corner) {
                                into.push([a, b, c]);
                            }
                        }
                    }
                }
                ShapeNode {
                    name: node.name.clone(),
                    parent: node.parent.filter(|&p| p >= 1 && p <= len).map(|p| p - 1),
                    local: Affine::from_rows(&node.transform),
                    tris,
                    null,
                    instance: matches!(node.body, NodeBody::Instance(_)),
                }
            })
            .collect();
        Self { nodes }
    }

    fn children(&self) -> Vec<Vec<usize>> {
        let mut kids = vec![Vec::new(); self.nodes.len()];
        for (i, n) in self.nodes.iter().enumerate() {
            if let Some(p) = n.parent {
                kids[p].push(i);
            }
        }
        kids
    }

    /// Global transforms as `exa_world::spawn_model` places the model:
    /// without a placement every node keeps its stored transform; with one,
    /// the placement stands in for the first root's transform.
    fn placed(&self, placement: Option<&Affine>) -> Vec<Affine> {
        let first = self.nodes.iter().position(|n| n.parent.is_none());
        let first_inverse = first
            .and_then(|f| self.nodes[f].local.inverse())
            .unwrap_or(Affine::IDENTITY);
        globals(
            self.nodes.len(),
            |i| self.nodes[i].parent,
            |i| self.nodes[i].local,
            |i| match placement {
                None => self.nodes[i].local,
                Some(p) if Some(i) == first => *p,
                Some(p) => p.then(&first_inverse.then(&self.nodes[i].local)),
            },
        )
    }
}

/// What a triangle is to a body coming down on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Facing {
    /// Not near-horizontal, or degenerate.
    Steep,
    /// Front up: holds a body that lands on it.
    Up,
    /// Front down, and the first face of its object straight above it faces
    /// up (is met from behind): the bottom of a solid (a wall, a slab, a
    /// rock) or the lower side of a two-sided sheet, never met from above.
    Under,
    /// Front down with nothing of its object above, or a downward face
    /// first: a ceiling or an overhang if a body can stand under it; a floor
    /// wound the wrong way if a body comes down on it (it meets the back and
    /// passes through).
    Open,
}

/// A judged triangle: its facing, area (m²) and whether its centre is at
/// floor height (below [`OVERHEAD`]).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Judged {
    facing: Facing,
    area: f32,
    low: bool,
}

/// Judges triangles in the frame they are given in (Y up, the floor at
/// y = 0). Triangles not marked in `solid` are judged but hide nothing.
fn judge(tris: &[[V3; 3]], solid: &[bool]) -> Vec<Judged> {
    let normals: Vec<(V3, f32)> = tris.iter().map(normal).collect();
    let grid = FaceGrid::new(tris, &normals, solid);
    tris.iter()
        .zip(&normals)
        .enumerate()
        .map(|(i, (t, &(n, area)))| {
            let c: V3 = std::array::from_fn(|k| (t[0][k] + t[1][k] + t[2][k]) / 3.0);
            let facing = if n[1] > HORIZONTAL {
                Facing::Up
            } else if n[1] < -HORIZONTAL {
                match grid.first_above(tris, &normals, i, c) {
                    Some(j) if normals[j].0[1] > 0.0 => Facing::Under,
                    _ => Facing::Open,
                }
            } else {
                Facing::Steep
            };
            Judged {
                facing,
                area,
                low: c[1] < OVERHEAD,
            }
        })
        .collect()
}

/// The non-vertical faces of one object, bucketed on a grid over x and z.
struct FaceGrid {
    min: [f32; 2],
    cell: f32,
    size: [usize; 2],
    cells: Vec<Vec<u32>>,
}

impl FaceGrid {
    const MAX_CELLS: usize = 512;
    const MIN_CELL: f32 = 0.25;

    fn new(tris: &[[V3; 3]], normals: &[(V3, f32)], solid: &[bool]) -> Self {
        let faces: Vec<usize> = (0..tris.len())
            .filter(|&i| normals[i].0[1].abs() > 0.05 && solid.get(i).copied().unwrap_or(false))
            .collect();
        let (mut min, mut max) = ([f32::MAX; 2], [f32::MIN; 2]);
        for &i in &faces {
            for p in &tris[i] {
                for (k, v) in [p[0], p[2]].into_iter().enumerate() {
                    min[k] = min[k].min(v);
                    max[k] = max[k].max(v);
                }
            }
        }
        let extent = (max[0] - min[0]).max(max[1] - min[1]);
        let cell = (extent / Self::MAX_CELLS as f32).max(Self::MIN_CELL);
        let mut grid = Self {
            min,
            cell,
            size: [0, 0],
            cells: Vec::new(),
        };
        if faces.is_empty() || !cell.is_finite() {
            return grid;
        }
        grid.size = [0, 1].map(|k| grid.index(k, max[k]) + 1);
        grid.cells = vec![Vec::new(); grid.size[0] * grid.size[1]];
        for &i in &faces {
            let t = &tris[i];
            let lo =
                [0, 1].map(|k| grid.index(k, t.iter().map(|p| p[k * 2]).fold(f32::MAX, f32::min)));
            let hi =
                [0, 1].map(|k| grid.index(k, t.iter().map(|p| p[k * 2]).fold(f32::MIN, f32::max)));
            for z in lo[1]..=hi[1] {
                for x in lo[0]..=hi[0] {
                    grid.cells[z * grid.size[0] + x].push(i as u32);
                }
            }
        }
        grid
    }

    /// Cell index along x (`k` 0) or z (`k` 1), clamped to the grid.
    fn index(&self, k: usize, v: f32) -> usize {
        let i = ((v - self.min[k]) / self.cell).max(0.0) as usize;
        match self.size[k] {
            0 => i.min(Self::MAX_CELLS),
            size => i.min(size - 1),
        }
    }

    /// The nearest face other than `own` straight above `c`, from [`TWIN`]
    /// below it up; an upward face wins a tie.
    fn first_above(
        &self,
        tris: &[[V3; 3]],
        normals: &[(V3, f32)],
        own: usize,
        c: V3,
    ) -> Option<usize> {
        let (x, z) = (c[0], c[2]);
        if self.cells.is_empty() || x < self.min[0] || z < self.min[1] {
            return None;
        }
        let cell = self.index(1, z) * self.size[0] + self.index(0, x);
        let mut best: Option<(f32, bool, usize)> = None;
        for &j in &self.cells[cell] {
            let j = j as usize;
            let (t, n) = (&tris[j], normals[j].0);
            if j == own || !inside_xz(t, x, z) {
                continue;
            }
            let y = t[0][1] - (n[0] * (x - t[0][0]) + n[2] * (z - t[0][2])) / n[1];
            let rise = y - c[1];
            if rise.is_nan() || rise < -TWIN {
                continue;
            }
            let key = (rise.max(0.0), n[1] <= 0.0);
            if best.is_none_or(|b| key < (b.0, b.1)) {
                best = Some((key.0, key.1, j));
            }
        }
        best.map(|(_, _, j)| j)
    }
}

/// Whether (x, z) lies in the triangle's projection on the ground plane.
fn inside_xz(t: &[V3; 3], x: f32, z: f32) -> bool {
    let edge = |a: V3, b: V3| (b[0] - a[0]) * (z - a[2]) - (b[2] - a[2]) * (x - a[0]);
    let d = [edge(t[0], t[1]), edge(t[1], t[2]), edge(t[2], t[0])];
    const EPS: f32 = 1e-6;
    d.iter().all(|&v| v >= -EPS) || d.iter().all(|&v| v <= EPS)
}

/// Facing counts of a group of collider triangles.
#[derive(Debug, Default, Clone, Copy)]
struct Facings {
    /// Mesh nodes built, and those placed mirrored (negative determinant).
    nodes: usize,
    mirrored: usize,
    /// Near-horizontal triangles, their area, and those facing up.
    horizontal: usize,
    horizontal_area: f64,
    up: usize,
    up_area: f64,
    /// Near-horizontal triangles at floor height, and those facing up.
    low: usize,
    low_up: usize,
    /// Downward triangles: undersides, open ones at floor height (and
    /// their area), open ones overhead.
    under: usize,
    open_low: usize,
    open_low_area: f64,
    open_high: usize,
    /// `_null` triangles (not built), near-horizontal ones, facing up.
    null_tris: usize,
    null_horizontal: usize,
    null_up: usize,
}

impl Facings {
    fn count(&mut self, j: &Judged) {
        if j.facing == Facing::Steep {
            return;
        }
        let area = f64::from(j.area);
        self.horizontal += 1;
        self.horizontal_area += area;
        self.low += usize::from(j.low);
        match (j.facing, j.low) {
            (Facing::Up, low) => {
                self.up += 1;
                self.up_area += area;
                self.low_up += usize::from(low);
            }
            (Facing::Under, _) => self.under += 1,
            (Facing::Open, true) => {
                self.open_low += 1;
                self.open_low_area += area;
            }
            (Facing::Open, false) => self.open_high += 1,
            (Facing::Steep, _) => {}
        }
    }

    fn add(&mut self, o: &Facings) {
        self.nodes += o.nodes;
        self.mirrored += o.mirrored;
        self.horizontal += o.horizontal;
        self.horizontal_area += o.horizontal_area;
        self.up += o.up;
        self.up_area += o.up_area;
        self.low += o.low;
        self.low_up += o.low_up;
        self.under += o.under;
        self.open_low += o.open_low;
        self.open_low_area += o.open_low_area;
        self.open_high += o.open_high;
        self.null_tris += o.null_tris;
        self.null_horizontal += o.null_horizontal;
        self.null_up += o.null_up;
    }

    /// Node-level counts of a placed node: built, mirrored, and which way
    /// its `_null` triangles face.
    fn count_node(&mut self, node: &ShapeNode, global: &Affine) {
        let mirrored = global.det() < 0.0;
        if !node.tris.is_empty() {
            self.nodes += 1;
            self.mirrored += usize::from(mirrored);
        }
        self.null_tris += node.null.len();
        for t in &node.null {
            let [a, b, c] = t.map(|p| global.point(p));
            let (n, _) = normal(&if mirrored { [a, c, b] } else { [a, b, c] });
            if n[1].abs() > HORIZONTAL {
                self.null_horizontal += 1;
                self.null_up += usize::from(n[1] > 0.0);
            }
        }
    }

    fn up_share(&self) -> f64 {
        percent(self.up as f64, self.horizontal as f64)
    }
}

fn percent(part: f64, whole: f64) -> f64 {
    if whole > 0.0 {
        100.0 * part / whole
    } else {
        0.0
    }
}

fn facings_header(first: &str) {
    println!(
        "{first:<28} {:>6} {:>8} {:>6} {:>6} {:>8} {:>6} {:>7} {:>7} {:>7} {:>4} {:>6}",
        "nodes",
        "horiz",
        "up%",
        "upA%",
        "<2m",
        "up%",
        "under",
        "open<2m",
        "open>2m",
        "mirr",
        "_null"
    );
}

fn facings_row(label: &str, f: &Facings) {
    println!(
        "{label:<28} {:>6} {:>8} {:>6.2} {:>6.2} {:>8} {:>6.2} {:>7} {:>7} {:>7} {:>4} {:>6}",
        f.nodes,
        f.horizontal,
        f.up_share(),
        percent(f.up_area, f.horizontal_area),
        f.low,
        percent(f.low_up as f64, f.low as f64),
        f.under,
        f.open_low,
        f.open_high,
        f.mirrored,
        f.null_tris
    );
}

/// Grid-piece roles: the floor (floor-set pieces and the `+F…` strips of
/// wall pieces), walls, corners and special tiles, and ceilings (`+R…`,
/// built only with `--ceilings`).
const PIECE_ROLES: [&str; 3] = ["floor", "wall", "ceiling"];
const FLOOR: usize = 0;
const WALL: usize = 1;
const CEILING: usize = 2;

/// The role of a set's node that `exa_world::grid` indexes as a piece:
/// `+F…` in floor sets; `+W?`, `+C?`, `+R?` and `+T?` of the kinds it
/// places in wall sets.
fn piece_role(floor_set: bool, name: &str) -> Option<usize> {
    match (floor_set, name.as_bytes()) {
        (true, [b'+', b'F', ..]) => Some(FLOOR),
        (false, [b'+', b'R', b'M' | b'L' | b'R' | b'B' | b'C', ..]) => Some(CEILING),
        (false, [b'+', b'W', b'C' | b'D' | b'L' | b'R', ..])
        | (false, [b'+', b'C', b'L' | b'R' | b'I' | b'O', ..])
        | (
            false,
            [
                b'+',
                b'T',
                b'A' | b'C' | b'E' | b'N' | b'O' | b'P' | b'R' | b'W',
                ..,
            ],
        ) => Some(WALL),
        _ => None,
    }
}

/// Collider triangles of one placed object, judged together.
#[derive(Default)]
struct Object {
    tris: Vec<[V3; 3]>,
    /// Per triangle: the node it came from and its role.
    tags: Vec<(usize, usize)>,
}

impl Object {
    /// Adds a node's triangles placed by `global`, front sides kept where
    /// the node's own winding puts them (`exa_physics` reverses the winding
    /// of a mirrored placement).
    fn add(&mut self, node: &ShapeNode, index: usize, role: usize, global: &Affine) {
        let mirrored = global.det() < 0.0;
        for t in &node.tris {
            let [a, b, c] = t.map(|p| global.point(p));
            self.tris.push(if mirrored { [a, c, b] } else { [a, b, c] });
            self.tags.push((index, role));
        }
    }

    /// Judges the object (ceilings, built only with `--ceilings`, hide
    /// nothing) and hands each node's counts per role to `sink`.
    fn judge(&self, mut sink: impl FnMut(usize, usize, &Facings)) {
        let solid: Vec<bool> = self.tags.iter().map(|&(_, r)| r != CEILING).collect();
        let mut by_node: BTreeMap<(usize, usize), Facings> = BTreeMap::new();
        for (j, &tag) in judge(&self.tris, &solid).iter().zip(&self.tags) {
            by_node.entry(tag).or_default().count(j);
        }
        for ((node, role), f) in by_node {
            sink(node, role, &f);
        }
    }
}

/// Facing counts of one source (a node of a piece, level or model), summed
/// over its placements.
#[derive(Default)]
struct Source {
    uses: usize,
    facings: Facings,
}

type Sources = BTreeMap<String, Source>;

fn note(sources: &mut Sources, label: String, f: &Facings) {
    let s = sources.entry(label).or_default();
    s.uses += 1;
    s.facings.add(f);
}

/// Prints the sources with the most of `amount`, largest first.
fn worst(title: &str, sources: &Sources, amount: impl Fn(&Facings) -> f64) {
    println!("\n{title}");
    let mut list: Vec<_> = sources
        .iter()
        .filter(|(_, s)| amount(&s.facings) > 0.0)
        .collect();
    list.sort_by(|a, b| amount(&b.1.facings).total_cmp(&amount(&a.1.facings)));
    for (label, s) in list.iter().take(12) {
        let f = &s.facings;
        println!(
            "  {:>6} down ({:>4} under, {:>5} open<2m {:>8.2} m², {:>4} open>2m) of {:>6} horiz, x{:<4} {label}",
            f.horizontal - f.up,
            f.under,
            f.open_low,
            f.open_low_area,
            f.open_high,
            f.horizontal,
            s.uses
        );
    }
    if list.len() > 12 {
        println!("  … {} more", list.len() - 12);
    }
}

/// `exa-tool corpus winding`: which way the near-horizontal faces of the
/// level geometry point, as Exarust builds its level colliders (faces push
/// from their front only): each level's own meshes and its props in the
/// world, and the grid sets' pieces in their own frame.
pub fn winding(game: &GameData) -> Result<()> {
    use exa_rpk::PackKind;
    let Some(resource) = game.pack(PackKind::Resource) else {
        return Ok(());
    };
    let objlib = game.pack(PackKind::Objlib);
    let (mut level_sources, mut prop_sources, mut piece_sources, mut floor_sources) = (
        Sources::new(),
        Sources::new(),
        Sources::new(),
        Sources::new(),
    );
    // Set entry → (is a floor set, levels using it).
    let mut sets: BTreeMap<String, (bool, Vec<String>)> = BTreeMap::new();
    let mut models: BTreeMap<String, Option<Shape>> = BTreeMap::new();
    let (mut own_total, mut prop_total) = (Facings::default(), Facings::default());
    let (mut instances, mut placements, mut mirrored_placements, mut missing) = (0, 0, 0, 0);

    println!(
        "horiz: triangles with |n.y| > {HORIZONTAL} (front normal); up%, upA%: facing up by count, \
         by area; <2m: those centred below {OVERHEAD} m, and their up%; under: facing down with \
         the first face of the object above facing up (the bottom of a solid); open<2m, open>2m: \
         the other downward ones; mirr: nodes placed mirrored; _null: `_null` triangles (not built)\n"
    );
    println!("level meshes (world, floor at y = 0)");
    facings_header("level");
    let mut prop_rows = Vec::new();
    for e in resource.entries() {
        let b = resource.bytes(e);
        if !matches!(FileKind::detect(b), FileKind::ChunkTree) {
            continue;
        }
        let Ok(scene) = Scene::parse(b) else { continue };
        if scene.grid.is_none() && scene.props.is_empty() {
            continue;
        }
        if let Some(g) = &scene.grid {
            for (names, floor, fallback) in [
                (&g.wall_sets, false, "gews_uwa01.rfc"),
                (&g.floor_sets, true, "gefs_uwa01.rfc"),
            ] {
                for n in names {
                    let mut entry = format!("{}.rfc", n.to_ascii_lowercase());
                    if resource.find(&entry).is_none() {
                        entry = fallback.into();
                    }
                    let users = &mut sets.entry(entry).or_insert((floor, Vec::new())).1;
                    if !users.contains(&e.name) {
                        users.push(e.name.clone());
                    }
                }
            }
        }

        // The level's own nodes, as `spawn_level` spawns them.
        let own = Shape::of(&scene);
        instances += own.nodes.iter().filter(|n| n.instance).count();
        let placed = own.placed(None);
        let mut object = Object::default();
        let mut row = Facings::default();
        for (i, node) in own.nodes.iter().enumerate() {
            object.add(node, i, WALL, &placed[i]);
            row.count_node(node, &placed[i]);
        }
        object.judge(|i, _, f| {
            row.add(f);
            note(
                &mut level_sources,
                format!("{} [{}]", e.name, own.nodes[i].name),
                f,
            );
        });
        facings_row(&e.name, &row);
        own_total.add(&row);

        // Props: an Objlib model under each placement, children placed in
        // their parent prop's frame.
        let n = scene.props.len();
        let prop_globals = globals(
            n,
            |i| {
                scene.props[i]
                    .parent
                    .filter(|&p| p >= 1 && p <= n && p - 1 != i)
                    .map(|p| p - 1)
            },
            |i| Affine::from_rows(&scene.props[i].transform),
            |i| Affine::from_rows(&scene.props[i].transform),
        );
        let mut props = Facings::default();
        for (prop, placement) in scene.props.iter().zip(&prop_globals) {
            let key = prop.object.to_ascii_lowercase();
            let shape = models.entry(key.clone()).or_insert_with(|| {
                let bytes = objlib.and_then(|o| o.get(&prop.object))?;
                Scene::parse(bytes).ok().map(|s| Shape::of(&s))
            });
            let Some(shape) = shape else {
                missing += 1;
                continue;
            };
            placements += 1;
            mirrored_placements += usize::from(placement.det() < 0.0);
            instances += shape.nodes.iter().filter(|n| n.instance).count();
            let placed = shape.placed(Some(placement));
            let mut object = Object::default();
            for (i, node) in shape.nodes.iter().enumerate() {
                object.add(node, i, WALL, &placed[i]);
                props.count_node(node, &placed[i]);
            }
            object.judge(|i, _, f| {
                props.add(f);
                note(
                    &mut prop_sources,
                    format!("Objlib/{key} [{}]", shape.nodes[i].name),
                    f,
                );
            });
        }
        prop_total.add(&props);
        prop_rows.push((format!("{} ({n})", e.name), props));
    }
    facings_row("total", &own_total);

    println!("\nprops (world, floor at y = 0; label: level (props))");
    facings_header("level");
    for (label, f) in &prop_rows {
        facings_row(label, f);
    }
    facings_row("total", &prop_total);

    // Grid pieces in their own frame (a side turns them about +Y only), as
    // `exa_world::grid` places them: the piece root's mesh, its children
    // except `[…` occluders, `+F…` strips as floor and `+R…` as ceiling.
    println!("\ngrid-set pieces (piece frame, floor at y = 0; label: set (levels) role)");
    facings_header("set");
    let mut piece_total = [Facings::default(); 3];
    let mut piece_count = 0;
    for (entry, (floor_set, users)) in &sets {
        let Some(shape) = resource
            .get(entry)
            .and_then(|b| Scene::parse(b).ok())
            .map(|s| Shape::of(&s))
        else {
            println!("{entry}: not decoded");
            continue;
        };
        let kids = shape.children();
        let mut roles = [Facings::default(); 3];
        for (root, node) in shape.nodes.iter().enumerate() {
            let Some(role) = piece_role(*floor_set, &node.name).filter(|_| node.parent.is_none())
            else {
                continue;
            };
            piece_count += 1;
            let mut object = Object::default();
            let mut stack = vec![(root, Affine::IDENTITY, role)];
            while let Some((i, frame, role)) = stack.pop() {
                let n = &shape.nodes[i];
                object.add(n, i, role, &frame);
                roles[role].count_node(n, &frame);
                for &k in &kids[i] {
                    let child = &shape.nodes[k];
                    let role = match (i == root, child.name.as_bytes()) {
                        (true, [b'[', ..]) => continue,
                        (true, [b'+', b'R', ..]) => CEILING,
                        (true, [b'+', b'F', ..]) => FLOOR,
                        _ => role,
                    };
                    stack.push((k, frame.then(&child.local), role));
                }
            }
            object.judge(|i, role, f| {
                roles[role].add(f);
                let label = format!("{entry} {} [{}]", node.name, shape.nodes[i].name);
                if role == FLOOR {
                    note(&mut floor_sources, label.clone(), f);
                }
                if role != CEILING {
                    note(&mut piece_sources, label, f);
                }
            });
        }
        let stem = entry.trim_end_matches(".rfc");
        for (role, f) in roles.iter().enumerate() {
            if f.nodes > 0 {
                facings_row(
                    &format!("{stem} ({}) {}", users.len(), PIECE_ROLES[role]),
                    f,
                );
            }
            piece_total[role].add(f);
        }
    }

    println!("\noverall: near-horizontal collider triangles (|n.y| > {HORIZONTAL})");
    facings_header("");
    for (role, f) in piece_total.iter().enumerate() {
        facings_row(&format!("grid pieces, {}", PIECE_ROLES[role]), f);
    }
    facings_row("level meshes", &own_total);
    facings_row("props", &prop_total);
    let mut built = Facings::default();
    for f in [
        &piece_total[FLOOR],
        &piece_total[WALL],
        &own_total,
        &prop_total,
    ] {
        built.add(f);
    }
    facings_row("all but ceilings", &built);
    let mut all = built;
    all.add(&piece_total[CEILING]);
    println!(
        "`_null` triangles (not built): {}, {} near-horizontal, {:.2} % of those up",
        all.null_tris,
        all.null_horizontal,
        percent(all.null_up as f64, all.null_horizontal as f64)
    );
    println!(
        "{} grid sets ({piece_count} pieces), {placements} prop placements ({mirrored_placements} mirrored, {missing} missing), {instances} instance nodes not built",
        sets.len()
    );

    worst(
        "grid floors with downward faces (floor sets and `+F…` strips)",
        &floor_sources,
        |f| (f.horizontal - f.up) as f64,
    );
    let open_area = |f: &Facings| f.open_low_area;
    worst(
        "grid pieces: most open downward area at floor height",
        &piece_sources,
        open_area,
    );
    worst(
        "level meshes: most open downward area at floor height",
        &level_sources,
        open_area,
    );
    worst(
        "props (summed over placements): most open downward area at floor height",
        &prop_sources,
        open_area,
    );
    Ok(())
}

/// Every keyframe track of every chunk tree, the packs nested in a pack
/// included: the file, the node and its parent, the play mode, the
/// duration and the keys (seconds, cm).
pub fn tracks(game: &GameData) -> Result<()> {
    let is_tree = |b: &[u8]| {
        matches!(
            FileKind::detect(b),
            FileKind::ChunkTree | FileKind::ChunkTreeAlt
        )
    };
    let mut trees: Vec<(String, &[u8])> = Vec::new();
    for (kind, pack) in game.packs() {
        for e in pack.entries() {
            let b = pack.bytes(e);
            if is_tree(b) {
                trees.push((format!("{}/{}", kind.file_stem(), e.name), b));
            } else if FileKind::detect(b) == FileKind::Pack {
                let Ok(inner) = exa_rpk::Archive::parse(b) else {
                    continue;
                };
                for f in inner.entries() {
                    let b = inner.bytes(f);
                    if is_tree(b) {
                        trees.push((format!("{}/{}/{}", kind.file_stem(), e.name, f.name), b));
                    }
                }
            }
        }
    }
    let (mut files, mut found, mut bad) = (0, 0, 0);
    for (name, b) in &trees {
        let Ok(scene) = Scene::parse(b) else {
            continue;
        };
        files += 1;
        for node in &scene.nodes {
            for track in &node.key_tracks {
                let track = match track {
                    Ok(t) => t,
                    Err(e) => {
                        bad += 1;
                        println!("{name} [{}]: undecodable track: {e}", node.name);
                        continue;
                    }
                };
                found += 1;
                let parent = node
                    .parent
                    .and_then(|p| scene.nodes.get(p - 1))
                    .map_or("(root)", |p| p.name.as_str());
                println!(
                    "{name} [{}] under {parent}: {:?}, {} s, {} keys",
                    node.name,
                    track.mode,
                    track.duration,
                    track.keys.len()
                );
                for k in &track.keys {
                    let cm = k.position.map(|v| v * 100.0);
                    println!(
                        "  {:>7.3} s  ({:.2}, {:.2}, {:.2}) cm",
                        k.time, cm[0], cm[1], cm[2]
                    );
                }
            }
        }
    }
    println!("{files} chunk trees, {found} keyframe tracks, {bad} undecodable");
    Ok(())
}

/// One row of `corpus render-words`: one (level, bit 19) pair.
#[derive(Default)]
struct WordRow {
    meshes: usize,
    /// With the cull-front bit (0x800).
    cull_front: usize,
    /// With the keep-own-light bit (0x1000).
    own_light: usize,
    examples: Vec<String>,
}

/// `corpus render-words`: every mesh's render word (the word before its
/// streams, the original's node render flags), tallied by shadow-bias
/// level (bits 16..18) and the 0x80000 variant, with the cull-front
/// (0x800) and keep-own-light (0x1000) bits counted; then every node whose
/// name starts with `uw01_floor`, the broken floors of the dungeon levels.
pub fn render_words(game: &GameData) -> Result<()> {
    let mut tally: BTreeMap<(u32, bool), WordRow> = BTreeMap::new();
    let mut floors = Vec::new();
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
                let NodeBody::Mesh(m) = &node.body else {
                    continue;
                };
                let word = m.render_flags;
                let level = (word >> 16) & 7;
                let row = tally.entry((level, word & 0x80000 != 0)).or_default();
                row.meshes += 1;
                row.cull_front += usize::from(word & 0x800 != 0);
                row.own_light += usize::from(word & 0x1000 != 0);
                if row.examples.len() < 6 {
                    row.examples
                        .push(format!("{}/{}:{}", kind.file_stem(), e.name, node.name));
                }
                if node.name.to_ascii_lowercase().starts_with("uw01_floor") {
                    floors.push(format!(
                        "{}/{}:{} {word:#010x}",
                        kind.file_stem(),
                        e.name,
                        node.name
                    ));
                }
            }
        }
    }
    println!(
        "{:>5} {:>7} {:>8} {:>6} {:>6}  examples",
        "level", "0x80000", "meshes", "0x800", "0x1000"
    );
    for ((level, variant), row) in &tally {
        println!(
            "{level:>5} {variant:>7} {:>8} {:>6} {:>6}  {}",
            row.meshes,
            row.cull_front,
            row.own_light,
            row.examples.join(", ")
        );
    }
    println!("\nuw01_floor* nodes:");
    for f in floors {
        println!("  {f}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unit square at height `y`, facing up or down.
    fn quad(y: f32, up: bool) -> [[V3; 3]; 2] {
        let (a, b, c, d) = ([0.0, y, 0.0], [1.0, y, 0.0], [1.0, y, 1.0], [0.0, y, 1.0]);
        // Counter-clockwise seen from +Y: (a, d, c), (a, c, b).
        if up {
            [[a, d, c], [a, c, b]]
        } else {
            [[a, c, d], [a, b, c]]
        }
    }

    fn facings(tris: &[[V3; 3]]) -> Vec<(Facing, bool)> {
        judge(tris, &vec![true; tris.len()])
            .into_iter()
            .map(|j| (j.facing, j.low))
            .collect()
    }

    #[test]
    fn faces_are_judged_from_above() {
        use Facing::*;
        assert_eq!(facings(&quad(0.0, true)), [(Up, true); 2]);
        assert_eq!(facings(&quad(0.0, false)), [(Open, true); 2]);
        assert_eq!(facings(&quad(3.0, false)), [(Open, false); 2]);
        // A slab: its bottom is the underside of a solid.
        let mut slab = quad(0.2, true).to_vec();
        slab.extend(quad(0.0, false));
        assert_eq!(
            facings(&slab),
            [(Up, true), (Up, true), (Under, true), (Under, true)]
        );
        // A two-sided sheet.
        let mut sheet = quad(0.0, true).to_vec();
        sheet.extend(quad(0.0, false));
        assert_eq!(facings(&sheet)[2].0, Under);
        // A tall box: its bottom is still the underside of a solid.
        let mut tall = quad(3.5, true).to_vec();
        tall.extend(quad(0.0, false));
        assert_eq!(facings(&tall)[2].0, Under);
        // A floor wound downward under a closed slab above: the slab's
        // bottom is met first.
        let mut room = quad(0.0, false).to_vec();
        room.extend(quad(2.5, false));
        room.extend(quad(2.7, true));
        assert_eq!(
            facings(&room),
            [
                (Open, true),
                (Open, true),
                (Under, false),
                (Under, false),
                (Up, false),
                (Up, false)
            ]
        );
        // Faces that are not solid hide nothing.
        let solid = [false, false, true, true];
        assert_eq!(judge(&slab, &solid)[2].facing, Open);
    }

    #[test]
    fn mirrored_placements_keep_the_front() {
        let node = ShapeNode {
            name: "floor".into(),
            parent: None,
            local: Affine::IDENTITY,
            tris: quad(0.0, true).to_vec(),
            null: Vec::new(),
            instance: false,
        };
        let mut mirror = Affine::IDENTITY;
        mirror.axes[0] = [-1.0, 0.0, 0.0];
        let mut object = Object::default();
        object.add(&node, 0, FLOOR, &mirror);
        assert_eq!(facings(&object.tris), [(Facing::Up, true); 2]);
        let mut f = Facings::default();
        f.count_node(&node, &mirror);
        assert_eq!(f.mirrored, 1);
    }

    #[test]
    fn transforms_compose_and_invert() {
        let a = Affine {
            axes: [[0.0, 0.0, -1.0], [0.0, 2.0, 0.0], [1.0, 0.0, 0.0]],
            t: [1.0, 2.0, 3.0],
        };
        let p = [0.5, -1.0, 4.0];
        let back = a.inverse().unwrap().point(a.point(p));
        assert!(back.iter().zip(p).all(|(x, y)| (x - y).abs() < 1e-5));
        assert!((a.then(&a).det() - 4.0).abs() < 1e-5);
        let flat = Affine {
            axes: [[0.0; 3]; 3],
            t: [0.0; 3],
        };
        assert!(flat.inverse().is_none());
    }

    #[test]
    fn pieces_are_the_nodes_the_grid_places() {
        assert_eq!(piece_role(true, "+FMA01."), Some(FLOOR));
        assert_eq!(piece_role(false, "+RMA00.001"), Some(CEILING));
        assert_eq!(piece_role(false, "+WCA04."), Some(WALL));
        assert_eq!(piece_role(false, "+TCD23.200"), Some(WALL));
        assert_eq!(piece_role(false, "+FBA02.028"), None);
        assert_eq!(piece_role(false, "[WCA00.035"), None);
        assert_eq!(piece_role(true, "Box052"), None);
    }

    #[test]
    fn looping_parents_are_cut() {
        let g = globals(
            3,
            |i| Some((i + 1) % 3),
            |_| Affine::IDENTITY,
            |_| Affine::IDENTITY,
        );
        assert_eq!(g, vec![Affine::IDENTITY; 3]);
    }
}
