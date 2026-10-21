//! The dynamics world: the original's `RF_Dynamics` unit — the simulations'
//! fixed-step driver and clock, the environments' gravity and velocity
//! bands, the two shared grids a simulation keeps its static nodes and its
//! bodies in, the per-node and per-body state helpers, the sectors' purges
//! of what left a scene, and the overlap query a placement asks before it
//! puts a body down.
//!
//! The original runs one [`Simulation`] per scene: a clock in milliseconds
//! and ticks, a list of sectors, and two 600 cm grids of the same size (one
//! for the static nodes a sector registers, one for its bodies). The
//! driver steps the first simulation that is due ([`pick_due`]); a step
//! first updates the simulation ([`Simulation::update`]: the pending grid
//! shift, each sector's flags, the pending grid rebuild) and then advances
//! its clock.
//!
//! This module holds that unit's rules as data and algorithms over plain
//! records ([`World`], [`Sector`], [`Grid`]), so they can be reasoned about
//! and tested on their own; the ECS body solver in [`crate::body`] runs the
//! physical step itself and keeps its own sleeping and grouping state.
//! Nothing in the running game drives these records yet.
//!
//! Lengths are the original's centimetres and times its milliseconds.

use bevy::math::{Affine3A, Mat3, Mat4, Vec3, Vec4};

use crate::body::pairs::layers_apart;
use crate::body::{RETENTION, RETENTION_SPEEDS, per_tick};
use crate::mechanism::MECHANISM_RETENTION;

/// One fixed step: 12 ms, 83 Hz.
pub const STEP_MS: u32 = 12;
/// Steps a late simulation may fall behind before it drops the rest.
pub const MAX_STEPS_BEHIND: u32 = 4;
/// The longest the driver waits when no simulation is due (ms).
pub const IDLE_WAIT_MS: u32 = 100;
/// The simulation's default tick rate (ticks a second).
pub const TICK_HZ: u32 = 83;
/// The step the simulation constructor rounds `1000/83` to.
pub const DEFAULT_STEP_MS: u32 = 12;
/// The backlog the constructor allows: `1000/83` truncated, less one.
pub const MAX_BACKLOG_STEPS: u32 = 1000 / TICK_HZ - 1;
/// The default environments the constructor creates (the normal one and
/// the inert one; the count field holds the original's 5 or 1 by option).
pub const DEFAULT_ENVIRONMENTS: u32 = 5;
/// The grids' cell (cm).
pub const CELL_CM: f32 = 600.0;
/// One over the cell as the original multiplies by it (a float, so a
/// position on a cell boundary falls as the product does, not as a
/// division would).
pub const CELL_RECIPROCAL: f32 = 0.001_666_666_7;
/// The original's default gravity (m/s²), as the body solver's.
pub use crate::body::GRAVITY;

/// A simulation's clock: the tick counter, the game time and step in
/// milliseconds, the wall-clock base the game time counts from, and
/// whether it is paused (a paused simulation keeps its backlog).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clock {
    pub tick: u32,
    pub time: u32,
    pub step: u32,
    pub last: u32,
    pub paused: bool,
}

impl Default for Clock {
    fn default() -> Self {
        Self {
            tick: 0,
            time: 0,
            step: STEP_MS,
            last: 0,
            paused: false,
        }
    }
}

/// Why the step driver is waiting, or that a simulation runs now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Due {
    /// This simulation's next step is due.
    Now,
    /// How long to wait for the soonest step (ms, at most
    /// [`IDLE_WAIT_MS`]).
    Wait(u32),
}

impl Clock {
    /// The next step's end in game time.
    fn end(&self) -> u32 {
        self.time.wrapping_add(self.step)
    }

    /// The wall-clock time `now` as game time.
    fn late(&self, now: u32) -> u32 {
        now.wrapping_sub(self.last)
    }

    /// Whether the step at `time + step` is due at `now`.
    pub fn due(&self, now: u32) -> bool {
        self.end() <= self.late(now)
    }

    /// How far past the next step's end `now` is (0 when not due).
    pub fn behind(&self, now: u32) -> u32 {
        self.late(now).saturating_sub(self.end())
    }

    /// A due simulation more than four steps behind and not paused drops
    /// its backlog: its base moves so that the step it takes now ends
    /// exactly at `now`.
    pub fn drop_backlog(&mut self, now: u32) {
        if self.paused || !self.due(now) {
            return;
        }
        // The original compares the four steps (a 32-bit shift) with the
        // lateness, both widened to 64 bits.
        if i64::from(self.behind(now)) > i64::from(self.step << 2) {
            self.last = now.wrapping_sub(self.end());
        }
    }

    /// One step: the tick and the game time advance by the step.
    pub fn advance(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        self.time = self.time.wrapping_add(self.step);
    }

    /// The scaled time since `mark`, as the original's time-scale helper
    /// returns it: the unsigned game time less the signed mark as a 64-bit
    /// difference, converted to a float, scaled, and converted back with
    /// the processor's nearest-even rounding into 64 bits (a NaN or an
    /// answer out of range gives the conversion's `i64::MIN`).
    pub fn scale_since(&self, mark: i32, scale: f32) -> i64 {
        let delta = i64::from(self.time) - i64::from(mark);
        let rounded = ((delta as f32) * scale).round_ties_even();
        if rounded.is_nan() || rounded >= 9.223_372e18 || rounded < -9.223_372e18 {
            i64::MIN
        } else {
            rounded as i64
        }
    }
}

/// The step driver's choice over the simulations: the first whose step is
/// due runs (dropping its backlog first); otherwise the wait is the
/// soonest step's distance, at most [`IDLE_WAIT_MS`].
pub fn pick_due(sims: &mut [Clock], now: u32) -> (Due, Option<usize>) {
    let mut wait = IDLE_WAIT_MS;
    for (i, sim) in sims.iter_mut().enumerate() {
        if sim.due(now) {
            sim.drop_backlog(now);
            return (Due::Now, Some(i));
        }
        wait = wait.min(sim.end() - sim.late(now));
    }
    (Due::Wait(wait), None)
}

/// Drops the simulations whose scene closed, swapping the last one into
/// each place (the original's sweep before and after every step).
pub fn drop_closed<T>(sims: &mut Vec<T>, mut closed: impl FnMut(&T) -> bool) {
    let mut i = 0;
    while i < sims.len() {
        if closed(&sims[i]) {
            sims.swap_remove(i);
        } else {
            i += 1;
        }
    }
}

