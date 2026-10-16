//! `exa-tool nav <level>`: a level's navigation grid, built without a
//! window exactly as the game builds it at load (`docs/NAVIGATION.md`).

use std::path::Path;

use anyhow::{Context, Result};
use exa_rpk::GameData;
use exa_world::nav::view::cell_colour;
use exa_world::nav::{Layer, NavGrid, headless};

pub fn nav(game: &GameData, level: &str, png: Option<&Path>) -> Result<()> {
    let built = headless::level_source(game, level)
        .with_context(|| format!("level {level} not found or not readable"))?;
    let source = &built.source;
    let started = std::time::Instant::now();
    let grid = built.build();
    let s = grid.stats();
    println!(
        "{level}: {} × {} cells, voxel 0 tops out at {:.2} m",
        s.width,
        s.depth,
        grid.base()
    );
    println!(
        "inputs: {} meshes ({} points), {} floor meshes, {} terrain tiles; built in {:.2?}",
        source.meshes.len(),
        source.meshes.iter().map(|(p, _)| p.len()).sum::<usize>(),
        source.floors.len(),
        source.ground.len(),
        started.elapsed()
    );
    println!("columns by spans (0–3): {:?}", s.columns_by_spans);
    for layer in Layer::BOTH {
        let k = layer as usize;
        println!(
            "{layer:?}: {} floors, {} passable (size class 0), {} open sky",
            s.floors[k], s.passable[k], s.sky[k]
        );
    }
    if let Some(path) = png {
        let (w, h, rgba) = picture(&grid);
        crate::write_png(path, w, h, &rgba)?;
        println!(
            "wrote {} (lower layer left, upper right; X right, Z down)",
            path.display()
        );
    }
    Ok(())
}

/// Both layers side by side, a pixel per cell: black without a floor, dark
/// blue for open sky, else the overlay's colour by clearance, shaded by
/// the floor's height.
fn picture(grid: &NavGrid) -> (u32, u32, Vec<u8>) {
    let (w, d) = grid.size();
    let gap = 4;
    let width = 2 * w + gap;
    let mut rgba = vec![0u8; width * d * 4];
    let floors = Layer::BOTH
        .iter()
        .flat_map(|&l| (0..d).flat_map(move |z| (0..w).map(move |x| grid.cell(x, z, l))));
    let (lo, hi) = floors
        .filter(|c| c.has_floor())
        .fold((u8::MAX, 0u8), |(lo, hi), c| {
            (lo.min(c.floor), hi.max(c.floor))
        });
    for (k, layer) in Layer::BOTH.into_iter().enumerate() {
        for z in 0..d {
            for x in 0..w {
                let c = grid.cell(x, z, layer);
                let colour = if c.floor == exa_world::nav::SKY {
                    [20, 20, 90]
                } else if c.has_floor() {
                    let shade = if hi > lo {
                        0.45 + 0.55 * f32::from(c.floor - lo) / f32::from(hi - lo)
                    } else {
                        1.0
                    };
                    let rgb = cell_colour(layer, c.clearance).to_srgba();
                    [rgb.red, rgb.green, rgb.blue].map(|v| (v * shade * 255.0) as u8)
                } else {
                    [0, 0, 0]
                };
                let i = (z * width + k * (w + gap) + x) * 4;
                rgba[i..i + 3].copy_from_slice(&colour);
                rgba[i + 3] = 255;
            }
        }
    }
    (width as u32, d as u32, rgba)
}
