//! The dynamics world: the original's `RF_Dynamics` unit — the simulation's
//! fixed-step clock, its gravity and the environments' velocity bands, the
//! grid the sectors keep their bodies in, the per-node and per-body state
//! helpers, the removal of nodes that left a scene, and the overlap query a
//! placement asks before it puts a body down.
//!
//! The original runs one [`Simulation`] per scene: a clock in milliseconds
//! and ticks, a list of sectors, and one shared 600 cm grid the sectors
//! register their bodies in. Each fixed step takes the simulation that is
//! due, drops its backlog past four steps, and updates its sectors: the
//! grid shifts when the scene moved it, the bodies that left the scene are
//! purged, and the pending flags are served ([`Simulation::update`]).
//!
//! This module holds that unit's rules as data and algorithms over plain
//! records ([`World`], [`Sector`], [`Grid`]), so they can be reasoned about
//! and tested on their own; the ECS body solver in [`crate::body`] runs the
//! physical step itself and keeps its own sleeping and grouping state.
//!
//! Lengths are the original's centimetres and times its milliseconds.

use bevy::math::{Affine3A, Mat3, Vec3};

use crate::body::{RETENTION, RETENTION_SPEEDS};

/// One fixed step: 12 ms, 83 Hz.
pub const STEP_MS: u32 = 12;
/// Steps a late simulation may fall behind before it drops the rest.
pub const MAX_STEPS_BEHIND: u32 = 4;
/// The world's grid cell (cm): a body's cell is its node's position over
/// this, rounded to even, plus the grid's half.
pub const CELL_CM: f32 = 600.0;
/// The original's default gravity (m/s²), as the body solver's.
pub use crate::body::GRAVITY;

/// A simulation's clock: the tick counter, the game time and step in
/// milliseconds, the last run time and whether it is paused.
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
    /// The soonest a simulation will be due, in milliseconds.
    Wait(u32),
}

impl Clock {
    /// Whether the step at `time + step` is due at `now` (the time since
    /// the simulation last ran is at least the step's end).
    pub fn due(&self, now: u32) -> bool {
        let late = now.wrapping_sub(self.last);
        self.time.wrapping_add(self.step) <= late
    }

    /// How many steps behind `now` is: the time since the last run less
    /// the next step's end (0 when due).
    pub fn behind(&self, now: u32) -> u32 {
        let late = now.wrapping_sub(self.last);
        late.saturating_sub(self.time.wrapping_add(self.step))
    }

    /// Drops the backlog of a simulation more than four steps late and not
    /// paused, moving its last run time forward to now less its step.
    pub fn drop_backlog(&mut self, now: u32) {
        if self.paused || self.behind(now) <= self.step.saturating_mul(MAX_STEPS_BEHIND) {
            return;
        }
        self.last = now.wrapping_sub(self.step);
    }

    /// One step: the tick and the game time advance by the step.
    pub fn advance(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        self.time = self.time.wrapping_add(self.step);
    }

    /// The scaled time since the last `mark`, as the original's time-scale
    /// helper returns it: the unsigned game time less the mark, as a
    /// 64-bit signed difference, converted to a float, scaled, and
    /// converted back with the processor's nearest-even rounding.
    pub fn scale_since(&self, mark: i32, scale: f32) -> i32 {
        let delta = i64::from(self.time) - i64::from(mark);
        let scaled = (delta as f32) * scale;
        let rounded = scaled.round_ties_even();
        if rounded.is_nan() || rounded >= 2_147_483_648.0 || rounded < -2_147_483_648.0 {
            0
        } else {
            rounded as i32
        }
    }
}

/// The step driver's choice over a list of simulations: the first whose
/// step is due runs, the others' lateness updating the wait; the soonest
/// end wins. A simulation with no sectors is still stepped.
pub fn pick_due(sims: &mut [Clock], now: u32) -> Due {
    let mut wait = u32::MAX;
    for sim in sims.iter_mut() {
        let late = now.wrapping_sub(sim.last);
        let end = sim.time.wrapping_add(sim.step);
        if end <= late {
            sim.drop_backlog(now);
            return Due::Now;
        }
        wait = wait.min(end - late);
    }
    if wait == u32::MAX {
        Due::Wait(0)
    } else {
        Due::Wait(wait)
    }
}

