//! The force torrent: a sustained stream of force from the caster's hand,
//! sixteen streams of sixteen points that are fed from the hand, drift,
//! draw together and split, as the original's torrent simulation runs
//! them.
//!
//! - **Start** ([`Torrent::new`]): each stream is born at a blast chunk
//!   direction ([`chunk_directions`]) times 16 cm, its height 1.5 times less
//!   30 cm, in the hand's frame, with two random draws (its colour) and a
//!   spawn delay of 8; up to 24 streams. A point lives `round(83·0.8)` = 66
//!   ticks. The start then places each stream's first two points 18 and
//!   9 cm along the aim (widths 0.8 and 0.5, age 12), its head at slot 1,
//!   and a delay of `8 − roll(3)` ticks (the engine's own generator).
//! - **Aim**: the blast's ([`blast_aim`]); the frame turns with the
//!   player's turned aim (the player behaviour's turn spring, outside this
//!   group, which the caller gives).
//! - **Sounds** ([`step_torrent`]): the charge loop's volume follows
//!   `0.4·(2 − c)·c` while charging and then falls 0.01 a tick, stopping
//!   under 0.02; the stream loop's volume and pitch ease 6 % toward 0.5
//!   and 0.6 while it runs and keep 0.96 while it ends.
//! - **Charging** (state 0): each stream's source eases a tenth toward its
//!   birth place, its head point takes the birth place plus 15 cm along the
//!   aim and the point before it 30 cm; the caster pays the drain (running
//!   out fizzles), and past a charge of 1 the torrent runs (flags
//!   `0xb_0000`), each head and the point before it 9 cm out from where
//!   they were, and "PowerUsed" is raised. A fizzle (state −1) loses twice
//!   the rate and ends below 0.
//! - **Running** (state 1, paying half the drain; state 2 ends it): each
//!   stream's source takes its birth place. While running, when its delay
//!   runs out a new head is spawned 9 cm out along a direction drawn round
//!   the aim (`(0.3·sin a, 0.3·cos a, 1)` normalised, eased half way toward
//!   the stream's last one and turned into the frame), its width carried on
//!   from the old head over the distance to the source. Every live point,
//!   from the head back: ages a tick, pulls its neighbour toward it by
//!   `0.1·(1/2 − 2500/(d² + 2500))`, moves on by 0.999 of its last move and
//!   keeps the least cosine of that move with its direction. A point more
//!   than 1000 m (1e10 cm²) from its neighbour splits the stream there into
//!   a new one. The source's width follows the head's less `0.004·d`.
//!   Dead streams other than the first are removed: the original moves the
//!   last-but-one record into the hole, so the last stream is lost and the
//!   one before it doubled.
//! - **Bounds**: every live point's box, its middle, its half extents plus
//!   40 cm and the margin, and their length.
//! - **Frame** ([`Torrent::frame`]): the texture offset `(0, scroll, 1)`
//!   with the scroll between its last two ticks; each stream's samples (its
//!   source and its live points between their last two places, with their
//!   width, life curve `(2 − s)·s` and age) and colour bytes; the GI
//!   light's points ([`LightPoints::merge`]).
//! - **Hits** ([`torrent_reach`], [`body_reach`], [`torrent_hit`],
//!   [`push_torrent_point`], [`limb_terms`], [`limb_impulse`],
//!   [`torrent_knock`], [`surface_push`]): the contact arithmetic; the
//!   engine's mesh query is the caller's surface callback.
//! - **Ribbon** ([`Torrent::ribbon`]): each stream's samples doubled with
//!   midpoints, eye-facing normals kept between frames, and quads widened
//!   by the sample's age, walking the second texture coordinate by the
//!   squared sample distance.
//! - **Life** ([`Torrent::link`], [`Torrent::destroy`]): the geometry link
//!   and the destroy slot's two loops and GI light.
//!
//! Evidence: the torrent's start, chunk setup, aim, tick, contact entries
//! and callbacks, frame and light merge, read from `Exanima.exe`
//! (instructions for each compare, sum order and precision).

// NaN-sensitive compares are kept as the original makes them.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use bevy::prelude::*;

pub use super::blast::{CharacterKnock, character_knock};
use super::blast::{EPSILON, Frame, blast_aim, chunk_directions};
use super::ribbon::RibbonMesh;
use crate::arena::Mt19937;
use crate::thaum::casting::{CastBody, spend_tick};
use crate::thaum::engine::{Roll, dist_sq, lerp, normalize, sin_cos};
pub use crate::thaum::{TorrentMix, push_torrent_point, step_torrent, torrent_hit, torrent_reach};
use crate::thaum::{charge_rate, torrent_birth, torrent_streams};

/// A stream's points.
pub const POINTS: usize = 16;
/// The streams a torrent starts with, and the most it keeps.
pub const STREAMS: usize = 16;
pub const MOST_STREAMS: usize = STREAMS + 8;
/// The charging head's and the point before's reach along the aim, cm.
pub const CHARGE_HEAD: f32 = 15.0;
pub const CHARGE_TAIL: f32 = 30.0;
/// The spawn's and the release's step out, cm.
pub const STEP_OUT: f32 = 9.0;
/// A new point's age.
pub const NEW_AGE: f32 = 12.0;
/// The pull's scale and its distance term, and the move kept.
pub const PULL: f64 = -0.1;
pub const PULL_SPAN: f32 = 2500.0;
pub const KEEP_MOVE: f32 = f32::from_bits(0x3f7f_be77);
/// The squared gap a stream splits past.
pub const SPLIT_GAP: f32 = 1e10;
/// The source width's fall over the head's distance.
pub const WIDTH_FALL: f64 = 0.004;
/// The flags a running torrent sets.
pub const RUNNING_FLAGS: u32 = 0xb_0000;
/// The cast motion the start asks for.
pub const TORRENT_MOTION: u32 = 0x26;
/// The stream loop's starting pitch.
pub const LOOP_PITCH: f32 = 6.0;
/// The scroll a tick.
pub const SCROLL: f64 = 0.0015;
/// The bounds' margin, cm.
pub const BOUNDS_MARGIN: f32 = 40.0;
/// The light buckets' reach and merge distances (squared), and the most.
pub const LIGHT_REACH_SQ: f32 = 25_600.0;
pub const LIGHT_MERGE_SQ: f32 = 6_400.0;
pub const MOST_LIGHTS: usize = 16;
/// The ribbon's texture steps along a stream while it charges and while it
/// runs: the squared distance between two samples times one of these.
pub const RIBBON_STEP_CHARGING: f64 = 0.005;
pub const RIBBON_STEP_RUNNING: f64 = 0.0003;
/// The extra reach a torrent point tests a rigid body with, cm.
pub const BODY_EXTRA: f32 = 40.0;

