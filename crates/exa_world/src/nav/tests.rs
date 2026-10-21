use std::sync::Arc;

use super::source::{FloorMesh, floor_points, grid_base, grid_size};
use super::*;
use crate::rooms::CELL_OPEN;
use bevy::math::Affine3A;

/// A column with voxels `lo..=hi` of each slab solid.
fn column(slabs: &[(usize, usize)]) -> [u8; COLUMN_VOXELS / 8] {
    let mut c = [0u8; COLUMN_VOXELS / 8];
    for &(lo, hi) in slabs {
        for v in lo..=hi {
            c[v / 8] |= 1 << (v % 8);
        }
    }
    c
}

fn span(floor: u8, underside: u8) -> Span {
    Span { floor, underside }
}

#[test]
fn spans_need_more_than_120_cm_above() {
    // A floor slab from the bottom filling whole bytes: one span, no
    // underside.
    assert_eq!(column_spans(&column(&[(0, 215)])), vec![span(215, 0)]);
    assert_eq!(column_spans(&column(&[(0, 223)])), vec![span(223, 0)]);
    // Ending inside a byte, the scan steps down it a voxel at a time and
    // leaves the underside one above the byte's bottom (voxel 217 for a
    // floor at 221; at 217 itself, the floor).
    assert_eq!(column_spans(&column(&[(0, 221)])), vec![span(221, 217)]);
    assert_eq!(column_spans(&column(&[(0, 217)])), vec![span(217, 217)]);
    // A slab in the air: the underside creeps down to its byte's bottom (a
    // lone voxel at 221 has 216).
    assert_eq!(column_spans(&column(&[(221, 221)])), vec![span(221, 216)]);
    assert_eq!(column_spans(&column(&[(100, 110)])), vec![span(110, 96)]);
    // The room above a slab is counted from the bottom of the byte the
    // last change fell in. A ceiling slab from 235 has 13 free voxels over
    // a floor at 221 but the scan counts 11 (232 − 221): they join.
    assert_eq!(
        column_spans(&column(&[(0, 221), (235, 237)])),
        vec![span(237, 217)]
    );
    // One from 236 is counted from 232 too; with its byte's bottom at 232
    // (under 235) it stays a span over the floor's.
    assert_eq!(
        column_spans(&column(&[(0, 216), (236, 237)])),
        vec![span(237, 232), span(216, 0)]
    );
    // Counted so, 13 free voxels make a span and 12 join: a floor at 219
    // or 220 under that ceiling.
    assert_eq!(
        column_spans(&column(&[(0, 219), (235, 237)])),
        vec![span(237, 232), span(219, 217)]
    );
    assert_eq!(
        column_spans(&column(&[(0, 220), (235, 237)])),
        vec![span(237, 217)]
    );
    // A slab whose bottom is a byte's bottom leaves the cursor a bit down
    // in the byte under it, and skipping free bytes keeps it there: the
    // next byte's top voxel is missed, so a floor at 199 reads 198.
    assert_eq!(
        column_spans(&column(&[(0, 199), (216, 216)])),
        vec![span(216, 215), span(198, 0)]
    );
    // A joined slab in the air moves the underside under it.
    assert_eq!(
        column_spans(&column(&[(50, 60), (70, 80)])),
        vec![span(80, 48)]
    );
}

