//! Displacement wisps, as the original's tendril effect runs them: flame
//! strands (texture `FlameletsA3`) that curl up from anchors on a phased
//! body, or from the point where a blow passed through it, and trail away
//! as rings of eight nodes. Lengths are in cm; the simulation runs at the
//! body tick.
//!
//! - **Setup** ([`Wisps::new`], [`Wisps::place`]): each strand's budget is
//!   twice the 83-tick rate (166). A new node starts 0.5 cm from its anchor
//!   (5 cm under the anchor's point) along a random rising direction
//!   (`(0.3·sin a, 1, 0.3·cos a)` normalised, `a` a turn of the engine's
//!   generator); nodes are 40 cm apart, each grows 0.1 a tick from 12, and
//!   the fade-in runs at five sixths of the tick in seconds a tick. A body
//!   set starts three nodes up, the head unlit, the two behind it at half
//!   the budget and the oldest dark; a set from a blow starts lit with one
//!   node and spends 50 of its budget a node.
//! - **Each tick** ([`Wisps::tick`]): the node step eases from its start
//!   to 0.5 by 0.01 a tick. Fading in, the anchors ease a tenth of the way
//!   toward their points and the nodes hang 15 cm a step above them; past
//!   1 the strands start. Running, the alpha rises 0.03 a tick to 1; each
//!   strand's anchor follows its point (unless frozen) and, once its head
//!   is more than 40 cm away, a new head starts there along the mean of a
//!   fresh rising direction and the last, lit with the budget (ramped by
//!   `q(2 − q)` over the first 80 ticks of a body set, by the fade while it
//!   fades out, dark once the strand stops), the budget spent and the
//!   texture distance carried on. Every node darkens by 1 a tick (by 2 to
//!   6 for the oldest four), grows 0.1, and keeps 0.9998 of its Verlet
//!   motion. Fading out runs 0.02 a tick, then every strand stops.
//! - **Each frame** ([`Wisps::ribbon`]): each strand's lit nodes, from the
//!   anchor out, are smoothed into a camera-facing strip whose half width is
//!   each node's size, its colour the strand's two random channels with
//!   full blue and an alpha of the eased fade squared, its texture running
//!   along by 0.04 a cm (more while it fades out). An effect with nothing
//!   left to draw leaves the scene.
//!
//! Evidence: the effect's geometry setup, add-to-world, tick, ribbon and
//! strip helper, the random direction, the fade and freeze calls and the
//! creators on bodies and blows, read from `Exanima.exe`
//! (`context/re/thaum-wisps.md`).

use bevy::math::{Affine3A, Vec3};

use super::engine::{Roll, dist_sq, length, lerp, normalize, sin_cos};

/// Nodes in a strand's ring.
pub const RING: usize = 8;
/// The strip keeps at most this many points of a strand.
pub const STRIP_POINTS: usize = 10;

/// One node of a strand.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WispNode {
    pub at: Vec3,
    pub previous: Vec3,
    pub size: f32,
    /// How lit it is (0 dark).
    pub light: i32,
    /// Its texture distance along the strand.
    pub along: f32,
}

/// Strand flags.
pub mod flag {
    /// It stops lighting new nodes.
    pub const DIMMED: u32 = 1;
    /// It starts no more nodes.
    pub const STOPPED: u32 = 4;
    /// Its anchor no longer follows its point.
    pub const FROZEN: u32 = 8;
}

/// One strand.
#[derive(Debug, Clone, PartialEq)]
pub struct Strand {
    pub head: usize,
    pub nodes: [WispNode; RING],
    /// Its point: in its body's frame, or the effect's.
    pub local: Vec3,
    /// The body whose frame holds the point, as a slot of the caller's.
    pub body: Option<usize>,
    pub anchor: Vec3,
    pub anchor_before: Vec3,
    pub direction: Vec3,
    /// Ticks since its head started.
    pub since: i32,
    pub budget: i32,
    pub anchor_along: f32,
    /// Its two random colour channels.
    pub colour: [f32; 2],
    pub flags: u32,
}