/// A body environment's gravity (m/s²) and velocity bands: the retention
/// per second above each threshold (cm/s), and below the lower one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Environment {
    pub gravity: f32,
    pub fast: f32,
    pub slow: f32,
    pub resting: f32,
    pub fast_speed: f32,
    pub slow_speed: f32,
}

impl Environment {
    /// The default environment bodies take: gravity and the per-second
    /// retentions 0.426, 0.25 and 0.05 above 40, above 20 and at rest
    /// (cm/s).
    pub const DEFAULT: Self = Self {
        gravity: GRAVITY,
        fast: RETENTION[0],
        slow: RETENTION[1],
        resting: RETENTION[2],
        fast_speed: RETENTION_SPEEDS[0],
        slow_speed: RETENTION_SPEEDS[1],
    };
    /// The simulation's second environment: no gravity and no retention.
    pub const INERT: Self = Self {
        gravity: 0.0,
        fast: 0.0,
        slow: 0.0,
        resting: 0.0,
        ..Self::DEFAULT
    };
    /// The environment the mechanisms share: retention 0.001 a second in
    /// every band and no gravity.
    pub const MECHANISM: Self = Self {
        gravity: 0.0,
        fast: MECHANISM_RETENTION,
        slow: MECHANISM_RETENTION,
        resting: MECHANISM_RETENTION,
        ..Self::DEFAULT
    };

    /// The gravity a particle takes in one tick (m): g·dt², with dt the
    /// fixed step.
    pub fn gravity_per_tick(&self) -> Vec3 {
        let dt = 1.0 / crate::STEP_HZ as f32;
        Vec3::Y * self.gravity * dt * dt
    }

    /// The retention per tick for a body whose farthest particle moved
    /// `moved` cm in the last tick: fast above 40 cm/s, slow above 20,
    /// else resting, each its per-second value raised to a tick (none for
    /// a retention at or under zero).
    /// The environment's per-step table as the original's conversion
    /// writes it: each per-second retention raised to `1/step`, the gravity
    /// per tick (g/step², metres), and the two speed thresholds squared
    /// after their per-tick conversion. A non-positive retention gives 0.
    pub fn configure_step(&self, tick_rate: u32) -> StepEnvironment {
        let s = f64::from(tick_rate);
        let raise = |r: f32| {
            if r <= 0.0 {
                0.0
            } else {
                ((f64::from(r).ln()) / s).exp() as f32
            }
        };
        StepEnvironment {
            retention: [raise(self.fast), raise(self.slow), raise(self.resting)],
            gravity: Vec3::Y * (self.gravity / (tick_rate.wrapping_mul(tick_rate) as f32)),
            fast_speed: ((self.fast_speed / tick_rate as f32)
                * (self.fast_speed / tick_rate as f32)),
            slow_speed: ((self.slow_speed / tick_rate as f32)
                * (self.slow_speed / tick_rate as f32)),
        }
    }

    pub fn retention(&self, moved: f32) -> f32 {
        let per_second = if moved > self.fast_speed / 83.0 {
            self.fast
        } else if moved > self.slow_speed / 83.0 {
            self.slow
        } else {
            self.resting
        };
        per_tick(per_second)
    }
}

/// An environment's values for one step, as the original's conversion
/// leaves them on the environment object.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StepEnvironment {
    /// The retentions per tick: fast, slow and resting.
    pub retention: [f32; 3],
    /// Gravity per tick, g/step² (the original's centimetres, here m/s²).
    pub gravity: Vec3,
    /// The fast band's speed threshold, per tick and squared.
    pub fast_speed: f32,
    /// The slow band's speed threshold, per tick and squared.
    pub slow_speed: f32,
}

/// The simulation's per-frame interpolation fraction: the original's
/// `1 + (elapsed − previous)/step` when the elapsed time is above the
/// previous one, else `1 − (previous − elapsed)/step`, clamped to
/// [−2, 2]. Both differences are unsigned 32-bit differences widened to
/// double, and the result is rounded to an `f32` as the original stores
/// it.
pub fn frame_fraction(since_update: u32, previous: u32, step: u32) -> f32 {
    let f = if previous < since_update {
        let x = f64::from(since_update - previous) / f64::from(step) + 1.0;
        x as f32
    } else {
        let x = 1.0 - f64::from(previous - since_update) / f64::from(step);
        x as f32
    };
    f.clamp(-2.0, 2.0)
}

/// The per-frame timing state the simulation keeps: the game time of its
/// last update, the previous frame's elapsed time and the last fraction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameTiming {
    pub last_update: u32,
    pub previous: u32,
    pub fraction: f32,
}

impl Default for FrameTiming {
    fn default() -> Self {
        Self {
            last_update: 0,
            previous: 0,
            fraction: 1.0,
        }
    }
}

impl FrameTiming {
    /// The frame pass: with a time scale below 1 the simulation is held,
    /// so nothing moves; otherwise the elapsed game time since the last
    /// update gives the new fraction and becomes the previous value.
    pub fn update(&mut self, now: u32, step: u32, time_scale: f32) -> Option<f32> {
        if time_scale >= 1.0 {
            return None;
        }
        let elapsed = now.wrapping_sub(self.last_update);
        self.fraction = frame_fraction(elapsed, self.previous, step);
        self.previous = elapsed;
        Some(self.fraction)
    }
}

/// The per-axis grid dimensions the scene gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridSize {
    pub width: i32,
    pub height: i32,
}

/// The processor's rounding conversion (nearest, ties to even) into 64
/// bits, kept as its low 32 bits: a NaN or out-of-range value gives 0.
fn round_word(v: f32) -> i32 {
    let r = v.round_ties_even();
    if r.is_nan() || r >= 9.223_372e18 || r < -9.223_372e18 {
        0
    } else {
        r as i64 as i32
    }
}

/// The original's floor helper (it sets the rounding mode down for one
/// 32-bit conversion): a NaN or out-of-range value gives `i32::MIN`.
fn floor_word(v: f32) -> i32 {
    let r = v.floor();
    if r.is_nan() || r >= 2_147_483_648.0 || r < -2_147_483_648.0 {
        i32::MIN
    } else {
        r as i32
    }
}