/// A body environment's gravity (m/s²) and velocity bands: the retention
/// per second above each threshold (cm/s), and the retention below the
/// lower one.
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
    /// The default environment every body uses: gravity and the three
    /// per-second retentions 0.426, 0.25 and 0.05 above 40, above 20 and
    /// at rest (cm/s).
    pub const DEFAULT: Self = Self {
        gravity: GRAVITY,
        fast: RETENTION[0],
        slow: RETENTION[1],
        resting: RETENTION[2],
        fast_speed: RETENTION_SPEEDS[0],
        slow_speed: RETENTION_SPEEDS[1],
    };
    /// The environment the mechanisms share: retention 0.001 a second in
    /// every band and no gravity.
    pub const MECHANISM: Self = Self {
        gravity: 0.0,
        fast: 0.001,
        slow: 0.001,
        resting: 0.001,
        fast_speed: RETENTION_SPEEDS[0],
        slow_speed: RETENTION_SPEEDS[1],
    };
    /// The same bands without gravity (an item held on the cursor carries
    /// this environment in the original; only its zero gravity is
    /// established here).
    pub const ZERO_GRAVITY: Self = Self {
        gravity: 0.0,
        ..Self::DEFAULT
    };

    /// The gravity a particle takes in one tick (cm): g·dt², the solver's
    /// per-tick unit, with dt the fixed step.
    pub fn gravity_per_tick(&self) -> Vec3 {
        let dt = 1.0 / crate::STEP_HZ as f32;
        Vec3::Y * self.gravity * dt * dt
    }

    /// The retention per tick for a body whose farthest particle moved
    /// `moved` cm in the last tick: fast above 40 cm/s, slow above 20,
    /// else resting, each its per-second value raised to a tick.
    pub fn retention(&self, moved: f32) -> f32 {
        let per_second = if moved > self.fast_speed / 83.0 {
            self.fast
        } else if moved > self.slow_speed / 83.0 {
            self.slow
        } else {
            self.resting
        };
        per_second.powf(1.0 / 83.0)
    }
}

/// The per-axis grid dimensions: cells inside the grid, an odd number the
/// solver makes from the scene's extent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridSize {
    pub width: i32,
    pub height: i32,
}

/// The sector's shared body grid: one list of bodies per cell, Z-fastest
/// rows like the original's flat pointer table, with the nine neighbour
/// offsets the overlap query walks.
#[derive(Debug, Clone, PartialEq)]
pub struct Grid {
    pub width: i32,
    pub height: i32,
    /// The cell a node at the grid centre falls in: half the width and
    /// height, rounded down.
    pub bias: [i32; 2],
    cells: Vec<Vec<usize>>,
}

impl Grid {
    /// An empty grid of `size` cells, the cell lists all empty.
    pub fn new(size: GridSize) -> Self {
        let width = size.width.max(0);
        let height = size.height.max(0);
        let len = (width as usize).saturating_mul(height as usize);
        Self {
            width,
            height,
            bias: [width >> 1, height >> 1],
            cells: vec![Vec::new(); len],
        }
    }

    /// The cell a node at `position` (cm, the node's translation) sits in:
    /// each axis over 600 cm rounded to even and moved by the bias. None
    /// when the cell falls outside the grid.
    pub fn locate(&self, position: Vec3) -> Option<(i32, i32)> {
        if self.width <= 0 || self.height <= 0 {
            return None;
        }
        let x = (position.x / CELL_CM).round_ties_even() as i32 + self.bias[0];
        let y = (position.z / CELL_CM).round_ties_even() as i32 + self.bias[1];
        ((0..self.width).contains(&x) && (0..self.height).contains(&y)).then_some((x, y))
    }

    /// The flat index of a cell, x across and y down; None outside.
    pub fn index(&self, cell: (i32, i32)) -> Option<usize> {
        let (x, y) = cell;
        ((0..self.width).contains(&x) && (0..self.height).contains(&y))
            .then(|| (y * self.width + x) as usize)
    }

    /// The bodies in a cell, in the order they were linked.
    pub fn cell(&self, cell: (i32, i32)) -> &[usize] {
        self.index(cell)
            .and_then(|i| self.cells.get(i))
            .map_or(&[], |c| c)
    }

    /// The cell lists, x across and y down.
    pub fn cells(&self) -> &[Vec<usize>] {
        &self.cells
    }

    /// Adds a body to a cell (no duplicate check: the original's cell links
    /// come from the body's own step).
    pub fn insert(&mut self, cell: (i32, i32), body: usize) {
        if let Some(i) = self.index(cell)
            && let Some(list) = self.cells.get_mut(i)
        {
            list.push(body);
        }
    }