#[test]
fn spans_drop_high_undersides_and_cap_at_three() {
    // A slab just under the top whose underside is at voxel 235 or above is
    // dropped unless it is the last: a voxel at 240 alone over a floor (its
    // underside 239, as the next byte holds nothing).
    assert_eq!(
        column_spans(&column(&[(0, 221), (240, 240)])),
        vec![span(221, 217)]
    );
    // Slabs from 235 or 236 to 240 end their scan at 232: kept.
    assert_eq!(
        column_spans(&column(&[(0, 100), (236, 240)])),
        vec![span(240, 232), span(100, 97)]
    );
    // The last span is kept whatever its underside.
    assert_eq!(column_spans(&column(&[(240, 240)])), vec![span(240, 239)]);
    // Floors above voxel 240 are open sky.
    assert_eq!(column_spans(&column(&[(0, 247)])), vec![span(SKY, 0)]);
    assert_eq!(column_spans(&column(&[(0, 240)])), vec![span(240, 0)]);
    // A shelf at 229 under a beam at 246–248: the beam's byte ends at 240,
    // 11 voxels over the shelf, so they join under open sky.
    assert_eq!(
        column_spans(&column(&[(229, 229), (246, 248)])),
        vec![span(SKY, 224)]
    );
    // Four slabs 20 voxels apart: three spans.
    assert_eq!(
        column_spans(&column(&[(0, 10), (31, 40), (61, 70), (91, 100)])),
        vec![span(100, 88), span(70, 56), span(40, 24)]
    );
    assert!(column_spans(&column(&[])).is_empty());
    // Only voxel 0: a floor of 0 is no floor.
    assert!(column_spans(&column(&[(0, 0)])).is_empty());
}

#[test]
fn points_mark_the_voxel_and_cell_they_round_to() {
    // 2 × 2 rooms, grid top 3.4 m: voxel 221 tops out at 0.
    let base = grid_base(3.4);
    assert!((base - -22.1).abs() < 1e-5);
    let mut c = Columns::new(40, 40, base);
    // Cell 20 spans x from 0 to 30 cm; voxel 222 from 0 to 10 cm.
    c.stamp(DVec3::new(0.15, 0.05, 0.15), 0.0);
    assert!(c.solid(20, 20, 222));
    // On a voxel boundary the tie goes to the even voxel: 0 cm is grid
    // height 221.5 → 222; 20 cm is 223.5 → 224.
    c.stamp(DVec3::new(0.45, 0.0, 0.15), 0.0);
    assert!(c.solid(21, 20, 222));
    c.stamp(DVec3::new(0.45, 0.2, 0.15), 0.0);
    assert!(c.solid(21, 20, 224));
    // A cell boundary (x = 30 cm, grid 20.5) ties to the even cell, 20.
    c.stamp(DVec3::new(0.30, 0.55, 0.15), 0.0);
    assert!(c.solid(20, 20, 227));
    // A generated floor point 10 cm under a floor at 0 marks voxel 221.
    c.stamp(DVec3::new(0.15, -0.10, 0.45), FLOOR_LIFT);
    assert!(c.solid(20, 21, 221));
    // Border cells and voxel 255 take nothing.
    let mut edge = Columns::new(40, 40, base);
    edge.stamp(DVec3::new(-5.85, 0.0, 0.15), 0.0);
    edge.stamp(DVec3::new(0.15, 3.35, 0.15), 0.0);
    assert!((0..COLUMN_VOXELS).all(|v| !edge.solid(0, 20, v) && !edge.solid(20, 20, v)));
}

#[test]
fn ground_fills_columns_to_its_height() {
    let mut c = Columns::new(40, 40, grid_base(3.4));
    c.fill_ground(5, 5, 0.0);
    assert!((0..=221).all(|v| c.solid(5, 5, v)) && !c.solid(5, 5, 222));
    // 14 cm rounds to voxel 222; below the base fills nothing.
    c.fill_ground(6, 5, 0.14);
    assert!(c.solid(6, 5, 222) && !c.solid(6, 5, 223));
    c.fill_ground(7, 5, -22.2);
    assert!(!c.solid(7, 5, 0));
    // Up to the top of the column.
    c.fill_ground(8, 5, 3.4);
    assert!(c.solid(8, 5, 255));
    assert_eq!(column_spans(c.column(8, 5)), vec![span(SKY, 0)]);
}