/// One of a simulation's two grids: a list per cell, x across and z down
/// in one flat table, the bias that puts the scene's origin at the grid's
/// middle, and the nine neighbour offsets the overlap query walks.
#[derive(Debug, Clone, PartialEq)]
pub struct Grid {
    pub width: i32,
    pub height: i32,
    /// Half the width and height, rounded down.
    pub bias: [i32; 2],
    /// The flat offsets of a cell and its eight neighbours, the cell
    /// itself first: 0, −w−1, −w, −w+1, −1, 1, w−1, w, w+1.
    pub offsets: [i32; 9],
    cells: Vec<Vec<usize>>,
}

impl Grid {
    /// An empty grid of `size` cells.
    pub fn new(size: GridSize) -> Self {
        let width = size.width.max(0);
        let height = size.height.max(0);
        let len = (width as usize).saturating_mul(height as usize);
        let w = width;
        Self {
            width,
            height,
            bias: [width >> 1, height >> 1],
            offsets: [0, -1 - w, -w, 1 - w, -1, 1, w - 1, w, w + 1],
            cells: vec![Vec::new(); len],
        }
    }

    /// The cell a sector's node at `position` (cm, its translation) caches:
    /// x and z times the cell's reciprocal, rounded to even, plus the bias.
    pub fn sector_cell(&self, position: Vec3) -> [i32; 2] {
        [
            round_word(position.x * CELL_RECIPROCAL).wrapping_add(self.bias[0]),
            round_word(position.z * CELL_RECIPROCAL).wrapping_add(self.bias[1]),
        ]
    }

    /// The flat cell a body at `local` (cm, in its sector's space) falls
    /// in from its sector's cell: x and z times the reciprocal, rounded
    /// down, plus the sector's cell. None outside the table (the original
    /// reads whatever is there).
    pub fn body_cell(&self, local: Vec3, sector: [i32; 2]) -> Option<usize> {
        let x = floor_word(local.x * CELL_RECIPROCAL).wrapping_add(sector[0]);
        let y = floor_word(local.z * CELL_RECIPROCAL).wrapping_add(sector[1]);
        self.flat(i64::from(y.wrapping_mul(self.width).wrapping_add(x)))
    }

    /// A flat index if it is inside the table.
    pub fn flat(&self, index: i64) -> Option<usize> {
        (0..self.cells.len() as i64)
            .contains(&index)
            .then_some(index as usize)
    }

    /// The bodies in a cell, the latest linked first.
    pub fn cell(&self, index: usize) -> &[usize] {
        self.cells.get(index).map_or(&[], |c| c)
    }

    /// The cell lists.
    pub fn cells(&self) -> &[Vec<usize>] {
        &self.cells
    }

    /// Links an entry at the head of a cell's list (the original's cells
    /// are linked lists; the bodies link themselves as they step).
    pub fn insert(&mut self, index: usize, entry: usize) {
        if let Some(list) = self.cells.get_mut(index) {
            list.insert(0, entry);
        }
    }

    /// Unlinks an entry from a cell's list, if it is there.
    pub fn remove(&mut self, index: usize, entry: usize) {
        if let Some(list) = self.cells.get_mut(index)
            && let Some(at) = list.iter().position(|&e| e == entry)
        {
            list.remove(at);
        }
    }

    /// Empties every cell.
    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            cell.clear();
        }
    }

    /// Shifts the grid by `(dx, dy)` cells as the original moves its flat
    /// table: with the offset `o = dy·w + dx`, cell `j` takes what cell
    /// `j − o` held, and the cells left behind at the start (o > 0) or the
    /// end (o < 0) are emptied. Then, for a horizontal shift, a strip of
    /// `|dx|` cells is emptied in as many rows as the original walks: rows
    /// `dy` to `h − 1` when o > 0, and `h − dy` rows from row 0 when o < 0
    /// (beyond the table for dy < 0; those rows are skipped here). The
    /// strip starts at each row's first cell for dx > 0, and at column
    /// `w + dx − 1` for dx < 0 (the original's own start, one cell short of
    /// the column that wrapped in), counting rows from row 0 even when o >
    /// 0. A zero offset moves nothing.
    pub fn shift(&mut self, dx: i32, dy: i32) {
        let (w, h) = (i64::from(self.width), i64::from(self.height));
        let n = self.cells.len() as i64;
        let offset = i64::from(dy) * w + i64::from(dx);
        if n == 0 || offset == 0 {
            return;
        }
        let mut old = std::mem::take(&mut self.cells);
        let mut new = vec![Vec::new(); n as usize];
        for (j, cell) in new.iter_mut().enumerate() {
            let source = j as i64 - offset;
            if (0..n).contains(&source) {
                *cell = std::mem::take(&mut old[source as usize]);
            }
        }
        // Both branches walk h − dy rows.
        let rows = (h - i64::from(dy)).max(0);
        let first_row = if offset > 0 { i64::from(dy) } else { 0 };
        let mut clear = |start: i64, count: i64| {
            for k in (start..start + count).filter(|k| (0..n).contains(k)) {
                new[k as usize].clear();
            }
        };
        if dx > 0 {
            for r in 0..rows {
                clear((first_row + r) * w, i64::from(dx));
            }
        } else if dx < 0 {
            let mut start = i64::from(dx) + w - 1;
            for _ in 0..rows {
                clear(start, -i64::from(dx));
                start += w;
            }
        }
        self.cells = new;
    }
}

/// Body state word bits, as the unit's helpers set and clear them. The
/// names are Exarust's, from what each helper does with them.
pub mod state {
    /// Asleep (a freeze clears it).
    pub const ASLEEP: u32 = 0x1;
    /// Cleared by a wake (its meaning is not established).
    pub const RESTING: u32 = 0x2;
    /// Cleared through a node tree by its own helper (its meaning is not
    /// established).
    pub const MARKED: u32 = 0x4;
    /// Deep sleep (a freeze clears it).
    pub const DEEP: u32 = 0x8;
    /// Frozen.
    pub const FROZEN: u32 = 0x40;
    /// Kinematic.
    pub const KINEMATIC: u32 = 0x100;
    /// Held (in the hand or by a power): its pairs are dropped.
    pub const HELD: u32 = 0x400;
    /// Out of the world.
    pub const DISABLED: u32 = 0x800;
    /// Gravity without inertia for its own countdown (a placed body).
    pub const SETTLE: u32 = 0x1000;
    /// A node's bit: registered in a sector.
    pub const REGISTERED: u32 = 0x200_0000;
    /// A static node's bit: a removal calls the sector's node callback.
    pub const NOTIFY: u32 = 0x100;
}

