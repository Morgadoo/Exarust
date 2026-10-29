//! The original's terrain streaming decisions: where the streamed tiles are
//! gathered from, which 6 m blocks get a composite and at what resolution,
//! and the priority each block's look-ahead work is ranked by.
//!
//! Exarust's loader builds and inserts every tile behind its loading screen,
//! as the original's "block until every tile is loaded" does, and then
//! evaluates the composite per pixel. The decisions below are therefore
//! models of the original's rules rather than the driver of a live
//! streamer; they reproduce its numbers.
//!
//! Evidence: REA on the terrain's priority slot, its block selection and
//! the 3 × 3 gather: the composites' 44.4 m / 15 m cut-offs as squared
//! centimetres, the 189-entry list, and the pools of 21 high-res and 43
//! low-res texture sets.

use bevy::math::{Mat3, Vec2, Vec3};
use exa_physics::engine_math::round_to_i64;

use super::composite::{LIGHT_SORT_BIAS, NO_LIGHT_DISTANCE};

/// Distance (metres) within which a block gets a high-resolution composite:
/// the original compares the squared distance in centimetres against
/// 2 250 000 (15 m).
pub const NEAR: f32 = 15.0;
/// Distance (metres) within which a block gets any composite: squared
/// centimetres at most 19 713 600.
pub const FAR: f32 = 44.4;
/// High-resolution composite texture sets in the pool.
pub const HIGH_SETS: usize = 21;
/// Low-resolution composite texture sets in the pool.
pub const LOW_SETS: usize = 43;
/// Blocks the original keeps in its selected list (189 entries).
pub const MAX_SELECTED: usize = 189;
/// Block side, centimetres: a block's centre falls on a 6 m grid.
const BLOCK_CM: i64 = 600;
/// The original's reciprocal of the block side, 1/600 cm (its literal
/// single-precision constant).
const INV_BLOCK_CM: f32 = 0.001_666_666_7;
/// The original moves the camera −1000 cm along its back axis, 10 m ahead.
const LOOK_AHEAD_CM: f32 = -1000.0;

/// The point whose blocks get composites: the camera position (`camera`,
/// metres, world X and Z) moved −10 m along the camera's back axis
/// (`back`, its local +Z in world X and Z, unnormalised in the plane), so
/// 10 m ahead of it, and snapped to the centre of its 6 m block. In the
/// original's centimetres each axis is the engine floor of `p · 1/600`,
/// times 600, plus 300.
pub fn composite_focus(camera: Vec2, back: Vec2) -> Vec2 {
    let p = camera * 100.0 + back * LOOK_AHEAD_CM;
    let snap = |v: f32| (i64::from(round_down(v * INV_BLOCK_CM)) * BLOCK_CM + BLOCK_CM / 2) as f32;
    Vec2::new(snap(p.x), snap(p.y)) / 100.0
}

/// The engine's "round" helper: a conversion with the rounding mode set
/// toward minus infinity; NaN and out-of-range values give the integer
/// indefinite (`i32::MIN`).
fn round_down(value: f32) -> i32 {
    let r = value.floor();
    if r.is_nan() || !(-2_147_483_648.0..2_147_483_648.0).contains(&r) {
        i32::MIN
    } else {
        r as i32
    }
}

/// One block that would be composited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompositeBlock {
    /// Index in the list the caller passed.
    pub index: usize,
    /// High resolution (`d² < 2 250 000 cm²`); otherwise low.
    pub high: bool,
}

/// Whether a block's squared distance from the focus (squared centimetres)
/// is within the composite reach. The original's compare against the far
/// cut-off jumps on parity to the same keep branch as a distance at or below
/// it; only an ordered distance above the cut-off falls through to the
/// removal, so a NaN block is kept (at low resolution).
pub fn within_reach(d2: f32) -> bool {
    d2 <= super::composite::FAR_D2 || d2.is_nan()
}

/// Whether a block at squared distance `d2` takes the high-resolution
/// composite. The original compares against the near cut-off and jumps on
/// parity to the low branch, so a NaN distance is low; a block exactly
/// 15 m away is low too.
pub fn high_resolution(d2: f32) -> bool {
    !(d2.is_nan() || d2 >= super::composite::NEAR_D2)
}

