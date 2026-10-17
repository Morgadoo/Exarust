//! `exa-tool morphs`: the physique morph sets of the races' `*morph.rfc`,
//! and the weights every placed character's record gives them.

use anyhow::{Result, anyhow};
use exa_formats::characters::{CHARACTERS, LevelCharacters};
use exa_formats::morph::{self, MorphBody, Physique};
use exa_formats::races;
use exa_formats::rfc::{NodeBody, Scene};
use exa_rpk::{GameData, PackKind};

const CM: f32 = 100.0;

fn cm(v: [f32; 3]) -> String {
    format!("({:.2}, {:.2}, {:.2})", v[0] * CM, v[1] * CM, v[2] * CM)
}

fn length(v: [f32; 3]) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

/// Every `*morph.rfc` of `Resource.rpk`: the node carrying the set, its
/// frame, the landmarks and each target's reach; then, with `records`, the
/// weights of every placed character of every level.
pub fn morphs(game: &GameData, records: bool) -> Result<()> {
    if records {
        return record_weights(game);
    }
    let resource = game
        .pack(PackKind::Resource)
        .ok_or_else(|| anyhow!("Resource.rpk is not installed"))?;
    let mut files = 0;
    for e in resource.entries() {
        if !e.name.to_ascii_lowercase().ends_with("morph.rfc") {
            continue;
        }
        files += 1;
        let scene = Scene::parse(resource.bytes(e))?;
        let frames = morph::model_frames(&scene);
        for (node, frame) in scene.nodes.iter().zip(&frames) {
            let Some(set) = &node.morph else { continue };
            let set = match set {
                Ok(s) => s,
                Err(err) => {
                    println!(
                        "{} [{}]: morph set does not decode: {err}",
                        e.name, node.name
                    );
                    continue;
                }
            };
            let positions = match &node.body {
                NodeBody::Mesh(m) => m.position_count,
                _ => 0,
            };
            println!(
                "{} [{}]: {} landmarks, {} positions (mesh {positions}), {} targets, at {}",
                e.name,
                node.name,
                set.landmarks.len(),
                set.base.len(),
                set.targets.len(),
                cm(frame[3])
            );
            for (i, l) in set.landmarks.iter().enumerate() {
                println!("  landmark {i:>2} (bone {:>2}) {}", i + 1, cm(*l));
            }
            for (t, target) in set.targets.iter().enumerate() {
                let (far, at) = target
                    .positions
                    .iter()
                    .enumerate()
                    .map(|(i, d)| (length(*d), i))
                    .fold((0.0f32, 0), |a, b| if b.0 > a.0 { b } else { a });
                let n = target.positions.len().max(1) as f32;
                let mean: [f32; 3] =
                    std::array::from_fn(|k| target.positions.iter().map(|d| d[k]).sum::<f32>() / n);
                let joints: Vec<String> = target
                    .landmarks
                    .iter()
                    .enumerate()
                    .filter(|(_, d)| length(**d) >= 0.0005)
                    .map(|(i, d)| format!("{i}{}", cm(*d)))
                    .collect();
                println!(
                    "  target {t:>2}: farthest {:.2} cm (position {at} at {}), mean {}, joints moved: {}",
                    far * CM,
                    cm(set.base[at]),
                    cm(mean),
                    if joints.is_empty() {
                        "none".into()
                    } else {
                        joints.join(" ")
                    }
                );
            }
        }
        if let Some(body) = MorphBody::of_scene(&scene) {
            println!("  morph body ready: {} positions", body.set.base.len());
        }
    }
    println!("{files} morph files");
    Ok(())
}

/// The weights of every placed character of every level whose race has a
/// morph file.
fn record_weights(game: &GameData) -> Result<()> {
    let resource = game
        .pack(PackKind::Resource)
        .ok_or_else(|| anyhow!("Resource.rpk is not installed"))?;
    let races = resource
        .get("races.rdb")
        .map(races::parse)
        .transpose()?
        .unwrap_or_default();
    let (mut placed, mut morphed, mut neutral) = (0, 0, 0);
    let mut lo = [f32::MAX; morph::TARGETS];
    let mut hi = [f32::MIN; morph::TARGETS];
    for e in resource.entries() {
        if !e.name.to_ascii_lowercase().ends_with(".rfc") {
            continue;
        }
        let Some(section) = exa_formats::chunk::rfc_chunks(resource.bytes(e))
            .ok()
            .and_then(|c| c.flatten().find(|c| c.tag == CHARACTERS))
        else {
            continue;
        };
        let Ok(level) = LevelCharacters::parse(section.data) else {
            continue;
        };
        for p in &level.placements {
            let Some(r) = level.record(p.id) else {
                continue;
            };
            placed += 1;
            let stem = races::of_character(&races, r.race)
                .map_or_else(|| "human".into(), |race| race.stem.to_ascii_lowercase());
            if resource.get(&format!("{stem}morph.rfc")).is_none() {
                continue;
            }
            morphed += 1;
            let w = Physique::of_record(r).weights(morph::TARGETS);
            if !morph::morphs(&w) {
                neutral += 1;
            }
            for (k, v) in w.iter().enumerate() {
                lo[k] = lo[k].min(*v);
                hi[k] = hi[k].max(*v);
            }
            let shown: Vec<String> = w.iter().map(|v| format!("{v:.3}")).collect();
            println!(
                "{:<14} {:<16} {stem:<8} {}{} physique ({:.3}, {:.3}) height {:.3} age {:.3} feral {:.3}: [{}]",
                e.name,
                r.name,
                if r.female { "female" } else { "male" },
                if p.is_dead() { " dead" } else { "" },
                r.physique[0],
                r.physique[1],
                r.height,
                r.age,
                r.feral,
                shown.join(", ")
            );
        }
    }
    println!("{placed} placed, {morphed} with a morph set, {neutral} at neutral weights");
    for k in 0..morph::TARGETS {
        if morphed > 0 {
            println!("  w{k}: {:.4} … {:.4}", lo[k], hi[k]);
        }
    }
    Ok(())
}