/// A body's class bit for a living character body.
pub const CLASS_LIVING: u32 = 0x1000_0000;
/// The pair layer a grabbed parent's neighbours take, and a break clears.
pub const LAYER_GRAB: u32 = 0x40;

/// A sphere: centre and radius.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Bound {
    pub centre: Vec3,
    pub radius: f32,
}

impl Bound {
    /// Whether two spheres are apart, as the original tests it: the sign
    /// of `(r₁ + r₂)² − ((dx² + dy²) + dz²)`, so a NaN with its sign bit
    /// set counts as apart and touching spheres are not.
    pub fn apart(&self, other: &Bound) -> bool {
        let reach = self.radius + other.radius;
        let d = self.centre - other.centre;
        let gap = reach * reach - ((d.x * d.x + d.y * d.y) + d.z * d.z);
        gap.is_sign_negative()
    }
}

/// One body as the unit's own helpers see it.
#[derive(Debug, Clone, PartialEq)]
pub struct Body {
    pub node: Option<usize>,
    pub state: u32,
    /// The sleep countdown a wake sets.
    pub countdown: i32,
    /// The settling countdown (gravity without inertia).
    pub settle_ticks: i32,
    pub class: u32,
    pub layer: u32,
    /// Its index in its sector's body list (a purge renumbers them): of a
    /// placement's two bodies, the lower index is tested first.
    pub index: u16,
    pub parent: Option<usize>,
    /// Its tree's root body: bodies of one root never test each other.
    pub root: Option<usize>,
    pub children: Vec<usize>,
    /// Its cell word in the body grid: the query skips a body whose word
    /// is 0 (never linked, or in the first cell).
    pub cell: u32,
    /// Whether it is linked into its cell's list.
    pub linked: bool,
    pub links: Vec<LinkRef>,
    /// The held counter a hold resets.
    pub held_ticks: i32,
    pub rest: [Vec3; 4],
    /// The particles now, at the tick's start and as the solver last
    /// placed them.
    pub positions: [Vec3; 4],
    pub previous: [Vec3; 4],
    pub start: [Vec3; 4],
    /// The particles' two per-tick motion buffers a freeze empties.
    pub motion: [[Vec3; 4]; 2],
    pub matrix: Affine3A,
    pub previous_matrix: Affine3A,
    /// The bound sphere's centre in the body's own space, and its weights
    /// on the particles.
    pub bound_offset: Vec3,
    pub bound_weights: [f32; 4],
    /// The bound sphere now (its centre the matrix times the offset): the
    /// grid places the body by it.
    pub bound: Bound,
    /// The bound sphere at the tick's start (its centre the weights on the
    /// start particles): a placement tests it against the others' bound.
    pub start_bound: Bound,
}

impl Body {
    /// A body with four particles at `rest`, at rest, out of the world.
    pub fn new(rest: [Vec3; 4]) -> Self {
        Self {
            node: None,
            state: 0,
            countdown: 0,
            settle_ticks: 0,
            class: 0,
            layer: 0,
            index: 0,
            parent: None,
            root: None,
            children: Vec::new(),
            cell: 0,
            linked: false,
            links: Vec::new(),
            held_ticks: 0,
            rest,
            positions: rest,
            previous: rest,
            start: rest,
            motion: [[Vec3::ZERO; 4]; 2],
            matrix: Affine3A::IDENTITY,
            previous_matrix: Affine3A::IDENTITY,
            bound_offset: Vec3::ZERO,
            bound_weights: [0.25; 4],
            bound: Bound::default(),
            start_bound: Bound::default(),
        }
    }

    /// Its matrix from its particles ([`particle_matrix`]).
    pub fn transform(&self) -> Affine3A {
        particle_matrix(&self.rest, &self.positions)
    }

    /// Its root body, itself for a root.
    fn root_or(&self, me: usize) -> usize {
        self.root.unwrap_or(me)
    }
}

/// The matrix from four particles, as the original's helper makes it: the
/// particles times the inverse of the rest points' homogeneous basis, each
/// entry summed `((p₀b₀ + p₁b₁) + p₂b₂) + p₃b₃`. It is the affine map that
/// takes the rest points to the particles, not orthonormalised (a stretched
/// body gives a stretched matrix). The basis's inverse is Exarust's (the
/// original stores it with the body).
pub fn particle_matrix(rest: &[Vec3; 4], positions: &[Vec3; 4]) -> Affine3A {
    let basis = Mat4::from_cols(
        rest[0].extend(1.0),
        rest[1].extend(1.0),
        rest[2].extend(1.0),
        rest[3].extend(1.0),
    );
    let det = basis.determinant();
    if !det.is_finite() || det == 0.0 {
        // Exarust's guard: a flat rest tetrahedron has no basis.
        return Affine3A::IDENTITY;
    }
    // inverse.row(k)[i]: particle k's weight in the matrix's row i (rows 0
    // to 2 the axes, row 3 the translation).
    let inverse = basis.inverse();
    let row = |i: usize| -> Vec3 {
        let b = Vec4::new(
            inverse.row(0)[i],
            inverse.row(1)[i],
            inverse.row(2)[i],
            inverse.row(3)[i],
        );
        ((positions[0] * b.x + positions[1] * b.y) + positions[2] * b.z) + positions[3] * b.w
    };
    Affine3A::from_mat3_translation(Mat3::from_cols(row(0), row(1), row(2)), row(3))
}

/// The weighted point `((w₀p₀ + w₁p₁) + (w₂p₂ + w₃p₃))`, as the original's
/// helper sums it.
fn weighted(weights: [f32; 4], points: &[Vec3; 4]) -> Vec3 {
    (points[0] * weights[0] + points[1] * weights[1])
        + (points[2] * weights[2] + points[3] * weights[3])
}

/// A body link as the unit's grab helper keeps it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkRef {
    /// The parent body; none for a link to the world.
    pub parent: Option<usize>,
    pub flags: u32,
}

