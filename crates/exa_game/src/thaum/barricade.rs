//! The barricade, as the original's force barricade effect runs it: a wall
//! of force 600 by 200 cm (5 cm thick, its edges rounded) set at the cursor
//! facing away from the caster. It grows out from its middle, shoves away
//! what stands in it, ripples where it is struck, blocks the way it faces,
//! is stamped into the navigation grid while it stands, and sinks when its
//! power ends. Lengths are in cm, the frame's x along the wall, y up and z
//! its normal (the facing).
//!
//! - **Raising** ([`Barricade::raise`]): the wall starts rising at the
//!   clock with its ripple reach 30. Its two reaches (300 cm each way) are
//!   trimmed by the level: a ray from the middle along each side, 40 cm
//!   short of the reach, that meets the level leaves the reach 40 cm past
//!   the hit. The line between the ends is stamped into the navigation grid
//!   ([`NavStamp`]).
//! - **Each tick** ([`Barricade::tick`]): the ripple reach is 30 + 1.25 per
//!   ripple. A struck ripple's target moves to the mean of its hits (half
//!   way there once it shows). Every ripple then loses 0.03 of strength,
//!   its shown value moves toward that by at most 0.04, its place eases
//!   0.05 toward its target, and it goes once both are under 0.03. Rising
//!   (state 1), the share t of 1.6 s since the start is eased twice
//!   (e = t(2 − t), then e(2 − e)·0.9 + 0.1) and the wall spans e·300 to
//!   each side within its reaches, its half width that less 15 and its half
//!   height the node's less 10; at t = 1 it stands (state 0). Told to end
//!   (state 2), it falls from where it is over 750 ms (state 3): its half
//!   height is (1 − t⁴/2)·100 − 10, and at the end (state 4) it leaves the
//!   grid and the AI's sight.
//! - **Bodies** ([`Barricade::shove`]): a body within its radius plus 10
//!   of the plane and its radius plus the half height of the middle is
//!   tested sphere by sphere against the wall's rounded slab. Each sphere
//!   inside is pushed along the normal by ((r + 10) − |z|)·0.04 + 0.1 (at
//!   most 0.5) times m/(m + 1), a tenth of that from behind, and strikes a
//!   ripple of three times the push (a new one when none lies within the
//!   ripple reach, at most 8). A struck character's limb in front, its
//!   chest within 30 times its scale of the plane, is knocked back 40 along
//!   the normal, its instability counter set to 100 and its energy above
//!   0.25 lowered by (30 − d)·0.0002 + 0.002; the impact sound plays (pitch
//!   0.6 + 0.4·r, volume 0.3 + 0.2·r) at most every 400 ms. A held weapon in
//!   front interrupts its holder's attack (pitch 0.6 + 0.6·r, volume
//!   0.1 + 0.3·r).
//! - **Each frame** ([`Barricade::frame`]): rising, the shown share eases
//!   as the tick's does (without the 0.9 + 0.1) and slides the mesh's two
//!   halves apart to the wall's half width; its height is
//!   (e²/2 + 1/2)·100 and the texture slides by 1/300 a cm. Falling, the
//!   width shrinks by (1/2 − t⁴/5) and the height by (1 − 3t⁴/4). The hum
//!   starts at gain 0.5 and pitch 1, rises over the first quarter
//!   (e = 4t(2 − 4t)) and settles past three quarters, and while falling
//!   runs 1 − |1/2 − t| at pitch t + 1/4. The ripples feed eight points to
//!   the shader. At state 4 the wall leaves the scene.
//! - **The mesh** ([`barricade_mesh`]): 32 vertices — a front and a back
//!   face 5 cm from the middle, each edge rounded by three rings turned 45°
//!   apart about it — and 60 triangles; the vertices left of the middle come
//!   first, the halves built 270 cm toward the middle, their texture 0.81.
//!
//! Evidence: the barricade's add-to-world, trim, tick, body test, frame and
//! mesh build, and its creator, read from `Exanima.exe`
//! (`context/re/thaum-barricade.md`).

use bevy::math::{Vec2, Vec3};

use super::engine::{Draws, Roll, dist_sq, dot_zyx, normalize, rotate_about, sign};

/// The wall's frame: its middle and axes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallFrame {
    pub at: Vec3,
    /// Along the wall.
    pub x: Vec3,
    pub y: Vec3,
    /// The facing, away from the caster.
    pub z: Vec3,
}

impl WallFrame {
    /// The engine's frame from a facing: y the world's up less its share
    /// of the facing, x across both, each normalised.
    pub fn facing(at: Vec3, z: Vec3) -> Self {
        let up = Vec3::Y;
        let y = normalize(up - z * up.dot(z));
        let x = normalize(y.cross(z));
        Self { at, x, y, z }
    }

