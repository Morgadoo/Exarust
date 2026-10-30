//! Electric arcs, as the original's tendril emitter runs them: a set of
//! jittering, branching tendrils (chains of points) that grow from a
//! source, reach for characters and shock them, stop at the level, and are
//! drawn as camera-facing ribbons with a flickering light and hum. Level
//! electricity, the Shock power's hand arcs, shock weapons and sparks all
//! use it. Lengths are in cm; the simulation runs at the body tick.
//!
//! - **Setup** ([`ArcEmitter::new`], [`ArcParams`]): `count` tendril
//!   records of `segments` points each; segment length 4·size (times the
//!   scale when it is above 1.2e-7), ribbon width 2·size, the step
//!   fraction 1/(segments − 1) and the branch floor 1.5/segments. The first
//!   tendril starts at the source with one segment along +x (or along the
//!   record's direction) and lives `life` ticks, or every tendril respawns
//!   with a life of `life` to `2·life` ticks.
//! - **Each tick** ([`ArcEmitter::tick`]): the texture scroll moves 0.015
//!   (wrapping past 2); the segment length grows by the growth and the
//!   width by a tenth of it. Below a lift of 0.999 a step may rise at most
//!   0.3 of a segment over its neighbours and no point goes below the
//!   floor 0. A respawning emitter starts a free record half the ticks.
//!   Every tendril whose timer has run out loses 0.1 of its life a tick;
//!   it keeps `round(segments·life)` points and drops branches past that.
//!   Along each tendril a bend opens a branch where the chord over a point
//!   is 0.6 of a live segment or more (life `floor + r·(f − floor)/2`, f
//!   the share of the tendril left beyond the point), and a fold removes
//!   the point instead. A branch's life wanders by (r − ½)/2 a tick; below
//!   the floor it dies, and above f it trades places with the rest of the
//!   tendril (the main tendril only clips it to f). Then every tendril
//!   follows its origin, grows one point toward its length, and moves each
//!   point: one tick in 30 it jumps to its neighbours' middle (or half a
//!   live segment past the last) plus a jitter, otherwise it keeps 0.9 of
//!   its Verlet motion (none above 1 cm a tick) plus a 0.02-segment jitter
//!   and is held within half a segment of its neighbours' middle (a whole
//!   one behind the last). The main tendril's end sits on the target.
//! - **The level** ([`ArcEmitter::clip_to_level`]): a segment that meets
//!   the level ends the tendril at the hit, losing its branches past the
//!   cut.
//! - **Characters** ([`ArcEmitter::strike_limb`]): each segment but the
//!   last is given to the character's query with a 20 cm value
//!   ([`ArcProbe`]). The first segment that meets a limb of a character
//!   other than the caster shocks that character with the emitter's
//!   strength along the line from the arcs' mean point to its chest (once
//!   per character with flag 0x10), and the tendril sends a branch from the
//!   point before toward the limb.
//! - **Each frame** ([`ArcEmitter::frame`]): an ending emitter with no
//!   points left is removed. The fill is the points over segments·count,
//!   times 2; the light flickers by (r − ½)/2 within 0.75–1.2 and shines
//!   fill·flicker·light (times 1.5 and the node's fade for a voxel light);
//!   the hum's gain is fill²·flicker·volume (flicker within 0.5–1.5) and its
//!   pitch 0.5 + fill·flicker/2 (flicker within 0.8–1.1). The ribbon
//!   ([`ArcRibbon`]) runs along each tendril through Hermite midpoints,
//!   half as wide as a fifth of a segment at its root and narrowing by
//!   life, its first and last pair transparent; its points feed up to 16
//!   light points that merge within 40 cm.
//! - **The world** ([`attach`], [`ArcEmitter::add_to_world`],
//!   [`ArcEmitter::remove_from_world`], [`ArcEmitter::destroy`]): the node
//!   gains its child bit and the emitter as its child; entering the world
//!   starts the mean at the node's box centre, gives the node the
//!   local-frame bit when its parent has it, and starts a named hum looping
//!   at the node; leaving frees the light and stops the hum.
//!   [`follow_parent`] is the node's update under a local-frame parent: the
//!   parent's matrix with the node's own offset carried through it.
//!
//! Evidence: the emitter's constructor, setup, tick, level and character
//! tests, branch and respawn helpers, bounds and frame, read from
//! `Exanima.exe` (`context/re/thaum-arcs.md`).

use bevy::math::Vec3;

use super::engine::{Draws, dist_sq, dot_zyx, jitter, length, lerp, normalize, sign};

/// The smallest scale, lift and life the setup and the fade keep.
pub const EPSILON: f32 = f32::from_bits(0x33ff_ffd7);

/// Flag: the main tendril's end follows the source plus the node's axis
/// times the follow distance.
pub const FOLLOW: u32 = 2;
/// Flag: points are kept above the node's plane.
pub const PLANE: u32 = 4;
/// Flag: a character is shocked once.
pub const SHOCK_ONCE: u32 = 0x10;
/// Light kinds in the record's flags (bits 8–9).
pub const VOXEL_LIGHT: u32 = 0x200;
pub const GI_LIGHT: u32 = 0x300;

/// The light the arcs carry.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ArcLight {
    #[default]
    None,
    /// A point light (level electricity): fill·flicker·light.
    Point,
    /// A voxel light: (1 − fade)·light·fill·flicker·1.5.
    Voxel,
    /// Light points on the GI: (1 − fade)·(the tendrils' lives)·light.
    Points,
}

impl ArcLight {
    /// The kind a record's flags ask for.
    pub fn of_flags(flags: u32) -> Self {
        match flags & 0x300 {
            VOXEL_LIGHT => Self::Voxel,
            GI_LIGHT => Self::Points,
            _ => Self::None,
        }
    }
}

/// Flag: every tendril respawns.
pub const RESPAWNS: u32 = 1;

/// The 84-byte arc record the effects fill in.
#[derive(Debug, Clone, PartialEq)]
pub struct ArcParams {
    /// Respawns (bit 0), the light kind (bits 8–9), the emitter's own flags
    /// (low byte) and the node's (bits 16–19).
    pub flags: u32,
    pub count: i32,
    pub segments: i32,
    /// Ticks the first tendril lives, or the respawn life.
    pub life: i32,
    pub size: f32,
    /// Scales the segment length when above [`EPSILON`].
    pub scale: f32,
    /// Added to the segment length each tick.
    pub growth: f32,
    /// The vertical share of random steps, when above [`EPSILON`].
    pub lift: f32,
    /// The light's strength.
    pub light: f32,
    /// The hum's name (none for silence).
    pub sound: Option<&'static str>,
    /// The hum's volume, when above [`EPSILON`] (0.1 otherwise).
    pub volume: f32,
    /// Added to the source and the first tendril's second point.
    pub offset: Vec3,
    /// The first tendril's direction (a non-respawning emitter).
    pub direction: Vec3,
    /// The shock's strength.
    pub strength: f32,
}

impl Default for ArcParams {
    fn default() -> Self {
        Self {
            flags: 0,
            count: 1,
            segments: 2,
            life: 0,
            size: 1.0,
            scale: 0.0,
            growth: 0.0,
            lift: 0.0,
            light: 0.0,
            sound: None,
            volume: 0.0,
            offset: Vec3::ZERO,
            direction: Vec3::ZERO,
            strength: 0.0,
        }
    }
}

/// Where a tendril starts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Origin {
    /// The emitter's source.
    Source,
    /// A fixed point (a source sphere of the body the arcs come from).
    Fixed(Vec3),
    /// A point of another tendril, which this one branches from.
    Point { tendril: usize, index: usize },
}

/// One tendril record.
#[derive(Debug, Clone, PartialEq)]
pub struct Tendril {
    pub origin: Origin,
    /// Points in use; 0 is a free record.
    pub count: i32,
    pub points: Vec<Vec3>,
    pub previous: Vec<Vec3>,
    /// The branch at each point.
    pub branches: Vec<Option<usize>>,
    pub life: f32,
    /// The ribbon's texture phase.
    pub phase: f32,
    /// Ticks left before the fade (−1 for a branch, which never counts).
    pub timer: i32,
}

impl Tendril {
    fn new(capacity: usize) -> Self {
        Self {
            origin: Origin::Source,
            count: 0,
            points: vec![Vec3::ZERO; capacity],
            previous: vec![Vec3::ZERO; capacity],
            branches: vec![None; capacity],
            life: 0.0,
            phase: 0.0,
            timer: 0,
        }
    }