impl Strand {
    /// A strand on `local` of `body`.
    pub fn new(local: Vec3, body: Option<usize>) -> Self {
        Self {
            head: 0,
            nodes: [WispNode::default(); RING],
            local,
            body,
            anchor: Vec3::ZERO,
            anchor_before: Vec3::ZERO,
            direction: Vec3::Y,
            since: 8,
            budget: 0,
            anchor_along: 0.0,
            colour: [0.0, 0.0],
            flags: 0,
        }
    }
}

/// Effect states.
pub mod state {
    /// Fading in.
    pub const FADING_IN: i32 = 0;
    pub const RUNNING: i32 = 1;
    /// Fading out.
    pub const FADING_OUT: i32 = 0x10;
    /// Faded out: every strand stopped.
    pub const DONE: i32 = 0x30;
}

/// The wisps' simulation.
#[derive(Debug, Clone, PartialEq)]
pub struct Wisps {
    pub strands: Vec<Strand>,
    /// Strands that take part in the fade-in.
    pub fading: usize,
    pub state: i32,
    pub alpha: f32,
    pub budget: i32,
    pub rate: f32,
    pub size: f32,
    /// What each new node spends of its strand's budget.
    pub spend: i32,
    pub scroll: [f32; 2],
    /// Added to the bounds.
    pub margin: f32,
    /// The effect's own frame and up.
    pub frame: Affine3A,
    pub up: Vec3,
    pub step: f32,
    pub spacing: f32,
    pub ramp: i32,
    pub growth: f32,
    pub fade: f32,
    /// A set from a blow: lit at once.
    pub instant: bool,
    pub centre: Vec3,
    pub extent: Vec3,
    pub radius: f32,
    /// It left the scene.
    pub removed: bool,
}

/// The tick rate the strands' budget is twice of.
pub const TICKS_PER_SECOND: i32 = 83;

impl Wisps {
    /// The effect as its geometry setup leaves it: every strand at its
    /// point, unlit, `fading` strands taking part in the fade-in. `world`
    /// places a strand's point in the world. `draws` gives each strand its
    /// two colour channels in turn.
    pub fn new(
        strands: Vec<Strand>,
        size: f32,
        world: impl Fn(&Strand) -> Vec3,
        mut unit: impl FnMut() -> f32,
    ) -> Self {
        let mut wisps = Self {
            fading: strands.len(),
            strands,
            state: state::FADING_IN,
            alpha: 0.0,
            budget: (TICKS_PER_SECOND as f32 * 2.0).round_ties_even() as i32,
            rate: 0.0,
            size,
            spend: 0,
            scroll: [0.0, 0.0],
            margin: 0.0,
            frame: Affine3A::IDENTITY,
            up: Vec3::Y,
            step: 0.5,
            spacing: 40.0,
            ramp: 80,
            growth: 0.1,
            fade: 0.0,
            instant: false,
            centre: Vec3::ZERO,
            extent: Vec3::ZERO,
            radius: 0.0,
            removed: false,
        };
        for s in &mut wisps.strands {
            s.head = 0;
            s.colour = [unit(), unit()];
            s.since = 8;
            let mut at = world(s);
            at.y -= 5.0;
            s.nodes[0].at = at;
            s.nodes[0].previous = at;
            s.anchor = at;
            s.anchor_before = at;
            s.direction = Vec3::Y;
        }
        wisps
    }