/// The reach a torrent point tests a rigid body with: the width times the
/// scale, plus the margin, plus forty.
pub fn body_reach(width: f32, scale: f32, margin: f32) -> f32 {
    width * scale + margin + BODY_EXTRA
}

/// The static surface's contact scale, push and the ease a point takes
/// toward the contact.
pub const STATIC_SCALE: f32 = 0.25;
pub const STATIC_PUSH: f32 = 0.5;
pub const CONTACT_EASE: f32 = 0.5;

/// What a static surface's contact tells the walk.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StaticContact {
    pub scale: f32,
    pub push: f32,
    pub ease: f32,
}

/// The static surface's contact values.
pub const STATIC_CONTACT: StaticContact = StaticContact {
    scale: STATIC_SCALE,
    push: STATIC_PUSH,
    ease: CONTACT_EASE,
};

/// The normal-averaging weight and the normal lerps the ribbon keeps
/// between frames.
pub const NORMAL_AVERAGE: f32 = 0.1;
pub const NORMAL_LERP: f32 = 0.03;
/// The middle vertices' offset along the neighbours and the segment.
pub const MID_NEIGHBOURS: f32 = 0.25;
pub const MID_SEGMENT: f32 = -0.0625;

/// The torrent's states.
pub const FIZZLED: i32 = -2;
pub const FIZZLING: i32 = -1;
pub const CHARGING: i32 = 0;
pub const RUNNING: i32 = 1;
pub const ENDING: i32 = 2;

/// What linking a torrent to its geometry installs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TorrentLink {
    /// The bits the geometry node's flags gain.
    pub flags: u32,
    /// The node runs the torrent's stream draw.
    pub draws_streams: bool,
}

/// What the torrent's destroy slot releases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TorrentTeardown {
    /// The charge loop stops.
    pub charge_sound: bool,
    /// The stream loop stops.
    pub loop_sound: bool,
    /// The GI light is taken out of its manager and freed.
    pub light: bool,
    /// The object's own memory is freed.
    pub object: bool,
}

/// One point of a stream.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct StreamPoint {
    pub at: Vec3,
    pub previous: Vec3,
    pub age: f32,
    pub life: i32,
    pub width: f32,
    pub direction: Vec3,
    /// The least cosine of its move with its direction.
    pub straightness: f32,
}

/// One stream record.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Stream {
    pub head: usize,
    pub points: [StreamPoint; POINTS],
    /// Its birth place in the hand's frame.
    pub birth: Vec3,
    pub source_previous: Vec3,
    pub source: Vec3,
    /// The last spawn direction, before the frame.
    pub spread: Vec3,
    pub delay: i32,
    /// The source's width.
    pub base_width: f32,
    pub draws: [f32; 2],
    /// The ribbon's frames the draw keeps: the mean normal, the first, the
    /// last, and the one before it (the tick copies the last into the one
    /// before when a point dies).
    pub normals: [Vec3; 4],
    /// Split off another stream.
    pub split: bool,
}

/// What the world tells a tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TorrentView {
    /// The AI's aim point.
    pub target: Vec3,
    /// The hand's node and the hand part's place.
    pub hand_node: Vec3,
    pub hand: Vec3,
    /// The body's facing.
    pub facing: Vec3,
    /// The player behaviour's turned aim.
    pub turned_aim: Vec3,
}

/// The torrent's two sounds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TorrentSounds {
    pub charge_volume: f32,
    pub charge_playing: bool,
    pub loop_volume: f32,
    pub loop_pitch: f32,
}

/// What a tick asks of the world.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TorrentEvents {
    pub power_used: bool,
}

/// The torrent.
#[derive(Debug, Clone, PartialEq)]
pub struct Torrent {
    pub state: i32,
    pub drain: f32,
    pub charge: f32,
    pub rate: f32,
    pub point_life: i32,
    pub scroll: f32,
    pub previous_scroll: f32,
    pub aim: Vec3,
    pub frame: Frame,
    /// The stream records; `count` of them run, `charging` of them charge.
    pub records: Vec<Stream>,
    pub count: usize,
    pub charging: usize,
    pub flags: u32,
    pub margin: f32,
    pub sounds: TorrentSounds,
    pub lights: LightPoints,
    pub roll: Roll,
    pub centre: Vec3,
    pub half: Vec3,
    pub bound_radius: f32,
    /// The effect has linked to its geometry node.
    pub linked: bool,
}