    /// A world point in the wall's frame.
    pub fn local(&self, p: Vec3) -> Vec3 {
        let d = p - self.at;
        Vec3::new(d.dot(self.x), d.dot(self.y), d.dot(self.z))
    }

    /// A point of the wall's frame in the world.
    pub fn world(&self, p: Vec3) -> Vec3 {
        self.at + self.x * p.x + self.y * p.y + self.z * p.z
    }
}

/// One ripple on the wall.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ripple {
    pub at: Vec2,
    pub target: Vec2,
    pub shown: f32,
    pub strength: f32,
    pub sum: Vec2,
    pub hits: i32,
}

/// At most this many ripples.
pub const MOST_RIPPLES: usize = 8;

/// The barricade's states.
pub mod state {
    pub const STANDING: i32 = 0;
    pub const RISING: i32 = 1;
    pub const TOLD_TO_END: i32 = 2;
    pub const FALLING: i32 = 3;
    pub const GONE: i32 = 4;
}

/// The barricade's rise time and fall time (ms).
pub const RISE_MS: f64 = 1600.0;
pub const FALL_MS: f32 = 750.0;
/// The wall's full reach each way, its half height and its rounding.
pub const REACH: f32 = 300.0;
pub const HALF_HEIGHT: f32 = 100.0;
pub const THICKNESS: f32 = 5.0;
/// The impact sound's spacing (ms).
pub const SOUND_GAP: u32 = 400;

/// The cells of the navigation grid a barricade blocks: the patch's middle
/// and size (cells) and the points along the wall's line.
#[derive(Debug, Clone, PartialEq)]
pub struct NavStamp {
    pub centre: (i32, i32),
    pub size: (i32, i32),
    pub points: Vec<Vec3>,
}

/// A navigation grid's frame: its size in cells and cells per cm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridFrame {
    pub width: u32,
    pub height: u32,
    pub scale: f32,
}

impl GridFrame {
    /// The cell under a point: half the size plus `round(x·scale − ½)`.
    pub fn cell(&self, p: Vec3) -> (i32, i32) {
        (
            (self.width >> 1) as i32 + (p.x * self.scale - 0.5).round_ties_even() as i32,
            (self.height >> 1) as i32 + (p.z * self.scale - 0.5).round_ties_even() as i32,
        )
    }
}

/// The stamp of the wall's line from `a` to `b`: a patch 7 cells wider and
/// deeper than the line, centred between its ends (one on for an even
/// size), and one point per cell of the longer side, plus one.
pub fn nav_stamp(grid: &GridFrame, a: Vec3, b: Vec3) -> NavStamp {
    let (c0, r0) = grid.cell(a);
    let (c1, r1) = grid.cell(b);
    let w = (c1 - c0).abs() + 7;
    let h = (r1 - r0).abs() + 7;
    let mut centre = ((c1 + c0) >> 1, (r1 + r0) >> 1);
    if w & 1 == 0 {
        centre.0 += 1;
    }
    if h & 1 == 0 {
        centre.1 += 1;
    }
    let long = if h < w {
        (c1 - c0).abs()
    } else {
        (r1 - r0).abs()
    };
    let n = long + 1;
    let mut points = Vec::with_capacity(n as usize);
    points.push(a);
    let d = b - a;
    for k in 1..long {
        points.push(a + d * (k as f32 / (n as f32 - 1.0)));
    }
    if n > 1 {
        points.push(b);
    }
    NavStamp {
        centre,
        size: (w, h),
        points,
    }
}

/// The barricade's simulation.
#[derive(Debug, Clone, PartialEq)]
pub struct Barricade {
    pub frame: WallFrame,
    /// The wall's base middle.
    pub base: Vec3,
    /// How far it may reach along +x and along −x.
    pub reach: [f32; 2],
    pub state: i32,
    /// When the rise or the fall started (ms).
    pub start: u32,
    /// The shown share before and after the last tick.
    pub shown: [f32; 2],
    /// The share the last frame showed.
    pub progress: f32,
    pub centre: Vec3,
    pub half_width: f32,
    pub half_height: f32,
    pub half_depth: f32,
    pub radius: f32,
    pub ripples: Vec<Ripple>,
    pub ripple_reach: f32,
    /// When the impact sound may next play (ms).
    pub sound_at: u32,
    /// The focus it holds.
    pub cost: f32,
    /// The wall's two drawn half widths.
    pub drawn: [f32; 2],
    /// The texture's two random offsets.
    pub texture: [f32; 2],
    /// The node's half size and height (the mesh's).
    pub node_half: Vec3,
    pub stamp: Option<NavStamp>,
}