#[test]
fn steps_walls_and_drops() {
    let own = 221;
    let none = Span::default();
    // Level ground, a 40 cm step up, 50 cm (a wall), drops.
    assert_eq!(step_measure(own, span(221, 0), none), 0);
    assert_eq!(step_measure(own, span(225, 0), none), 4);
    assert_eq!(step_measure(own, span(226, 0), none), 8);
    assert_eq!(step_measure(own, span(207, 0), none), 14);
    assert_eq!(step_measure(own, span(206, 0), none), 15);
    // No ground at all.
    assert_eq!(step_measure(own, none, none), 15);
    // A low ceiling (under 150 cm of room) over level ground is a wall.
    assert_eq!(step_measure(own, span(221, 0), span(240, 230)), 8);
    assert_eq!(step_measure(own, span(221, 0), span(250, 236)), 0);
    // An upper floor under 50 cm of room is stepped onto when it is less
    // than 50 cm up.
    assert_eq!(step_measure(own, span(200, 0), span(224, 222)), 3);
    assert_eq!(step_measure(own, span(200, 0), span(226, 222)), 8);
    // A slab overhead (its ground high up, room under it).
    assert_eq!(step_measure(own, span(240, 230), none), 8);
    // Its weights: orthogonal 15 → 60, diagonal 15 → 42.
    assert_eq!((15 * ORTHOGONAL) >> 6, 60);
    assert_eq!((15 * DIAGONAL) >> 6, 42);
}

#[test]
fn walkers_meet_the_layer_within_150_cm() {
    let own = 221;
    let none = Span::default();
    assert_eq!(met_layer(own, span(221, 0), none), Layer::Lower);
    // An upper floor whose underside is more than 150 cm up: under it.
    assert_eq!(met_layer(own, span(221, 0), span(240, 237)), Layer::Lower);
    assert_eq!(met_layer(own, span(221, 0), span(240, 236)), Layer::Upper);
    // Without an underside the upper floor itself decides.
    assert_eq!(met_layer(own, span(200, 0), span(237, 0)), Layer::Lower);
    assert_eq!(met_layer(own, span(200, 0), span(236, 0)), Layer::Upper);
}

/// A 30 × 30 grid whose cells with x ≥ 10 have a floor at voxel 221.
fn half_plane() -> NavGrid {
    let mut c = Columns::new(30, 30, grid_base(3.4));
    for z in 0..30 {
        for x in 10..30 {
            c.fill_ground(x, z, 0.0);
        }
    }
    NavGrid::from_columns(&c)
}

#[test]
fn clearance_grows_toward_an_edge() {
    let g = half_plane();
    let row: Vec<NavCell> = (8..14).map(|x| g.cell(x, 15, Layer::Lower)).collect();
    let pick = |f: fn(&NavCell) -> u8| row.iter().map(f).collect::<Vec<u8>>();
    assert_eq!(pick(|c| c.floor), [0, 0, 221, 221, 221, 221]);
    assert_eq!(pick(|c| c.roughness), [120, 120, 60, 0, 0, 0]);
    // (60 + 42 + 30 + 60 + 42 + 30) / 2 at the edge, (21 + 21 + 30) / 2 next.
    assert_eq!(pick(|c| c.blurred), [120, 120, 132, 36, 0, 0]);
    assert_eq!(pick(|c| c.clearance), [240, 240, 132, 60, 18, 0]);
    // Size class 0 passes 180, class 1 90.
    let edge = g.cell(10, 15, Layer::Lower);
    assert!(edge.passable(0) && !edge.passable(1));
    assert!(g.cell(11, 15, Layer::Lower).passable(1));
    // The passes keep off the border, one cell further each.
    assert_eq!(g.cell(10, 1, Layer::Lower).roughness, 0);
    assert_eq!(g.cell(10, 2, Layer::Lower).roughness, 60);
    assert_eq!(g.cell(10, 2, Layer::Lower).blurred, 0);
    assert_eq!(g.cell(10, 3, Layer::Lower).clearance, 0);
    assert_eq!(g.cell(10, 4, Layer::Lower).clearance, 132);
}

