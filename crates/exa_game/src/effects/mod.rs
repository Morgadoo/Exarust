//! The original's small effect nodes.
//!
//! Each module reproduces one owner from the original: the level portal's
//! sparks and the dissolve burst of a body stepping through one
//! (`portal`, and the portal's spray, `portal_effect`), the flame drawn
//! along a lit blade (`fire_row`) and its node (`fire_node`), the
//! displacement shell (`shell`), the teleport bubble (`bubble`), the
//! teleporter scan (`scan`), the domain wheel of the power screen
//! (`wheel`), the rain (`rain`), the force domes (`force_dome`), the Force
//! Wave (`force_wave`), the vanish (`vanish`), the GI emissives the force
//! effects carry (`emissive`), the force geometries' programs and pass rows
//! (`effect_geometry`), what the AI sees of a blast, a wave or a
//! barricade (`danger`), and the force blast and torrent with their ribbons
//! and GI lights (`blast`, `torrent`).
//!
//! The rules are plain data and functions so they can be tested without a
//! Bevy app. The Bevy systems step components that gameplay inserts, on
//! the original's 83 Hz tick. The one effect spawned here is a level
//! portal's light, on the marker node a loaded model carries
//! ([`exa_world::PortalMarker`], `uwportal01`), and the reconstruction
//! device puts a scan on the player ([`crate::teleporters`]) and the
//! destination screen's journey a bubble ([`crate::destination`]); the
//! others wait for the systems that start them in the original (the
//! powers, the power tree, the scene's weather and a burning item's
//! ability record), which Exarust does not have yet.

pub mod blast;
pub mod bubble;
pub mod danger;
pub mod draw;
pub mod effect_geometry;
pub mod emissive;
pub mod fire_node;
pub mod fire_row;
pub mod force_dome;
pub mod force_wave;
pub mod portal;
pub mod portal_effect;
pub mod rain;
pub mod scan;
pub mod shell;
pub mod torrent;
pub mod vanish;
pub mod wheel;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

/// The original's simulation rate, ticks per second.
pub const TICK_HZ: u32 = 83;

/// The effects' clock: the original's game clock, whole milliseconds of
/// game time that stop while the game is paused, as an unsigned word that
/// wraps. Effects count their phases in it. It follows the fixed clock, so
/// a tick moves it by 12 or 13 ms and it never drifts from game time.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EffectClock {
    pub now_ms: u32,
}

impl EffectClock {
    /// The clock at `elapsed` game time: its whole milliseconds, wrapped
    /// to the original's 32-bit word.
    pub fn at(elapsed: std::time::Duration) -> Self {
        Self {
            now_ms: elapsed.as_millis() as u32,
        }
    }
}

/// A 3 × 4 affine frame: its three basis columns, then the translation, in
/// the original's centimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub x: Vec3,
    pub y: Vec3,
    pub z: Vec3,
    pub at: Vec3,
}

impl Frame {
    pub const IDENTITY: Self = Self {
        x: Vec3::X,
        y: Vec3::Y,
        z: Vec3::Z,
        at: Vec3::ZERO,
    };

    /// A point in the frame's own space, in its parent's. The sums pair as
    /// the original's: the first two columns, then the third with the
    /// translation, then the two halves.
    pub fn point(&self, point: Vec3) -> Vec3 {
        (self.x * point.x + self.y * point.y) + (self.z * point.z + self.at)
    }

    /// A direction in the frame's own space, in its parent's: the basis
    /// alone, without the translation.
    pub fn direction(&self, direction: Vec3) -> Vec3 {
        (self.x * direction.x + self.y * direction.y) + self.z * direction.z
    }

    /// A frame whose parent space is this frame's space. (The burst's
    /// callers compose a node's frame with its inverse rest frame; the
    /// original's own composition is outside this group and its exact
    /// rounding is not reproduced.)
    pub fn compose(&self, child: &Self) -> Self {
        Self {
            x: self.direction(child.x),
            y: self.direction(child.y),
            z: self.direction(child.z),
            at: self.point(child.at),
        }
    }
}

pub struct EffectsPlugin;

impl Plugin for EffectsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EffectClock>().add_systems(
            FixedUpdate,
            (
                advance_clock,
                (
                    bubble::tick_bubbles,
                    scan::tick_scans,
                    shell::tick_shells,
                    fire_row::tick_rows,
                ),
            )
                .chain(),
        );
        app.add_systems(Update, (fire_row::frame_rows, portal::light_portals));
    }
}

pub(crate) fn advance_clock(time: Res<Time>, mut clock: ResMut<EffectClock>) {
    *clock = EffectClock::at(time.elapsed());
}