/// The node's half size the mesh build leaves: 30 across, 100 up, 5 deep.
pub const NODE_HALF: Vec3 = Vec3::new(30.0, 100.0, THICKNESS);

impl Barricade {
    /// Raises a barricade at `frame` from the clock `now`. `trim` casts a
    /// ray (start, unit direction, length) against the level and returns
    /// the hit's distance; `grid` stamps the wall's line.
    pub fn raise(
        frame: WallFrame,
        cost: f32,
        now: u32,
        mut trim: impl FnMut(Vec3, Vec3, f32) -> Option<f32>,
        grid: Option<&GridFrame>,
        draws: &mut impl Draws,
    ) -> Self {
        let mut reach = [REACH, REACH];
        let along = frame.x;
        if let Some(hit) = trim(frame.at, along, reach[0] - 40.0) {
            reach[0] = hit + 40.0;
        }
        if let Some(hit) = trim(frame.at, -along, reach[1] - 40.0) {
            reach[1] = hit + 40.0;
        }
        let texture = [draws.unit() as f32, draws.unit() as f32];
        let stamp = grid.map(|g| {
            let a = frame.at + along * reach[0];
            let b = frame.at + along * -reach[1];
            nav_stamp(g, a, b)
        });
        Self {
            frame,
            base: frame.at,
            reach,
            state: state::RISING,
            start: now,
            shown: [0.0, 0.0],
            progress: 0.0,
            centre: frame.at,
            half_width: 0.0,
            half_height: 0.0,
            half_depth: 0.0,
            radius: 0.0,
            ripples: Vec::new(),
            ripple_reach: 30.0,
            sound_at: 0,
            cost,
            drawn: [0.0, 0.0],
            texture,
            node_half: NODE_HALF,
            stamp,
        }
    }

    /// Its power ended: the wall falls from the next tick.
    pub fn end(&mut self) {
        if self.state == state::RISING || self.state == state::STANDING {
            self.state = state::TOLD_TO_END;
        }
    }

    /// One tick at the clock `now`. Returns true on the tick it is gone
    /// (its grid stamp and its AI entity leave then).
    pub fn tick(&mut self, now: u32) -> bool {
        self.ripple_reach = self.ripples.len() as f32 * 1.25 + 30.0;
        for ripple in &mut self.ripples {
            if 0 < ripple.hits {
                ripple.sum *= 1.0 / ripple.hits as f32;
                if ripple.shown <= 0.0 {
                    ripple.target = ripple.sum;
                } else {
                    ripple.target = ripple.target + (ripple.sum - ripple.target) * 0.5;
                }
                ripple.hits = 0;
            }
            ripple.sum = Vec2::ZERO;
        }
        let mut k = 0;
        while k < self.ripples.len() {
            let r = &mut self.ripples[k];
            r.strength -= 0.03;
            r.shown += (r.strength - r.shown).clamp(-0.04, 0.04);
            if 0.03 <= r.shown || 0.03 <= r.strength {
                r.at = r.at + (r.target - r.at) * 0.05;
                k += 1;
            } else {
                self.ripples.swap_remove(k);
            }
        }
        self.shown[0] = self.shown[1];
        match self.state {
            state::RISING => {
                let t = (f64::from(now.wrapping_sub(self.start)) * 0.000625) as f32;
                self.shown[1] = t;
                if 1.0 < t {
                    self.shown[1] = 1.0;
                    self.state = state::STANDING;
                }
                let e = self.shown[1] * (2.0 - self.shown[1]);
                let e = e * (2.0 - e) * 0.9 + 0.1;
                let left = (e * REACH).min(self.reach[0]);
                let right = (e * REACH).min(self.reach[1]);
                self.half_width = (left + right) * 0.5;
                let offset = ((e * REACH - right) + (left - e * REACH)) * 0.5;
                self.centre = self.base + self.frame.x * offset;
                self.half_width -= 15.0;
                self.half_height = self.node_half.y - 10.0;
                self.half_depth = self.node_half.z;
                self.radius = self.half_width;
                false
            }
            state::TOLD_TO_END => {
                self.state = state::FALLING;
                self.shown[1] = 1.0 - self.shown[1];
                self.shown[0] = self.shown[1];
                self.progress = self.shown[1];
                self.start = now.wrapping_sub((self.shown[1] * FALL_MS).round_ties_even() as u32);
                false
            }
            state::FALLING => {
                let t = (f64::from(now.wrapping_sub(self.start)) * (1.0 / 750.0)) as f32;
                self.shown[1] = t;
                let mut gone = false;
                if 1.0 < t {
                    self.state = state::GONE;
                    self.shown[1] = 1.0;
                    gone = true;
                }
                let t2 = self.shown[1] * self.shown[1];
                self.half_height = (1.0 - t2 * t2 * 0.5) * 100.0 - 10.0;
                gone
            }
            _ => false,
        }
    }