    /// Removes a body from a cell's list, if it is there.
    pub fn remove(&mut self, cell: (i32, i32), body: usize) {
        if let Some(i) = self.index(cell)
            && let Some(list) = self.cells.get_mut(i)
            && let Some(at) = list.iter().position(|&b| b == body)
        {
            list.remove(at);
        }
    }

    /// Empties every cell, as the original's grid build does before it
    /// re-locates the bodies.
    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            cell.clear();
        }
    }

    /// The nine cells around `cell`: dy from −1 to 1, dx from −1 to 1 (the
    /// original's row-major offsets), those inside the grid only.
    pub fn neighbours(&self, cell: (i32, i32)) -> impl Iterator<Item = usize> {
        let mut out = Vec::with_capacity(9);
        for dy in -1..=1 {
            for dx in -1..=1 {
                if let Some(i) = self.index((cell.0 + dx, cell.1 + dy)) {
                    out.push(i);
                }
            }
        }
        out.into_iter()
    }

    /// Shifts the grid by `(dx, dy)` cells: every cell's list moves by the
    /// flat offset `dy·width + dx` (a cell at index i takes what was at
    /// i + offset), what falls outside is dropped, and the strip the
    /// horizontal shift leaves is cleared (the first `dx` cells of every
    /// row from `dy` down, or the last `−dx` when `dx` is negative). This
    /// is the original's flat-row move and its row fix-up.
    pub fn shift(&mut self, dx: i32, dy: i32) {
        let (w, h) = (self.width, self.height);
        if w <= 0 || h <= 0 {
            return;
        }
        let n = (w as usize) * (h as usize);
        let offset = dy * w + dx;
        let mut old = std::mem::take(&mut self.cells);
        let mut new = vec![Vec::new(); n];
        for (i, cell) in new.iter_mut().enumerate() {
            let source = i as i64 + i64::from(offset);
            if (0..n as i64).contains(&source) {
                *cell = std::mem::take(&mut old[source as usize]);
            }
        }
        for y in dy.max(0)..h {
            for x in 0..w {
                if dx > 0 && x < dx || dx < 0 && x >= w + dx {
                    new[(y * w + x) as usize].clear();
                }
            }
        }
        self.cells = new;
    }
}

/// Body state word bits, as the unit's helpers set and clear them.
pub mod state {
    /// Asleep.
    pub const ASLEEP: u32 = 0x1;
    /// Awake (a wake clears it).
    pub const AWAKE: u32 = 0x2;
    /// In the hand: its pairs are dropped.
    pub const HELD: u32 = 0x4;
    /// Deep sleep.
    pub const DEEP: u32 = 0x8;
    /// Frozen.
    pub const FROZEN: u32 = 0x40;
    /// Kinematic.
    pub const KINEMATIC: u32 = 0x100;
    /// Not in the world.
    pub const DISABLED: u32 = 0x800;
    /// Gravity without inertia for a countdown (a placed body).
    pub const SETTLE: u32 = 0x1000;
    /// The node is registered in a sector.
    pub const REGISTERED: u32 = 0x200_0000;
}

/// A body's class bit for a living character body (the setup's settle
/// filter and the grip's parent check).
pub const CLASS_LIVING: u32 = 0x1000_0000;
/// The pair layer a grabbed parent's neighbours take, and a break clears.
pub const LAYER_GRAB: u32 = 0x40;

/// One body as the unit's own helpers see it: its state word, sleep
/// countdown, class and pair layer, its place in the tree, its links and
/// the cell it was last located in.
#[derive(Debug, Clone, PartialEq)]
pub struct Body {
    pub node: Option<usize>,
    pub scene: u32,
    pub state: u32,
    pub countdown: i32,
    pub class: u32,
    pub layer: u32,
    /// Larger bodies move first in a placement's overlap query.
    pub order: u32,
    pub parent: Option<usize>,
    pub root: Option<usize>,
    pub children: Vec<usize>,
    pub cell: Option<(i32, i32)>,
    pub links: Vec<LinkRef>,
    /// The held counter at the body's `+0x348`; a hold resets it.
    pub held: i32,
    pub rest: [Vec3; 4],
    pub positions: [Vec3; 4],
    pub previous: [Vec3; 4],
    pub matrix: Affine3A,
}

