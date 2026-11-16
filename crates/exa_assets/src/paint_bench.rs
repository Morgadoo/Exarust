//! A timing harness for the CPU texture generators (`exa-tool paint-bench`):
//! every apparel item of the object database painted with its appearance,
//! and every factory recipe assembled at a fixed seed and painted into its
//! atlas. The textures the painters read are decoded once first, so the
//! timed rounds measure the generators (painting, atlas composition and
//! the painted images' mip chains), not the texture decoder.

use std::time::{Duration, Instant};

use bevy::asset::Assets;
use bevy::image::Image;
use bevy::mesh::Mesh;
use exa_formats::objdb::{Appearance, Weapon};
use exa_render::material::ExaMaterial;

use crate::model::{ModelCache, ModelLoader, Paint, painted_model_stem};
use crate::{GameData, PackKind};

/// What one timed round painted, and how long it took.
#[derive(Debug, Clone, Default)]
pub struct BenchRound {
    /// Apparel texture sets painted, and their time.
    pub apparel: usize,
    pub apparel_time: Duration,
    /// Factory atlases painted, and their time.
    pub factories: usize,
    pub factory_time: Duration,
}

/// The apparel jobs of the object database: one per distinct worn model
/// stem and appearance.
fn apparel_jobs(loader: &mut ModelLoader<'_>) -> Vec<(String, Appearance)> {
    let Some(bytes) = loader
        .game
        .pack(PackKind::Resource)
        .and_then(|p| p.get("objdb.rdb"))
    else {
        return Vec::new();
    };
    let Ok(db) = exa_formats::rdb::Database::parse(bytes) else {
        return Vec::new();
    };
    let mut jobs: Vec<(String, Appearance)> = Vec::new();
    for item in exa_formats::objdb::items(&db) {
        let Some(appearance) = item.appearance else {
            continue;
        };
        if item.worn_model.is_empty() {
            continue;
        }
        let Some(model) = loader.model(PackKind::Characters, &item.worn_model) else {
            continue;
        };
        let Some(stem) = painted_model_stem(&model) else {
            continue;
        };
        if !jobs.iter().any(|(s, a)| *s == stem && *a == appearance) {
            jobs.push((stem, appearance));
        }
    }
    jobs
}

/// The factory recipes of `Factories.rpk`, sorted, each with the weapon a
/// bench paints it as (seeds from the recipe's index, middling quality).
fn factory_jobs(game: &GameData) -> Vec<(String, Weapon)> {
    let Some(pack) = game.pack(PackKind::Factories) else {
        return Vec::new();
    };
    let mut names: Vec<String> = pack
        .entries()
        .iter()
        .map(|e| e.name.trim_end_matches(".fty").to_string())
        .collect();
    names.sort();
    names
        .into_iter()
        .enumerate()
        .map(|(i, name)| {
            let seed = (i as u32 + 1).wrapping_mul(0x9E37_79B9);
            let weapon = Weapon {
                seeds: [seed, seed.rotate_left(7)],
                quality: 128,
                damage: 64,
                dirt: 32,
                ..Default::default()
            };
            (name, weapon)
        })
        .collect()
}

/// Paints every apparel set and factory atlas `rounds` times after one
/// warm-up round, and returns the timed rounds. `limit` caps the jobs of
/// each kind (0: all).
pub fn run(game: &GameData, rounds: usize, limit: usize) -> Vec<BenchRound> {
    let mut cache = ModelCache::default();
    let mut meshes = Assets::<Mesh>::default();
    let mut materials = Assets::<ExaMaterial>::default();
    let mut images = Assets::<Image>::default();
    let mut loader = ModelLoader {
        game,
        cache: &mut cache,
        meshes: &mut meshes,
        materials: &mut materials,
        images: &mut images,
        bc_supported: false,
    };
    let cap = |n: usize| if limit == 0 { n } else { n.min(limit) };
    let mut apparel = apparel_jobs(&mut loader);
    apparel.truncate(cap(apparel.len()));
    let mut factories = factory_jobs(game);
    factories.truncate(cap(factories.len()));
    let far = Instant::now() + Duration::from_secs(3600);
    // Warm-up: decode every texture the painters read, and assemble every
    // weapon once (its geometry is kept, so later rounds only repaint).
    for (stem, appearance) in &apparel {
        loader.warm_paint(stem, appearance, far);
    }
    for (name, weapon) in &factories {
        loader.factory_weapon(name, Some(weapon));
    }
    let mut out = Vec::with_capacity(rounds);
    for _ in 0..rounds {
        loader.cache.forget_painted();
        let mut round = BenchRound::default();
        let start = Instant::now();
        for (stem, appearance) in &apparel {
            if let Paint::Job(job) = loader.paint_job(stem, appearance) {
                let painted = job.paint();
                loader.painted_material(painted);
                round.apparel += 1;
            }
        }
        round.apparel_time = start.elapsed();
        let start = Instant::now();
        for (name, weapon) in &factories {
            if loader.factory_weapon(name, Some(weapon)).is_some() {
                round.factories += 1;
            }
        }
        round.factory_time = start.elapsed();
        out.push(round);
    }
    out
}