    /// One body against the wall: its middle and radius, its spheres in the
    /// world, its mass, and whose it is. Returns what the wall does to it.
    pub fn shove(&mut self, body: &WallBody, now: u32, roll: &mut Roll) -> WallShove {
        let mut out = WallShove::default();
        let c = self.frame.local(body.centre);
        if !(c.z.abs() <= body.radius + 10.0 && c.y.abs() <= body.radius + self.half_height) {
            return out;
        }
        let share = body.mobility / (body.mobility + 1.0);
        let inner = (self.half_width - (self.half_height + 10.0)).max(0.0);
        let round = self.half_height;
        let mut front = false;
        let mut depth = 0.0;
        for (k, &(centre, r)) in body.spheres.iter().enumerate() {
            let p = self.frame.local(centre);
            let side = p.x.abs() - inner;
            let reach = r + round;
            let inside = p.z.abs() <= r + 10.0
                && (side <= 0.0 || p.y * p.y + side * side <= reach * reach)
                && p.y.abs() - self.half_height <= r;
            if !inside {
                continue;
            }
            out.touched = true;
            let mut push = ((r + 10.0) - p.z.abs()) * 0.04 + 0.1;
            if 0.5 < push.abs() {
                push = sign(push) * 0.5;
            }
            if p.z <= 0.0 {
                push *= 0.1;
            } else {
                front = true;
                depth = p.z;
            }
            out.pushes.push((k, self.frame.z * (push * share * 1.0)));
            self.ripple(Vec2::new(p.x, p.y), push * 3.0);
        }
        if !front {
            return out;
        }
        match body.kind {
            WallBodyKind::Limb { chest, scale } => {
                let n = self.frame.z;
                let d = dot_zyx(chest, n) - dot_zyx(self.frame.at, n);
                if d < scale * 30.0 && 0.0 < d {
                    out.knockback = Some(knock_offset(n, 40.0, body.step));
                    out.instability = Some(100);
                    out.energy_loss = (30.0 - d) * 0.0002 + 0.002;
                    if self.sound_at < now {
                        let pitch = roll.unit() * 0.4 + 0.6;
                        let volume = roll.unit() * 0.2 + 0.3;
                        out.sound = Some(WallSound {
                            depth,
                            volume,
                            pitch,
                        });
                        self.sound_at = now.wrapping_add(SOUND_GAP);
                        out.alarm = true;
                    }
                }
            }
            WallBodyKind::Weapon => {
                out.interrupt = true;
                if self.sound_at < now {
                    let pitch = roll.unit() * 0.6 + 0.6;
                    let volume = roll.unit() * 0.3 + 0.1;
                    out.sound = Some(WallSound {
                        depth,
                        volume,
                        pitch,
                    });
                    self.sound_at = now.wrapping_add(SOUND_GAP);
                }
            }
            WallBodyKind::Other => {}
        }
        out
    }

    fn ripple(&mut self, at: Vec2, strength: f32) {
        let mut best = self.ripple_reach * self.ripple_reach;
        let mut nearest = None;
        for (k, r) in self.ripples.iter().enumerate() {
            let d = r.target.distance_squared(at);
            if d < best {
                nearest = Some(k);
                best = d;
            }
        }
        match nearest {
            Some(k) => {
                let r = &mut self.ripples[k];
                r.hits += 1;
                r.sum += at;
                if r.strength < strength {
                    r.strength = strength;
                }
            }
            None if self.ripples.len() < MOST_RIPPLES => self.ripples.push(Ripple {
                at,
                target: at,
                shown: 0.0,
                strength,
                sum: at,
                hits: 1,
            }),
            None => {}
        }
    }