impl Body {
    /// A body with four particles at `rest`, at rest, out of the world.
    pub fn new(rest: [Vec3; 4]) -> Self {
        let positions = rest;
        Self {
            node: None,
            scene: 0,
            state: 0,
            countdown: 0,
            class: CLASS_LIVING,
            layer: 0,
            order: 0,
            parent: None,
            root: None,
            children: Vec::new(),
            cell: None,
            links: Vec::new(),
            held: 0,
            rest,
            positions,
            previous: positions,
            matrix: Affine3A::IDENTITY,
        }
    }

    /// Its rigid transform from its particles, as the solver works it out
    /// (the nearest rotation to the particle edges over the rest edges).
    pub fn transform(&self) -> Affine3A {
        transform_of(&self.rest, &self.positions)
    }
}

/// A rigid transform from four rest points and their current positions.
fn transform_of(rest: &[Vec3; 4], positions: &[Vec3; 4]) -> Affine3A {
    let rest_edges = Mat3::from_cols(rest[1] - rest[0], rest[2] - rest[0], rest[3] - rest[0]);
    let edges = Mat3::from_cols(
        positions[1] - positions[0],
        positions[2] - positions[0],
        positions[3] - positions[0],
    );
    let m = match rest_edges.try_inverse() {
        Some(inverse) => edges * inverse,
        None => Mat3::IDENTITY,
    };
    let x = m.x_axis.normalize_or(Vec3::X);
    let y = (m.y_axis - x * x.dot(m.y_axis)).normalize_or(Vec3::Y);
    let r = Mat3::from_cols(x, y, x.cross(y));
    let rest_centre: Vec3 = rest.iter().copied().sum::<Vec3>() * 0.25;
    let centre: Vec3 = positions.iter().copied().sum::<Vec3>() * 0.25;
    Affine3A::from_mat3_translation(r, centre - r * rest_centre)
}

/// A body link as the unit's grab helpers keep it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkRef {
    /// The parent body; none for a link to the world.
    pub parent: Option<usize>,
    /// The link flags: a breakable grab is [`link::GRAB`].
    pub flags: u32,
}

/// Link flags the unit's own helpers read.
pub mod link {
    /// A breakable grab (a blade in flesh, a hand's grip): its parent's
    /// neighbours are in the grab layer.
    pub const GRAB: u32 = 0x1000;
    /// A grab that was already re-anchored (not one the removal takes).
    pub const REANCHORED: u32 = 0x100_000;
}

/// A scene node as the unit's tree helpers see it: its scene, state word,
/// body and children.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub scene: u32,
    pub state: u32,
    pub body: Option<usize>,
    pub children: Vec<usize>,
    /// The node's translation: the grid places its body by it (cm).
    pub position: Vec3,
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
        }
    }
}

/// A body pair, by its two bodies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BodyPair {
    pub a: usize,
    pub b: usize,
}

/// A simulated object (cloth, an effect) and the node it belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimObject {
    pub node: usize,
}

/// The bodies, nodes, pairs and simulated objects of a scene, the records
/// the unit's helpers act on.
#[derive(Debug, Default, Clone)]
pub struct World {
    pub bodies: Vec<Body>,
    pub nodes: Vec<Node>,
    pub pairs: Vec<BodyPair>,
    pub sim_objects: Vec<SimObject>,
}

impl World {
    /// A node's body, if it has one.
    pub fn body_of(&self, node: usize) -> Option<usize> {
        self.nodes.get(node).and_then(|n| n.body)
    }

    /// The node tree under `node`, children first, including `node`.
    fn nodes_under(&self, node: usize) -> Vec<usize> {
        let mut out = Vec::new();
        let mut stack = vec![node];
        let mut seen = vec![false; self.nodes.len()];
        while let Some(n) = stack.pop() {
            if n >= self.nodes.len() || seen[n] {
                continue;
            }
            seen[n] = true;
            out.push(n);
            stack.extend(self.nodes[n].children.iter().copied());
        }
        out
    }

    /// Wakes a node's body tree to `ticks`: every body from this node down
    /// through child nodes that have bodies (the recursion stops at a node
    /// without one) clears the awake bit and takes the countdown.
    pub fn wake(&mut self, node: usize, ticks: i32) {
        let Some(body) = self.body_of(node) else {
            return;
        };
        self.bodies[body].state &= !state::AWAKE;
        self.bodies[body].countdown = ticks;
        for child in self
            .nodes
            .get(node)
            .map(|n| n.children.clone())
            .unwrap_or_default()
        {
            if self.body_of(child).is_some() {
                self.wake(child, ticks);
            }
        }
    }