/// Link flags the grab helper reads.
pub mod link {
    /// A breakable grab (a blade in flesh, a hand's grip).
    pub const GRAB: u32 = 0x1000;
    /// A grab the helper passes over (its meaning is not established).
    pub const KEPT: u32 = 0x10_0000;
}

/// A scene node as the unit's helpers see it.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub scene: u32,
    pub state: u32,
    pub body: Option<usize>,
    pub children: Vec<usize>,
    /// The node's translation (cm): a sector's node places the sector.
    pub position: Vec3,
    /// The flat cells of the node grid a static node is linked in (one,
    /// or several for a node spanning cells).
    pub cells: Vec<usize>,
}

impl Node {
    /// A node of `scene` with no body and no children.
    pub fn new(scene: u32) -> Self {
        Self {
            scene,
            state: 0,
            body: None,
            children: Vec::new(),
            position: Vec3::ZERO,
            cells: Vec::new(),
        }
    }
}

/// A body pair, by its two bodies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BodyPair {
    pub a: usize,
    pub b: usize,
}

/// The bodies and nodes of a scene, the records the helpers act on.
#[derive(Debug, Default, Clone)]
pub struct World {
    pub bodies: Vec<Body>,
    pub nodes: Vec<Node>,
}

impl World {
    /// A node's body, if it has one.
    pub fn body_of(&self, node: usize) -> Option<usize> {
        self.nodes.get(node).and_then(|n| n.body)
    }

    fn node_children(&self, node: usize) -> Vec<usize> {
        self.nodes
            .get(node)
            .map(|n| n.children.clone())
            .unwrap_or_default()
    }

    fn body_children(&self, body: usize) -> Vec<usize> {
        self.bodies
            .get(body)
            .map(|b| b.children.clone())
            .unwrap_or_default()
    }

    /// A body's node's scene.
    fn scene_of_body(&self, body: usize) -> Option<u32> {
        let node = self.bodies.get(body)?.node?;
        Some(self.nodes.get(node)?.scene)
    }

    /// Wakes a node's body tree to `ticks`: the node's body clears its
    /// resting bit and takes the countdown, and every child node is woken
    /// in turn; a node without a body stops its branch.
    pub fn wake(&mut self, node: usize, ticks: i32) {
        let Some(body) = self.body_of(node) else {
            return;
        };
        self.bodies[body].state &= !state::RESTING;
        self.bodies[body].countdown = ticks;
        for child in self.node_children(node) {
            self.wake(child, ticks);
        }
    }

    /// Wakes a body's whole tree (from its root body's node) to `ticks`
    /// when its countdown is under that.
    pub fn wake_at_least(&mut self, body: usize, ticks: i32) {
        let Some(b) = self.bodies.get(body) else {
            return;
        };
        if b.countdown >= ticks {
            return;
        }
        let root = b.root_or(body);
        if let Some(node) = self.bodies.get(root).and_then(|r| r.node) {
            self.wake(node, ticks);
        }
    }

    /// Gives every living body from `node` down the settling state for
    /// `ticks` (gravity without inertia, its own countdown): only bodies
    /// whose class has [`CLASS_LIVING`], through every child node.
    pub fn settle(&mut self, node: usize, ticks: i32) {
        if let Some(body) = self.body_of(node)
            && self.bodies[body].class & CLASS_LIVING != 0
        {
            self.bodies[body].state |= state::SETTLE;
            self.bodies[body].settle_ticks = ticks;
        }
        for child in self.node_children(node) {
            self.settle(child, ticks);
        }
    }

    /// Freezes a body where `at` puts its rest points, or where its
    /// particles are: clears asleep and deep, sets frozen and kinematic,
    /// puts its particles now, at the tick's start and as last placed at
    /// the same points, empties its two motion buffers, takes the matrix
    /// as both the current and the previous one, moves its bound spheres
    /// with it, and sets its countdown to two.
    pub fn freeze(&mut self, body: usize, at: Option<Affine3A>) {
        let Some(b) = self.bodies.get_mut(body) else {
            return;
        };
        b.state = (b.state & !(state::ASLEEP | state::DEEP)) | state::FROZEN | state::KINEMATIC;
        match at {
            Some(matrix) => {
                b.start = b.rest.map(|r| matrix.transform_point3(r));
                b.matrix = matrix;
            }
            None => {
                b.start = b.positions;
                b.matrix = particle_matrix(&b.rest, &b.positions);
            }
        }
        b.previous = b.start;
        b.positions = b.start;
        b.motion = [[Vec3::ZERO; 4]; 2];
        b.previous_matrix = b.matrix;
        b.bound.centre = b.matrix.transform_point3(b.bound_offset);
        b.start_bound.centre = weighted(b.bound_weights, &b.start);
        b.countdown = 2;
    }

    /// Clears a body's frozen and kinematic bits.
    pub fn unfreeze(&mut self, body: usize) {
        if let Some(b) = self.bodies.get_mut(body) {
            b.state &= !(state::FROZEN | state::KINEMATIC);
        }
    }

    /// Disables a body and its child bodies, down the body tree.
    pub fn disable_tree(&mut self, body: usize) {
        if let Some(b) = self.bodies.get_mut(body) {
            b.state |= state::DISABLED;
        }
        for child in self.body_children(body) {
            self.disable_tree(child);
        }
    }

    /// Enables a body and its child bodies, down the body tree.
    pub fn enable_tree(&mut self, body: usize) {
        if let Some(b) = self.bodies.get_mut(body) {
            b.state &= !state::DISABLED;
        }
        for child in self.body_children(body) {
            self.enable_tree(child);
        }
    }

    /// Disables the bodies from `node` down through every child node: a
    /// living body (class [`CLASS_LIVING`]) is also made kinematic.
    pub fn disable_bodies(&mut self, node: usize) {
        if let Some(body) = self.body_of(node) {
            let b = &mut self.bodies[body];
            b.state |= if b.class & CLASS_LIVING != 0 {
                state::DISABLED | state::KINEMATIC
            } else {
                state::DISABLED
            };
        }
        for child in self.node_children(node) {
            self.disable_bodies(child);
        }
    }