    /// One frame, `alpha` of the way to the next tick: the mesh's halves,
    /// the hum, the ripples for the shader. `None` once the wall is gone.
    pub fn frame(&mut self, alpha: f32) -> Option<WallFrameOut> {
        let mut out = WallFrameOut::default();
        match self.state {
            state::RISING => {
                out.mode = 1;
                self.progress = (1.0 - alpha) * self.shown[0] + self.shown[1] * alpha;
                let e = self.progress * (2.0 - self.progress);
                let e = e * (2.0 - e);
                let left = (e * REACH).min(self.reach[0]);
                let right = (e * REACH).min(self.reach[1]);
                self.drawn = [left, right];
                out.half_width = (left + right) * 0.5;
                out.half_height = (e * e * 0.5 + 0.5) * 100.0;
                let offset = ((e * REACH - right) + (left - e * REACH)) * 0.5;
                out.at = self.base + self.frame.x * offset;
                out.texture = offset * 0.003_333_333_4 + self.texture[0];
                out.slide = REACH - out.half_width;
            }
            state::FALLING => {
                out.mode = 2;
                let t = (1.0 - alpha) * self.shown[0] + self.shown[1] * alpha;
                self.progress = t;
                let t2 = t * t;
                out.half_width = (0.5 - t2 * t2 * 0.2) * (self.drawn[0] + self.drawn[1]);
                out.half_height = (1.0 - t2 * t2 * 0.75) * 100.0;
            }
            state::GONE => return None,
            _ => {}
        }
        out.hum = match self.state {
            state::STANDING => Some((0.5, 1.0)),
            state::RISING if self.progress < 0.25 => {
                let e = self.progress * 4.0;
                let e = e * (2.0 - e);
                Some((e, 0.75 - e * 0.25))
            }
            state::RISING if 0.75 < self.progress => {
                let e = 1.0 - (self.progress - 0.75) * 4.0;
                Some((e * 0.5 + 0.5, 0.75 - e * 0.25))
            }
            state::RISING | state::TOLD_TO_END => None,
            _ => Some((
                1.0 - (0.5 - self.progress * 1.0).abs(),
                self.progress + 0.25,
            )),
        };
        for (k, r) in self.ripples.iter().enumerate() {
            out.ripples[k] = Vec3::new(r.at.x, r.at.y, r.shown * 1.0);
        }
        Some(out)
    }
}

/// A body the wall is tested against.
#[derive(Debug, Clone, PartialEq)]
pub struct WallBody {
    pub centre: Vec3,
    pub radius: f32,
    /// Its mobility: a push moves it by m/(m + 1) of itself.
    pub mobility: f32,
    /// Its character's step length (cm), which caps a knockback.
    pub step: f32,
    /// Its spheres in the world: centre and radius.
    pub spheres: Vec<(Vec3, f32)>,
    pub kind: WallBodyKind,
}

/// Whose a body is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WallBodyKind {
    /// A character's limb: the character's chest and scale.
    Limb {
        chest: Vec3,
        scale: f32,
    },
    /// A weapon a character holds.
    Weapon,
    Other,
}

/// What the wall does to a body.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WallShove {
    /// Something touched the wall: the body is woken for 32 ticks.
    pub touched: bool,
    /// Each sphere's push (cm) along the wall's normal.
    pub pushes: Vec<(usize, Vec3)>,
    /// The character's knockback step (cm, from its chest).
    pub knockback: Option<Vec3>,
    pub instability: Option<i32>,
    /// Taken from the character's energy while it is above 0.25.
    pub energy_loss: f32,
    /// The holder's attack is interrupted.
    pub interrupt: bool,
    /// The AI notices the wall as an obstacle.
    pub alarm: bool,
    pub sound: Option<WallSound>,
}

/// A knockback step: the push's direction on the level times twice the
/// push, at most the step length.
pub fn knock_offset(direction: Vec3, push: f32, step: f32) -> Vec3 {
    let mut v = Vec3::new(direction.x, 0.0, direction.z) * (push * 2.0);
    let len2 = v.z * v.z + v.y * v.y + v.x * v.x;
    if step * step < len2 {
        v *= step / len2.sqrt();
    }
    v
}

/// The wake the wall gives a body it touches (ticks).
pub const WAKE_TICKS: i32 = 32;

/// The impact sound.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallSound {
    /// How far in front of the wall the struck sphere was.
    pub depth: f32,
    pub volume: f32,
    pub pitch: f32,
}

/// What a frame of the wall shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallFrameOut {
    /// 1 rising, 2 falling, 0 standing.
    pub mode: i32,
    pub at: Vec3,
    pub half_width: f32,
    pub half_height: f32,
    /// How far the mesh's halves slide toward the middle (cm).
    pub slide: f32,
    pub texture: f32,
    /// The hum's gain and pitch, when the frame sets them.
    pub hum: Option<(f32, f32)>,
    pub ripples: [Vec3; MOST_RIPPLES],
}

impl Default for WallFrameOut {
    fn default() -> Self {
        Self {
            mode: 0,
            at: Vec3::ZERO,
            half_width: 0.0,
            half_height: 0.0,
            slide: 0.0,
            texture: 0.0,
            hum: None,
            ripples: [Vec3::ZERO; MOST_RIPPLES],
        }
    }
}

/// The barricade's mesh: positions, texture coordinates (the left half's
/// first), triangles, as the build leaves them.
#[derive(Debug, Clone, PartialEq)]
pub struct WallMesh {
    pub positions: Vec<Vec3>,
    pub uvs: Vec<Vec2>,
    pub triangles: Vec<[u16; 3]>,
}