    /// Wakes a body's whole tree to `ticks` when it is not already that
    /// awake: the tree from the body's root node.
    pub fn wake_at_least(&mut self, body: usize, ticks: i32) {
        let (countdown, node) = match self.bodies.get(body) {
            Some(b) => {
                let root_node = b.root.and_then(|r| self.bodies.get(r)).and_then(|r| r.node);
                (b.countdown, root_node.or(b.node))
            }
            None => return,
        };
        if ticks <= countdown {
            return;
        }
        if let Some(node) = node {
            self.wake(node, ticks);
        }
    }

    /// Gives every living body from `node` down the settling state for
    /// `ticks` (gravity without inertia): only bodies whose class has
    /// [`CLASS_LIVING`], and the recursion visits every child node,
    /// whether it has a body or not.
    pub fn settle(&mut self, node: usize, ticks: i32) {
        if let Some(body) = self.body_of(node)
            && self.bodies[body].class & CLASS_LIVING != 0
        {
            self.bodies[body].state |= state::SETTLE;
            self.bodies[body].countdown = ticks;
        }
        for child in self
            .nodes
            .get(node)
            .map(|n| n.children.clone())
            .unwrap_or_default()
        {
            self.settle(child, ticks);
        }
    }

    /// Freezes a node's body where `at` puts its rest points (or where it
    /// is now): clears the asleep and deep bits, sets frozen and kinematic,
    /// puts the previous particles at the matrix, zeroes the tick's
    /// displacements, copies the matrix and sets the countdown to two.
    pub fn freeze(&mut self, node: usize, at: Option<Affine3A>) {
        let Some(body) = self.body_of(node) else {
            return;
        };
        let b = &mut self.bodies[body];
        b.state = (b.state & !(state::ASLEEP | state::DEEP)) | state::FROZEN | state::KINEMATIC;
        match at {
            Some(matrix) => {
                b.matrix = matrix;
                for i in 0..4 {
                    b.positions[i] = matrix.transform_point3(b.rest[i]);
                    b.previous[i] = b.positions[i];
                }
            }
            None => {
                b.matrix = b.transform();
                b.previous = b.positions;
            }
        }
        b.countdown = 2;
    }

    /// Clears a node body's frozen and kinematic bits.
    pub fn unfreeze(&mut self, node: usize) {
        if let Some(body) = self.body_of(node) {
            self.bodies[body].state &= !(state::FROZEN | state::KINEMATIC);
        }
    }

    /// Disables every node from `node` down (no body needed).
    pub fn disable_tree(&mut self, node: usize) {
        for n in self.nodes_under(node) {
            self.nodes[n].state |= state::DISABLED;
        }
    }

    /// Enables every node from `node` down.
    pub fn enable_tree(&mut self, node: usize) {
        for n in self.nodes_under(node) {
            self.nodes[n].state &= !state::DISABLED;
        }
    }

    /// Disables the bodies from `node` down: a living body (class
    /// [`CLASS_LIVING`]) is also made kinematic, any other only disabled.
    pub fn disable_bodies(&mut self, node: usize) {
        for n in self.nodes_under(node) {
            if let Some(body) = self.nodes[n].body {
                let b = &mut self.bodies[body];
                if b.class & CLASS_LIVING != 0 {
                    b.state |= state::DISABLED | state::KINEMATIC;
                } else {
                    b.state |= state::DISABLED;
                }
            }
        }
    }

    /// Clears the grab layer of a body and of its parent's and its own
    /// children's: the break of a grab.
    pub fn clear_pair_layer(&mut self, body: usize) {
        if let Some(parent) = self.bodies.get(body).and_then(|b| b.parent)
            && let Some(p) = self.bodies.get_mut(parent)
        {
            p.layer &= !LAYER_GRAB;
        }
        if let Some(b) = self.bodies.get_mut(body) {
            b.layer &= !LAYER_GRAB;
        }
        let children = self.bodies.get(body).map(|b| b.children.clone());
        for child in children.unwrap_or_default() {
            if let Some(c) = self.bodies.get_mut(child) {
                c.layer &= !LAYER_GRAB;
            }
        }
    }