    fn fit(&mut self, len: usize) {
        if self.points.len() < len {
            self.points.resize(len, Vec3::ZERO);
            self.previous.resize(len, Vec3::ZERO);
            self.branches.resize(len, None);
        }
    }
}

/// One of the arcs' light points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightPoint {
    pub at: Vec3,
    pub weight: f32,
    pub count: i32,
}

/// The arcs' simulation.
#[derive(Debug, Clone, PartialEq)]
pub struct ArcEmitter {
    pub tendrils: Vec<Tendril>,
    pub segments: i32,
    pub step: f32,
    pub branch_floor: f32,
    /// Segment length (cm).
    pub length: f32,
    pub width: f32,
    pub growth: f32,
    pub source: Vec3,
    pub target: Vec3,
    pub main: Option<usize>,
    pub follow: f32,
    pub lift: f32,
    pub respawns: bool,
    pub respawn_life: i32,
    pub flags: u32,
    /// The emitter is in the world.
    pub in_world: bool,
    /// Told to end: removed once its points are gone.
    pub ending: bool,
    /// Points in use after the last tick.
    pub total: i32,
    pub light: f32,
    pub strength: f32,
    pub volume: f32,
    pub sound: Option<&'static str>,
    pub light_kind: ArcLight,
    /// The texture scroll before and after the last tick.
    pub scroll: [f32; 2],
    pub light_flicker: f32,
    pub gain_flicker: f32,
    pub pitch_flicker: f32,
    pub centre: Vec3,
    pub extent: Vec3,
    pub radius: f32,
    pub mean: Vec3,
    /// Where the points' frame sits (the hand the arcs ride with).
    pub anchor: Option<Vec3>,
    /// The plane the points stay above: its normal and a point on it.
    pub plane: Option<(Vec3, Vec3)>,
    /// The spheres of the body the arcs come from, in the emitter's frame.
    pub source_spheres: Vec<Vec3>,
    /// Characters already shocked (with [`SHOCK_ONCE`]).
    pub shocked: Vec<u64>,
    pub lights: Vec<LightPoint>,
}

/// Up to this many light points.
pub const LIGHT_POINTS: usize = 16;

impl ArcEmitter {
    /// An emitter as the constructor and the setup leave it: `count`
    /// records of `segments` points, size `size`.
    pub fn allocate(count: i32, segments: i32, size: f32) -> Self {
        let count = count.max(1);
        let segments = segments.max(2);
        let capacity = segments as usize + 1;
        let length = size * 4.0;
        let mut tendrils = vec![Tendril::new(capacity); count as usize];
        let source = Vec3::ZERO;
        let target = Vec3::new(segments as f32 * length + source.x, source.y, source.z);
        let first = &mut tendrils[0];
        first.count = 2;
        first.origin = Origin::Source;
        first.points[0] = source;
        first.points[1] = Vec3::new(first.points[0].x + length, first.points[0].y, 0.0);
        first.previous[0] = first.points[0];
        first.previous[1] = first.points[1];
        first.life = 1.0;
        first.timer = -1;
        Self {
            tendrils,
            segments,
            step: 1.0 / (segments as f32 - 1.0),
            branch_floor: 1.5 / segments as f32,
            length,
            width: size * 2.0,
            growth: 0.0,
            source,
            target,
            main: None,
            follow: 0.0,
            lift: 1.0,
            respawns: false,
            respawn_life: 10,
            flags: 0,
            in_world: false,
            ending: false,
            total: 0,
            light: 0.0,
            strength: 0.0,
            volume: 0.1,
            sound: None,
            light_kind: ArcLight::None,
            scroll: [0.0, 0.0],
            light_flicker: 0.0,
            gain_flicker: 0.0,
            pitch_flicker: 0.0,
            centre: Vec3::ZERO,
            extent: Vec3::ZERO,
            radius: 0.0,
            mean: Vec3::ZERO,
            anchor: None,
            plane: None,
            source_spheres: Vec::new(),
            shocked: Vec::new(),
            lights: Vec::new(),
        }
    }

    /// An emitter from an arc record.
    pub fn new(params: &ArcParams, draws: &mut impl Draws) -> Self {
        let mut arc = Self::allocate(params.count, params.segments, params.size);
        arc.flags = params.flags & 0xff;
        arc.light_kind = ArcLight::of_flags(params.flags);
        if EPSILON < params.scale {
            arc.length *= params.scale;
        }
        if params.flags & RESPAWNS == 0 {
            arc.tendrils[0].points[1] = params.direction * arc.length;
            arc.tendrils[0].timer = params.life;
            arc.ending = true;
            arc.total = 1;
        } else {
            arc.respawns = true;
            arc.respawn_life = params.life;
            arc.tendrils[0].timer = draws.below(params.life) + arc.respawn_life;
        }
        if EPSILON < params.lift {
            arc.lift = params.lift;
        }
        arc.growth = params.growth;
        arc.light = params.light;
        arc.strength = params.strength;
        arc.sound = params.sound;
        if EPSILON < params.volume {
            arc.volume = params.volume;
        }
        arc.source += params.offset;
        arc.tendrils[0].points[1] += params.offset;
        arc
    }

    /// The first free record.
    pub fn free_record(&self) -> Option<usize> {
        self.tendrils.iter().position(|t| t.count == 0)
    }

    /// Clears a record and, first, its branches'.
    pub fn kill(&mut self, record: usize) {
        self.tendrils[record].life = 0.0;
        let count = self.tendrils[record].count;
        for k in 0..count.max(0) as usize {
            if let Some(branch) = self.tendrils[record].branches.get(k).copied().flatten() {
                self.kill(branch);
                self.tendrils[record].branches[k] = None;
            }
        }
        self.tendrils[record].count = 0;
    }

    fn kill_branch(&mut self, record: usize, index: usize) {
        if let Some(branch) = self.tendrils[record].branches.get(index).copied().flatten() {
            self.kill(branch);
            self.tendrils[record].branches[index] = None;
        }
    }

    fn origin_point(&self, origin: Origin) -> Vec3 {
        match origin {
            Origin::Source => self.source,
            Origin::Fixed(at) => at,
            Origin::Point { tendril, index } => self
                .tendrils
                .get(tendril)
                .and_then(|t| t.points.get(index))
                .copied()
                .unwrap_or(self.source),
        }
    }

    /// Starts a free record with a 10–19 tick life from the source, as the
    /// respawn does. Returns the record it started.
    pub fn start_from_source(&mut self, draws: &mut impl Draws) -> Option<usize> {
        let free = self.free_record()?;
        let timer = draws.below(10) + 10;
        self.start(free, Origin::Source, timer, draws);
        Some(free)
    }

    /// Starts a free record with a 10–19 tick life from the source.
    pub fn respawn(&mut self, draws: &mut impl Draws) {
        self.start_from_source(draws);
    }

    fn start(&mut self, record: usize, origin: Origin, timer: i32, draws: &mut impl Draws) {
        let from = self.origin_point(origin);
        let dir = jitter(draws, self.lift);
        let scale = self.length / length(dir);
        let t = &mut self.tendrils[record];
        t.timer = timer;
        t.origin = origin;
        t.points[0] = from;
        t.points[1] = t.points[0] + dir * scale;
        t.previous[0] = t.points[0];
        t.previous[1] = t.points[1];
        t.life = 1.0;
        t.count = 2;
    }

    /// One tick of the arcs.
    pub fn tick(&mut self, draws: &mut impl Draws) {
        let plane = if self.flags & PLANE != 0 {
            self.plane.map(|(n, p)| (n, dot_zyx(n, p)))
        } else {
            None
        };
        self.scroll[0] = self.scroll[1];
        self.scroll[1] += 0.015;
        if 2.0 < self.scroll[1] {
            self.scroll[0] -= 1.0;
            self.scroll[1] -= 1.0;
        }
        self.length += self.growth;
        self.width += self.growth * 0.1;
        let (limit, floor) = if 0.999 <= self.lift {
            (10000.0, -10000.0)
        } else {
            (self.length * 0.3, 0.0)
        };
        if self.respawns
            && draws.below(2) == 0
            && let Some(free) = self.free_record()
        {
            let timer = draws.below(self.respawn_life) + self.respawn_life;
            let origin = if self.source_spheres.is_empty() {
                Some(Origin::Source)
            } else if self.source_spheres.iter().any(|s| 10.0 < s.y) {
                let n = self.source_spheres.len() as i32;
                loop {
                    let pick = self.source_spheres[draws.below(n).max(0) as usize];
                    if 10.0 < pick.y {
                        break Some(Origin::Fixed(pick));
                    }
                }
            } else {
                // The original draws forever here; no sphere stands high
                // enough, so nothing starts.
                None
            };
            if let Some(origin) = origin {
                self.start(free, origin, timer, draws);
            }
        }
        for t in 0..self.tendrils.len() {
            self.age_and_branch(t, draws);
        }
        for t in 0..self.tendrils.len() {
            self.move_points(t, limit, floor, plane, draws);
        }
        if let Some(main) = self.main {
            let last = self.tendrils[main].count - 1;
            if 2 < last {
                self.tendrils[main].points[last as usize] = self.target;
            }
        }
        self.bounds();
    }