/// Builds the barricade's mesh.
pub fn barricade_mesh() -> WallMesh {
    let t = THICKNESS;
    let mut p = vec![Vec3::ZERO; 32];
    let mut uv = vec![Vec2::ZERO; 32];
    let corners = [
        (t - REACH, t - HALF_HEIGHT),
        (t - REACH, HALF_HEIGHT - t),
        (REACH - t, HALF_HEIGHT - t),
        (REACH - t, t - HALF_HEIGHT),
    ];
    let corner_uv = [(-0.9, 0.1), (-0.9, 0.9), (0.9, 0.9), (0.9, 0.1)];
    for k in 0..4 {
        p[k] = Vec3::new(corners[k].0, corners[k].1, -t);
        p[k + 4] = Vec3::new(corners[k].0, corners[k].1, t);
        uv[k] = Vec2::new(corner_uv[k].0, corner_uv[k].1);
        uv[k + 4] = uv[k];
    }
    let mut tris: Vec<[u16; 3]> = vec![[0, 1, 2], [0, 2, 3], [5, 4, 6], [6, 4, 7]];
    let mut next: u16 = 8;
    let mut before: u16 = 3;
    for edge in 0..4u16 {
        let dir = normalize(p[edge as usize] - p[before as usize]);
        let across = Vec2::new(
            uv[before as usize].y - uv[edge as usize].y,
            uv[edge as usize].x - uv[before as usize].x,
        );
        let across = across * across.length_squared().sqrt().recip();
        let mut from = before;
        let mut to = edge;
        for k in 1..=4 {
            let mut turned = rotate_about(
                Vec3::new(0.0, 0.0, t),
                dir,
                k as f32 * -std::f32::consts::PI * 0.25,
            );
            turned.z = t - turned.z;
            let lift = if k < 3 { k as f32 } else { (4 - k) as f32 };
            if k == 4 {
                tris.push([to, from, before + 4]);
                tris.push([to, before + 4, edge + 4]);
            } else {
                let a = next;
                let b = next + 1;
                p[a as usize] = p[before as usize] + turned;
                uv[a as usize] = uv[before as usize] + across * (lift * 0.025);
                p[b as usize] = p[edge as usize] + turned;
                uv[b as usize] = uv[edge as usize] + across * (lift * 0.025);
                tris.push([to, from, a]);
                tris.push([to, a, b]);
                from = a;
                to = b;
                next += 2;
            }
        }
        before = edge;
    }
    let (mut a, mut b, mut c): (u16, u16, u16) = (0, 9, 14);
    for k in 0..4 {
        tris.push([a, b, c]);
        tris.push([b, b + 2, c]);
        tris.push([b + 2, c + 2, c]);
        tris.push([b + 2, b + 4, c + 2]);
        tris.push([b + 4, c + 4, c + 2]);
        tris.push([c + 4, b + 4, a + 4]);
        b += 6;
        c = if k == 2 { 8 } else { c + 6 };
        a += 1;
    }
    let half = 16u16;
    let mut map = [0u16; 32];
    let (mut left, mut right) = (0u16, half);
    for (k, v) in p.iter().enumerate() {
        if 0.0 <= v.x {
            map[k] = right;
            right += 1;
        } else {
            map[k] = left;
            left += 1;
        }
    }
    let mut positions = vec![Vec3::ZERO; 32];
    let mut uvs = vec![Vec2::ZERO; 32];
    for k in 0..32 {
        positions[map[k] as usize] = p[k];
        uvs[map[k] as usize] = uv[k];
    }
    for k in 0..32 {
        let toward = if k < usize::from(half) { 1.0 } else { -1.0 };
        positions[k].x += 270.0 * toward;
        uvs[k].x += 0.81 * toward;
    }
    let triangles = tris
        .iter()
        .map(|tri| tri.map(|v| map[v as usize]))
        .collect();
    WallMesh {
        positions,
        uvs,
        triangles,
    }
}

/// The mesh's halves slid apart to a frame's half width: the left half
/// right by `300 − half width`, the right half left by as much, and their
/// texture by a three-hundredth of that.
pub fn slide_mesh(mesh: &WallMesh, slide: f32) -> (Vec<Vec3>, Vec<Vec2>) {
    let mut positions = mesh.positions.clone();
    let mut uvs = mesh.uvs.clone();
    let k_half = positions.len() / 2;
    let along = slide * 0.003_333_333_4;
    for (k, (p, uv)) in positions.iter_mut().zip(uvs.iter_mut()).enumerate() {
        let toward = if k < k_half { 1.0 } else { -1.0 };
        p.x -= 270.0 * toward;
        uv.x -= 0.81 * toward;
        p.x += slide * toward;
        uv.x += along * toward;
    }
    (positions, uvs)
}