    /// Disables the node's body and drops its first breakable grab link:
    /// the grab's parent and children leave the grab layer. The held bit is
    /// not touched; every child node is visited afterwards.
    pub fn drop_grab_link(&mut self, node: usize) {
        if let Some(body) = self.body_of(node) {
            self.bodies[body].state |= state::DISABLED;
            if self.bodies[body].class & CLASS_LIVING != 0
                && let Some(at) = self.bodies[body]
                    .links
                    .iter()
                    .position(|l| l.flags & link::REANCHORED == 0 && l.flags & link::GRAB != 0)
            {
                let parent = self.bodies[body].links[at].parent;
                if let Some(parent) = parent {
                    self.clear_pair_layer(parent);
                }
                self.bodies[body].layer &= !LAYER_GRAB;
                self.bodies[body].links.remove(at);
            }
        }
        for child in self
            .nodes
            .get(node)
            .map(|n| n.children.clone())
            .unwrap_or_default()
        {
            self.drop_grab_link(child);
        }
    }

    /// Clears the held bit of every body from `node` down.
    pub fn clear_held(&mut self, node: usize) {
        if let Some(body) = self.body_of(node) {
            self.bodies[body].state &= !state::HELD;
        }
        for child in self
            .nodes
            .get(node)
            .map(|n| n.children.clone())
            .unwrap_or_default()
        {
            self.clear_held(child);
        }
    }

    /// Holds every body from `node` down and resets its held counter.
    pub fn hold_tree(&mut self, node: usize) {
        if let Some(body) = self.body_of(node) {
            self.bodies[body].state |= state::HELD;
            self.bodies[body].held = 0;
        }
        for child in self
            .nodes
            .get(node)
            .map(|n| n.children.clone())
            .unwrap_or_default()
        {
            self.hold_tree(child);
        }
    }

    /// Releases every body from `node` down.
    pub fn release_tree(&mut self, node: usize) {
        if let Some(body) = self.body_of(node) {
            self.bodies[body].state &= !state::HELD;
        }
        for child in self
            .nodes
            .get(node)
            .map(|n| n.children.clone())
            .unwrap_or_default()
        {
            self.release_tree(child);
        }
    }

    /// Removes every pair that touches a body in the held state (the
    /// original's pair sweep).
    pub fn remove_held_pairs(&mut self) {
        let held = |i: usize| {
            self.bodies
                .get(i)
                .is_some_and(|b| b.state & state::HELD != 0)
        };
        self.pairs.retain(|p| !held(p.a) && !held(p.b));
    }

    /// Whether a placement of `body` overlaps another body: the nine cells
    /// around it, each body in them, skipping its own tree, bodies whose
    /// pair masks cannot meet, and disabled ones; the more important body
    /// (the higher order) is the one that moves, and `overlaps` answers
    /// whether the two bodies' shapes touch. This is the structure of the
    /// original's placement query; its two shape tests are the solver's
    /// ([`crate::body::pairs`] uses the same masks).
    pub fn space_free(
        &self,
        grid: &Grid,
        body: usize,
        mut overlaps: impl FnMut(&Body, &Body) -> bool,
    ) -> bool {
        if self.bodies.get(body).is_none() {
            return true;
        }
        // The query runs over every body of the tree.
        let mut tree = vec![body];
        let mut k = 0;
        while k < tree.len() {
            if let Some(b) = self.bodies.get(tree[k]) {
                tree.extend(b.children.iter().copied());
            }
            k += 1;
        }
        for &tested in &tree {
            let Some(candidate) = self.bodies.get(tested) else {
                continue;
            };
            let Some(cell) = candidate.cell else {
                continue;
            };
            for index in grid.neighbours(cell) {
                for &other in grid.cells().get(index).map_or(&[][..], |c| c) {
                    if other == tested || tree.contains(&other) {
                        continue;
                    }
                    let Some(o) = self.bodies.get(other) else {
                        continue;
                    };
                    if tree.contains(&o.root.unwrap_or(other))
                        || !masks_meet(candidate.layer, o.layer)
                        || o.state & state::DISABLED != 0
                    {
                        continue;
                    }
                    let (first, second) = if o.order < candidate.order {
                        (o, candidate)
                    } else {
                        (candidate, o)
                    };
                    if overlaps(first, second) {
                        return false;
                    }
                }
            }
        }
        true
    }
}

/// Whether two pair masks meet: they share a layer bit other than the
/// lowest, and their shared group-mask bits (0x55555550) shifted up one do
/// not miss both bodies.
pub fn masks_meet(a: u32, b: u32) -> bool {
    a & b & 0xffff_fffe != 0 && ((a & b & 0x5555_5550) << 1) & (a | b) != 0
}