    fn age_and_branch(&mut self, t: usize, draws: &mut impl Draws) {
        let rec = &mut self.tendrils[t];
        if rec.timer == 0 {
            rec.life -= 0.1;
            if rec.life < EPSILON {
                rec.life = 0.0;
            }
        } else if 0 < rec.timer {
            rec.timer -= 1;
        }
        let keep = (self.segments as f32 * rec.life).round_ties_even() as i32;
        if keep < rec.count {
            for k in (keep.max(0)..rec.count).rev() {
                self.kill_branch(t, k as usize);
            }
            self.tendrils[t].count = keep;
        }
        if self.tendrils[t].count < 2 {
            self.tendrils[t].count = 0;
            self.tendrils[t].life = 0.0;
            return;
        }
        let last = (self.tendrils[t].count - 1) as usize;
        self.kill_branch(t, last);
        let mut cur = t;
        let mut i: i64 = 0;
        while i < i64::from(self.tendrils[cur].count) - 2 {
            i += 1;
            let iu = i as usize;
            let share = (i64::from(keep) - i) as f32 / self.segments as f32;
            match self.tendrils[cur].branches[iu] {
                None => {
                    let live = self.length * self.tendrils[cur].life;
                    let chord = dist_sq(
                        self.tendrils[cur].points[iu + 1],
                        self.tendrils[cur].points[iu - 1],
                    );
                    if live * 0.6 * live * 0.6 <= chord {
                        if let Some(b) = self.free_record() {
                            self.open_branch(cur, iu, b, share, live, draws);
                        }
                    } else {
                        let rec = &mut self.tendrils[cur];
                        rec.count = (rec.count - 1).max(0);
                        let end = rec.count - 1;
                        if iu as i32 <= end {
                            for k in iu..=end as usize {
                                rec.points[k] = rec.points[k + 1];
                            }
                        }
                        let at = rec.count as usize;
                        self.kill_branch(cur, at);
                        i -= 1;
                    }
                }
                Some(b) => {
                    let life = self.tendrils[b].life;
                    self.tendrils[b].life = ((draws.unit() - 0.5) * 0.5 + f64::from(life)) as f32;
                    let branch = &self.tendrils[b];
                    if branch.life < self.branch_floor || branch.count < 2 {
                        self.kill(b);
                        self.tendrils[cur].branches[iu] = None;
                    } else if share < branch.life {
                        if self.main == Some(cur) {
                            self.tendrils[b].life = share;
                        } else {
                            self.trade(cur, iu, b);
                            self.tendrils[b].life = share;
                            i = 0;
                            cur = b;
                        }
                    }
                }
            }
        }
    }

    fn open_branch(
        &mut self,
        cur: usize,
        i: usize,
        b: usize,
        share: f32,
        live: f32,
        draws: &mut impl Draws,
    ) {
        self.tendrils[b].timer = -1;
        self.tendrils[cur].branches[i] = Some(b);
        self.tendrils[b].origin = Origin::Point {
            tendril: cur,
            index: i,
        };
        let floor = self.branch_floor;
        let life = (draws.unit() * f64::from(share - floor) * 0.5 + f64::from(floor)) as f32;
        self.tendrils[b].life = if 1.0 < life { 1.0 } else { life };
        let from = self.tendrils[cur].points[i];
        let mut v = from - self.tendrils[cur].points[i - 1];
        v *= live / length(v);
        let r = jitter(draws, self.lift);
        v += r * live;
        let limit = if 0.999 <= self.lift {
            10000.0
        } else {
            self.length * 0.3
        };
        if limit < v.y.abs() {
            v.y = sign(v.y) * limit;
        }
        let phase = draws.unit() as f32;
        let rec = &mut self.tendrils[b];
        rec.points[0] = from;
        rec.points[1] = from + v;
        rec.previous[0] = rec.points[0];
        rec.previous[1] = rec.points[1];
        rec.phase = phase;
        rec.count = 2;
    }

    /// The branch `b` at point `i` of `cur` takes the rest of `cur` past the
    /// point, and `cur` takes the branch's.
    fn trade(&mut self, cur: usize, i: usize, b: usize) {
        let from = i + 1;
        let cur_count = self.tendrils[cur].count as usize;
        let b_count = self.tendrils[b].count as usize;
        let tail = cur_count.saturating_sub(from);
        let head = b_count.saturating_sub(1);
        self.tendrils[cur].fit(from + head + 1);
        self.tendrils[b].fit(1 + tail + 1);
        let cur_points: Vec<Vec3> = self.tendrils[cur].points[from..from + tail].to_vec();
        let cur_previous: Vec<Vec3> = self.tendrils[cur].previous[from..from + tail].to_vec();
        let cur_branches: Vec<Option<usize>> =
            self.tendrils[cur].branches[from..from + tail].to_vec();
        let b_points: Vec<Vec3> = self.tendrils[b].points[1..1 + head].to_vec();
        let b_previous: Vec<Vec3> = self.tendrils[b].previous[1..1 + head].to_vec();
        let b_branches: Vec<Option<usize>> = self.tendrils[b].branches[1..1 + head].to_vec();
        self.tendrils[cur].points[from..from + head].copy_from_slice(&b_points);
        self.tendrils[cur].previous[from..from + head].copy_from_slice(&b_previous);
        self.tendrils[cur].branches[from..from + head].copy_from_slice(&b_branches);
        self.tendrils[b].points[1..1 + tail].copy_from_slice(&cur_points);
        self.tendrils[b].previous[1..1 + tail].copy_from_slice(&cur_previous);
        self.tendrils[b].branches[1..1 + tail].copy_from_slice(&cur_branches);
        self.tendrils[cur].count = (b_count + i) as i32;
        self.tendrils[b].count = cur_count as i32 - i as i32;
        self.tendrils[b].origin = Origin::Point {
            tendril: cur,
            index: i,
        };
        self.tendrils[b].previous[0] = self.tendrils[b].points[0];
        self.tendrils[cur].previous[i] = self.tendrils[cur].points[i];
        for k in i..self.tendrils[cur].count.max(0) as usize {
            if let Some(child) = self.tendrils[cur].branches[k] {
                self.tendrils[child].origin = Origin::Point {
                    tendril: cur,
                    index: k,
                };
            }
        }
        for k in 0..self.tendrils[b].count.max(0) as usize {
            if let Some(child) = self.tendrils[b].branches[k] {
                self.tendrils[child].origin = Origin::Point {
                    tendril: b,
                    index: k,
                };
            }
        }
    }