/// The bounds a wall's node keeps: 30 by 100 by 5, its radius their length
/// less 10.
pub fn node_radius() -> f32 {
    let h = NODE_HALF;
    dist_sq(h, Vec3::ZERO).sqrt() - 10.0
}

#[cfg(test)]
mod tests {
    use super::super::engine::testing::Fixed;
    use super::*;

    fn wall() -> Barricade {
        let frame = WallFrame::facing(Vec3::new(0.0, 100.0, 0.0), Vec3::X);
        let mut draws = Fixed::new(&[0.25, 0.75], &[0]);
        Barricade::raise(frame, 0.25, 1000, |_, _, _| None, None, &mut draws)
    }

    #[test]
    fn the_frame_faces_away() {
        let f = WallFrame::facing(Vec3::ZERO, Vec3::X);
        assert!((f.y - Vec3::Y).length() < 1e-3);
        assert!((f.x - Vec3::NEG_Z).length() < 1e-3);
        let p = Vec3::new(3.0, 4.0, -5.0);
        assert!((f.world(f.local(p)) - p).length() < 1e-2);
    }

    #[test]
    fn the_level_trims_each_reach() {
        let frame = WallFrame {
            at: Vec3::ZERO,
            x: Vec3::X,
            y: Vec3::Y,
            z: Vec3::Z,
        };
        let mut draws = Fixed::new(&[0.5], &[0]);
        let wall = Barricade::raise(
            frame,
            0.25,
            0,
            |_, dir, len| {
                assert_eq!(len, 260.0);
                (dir.x > 0.0).then_some(100.0)
            },
            None,
            &mut draws,
        );
        assert_eq!(wall.reach, [140.0, 300.0]);
        assert_eq!(wall.state, state::RISING);
        assert_eq!(wall.ripple_reach, 30.0);
    }

    #[test]
    fn the_wall_rises_over_a_second_and_six_tenths() {
        let mut w = wall();
        assert!(!w.tick(1000 + 800));
        assert!((w.shown[1] - 0.5).abs() < 1e-6);
        // e = 0.75, then 0.75·1.25·0.9 + 0.1 = 0.94375: 283.125 each way.
        assert!((w.half_width - (283.125 - 15.0)).abs() < 1e-3);
        assert_eq!(w.half_height, 90.0);
        assert!(!w.tick(1000 + 1601));
        assert_eq!(w.state, state::STANDING);
        assert!((w.half_width - 285.0).abs() < 1e-3);
    }

    #[test]
    fn a_short_reach_moves_the_middle() {
        let mut w = wall();
        w.reach = [100.0, 300.0];
        w.tick(1000 + 1700);
        assert!((w.half_width - (200.0 - 15.0)).abs() < 1e-3);
        // Shifted half the difference toward +x.
        let shift = w.centre - w.base;
        assert!((shift - w.frame.x * -100.0).length() < 1e-2);
    }

    #[test]
    fn the_wall_falls_from_where_it_stands() {
        let mut w = wall();
        w.tick(1000 + 400);
        w.end();
        w.tick(2000);
        assert_eq!(w.state, state::FALLING);
        let started = w.start;
        assert_eq!(
            started,
            2000 - (w.shown[1] * 750.0).round_ties_even() as u32
        );
        assert!(!w.tick(started + 375));
        let t2 = 0.25f32;
        assert!((w.half_height - ((1.0 - t2 * t2 * 0.5) * 100.0 - 10.0)).abs() < 1e-3);
        assert!(w.tick(started + 751));
        assert_eq!(w.state, state::GONE);
        assert!(w.frame(0.5).is_none());
    }

    #[test]
    fn ripples_ease_and_fade() {
        let mut w = wall();
        w.ripple(Vec2::new(10.0, 0.0), 0.3);
        w.ripple(Vec2::new(20.0, 0.0), 0.5);
        assert_eq!(w.ripples.len(), 1);
        assert_eq!(w.ripples[0].hits, 2);
        assert_eq!(w.ripples[0].strength, 0.5);
        w.tick(1001);
        let r = w.ripples[0];
        assert_eq!(r.target, Vec2::new(15.0, 0.0));
        assert!((r.strength - 0.47).abs() < 1e-6);
        assert!((r.shown - 0.04).abs() < 1e-6);
        assert!((r.at.x - 10.25).abs() < 1e-5);
        for _ in 0..40 {
            w.tick(1001);
        }
        assert!(w.ripples.is_empty());
        for k in 0..10 {
            w.ripple(Vec2::new(k as f32 * 100.0, 0.0), 1.0);
        }
        assert_eq!(w.ripples.len(), MOST_RIPPLES);
    }