impl Torrent {
    /// A torrent costing `cost` focus a second (drained on the start's tick
    /// length), its stream draws from `random` and its delays from `roll`.
    pub fn new(
        cost: f32,
        tick_ms: u32,
        view: &TorrentView,
        random: &mut Mt19937,
        roll: Roll,
    ) -> Self {
        let seconds = (f64::from(tick_ms) * 0.001) as f32;
        let aim = blast_aim(view.hand_node, view.target, view.facing);
        let frame = Frame::looking(view.turned_aim, view.hand);
        let point_life = torrent_streams(crate::effects::TICK_HZ as i32);
        let mut records = vec![Stream::default(); MOST_STREAMS];
        for (stream, direction) in records.iter_mut().zip(chunk_directions()) {
            stream.head = 0;
            stream.draws = [random.unit() as f32, random.unit() as f32];
            stream.delay = 8;
            stream.birth = Vec3::from_array(torrent_birth(direction.to_array()));
            let at = frame.place(stream.birth);
            stream.points[0].at = at;
            stream.points[0].previous = at;
            stream.source = at;
            stream.source_previous = at;
            stream.spread = Vec3::Z;
        }
        let mut torrent = Self {
            state: CHARGING,
            drain: cost * seconds,
            charge: 0.0,
            rate: charge_rate(tick_ms),
            point_life,
            scroll: 0.0,
            previous_scroll: 0.0,
            aim,
            frame,
            records,
            count: STREAMS,
            charging: STREAMS,
            flags: 0,
            margin: 0.0,
            sounds: TorrentSounds {
                charge_volume: 0.0,
                charge_playing: true,
                loop_volume: 0.0,
                loop_pitch: LOOP_PITCH,
            },
            lights: LightPoints::default(),
            roll,
            centre: Vec3::ZERO,
            half: Vec3::ZERO,
            bound_radius: 0.0,
            linked: false,
        };
        for stream in &mut torrent.records[..torrent.count] {
            stream.head = 1;
            let place = torrent.frame.place(stream.birth);
            let p1 = &mut stream.points[1];
            p1.previous = place;
            p1.at = place + torrent.aim * STEP_OUT;
            p1.life = point_life;
            p1.age = NEW_AGE;
            p1.width = 0.5;
            let p0 = &mut stream.points[0];
            p0.previous = place;
            p0.at = place + torrent.aim * (STEP_OUT * 2.0);
            p0.age = NEW_AGE;
            p0.width = 0.8;
            stream.source = stream.points[1].at;
            stream.delay = 8 - below3(&mut torrent.roll);
        }
        torrent
    }

    /// The cast's hook, when the caster lets go or runs out of focus: a
    /// charging torrent fizzles, any other ends.
    pub fn release(&mut self) {
        self.state = if self.state == CHARGING {
            FIZZLING
        } else {
            ENDING
        };
    }

    /// Links the torrent to its geometry node, as the blast's link does.
    pub fn link(&mut self) -> TorrentLink {
        self.linked = true;
        TorrentLink {
            flags: crate::effects::blast::GEOMETRY_LINK_FLAG,
            draws_streams: true,
        }
    }

    /// The destroy slot: both loops stop, the GI light leaves its manager
    /// and the object's memory goes.
    pub fn destroy(&mut self) -> TorrentTeardown {
        self.linked = false;
        TorrentTeardown {
            charge_sound: true,
            loop_sound: true,
            light: true,
            object: true,
        }
    }

    /// One tick. `caster` pays the drain; `random` draws the spawn angles.
    pub fn tick(
        &mut self,
        view: &TorrentView,
        caster: &mut CastBody,
        random: &mut Mt19937,
    ) -> TorrentEvents {
        self.tick_paying(view, random, |cost| {
            let short = spend_tick(caster, cost);
            if short {
                crate::thaum::casting::end_cast(caster);
            }
            short
        })
    }

    /// One tick, `spend` paying each drain and answering whether the
    /// shortfall called this torrent's hook.
    pub fn tick_paying(
        &mut self,
        view: &TorrentView,
        random: &mut Mt19937,
        mut spend: impl FnMut(f32) -> bool,
    ) -> TorrentEvents {
        let mut events = TorrentEvents::default();
        self.aim = blast_aim(view.hand_node, view.target, view.facing);
        self.frame = Frame::looking(view.turned_aim, view.hand);
        self.previous_scroll = self.scroll;
        self.scroll = (f64::from(self.scroll) + SCROLL) as f32;
        let mix = step_torrent(
            self.state,
            self.charge,
            self.sounds.charge_volume,
            self.sounds.loop_volume,
            self.sounds.loop_pitch,
        );
        if self.sounds.charge_playing {
            self.sounds.charge_volume = mix.level;
            self.sounds.charge_playing = mix.playing;
        }
        self.sounds.loop_volume = mix.left;
        self.sounds.loop_pitch = mix.right;
        if self.state < RUNNING {
            for stream in &mut self.records[..self.charging] {
                stream.source_previous = stream.source;
                let gap = self.frame.place(stream.birth) - stream.source;
                stream.source += gap * 0.1;
                let h = stream.head;
                let p = &mut stream.points[h];
                p.previous = p.at;
                p.at = self.frame.place(stream.birth) + self.aim * CHARGE_HEAD;
                let p = &mut stream.points[h - 1];
                p.previous = p.at;
                p.at = self.frame.place(stream.birth) + self.aim * CHARGE_TAIL;
            }
            if self.state < CHARGING {
                self.charge -= self.rate * 2.0;
                if self.charge < 0.0 {
                    self.charge = 0.0;
                    self.state = FIZZLED;
                }
            } else {
                if spend(self.drain) {
                    self.release();
                }
                self.charge += self.rate;
                if 1.0 < self.charge {
                    self.charge = 1.0;
                    self.state = RUNNING;
                    self.flags |= RUNNING_FLAGS;
                    for stream in &mut self.records[..self.charging] {
                        let h = stream.head;
                        for k in [h, h - 1] {
                            let p = &mut stream.points[k];
                            p.at = p.previous + self.aim * STEP_OUT;
                        }
                    }
                    events.power_used = true;
                }
            }
        } else {
            if self.state == RUNNING && spend(self.drain * 0.5) {
                self.release();
            }
            let streams = self.count;
            for index in 0..streams {
                self.run_stream(index, random);
            }
            let mut i = 1;
            while i < self.count {
                let stream = &self.records[i];
                if stream.points[stream.head].life < 1 {
                    self.count -= 1;
                    // The original copies the record before the last.
                    self.records[i] = self.records[self.count - 1];
                } else {
                    i += 1;
                }
            }
        }
        self.update_bounds();
        events
    }

