//! Rigid bodies of models and the mechanism records of the object
//! databases: `exa-tool bodies`, `exa-tool mechanisms`, `exa-tool corpus
//! bodies`. Lengths are printed in centimetres, as the files store them.

use anyhow::{Context, Result};
use exa_formats::FileKind;
use exa_formats::material::Material;
use exa_formats::objdb::{self, Item, door_sound};
use exa_formats::rdb::Database;
use exa_formats::rfc::NodeBody;
use exa_formats::rfc::Scene;
use exa_formats::rigid_body::RigidBody;
use exa_rpk::{Archive, GameData, PackKind};

const CM: f32 = 100.0;

fn cm(p: [f32; 3]) -> String {
    format!("({:.2}, {:.2}, {:.2})", p[0] * CM, p[1] * CM, p[2] * CM)
}

/// Every node of a model that carries a rigid body or the cloth marker.
pub fn bodies(game: &GameData, pack: &str, entry: &str) -> Result<()> {
    let scene = Scene::parse(crate::entry_bytes(game, pack, entry)?)
        .with_context(|| format!("decoding {pack}/{entry}"))?;
    let mut any = false;
    for (i, node) in scene.nodes.iter().enumerate() {
        if node.cloth_marker {
            any = true;
            println!("node {} {:?}: cloth marker", i + 1, node.name);
        }
        match &node.rigid_body {
            None => {}
            Some(Err(e)) => {
                any = true;
                println!("node {} {:?}: undecodable body: {e}", i + 1, node.name);
            }
            Some(Ok(body)) => {
                any = true;
                let parent = node.parent.and_then(|p| scene.nodes.get(p - 1));
                let child = parent.is_some_and(|p| p.has_rigid_body());
                print_body(i + 1, &node.name, child, body);
                print_material(game, &scene, node, body);
            }
        }
    }
    if !any {
        println!("{pack}/{entry}: no rigid body or cloth marker");
    }
    Ok(())
}

fn print_body(index: usize, name: &str, child: bool, b: &RigidBody) {
    println!(
        "node {index} {name:?}: {} body, class {:#010x}, mobility {}",
        if child { "child" } else { "root" },
        b.class,
        b.mobility
    );
    let rest: Vec<String> = b.rest.iter().map(|p| cm(*p)).collect();
    println!("  rest points {}", rest.join(" "));
    println!("  point masses {:?}", b.point_masses);
    if let (Some(min), Some(max)) = (
        b.spheres.iter().map(|s| s.radius).reduce(f32::min),
        b.spheres.iter().map(|s| s.radius).reduce(f32::max),
    ) {
        println!(
            "  {} spheres, radius {:.2}..{:.2} cm",
            b.spheres.len(),
            min * CM,
            max * CM
        );
    }
    for l in &b.links {
        let kind = match (l.is_slider(), l.slack) {
            (true, t) => format!("slider, travel {:.2} cm", t * CM),
            (false, 0.0) => "pin".to_string(),
            (false, s) => format!("slack {:.2} cm", s * CM),
        };
        println!(
            "  link {:#x}: {kind}, body {} parent {}, strength {}, smoothing {}{}",
            l.flags,
            cm(l.on_body),
            cm(l.on_parent),
            l.strength,
            l.smoothing,
            l.axis
                .map(|a| format!(", axis {}", cm(a)))
                .unwrap_or_default()
        );
    }
    for s in &b.sounds {
        println!(
            "  sound slot {}: {:?} at {}, volume {}, pitch {}",
            s.id,
            s.sound,
            cm(s.point),
            s.volume,
            s.pitch
        );
    }
}

/// A material by name: the file's own record, else the libraries'.
fn find_material(game: &GameData, scene: &Scene, name: &str) -> Option<Material> {
    scene.material(name).cloned().or_else(|| {
        let libs = game.project().material_libraries.iter().map(String::as_str);
        libs.chain(["humanbase.rml"]).find_map(|lib| {
            let archive = Archive::parse(game.resource_file(lib)?).ok()?;
            Material::parse(archive.get(name)?).ok()
        })
    })
}

/// The material of the body node's first mesh part, what the body's
/// mobility is divided by when it registers, and its impact sound set.
fn print_material(game: &GameData, scene: &Scene, node: &exa_formats::rfc::Node, b: &RigidBody) {
    let NodeBody::Mesh(mesh) = &node.body else {
        return;
    };
    let Some(name) = mesh.groups.first().map(|g| g.material.as_str()) else {
        return;
    };
    match find_material(game, scene, name) {
        Some(m) => println!(
            "  material {name:?}: divisor {} (stored {}), registered mobility {}, impact sounds {}",
            m.mobility_divisor(),
            m.physical_divisor,
            b.mobility / m.mobility_divisor(),
            m.collision_sound_set(name)
        ),
        None => println!("  material {name:?} not found: divisor 1"),
    }
}