    /// Clears the grab layer of a body's parent, of the body and of its
    /// children: the break of a grab.
    pub fn clear_pair_layer(&mut self, body: usize) {
        if let Some(parent) = self.bodies.get(body).and_then(|b| b.parent)
            && let Some(p) = self.bodies.get_mut(parent)
        {
            p.layer &= !LAYER_GRAB;
        }
        if let Some(b) = self.bodies.get_mut(body) {
            b.layer &= !LAYER_GRAB;
        }
        for child in self.body_children(body) {
            if let Some(c) = self.bodies.get_mut(child) {
                c.layer &= !LAYER_GRAB;
            }
        }
    }

    /// Disables the node's body and, for a living one, drops its first
    /// breakable grab link that is not passed over: the grab's parent body
    /// leaves the grab layer with its own parent and children, and the body
    /// leaves it too. Every child node is visited afterwards.
    pub fn drop_grab_link(&mut self, node: usize) {
        if let Some(body) = self.body_of(node) {
            self.bodies[body].state |= state::DISABLED;
            if self.bodies[body].class & CLASS_LIVING != 0
                && let Some(at) = self.bodies[body]
                    .links
                    .iter()
                    .position(|l| l.flags & link::KEPT == 0 && l.flags & link::GRAB != 0)
            {
                if let Some(parent) = self.bodies[body].links[at].parent {
                    self.clear_pair_layer(parent);
                }
                self.bodies[body].layer &= !LAYER_GRAB;
                self.bodies[body].links.remove(at);
            }
        }
        for child in self.node_children(node) {
            self.drop_grab_link(child);
        }
    }

    /// Clears the [`state::MARKED`] bit of every body from `node` down
    /// through every child node.
    pub fn clear_marks(&mut self, node: usize) {
        if let Some(body) = self.body_of(node) {
            self.bodies[body].state &= !state::MARKED;
        }
        for child in self.node_children(node) {
            self.clear_marks(child);
        }
    }

    /// Holds a body and its child bodies, down the body tree, each with
    /// its held counter reset.
    pub fn hold_tree(&mut self, body: usize) {
        if let Some(b) = self.bodies.get_mut(body) {
            b.state |= state::HELD;
            b.held_ticks = 0;
        }
        for child in self.body_children(body) {
            self.hold_tree(child);
        }
    }

    /// Releases a body and its child bodies, down the body tree.
    pub fn release_tree(&mut self, body: usize) {
        if let Some(b) = self.bodies.get_mut(body) {
            b.state &= !state::HELD;
        }
        for child in self.body_children(body) {
            self.release_tree(child);
        }
    }

    /// Removes from `pairs` every pair with a held body, swapping the last
    /// pair into each place.
    pub fn remove_held_pairs(&self, pairs: &mut Vec<BodyPair>) {
        let held = |i: usize| {
            self.bodies
                .get(i)
                .is_some_and(|b| b.state & state::HELD != 0)
        };
        let mut i = 0;
        while i < pairs.len() {
            if held(pairs[i].a) || held(pairs[i].b) {
                pairs.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    /// Whether placing `body` (and then each of its child bodies) would
    /// overlap another body of the sector, as the original's placement
    /// query walks it: a body whose cell word is 0 is not tested; else the
    /// nine cells around the cell its bound sphere's centre falls in (the
    /// cell itself first), and in each every body that is not apart from
    /// its start bound by the bound-sphere test, not of its own root, whose
    /// pair layers do not keep them apart, and not disabled. Of the two,
    /// the one with the lower sector index goes first to `overlaps` (the
    /// original's swept shape test and its insertion test, which are the
    /// sector's).
    pub fn space_occupied(
        &self,
        grid: &Grid,
        sector_cell: [i32; 2],
        body: usize,
        overlaps: &mut impl FnMut(&Body, &Body) -> bool,
    ) -> bool {
        let Some(b) = self.bodies.get(body) else {
            return false;
        };
        if b.cell == 0 {
            return false;
        }
        let root = b.root_or(body);
        // The cell the body's bound falls in; the nine offsets around it.
        let x = floor_word(b.bound.centre.x * CELL_RECIPROCAL);
        let z = floor_word(b.bound.centre.z * CELL_RECIPROCAL);
        let base = z
            .wrapping_add(sector_cell[1])
            .wrapping_mul(grid.width)
            .wrapping_add(x)
            .wrapping_add(sector_cell[0]);
        for offset in grid.offsets {
            let Some(index) = grid.flat(i64::from(base) + i64::from(offset)) else {
                continue;
            };
            for &other in grid.cell(index) {
                let Some(o) = self.bodies.get(other) else {
                    continue;
                };
                let shared = o.layer & b.layer;
                if b.start_bound.apart(&o.bound)
                    || o.root_or(other) == root
                    || (shared & !1 != 0 && layers_apart(o.layer, b.layer))
                    || o.state & state::DISABLED != 0
                {
                    continue;
                }
                let (first, second) = if o.index < b.index { (o, b) } else { (b, o) };
                if overlaps(first, second) {
                    return true;
                }
            }
        }
        b.children
            .iter()
            .any(|&child| self.space_occupied(grid, sector_cell, child, overlaps))
    }
}

/// Sector flag bits: what the simulation's update serves.
pub mod sector {
    /// Purge the nodes and bodies that left the sector's scene.
    pub const SCENE_LEFT: u32 = 0x1_0000;
    /// Reset the sector (its lists emptied, kept in the simulation).
    pub const RESET: u32 = 0x2_0000;
    /// Set by a reset.
    pub const WAS_RESET: u32 = 0x4_0000;
    /// The sector is gone: reset it and drop it from the simulation.
    pub const DEAD: u32 = 0x8_0000;
    /// Register the sector's pending nodes.
    pub const PENDING: u32 = 0x10_0000;
    /// Purge what belongs to the shared scene.
    pub const SHARED_SCENE: u32 = 0x20_0000;
    /// Purge what belongs to the simulation's scene's sub-scene.
    pub const SUB_SCENE: u32 = 0x40_0000;
    /// The bits a reset clears before it sets [`WAS_RESET`].
    pub const RESET_CLEARS: u32 = 0x37_0000;
}

/// What a purge took out, for the caller's callbacks: the static nodes
/// that asked for a notice, and the bodies (the original calls each one's
/// removal).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Purged {
    pub nodes: Vec<usize>,
    pub bodies: Vec<usize>,
}

/// One sector of a simulation: its flag word, its scene and node, its
/// three lists (static nodes, bodies, and simulated objects by their
/// node), its body pairs, its pending nodes and, when it uses the grids,
/// the cell its node falls in.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Sector {
    pub flags: u32,
    pub scene: u32,
    /// The sector's own node (it places the sector in the grids).
    pub node: Option<usize>,
    /// Static nodes, linked in the node grid.
    pub nodes: Vec<usize>,
    /// Bodies, linked in the body grid; each knows its index here.
    pub bodies: Vec<usize>,
    /// Simulated objects (cloths, effects), by their nodes.
    pub objects: Vec<usize>,
    pub pairs: Vec<BodyPair>,
    /// Nodes waiting to be registered.
    pub pending: Vec<usize>,
    /// Whether the sector uses the simulation's grids.
    pub uses_grid: bool,
    /// The cell the sector's node falls in.
    pub cell: [i32; 2],
    /// The simulation tick of the sector's last step.
    pub tick: u32,
}

impl Sector {
    /// A sector of `scene`.
    pub fn new(scene: u32) -> Self {
        Self {
            scene,
            ..Self::default()
        }
    }