    /// Places the strands as the effect enters the world: running, the
    /// step 0.5 and the spacing 40 (an 80-tick ramp), lit at once for a set
    /// from a blow, fading in at five sixths of `tick_ms` in seconds.
    pub fn place(&mut self, tick_ms: u32, world: impl Fn(&Strand) -> Vec3, roll: &mut Roll) {
        self.state = state::RUNNING;
        self.step = 0.5;
        self.spacing = 40.0;
        self.ramp = (self.spacing / self.step).round_ties_even() as i32;
        self.growth = 0.1;
        self.alpha = if self.instant { 1.0 } else { 0.0 };
        let rate = (f64::from(tick_ms) * 0.001) as f32;
        self.rate = (f64::from(rate) * 0.833_333_333_333_333_4) as f32;
        let (step, spacing, ramp, growth, size, budget) = (
            self.step,
            self.spacing,
            self.ramp,
            self.growth,
            self.size,
            self.budget,
        );
        let instant = self.instant;
        for s in &mut self.strands {
            s.budget = budget;
            let dir = rising(roll);
            s.direction = dir;
            s.head = if instant { 1 } else { 3 };
            let point = world(s);
            let head = s.head;
            let n = &mut s.nodes[head];
            n.previous = point;
            n.previous.y -= 5.0;
            n.at = n.previous + dir * step;
            n.light = 0;
            n.size = size;
            n.along = 0.0;
            for j in (0..head).rev() {
                let behind = (head - j) as f32;
                let n = &mut s.nodes[j];
                n.previous = point;
                n.previous.y -= 5.0;
                n.previous += dir * (behind * spacing);
                n.at = n.previous + dir * (step * 1.0);
                n.light = budget >> 1;
                n.size = behind * ramp as f32 * growth + size;
                n.along = behind * spacing * 0.003;
            }
            s.nodes[1].light = 0;
            s.nodes[0].light = 0;
            s.anchor = s.nodes[1].at;
        }
    }

    /// Starts the fade-out.
    pub fn fade_out(&mut self) {
        self.fade = 0.0;
        self.state = state::FADING_OUT;
    }

    /// Freezes every strand's anchor.
    pub fn freeze(&mut self) {
        for s in &mut self.strands {
            s.flags |= flag::FROZEN;
        }
    }

    /// One tick. `world` places a strand's point in the world.
    pub fn tick(&mut self, world: impl Fn(&Strand) -> Vec3, roll: &mut Roll) {
        self.scroll[1] = self.scroll[0];
        self.scroll[0] += 0.00075;
        if self.state == state::FADING_OUT {
            self.fade += 0.02;
            if 1.0 <= self.fade {
                self.fade = 1.0;
                self.state = state::DONE;
                for s in &mut self.strands {
                    s.flags |= flag::DIMMED;
                }
            }
        }
        if 0.5 < self.step {
            self.step = (self.step - 0.01).max(0.5);
        }
        if self.state < 1 {
            self.fade_in(&world);
        } else {
            self.alpha = ((f64::from(self.alpha) + 0.03) as f32).min(1.0);
            for k in 0..self.strands.len() {
                self.run_strand(k, &world, roll);
            }
        }
        self.bounds();
    }

    fn fade_in(&mut self, world: &impl Fn(&Strand) -> Vec3) {
        let up = self.up;
        for s in self.strands.iter_mut().take(self.fading) {
            s.anchor_before = s.anchor;
            let point = world(s);
            let toward = Vec3::new(point.x, point.y - 5.0, point.z) - s.anchor;
            s.anchor += toward * 0.1;
            for j in (1..=s.head).rev() {
                let n = &mut s.nodes[j];
                n.previous = n.at;
                n.at = point + up * (((s.head - j) as f32) + 1.0) * 15.0;
            }
        }
        if self.state < 0 {
            self.alpha -= self.rate * 2.0;
            if self.alpha < 0.0 {
                self.alpha = 0.0;
                self.state = -2;
            }
        } else {
            self.alpha += self.rate;
            if 1.0 < self.alpha {
                self.alpha = 1.0;
                self.state = state::RUNNING;
                let step = self.step;
                for s in self.strands.iter_mut().take(self.fading) {
                    let head = s.head;
                    s.nodes[head].at = s.nodes[head].previous + up * step;
                    // The node behind a head of 0 lies outside the ring.
                    if let Some(behind) = head.checked_sub(1) {
                        s.nodes[behind].at = s.nodes[behind].previous + up * step;
                    }
                }
            }
        }
    }