/// A ribbon as a mesh for the glow material: every point gets a vertex on
/// each side of `side`, the samples join into a triangle list, and each
/// vertex carries the effect's linear colour (the glow material multiplies
/// it by its tint). Points are in the original's centimetres, so the caller
/// converts them to metres before drawing.
pub fn ribbon_mesh(points: &[[f32; 3]], side: [[f32; 3]; 2], colour: [f32; 4]) -> Mesh {
    let mut positions = Vec::with_capacity(points.len() * 2);
    let mut colours = Vec::with_capacity(points.len() * 2);
    for point in points {
        for offset in side {
            positions.push([
                point[0] + offset[0],
                point[1] + offset[1],
                point[2] + offset[2],
            ]);
            colours.push(colour);
        }
    }
    let mut indices = Vec::new();
    for sample in 0..points.len().saturating_sub(1) {
        let a = (sample * 2) as u32;
        let b = a + 1;
        let c = a + 2;
        let d = a + 3;
        indices.extend_from_slice(&[a, b, c, b, d, c]);
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colours);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// The original's float-to-integer conversion in these effects: the
/// processor's default rounding, ties to the even integer.
pub(crate) fn round_i32(value: f32) -> i32 {
    value.round_ties_even() as i32
}

/// A millisecond difference as the original takes it: both words widened
/// to 64 bits before the subtraction, so a stamp ahead of the clock gives a
/// negative span instead of wrapping.
pub(crate) fn span_ms(now_ms: u32, stamp: u32) -> i64 {
    i64::from(now_ms) - i64::from(stamp)
}

/// A squared distance, summed as the original's helper sums it.
pub(crate) fn distance_squared(a: Vec3, b: Vec3) -> f32 {
    let d = a - b;
    (d.x * d.x + d.y * d.y) + d.z * d.z
}

/// The mesh level of detail an effect stores: `base` over the squared
/// camera distance, which is raised to one when it is below one (a NaN
/// stays).
pub(crate) fn level_of_detail(base: f32, camera: Vec3, at: Vec3) -> f32 {
    let d = distance_squared(camera, at);
    base / if d < 1.0 { 1.0 } else { d }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_clock_is_whole_milliseconds_of_game_time() {
        use std::time::Duration;
        // Two 83 Hz ticks are 24.096 ms: the clock reads 24, not 2 × 12.
        let two_ticks = Duration::from_secs_f64(2.0 / f64::from(TICK_HZ));
        assert_eq!(EffectClock::at(two_ticks).now_ms, 24);
        // 83 ticks are one second.
        let second = Duration::from_secs_f64(83.0 / f64::from(TICK_HZ));
        assert_eq!(EffectClock::at(second).now_ms, 1_000);
        // The word wraps after 2³² ms.
        let wrapped = Duration::from_millis(u64::from(u32::MAX) + 7);
        assert_eq!(EffectClock::at(wrapped).now_ms, 6);
    }

    #[test]
    fn a_span_is_signed_and_a_detail_level_floors_its_distance() {
        assert_eq!(span_ms(100, 40), 60);
        assert_eq!(span_ms(40, 100), -60);
        assert_eq!(span_ms(0, u32::MAX), -i64::from(u32::MAX));
        assert_eq!(
            level_of_detail(900.0, Vec3::ZERO, Vec3::new(0.5, 0.0, 0.0)),
            900.0
        );
        assert_eq!(
            level_of_detail(900.0, Vec3::ZERO, Vec3::new(3.0, 0.0, 0.0)),
            100.0
        );
        assert_eq!(
            level_of_detail(200.0, Vec3::new(1.0, 2.0, 2.0), Vec3::ZERO),
            200.0 / 9.0
        );
        assert!(level_of_detail(200.0, Vec3::splat(f32::NAN), Vec3::ZERO).is_nan());
    }

    #[test]
    fn a_frame_composes_and_maps_points() {
        let moved = Frame {
            at: Vec3::new(10.0, 0.0, 0.0),
            ..Frame::IDENTITY
        };
        assert_eq!(
            moved.point(Vec3::new(1.0, 2.0, 3.0)),
            Vec3::new(11.0, 2.0, 3.0)
        );
        let turned = Frame {
            x: Vec3::Y,
            y: -Vec3::X,
            z: Vec3::Z,
            at: Vec3::ZERO,
        };
        let child = moved.compose(&turned);
        assert_eq!(
            child.point(Vec3::new(1.0, 0.0, 0.0)),
            Vec3::new(10.0, 1.0, 0.0)
        );
    }

    #[test]
    fn a_ribbon_makes_two_triangles_a_segment() {
        let mesh = ribbon_mesh(
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]],
            [[0.0, 0.5, 0.0], [0.0, -0.5, 0.0]],
            [1.0, 0.0, 0.0, 1.0],
        );
        assert_eq!(mesh.count_vertices(), 6);
        let Some(Indices::U32(indices)) = mesh.indices() else {
            panic!("32-bit indices");
        };
        assert_eq!(indices.len(), 12);
    }

    #[test]
    fn a_round_sends_ties_to_even() {
        assert_eq!(round_i32(2.5), 2);
        assert_eq!(round_i32(3.5), 4);
        assert_eq!(round_i32(-2.5), -2);
        assert_eq!(round_i32(432.64), 433);
    }

    #[test]
    fn a_point_pairs_its_sums_as_the_original() {
        // In single precision 1e8 + 1 is 1e8: paired, the translation is
        // lost in the second half (0); summed left to right it would
        // survive (1).
        let frame = Frame {
            x: Vec3::new(1.0e8, 0.0, 0.0),
            y: Vec3::ZERO,
            z: Vec3::new(-1.0e8, 0.0, 0.0),
            at: Vec3::new(1.0, 0.0, 0.0),
        };
        assert_eq!(frame.point(Vec3::ONE).x, 0.0);
    }
}