    /// Caches the sector's cell from the grid and its node's translation.
    pub fn cache_grid(&mut self, world: &World, grid: &Grid) {
        let at = self
            .node
            .and_then(|n| world.nodes.get(n))
            .map_or(Vec3::ZERO, |n| n.position);
        self.cell = grid.sector_cell(at);
    }

    /// Resets the sector: its three lists and its pairs are emptied, its
    /// cell is cached again when it uses the grids, and its flags lose the
    /// purge, reset and registration bits and gain [`sector::WAS_RESET`].
    pub fn reset(&mut self, world: &World, grid: &Grid) {
        self.bodies.clear();
        self.objects.clear();
        self.nodes.clear();
        self.pairs.clear();
        if self.uses_grid {
            self.cache_grid(world, grid);
        }
        self.flags = (self.flags & !sector::RESET_CLEARS) | sector::WAS_RESET;
    }

    /// Unlinks a static node from every node-grid cell it is in.
    fn unlink_node(world: &mut World, grids: &mut Grids, node: usize) {
        if let Some(n) = world.nodes.get_mut(node) {
            for cell in n.cells.drain(..) {
                grids.nodes.remove(cell, node);
            }
        }
    }

    /// Unlinks a body from its body-grid cell.
    fn unlink_body(world: &mut World, grids: &mut Grids, body: usize) {
        if let Some(b) = world.bodies.get_mut(body)
            && b.linked
        {
            grids.bodies.remove(b.cell as usize, body);
            b.linked = false;
        }
    }

    fn unregister(world: &mut World, node: Option<usize>) {
        if let Some(n) = node.and_then(|n| world.nodes.get_mut(n)) {
            n.state &= !state::REGISTERED;
        }
    }

    /// Removes from the sector what `gone` marks, in the original's four
    /// sweeps: static nodes (swap-removed, unlinked from the node grid,
    /// unregistered, and noticed when `notify` and the node asks), bodies
    /// (compacted in order and renumbered, unlinked from the body grid,
    /// unregistered), pairs with a body for which `pair_gone` holds
    /// (swap-removed), and objects (shifted down, unregistered).
    fn sweep(
        &mut self,
        world: &mut World,
        grids: &mut Grids,
        gone: impl Fn(&World, u32) -> bool,
        pair_gone: impl Fn(&World, u32) -> bool,
        notify: bool,
    ) -> Purged {
        let mut purged = Purged::default();
        let mut i = 0;
        while i < self.nodes.len() {
            let node = self.nodes[i];
            let scene = world.nodes.get(node).map_or(u32::MAX, |n| n.scene);
            if !gone(world, scene) {
                i += 1;
                continue;
            }
            if self.uses_grid {
                Self::unlink_node(world, grids, node);
            }
            if notify
                && world
                    .nodes
                    .get(node)
                    .is_some_and(|n| n.state & state::NOTIFY != 0)
            {
                purged.nodes.push(node);
            }
            Self::unregister(world, Some(node));
            self.nodes.swap_remove(i);
        }
        let mut kept = 0;
        for i in 0..self.bodies.len() {
            let body = self.bodies[i];
            let scene = world.scene_of_body(body).unwrap_or(u32::MAX);
            if gone(world, scene) {
                if self.uses_grid {
                    Self::unlink_body(world, grids, body);
                }
                purged.bodies.push(body);
                let node = world.bodies.get(body).and_then(|b| b.node);
                Self::unregister(world, node);
            } else {
                self.bodies[kept] = body;
                if let Some(b) = world.bodies.get_mut(body) {
                    b.index = kept as u16;
                }
                kept += 1;
            }
        }
        self.bodies.truncate(kept);
        let mut i = 0;
        while i < self.pairs.len() {
            let p = self.pairs[i];
            let a = world.scene_of_body(p.a).unwrap_or(u32::MAX);
            let b = world.scene_of_body(p.b).unwrap_or(u32::MAX);
            if pair_gone(world, a) || pair_gone(world, b) {
                self.pairs.swap_remove(i);
            } else {
                i += 1;
            }
        }
        let mut i = 0;
        while i < self.objects.len() {
            let node = self.objects[i];
            let scene = world.nodes.get(node).map_or(u32::MAX, |n| n.scene);
            if gone(world, scene) {
                Self::unregister(world, Some(node));
                self.objects.remove(i);
            } else {
                i += 1;
            }
        }
        purged
    }

    /// Purges what is no longer in the sector's scene (the
    /// [`sector::SCENE_LEFT`] flag, which it clears). The pair sweep
    /// removes every pair with a body still in the scene, as the original
    /// compares it (the pairs are found again on the next step).
    pub fn purge_left(&mut self, world: &mut World, grids: &mut Grids) -> Purged {
        self.flags &= !sector::SCENE_LEFT;
        let scene = self.scene;
        self.sweep(world, grids, |_, s| s != scene, |_, s| s == scene, true)
    }

    /// Purges everything of `scene` (a shared or sub-scene removed): its
    /// static nodes, bodies, pairs and objects. No node is noticed.
    pub fn purge_scene(&mut self, world: &mut World, grids: &mut Grids, scene: u32) -> Purged {
        self.sweep(world, grids, |_, s| s == scene, |_, s| s == scene, false)
    }