    fn run_strand(&mut self, k: usize, world: &impl Fn(&Strand) -> Vec3, roll: &mut Roll) {
        let (spacing, step, size, ramp, growth, spend) = (
            self.spacing,
            self.step,
            self.size,
            self.ramp,
            self.growth,
            self.spend,
        );
        let fading_out = self.state & state::FADING_OUT != 0;
        let fade = self.fade;
        let instant = self.instant;
        let s = &mut self.strands[k];
        s.anchor_before = s.anchor;
        if s.flags & flag::FROZEN == 0 {
            let mut at = world(s);
            at.y -= 5.0;
            s.anchor = at;
        }
        if s.flags & flag::STOPPED == 0 {
            s.since += 1;
            let d = s.nodes[s.head].at - s.anchor;
            if spacing * spacing < (d.z * d.z + d.y * d.y) + d.x * d.x {
                let old = s.head;
                let new = if old + 1 > RING - 1 { 0 } else { old + 1 };
                s.head = new;
                s.nodes[new].previous = s.anchor;
                let fresh = rising(roll);
                let dir = fresh + (s.direction - fresh) * 0.5;
                s.direction = dir;
                s.nodes[new].at = s.nodes[new].previous + dir * step;
                s.nodes[new].light = if s.flags & flag::DIMMED != 0 {
                    s.flags |= flag::STOPPED;
                    0
                } else if fading_out {
                    ((1.0 - fade) * s.budget as f32).round_ties_even() as i32
                } else if s.since < ramp && !instant {
                    let q = s.since as f32 / ramp as f32;
                    let q = (2.0 - q) * q;
                    (s.budget as f32 * q).round_ties_even() as i32
                } else {
                    s.budget
                };
                s.budget -= spend;
                if s.budget < 1 {
                    s.flags |= flag::DIMMED | flag::FROZEN;
                }
                s.nodes[new].size = size;
                let gap = length(s.nodes[new].at - s.nodes[old].at);
                s.nodes[new].along = gap * 0.003 + s.nodes[old].along;
                s.since = 0;
            }
        }
        let mut idx = s.head;
        for age in 0..RING {
            let n = &mut s.nodes[idx];
            n.light -= if age < 4 { 1 } else { age as i32 - 2 };
            if n.light < 0 {
                n.light = 0;
            }
            n.size += growth;
            let v = n.at - n.previous;
            n.previous = n.at;
            n.at += v * 0.9998;
            idx = if idx == 0 { RING - 1 } else { idx - 1 };
        }
        let d2 = dist_sq(s.anchor, s.nodes[s.head].at);
        if super::arcs::EPSILON < d2 {
            s.anchor_along = d2.sqrt() * 0.003 + s.nodes[s.head].along;
        }
    }

    fn bounds(&mut self) {
        let Some(first) = self.strands.first() else {
            return;
        };
        let anchored = first.flags & flag::STOPPED == 0;
        let start = if anchored {
            first.anchor
        } else {
            first.nodes[first.head].at
        };
        let (mut lo, mut hi) = (start, start);
        let mut grow = |p: Vec3| {
            for axis in 0..3 {
                if lo[axis] <= p[axis] {
                    if hi[axis] < p[axis] {
                        hi[axis] = p[axis];
                    }
                } else {
                    lo[axis] = p[axis];
                }
            }
        };
        for s in &self.strands {
            if anchored {
                grow(s.anchor);
            }
            for n in &s.nodes {
                if 0 < n.light {
                    grow(n.at);
                }
            }
        }
        self.centre = lerp(lo, hi, 0.5);
        self.extent = hi - self.centre + Vec3::splat(40.0 + self.margin);
        let e = self.extent;
        let r2 = (e.z * e.z + e.y * e.y) + e.x * e.x;
        self.radius = if 0.0 < r2 { r2.sqrt() } else { r2 };
    }