/// Sector flags: what the simulation's update must serve.
pub mod sector {
    /// The sector left its scene: clear it and forget it.
    pub const DEAD: u32 = 0x8_0000;
    /// Purge the nodes and bodies that left the scene.
    pub const SCENE_LEFT: u32 = 0x1_0000;
    /// Remove the world links.
    pub const WORLD_LINKS: u32 = 0x20_0000;
    /// Remove the links to another scene.
    pub const SCENE_LINKS: u32 = 0x40_0000;
    /// Remove a sub-scene.
    pub const SUB_SCENE: u32 = 0x10_0000;
}

/// One sector of a simulation: its flag word, the scene it belongs to, and
/// the nodes, bodies, pairs, simulated objects and other nodes it keeps.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Sector {
    pub flags: u32,
    pub scene: u32,
    pub nodes: Vec<usize>,
    pub bodies: Vec<usize>,
    pub pairs: Vec<usize>,
    pub sim_objects: Vec<usize>,
    pub others: Vec<usize>,
    /// The grid the sector put its bodies in, if it built one.
    pub grid: Option<Grid>,
}

impl Sector {
    /// A sector of `scene`.
    pub fn new(scene: u32) -> Self {
        Self {
            scene,
            ..Self::default()
        }
    }

    /// Places every body in the grid's cells by its node's translation,
    /// clearing the grid first, as the original's grid build does.
    pub fn build_grid(&mut self, world: &World, size: GridSize) {
        let grid = self.grid.get_or_insert_with(|| Grid::new(size));
        if grid.width != size.width || grid.height != size.height {
            *grid = Grid::new(size);
        }
        grid.clear();
        for &body in &self.bodies {
            let at = world
                .bodies
                .get(body)
                .and_then(|b| b.node)
                .and_then(|n| world.nodes.get(n))
                .map(|n| n.position);
            if let Some(cell) = at.and_then(|p| grid.locate(p)) {
                grid.insert(cell, body);
            }
        }
    }