    /// Appends a batch of nodes to the pending list and asks the update to
    /// register them (the original's sector list append).
    pub fn add_nodes(&mut self, nodes: &[usize]) {
        self.pending.extend_from_slice(nodes);
        self.flags |= sector::PENDING;
    }

    /// Takes the sector's pending nodes for registration (the
    /// [`sector::PENDING`] flag, which it clears).
    pub fn take_pending(&mut self) -> Vec<usize> {
        self.flags &= !sector::PENDING;
        std::mem::take(&mut self.pending)
    }
}

/// A simulation's two grids: one for the sectors' static nodes, one for
/// their bodies (the one a placement queries).
#[derive(Debug, Clone, PartialEq)]
pub struct Grids {
    pub nodes: Grid,
    pub bodies: Grid,
}

impl Grids {
    pub fn new(size: GridSize) -> Self {
        Self {
            nodes: Grid::new(size),
            bodies: Grid::new(size),
        }
    }
}

/// What a sector's turn in the update did.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SectorTurn {
    pub purged: Purged,
    /// Nodes to register.
    pub pending: Vec<usize>,
}

/// The simulation flag bit that asks for the grids to be rebuilt.
pub const REBUILD_GRID: u32 = 0x2_0000;

/// A simulation: one scene's clock, its sectors and its grids.
#[derive(Debug, Clone)]
pub struct Simulation {
    pub clock: Clock,
    pub flags: u32,
    pub sectors: Vec<Sector>,
    pub grids: Grids,
    /// A grid shift waiting for the next update (cells).
    pub shift: [i32; 2],
    /// Bodies a listener is watching.
    pub watch: Vec<usize>,
    /// The listeners, called with a removed watch entry.
    pub listeners: Vec<usize>,
}

impl Simulation {
    /// A simulation of one scene at the given step, with grids of `size`.
    pub fn new(step: u32, size: GridSize) -> Self {
        Self {
            clock: Clock {
                step,
                ..Clock::default()
            },
            flags: 0,
            sectors: Vec::new(),
            grids: Grids::new(size),
            shift: [0, 0],
            watch: Vec::new(),
            listeners: Vec::new(),
        }
    }

    /// Rebuilds the grids for `size`: new tables when the size changed,
    /// every cell emptied, and every sector's cell cached again (the
    /// bodies link themselves back as they step).
    pub fn build_grid(&mut self, world: &mut World, size: GridSize) {
        if size.width != self.grids.nodes.width || size.height != self.grids.nodes.height {
            self.grids = Grids::new(size);
        }
        self.grids.nodes.clear();
        self.grids.bodies.clear();
        for b in &mut world.bodies {
            b.linked = false;
        }
        for n in &mut world.nodes {
            n.cells.clear();
        }
        for s in &mut self.sectors {
            s.cache_grid(world, &self.grids.bodies);
        }
    }

    /// Adds a body to the watch list, last.
    pub fn watch_add(&mut self, body: usize) {
        self.watch.push(body);
    }

    /// Removes a body from the watch list (the last one takes its place)
    /// and reports whether it was there; the original then calls every
    /// listener with the body (the listeners are returned here).
    pub fn watch_remove(&mut self, body: usize) -> (bool, Vec<usize>) {
        let Some(at) = self.watch.iter().position(|&b| b == body) else {
            return (false, Vec::new());
        };
        self.watch.swap_remove(at);
        (true, self.listeners.clone())
    }

    /// The simulation's update: the pending grid shift (both grids moved,
    /// every sector's cell placed again), then each sector's flags in the
    /// original's order (a dead sector is reset and removed, the others
    /// keeping their order; a reset one is reset; otherwise the scene-left
    /// purge, the shared-scene purge, the sub-scene purge and the pending
    /// registration), then the asked-for grid rebuild. `shared` and `sub`
    /// are the shared scene and the simulation's sub-scene.
    pub fn update(
        &mut self,
        world: &mut World,
        shared: u32,
        sub: u32,
        size: GridSize,
    ) -> Vec<SectorTurn> {
        if self.shift != [0, 0] {
            let [dx, dy] = self.shift;
            self.grids.nodes.shift(dx, dy);
            self.grids.bodies.shift(dx, dy);
            for s in &mut self.sectors {
                s.cache_grid(world, &self.grids.bodies);
            }
            self.shift = [0, 0];
        }
        let mut turns = Vec::new();
        let mut i = 0;
        while i < self.sectors.len() {
            let grids = &mut self.grids;
            let s = &mut self.sectors[i];
            if s.flags & sector::DEAD != 0 {
                s.reset(world, &grids.bodies);
                self.sectors.remove(i);
                continue;
            }
            let mut turn = SectorTurn::default();
            if s.flags & sector::RESET != 0 {
                s.reset(world, &grids.bodies);
            } else {
                if s.flags & sector::SCENE_LEFT != 0 {
                    let p = s.purge_left(world, grids);
                    turn.purged.nodes.extend(p.nodes);
                    turn.purged.bodies.extend(p.bodies);
                }
                if s.flags & sector::SHARED_SCENE != 0 {
                    s.flags &= !sector::SHARED_SCENE;
                    turn.purged
                        .bodies
                        .extend(s.purge_scene(world, grids, shared).bodies);
                }
                if s.flags & sector::SUB_SCENE != 0 {
                    s.flags &= !sector::SUB_SCENE;
                    turn.purged
                        .bodies
                        .extend(s.purge_scene(world, grids, sub).bodies);
                }
                if s.flags & sector::PENDING != 0 {
                    turn.pending = s.take_pending();
                }
            }
            turns.push(turn);
            i += 1;
        }
        if self.flags & REBUILD_GRID != 0 {
            self.build_grid(world, size);
            self.flags &= !REBUILD_GRID;
        }
        turns
    }

    /// One fixed step's bookkeeping, in the original's order: the update,
    /// then the tick and the game time advance, and every sector takes the
    /// new tick.
    pub fn turn(&mut self, world: &mut World, shared: u32, sub: u32) -> Vec<SectorTurn> {
        let size = GridSize {
            width: self.grids.nodes.width,
            height: self.grids.nodes.height,
        };
        let turns = self.update(world, shared, sub, size);
        self.clock.advance();
        for s in &mut self.sectors {
            s.tick = self.clock.tick;
        }
        turns
    }
}

#[cfg(test)]
#[path = "dynamics_test.rs"]
mod tests;