    /// One frame's strips, `alpha` of the way to the next tick, `eye` the
    /// camera's axis. Marks the effect removed when nothing is drawn.
    pub fn ribbon(&mut self, alpha: f32, eye: Vec3) -> WispRibbon {
        let mut out = WispRibbon {
            scroll: (1.0 - alpha) * self.scroll[1] + self.scroll[0] * alpha,
            ..WispRibbon::default()
        };
        let eased = (2.0 - self.alpha) * self.alpha;
        let width = if self.state & state::FADING_OUT != 0 {
            self.fade * 0.15 + 0.04
        } else {
            0.04
        };
        for s in &self.strands {
            let colour = [
                (s.colour[0] * 255.0).round_ties_even() as u8,
                (s.colour[1] * 255.0).round_ties_even() as u8,
                0xff,
                (eased * eased * 255.0).round_ties_even() as u8,
            ];
            let mut points: Vec<Vec3> = Vec::with_capacity(STRIP_POINTS + 1);
            let mut attrs: Vec<[f32; 4]> = Vec::with_capacity(STRIP_POINTS + 1);
            let mut idx = s.head;
            if s.flags & flag::STOPPED == 0 {
                let mut root = lerp(s.anchor_before, s.anchor, alpha);
                let light = if s.flags & flag::DIMMED == 0 {
                    if self.state & state::FADING_OUT == 0 {
                        1.0
                    } else {
                        1.0 - self.fade
                    }
                } else {
                    if self.instant {
                        let n = &s.nodes[idx];
                        root = lerp(n.previous, n.at, alpha);
                    }
                    0.0
                };
                points.push(root);
                attrs.push([s.anchor_along, light, 12.0, 0.0]);
            }
            let mut last_lit = points.len();
            for k in 0..=STRIP_POINTS - 2 {
                let n = &s.nodes[idx];
                points.push(lerp(n.previous, n.at, alpha));
                attrs.push([n.along, n.light as f32 / self.budget as f32, n.size, 0.0]);
                if 0 < n.light {
                    last_lit = points.len();
                }
                idx = if idx == 0 { RING - 1 } else { idx - 1 };
                if idx == s.head || k >= STRIP_POINTS - 2 {
                    break;
                }
            }
            let count = last_lit + 1;
            if count < 2 {
                continue;
            }
            let end = (last_lit + 2).min(points.len());
            // A lit last point leaves the original reading its scratch past
            // the strand; the last point stands in for it here.
            while points.len() < count {
                points.push(points[points.len() - 1]);
                attrs.push(attrs[attrs.len() - 1]);
            }
            attrs[count - 1][1] = 0.0;
            if end <= count {
                if points.len() <= count {
                    points.push(points[count - 1]);
                    attrs.push(attrs[count - 1]);
                } else {
                    points[count] = points[count - 1];
                    attrs[count] = attrs[count - 1];
                }
            }
            let (smooth, smooth_attrs, drawn) =
                smooth_strip(&points, &attrs, count, end, self.spacing);
            let sides = strip_sides(&smooth, eye);
            let base = out.positions.len() as u32;
            let mut v_along = if s.flags & flag::STOPPED != 0 {
                1.0
            } else {
                0.0
            };
            for j in 0..drawn {
                let (p, a) = (smooth[j], smooth_attrs[j]);
                let w = sides[j] * a[2];
                out.positions.push(p - w);
                out.positions.push(p + w);
                out.uvs.push([0.0, a[0]]);
                out.uvs.push([1.0, a[0]]);
                out.fade.push([a[1], v_along]);
                out.fade.push([a[1], v_along]);
                out.colours.push(colour);
                out.colours.push(colour);
                v_along += length(smooth[j + 1] - p) * width;
                if j > 0 {
                    let q = base + 2 * (j as u32 - 1);
                    out.indices
                        .extend_from_slice(&[q, q + 1, q + 2, q + 2, q + 1, q + 3]);
                }
            }
        }
        if out.indices.is_empty() && !self.removed {
            self.removed = true;
            out.removed = true;
        }
        out
    }
}