/// The blocks within [`FAR`] of `focus`, in the caller's order, at most
/// [`MAX_SELECTED`]. Distances are squared centimetres, as the original
/// compares them; a block exactly 15 m away is low resolution and a NaN
/// distance is kept at low resolution (its unordered compares take the keep
/// and the low branches).
pub fn select_composites(
    focus: Vec2,
    centres: impl IntoIterator<Item = Vec2>,
) -> Vec<CompositeBlock> {
    let mut out = Vec::new();
    for (index, centre) in centres.into_iter().enumerate() {
        if out.len() >= MAX_SELECTED {
            break;
        }
        let (dx, dz) = ((centre.x - focus.x) * 100.0, (centre.y - focus.y) * 100.0);
        let d2 = dz * dz + dx * dx;
        if within_reach(d2) {
            out.push(CompositeBlock {
                index,
                high: high_resolution(d2),
            });
        }
    }
    out
}

/// Where a block's look-ahead work ranks: its distance rank and the priority
/// the original writes for it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlockPriority {
    /// The block's centre, metres (world X and Z).
    pub centre: Vec2,
    /// `round(6000 − distance)` in centimetres while the block is within
    /// 60 m, else 0.
    pub distance: i16,
    /// The block's priority.
    pub rank: u32,
    /// The block still needs its composite drawn (the selection sets this
    /// when a block turns high resolution).
    pub wants: bool,
    /// The block's scene node is visible this frame.
    pub visible: bool,
}

/// The distance rank of a block: `round(6000 − d)` centimetres for a block
/// within 60 m, 0 otherwise. The distance is the square root of
/// `dz² + dx²`, and the rounding is the hardware's round-to-nearest-even.
pub fn distance_rank(centre: Vec2, focus: Vec2) -> i16 {
    let (dx, dz) = ((focus.x - centre.x) * 100.0, (focus.y - centre.y) * 100.0);
    let d = (dz * dz + dx * dx).sqrt();
    if d >= 6000.0 || d.is_nan() {
        return 0;
    }
    (6000.0 - d).round_ties_even() as i16
}

/// Updates every block's priority from `focus` and returns the terrain's
/// new priority base.
///
/// Blocks that want a composite and are visible keep their relative order:
/// the base is half the spread of their old ranks, plus 6000. Every block
/// then takes `base − distance`, and those in the visible set take half
/// their old rank's offset from the lowest one, plus 6000, instead, so the
/// visible blocks stay ahead of the rest. The original scans for the lowest
/// and highest rank in one pass where a rank that lowers the lowest is not
/// also tried as the highest, and drops both (lowest and highest 0) when
/// the highest ends below the lowest: with no visible block, or with one,
/// or with ranks that only fall, the base is 6000.
pub fn update_priorities(blocks: &mut [BlockPriority], focus: Vec2) -> u32 {
    let mut lowest = u32::MAX;
    let mut highest = 0u32;
    for block in blocks.iter_mut() {
        block.distance = distance_rank(block.centre, focus);
    }
    for block in blocks.iter() {
        if block.wants && block.visible {
            if block.rank < lowest {
                lowest = block.rank;
            } else if block.rank > highest {
                highest = block.rank;
            }
        }
    }
    if highest < lowest {
        lowest = 0;
        highest = 0;
    }
    let base = ((highest - lowest) as i64 / 2) as u32 + 6000;
    for block in blocks.iter_mut() {
        let d = block.distance as u32;
        block.rank = if block.wants && block.visible {
            ((block.rank - lowest) / 2) + 6000 - d
        } else {
            base - d
        };
    }
    base
}

/// The 3 × 3 tiles around (`x`, `y`) as the original gathers them for a
/// terrain plugin: row-major, with the tile itself in the middle. Slots
/// outside the grid, or with no tile, are `None` (the original's empty
/// placeholder).
pub fn neighbours<T: Copy>(
    tiles: &[Option<T>],
    width: i32,
    height: i32,
    x: i32,
    y: i32,
) -> [[Option<T>; 3]; 3] {
    let mut out = [[None; 3]; 3];
    for (row, slots) in out.iter_mut().enumerate() {
        for (col, slot) in slots.iter_mut().enumerate() {
            let (tx, ty) = (x + col as i32 - 1, y + row as i32 - 1);
            if tx < 0 || ty < 0 || tx >= width || ty >= height {
                continue;
            }
            let index = ty as usize * width as usize + tx as usize;
            *slot = tiles.get(index).copied().flatten();
        }
    }
    out
}

/// One plane of a light's reach. Each plane faces outwards: `normal · p +
/// distance` is how far a point lies beyond that face (positive outside the
/// light, negative inside).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LightPlane {
    pub normal: Vec3,
    pub distance: f32,
}