    fn run_stream(&mut self, index: usize, random: &mut Mt19937) {
        let frame = self.frame;
        let point_life = self.point_life;
        let running = self.state == RUNNING;
        let stream = &mut self.records[index];
        stream.source_previous = stream.source;
        stream.source = frame.place(stream.birth);
        if running {
            stream.delay -= 1;
            if stream.delay < 1 {
                let next = if stream.head + 1 > POINTS - 1 {
                    0
                } else {
                    stream.head + 1
                };
                let angle = random.unit() as f32 * std::f32::consts::TAU;
                let (sin, cos) = sin_cos(angle);
                let mut v = Vec3::new(
                    (f64::from(sin) * 0.3) as f32,
                    (f64::from(cos) * 0.3) as f32,
                    1.0,
                );
                v = normalize(v);
                v = lerp(v, stream.spread, 0.5);
                stream.spread = v;
                let turned = frame.rotate(v);
                let place = frame.place(stream.birth);
                let head_width = stream.points[stream.head].width;
                let d2 = dist_sq(stream.source, stream.points[stream.head].at);
                let p = &mut stream.points[next];
                p.direction = turned;
                p.straightness = 1.0;
                p.previous = place;
                p.at = place + turned * STEP_OUT;
                p.life = point_life;
                p.age = NEW_AGE;
                p.width =
                    (head_width - stream.base_width) * (STEP_OUT / d2.sqrt()) + stream.base_width;
                stream.head = next;
                stream.delay = 8 - below3(&mut self.roll);
            }
        }
        let stream = &mut self.records[index];
        let mut prev = stream.head;
        let mut k = stream.head;
        let mut split_at = None;
        loop {
            stream.points[k].life -= 1;
            if stream.points[k].life == 0 {
                stream.normals[2] = stream.normals[3];
            }
            stream.points[k].age += 1.0;
            if k != stream.head {
                let d = stream.points[k].at - stream.points[prev].at;
                let d2 = (d.x * d.x + d.y * d.y) + d.z * d.z;
                let f = (f64::from(PULL_SPAN / (d2 + PULL_SPAN) - 0.5) * PULL) as f32;
                let d = d * f;
                stream.points[prev].at += d;
                stream.points[k].at -= d;
                stream.points[prev].previous += d;
                stream.points[k].previous -= d;
            }
            let p = &mut stream.points[k];
            let step = p.at - p.previous;
            p.previous = p.at;
            let cosine = cosine_between(step, p.direction);
            if cosine < p.straightness {
                p.straightness = cosine;
            }
            p.at += step * KEEP_MOVE;
            if stream.points[k].life < 1 {
                break;
            }
            if self.count < MOST_STREAMS
                && k != stream.head
                && !stream.split
                && SPLIT_GAP < dist_sq(stream.points[k].at, stream.points[prev].at)
            {
                split_at = Some((k, prev));
                break;
            }
            prev = k;
            k = if k == 0 { POINTS - 1 } else { k - 1 };
        }
        if let Some((k, prev)) = split_at {
            let parent = self.records[index];
            let new_index = self.count;
            let mut slot = POINTS - 1;
            let mut from = prev;
            loop {
                self.records[new_index].points[slot] = parent.points[from];
                slot = if slot == 0 { POINTS - 1 } else { slot - 1 };
                if parent.points[from].life < 1 {
                    break;
                }
                from = if from == 0 { POINTS - 1 } else { from - 1 };
            }
            let new = &mut self.records[new_index];
            new.points[slot].life = 0;
            new.head = POINTS - 1;
            new.delay = i32::MAX;
            new.split = true;
            new.draws = parent.draws;
            self.count += 1;
            self.records[index].points[k].life = 0;
        }
        let stream = &mut self.records[index];
        let d2 = dist_sq(stream.source, stream.points[stream.head].at);
        if EPSILON < d2 {
            stream.base_width = (f64::from(stream.points[stream.head].width)
                - f64::from(d2.sqrt()) * WIDTH_FALL) as f32;
        }
    }

    fn update_bounds(&mut self) {
        let first = &self.records[0];
        let mut low = first.points[first.head].at;
        let mut high = low;
        for stream in &self.records[..self.count] {
            let mut k = stream.head;
            loop {
                let v = stream.points[k].at;
                for axis in 0..3 {
                    if v[axis] < low[axis] {
                        low[axis] = v[axis];
                    } else if high[axis] < v[axis] {
                        high[axis] = v[axis];
                    }
                }
                k = if k == 0 { POINTS - 1 } else { k - 1 };
                if stream.points[k].life < 1 {
                    break;
                }
            }
        }
        self.centre = lerp(low, high, 0.5);
        self.half = high - self.centre;
        self.half += Vec3::splat(BOUNDS_MARGIN);
        self.half += Vec3::splat(self.margin);
        let h = self.half;
        let r2 = (h.x * h.x + h.y * h.y) + h.z * h.z;
        self.bound_radius = if 0.0 < r2 { r2.sqrt() } else { r2 };
    }
}

/// `8 − roll(3)`'s draw: the high half of the state times 3.
fn below3(roll: &mut Roll) -> i32 {
    ((u64::from(roll.step()) * 3) >> 32) as i32
}

/// The cosine between two moves, the engine's
/// ([`exa_physics::engine_math::cosine`]): 0 when the product of their
/// squared lengths is at most 1e-8.
pub fn cosine_between(a: Vec3, b: Vec3) -> f32 {
    exa_physics::engine_math::cosine(a, b)
}