/// The strand's smoothed points: the root, a Hermite point a little out
/// (`t = 0.6·|P1 − P0|/spacing`, at most 0.5), then each point and the
/// middle to the next, bent toward its neighbours; one more point past a
/// visible end for the last side. Returns the points, their attributes and
/// how many are drawn.
fn smooth_strip(
    p: &[Vec3],
    a: &[[f32; 4]],
    count: usize,
    end: usize,
    spacing: f32,
) -> (Vec<Vec3>, Vec<[f32; 4]>, usize) {
    let get = |k: usize| p.get(k).copied().unwrap_or(p[p.len() - 1]);
    let attr = |k: usize| a.get(k).copied().unwrap_or(a[a.len() - 1]);
    let mut s = vec![p[0]];
    let mut t_attr = vec![a[0]];
    let t = ((length(get(1) - p[0]) / spacing) * 0.6).min(0.5);
    let cube = t * t * t;
    let mut h = p[0] * ((cube * 2.0 - t * t * 3.0) + 1.0);
    h += get(1) * (t * t * 3.0 + cube * -2.0);
    h += (get(1) - p[0]) * ((cube - t * t * 2.0) + t);
    h += (get(2) - p[0]) * ((cube - t * t) * 0.5);
    s.push(h);
    t_attr.push(lerp4(a[0], attr(1), t));
    for j in 1..count {
        s.push(get(j));
        t_attr.push(attr(j));
        let mut m = get(j) * 0.5 + get(j + 1) * 0.5;
        if j == 1 {
            m += (get(2) - get(1)) * ((1.0 - t) * 0.125);
            m += (get(1) - get(0)) * (t * 0.125);
        } else {
            m += (get(j + 1) - get(j - 1)) * 0.0625;
        }
        let (v, w) = if (j as i64) < end as i64 - 1 {
            (get(j + 2) - get(j), 0.5)
        } else {
            (get(j + 1) - get(j), 1.0)
        };
        m += v * (w * -0.125);
        s.push(m);
        t_attr.push(lerp4(attr(j), attr(j + 1), 0.5));
    }
    let drawn = s.len() - 1;
    if count < end {
        s.push(get(count));
        t_attr.push(attr(count));
    } else {
        s.push(s[s.len() - 1]);
        t_attr.push(t_attr[t_attr.len() - 1]);
    }
    (s, t_attr, drawn)
}

fn lerp4(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

/// Each drawn point's side, across its neighbours and the eye, eased
/// toward its neighbours' (a quarter at the root, three quarters at the
/// tip, the mean between). The last of `s` only steers the last side.
fn strip_sides(s: &[Vec3], eye: Vec3) -> Vec<Vec3> {
    let m = s.len().saturating_sub(1);
    if m < 2 {
        return vec![normalize((s[s.len() - 1] - s[0]).cross(eye)); m.max(1)];
    }
    let mut raw = Vec::with_capacity(m);
    raw.push(normalize((s[1] - s[0]).cross(eye)));
    for j in 1..m {
        raw.push(normalize((s[j + 1] - s[j - 1]).cross(eye)));
    }
    let mut out = vec![Vec3::ZERO; m];
    out[0] = lerp(raw[0], raw[1], 0.25);
    for j in 1..m - 1 {
        out[j] = lerp(raw[j - 1], raw[j + 1], 0.5);
    }
    out[m - 1] = lerp(raw[m - 2], raw[m - 1], 0.75);
    out
}

/// A rising direction: `(0.3·sin a, 1, 0.3·cos a)` normalised, `a` a turn
/// of the engine's generator.
pub fn rising(roll: &mut Roll) -> Vec3 {
    let (s, c) = sin_cos(roll.unit() * std::f32::consts::TAU);
    normalize(Vec3::new(s * 0.3, 1.0, c * 0.3))
}

/// The wisps' strips for a frame.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WispRibbon {
    pub positions: Vec<Vec3>,
    pub uvs: Vec<[f32; 2]>,
    /// Each vertex's light share and texture distance.
    pub fade: Vec<[f32; 2]>,
    pub colours: Vec<[u8; 4]>,
    pub indices: Vec<u32>,
    pub scroll: f32,
    /// The effect drew nothing and left the scene this frame.
    pub removed: bool,
}