    fn move_points(
        &mut self,
        t: usize,
        limit: f32,
        floor: f32,
        plane: Option<(Vec3, f32)>,
        draws: &mut impl Draws,
    ) {
        if self.tendrils[t].count < 2 {
            return;
        }
        let live = self.length * self.tendrils[t].life;
        let keep = (self.segments as f32 * self.tendrils[t].life).round_ties_even() as i32;
        let origin = self.origin_point(self.tendrils[t].origin);
        let length_now = self.length;
        let lift = self.lift;
        let rec = &mut self.tendrils[t];
        rec.points[0] = origin;
        if rec.count < keep {
            let count = rec.count as usize;
            rec.fit(count + 1);
            let mut v = rec.points[count - 1] - rec.points[0];
            v *= live / length(v);
            let r = jitter(draws, lift);
            v += r * live;
            if limit < v.y.abs() {
                v.y = sign(v.y) * limit;
            }
            rec.points[count] = rec.points[count - 1] + v;
            rec.previous[count] = rec.points[count];
            rec.branches[count] = None;
            rec.count += 1;
        }
        let last = rec.count as usize - 1;
        for i in 1..=last {
            if rec.points[i].y < floor {
                rec.points[i].y = floor;
                rec.previous[i].y = floor;
            }
            if let Some((n, d)) = plane {
                let here = dot_zyx(rec.points[i], n);
                if here < d {
                    rec.points[i] += n * (d - here);
                }
            }
            let mut saved = rec.points[i];
            let r = jitter(draws, lift);
            let reference;
            if draws.below(30) == 0 {
                if i == last {
                    let mut v = rec.points[i] - rec.points[i - 1];
                    v *= live / length(v);
                    v += rec.points[i - 1];
                    reference = rec.points[i - 1].y;
                    rec.points[i] = v + r * (live * 0.5);
                } else {
                    let span = rec.points[i + 1] - rec.points[i - 1];
                    let reach = length(span);
                    let mid = rec.points[i - 1] + span * 0.5;
                    reference = mid.y;
                    rec.points[i] = mid + r * (reach * 0.25);
                }
                saved = rec.points[i];
            } else {
                reference = if i == last {
                    rec.points[i - 1].y
                } else {
                    let below = rec.points[i - 1].y;
                    (rec.points[i + 1].y - below) * 0.5 + below
                };
                let mut v = rec.points[i] - rec.previous[i];
                if 1.0 < (v.z * v.z + v.y * v.y) + v.x * v.x {
                    v = Vec3::ZERO;
                }
                v += r * (length_now * 0.02);
                v *= 0.9;
                rec.points[i] += v;
                if i == last {
                    let d2 = dist_sq(rec.points[i], rec.points[i - 1]);
                    if length_now * length_now < d2 {
                        let off = rec.points[i] - rec.points[i - 1];
                        rec.points[i] = rec.points[i - 1] + off * (length_now / d2.sqrt());
                    }
                } else {
                    let mid = lerp(rec.points[i - 1], rec.points[i + 1], 0.5);
                    let d2 = dist_sq(rec.points[i], mid);
                    let half = length_now * 0.5;
                    if half * half < d2 {
                        let off = rec.points[i] - mid;
                        rec.points[i] = mid + off * ((length_now * 0.5) / length(off));
                    }
                }
            }
            let rise = rec.points[i].y - reference;
            if limit < rise.abs() {
                rec.points[i].y = sign(rise) * limit + reference;
            }
            rec.previous[i] = saved;
        }
    }

    /// The bounds, the point count and the mean point, after a tick.
    pub fn bounds(&mut self) {
        let mut lo = self.source;
        let mut hi = self.source;
        let mut total = 0;
        let mut sum = Vec3::ZERO;
        for t in &self.tendrils {
            total += t.count;
            for p in t.points.iter().take(t.count.max(0) as usize) {
                for axis in 0..3 {
                    if lo[axis] <= p[axis] {
                        if hi[axis] < p[axis] {
                            hi[axis] = p[axis];
                        }
                    } else {
                        lo[axis] = p[axis];
                    }
                }
                sum += *p;
            }
        }
        if let Some(anchor) = self.anchor {
            lo += anchor;
            hi += anchor;
        }
        self.total = total;
        self.centre = lerp(lo, hi, 0.5);
        self.extent = hi - self.centre + Vec3::splat(5.0);
        let e = self.extent;
        let r2 = (e.z * e.z + e.y * e.y) + e.x * e.x;
        self.radius = if 0.0 < r2 { r2.sqrt() } else { r2 };
        if total < 1 {
            self.mean = self.centre;
        } else {
            self.mean = sum * (1.0 / total as f32);
            if let Some(anchor) = self.anchor {
                self.mean += anchor;
            }
        }
    }

    /// The main tendril follows the source plus `axis` times the follow
    /// distance ([`FOLLOW`]).
    pub fn follow_axis(&mut self, axis: Vec3) {
        if self.flags & FOLLOW != 0 {
            let first = self.tendrils[0].points[0];
            self.target = first + axis * self.follow;
        }
    }

    fn world(&self, p: Vec3) -> Vec3 {
        p + self.anchor.unwrap_or(Vec3::ZERO)
    }

    /// The level's test: `ray(start, unit direction, length)` returns how
    /// far along the level was met (the original's query writes the hit's
    /// distance back into the ray). A tendril ends at the hit on the first
    /// segment that meets the level, losing the branches from that segment's
    /// end point on.
    pub fn clip_to_level(&mut self, mut ray: impl FnMut(Vec3, Vec3, f32) -> Option<f32>) {
        for t in 0..self.tendrils.len() {
            if self.tendrils[t].count < 2 {
                continue;
            }
            let mut start = self.world(self.tendrils[t].points[0]);
            let mut i = 0usize;
            while (i as i32) < self.tendrils[t].count - 1 {
                let end = self.world(self.tendrils[t].points[i + 1]);
                let d = end - start;
                let len = length(d);
                let dir = d * (1.0 / len);
                if let Some(hit) = ray(start, dir, len) {
                    let last = self.tendrils[t].count - 1;
                    for k in i + 1..=last.max(0) as usize {
                        self.kill_branch(t, k);
                    }
                    let at = start + dir * hit;
                    self.tendrils[t].points[i + 1] = at - self.anchor.unwrap_or(Vec3::ZERO);
                    self.tendrils[t].count = i as i32 + 2;
                    break;
                }
                start = end;
                i += 1;
            }
        }
    }

    /// One limb of a character against the arcs: `meets` runs the
    /// character's own query on each segment ([`ArcProbe`]), from the first
    /// point's segment up to two short of the end. Returns the shock dealt,
    /// when the first segment that meets the limb is the character's first
    /// this tick (and, with [`SHOCK_ONCE`], ever); the tendril then sends a
    /// branch from the point before toward the limb either way.
    pub fn strike_limb(
        &mut self,
        limb: &ArcLimb,
        mut meets: impl FnMut(&ArcProbe) -> bool,
        draws: &mut impl Draws,
    ) -> Option<ArcShock> {
        if limb.caster {
            return None;
        }
        let fresh = !(self.flags & SHOCK_ONCE != 0 && self.shocked.contains(&limb.character));
        for t in 0..self.tendrils.len() {
            if self.tendrils[t].count < 2 {
                continue;
            }
            let upto = self.tendrils[t].count - 2;
            if upto < 1 {
                continue;
            }
            let mut start = self.world(self.tendrils[t].points[0]);
            let mut k = 0i32;
            while k < upto {
                k += 1;
                let end = self.world(self.tendrils[t].points[k as usize]);
                let d = end - start;
                let len = length(d);
                let probe = ArcProbe {
                    start,
                    end,
                    direction: d * (1.0 / len),
                    length: len,
                    reach: PROBE_REACH,
                };
                if meets(&probe) {
                    let mut shock = None;
                    if fresh {
                        if self.flags & SHOCK_ONCE != 0 {
                            self.shocked.push(limb.character);
                        }
                        shock = Some(ArcShock {
                            character: limb.character,
                            direction: normalize(limb.chest - self.mean),
                            strength: self.strength,
                        });
                    }
                    self.reach_for(t, k - 1, limb.at, draws);
                    return shock;
                }
                start = end;
            }
        }
        None
    }

    /// A new branch from point `index` (1 at least) of tendril `t` toward
    /// `at`, one live segment long.
    pub fn reach_for(&mut self, t: usize, index: i32, at: Vec3, draws: &mut impl Draws) {
        let index = index.max(1) as usize;
        self.kill_branch(t, index);
        let Some(b) = self.free_record() else {
            return;
        };
        self.tendrils[b].timer = -1;
        self.tendrils[t].branches[index] = Some(b);
        self.tendrils[b].origin = Origin::Point { tendril: t, index };
        self.tendrils[b].life = 1.0;
        let from = self.tendrils[t].points[index];
        let mut v = at - from;
        if let Some(anchor) = self.anchor {
            v -= anchor;
        }
        v *= (self.length * self.tendrils[t].life) / length(v);
        let phase = draws.unit() as f32;
        let rec = &mut self.tendrils[b];
        rec.points[0] = from;
        rec.points[1] = from + v;
        rec.previous[0] = rec.points[0];
        rec.previous[1] = rec.points[1];
        rec.phase = phase;
        rec.count = 2;
    }