#[test]
fn a_step_is_rough_on_both_sides() {
    let mut c = Columns::new(30, 30, grid_base(3.4));
    for z in 0..30 {
        for x in 0..30 {
            c.fill_ground(x, z, if x < 15 { 0.0 } else { 0.4 });
        }
    }
    let g = NavGrid::from_columns(&c);
    assert_eq!(g.cell(14, 15, Layer::Lower).floor, 221);
    assert_eq!(g.cell(15, 15, Layer::Lower).floor, 225);
    // 4 × 256 / 64 either side.
    assert_eq!(g.cell(14, 15, Layer::Lower).roughness, 16);
    assert_eq!(g.cell(15, 15, Layer::Lower).roughness, 16);
    assert_eq!(g.cell(13, 15, Layer::Lower).roughness, 0);
}

#[test]
fn a_flat_6_m_floor_gets_900_points_10_cm_under_it() {
    // One cell floor, counter-clockwise from above.
    let (a, b, c, d) = (
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 6.0),
        Vec3::new(6.0, 0.0, 6.0),
        Vec3::new(6.0, 0.0, 0.0),
    );
    let points = floor_points([[a, b, c], [a, c, d]].into_iter());
    assert_eq!(points.len(), 900);
    assert!(points.iter().all(|p| (p.y - -0.10).abs() < 1e-9));
    assert!((points[0].x - 0.10).abs() < 1e-9 && (points[0].z - 0.10).abs() < 1e-9);
    // Turned down: nothing.
    assert!(floor_points([[a, c, b]].into_iter()).is_empty());
    // 13 cm up marks the band from 0 to 20 cm; 4 cm the one under 0.
    let lift = |h: f32| {
        let up = Vec3::Y * h;
        floor_points([[a + up, b + up, c + up]].into_iter())[0].y
    };
    assert!((lift(0.13) - 0.10).abs() < 1e-9);
    assert!((lift(0.04) - -0.10).abs() < 1e-9);
    // A 2 × 2 block whose columns fall in two bands gets no point: a ramp
    // 3 cm up at the first column, 9 cm at the second.
    let (p, q, r, s) = (
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 0.2),
        Vec3::new(0.2, 0.12, 0.2),
        Vec3::new(0.2, 0.12, 0.0),
    );
    assert!(floor_points([[p, q, r], [p, r, s]].into_iter()).is_empty());
    let flat = |h: f32| [p, q, r, s].map(|v| Vec3::new(v.x, h, v.z));
    let [p, q, r, s] = flat(0.09);
    assert_eq!(floor_points([[p, q, r], [p, r, s]].into_iter()).len(), 1);
}

#[test]
fn ground_tiles_sample_bilinearly_and_read_holes_a_byte_at_a_time() {
    let side = 241;
    let mut tile = GroundTile {
        x: 0,
        y: 0,
        corner: Vec2::ZERO,
        spacing: 0.2,
        side,
        heights: (0..side * side).map(|v| (v % side) as f32 * 0.01).collect(),
        holes: vec![false; side * side],
        blocks: GroundTile::ALL_BLOCKS,
    };
    assert!((tile.height(10.5, 3.0).unwrap() - 0.105).abs() < 1e-6);
    tile.holes[8] = true;
    // Vertex 8 starts the hole mask's second byte: the sample between 7
    // and 8 does not see it, the one between 8 and 9 does.
    assert!(tile.height(7.5, 0.5).is_some());
    assert!(tile.height(8.5, 0.5).is_none());
    tile.holes[side + 4] = true;
    assert!(tile.height(3.5, 0.5).is_none());
    assert!(tile.height(3.5, 1.5).is_none());
    assert!(tile.height(3.5, 2.5).is_some());
}