/// A light's convex reach, as the original keeps it: a list of outward
/// planes. The plane list is what the block selector's distance test reads
/// from each light of a block's room; a box-shaped light (the model Exarust
/// builds from a point light) is [`LightVolume::box_volume`]. (Guess: the
/// original's plane list is the light's convex reach volume; the pack shows
/// the list and the distance arithmetic, not what fills it.)
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LightVolume {
    pub planes: Vec<LightPlane>,
}

impl LightVolume {
    /// The six outward planes of an axis-aligned box of `half_extents` about
    /// `centre`, the shape Exarust uses for a point light's reach. Each
    /// plane's outward normal is an axis sign and its reach is the box's
    /// extent along that normal.
    pub fn box_volume(centre: Vec3, half_extents: Vec3) -> Self {
        let mut planes = Vec::with_capacity(6);
        for axis in 0..3 {
            for sign in [1.0f32, -1.0] {
                let mut normal = Vec3::ZERO;
                normal[axis] = sign;
                let reach = sign * centre[axis] + half_extents[axis];
                planes.push(LightPlane {
                    normal,
                    distance: -reach,
                });
            }
        }
        Self { planes }
    }
}

/// An oriented box: a centre, half extents and its axes as the matrix's
/// columns.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrientedBox {
    pub centre: Vec3,
    pub half_extents: Vec3,
    pub axes: Mat3,
}

impl OrientedBox {
    /// A box aligned with the world axes, as a terrain block's bounds are.
    pub fn axis_aligned(centre: Vec3, half_extents: Vec3) -> Self {
        Self {
            centre,
            half_extents,
            axes: Mat3::IDENTITY,
        }
    }
}

/// The distance from an oriented box to a light's reach, centimetres, as the
/// original's plane walk computes it: for every plane the box centre lies in
/// front of, the centre's plane distance less the box's support radius along
/// the plane's normal in the box's own frame; the largest non-negative value
/// wins, and a box overlapping the light answers 0. The sums are the
/// instruction order, `(z + (y + x))`, and the support's component signs are
/// copied onto the half extents (so a signed zero or NaN half extent keeps
/// its own payload).
pub fn light_distance(light: &LightVolume, block: &OrientedBox) -> f32 {
    let mut best = 0.0f32;
    for plane in &light.planes {
        let n = plane.normal;
        let c = block.centre;
        let centre_distance = (n.z * c.z + (n.y * c.y + n.x * c.x)) + plane.distance;
        // Only the planes the centre is beyond take part. The instruction
        // lets a NaN through here; its clearance is then NaN and fails the
        // final compare, so skipping it now is the same.
        if centre_distance.is_nan() || centre_distance < 0.0 {
            continue;
        }
        // The normal in the box's frame: its dot product with each of the
        // box's axes (the matrix's columns), summed `(x + y) + z` as the
        // vector-by-matrix helper does.
        let axis = |a: Vec3| (a.x * n.x + a.y * n.y) + a.z * n.z;
        let v = Vec3::new(
            axis(block.axes.x_axis),
            axis(block.axes.y_axis),
            axis(block.axes.z_axis),
        );
        let extent = |value: f32, half: f32| {
            f32::from_bits(half.to_bits() | (value.to_bits() & 0x8000_0000))
        };
        let e = Vec3::new(
            extent(v.x, block.half_extents.x),
            extent(v.y, block.half_extents.y),
            extent(v.z, block.half_extents.z),
        );
        let support = v.z * e.z + (v.y * e.y + v.x * e.x);
        let clearance = centre_distance - support;
        if clearance.is_nan() || clearance < 0.0 {
            continue;
        }
        if best < clearance {
            best = clearance;
        }
    }
    best
}

/// The nearest light's distance for a block: the smallest
/// [`light_distance`] over the block's room lights, starting from the
/// original's 65 000 cm when there is none. A NaN answer is ignored, as the
/// original's `value < nearest` test is.
pub fn nearest_light_distance<'a>(
    lights: impl IntoIterator<Item = &'a LightVolume>,
    block: &OrientedBox,
) -> f32 {
    let mut nearest = NO_LIGHT_DISTANCE;
    for light in lights {
        let distance = light_distance(light, block);
        if distance < nearest {
            nearest = distance;
        }
    }
    nearest
}

/// The sort key a block hidden from the camera takes: its nearest light's
/// rounded distance plus the light bias.
pub fn light_sort_key(nearest: f32) -> u32 {
    (round_to_i64(nearest) + i64::from(LIGHT_SORT_BIAS)) as u32
}