    /// One frame: the light, the hum, the ribbon and the light points.
    /// `alpha` is how far the next tick is, `eye` the camera's axis in the
    /// emitter's frame, `fade` the node's fade, `to_world` the emitter's
    /// frame placed in the world.
    pub fn frame(
        &mut self,
        alpha: f32,
        eye: Vec3,
        fade: f32,
        to_world: impl Fn(Vec3) -> Vec3,
        draws: &mut impl Draws,
    ) -> ArcFrame {
        if self.ending && self.total == 0 {
            return ArcFrame {
                removed: true,
                ..ArcFrame::default()
            };
        }
        let fill = (self.total as f32
            / (i64::from(self.segments) * self.tendrils.len() as i64) as f32)
            * 2.0;
        for light in &mut self.lights {
            light.weight = 0.0;
            light.count = 1;
        }
        let mut out = ArcFrame {
            fill,
            ..ArcFrame::default()
        };
        if matches!(self.light_kind, ArcLight::Voxel | ArcLight::Point) {
            let f = self.light_flicker;
            self.light_flicker = ((draws.unit() - 0.5) * 0.5 + f64::from(f)) as f32;
            self.light_flicker = self.light_flicker.clamp(0.75, 1.2);
            out.light = if self.light_kind == ArcLight::Voxel {
                (1.0 - fade) * self.light * fill * self.light_flicker * 1.5
            } else {
                self.light * fill * self.light_flicker
            };
        }
        if self.sound.is_some() {
            let g = self.gain_flicker;
            self.gain_flicker = ((draws.unit() - 0.5) * 0.2 + f64::from(g)) as f32;
            self.gain_flicker = self.gain_flicker.clamp(0.5, 1.5);
            out.gain = fill * fill * self.gain_flicker * self.volume;
            let p = self.pitch_flicker;
            self.pitch_flicker = ((draws.unit() - 0.5) * 0.05 + f64::from(p)) as f32;
            self.pitch_flicker = self.pitch_flicker.clamp(0.8, 1.1);
            out.pitch = (EPSILON < fill).then_some(fill * self.pitch_flicker * 0.5 + 0.5);
        }
        out.scroll = self.scroll[1] * alpha + (1.0 - alpha) * self.scroll[0];
        let (ribbon, life_sum) = self.ribbon(eye, &to_world);
        out.ribbon = ribbon;
        if self.light_kind == ArcLight::Points {
            let mut i = 0;
            while i < self.lights.len() {
                if 1.0 <= self.lights[i].weight {
                    i += 1;
                } else {
                    self.lights.swap_remove(i);
                }
            }
            out.light = (1.0 - fade) * life_sum * self.light;
        }
        out
    }

    fn light_point(&mut self, at: Vec3, weight: f32) {
        let weight = (2.0 - weight) * weight;
        if self.lights.is_empty() {
            self.lights.push(LightPoint {
                at,
                weight,
                count: 1,
            });
            return;
        }
        let mut nearest = None;
        let mut best = 6400.0;
        for (k, light) in self.lights.iter().enumerate() {
            let d = dist_sq(light.at, at);
            if d < best {
                nearest = Some(k);
                best = d;
            }
        }
        if LIGHT_POINTS - 1 < self.lights.len() || best <= 1600.0 {
            if let Some(k) = nearest {
                let light = &mut self.lights[k];
                light.count += 1;
                light.at = lerp(light.at, at, 1.0 / light.count as f32);
                light.weight += weight;
            }
        } else {
            self.lights.push(LightPoint {
                at,
                weight,
                count: 1,
            });
        }
    }

    fn ribbon(&mut self, eye: Vec3, to_world: &impl Fn(Vec3) -> Vec3) -> (ArcRibbon, f32) {
        let capacity = ((self.segments * 4 + 2) * self.tendrils.len() as i32 * 3).max(0) as usize;
        let mut r = ArcRibbon {
            indices: vec![0; capacity],
            ..ArcRibbon::default()
        };
        let half = self.length * 0.2;
        let offset = self.anchor.unwrap_or(Vec3::ZERO);
        let mut quads: i32 = 0;
        let mut life_sum = 0.0;
        for t in 0..self.tendrils.len() {
            if self.tendrils[t].count < 2 {
                continue;
            }
            life_sum += self.tendrils[t].life;
            let mut life = self.tendrils[t].life;
            if life < self.branch_floor {
                continue;
            }
            let step = self.step;
            let first_quad = quads;
            let first_vertex = r.positions.len() as u16;
            let origin = self.origin_point(self.tendrils[t].origin);
            self.tendrils[t].points[0] = origin;
            let root = self.tendrils[t].points[0] + offset;
            self.light_point(to_world(root), 1.0);
            let mut b = Builder {
                previous: root,
                current: root,
                u: self.tendrils[t].phase,
                half: 0.0,
            };
            let count = self.tendrils[t].count as usize;
            let points = self.tendrils[t].points[..count].to_vec();
            let width = self.width;
            let rounded = move |life: f32| (1.0 - (1.0 - life) * (1.0 - life)) * width;
            let mut narrow = rounded(life);
            quads += 2;
            b.half = narrow;
            b.emit(hermite(&points, 1, 0.2) + offset, eye, &mut r, &mut quads);
            r.clear_last_pair();
            quads -= 2;
            b.u = (half / (narrow + self.width)) * 0.4 + self.tendrils[t].phase;
            for i in 1..count {
                life = (life - step * 0.5).max(0.0);
                narrow = rounded(life);
                b.half = narrow;
                b.emit(hermite(&points, i, 0.5) + offset, eye, &mut r, &mut quads);
                b.u += half / (narrow + self.width);
                life = (life - step * 0.5).max(0.0);
                narrow = rounded(life);
                b.half = narrow;
                let at = points[i] + offset;
                b.emit(at, eye, &mut r, &mut quads);
                self.light_point(to_world(at), 1.0);
                b.u += half / (narrow + self.width);
            }
            b.emit(points[count - 1] + offset, eye, &mut r, &mut quads);
            r.clear_last_pair();
            quads -= 2;
            let s = first_vertex;
            let slot = first_quad as usize * 3;
            for (k, v) in [s, s + 1, s + 3, s, s + 3, s + 2].into_iter().enumerate() {
                r.write(slot + k, v);
            }
        }
        quads -= 1;
        if quads < 1 {
            quads = 0;
        }
        r.index_count = quads as usize * 3;
        (r, life_sum)
    }
}

/// The value the character query receives with each segment (20 cm),
/// between the segment's start and its end; its role in that query (a
/// radius or a reach) is not traced.
pub const PROBE_REACH: f32 = 20.0;

/// One segment as the character query receives it: its start and end, its
/// unit direction and length, and [`PROBE_REACH`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArcProbe {
    pub start: Vec3,
    pub end: Vec3,
    pub direction: Vec3,
    pub length: f32,
    pub reach: f32,
}

/// The node an emitter rides: its flag word, whether its parent keeps a
/// local frame, its render word and whether the emitter is its child.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ArcNode {
    pub flags: u32,
    pub parent_local_frame: bool,
    pub render: u32,
    pub child: bool,
}

/// The node flag saying it has a child.
pub const HAS_CHILD: u32 = 0x1_0000;
/// The render bit of a node that keeps its own local frame.
pub const LOCAL_FRAME: u32 = 0x10;

/// Attaches an emitter to its node: the node's flag word gains the child
/// bit and its child is the emitter.
pub fn attach(node: &mut ArcNode) {
    node.flags |= HAS_CHILD;
    node.child = true;
}

/// A node's 4×4 matrix as the scene stores it: three axis rows and the
/// translation row, four floats each.
pub type NodeMatrix = [[f32; 4]; 4];

/// The node's update under its parent: when the parent keeps a local frame
/// the node takes the parent's matrix, its translation row replaced by the
/// node's own offset `local` carried through it, `(x·m0 + y·m1) + (z·m2 +
/// m3)` (the row's fourth float kept). `None` otherwise.
pub fn follow_parent(
    parent_local_frame: bool,
    parent: &NodeMatrix,
    local: Vec3,
) -> Option<NodeMatrix> {
    if !parent_local_frame {
        return None;
    }
    let mut out = *parent;
    for c in 0..3 {
        out[3][c] = (local.x * parent[0][c] + local.y * parent[1][c])
            + (local.z * parent[2][c] + parent[3][c]);
    }
    Some(out)
}

/// The hum an emitter starts when it enters the world: a looping voice at
/// the node's translation (started with the engine's arguments 0 and 1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HumStart {
    pub at: Vec3,
}