#[test]
fn ground_fills_only_the_blocks_that_exist() {
    // A 48 m tile centred on a 200 × 200 cell grid (its vertex 0 at cell
    // 20), its ground rising 5 mm a vertex along Z.
    let side = 241;
    let full = GroundTile {
        x: 0,
        y: 0,
        corner: Vec2::splat(-24.0),
        spacing: 0.2,
        side,
        heights: (0..side * side)
            .map(|v| (v / side) as f32 * 0.005)
            .collect(),
        holes: vec![false; side * side],
        blocks: GroundTile::ALL_BLOCKS,
    };
    let build = |ground: GroundTile| {
        NavSource {
            width: 200,
            depth: 200,
            base: grid_base(3.4),
            ground: vec![Arc::new(ground)],
            ..Default::default()
        }
        .build()
    };
    // Every block: 8 × 8 blocks of 20 × 20 cells.
    let all = build(full.clone());
    assert_eq!(all.stats().floors, [25_600, 0]);
    // Blocks (0, 0) and (5, 2) of a grid level: 400 cells each.
    let mut present = [0u8; 8];
    present[0] = 1;
    present[2] = 1 << 5;
    let some = full.with_blocks(present);
    assert!(some.has_block(0, 0) && some.has_block(5, 2) && !some.has_block(1, 0));
    let g = build(some);
    assert_eq!(g.stats().floors, [800, 0]);
    assert_eq!(g.cell(20, 20, Layer::Lower).floor, 221);
    assert!(!g.cell(40, 20, Layer::Lower).has_floor());
    // Block (5, 2) samples its cells where the whole tile does: cell (125,
    // 65) at vertex row 2·30 + 5.5·1.5 = 68.25, 34 cm up, voxel 224.
    assert_eq!(g.cell(125, 65, Layer::Lower).floor, 224);
    for z in 60..80 {
        for x in 120..140 {
            let (a, b) = (g.cell(x, z, Layer::Lower), all.cell(x, z, Layer::Lower));
            assert_eq!(a.floor, b.floor, "({x}, {z})");
        }
    }
}

#[test]
fn the_overlay_finds_the_floor_the_camera_looks_at() {
    let g = half_plane();
    let centre = g.cell_centre(15, 15);
    let down = view::floor_along(&g, Vec3::new(centre.x, 5.0, centre.y), Vec3::NEG_Y, 10.0);
    let hit = down.expect("floor under the ray");
    assert!(hit.y.abs() < 1e-4 && (hit.x - centre.x).abs() < 1e-4);
    // Over the void there is none.
    let void = g.cell_centre(5, 15);
    assert!(view::floor_along(&g, Vec3::new(void.x, 5.0, void.y), Vec3::NEG_Y, 10.0).is_none());
    // Colours: green when open, yellow at what passes, red beyond.
    assert_eq!(
        view::cell_colour(Layer::Lower, 0),
        bevy::color::Color::srgb(0.0, 1.0, 0.0)
    );
    assert_eq!(
        view::cell_colour(Layer::Lower, 180),
        bevy::color::Color::srgb(1.0, 1.0, 0.0)
    );
    assert_eq!(
        view::cell_colour(Layer::Lower, 181),
        bevy::color::Color::srgb(1.0, 0.0, 0.0)
    );
}

#[test]
fn grids_cover_the_tile_grid() {
    assert_eq!(grid_size(Some((32, 40))), (640, 800));
    assert_eq!(grid_size(Some((0, 40))), (320, 320));
    assert_eq!(grid_size(None), (320, 320));
    let g = NavGrid::empty(640, 800, grid_base(3.4));
    // Cell 320 starts at x = 0; the centre of cell 0 is 95.85 m to the west.
    assert_eq!(g.cell_at(Vec3::new(0.01, 0.0, 0.01)), Some((320, 400)));
    assert_eq!(g.cell_at(Vec3::new(-0.01, 0.0, -0.01)), Some((319, 399)));
    assert!((g.cell_centre(0, 0).x - -95.85).abs() < 1e-4);
    assert_eq!(g.cell_at(Vec3::new(96.1, 0.0, 0.0)), None);
    assert!((g.floor_height(221) - 0.0).abs() < 1e-5);
}