/// One sample the draw takes of a stream.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StreamSample {
    pub at: Vec3,
    pub width: f32,
    /// The life curve `(2 − s)·s` (1 for the source).
    pub weight: f32,
    pub age: f32,
}

/// The torrent's GI light points: places, their weights and how many
/// points each gathered.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LightPoints {
    pub points: Vec<(Vec3, f32, i32)>,
}

impl LightPoints {
    /// Gathers a sample of weight `w` (as `(2 − w)·w`): into the nearest
    /// point within 160 cm when 16 are kept or it is within 80 cm, else as a
    /// new point.
    pub fn merge(&mut self, at: Vec3, w: f32) {
        let w = (2.0 - w) * w;
        if self.points.is_empty() {
            self.points.push((at, w, 1));
            return;
        }
        let mut best = LIGHT_REACH_SQ;
        let mut nearest = None;
        for (i, point) in self.points.iter().enumerate() {
            let d2 = dist_sq(point.0, at);
            if d2 < best {
                best = d2;
                nearest = Some(i);
            }
        }
        if self.points.len() > MOST_LIGHTS - 1 || best <= LIGHT_MERGE_SQ {
            if let Some(i) = nearest {
                let point = &mut self.points[i];
                point.2 += 1;
                point.0 = lerp(point.0, at, 1.0 / point.2 as f32);
                point.1 += w;
            }
        } else {
            self.points.push((at, w, 1));
        }
    }
}

/// A frame's values outside the draw.
#[derive(Debug, Clone, PartialEq)]
pub struct TorrentFrame {
    pub texture_offset: Vec3,
    /// Each stream's colour bytes and samples.
    pub streams: Vec<([u8; 4], Vec<StreamSample>)>,
}

impl Torrent {
    /// Whether a frame would build no triangles, when the original's frame
    /// takes the torrent's light away and removes it from the scene: it has
    /// fizzled, or no stream has more than two samples (each segment of a
    /// stream's ribbon adds two, less the first segment's two).
    pub fn builds_nothing(&self) -> bool {
        if self.state == FIZZLED {
            return true;
        }
        !self.records[..self.count].iter().any(|stream| {
            let mut samples = usize::from(!stream.split && self.state != ENDING);
            let mut k = stream.head;
            loop {
                samples += 1;
                if stream.points[k].life < 1 {
                    break;
                }
                k = if k == 0 { POINTS - 1 } else { k - 1 };
                if k == stream.head {
                    break;
                }
            }
            2 < samples
        })
    }

    /// The frame `t` of the way between the last two ticks; it also
    /// gathers the light points.
    pub fn frame(&mut self, t: f32) -> TorrentFrame {
        let scroll = (1.0 - t) * self.previous_scroll + self.scroll * t;
        for point in &mut self.lights.points {
            point.1 = 0.0;
            point.2 = 1;
        }
        let byte = |v: f32| (v * 255.0).round_ties_even() as i32 as u8;
        let level = self.charge;
        let mut streams = Vec::new();
        if self.state != FIZZLED {
            for stream in &self.records[..self.count] {
                let colour = [
                    byte(stream.draws[0]),
                    byte(stream.draws[1]),
                    0xff,
                    byte((2.0 - level) * level),
                ];
                let mut samples = Vec::new();
                if !stream.split && self.state != ENDING {
                    samples.push(StreamSample {
                        at: lerp(stream.source_previous, stream.source, t),
                        width: stream.base_width,
                        weight: 1.0,
                        age: NEW_AGE,
                    });
                }
                let mut k = stream.head;
                loop {
                    let p = &stream.points[k];
                    let s = p.life as f32 / self.point_life as f32;
                    samples.push(StreamSample {
                        at: lerp(p.previous, p.at, t),
                        width: p.width,
                        weight: (2.0 - s) * s,
                        age: p.age,
                    });
                    if p.life < 1 {
                        break;
                    }
                    k = if k == 0 { POINTS - 1 } else { k - 1 };
                    if k == stream.head {
                        break;
                    }
                }
                if samples.len() > 2 {
                    if stream.split {
                        self.lights.merge(samples[0].at, samples[0].weight);
                    }
                    for sample in &samples[1..samples.len() - 1] {
                        self.lights.merge(sample.at, sample.weight);
                    }
                }
                streams.push((colour, samples));
            }
        }
        let mut i = 0;
        while i < self.lights.points.len() {
            if 1.0 <= self.lights.points[i].1 {
                i += 1;
            } else {
                self.lights.points.swap_remove(i);
            }
        }
        TorrentFrame {
            texture_offset: Vec3::new(0.0, scroll, 1.0),
            streams,
        }
    }

