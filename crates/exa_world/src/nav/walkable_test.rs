use bevy::math::Vec3;

use super::*;
use crate::nav::source::grid_base;
use crate::nav::{Columns, Layer, NavCell, NavGrid, SKY};

/// Voxel 221 tops out at 0 in a grid whose top is 3.4 m.
const GROUND: u8 = 221;

fn grid(width: usize, depth: usize) -> NavGrid {
    NavGrid::empty(width, depth, grid_base(3.4))
}

/// A `width` × `depth` grid whose every cell has a lower floor at 0 (slab
/// from the bottom).
fn flat(width: usize, depth: usize) -> NavGrid {
    let mut g = grid(width, depth);
    for z in 0..depth {
        for x in 0..width {
            put(&mut g, x, z, Layer::Lower, GROUND, 0);
        }
    }
    g
}

fn put(g: &mut NavGrid, x: usize, z: usize, layer: Layer, floor: u8, underside: u8) {
    let c = g.cell(x, z, layer);
    g.set_cell(
        x,
        z,
        layer,
        NavCell {
            floor,
            underside,
            ..c
        },
    );
}

fn cost(g: &mut NavGrid, x: usize, z: usize, layer: Layer, clearance: u8, penalty: u8) {
    let c = g.cell(x, z, layer);
    g.set_cell(
        x,
        z,
        layer,
        NavCell {
            clearance,
            penalty,
            ..c
        },
    );
}

/// A point in centimetres, as metres.
fn at(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z) / 100.0
}

fn close(a: Vec3, b: Vec3) -> bool {
    a.distance(b) < 1e-4
}

/// The cost bytes over 255 of the layer a point's height meets: 132 and 16
/// on the ground; with an upper floor 1.5 m up, its 60 for a point on it.
#[test]
fn the_cost_of_a_point_by_its_layer() {
    let mut g = flat(40, 40);
    cost(&mut g, 20, 20, Layer::Lower, 132, 16);
    let (clearance, penalty) = cost_at(&g, at(10.0, 0.0, 10.0));
    assert!((clearance - 132.0 / 255.0).abs() < 1e-7);
    assert!((penalty - 16.0 / 255.0).abs() < 1e-7);
    put(&mut g, 20, 20, Layer::Upper, GROUND + 15, GROUND + 13);
    cost(&mut g, 20, 20, Layer::Upper, 60, 0);
    assert!((cost_at(&g, at(10.0, 150.0, 10.0)).0 - 60.0 / 255.0).abs() < 1e-7);
    assert!((cost_at(&g, at(10.0, 0.0, 10.0)).0 - 132.0 / 255.0).abs() < 1e-7);
}

/// The layer pick: the nearer floor, the upper one with under 110 cm
/// between the ground and its slab; open sky over the ground is no upper
/// floor, open sky alone no floor.
#[test]
fn a_point_takes_the_nearer_layer_unless_one_has_no_room() {
    let mut g = flat(4, 4);
    assert_eq!(layer_at(&g, 1, 1, 300), Some(Layer::Lower));
    put(&mut g, 1, 1, Layer::Upper, GROUND + 19, GROUND + 17);
    assert_eq!(
        layer_at(&g, 1, 1, i32::from(GROUND) + 2),
        Some(Layer::Lower)
    );
    assert_eq!(
        layer_at(&g, 1, 1, i32::from(GROUND) + 20),
        Some(Layer::Upper)
    );
    put(&mut g, 1, 1, Layer::Upper, GROUND + 12, GROUND + 10);
    assert_eq!(layer_at(&g, 1, 1, i32::from(GROUND)), Some(Layer::Upper));
    put(&mut g, 1, 1, Layer::Upper, SKY, GROUND + 10);
    assert_eq!(layer_at(&g, 1, 1, i32::from(GROUND)), Some(Layer::Lower));
    put(&mut g, 1, 1, Layer::Upper, 0, 0);
    put(&mut g, 1, 1, Layer::Lower, SKY, 0);
    assert_eq!(layer_at(&g, 1, 1, i32::from(GROUND)), None);
    put(&mut g, 1, 1, Layer::Lower, 0, 0);
    assert_eq!(layer_at(&g, 1, 1, i32::from(GROUND)), None);
}

/// Cells x ≥ 10 of a 30 × 30 grid have a floor at 0; the edge column reads
/// clearance 132, the next 60.
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
fn a_walkable_point_stays_and_takes_its_floor() {
    let g = half_plane();
    let p = nearest_walkable(&g, at(160.0, 37.0, 20.0));
    assert!(close(p, at(160.0, 0.0, 20.0)), "{:?}", p * 100.0);
}

/// Cell (10, 15), clearance 132, is not walkable: of its neighbours at 60,
/// the first met in the original's order, (11, 14), wins.
#[test]
fn an_edge_point_moves_to_the_first_cheaper_neighbour() {
    let g = half_plane();
    let p = nearest_walkable(&g, at(-135.0, 0.0, 15.0));
    assert!(close(p, at(-105.0, 0.0, -15.0)), "{:?}", p * 100.0);
}

/// Cell (5, 15) has no floor: the original takes the nearest cell across
/// the ground, a lower layer without a floor included, (5, 14), and floods
/// from it, reaching nothing: its centre at the grid's base.
#[test]
fn a_point_off_the_floors_takes_the_nearest_cell_as_the_original() {
    let g = half_plane();
    let p = nearest_walkable(&g, at(-285.0, 0.0, 15.0));
    assert!(close(p, at(-285.0, -2210.0, -15.0)), "{:?}", p * 100.0);
}

/// Every cell costs 100 but one walkable cell 9 cells away: past the
/// flood's 220 cells the point lands on its own cell's centre; 5 cells
/// away the walkable one is found.
#[test]
fn the_flood_stops_after_220_cells() {
    let mut g = flat(60, 60);
    for z in 0..60 {
        for x in 0..60 {
            cost(&mut g, x, z, Layer::Lower, 100, 0);
        }
    }
    cost(&mut g, 39, 30, Layer::Lower, 0, 0);
    let p = nearest_walkable(&g, at(10.0, 0.0, 10.0));
    assert!(close(p, at(15.0, 0.0, 15.0)), "{:?}", p * 100.0);
    cost(&mut g, 35, 30, Layer::Lower, 0, 0);
    let p = nearest_walkable(&g, at(10.0, 0.0, 10.0));
    assert!(close(p, at(165.0, 0.0, 15.0)), "{:?}", p * 100.0);
}

/// The point's cell costs 100; its east neighbour is walkable but 50 cm
/// up: not stepped onto; 40 cm up it is.
#[test]
fn the_flood_steps_as_the_planner() {
    let mut g = flat(60, 60);
    for z in 0..60 {
        for x in 0..60 {
            cost(&mut g, x, z, Layer::Lower, 100, 0);
        }
    }
    cost(&mut g, 31, 30, Layer::Lower, 0, 0);
    put(&mut g, 31, 30, Layer::Lower, GROUND + 5, 0);
    let p = nearest_walkable(&g, at(10.0, 0.0, 10.0));
    assert!(!close(p, at(45.0, 50.0, 15.0)), "{:?}", p * 100.0);
    put(&mut g, 31, 30, Layer::Lower, GROUND + 4, 0);
    let p = nearest_walkable(&g, at(10.0, 0.0, 10.0));
    assert!(close(p, at(45.0, 40.0, 15.0)), "{:?}", p * 100.0);
}