/// A tile's streaming bits, as the original's terrain worker and frame hook
/// write them.
pub mod tile_flags {
    /// The raw tile data is present (the state the `.rft` reader leaves).
    pub const RAW: u32 = 0x1;
    /// The tile's geometry has been built (the worker's state).
    pub const BUILT: u32 = 0x8;
    /// The tile is ready for the frame hook.
    pub const READY: u32 = 0x20;
    /// The tile's nodes are in the scene.
    pub const INSERTED: u32 = 0x80;
    /// The tile is in the frame hook's walk (the worker sets it; nothing in
    /// the terrain clears it, so an inserted tile stays in the walk and is
    /// passed over while it needs nothing).
    pub const STREAM: u32 = 0x800;
    /// The tile's data changed and its GPU side must be reloaded (the worker
    /// sets this when the terrain's flags ask); the frame hook unloads it.
    pub const RELOAD: u32 = 0x1000;
    /// The tile has been unloaded.
    pub const UNLOADED: u32 = 0x2000;
    /// The tile's GPU data waits for an upload (cleared by the upload).
    pub const UPLOAD: u32 = 0x4000;
}

/// What the frame hook does with one tile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileAction {
    /// Nothing this frame (the tile is mid-load).
    None,
    /// Insert the tile's nodes into the scene.
    Insert,
    /// Upload the tile's GPU data.
    Upload,
    /// Unload the tile's GPU data.
    Unload,
}

/// One tile's streaming state: the bits above, and the refresh delay the
/// per-tile refresh pass bumps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TileStream {
    pub state: u32,
    /// The refresh pass's delay counter.
    pub delay: i32,
}

impl Default for TileStream {
    fn default() -> Self {
        Self::raw()
    }
}

impl TileStream {
    /// A tile as the terrain reader leaves it: raw data, state 7.
    pub fn raw() -> Self {
        Self {
            state: 0x7,
            delay: 0,
        }
    }

    /// The worker finished the tile's geometry and masks: state
    /// `(state & ~0x10) | 0x828`, plus the reload bit when the terrain's
    /// flags ask for one.
    pub fn mark_built(&mut self, reload: bool) {
        self.state = (self.state & !0x10) | 0x828;
        if reload {
            self.state |= tile_flags::RELOAD;
        }
    }

    /// The frame hook inserted the tile's nodes.
    pub fn mark_inserted(&mut self) {
        self.state = (self.state & !tile_flags::READY) | tile_flags::INSERTED;
    }

    /// Queues the tile's upload.
    pub fn begin_upload(&mut self) {
        self.state |= tile_flags::UPLOAD;
    }

    /// The frame hook's choice for this tile. The insert test is the
    /// original's `(state & 0x309f) == 0xf`; the upload test is
    /// `(state & 0x4080) == 0x4080`; a reload bit wins over both and is
    /// handled by unloading the tile.
    pub fn action(&self) -> TileAction {
        if self.state & tile_flags::RELOAD != 0 {
            return TileAction::Unload;
        }
        if self.state & 0x309f == 0xf {
            return TileAction::Insert;
        }
        if self.state & 0x4080 == 0x4080 {
            return TileAction::Upload;
        }
        TileAction::None
    }

    /// Applies the frame hook's action: the state transitions its branches
    /// make (an insert also uploads, so it clears the upload bit too).
    pub fn apply(&mut self, action: TileAction) {
        match action {
            TileAction::Insert => {
                self.state = (self.state & !tile_flags::READY) | tile_flags::INSERTED;
                self.state &= !tile_flags::UPLOAD;
            }
            TileAction::Upload => {
                self.state &= !tile_flags::UPLOAD;
            }
            TileAction::Unload => {
                self.state = (self.state & !tile_flags::RELOAD) | tile_flags::UNLOADED;
            }
            TileAction::None => {}
        }
    }

    /// The frame hook marks every block of a ready, inserted tile as wanting
    /// a composite (`state & 0xa0 == 0xa0`).
    pub fn blocks_wanted(&self) -> bool {
        self.state & 0xa0 == 0xa0
    }

    /// The refresh pass: the tile leaves the scene (its
    /// nodes are detached, visibility cleared) and its delay counter goes up.
    pub fn refresh(&mut self) {
        self.state &= !tile_flags::INSERTED;
        self.delay += 1;
    }
}