    #[test]
    fn a_limb_in_front_is_pushed_and_knocked_back() {
        let mut w = wall();
        w.tick(1000 + 1700);
        let n = w.frame.z;
        let body = WallBody {
            centre: w.frame.at + n * 5.0,
            radius: 10.0,
            mobility: 1.0,
            step: 80.0,
            spheres: vec![(w.frame.at + n * 5.0, 10.0)],
            kind: WallBodyKind::Limb {
                chest: w.frame.at + n * 20.0,
                scale: 1.0,
            },
        };
        let mut roll = Roll(0);
        let shove = w.shove(&body, 5000, &mut roll);
        assert!(shove.touched);
        let (k, push) = shove.pushes[0];
        assert_eq!(k, 0);
        // ((10 + 10) − 5)·0.04 + 0.1 = 0.7, held to 0.5; half of it for a
        // mass of 1.
        assert!((push - n * 0.25).length() < 1e-5);
        let step = shove.knockback.expect("a knockback");
        assert!((step - Vec3::new(n.x, 0.0, n.z).normalize() * 80.0).length() < 1e-3);
        assert_eq!(knock_offset(Vec3::X, 40.0, 50.0), Vec3::new(50.0, 0.0, 0.0));
        assert_eq!(shove.instability, Some(100));
        assert!((shove.energy_loss - (10.0 * 0.0002 + 0.002)).abs() < 1e-7);
        assert!(shove.sound.is_some() && shove.alarm);
        assert_eq!(w.sound_at, 5400);
        assert_eq!(w.ripples.len(), 1);
        assert!((w.ripples[0].strength - 1.5).abs() < 1e-6);
        // Within 400 ms no second sound.
        assert!(w.shove(&body, 5300, &mut roll).sound.is_none());
    }

    #[test]
    fn a_body_behind_gets_a_tenth() {
        let mut w = wall();
        w.tick(1000 + 1700);
        let n = w.frame.z;
        let body = WallBody {
            centre: w.frame.at - n * 5.0,
            radius: 10.0,
            mobility: 1.0,
            step: 80.0,
            spheres: vec![(w.frame.at - n * 5.0, 10.0)],
            kind: WallBodyKind::Weapon,
        };
        let mut roll = Roll(0);
        let shove = w.shove(&body, 5000, &mut roll);
        assert!((shove.pushes[0].1 - n * 0.025).length() < 1e-6);
        assert!(!shove.interrupt, "only in front");
        let far = WallBody {
            centre: w.frame.at + n * 50.0,
            ..body
        };
        assert!(!w.shove(&far, 5000, &mut roll).touched);
    }

    #[test]
    fn the_mesh_has_two_halves() {
        let mesh = barricade_mesh();
        assert_eq!(mesh.positions.len(), 32);
        assert_eq!(mesh.triangles.len(), 60);
        assert!(mesh.positions[..16].iter().all(|p| p.x >= -30.0 - 1e-3));
        assert!(mesh.positions[16..].iter().all(|p| p.x <= 30.0 + 1e-3));
        let (full, _) = slide_mesh(&mesh, 0.0);
        let left = full.iter().map(|p| p.x).fold(f32::MAX, f32::min);
        let right = full.iter().map(|p| p.x).fold(f32::MIN, f32::max);
        assert!((left + 300.0).abs() < 1e-3 && (right - 300.0).abs() < 1e-3);
        let top = full.iter().map(|p| p.y).fold(f32::MIN, f32::max);
        assert!((top - 100.0).abs() < 1e-3);
        assert!(mesh.triangles.iter().flatten().all(|&v| v < 32));
    }

    #[test]
    fn the_stamp_spans_the_line() {
        let grid = GridFrame {
            width: 100,
            height: 100,
            scale: 0.04,
        };
        let s = nav_stamp(
            &grid,
            Vec3::new(-100.0, 0.0, 0.0),
            Vec3::new(100.0, 0.0, 0.0),
        );
        // Cells 50 + round(−4.5) = 46 and 50 + round(3.5) = 54.
        assert_eq!(s.size, (8 + 7, 7));
        // Row 50 + round(−0.5) = 50 (ties to even).
        assert_eq!(s.centre, (50, 50));
        assert_eq!(s.points.len(), 9);
        assert_eq!(s.points[8], Vec3::new(100.0, 0.0, 0.0));
    }

    #[test]
    fn the_hum_follows_the_rise() {
        let mut w = wall();
        w.shown = [0.0, 0.1];
        let f = w.frame(1.0).expect("standing");
        let e = 0.4f32 * (2.0 - 0.4);
        let (gain, pitch) = f.hum.expect("the rise's first quarter sets the hum");
        assert!((gain - e).abs() < 1e-6);
        assert!((pitch - (0.75 - e * 0.25)).abs() < 1e-6);
        assert_eq!(f.mode, 1);
        assert!(node_radius() > 90.0);
    }
}