#[test]
fn a_floor_and_a_wall_from_their_points() {
    // A 6 m room's floor at 0 in a 2 × 2 room grid, and a wall of points
    // 3 m high along its west edge.
    let quad = FloorMesh {
        positions: vec![
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 6.0],
            [6.0, 0.0, 6.0],
            [6.0, 0.0, 0.0],
        ],
        indices: vec![0, 1, 2, 0, 2, 3],
        at: Affine3A::from_translation(Vec3::new(-3.0, 0.0, -3.0)),
    };
    let wall: Vec<Vec3> = (0..30)
        .flat_map(|k| {
            (0..15).map(move |j| Vec3::new(0.1, 0.1 + 0.2 * j as f32, 0.1 + 0.2 * k as f32))
        })
        .collect();
    let source = NavSource {
        width: 40,
        depth: 40,
        base: grid_base(3.4),
        meshes: vec![(
            std::sync::Arc::new(wall),
            Affine3A::from_translation(Vec3::new(-3.0, 0.0, -3.0)),
        )],
        floors: vec![quad],
        ground: Vec::new(),
    };
    let g = source.build();
    let s = g.stats();
    // The floor's 20 × 20 cells. The wall's points (every other voxel up
    // to 2.9 m, 20 cm apart) join its column into one span whose floor is
    // the wall's top, voxel 250: above voxel 240 (1.9 m), open sky.
    let (x, z) = g.cell_at(Vec3::new(0.0, 0.0, 0.0)).unwrap();
    assert_eq!(g.cell(x, z, Layer::Lower).floor, 221);
    let (wx, _) = g.cell_at(Vec3::new(-2.9, 0.0, 0.0)).unwrap();
    assert_eq!(g.cell(wx, z, Layer::Lower).floor, SKY);
    assert_eq!(g.cell(wx, z, Layer::Lower).clearance, NO_FLOOR_CLEARANCE);
    assert_eq!(s.columns_by_spans[1], 400);
    // Beside the wall: its column is a wall (8 × 256 / 64), and its open
    // sky's roughness 60 spreads into the blur: (32 + 21 + 30 + 21 + 16 +
    // 16) / 2.
    let beside = g.cell(wx + 1, z, Layer::Lower);
    assert_eq!(
        (
            beside.floor,
            beside.roughness,
            beside.blurred,
            beside.clearance
        ),
        (221, 32, 68, 68)
    );
}

/// The cells a walker of size class 0 reaches from `start` by steps of
/// less than 50 cm up and 2.1 m down onto passable floors (the planner's
/// step rules, `docs/NAVIGATION.md`), with 1 m of room under an upper
/// floor: for checking that rooms, doors and stairs join.
fn reach(g: &NavGrid, start: Vec3) -> std::collections::HashSet<(usize, usize, usize)> {
    let mut seen = std::collections::HashSet::new();
    let Some((x, z)) = g.cell_at(start) else {
        return seen;
    };
    let near = |x: usize, z: usize, y: f32| {
        Layer::BOTH
            .into_iter()
            .filter(|&l| g.cell(x, z, l).passable(0))
            .min_by(|&a, &b| {
                let h = |l| (g.floor_height(g.cell(x, z, l).floor) - y).abs();
                h(a).total_cmp(&h(b))
            })
    };
    let Some(layer) = near(x, z, start.y) else {
        return seen;
    };
    let mut todo = vec![(x, z, layer as usize)];
    seen.insert((x, z, layer as usize));
    let (w, d) = g.size();
    while let Some((x, z, l)) = todo.pop() {
        let f = i32::from(g.cell(x, z, Layer::BOTH[l]).floor);
        for (dx, dz, _) in NEIGHBOURS {
            let (nx, nz) = (x as i64 + i64::from(dx), z as i64 + i64::from(dz));
            if nx < 0 || nz < 0 || nx >= w as i64 || nz >= d as i64 {
                continue;
            }
            let (nx, nz) = (nx as usize, nz as usize);
            let upper = g.cell(nx, nz, Layer::Upper);
            for (k, layer) in Layer::BOTH.into_iter().enumerate() {
                let c = g.cell(nx, nz, layer);
                let step = i32::from(c.floor) - f;
                let room = layer == Layer::Upper
                    || upper.floor == 0
                    || i32::from(upper.underside) - f >= 10;
                if c.passable(0) && step <= 4 && step > -21 && room && seen.insert((nx, nz, k)) {
                    todo.push((nx, nz, k));
                }
            }
        }
    }
    seen
}