    /// The torrent's ribbon: one strip a stream, its samples doubled by
    /// the midpoints between them — each midpoint the two neighbours'
    /// middle, pulled a quarter of the way by the samples around it and a
    /// sixteenth back along its own segment — and every pair widened by a
    /// normal across the eye and the sample's age. The normals come from
    /// the neighbour differences crossed with the eye: the first and last
    /// averaged, the middles interpolated between the two ends' normals,
    /// each kept between draws by [`NORMAL_LERP`]. The second texture
    /// coordinate walks along the strip by the squared sample distance
    /// times [`RIBBON_STEP_CHARGING`] while the torrent charges and
    /// [`RIBBON_STEP_RUNNING`] while it runs.
    ///
    /// The original crosses the differences with the eye position carried
    /// into the effect node's frame; Exarust uses the world eye, the same
    /// when the node is unturned.
    #[allow(clippy::needless_range_loop)]
    pub fn ribbon(&mut self, frame: &TorrentFrame, eye: Vec3) -> RibbonMesh {
        let mut mesh = RibbonMesh::default();
        let charging = self.state < RUNNING;
        let step = if charging {
            RIBBON_STEP_CHARGING
        } else {
            RIBBON_STEP_RUNNING
        };
        for (index, (colour, samples)) in frame.streams.iter().enumerate() {
            if samples.len() < 3 {
                continue;
            }
            let mut points = Vec::with_capacity((samples.len() - 1) * 2);
            let mut widths = Vec::with_capacity((samples.len() - 1) * 2);
            for i in 0..samples.len() - 1 {
                points.push(samples[i].at);
                let mut middle = lerp(samples[i].at, samples[i + 1].at, 0.5);
                if i > 0 {
                    middle += (samples[i + 1].at - samples[i - 1].at) * MID_NEIGHBOURS;
                }
                middle += (samples[i + 1].at - samples[i].at) * MID_SEGMENT;
                points.push(middle);
                let midpoint = |a: f32, b: f32| a + (b - a) * 0.5;
                widths.push(samples[i]);
                widths.push(StreamSample {
                    at: middle,
                    width: midpoint(samples[i].width, samples[i + 1].width),
                    weight: midpoint(samples[i].weight, samples[i + 1].weight),
                    age: midpoint(samples[i].age, samples[i + 1].age),
                });
            }
            let count = points.len();
            let mut normals = vec![Vec3::ZERO; count];
            let first = normalize(cross(points[1] - points[0], eye));
            normals[0] = first;
            let mut sum = first;
            for i in 1..count - 1 {
                let normal = normalize(cross(points[i + 1] - points[i - 1], eye));
                normals[i] = normal;
                sum += normal;
            }
            normals[count - 1] = first;
            let sum = normalize(sum);
            normals[0] = normalize(lerp(normals[0], sum, NORMAL_AVERAGE));
            normals[count - 1] = normals[0];
            let stream = &mut self.records[index];
            stream.normals[1] = normalize(lerp(
                stream.normals[1],
                normalize(cross(points[1] - points[0], eye)),
                NORMAL_LERP,
            ));
            stream.normals[2] = normalize(lerp(
                stream.normals[2],
                normalize(cross(points[count - 1] - points[count - 3], eye)),
                NORMAL_LERP,
            ));
            normals[0] = stream.normals[1];
            for i in 1..count - 1 {
                let t = i as f32 / (count - 2) as f32;
                normals[i] = normalize(lerp(stream.normals[1], stream.normals[2], t));
            }
            stream.normals[3] = normals[count - 2];
            let mut along = 0.0f32;
            for i in 0..count - 2 {
                mesh.push_torrent_quad(
                    points[i],
                    normals[i],
                    widths[i].age,
                    *colour,
                    widths[i].width,
                    widths[i].weight,
                    along,
                );
                let delta = points[i + 1] - points[i];
                let squared = (delta.x * delta.x + delta.y * delta.y) + delta.z * delta.z;
                along = (f64::from(along) + f64::from(squared) * step) as f32;
            }
        }
        mesh
    }
}