/// What adding an emitter to the world reports.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArcStart {
    /// The node's render word after the add.
    pub render: u32,
    /// The mean point the shock directions are taken from.
    pub mean: Vec3,
    /// The hum to start, when the emitter names a sound.
    pub hum: Option<HumStart>,
}

/// What removing an emitter frees.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ArcRemoval {
    /// A hum was stopped.
    pub sound: bool,
    /// Light points dropped with the light.
    pub lights: usize,
}

impl ArcEmitter {
    /// Enters the world on `node`: the emitter is marked in the world, its
    /// mean starts at the node's box centre, the node gains the local-frame
    /// render bit when its parent has one, and a named hum starts at the
    /// node's translation.
    pub fn add_to_world(
        &mut self,
        node: &mut ArcNode,
        centre: Vec3,
        translation: Vec3,
    ) -> ArcStart {
        self.in_world = true;
        self.mean = centre;
        if node.parent_local_frame {
            node.render |= LOCAL_FRAME;
        }
        let hum = self.sound.map(|_| HumStart { at: translation });
        ArcStart {
            render: node.render,
            mean: self.mean,
            hum,
        }
    }

    /// Leaves the world: the light (and its points) is freed and the hum
    /// stops.
    pub fn remove_from_world(&mut self) -> ArcRemoval {
        let lights = self.lights.len();
        self.lights.clear();
        self.in_world = false;
        ArcRemoval {
            sound: self.sound.is_some(),
            lights,
        }
    }

    /// Frees the emitter: it leaves the world, its sound is released and its
    /// shocked list and records are emptied. The strings the original
    /// releases are Exarust's own storage.
    pub fn destroy(&mut self) -> ArcRemoval {
        let removal = self.remove_from_world();
        self.tendrils.clear();
        self.shocked.clear();
        self.total = 0;
        removal
    }
}

/// A Hermite point between `points[i − 1]` and `points[i]` at `t`, its
/// tangents from the neighbours (doubled at the ends).
pub fn hermite(points: &[Vec3], i: usize, t: f32) -> Vec3 {
    let cube = t * t * t;
    let a = points[i - 1];
    let b = points[i];
    let mut out = a * ((cube * 2.0 - t * t * 3.0) + 1.0);
    out += b * (t * t * 3.0 + cube * -2.0);
    if i < 2 {
        out += (b - a) * (((cube - t * t * 2.0) + t) * 2.0);
    } else {
        out += (b - points[i - 2]) * ((cube - t * t * 2.0) + t);
    }
    if (i as i64) < points.len() as i64 - 1 {
        out += (points[i + 1] - a) * (cube - t * t);
    } else {
        out += (b - a) * ((cube - t * t) * 2.0);
    }
    out
}

struct Builder {
    previous: Vec3,
    current: Vec3,
    u: f32,
    half: f32,
}

impl Builder {
    /// Places the pair of the current point, across the line from the
    /// previous point to `next` and the eye, and indexes its quad.
    fn emit(&mut self, next: Vec3, eye: Vec3, r: &mut ArcRibbon, quads: &mut i32) {
        let d = next - self.previous;
        let mut side = d.cross(eye);
        let len = length(side);
        side *= self.half / len;
        r.positions.push(self.current - side);
        r.uvs.push([self.u, 0.0]);
        r.alphas.push(0xff);
        r.positions.push(self.current + side);
        r.uvs.push([self.u, 1.0]);
        r.alphas.push(0xff);
        self.previous = self.current;
        self.current = next;
        let v = r.positions.len() as i32;
        let e = *quads;
        r.write_signed(e * 3 - 4, v - 1);
        r.write_signed(e * 3 - 2, v - 1);
        r.write_signed(e * 3 - 1, v - 2);
        *quads += 2;
        let e = *quads;
        r.write_signed(e * 3 - 6, v - 2);
        r.write_signed(e * 3 - 5, v - 1);
        r.write_signed(e * 3 - 3, v - 2);
    }
}

/// The arcs' ribbon: positions, texture coordinates, per-vertex alpha
/// (the colour is 25, 0, 255) and triangles.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ArcRibbon {
    pub positions: Vec<Vec3>,
    pub uvs: Vec<[f32; 2]>,
    pub alphas: Vec<u8>,
    pub indices: Vec<u16>,
    /// How many of `indices` are drawn.
    pub index_count: usize,
}

/// The ribbon's colour before its alpha.
pub const ARC_COLOUR: [u8; 3] = [0x19, 0x00, 0xff];

impl ArcRibbon {
    fn write(&mut self, slot: usize, value: u16) {
        if self.indices.len() <= slot {
            self.indices.resize(slot + 1, 0);
        }
        self.indices[slot] = value;
    }

    fn write_signed(&mut self, slot: i32, value: i32) {
        if let Ok(slot) = usize::try_from(slot) {
            self.write(slot, value as u16);
        }
    }

    fn clear_last_pair(&mut self) {
        let n = self.alphas.len();
        for a in self.alphas.iter_mut().skip(n.saturating_sub(2)) {
            *a = 0;
        }
    }

    /// The triangles drawn.
    pub fn drawn(&self) -> &[u16] {
        &self.indices[..self.index_count.min(self.indices.len())]
    }
}

/// What a frame of the arcs shows.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ArcFrame {
    /// The arcs ended and leave the scene.
    pub removed: bool,
    pub fill: f32,
    pub light: f32,
    pub gain: f32,
    pub pitch: Option<f32>,
    pub scroll: f32,
    pub ribbon: ArcRibbon,
}

/// A limb the arcs are tested against.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArcLimb {
    /// The character it belongs to.
    pub character: u64,
    /// The limb belongs to whoever casts the arcs.
    pub caster: bool,
    /// The limb's place.
    pub at: Vec3,
    /// The character's chest.
    pub chest: Vec3,
}

/// A shock the arcs deal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArcShock {
    pub character: u64,
    pub direction: Vec3,
    pub strength: f32,
}

/// The Shock power's hand arcs: 20 respawning tendrils of 5 points, size 2
/// at a quarter scale, growth 0.25, light 48, with a voxel light; the node
/// sits at (−15, −5, 0) from the hand.
pub fn hand_arcs() -> ArcParams {
    ArcParams {
        flags: RESPAWNS | VOXEL_LIGHT,
        count: 20,
        segments: 5,
        life: 10,
        size: 2.0,
        scale: 0.25,
        growth: 0.25,
        light: 48.0,
        ..ArcParams::default()
    }
}

/// Where the hand arcs' node sits from the hand.
pub const HAND_ARCS_OFFSET: Vec3 = Vec3::new(-15.0, -5.0, 0.0);

/// A shock weapon's arcs: 40 respawning tendrils of 4 points from the
/// blade's spheres, size 2 at 0.75 scale, light 0.02, the `electricity03`
/// hum at 0.15, with a light point.
pub fn weapon_arcs() -> ArcParams {
    ArcParams {
        flags: RESPAWNS | GI_LIGHT,
        count: 40,
        segments: 4,
        life: 20,
        size: 2.0,
        scale: 0.75,
        light: 0.02,
        sound: Some("electricity03"),
        volume: 0.15,
        ..ArcParams::default()
    }
}

/// A spark burst: 20 records of 5 points at size 3, six started at once
/// (10–19 ticks each) and the segment doubled, light 8, volume 0.05, an
/// ending emitter with a voxel light.
pub fn spark_burst(draws: &mut impl Draws) -> ArcEmitter {
    let mut arc = ArcEmitter::allocate(20, 5, 3.0);
    arc.light = 8.0;
    arc.volume = 0.05;
    arc.light_kind = ArcLight::Voxel;
    arc.tendrils[0].timer = draws.below(10) + 10;
    for _ in 0..6 {
        arc.respawn(draws);
    }
    arc.length *= 2.0;
    arc.ending = true;
    arc.total = 1;
    arc
}

/// The shock's own burst when its charge passes one: ten tendrils of
/// fourteen points, forty ticks of life, size 10, scale 0.25, growth 0.5,
/// a level lift, volume 0.05, its first tendril aimed along the caster's
/// aim and a point light; the node sits at the hand's −15, −5 cm. The
/// flags carry the emitter's 0x10 and 0x30000 bits as the original sets
/// them.
pub fn shock_burst() -> ArcParams {
    ArcParams {
        flags: 0x0003_0310,
        count: 10,
        segments: 14,
        life: 40,
        size: 10.0,
        scale: 0.25,
        growth: 0.5,
        lift: 1.0,
        volume: 0.05,
        ..ArcParams::default()
    }
}