/// The texture the wisps draw with.
pub const WISP_TEXTURE: &str = "FlameletsA3.rfi";
/// A set from a blow spends this much of its budget a node.
pub const BLOW_SPEND: i32 = 50;

/// A set from a blow: one strand at `local` of `body`, lit at once.
pub fn blow_wisps(local: Vec3, body: usize, world: impl Fn(&Strand) -> Vec3) -> Wisps {
    let mut wisps = Wisps::new(vec![Strand::new(local, Some(body))], 12.0, world, || 0.5);
    wisps.instant = true;
    wisps.spend = BLOW_SPEND;
    wisps.alpha = 1.0;
    wisps
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at_origin(s: &Strand) -> Vec3 {
        s.local
    }

    #[test]
    fn the_rising_direction_leans_a_little() {
        let mut roll = Roll(0);
        let d = rising(&mut roll);
        assert!((d.length() - 1.0).abs() < 1e-3);
        assert!(d.y > 0.95);
        let flat = (d.x * d.x + d.z * d.z).sqrt();
        assert!((flat - 0.3 / (1.09f32).sqrt()).abs() < 1e-3);
    }

    #[test]
    fn a_body_set_starts_three_nodes_up() {
        let strands = vec![Strand::new(Vec3::new(0.0, 100.0, 0.0), None)];
        let mut w = Wisps::new(strands, 12.0, at_origin, || 0.25);
        assert_eq!(w.budget, 166);
        assert_eq!(w.strands[0].colour, [0.25, 0.25]);
        let mut roll = Roll(0);
        w.place(12, at_origin, &mut roll);
        let s = &w.strands[0];
        assert_eq!(s.head, 3);
        assert_eq!(s.nodes[2].light, 83);
        assert_eq!(
            (s.nodes[1].light, s.nodes[0].light, s.nodes[3].light),
            (0, 0, 0)
        );
        assert_eq!(s.nodes[3].size, 12.0);
        assert!((s.nodes[2].size - (80.0 * 0.1 + 12.0)).abs() < 1e-5);
        assert_eq!(w.ramp, 80);
        assert_eq!(w.rate, ((0.012f32 as f64) * 0.833_333_333_333_333_4) as f32);
        // The head's previous place is 5 cm under the point.
        assert_eq!(s.nodes[3].previous, Vec3::new(0.0, 95.0, 0.0));
    }

    #[test]
    fn a_strand_starts_a_node_past_forty_centimetres() {
        let strands = vec![Strand::new(Vec3::ZERO, None)];
        let mut w = Wisps::new(strands, 12.0, at_origin, || 0.5);
        let mut roll = Roll(7);
        w.place(12, at_origin, &mut roll);
        w.alpha = 1.0;
        let moved = |s: &Strand| s.local + Vec3::new(41.0, 0.0, 0.0);
        let head = w.strands[0].head;
        w.strands[0].nodes[head].at = Vec3::new(0.0, -5.0, 0.0);
        w.tick(moved, &mut roll);
        let s = &w.strands[0];
        assert_eq!(s.head, (head + 1) % RING);
        assert_eq!(s.since, 0);
        // Nine ticks since its last head (eight from the setup, then this
        // one) of the 80-tick ramp: q = 9/80, lit q(2 − q) of the budget.
        let q = 9.0f32 / 80.0;
        let lit = (166.0 * ((2.0 - q) * q)).round_ties_even() as i32;
        assert_eq!(s.nodes[s.head].light, lit - 1);
    }

    #[test]
    fn nodes_darken_faster_with_age() {
        let strands = vec![Strand::new(Vec3::ZERO, None)];
        let mut w = Wisps::new(strands, 12.0, at_origin, || 0.5);
        w.state = state::RUNNING;
        w.strands[0].flags = flag::STOPPED;
        for n in &mut w.strands[0].nodes {
            n.light = 100;
        }
        let mut roll = Roll(0);
        w.tick(at_origin, &mut roll);
        let lights: Vec<i32> = (0..RING)
            .map(|age| w.strands[0].nodes[(RING - age) % RING].light)
            .collect();
        assert_eq!(lights, vec![99, 99, 99, 99, 98, 97, 96, 95]);
    }

    #[test]
    fn a_fade_out_dims_every_strand() {
        let strands = vec![Strand::new(Vec3::ZERO, None)];
        let mut w = Wisps::new(strands, 12.0, at_origin, || 0.5);
        w.fade_out();
        let mut roll = Roll(0);
        for _ in 0..60 {
            w.tick(at_origin, &mut roll);
        }
        assert_eq!(w.state, state::DONE);
        assert!(w.strands[0].flags & flag::DIMMED != 0);
    }

    #[test]
    fn a_blow_set_spends_its_budget() {
        let mut w = blow_wisps(Vec3::ZERO, 0, at_origin);
        let mut roll = Roll(3);
        w.place(12, at_origin, &mut roll);
        assert_eq!(w.strands[0].head, 1);
        assert_eq!(w.alpha, 1.0);
        let mut x = 0.0;
        for _ in 0..4 {
            x += 50.0;
            let moved = |s: &Strand| s.local + Vec3::new(x, 0.0, 0.0);
            w.tick(moved, &mut roll);
        }
        // 166 − 4·50 < 1: dimmed and frozen.
        let s = &w.strands[0];
        assert_eq!(s.budget, 166 - 200);
        assert_eq!(
            s.flags & (flag::DIMMED | flag::FROZEN),
            flag::DIMMED | flag::FROZEN
        );
    }

    #[test]
    fn the_fade_in_rises_by_its_rate() {
        let strands = vec![Strand::new(Vec3::ZERO, None)];
        let mut w = Wisps::new(strands, 12.0, at_origin, || 0.5);
        w.rate = 0.5;
        let mut roll = Roll(0);
        w.tick(at_origin, &mut roll);
        assert_eq!(w.alpha, 0.5);
        assert_eq!(w.state, state::FADING_IN);
        w.tick(at_origin, &mut roll);
        w.tick(at_origin, &mut roll);
        assert_eq!(w.state, state::RUNNING);
        assert_eq!(w.alpha, 1.0);
    }

    #[test]
    fn a_lit_strand_draws_a_strip() {
        let strands = vec![Strand::new(Vec3::ZERO, None)];
        let mut w = Wisps::new(strands, 12.0, at_origin, || 0.5);
        let mut roll = Roll(5);
        w.place(12, at_origin, &mut roll);
        for n in &mut w.strands[0].nodes {
            n.light = 100;
        }
        w.alpha = 1.0;
        let r = w.ribbon(1.0, Vec3::Z);
        assert!(!r.removed);
        assert!(!r.indices.is_empty());
        assert_eq!(r.positions.len() % 2, 0);
        assert_eq!(r.colours[0], [128, 128, 255, 255]);
        let dark = Wisps::new(vec![Strand::new(Vec3::ZERO, None)], 12.0, at_origin, || 0.5);
        let mut dark = dark;
        dark.strands[0].flags = flag::STOPPED;
        assert!(dark.ribbon(1.0, Vec3::Z).removed);
    }
}