/// `(a × b)`, each term in the original's cross order. The original's
/// helper is the engine's shuffle form of the same product.
fn cross(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

/// The limb contact's scale, push and ease: a character's limb 0.5,
/// `0.8/mass` and `0.5/(inverse + 1)`; any other body's
/// `min(inverse/4 + 1/2, 3)`, 0.5 and the same ease.
pub fn limb_terms(character: bool, inverse_mass: f32, body_mass: f32) -> (f32, f32, f32) {
    let ease = 0.5 / (inverse_mass + 1.0);
    if character {
        (0.5, (1.0 / body_mass) * 0.8, ease)
    } else {
        ((inverse_mass * 0.25 + 0.5).min(3.0), 0.5, ease)
    }
}

/// The impulse a limb takes from a stream point: `k = inverse/(inverse +
/// 0.5)·push·0.3·(2 − s)·s` with `s` the point's life share; along its move
/// by `k·f·straightness` (`f = 1 − (v·move)/(move·move)` with the limb's
/// velocity `v` at the contacts' mean, clamped to 0..1, nothing at 0 or
/// below), plus `(contact − point)·(1 − straightness)·(−k)`.
pub fn limb_impulse(
    point: &StreamPoint,
    contact: Vec3,
    limb_velocity: Vec3,
    inverse_mass: f32,
    push: f32,
    life_share: f32,
) -> Vec3 {
    let motion = point.at - point.previous;
    let k = (inverse_mass / (inverse_mass + 0.5)) * push * 0.3 * (2.0 - life_share) * life_share;
    let along =
        limb_velocity.z * motion.z + limb_velocity.y * motion.y + limb_velocity.x * motion.x;
    let length = motion.z * motion.z + motion.y * motion.y + motion.x * motion.x;
    let mut f = 1.0 - along / length;
    let mut impulse = if f <= 0.0 {
        Vec3::ZERO
    } else {
        if 1.0 < f {
            f = 1.0;
        }
        motion * (k * f * point.straightness)
    };
    impulse += (contact - point.at) * ((1.0 - point.straightness) * -k);
    impulse
}

/// A character's energy loss on a torrent hit: `k/2000` with `k = 1/mass`,
/// only above 0.25.
pub fn energy_after(energy: f32, body_mass: f32) -> f32 {
    let k = 1.0 / body_mass;
    if 0.25 < energy {
        energy - k * 0.0005
    } else {
        energy
    }
}

/// A character struck by the torrents: `k = 1/mass`, its energy, the
/// gathered impulse (the same cap at thirty, doubled) and the stun
/// `(1 − k)·0.08` clamped below zero and above 0.08.
pub fn torrent_knock(body_mass: f32, energy: f32, impulse: Vec3) -> CharacterKnock {
    let k = 1.0 / body_mass;
    let energy = energy_after(energy, body_mass);
    let size2 = (impulse.x * impulse.x + impulse.y * impulse.y) + impulse.z * impulse.z;
    let knock = if 1.0 < size2 {
        let size = size2.sqrt();
        let direction = impulse * (1.0 / size);
        let size = if 30.0 < size { 30.0 } else { size };
        Some((direction, size * 2.0))
    } else {
        None
    };
    let mut stun = (f64::from(1.0 - k) * 0.08) as f32;
    if 0.0 > stun {
        stun = 0.0;
    } else if f64::from(stun) > 0.08 {
        stun = 0.008;
    }
    CharacterKnock {
        energy,
        knock,
        stun,
    }
}

/// The static-surface walk: each live point whose centre is within
/// `torrent_reach(width, scale, margin)` of the surface's nearest point
/// moves half way to it, and the loop sound's channels rise by the point's
/// life share: 0.005 on the first, and 0.01 on the second while it is under
/// 1.8. Returns how many points met the surface.
pub fn surface_push(
    torrent: &mut Torrent,
    nearest: impl Fn(Vec3) -> Option<Vec3>,
    scale: f32,
    sound: &mut crate::thaum::TorrentHit,
) -> usize {
    let margin = torrent.margin;
    let point_life = torrent.point_life;
    let mut hits = 0;
    for stream in &mut torrent.records[..torrent.count] {
        let head = stream.head;
        let mut k = head;
        loop {
            let point = &mut stream.points[k];
            if point.life < 1 {
                break;
            }
            let reach = torrent_reach(point.width, scale, margin);
            if let Some(contact) = nearest(point.at)
                && dist_sq(point.at, contact) <= reach * reach
            {
                point.at = lerp(point.at, contact, 0.5);
                *sound = torrent_hit(point.life, point_life, sound.left, sound.right);
                hits += 1;
            }
            k = if k == 0 { POINTS - 1 } else { k - 1 };
            if k == head {
                break;
            }
        }
    }
    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view() -> TorrentView {
        TorrentView {
            target: Vec3::new(0.0, 150.0, 1000.0),
            hand_node: Vec3::new(0.0, 150.0, 0.0),
            hand: Vec3::new(0.0, 150.0, 0.0),
            facing: Vec3::Z,
            turned_aim: Vec3::Z,
        }
    }

    fn torrent() -> Torrent {
        Torrent::new(0.2, 12, &view(), &mut Mt19937::new(1), Roll(7))
    }

    #[test]
    fn a_torrent_builds_nothing_once_fizzled_or_its_streams_are_spent() {
        let mut t = torrent();
        // The source, the head and the dead point before it: three samples.
        assert!(!t.builds_nothing());
        t.state = ENDING;
        assert!(t.builds_nothing());
        t.state = RUNNING;
        for stream in &mut t.records {
            stream.points[stream.head].life = 0;
        }
        assert!(t.builds_nothing());
        let mut t = torrent();
        t.state = FIZZLED;
        assert!(t.builds_nothing());
    }

    #[test]
    fn a_new_torrent_places_its_first_two_points() {
        let t = torrent();
        assert_eq!((t.count, t.charging, t.point_life), (16, 16, 66));
        let s = &t.records[0];
        // Stream 0 is born straight ahead: 16 cm along +z, then 1.5 times
        // less 30.
        assert_eq!(s.birth, Vec3::new(0.0, 0.0, -6.0));
        assert_eq!(s.head, 1);
        assert!((s.points[1].at.z - (-6.0 + 9.0)).abs() < 1e-3);
        assert!((s.points[0].at.z - (-6.0 + 18.0)).abs() < 1e-3);
        assert_eq!((s.points[1].width, s.points[0].width), (0.5, 0.8));
        assert_eq!(s.points[1].life, 66);
        assert_eq!(s.points[0].life, 0);
        assert!((6..=8).contains(&s.delay));
        assert!(t.records[16..].iter().all(|r| *r == Stream::default()));
    }

    #[test]
    fn charging_feeds_the_points_and_runs_past_one() {
        let mut t = Torrent::new(0.2, 1300, &view(), &mut Mt19937::new(1), Roll(7));
        let mut caster = CastBody::default();
        let events = t.tick(&view(), &mut caster, &mut Mt19937::new(2));
        assert!(events.power_used);
        assert_eq!((t.state, t.charge, t.flags), (RUNNING, 1.0, RUNNING_FLAGS));
        let s = &t.records[0];
        // The head was put 15 cm out, then 9 cm on from where it was.
        assert!((s.points[1].at.z - (-6.0 + 9.0 + 9.0)).abs() < 1e-3);
        assert!((t.sounds.charge_volume - 0.0).abs() < 1e-6);
        assert_eq!(caster.spent, t.drain);
    }

    #[test]
    fn a_running_stream_spawns_ages_and_pulls() {
        let mut t = torrent();
        t.state = RUNNING;
        t.charge = 1.0;
        for s in &mut t.records[..16] {
            s.delay = 1;
        }
        t.tick(&view(), &mut CastBody::default(), &mut Mt19937::new(3));
        let s = &t.records[0];
        assert_eq!(s.head, 2);
        let p = &s.points[2];
        assert_eq!(p.life, 65);
        assert_eq!(p.age, 13.0);
        // Half way between a unit draw round +z and +z, not renormalised.
        assert!(p.direction.z > 0.9 && p.direction.length() <= 1.0);
        // Point 1 aged and lost a tick of life.
        assert_eq!(s.points[1].life, 65);
        assert!((5..=8).contains(&s.delay));
    }

    #[test]
    fn a_dead_stream_is_replaced_by_the_one_before_the_last() {
        let mut t = torrent();
        t.state = RUNNING;
        t.charge = 1.0;
        for (i, s) in t.records[..16].iter_mut().enumerate() {
            s.draws = [i as f32, 0.0];
            s.delay = 100;
        }
        let h = t.records[5].head;
        t.records[5].points[h].life = 1;
        t.tick(&view(), &mut CastBody::default(), &mut Mt19937::new(3));
        assert_eq!(t.count, 15);
        assert_eq!(t.records[5].draws[0], 14.0);
        assert_eq!(t.records[14].draws[0], 14.0);
    }

    #[test]
    fn the_cosine_and_the_light_merge_carry_the_numbers() {
        assert_eq!(cosine_between(Vec3::X, Vec3::X * 2.0), 1.0);
        assert_eq!(cosine_between(Vec3::ZERO, Vec3::X), 0.0);
        let mut lights = LightPoints::default();
        lights.merge(Vec3::ZERO, 1.0);
        lights.merge(Vec3::new(50.0, 0.0, 0.0), 1.0);
        assert_eq!(lights.points.len(), 1);
        assert_eq!(lights.points[0], (Vec3::new(25.0, 0.0, 0.0), 2.0, 2));
        lights.merge(Vec3::new(200.0, 0.0, 0.0), 0.5);
        assert_eq!(lights.points.len(), 2);
        assert_eq!(lights.points[1].1, 0.75);
    }

    #[test]
    fn the_frame_samples_the_streams() {
        let mut t = torrent();
        let frame = t.frame(0.5);
        assert_eq!(frame.texture_offset, Vec3::new(0.0, 0.0, 1.0));
        assert_eq!(frame.streams.len(), 16);
        let (colour, samples) = &frame.streams[0];
        assert_eq!(colour[2], 0xff);
        // The source, point 1 and the dead point 0.
        assert_eq!(samples.len(), 3);
        assert_eq!(samples[0].weight, 1.0);
        assert_eq!(samples[2].weight, 0.0);
    }

    #[test]
    fn the_contact_rules_carry_the_numbers() {
        assert_eq!(
            limb_terms(true, 1.0, 80.0),
            (0.5, (1.0f32 / 80.0) * 0.8, 0.25)
        );
        assert_eq!(limb_terms(false, 12.0, 0.0), (3.0, 0.5, 0.5 / 13.0));
        let p = StreamPoint {
            at: Vec3::new(0.0, 0.0, 10.0),
            previous: Vec3::ZERO,
            straightness: 1.0,
            ..StreamPoint::default()
        };
        let i = limb_impulse(&p, Vec3::new(0.0, 0.0, 12.0), Vec3::ZERO, 1.0, 0.5, 0.5);
        let k = (1.0 / 1.5) * 0.5 * 0.3 * 1.5 * 0.5;
        assert!((i.z - 10.0 * k).abs() < 1e-5);
        assert!((energy_after(1.0, 80.0) - (1.0 - 0.0125 * 0.0005)).abs() < 1e-7);
        assert_eq!(energy_after(0.2, 80.0), 0.2);
        assert_eq!(torrent_reach(12.0, 0.25, 0.0), 13.0);
        // A rigid body's reach is forty past the same scale, not ten.
        assert_eq!(body_reach(12.0, 0.25, 0.0), 43.0);
        assert_eq!(
            STATIC_CONTACT,
            StaticContact {
                scale: 0.25,
                push: 0.5,
                ease: 0.5
            }
        );
    }

    #[test]
    fn a_stream_becomes_a_doubled_strip_of_quads() {
        let mut t = torrent();
        // The source sits 20 cm behind the head, so the first quad has a
        // length and the texture walk moves.
        for stream in &mut t.records[..16] {
            stream.source = stream.points[1].at + Vec3::new(0.0, 0.0, 20.0);
        }
        let frame = t.frame(1.0);
        let mesh = t.ribbon(&frame, Vec3::new(1000.0, 1000.0, 0.0));
        // Three samples make four vertices and two quads a stream.
        assert_eq!(mesh.positions.len(), 16 * 4);
        assert_eq!(mesh.indices.len(), 16 * 2 * 6);
        let (colour, _) = &frame.streams[0];
        assert_eq!(mesh.colours[0], *colour);
        assert_eq!(mesh.uvs[0][0], 0.0);
        assert_eq!(mesh.uvs[1][0], 1.0);
        // The first quad starts the texture walk at zero; the second has
        // moved on by the squared sample distance times 0.005 (charging).
        assert_eq!(mesh.uvs2[0][1], 0.0);
        assert!(mesh.uvs2[2][1] > 0.0);
        // The sample's age is the half width.
        assert_eq!(mesh.uvs[0][1], frame.streams[0].1[0].width);
        assert!(mesh.positions[0] != mesh.positions[1]);
    }

    #[test]
    fn the_surface_walk_moves_points_and_raises_the_loop() {
        let mut t = torrent();
        t.state = RUNNING;
        t.charge = 1.0;
        // Every point sits on a floor at z = 0 and the surface is 2 cm up.
        for stream in &mut t.records[..16] {
            for point in &mut stream.points {
                point.at = Vec3::new(0.0, 0.0, 0.0);
            }
        }
        let mut sound = crate::thaum::TorrentHit {
            left: 0.0,
            right: 0.0,
        };
        let hits = surface_push(&mut t, |_| Some(Vec3::new(0.0, 0.0, 2.0)), 0.25, &mut sound);
        // One live sample a stream: the head; the dead point stops the walk.
        assert_eq!(hits, 16);
        assert!((t.records[0].points[1].at.z - 1.0).abs() < 1e-6);
        assert!((sound.left - 16.0 * 0.005).abs() < 1e-6);
        assert!((sound.right - 16.0 * 0.01).abs() < 1e-6);
    }

    #[test]
    fn linking_and_destroying_a_torrent_releases_its_two_loops() {
        let mut t = torrent();
        let link = t.link();
        assert_eq!(link.flags, crate::effects::blast::GEOMETRY_LINK_FLAG);
        assert!(link.draws_streams && t.linked);
        assert_eq!(
            t.destroy(),
            TorrentTeardown {
                charge_sound: true,
                loop_sound: true,
                light: true,
                object: true,
            }
        );
        assert!(!t.linked);
    }
}