/// One frame of the frame hook's tile walk.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FrameStep {
    /// Tiles whose every block was marked as wanting a composite this frame:
    /// a ready, inserted tile passed on the walk (its ready bit cleared), and
    /// a tile just inserted.
    pub wanted: Vec<usize>,
    /// The one tile handled (insert, upload or unload), which ends the walk.
    pub handled: Option<(usize, TileAction)>,
}

/// A level's tiles as the frame hook streams them: at most one tile handled
/// a frame, and a delay counter that skips frames after a refresh pass.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TerrainStream {
    pub tiles: Vec<TileStream>,
    /// The terrain's refresh delay; while positive, frames only decrease it.
    pub delay: i32,
}

impl TerrainStream {
    pub fn new(tiles: Vec<TileStream>) -> Self {
        Self { tiles, delay: 0 }
    }

    /// The frame hook's walk. While the delay is positive the frame only
    /// counts it down (`None`: no walk and no block selection). Otherwise
    /// the tiles are walked in order: a tile without the stream bit is
    /// passed; a ready, inserted one has its blocks marked wanted and its
    /// ready bit cleared; then the first tile with an action (reload first,
    /// then insert, then upload) is handled, its transition applied, and
    /// the walk ends. A tile with no action is passed. The block selection
    /// runs after the walk.
    pub fn frame(&mut self) -> Option<FrameStep> {
        if self.delay >= 1 {
            self.delay -= 1;
            return None;
        }
        let mut step = FrameStep::default();
        for (index, tile) in self.tiles.iter_mut().enumerate() {
            if tile.state & tile_flags::STREAM == 0 {
                continue;
            }
            if tile.blocks_wanted() {
                step.wanted.push(index);
                tile.state &= !tile_flags::READY;
            }
            let action = tile.action();
            if action == TileAction::None {
                continue;
            }
            tile.apply(action);
            if action == TileAction::Insert {
                step.wanted.push(index);
            }
            step.handled = Some((index, action));
            break;
        }
        Some(step)
    }

    /// Whether every tile of the terrain has been inserted, as the
    /// block-until-loaded slot tests (`state & 0x80`).
    pub fn all_inserted(&self) -> bool {
        self.tiles
            .iter()
            .all(|tile| tile.state & tile_flags::INSERTED != 0)
    }

    /// The block-until-loaded slot: run the frame hook until every tile is
    /// inserted, handing each handled action to `on_action` (the caller
    /// attaches or detaches the tile's nodes). Bounded so a level with an
    /// unbuildable tile cannot spin forever; returns the calls made.
    pub fn wait_until_inserted(&mut self, mut on_action: impl FnMut(usize, TileAction)) -> usize {
        let cap = self.tiles.len().saturating_mul(4) + 64;
        let mut calls = 0;
        while !self.all_inserted() && calls < cap {
            if let Some(step) = self.frame()
                && let Some((index, action)) = step.handled
            {
                on_action(index, action);
            }
            calls += 1;
        }
        calls
    }