    /// Clears the sector's lists, as the original's sector reset does
    /// (bodies, pairs, nodes and other nodes, and its simulation objects).
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.bodies.clear();
        self.pairs.clear();
        self.sim_objects.clear();
        self.others.clear();
    }

    /// Purges the nodes and bodies that are not in the sector's scene: the
    /// nodes and other nodes whose scene differs, the bodies whose node's
    /// scene differs (unlinking them from the grid), the pairs with such a
    /// body, and the simulated objects of such a node. Swap-removal, except
    /// the other nodes keep their order (the original shifts them down).
    pub fn purge_left(&mut self, world: &mut World) {
        let scene = self.scene;
        let scene_of_body = |world: &World, body: usize| {
            world
                .bodies
                .get(body)
                .map(|b| b.node.map(|n| world.nodes[n].scene).unwrap_or(b.scene))
        };
        self.nodes.retain(|&n| {
            let keep = world.nodes.get(n).is_some_and(|node| node.scene == scene);
            if !keep && let Some(node) = world.nodes.get_mut(n) {
                node.state &= !state::REGISTERED;
            }
            keep
        });
        let mut i = 0;
        while i < self.bodies.len() {
            let body = self.bodies[i];
            if scene_of_body(world, body) == Some(scene) {
                i += 1;
                continue;
            }
            if let (Some(b), Some(grid)) = (world.bodies.get(body), self.grid.as_mut())
                && let Some(cell) = b.cell
            {
                grid.remove(cell, body);
            }
            if let Some(node) = world.bodies.get(body).and_then(|b| b.node)
                && let Some(node) = world.nodes.get_mut(node)
            {
                node.state &= !state::REGISTERED;
            }
            self.bodies.swap_remove(i);
        }
        let in_scene = |world: &World, body: usize| scene_of_body(world, body) == Some(scene);
        self.pairs.retain(|&p| {
            world
                .pairs
                .get(p)
                .is_some_and(|pair| in_scene(world, pair.a) && in_scene(world, pair.b))
        });
        self.sim_objects.retain(|&s| {
            world
                .sim_objects
                .get(s)
                .is_some_and(|o| world.nodes.get(o.node).is_some_and(|n| n.scene == scene))
        });
        self.others
            .retain(|&n| world.nodes.get(n).is_some_and(|node| node.scene == scene));
    }

    /// Purges everything of `scene` itself (a sub-scene removed): nodes of
    /// that scene, the bodies on them, their pairs and simulated objects.
    pub fn purge_scene(&mut self, world: &mut World, scene: u32) {
        for &n in &self.nodes {
            if world.nodes.get(n).is_some_and(|node| node.scene == scene)
                && let Some(node) = world.nodes.get_mut(n)
            {
                node.state &= !state::REGISTERED;
            }
        }
        self.nodes
            .retain(|&n| !world.nodes.get(n).is_some_and(|node| node.scene == scene));
        let mut i = 0;
        while i < self.bodies.len() {
            let body = self.bodies[i];
            let belongs = world
                .bodies
                .get(body)
                .and_then(|b| b.node)
                .and_then(|n| world.nodes.get(n))
                .is_some_and(|node| node.scene == scene);
            if belongs {
                if let (Some(b), Some(grid)) = (world.bodies.get(body), self.grid.as_mut())
                    && let Some(cell) = b.cell
                {
                    grid.remove(cell, body);
                }
                self.bodies.swap_remove(i);
            } else {
                i += 1;
            }
        }
        self.pairs.retain(|&p| {
            world.pairs.get(p).is_some_and(|pair| {
                let of = |b: usize| {
                    world
                        .bodies
                        .get(b)
                        .and_then(|b| b.node)
                        .and_then(|n| world.nodes.get(n))
                        .is_some_and(|node| node.scene == scene)
                };
                !of(pair.a) && !of(pair.b)
            })
        });
        self.sim_objects.retain(|&s| {
            !world
                .sim_objects
                .get(s)
                .is_some_and(|o| world.nodes.get(o.node).is_some_and(|n| n.scene == scene))
        });
        self.others
            .retain(|&n| !world.nodes.get(n).is_some_and(|node| node.scene == scene));
    }

    /// The sector's turn in the simulation's update: serve the flags in the
    /// original's order (a dead sector is cleared and reported, a scene-left
    /// purge, the world-link and scene-link removals, the sub-scene purge),
    /// then build the grid when asked. Returns whether the sector died.
    pub fn update(&mut self, world: &mut World) -> bool {
        if self.flags & sector::DEAD != 0 {
            self.clear();
            return true;
        }
        if self.flags & sector::SCENE_LEFT != 0 {
            self.flags &= !sector::SCENE_LEFT;
            self.purge_left(world);
        }
        if self.flags & sector::WORLD_LINKS != 0 {
            self.flags &= !sector::WORLD_LINKS;
            for &body in &self.bodies {
                if let Some(b) = world.bodies.get_mut(body) {
                    b.links.retain(|l| l.parent.is_some());
                }
            }
        }
        if self.flags & sector::SCENE_LINKS != 0 {
            self.flags &= !sector::SCENE_LINKS;
        }
        if self.flags & sector::SUB_SCENE != 0 {
            self.flags &= !sector::SUB_SCENE;
        }
        false
    }
}

/// A simulation: one scene's clock and its sectors. The original shares
/// one grid across a simulation's sectors.
#[derive(Debug, Clone)]
pub struct Simulation {
    pub clock: Clock,
    pub sectors: Vec<Sector>,
    /// Bodies a listener is watching (the original's watch list).
    pub watch: Vec<usize>,
    /// The listeners' bodies, called with a removed watch entry.
    pub listeners: Vec<usize>,
}

impl Simulation {
    /// A simulation of one scene at the given step.
    pub fn new(step: u32) -> Self {
        Self {
            clock: Clock {
                step,
                ..Clock::default()
            },
            sectors: Vec::new(),
            watch: Vec::new(),
            listeners: Vec::new(),
        }
    }

    /// Adds a body to the watch list: appended last.
    pub fn watch_add(&mut self, body: usize) {
        self.watch.push(body);
    }

    /// Removes a body from the watch list (swap with the last) and reports
    /// whether it was there; the original then calls every listener's
    /// add-to-world with the body (the listeners are returned here).
    pub fn watch_remove(&mut self, body: usize) -> (bool, Vec<usize>) {
        let Some(at) = self.watch.iter().position(|&b| b == body) else {
            return (false, Vec::new());
        };
        self.watch.swap_remove(at);
        (true, self.listeners.clone())
    }

    /// One fixed step's non-physical work: the clock advances and every
    /// sector is updated; dead sectors are dropped.
    pub fn turn(&mut self, world: &mut World) {
        self.clock.advance();
        let mut i = 0;
        while i < self.sectors.len() {
            if self.sectors[i].update(world) {
                self.sectors.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }
}

#[cfg(test)]
#[path = "dynamics_test.rs"]
mod tests;