/// Cells with a floor on either layer whose 6 m tile-grid room is closed
/// (the rock between rooms), in a grid of 20 cells a room.
fn floors_over_closed_rooms(g: &NavGrid, grid: &exa_formats::level::LevelGrid) -> usize {
    let (w, d) = g.size();
    (0..d)
        .flat_map(|z| (0..w).map(move |x| (x, z)))
        .filter(|&(x, z)| {
            let room = grid.cell((x / CELLS_PER_ROOM) as u32, (z / CELLS_PER_ROOM) as u32);
            room.is_some_and(|c| grid.edges[c] & CELL_OPEN == 0)
                && Layer::BOTH.iter().any(|&l| g.cell(x, z, l).floor != 0)
        })
        .count()
}

/// The installed game's levels (with `EXANIMA_DIR`), built as the game
/// builds them: sizes, spans, floors, open cells, and how far a walker gets
/// from the level's start (in exanima01, down the stair room to its floor
/// a metre lower). Terrain fills only the blocks the grid flags: 34 in
/// arenao1, 682 in exanima01, none over its closed rooms.
#[test]
fn shipped_levels() {
    let Some(dir) = exa_assets::locate_install(None) else {
        return;
    };
    let game = exa_assets::GameData::open(&dir).unwrap();
    let cases = [
        (
            "arena",
            NavStats {
                width: 640,
                depth: 640,
                columns_by_spans: [407_126, 2474, 0, 0],
                floors: [1941, 0],
                passable: [1941, 0],
                sky: [533, 0],
            },
            1860,
            0,
        ),
        (
            "arenao1",
            NavStats {
                width: 640,
                depth: 640,
                columns_by_spans: [392_042, 13_612, 3946, 0],
                floors: [14_684, 3676],
                passable: [14_673, 3676],
                sky: [2874, 270],
            },
            5006,
            34,
        ),
        (
            "exanima01",
            NavStats {
                width: 640,
                depth: 800,
                columns_by_spans: [190_921, 320_149, 930, 0],
                floors: [219_539, 825],
                passable: [219_364, 825],
                sky: [101_540, 105],
            },
            // The grid's special tiles now mask their edges in every
            // rotation, as the original's start-up table does.
            191_331,
            682,
        ),
    ];
    for (name, stats, reached, blocks) in cases {
        let level = headless::level_source(&game, name).expect("level loads");
        let present: u32 = level
            .source
            .ground
            .iter()
            .flat_map(|t| t.blocks)
            .map(u8::count_ones)
            .sum();
        assert_eq!(present, blocks, "{name}");
        let g = level.build();
        assert_eq!(g.stats(), stats, "{name}");
        let start = crate::start::player_start(name).unwrap().position;
        let r = reach(&g, start);
        assert_eq!(r.len(), reached, "{name}");
        let reaches = |p: Vec3| {
            let (x, z) = g.cell_at(p).unwrap();
            r.iter().any(|&(rx, rz, _)| (rx, rz) == (x, z))
        };
        // A long plan, as an NPC's: to the reached cell farthest from the
        // start within the planner's 253 cells, timed (it must stay cheap
        // enough for one a tick).
        let (sx, sz) = g.cell_at(start).unwrap();
        let far = r
            .iter()
            .filter(|&&(x, z, _)| x.abs_diff(sx) <= 240 && z.abs_diff(sz) <= 240)
            .max_by_key(|&&(x, z, _)| (x.abs_diff(sx).pow(2) + z.abs_diff(sz).pow(2), x, z))
            .copied()
            .unwrap();
        let layer = if far.2 == 0 {
            Layer::Lower
        } else {
            Layer::Upper
        };
        let centre = g.cell_centre(far.0, far.1);
        let goal = Vec3::new(
            centre.x,
            g.floor_height(g.cell(far.0, far.1, layer).floor),
            centre.y,
        );
        let clock = std::time::Instant::now();
        let plan = super::find_path_with(&g, start, goal, 3, super::ACCEPT_GOAL_CELL, 1);
        let took = clock.elapsed();
        eprintln!(
            "{name}: plan of {:.1} m: {} points, cost {}, reached {}, {:?}",
            start.distance(goal),
            plan.points.len(),
            plan.cost,
            plan.reached,
            took
        );
        assert!(plan.points.len() >= 2, "{name}: {plan:?}");
        assert!(
            took < std::time::Duration::from_millis(250),
            "{name}: {took:?}"
        );
        match name {
            "arena" => assert!(reaches(Vec3::new(0.0, 0.0, -5.0))),
            "exanima01" => {
                // The stair room: its foot a metre under the corridor.
                let (x, z) = g.cell_at(Vec3::new(8.9, -1.0, 64.0)).unwrap();
                assert_eq!(g.cell(x, z, Layer::Lower).floor, 211);
                assert!(reaches(Vec3::new(8.9, -1.0, 64.0)));
                // Over its 477 closed rooms only the walls' edges have floors
                // (1555 cells); filling whole tiles, the ground between the
                // rooms made 186 721 and the walk reached 338 602 cells.
                let pack = game.pack(exa_assets::PackKind::Resource).unwrap();
                let scene = exa_formats::rfc::Scene::parse(pack.get("exanima01.rfc").unwrap());
                let grid = scene.unwrap().grid.unwrap();
                let closed = grid.edges.iter();
                assert_eq!(closed.filter(|&&e| e & CELL_OPEN == 0).count(), 477);
                assert_eq!(floors_over_closed_rooms(&g, &grid), 1555);
                let mut whole = level.source.clone();
                for tile in &mut whole.ground {
                    *tile = Arc::new((**tile).clone().with_blocks(GroundTile::ALL_BLOCKS));
                }
                let g = whole.build();
                assert_eq!(floors_over_closed_rooms(&g, &grid), 186_721);
                assert_eq!(reach(&g, start).len(), 338_602);
            }
            _ => {}
        }
    }
}