/// The shock a shock weapon's blade deals.
pub const WEAPON_SHOCK: f32 = 0.01;

#[cfg(test)]
mod tests {
    use super::super::engine::testing::Fixed;
    use super::*;

    #[test]
    fn the_setup_sizes_the_records() {
        let arc = ArcEmitter::allocate(3, 5, 2.0);
        assert_eq!(arc.tendrils.len(), 3);
        assert_eq!(arc.length, 8.0);
        assert_eq!(arc.width, 4.0);
        assert_eq!(arc.step, 0.25);
        assert_eq!(arc.branch_floor, 0.3);
        assert_eq!(arc.target, Vec3::new(40.0, 0.0, 0.0));
        let first = &arc.tendrils[0];
        assert_eq!(first.count, 2);
        assert_eq!(first.points[1], Vec3::new(8.0, 0.0, 0.0));
        assert_eq!((first.life, first.timer), (1.0, -1));
        assert_eq!(arc.lift, 1.0);
        assert_eq!(arc.respawn_life, 10);
        assert_eq!(arc.volume, 0.1);
    }

    #[test]
    fn a_record_starts_one_segment_from_the_source() {
        let mut draws = Fixed::new(&[1.0, 0.5, 0.5, 0.25], &[3]);
        let mut arc = ArcEmitter::allocate(2, 5, 2.0);
        arc.source = Vec3::new(1.0, 2.0, 3.0);
        arc.respawn(&mut draws);
        let t = &arc.tendrils[1];
        assert_eq!(t.timer, 13);
        assert_eq!(t.count, 2);
        assert_eq!(t.points[0], Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(t.points[1], Vec3::new(9.0, 2.0, 3.0));
        assert_eq!(t.previous[1], t.points[1]);
        assert_eq!(t.life, 1.0);
    }

    #[test]
    fn the_hand_arcs_respawn_from_their_record() {
        let mut draws = Fixed::new(&[0.5], &[4]);
        let arc = ArcEmitter::new(&hand_arcs(), &mut draws);
        assert!(arc.respawns);
        assert_eq!(arc.length, 2.0);
        assert_eq!(arc.width, 4.0);
        assert_eq!(arc.tendrils[0].timer, 14);
        assert_eq!(arc.light_kind, ArcLight::Voxel);
        assert_eq!(arc.light, 48.0);
        let weapon = ArcEmitter::new(&weapon_arcs(), &mut draws);
        assert_eq!(weapon.length, 6.0);
        assert_eq!(weapon.volume, 0.15);
        assert_eq!(weapon.segments, 4);
        assert_eq!(weapon.tendrils.len(), 40);
    }

    #[test]
    fn a_single_arc_lives_its_ticks_then_fades() {
        let mut draws = Fixed::new(&[0.5], &[1]);
        let params = ArcParams {
            count: 1,
            segments: 4,
            life: 2,
            size: 1.0,
            direction: Vec3::X,
            ..ArcParams::default()
        };
        let mut arc = ArcEmitter::new(&params, &mut draws);
        assert!(arc.ending);
        assert_eq!(arc.tendrils[0].timer, 2);
        arc.tick(&mut draws);
        arc.tick(&mut draws);
        assert_eq!(arc.tendrils[0].timer, 0);
        assert_eq!(arc.tendrils[0].life, 1.0);
        arc.tick(&mut draws);
        assert!((arc.tendrils[0].life - 0.9).abs() < 1e-6);
        for _ in 0..20 {
            arc.tick(&mut draws);
        }
        assert_eq!(arc.tendrils[0].count, 0);
        assert_eq!(arc.total, 0);
        let frame = arc.frame(0.0, Vec3::Z, 0.0, |p| p, &mut draws);
        assert!(frame.removed);
    }

    #[test]
    fn a_tendril_grows_toward_its_length() {
        let mut draws = Fixed::new(&[0.5], &[1]);
        let mut arc = ArcEmitter::allocate(1, 4, 1.0);
        arc.tendrils[0].timer = 100;
        arc.tick(&mut draws);
        assert_eq!(arc.tendrils[0].count, 3);
        arc.tick(&mut draws);
        assert_eq!(arc.tendrils[0].count, 4);
        arc.tick(&mut draws);
        assert_eq!(arc.tendrils[0].count, 4);
        // A still jitter: each new point a segment on along the line.
        assert!((arc.tendrils[0].points[1].x - 4.0).abs() < 1e-4);
        assert_eq!(arc.total, 4);
    }

    #[test]
    fn the_scroll_wraps_past_two() {
        let mut draws = Fixed::new(&[0.5], &[1]);
        let mut arc = ArcEmitter::allocate(1, 4, 1.0);
        arc.scroll = [1.985, 1.995];
        arc.tick(&mut draws);
        assert!((arc.scroll[0] - 0.995).abs() < 1e-6);
        assert!((arc.scroll[1] - 1.01).abs() < 1e-6);
    }

    #[test]
    fn a_low_lift_keeps_points_off_the_floor() {
        let mut draws = Fixed::new(&[0.5], &[1]);
        let mut arc = ArcEmitter::allocate(1, 4, 1.0);
        arc.lift = 0.5;
        arc.tendrils[0].timer = 100;
        arc.tendrils[0].points[1] = Vec3::new(4.0, -3.0, 0.0);
        arc.tendrils[0].previous[1] = Vec3::new(4.0, -3.0, 0.0);
        arc.tick(&mut draws);
        // Floored before it moves: its previous place stays on the floor.
        assert_eq!(arc.tendrils[0].previous[1].y, 0.0);
        assert_eq!(arc.tendrils[0].previous[2].y, 0.0);
        // A step rises at most 0.3 of a segment (1.2) over its neighbours.
        let p = &arc.tendrils[0].points;
        let reference = (p[2].y - p[0].y) * 0.5 + p[0].y;
        assert!((p[1].y - reference).abs() <= 1.2 + 1e-5);
    }

    #[test]
    fn the_level_ends_a_tendril() {
        let mut arc = ArcEmitter::allocate(1, 5, 1.0);
        let t = &mut arc.tendrils[0];
        t.count = 4;
        for k in 0..4 {
            t.points[k] = Vec3::new(k as f32 * 4.0, 0.0, 0.0);
        }
        arc.clip_to_level(|start, dir, len| {
            let end = start + dir * len;
            (start.x < 6.0 && 6.0 <= end.x).then_some(6.0 - start.x)
        });
        // The tendril ends at the hit on the segment that met the level.
        assert_eq!(arc.tendrils[0].count, 3);
        assert_eq!(arc.tendrils[0].points[2], Vec3::new(6.0, 0.0, 0.0));
    }

    #[test]
    fn a_struck_character_is_shocked_once() {
        let mut draws = Fixed::new(&[0.5], &[1]);
        let mut arc = ArcEmitter::allocate(3, 5, 1.0);
        arc.flags = SHOCK_ONCE;
        arc.strength = 0.25;
        let t = &mut arc.tendrils[0];
        t.count = 4;
        for k in 0..4 {
            t.points[k] = Vec3::new(k as f32 * 4.0, 0.0, 0.0);
        }
        arc.bounds();
        let limb = ArcLimb {
            character: 7,
            caster: false,
            at: Vec3::new(4.0, 10.0, 0.0),
            chest: Vec3::new(6.0, 100.0, 0.0),
        };
        let shock = arc
            .strike_limb(
                &limb,
                |probe| {
                    assert_eq!(probe.reach, PROBE_REACH);
                    assert_eq!(probe.length, 4.0);
                    assert_eq!(probe.direction, Vec3::X);
                    probe.end.x >= 4.0
                },
                &mut draws,
            )
            .expect("the first segment meets the limb");
        assert_eq!(shock.character, 7);
        assert_eq!(shock.strength, 0.25);
        assert!((shock.direction - Vec3::Y).length() < 1e-3);
        // The tendril reaches for the limb from its first point.
        let b = arc.tendrils[0].branches[1].expect("a branch at point 1");
        assert_eq!(arc.tendrils[b].count, 2);
        assert_eq!(arc.tendrils[b].points[1], Vec3::new(4.0, 4.0, 0.0));
        assert!(
            arc.strike_limb(&limb, |_| true, &mut draws).is_none(),
            "once only"
        );
        let caster = ArcLimb {
            caster: true,
            character: 8,
            ..limb
        };
        assert!(arc.strike_limb(&caster, |_| true, &mut draws).is_none());
    }

    #[test]
    fn a_shock_points_from_the_world_mean_to_the_chest() {
        let mut draws = Fixed::new(&[0.5], &[1]);
        let mut arc = ArcEmitter::allocate(3, 5, 1.0);
        arc.anchor = Some(Vec3::new(0.0, 0.0, 5.0));
        let t = &mut arc.tendrils[0];
        t.count = 4;
        for k in 0..4 {
            t.points[k] = Vec3::new(k as f32 * 4.0, 0.0, 0.0);
        }
        arc.bounds();
        // The mean is in the world: the tendril's six, plus the anchor's z.
        assert_eq!(arc.mean, Vec3::new(6.0, 0.0, 5.0));
        let limb = ArcLimb {
            character: 1,
            caster: false,
            at: Vec3::new(4.0, 10.0, 5.0),
            chest: Vec3::new(6.0, 100.0, 5.0),
        };
        let shock = arc
            .strike_limb(&limb, |_| true, &mut draws)
            .expect("a shock");
        assert!((shock.direction - Vec3::Y).length() < 1e-3);
    }

    #[test]
    fn a_node_follows_its_local_frame_parent() {
        let parent: NodeMatrix = [
            [0.0, 1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [10.0, 20.0, 30.0, 1.0],
        ];
        assert_eq!(follow_parent(false, &parent, Vec3::X), None);
        let m = follow_parent(true, &parent, Vec3::new(1.0, 2.0, 3.0)).unwrap();
        assert_eq!(&m[..3], &parent[..3]);
        assert_eq!(m[3], [8.0, 21.0, 33.0, 1.0]);
    }

    #[test]
    fn an_emitter_enters_and_leaves_the_world() {
        let mut node = ArcNode {
            flags: 0x4,
            parent_local_frame: true,
            render: 0x20,
            child: false,
        };
        let mut arc = ArcEmitter::allocate(1, 4, 1.0);
        arc.sound = Some("electricity01");
        arc.lights.push(LightPoint {
            at: Vec3::ZERO,
            weight: 1.0,
            count: 1,
        });
        attach(&mut node);
        assert_eq!((node.flags, node.render), (0x1_0004, 0x20));
        assert!(node.child);
        let start = arc.add_to_world(
            &mut node,
            Vec3::new(5.0, 6.0, 7.0),
            Vec3::new(5.0, 0.0, 7.0),
        );
        assert_eq!(start.render, 0x20 | LOCAL_FRAME);
        assert_eq!(start.mean, Vec3::new(5.0, 6.0, 7.0));
        assert_eq!(
            start.hum,
            Some(HumStart {
                at: Vec3::new(5.0, 0.0, 7.0)
            })
        );
        assert!(arc.in_world);
        assert_eq!(arc.flags, 0, "the record flags are left alone");
        let removal = arc.remove_from_world();
        assert_eq!(
            removal,
            ArcRemoval {
                sound: true,
                lights: 1
            }
        );
        assert!(arc.lights.is_empty() && !arc.in_world);
        let teardown = arc.destroy();
        assert!(teardown.sound);
        assert!(arc.tendrils.is_empty());
        assert_eq!(arc.total, 0);
        let mut plain = ArcNode::default();
        ArcEmitter::allocate(1, 4, 1.0).add_to_world(&mut plain, Vec3::ZERO, Vec3::ZERO);
        assert_eq!(plain.render, 0, "no local-frame parent: no bit");
    }

    #[test]
    fn the_light_flickers_within_its_bounds() {
        let mut draws = Fixed::new(&[1.0, 0.0, 0.0, 0.0], &[1]);
        let mut arc = ArcEmitter::allocate(2, 5, 1.0);
        arc.light_kind = ArcLight::Voxel;
        arc.light = 10.0;
        arc.sound = Some("electricity01");
        arc.tendrils[0].count = 5;
        arc.total = 5;
        let frame = arc.frame(0.5, Vec3::Z, 0.0, |p| p, &mut draws);
        assert_eq!(arc.light_flicker, 0.75);
        assert_eq!(frame.fill, 1.0);
        assert_eq!(frame.light, 10.0 * 0.75 * 1.5);
        assert_eq!(arc.gain_flicker, 0.5);
        assert_eq!(arc.pitch_flicker, 0.8);
        assert!((frame.gain - 0.05).abs() < 1e-7);
        assert!((frame.pitch.expect("a fill above zero") - 0.9).abs() < 1e-6);
    }

    #[test]
    fn the_hermite_point_runs_between_its_ends() {
        let points = [Vec3::ZERO, Vec3::X, Vec3::X * 2.0];
        assert_eq!(hermite(&points, 1, 0.0), Vec3::ZERO);
        assert!((hermite(&points, 1, 1.0) - Vec3::X).length() < 1e-6);
        assert!((hermite(&points, 2, 0.5) - Vec3::X * 1.5).length() < 1e-6);
    }

    #[test]
    fn a_ribbon_pairs_every_point_and_midpoint() {
        let mut draws = Fixed::new(&[0.5], &[1]);
        let mut arc = ArcEmitter::allocate(1, 4, 1.0);
        let t = &mut arc.tendrils[0];
        t.count = 3;
        for k in 0..3 {
            t.points[k] = Vec3::new(k as f32 * 4.0, 0.0, 0.0);
        }
        arc.total = 3;
        let frame = arc.frame(1.0, Vec3::Z, 0.0, |p| p, &mut draws);
        let r = &frame.ribbon;
        // Two pairs a point: 3 points make 6 pairs.
        assert_eq!(r.positions.len(), 12);
        assert_eq!(&r.alphas[..2], &[0, 0]);
        assert_eq!(&r.alphas[10..], &[0, 0]);
        assert!(r.alphas[2..10].iter().all(|&a| a == 0xff));
        // 2·6 − 2 triangles, one fewer at the end.
        assert_eq!(r.index_count, 9 * 3);
        assert_eq!(&r.drawn()[..6], &[0, 1, 3, 0, 3, 2]);
        assert_eq!(&r.drawn()[6..12], &[2, 3, 5, 2, 5, 4]);
        // The ribbon spans the width across the line from the eye.
        assert!((r.positions[0] - Vec3::new(0.0, 2.0, 0.0)).length() < 1e-5);
        assert!((r.positions[1] - Vec3::new(0.0, -2.0, 0.0)).length() < 1e-5);
        assert!(!arc.lights.is_empty());
    }

    #[test]
    fn light_points_merge_within_forty_centimetres() {
        let mut arc = ArcEmitter::allocate(1, 4, 1.0);
        arc.light_point(Vec3::ZERO, 1.0);
        arc.light_point(Vec3::new(30.0, 0.0, 0.0), 1.0);
        assert_eq!(arc.lights.len(), 1);
        assert_eq!(arc.lights[0].count, 2);
        assert_eq!(arc.lights[0].at, Vec3::new(15.0, 0.0, 0.0));
        assert_eq!(arc.lights[0].weight, 2.0);
        arc.light_point(Vec3::new(100.0, 0.0, 0.0), 1.0);
        assert_eq!(arc.lights.len(), 2);
    }

    #[test]
    fn the_shock_burst_is_ten_long_tendrils() {
        // The params the burst's creator sets.
        let params = shock_burst();
        assert_eq!(params.flags, 0x0003_0310);
        assert_eq!((params.count, params.segments, params.life), (10, 14, 40));
        assert_eq!(
            (params.size, params.scale, params.growth),
            (10.0, 0.25, 0.5)
        );
        assert_eq!((params.lift, params.volume), (1.0, 0.05));
        // A live emitter of them: ten records of fourteen points.
        let mut draws = Fixed::new(&[0.5], &[1]);
        let emitter = ArcEmitter::new(&params, &mut draws);
        assert_eq!(emitter.tendrils.len(), 10);
        assert_eq!(emitter.segments, 14);
        assert_eq!(emitter.light, 0.0);
        assert_eq!(emitter.sound, None);
        assert_eq!(emitter.volume, 0.05);
    }

    #[test]
    fn a_spark_burst_starts_six() {
        let mut draws = Fixed::new(&[0.5, 1.0, 0.5], &[2]);
        let burst = spark_burst(&mut draws);
        let live = burst.tendrils.iter().filter(|t| t.count > 0).count();
        assert_eq!(live, 7);
        assert_eq!(burst.length, 24.0);
        assert!(burst.ending);
        assert_eq!(burst.tendrils[0].timer, 12);
    }
}