/// The door and operative records of `objdb.rdb`, or of a level's own
/// database.
pub fn mechanisms(game: &GameData, level: Option<&str>) -> Result<()> {
    let items = match level {
        None => {
            let db = Database::parse(crate::entry_bytes(game, "Resource", "objdb.rdb")?)?;
            objdb::items(&db)
        }
        Some(level) => {
            let name = if level.ends_with(".rfc") {
                level.to_string()
            } else {
                format!("{level}.rfc")
            };
            Scene::parse(crate::entry_bytes(game, "Resource", &name)?)
                .with_context(|| format!("decoding {name}"))?
                .local_items
        }
    };
    let (mut doors, mut operatives) = (0, 0);
    for item in &items {
        if item.door.is_none() && item.operative.is_none() {
            continue;
        }
        print_item(item);
        doors += usize::from(item.door.is_some());
        operatives += usize::from(item.operative.is_some());
    }
    println!("{doors} doors, {operatives} operative records");
    Ok(())
}

fn print_item(item: &Item) {
    let mechanism = item
        .mechanism()
        .map_or("plain".to_string(), |m| format!("{m:?}"));
    println!(
        "{:#06x} {:<18} model {:<18} {mechanism}",
        item.id,
        format!("{:?}", item.name),
        format!("{:?}", item.model)
    );
    if let Some(d) = item.door {
        const NAMES: [(u32, &str); 8] = [
            (objdb::door::LATCHES, "latches"),
            (objdb::door::LOCKED, "locked"),
            (objdb::door::NOT_CLICKABLE, "not-clickable"),
            (objdb::door::MOTORISED, "motorised"),
            (objdb::door::FAST, "fast"),
            (objdb::door::INVERTED, "inverted"),
            (objdb::door::FROZEN, "frozen"),
            (objdb::door::NODE_MARK, "node-mark"),
        ];
        let flags: Vec<&str> = NAMES
            .iter()
            .filter(|(f, _)| d.has(*f))
            .map(|(_, n)| *n)
            .collect();
        println!(
            "  door flags {:#x} [{}], linked {:#x}, sounds {} {} {} {}",
            d.flags,
            flags.join(" "),
            d.key,
            door_sound(d.lock_sound, "LkO"),
            door_sound(d.slam_sound, "slm"),
            door_sound(d.hinge_sound, "hng"),
            door_sound(d.unlatch_sound, "opn")
        );
    }
    if let Some(o) = item.operative {
        println!(
            "  operative class {} kind {}, flags {:#x}, value {}, time {} (x100 ms), targets {:#x} {:#x}",
            o.class, o.kind, o.flags, o.value, o.time, o.target, o.second
        );
    }
}

/// Bodies, links, sliders and cloth markers across the model packs.
pub fn corpus(game: &GameData) -> Result<()> {
    for kind in [
        PackKind::Objects,
        PackKind::Objlib,
        PackKind::Characters,
        PackKind::CharExt,
        PackKind::Resource,
    ] {
        let Some(pack) = game.pack(kind) else {
            continue;
        };
        let (mut models, mut with_body, mut bodies, mut bad) = (0, 0, 0, 0);
        let (mut with_links, mut hung, mut sliders, mut cloth) = (0, 0, 0, 0);
        for e in pack.entries() {
            let b = pack.bytes(e);
            if FileKind::detect(b) != FileKind::ChunkTree {
                continue;
            }
            let Ok(scene) = Scene::parse(b) else {
                continue;
            };
            models += 1;
            let decoded: Vec<&RigidBody> = scene.nodes.iter().filter_map(|n| n.rigid()).collect();
            bad += scene.nodes.iter().filter(|n| n.has_rigid_body()).count() - decoded.len();
            bodies += decoded.len();
            with_body += usize::from(!decoded.is_empty());
            with_links += usize::from(decoded.iter().any(|b| !b.links.is_empty()));
            hung += usize::from(scene.nodes.iter().any(|n| {
                let parent_body = n
                    .parent
                    .and_then(|p| scene.nodes.get(p - 1))
                    .is_some_and(|p| p.has_rigid_body());
                !parent_body && n.rigid().is_some_and(|b| !b.links.is_empty())
            }));
            sliders += decoded
                .iter()
                .filter(|b| b.links.iter().any(|l| l.is_slider()))
                .count();
            cloth += scene.nodes.iter().filter(|n| n.cloth_marker).count();
        }
        println!(
            "{:<11} {models:>5} models, {with_body:>5} with bodies ({bodies} bodies, {bad} undecodable), \
             {with_links} with links ({hung} hung from the world), {sliders} slider bodies, {cloth} cloth markers",
            kind.file_stem()
        );
    }
    Ok(())
}