#[test]
fn the_pathfinder_stores_its_opening_metrics() {
    let metrics = pathfinder_metrics(7);
    assert_eq!(metrics.cell, 30.0);
    assert_eq!(metrics.depth, 10.0);
    assert_eq!(metrics.inverse_cell, 1.0 / 30.0);
    assert_eq!(metrics.seed, 7);
    assert_eq!(metrics.counter, 0);
    assert_eq!(metrics.base_calls, 1);
}

#[test]
fn the_footprint_extents_advance_and_clamp() {
    let extents = path_extents(PathExtentInput {
        slot: 15,
        inverse_cell: 1.0,
        axes: [10, 10],
        local: [2.0, 0.0, 4.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        extents: [1.0, 1.0, 1.0],
    });
    assert_eq!(extents.slot, 0);
    assert_eq!(extents.mins, [5, 5]);
    assert_eq!(extents.maxs, [6, 6]);
}

#[test]
fn a_penalty_stamp_raises_and_lowers_both_layers() {
    let mut grid = NavGrid::empty(6, 6, 0.0);
    assert!(grid.raise_penalty(2, 3, 255));
    for layer in Layer::BOTH {
        assert_eq!(grid.cell(2, 3, layer).penalty, 255);
    }
    assert_eq!(grid.cell(2, 2, Layer::Lower).penalty, 0);
    assert!(!grid.raise_penalty(-1, 0, 1));
    assert!(!grid.raise_penalty(6, 0, 1));
    assert!(grid.lower_penalty(2, 3, 10));
    assert_eq!(grid.cell(2, 3, Layer::Upper).penalty, 245);
    assert!(grid.lower_penalty(2, 3, 255));
    assert_eq!(grid.cell(2, 3, Layer::Lower).penalty, 0);
}