    /// The per-tile refresh pass: every tile detaches its nodes (or clears
    /// their visibility when it was not inserted) and the terrain's refresh
    /// delay goes up by one, as the refresh slot does.
    pub fn refresh_all(&mut self) {
        for tile in &mut self.tiles {
            tile.refresh();
        }
        self.delay += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(centre: Vec2, rank: u32, wants: bool, visible: bool) -> BlockPriority {
        BlockPriority {
            centre,
            distance: 0,
            rank,
            wants,
            visible,
        }
    }

    #[test]
    fn the_focus_looks_ahead_and_snaps_to_a_block_centre() {
        // Camera at (20, −4) looking along −Z (its back axis is +Z): 10 m
        // ahead is (20, −14) m, in block (3, −3) of 600 cm, whose centre is
        // (2100, −1500) cm.
        assert_eq!(
            composite_focus(Vec2::new(20.0, -4.0), Vec2::new(0.0, 1.0)),
            Vec2::new(21.0, -15.0)
        );
        // Looking down at 60°, the back axis is short in the plane and the
        // look-ahead with it: (0, 0) + 0.5 · 10 m along −X is (−5, 0),
        // block (−1, 0).
        assert_eq!(
            composite_focus(Vec2::ZERO, Vec2::new(0.5, 0.0)),
            Vec2::new(-3.0, 3.0)
        );
        // A point exactly on a block's low edge floors into that block.
        assert_eq!(
            composite_focus(Vec2::new(0.0, 0.0), Vec2::ZERO),
            Vec2::new(3.0, 3.0)
        );
        assert_eq!(
            composite_focus(Vec2::new(-6.0, -0.01), Vec2::ZERO),
            Vec2::new(-3.0, -3.0)
        );
    }

    #[test]
    fn composite_selection_uses_the_squared_centimetre_cutoffs() {
        let focus = Vec2::ZERO;
        let near = select_composites(
            focus,
            [
                Vec2::new(0.0, 0.0),   // 0 m: high
                Vec2::new(14.99, 0.0), // high
                Vec2::new(15.0, 0.0),  // exactly 15 m: low
                Vec2::new(44.4, 0.0),  // exactly 44.4 m: low
                Vec2::new(44.41, 0.0), // out
            ],
        );
        assert_eq!(
            near,
            vec![
                CompositeBlock {
                    index: 0,
                    high: true
                },
                CompositeBlock {
                    index: 1,
                    high: true
                },
                CompositeBlock {
                    index: 2,
                    high: false
                },
                CompositeBlock {
                    index: 3,
                    high: false
                },
            ]
        );
        // A NaN distance never compares above the cut-off: kept, low.
        let nan = select_composites(focus, [Vec2::new(f32::NAN, 0.0)]);
        assert_eq!(
            nan,
            vec![CompositeBlock {
                index: 0,
                high: false
            }]
        );
        assert!(!high_resolution(f32::NAN));
        assert!(within_reach(0.0) && within_reach(f32::NAN));
        assert!(!within_reach(19_713_601.0) && within_reach(19_713_600.0));
        // The list stops at 189 entries.
        let many = select_composites(focus, std::iter::repeat_n(Vec2::ZERO, 300));
        assert_eq!(many.len(), MAX_SELECTED);
        assert_eq!((HIGH_SETS, LOW_SETS), (21, 43));
    }

    #[test]
    fn distance_ranks_round_to_even_and_stop_at_sixty_metres() {
        assert_eq!(distance_rank(Vec2::ZERO, Vec2::ZERO), 6000);
        // 59.99 m leaves 1 cm → rank 1.
        assert_eq!(distance_rank(Vec2::ZERO, Vec2::new(59.99, 0.0)), 1);
        // 0.005 cm below 60 m is not a whole centimetre either; 58 m gives
        // exactly 200.
        assert_eq!(distance_rank(Vec2::ZERO, Vec2::new(58.0, 0.0)), 200);
        // 30 m gives 3000.
        assert_eq!(distance_rank(Vec2::ZERO, Vec2::new(0.0, 30.0)), 3000);
        // Half a centimetre of remainder rounds to even: 59.995 m → the
        // distance is 5999.5 cm, 0.5 left, which rounds to 0.
        assert_eq!(distance_rank(Vec2::ZERO, Vec2::new(59.995, 0.0)), 0);
        assert_eq!(distance_rank(Vec2::ZERO, Vec2::new(60.0, 0.0)), 0);
        assert_eq!(distance_rank(Vec2::ZERO, Vec2::new(70.0, 0.0)), 0);
    }

    #[test]
    fn visible_blocks_keep_their_order_above_the_base() {
        let focus = Vec2::ZERO;
        let mut blocks = vec![
            block(Vec2::new(0.0, 0.0), 100, true, true),
            block(Vec2::new(6.0, 0.0), 300, true, true),
            block(Vec2::new(30.0, 0.0), 0, false, false),
            block(Vec2::new(60.0, 0.0), 0, true, false),
        ];
        let base = update_priorities(&mut blocks, focus);
        // Visible ranks 100..300 → base (300 − 100)/2 + 6000 = 6100.
        assert_eq!(base, 6100);
        // Distances: 0, 600, 3000, 6000 → ranks 0/6000/6000/6000.
        assert_eq!(blocks[0].distance, 6000);
        assert_eq!(blocks[1].distance, 5400);
        assert_eq!(blocks[2].distance, 3000);
        assert_eq!(blocks[3].distance, 0);
        // Visible: (rank − 100)/2 + 6000 − distance.
        assert_eq!(blocks[0].rank, 0);
        assert_eq!(blocks[1].rank, 700);
        // Not visible: base − distance.
        assert_eq!(blocks[2].rank, 6100 - 3000);
        assert_eq!(blocks[3].rank, 6100);
        // With no visible block, the old ranks are dropped: base 6000.
        let mut blocks = vec![block(Vec2::new(0.0, 0.0), 9, false, false)];
        assert_eq!(update_priorities(&mut blocks, focus), 6000);
        assert_eq!(blocks[0].rank, 0);
        // Ranks that only fall never reach the highest: 300 then 100 sets
        // the lowest twice, the highest stays 0 below it, and both drop to
        // 0. The visible blocks then take rank/2 + 6000 − distance.
        let mut blocks = vec![
            block(Vec2::new(0.0, 0.0), 300, true, true),
            block(Vec2::new(6.0, 0.0), 100, true, true),
            block(Vec2::new(30.0, 0.0), 0, false, false),
        ];
        assert_eq!(update_priorities(&mut blocks, focus), 6000);
        assert_eq!(blocks[0].rank, 150);
        assert_eq!(blocks[1].rank, 50 + 600);
        assert_eq!(blocks[2].rank, 3000);
    }

    #[test]
    fn the_gather_is_three_by_three_with_empty_placeholders() {
        let tiles: Vec<Option<u8>> = (0..9).map(Some).collect();
        let n = neighbours(&tiles, 3, 3, 1, 1);
        assert_eq!(
            n,
            [
                [Some(0), Some(1), Some(2)],
                [Some(3), Some(4), Some(5)],
                [Some(6), Some(7), Some(8)],
            ]
        );
        let n = neighbours(&tiles, 3, 3, 0, 0);
        assert_eq!(
            n,
            [
                [None, None, None],
                [None, Some(0), Some(1)],
                [None, Some(3), Some(4)],
            ]
        );
        // A missing tile is the placeholder too.
        let mut tiles = tiles;
        tiles[4] = None;
        let n = neighbours(&tiles, 3, 3, 1, 1);
        assert_eq!(n[1][1], None);
        assert_eq!(n[0][0], Some(0));
    }

    #[test]
    fn a_built_tile_keeps_the_reader_bits_and_inserts_once() {
        let mut tile = TileStream::raw();
        assert_eq!(tile.state, 0x7);
        assert_eq!(tile.action(), TileAction::None);
        // The worker clears bit 0x10 and ORs 0x828 into the reader's 7, so
        // the insert test (low bits 0xf, no 0x3000) passes.
        tile.mark_built(false);
        assert_eq!(tile.state, 0x82f);
        assert_eq!(tile.action(), TileAction::Insert);
        tile.apply(TileAction::Insert);
        assert_eq!(tile.state, 0x88f);
        assert_eq!(tile.action(), TileAction::None);
        // A queued upload is handled next.
        tile.begin_upload();
        assert_eq!(tile.action(), TileAction::Upload);
        tile.apply(TileAction::Upload);
        assert_eq!(tile.state, 0x88f);
        // A reload request (bit 0x1000) on a fresh tile is handled by
        // unloading.
        let mut reload = TileStream::raw();
        reload.mark_built(true);
        assert_eq!(reload.state, 0x182f);
        assert_eq!(reload.action(), TileAction::Unload);
        reload.apply(TileAction::Unload);
        assert_eq!(reload.state, 0x282f);
    }

    #[test]
    fn a_block_with_no_light_in_reach_ranks_at_sixty_five_thousand() {
        assert_eq!(light_sort_key(65_000.0), 65_000 + LIGHT_SORT_BIAS);
        assert_eq!(light_sort_key(0.0), LIGHT_SORT_BIAS);
        // Ties to even: 2.5 rounds to 2.
        assert_eq!(light_sort_key(2.5), 2 + LIGHT_SORT_BIAS);
    }

    #[test]
    fn the_light_distance_is_the_largest_plane_clearance() {
        let light = LightVolume::box_volume(Vec3::ZERO, Vec3::splat(100.0));
        let block = OrientedBox::axis_aligned(Vec3::new(500.0, 0.0, 0.0), Vec3::splat(50.0));
        assert_eq!(light_distance(&light, &block), 350.0);
        // Touching the +X face is 0, and so is a block overlapping the light.
        let touching = OrientedBox::axis_aligned(Vec3::new(150.0, 0.0, 0.0), Vec3::splat(50.0));
        assert_eq!(light_distance(&light, &touching), 0.0);
        let inside = OrientedBox::axis_aligned(Vec3::new(120.0, 0.0, 0.0), Vec3::splat(50.0));
        assert_eq!(light_distance(&light, &inside), 0.0);
        // A rotated box reaches further along +X: 50·cos45 + 50·sin45.
        let turned = OrientedBox {
            centre: Vec3::new(500.0, 0.0, 0.0),
            half_extents: Vec3::splat(50.0),
            axes: Mat3::from_rotation_z(45f32.to_radians()),
        };
        let d = light_distance(&light, &turned);
        assert!(
            (d - (400.0 - 50.0 * std::f32::consts::SQRT_2)).abs() < 1e-3,
            "{d}"
        );
    }

    #[test]
    fn the_light_distance_takes_the_normal_into_the_boxs_own_axes() {
        // A sheared box: its x axis is (1, 1, 0) and its y axis (0, 1, 0).
        // The +X plane's normal in the box's frame is (n·x_axis, n·y_axis,
        // n·z_axis) = (1, 0, 0), so the support is the x half extent alone:
        // 500 − 100 − 10 = 390. (Multiplying by the matrix instead would
        // give (1, 1, 0) and a support of 10 + 20.)
        let light = LightVolume::box_volume(Vec3::ZERO, Vec3::splat(100.0));
        let sheared = OrientedBox {
            centre: Vec3::new(500.0, 0.0, 0.0),
            half_extents: Vec3::new(10.0, 20.0, 30.0),
            axes: Mat3::from_cols(Vec3::new(1.0, 1.0, 0.0), Vec3::Y, Vec3::Z),
        };
        assert_eq!(light_distance(&light, &sheared), 390.0);
    }

    #[test]
    fn the_light_distance_skips_nan_and_picks_the_nearest_light() {
        // A NaN plane is skipped (its centre distance never compares >= 0).
        let nan = LightVolume {
            planes: vec![LightPlane {
                normal: Vec3::new(f32::NAN, 0.0, 0.0),
                distance: 1000.0,
            }],
        };
        let block = OrientedBox::axis_aligned(Vec3::ZERO, Vec3::splat(50.0));
        assert_eq!(light_distance(&nan, &block), 0.0);
        // The nearest of two box lights.
        let far = LightVolume::box_volume(Vec3::new(1000.0, 0.0, 0.0), Vec3::splat(100.0));
        let near = LightVolume::box_volume(Vec3::new(400.0, 0.0, 0.0), Vec3::splat(100.0));
        let distance = nearest_light_distance([&far, &near], &block);
        // Near light: 400 − 100 − 50 = 250.
        assert_eq!(distance, 250.0);
        // No lights at all.
        assert_eq!(
            nearest_light_distance(std::iter::empty(), &block),
            NO_LIGHT_DISTANCE
        );
    }

    #[test]
    fn the_frame_hook_handles_one_tile_a_frame_and_passes_idle_ones() {
        let mut stream = TerrainStream::new(vec![
            TileStream::raw(),
            TileStream::raw(),
            TileStream::raw(),
        ]);
        // No tile is in the walk yet.
        assert_eq!(stream.frame(), Some(FrameStep::default()));
        for tile in &mut stream.tiles {
            tile.mark_built(false);
        }
        // The first tile is inserted, its blocks wanted, and the walk ends.
        let step = stream.frame().unwrap();
        assert_eq!(step.handled, Some((0, TileAction::Insert)));
        assert_eq!(step.wanted, [0]);
        assert_eq!(stream.tiles[0].state, 0x88f);
        // The inserted tile keeps its stream bit and is passed over: the
        // next frame inserts the second tile.
        let step = stream.frame().unwrap();
        assert_eq!(step.handled, Some((1, TileAction::Insert)));
        // A ready, inserted tile on the walk has its blocks wanted and its
        // ready bit cleared, and the walk goes on to the next action.
        stream.tiles[0].state |= tile_flags::READY;
        stream.tiles[1].begin_upload();
        let step = stream.frame().unwrap();
        assert_eq!(step.wanted, [0]);
        assert_eq!(step.handled, Some((1, TileAction::Upload)));
        assert_eq!(stream.tiles[0].state & tile_flags::READY, 0);
        assert_eq!(stream.tiles[1].state & tile_flags::UPLOAD, 0);
        // A refresh sets the delay and the next frames only count it down.
        stream.tiles[1].refresh();
        stream.delay = 2;
        assert_eq!(stream.frame(), None);
        assert_eq!(stream.delay, 1);
        assert_eq!(stream.frame(), None);
        assert_eq!(stream.delay, 0);
        // The refreshed tile left the scene, so it is inserted again before
        // the third tile.
        assert_eq!(
            stream.frame().unwrap().handled,
            Some((1, TileAction::Insert))
        );
        assert_eq!(
            stream.frame().unwrap().handled,
            Some((2, TileAction::Insert))
        );
    }
}
