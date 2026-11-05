//! The point the original's per-object `VXGIPos` lookup takes: the node's
//! world bounding-sphere centre (the same sphere the visibility test reads),
//! not its origin. The lit-material writer passes that centre to the GI
//! manager's lookup, which floors it into the 48 m block grid and binds the
//! block's light texture and offset (`crate::gi::GiPick`).
//!
//! Exarust finds each surface's world bounds centre (its mesh bounds' centre
//! through its transform), floors it into the block grid, and carries the
//! cell to the shader in the mesh tag's top twelve bits ([`GiCellTag`]):
//! bits 20–25 hold `x + 1`, bits 26–31 `z + 1`; zero means no cell yet, and
//! the shader then falls back to the mesh's origin. A cell off the 0–61
//! range is stored as 63, which no resident block holds, so it reads no
//! light, as the original's lookup off the grid binds texture 0. Rules:
//! `docs/RENDERING.md` (Voxel GI, per-object pick).

use bevy::camera::primitives::Aabb;
use bevy::prelude::*;

use crate::gi::{GiPick, GiTextures};
use crate::material::ExaMaterial;

/// The tag bits below the GI cell: the shadow-bias level and the light-list
/// slot ([`crate::lights`]).
pub const LOW_BITS: u32 = 20;
/// The largest stored axis value (`x + 1` or `z + 1`); [`OFF_GRID`] marks a
/// cell outside it.
pub const MOST_AXIS: i32 = 62;
pub const OFF_GRID: u32 = 63;

/// A surface's GI block cell, packed for the mesh tag's top bits.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GiCellTag(pub u32);

/// The tag bits for block-grid cell `cell`.
pub fn pack(cell: IVec2) -> u32 {
    let axis = |v: i32| {
        if (0..MOST_AXIS).contains(&v) {
            (v + 1) as u32
        } else {
            OFF_GRID
        }
    };
    (axis(cell.x) | axis(cell.y) << 6) << LOW_BITS
}

/// The cell a tag carries; `None` when it carries none.
pub fn unpack(tag: u32) -> Option<IVec2> {
    let bits = tag >> LOW_BITS;
    if bits == 0 {
        return None;
    }
    Some(IVec2::new(
        (bits & 63) as i32 - 1,
        (bits >> 6 & 63) as i32 - 1,
    ))
}

/// A surface's world bounds centre, metres: its mesh bounds' centre through
/// its transform.
pub fn bounds_centre(transform: &GlobalTransform, aabb: &Aabb) -> Vec3 {
    transform.transform_point(Vec3::from(aabb.center))
}

pub struct GiTagPlugin;

impl Plugin for GiTagPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, tag_cells.after(TransformSystems::Propagate));
    }
}

type Moved = Or<(Changed<GlobalTransform>, Changed<Aabb>, Added<Aabb>)>;
type Surface = With<MeshMaterial3d<ExaMaterial>>;
type Bounds = (
    Entity,
    &'static GlobalTransform,
    &'static Aabb,
    Option<&'static GiCellTag>,
);

/// Gives surfaces that moved (or all of them, when the block grid changes)
/// the cell their bounds centre falls in. Only a changed value is written,
/// so a still surface never touches its tag.
fn tag_cells(
    mut commands: Commands,
    textures: Option<Res<GiTextures>>,
    mut grid: Local<Option<Vec2>>,
    moved: Query<Bounds, (Surface, Moved)>,
    all: Query<Bounds, Surface>,
) {
    let origin = textures.and_then(|t| t.pick).map(|p| p.cells_origin_cm);
    let mut write = |e: Entity, t: &GlobalTransform, b: &Aabb, old: Option<&GiCellTag>| {
        let new = origin.map_or(GiCellTag(0), |o| {
            GiCellTag(pack(GiPick::cell_of(o, bounds_centre(t, b))))
        });
        if old != Some(&new) {
            commands.entity(e).insert(new);
        }
    };
    if *grid != origin {
        *grid = origin;
        let mut moved_block = 0;
        for (e, t, b, old) in &all {
            write(e, t, b, old);
            if let Some(o) = origin
                && GiPick::cell_of(o, bounds_centre(t, b)) != GiPick::cell_of(o, t.translation())
            {
                moved_block += 1;
            }
        }
        debug!(
            "GI cells: {} surfaces, {moved_block} in another block than their origin's",
            all.iter().len()
        );
        return;
    }
    if origin.is_none() {
        return;
    }
    for (e, t, b, old) in &moved {
        write(e, t, b, old);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cells_round_trip_through_the_tag() {
        for cell in [IVec2::ZERO, IVec2::new(3, 7), IVec2::new(61, 0)] {
            let tag = pack(cell);
            assert_eq!(tag & ((1 << LOW_BITS) - 1), 0);
            assert_eq!(unpack(tag), Some(cell));
        }
        assert_eq!(unpack(0), None);
        // Low bits (bias, light slot) do not read as a cell.
        assert_eq!(unpack(0xF_FFFF), None);
    }

    #[test]
    fn a_cell_off_the_grid_matches_no_block() {
        assert_eq!(unpack(pack(IVec2::new(-1, 2))), Some(IVec2::new(62, 2)));
        assert_eq!(unpack(pack(IVec2::new(4, 62))), Some(IVec2::new(4, 62)));
    }

    #[test]
    fn the_centre_follows_the_bounds_not_the_origin() {
        // A wall whose mesh sits 30 m from its node's origin.
        let t = GlobalTransform::from_translation(Vec3::new(40.0, 0.0, 0.0));
        let b = Aabb::from_min_max(Vec3::new(5.0, 0.0, -1.0), Vec3::new(15.0, 3.0, 1.0));
        let c = bounds_centre(&t, &b);
        assert_eq!(c, Vec3::new(50.0, 1.5, 0.0));
        // Origin at 40 m picks block 0; the centre at 50 m picks block 1.
        assert_eq!(GiPick::cell_of(Vec2::ZERO, t.translation()).x, 0);
        assert_eq!(GiPick::cell_of(Vec2::ZERO, c).x, 1);
    }

    #[test]
    fn the_shader_decodes_the_same_bits() {
        let src = include_str!("shading.wesl");
        assert!(src.contains("const GI_TAG_SHIFT: u32 = 20u;"));
        assert!(src.contains("const LIGHT_SLOT_MASK: u32 = 131071u;"));
    }
}
