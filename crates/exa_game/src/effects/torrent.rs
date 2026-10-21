//! The force torrent: a sustained stream of force from the caster's hand.
//!
//! - **Streams**: sixteen stream records are born on a sphere 16 cm out,
//!   their height 1.5 times the direction less 30 cm, each with two random
//!   tints and a spawn delay of 8 less a random 0 to 2 ticks.
//! - **Charge**: the level follows `0.4·(2 − charge)·charge`; charge rises
//!   by five sixths of the tick in seconds. Past 1 the torrent runs: every
//!   stream pushes 15 cm along the aim a tick, respawning after its delay.
//! - **Fade**: a torrent asked to end drops its level by 0.01 a tick and
//!   stops below 0.02; a fizzle falls at twice the charge rate.
//! - **Hits**: a point inside a stream's reach takes 1 % of its overshoot
//!   as a push; the stream's two channels rise by the hit's age share,
//!   0.005 and 0.01, the second stopping at 1.8.
//! - **Draw**: each stream's ribbon runs through 16 history points, 12
//!   quads of 2 vertices each.
//! - **Light**: tint (0.12, 0.06, 0), radius 120 cm, strength
//!   `caster light · level · 0.25 · (2 − charge) · charge · 1.2`, 2000 in
//!   the light's own strength uniform.
//!
//! Evidence: the torrent constructor, start, tick, hit, draw and light
//! routines, read from `Exanima.exe`.

use bevy::prelude::*;

use crate::effects::round_i32;

/// The stream records' history length.
pub const STREAM_POINTS: usize = 16;
/// The quads one stream draws.
pub const STREAM_QUADS: usize = 12;
/// The streams the constructor makes.
pub const STREAMS: usize = 16;
/// The spawn radius, cm.
pub const BIRTH_RADIUS: f32 = 16.0;
/// The height's scale and offset, cm.
pub const BIRTH_HEIGHT: f32 = 1.5;
pub const BIRTH_DROP: f32 = 30.0;
/// The charge's rise: five sixths of the tick in seconds.
pub const CHARGE_RATE: f64 = 5.0 / 6.0;
/// The stream record's life in ticks: 0.8 s.
pub const STREAM_LIFE_TICKS: i32 = 66;
/// The run's forward step, cm.
pub const STREAM_STEP: f32 = 15.0;
/// The fade's fall and its floor.
pub const LEVEL_FALL: f32 = 0.01;
pub const LEVEL_FLOOR: f32 = 0.02;
/// The charge's level curve.
pub const LEVEL_CURVE: f32 = 0.4;
/// The push's share of the overshoot.
pub const PUSH_SHARE: f32 = 0.01;
/// The hit channels' gains and the second's ceiling.
pub const CHANNEL_LEFT: f32 = 0.005;
pub const CHANNEL_RIGHT: f32 = 0.01;
pub const CHANNEL_RIGHT_MAX: f32 = 1.8;
/// The hit's reach terms, cm.
pub const REACH_ADD: f32 = 40.0;
/// The light's radius and tint.
pub const LIGHT_RADIUS: f32 = 120.0;
pub const LIGHT_TINT: [f32; 3] = [0.12, 0.06, 0.0];
pub const LIGHT_UNIFORM: f32 = 2000.0;
/// The light's charge curve terms.
pub const LIGHT_CURVE: f32 = 0.25;
pub const LIGHT_CHARGE: f32 = 1.2;
/// The cast motion the start asks for and the two loop sounds.
pub const TORRENT_MOTION: u32 = 0x26;
pub const CHARGE_SOUND: &str = "thmfrc_cu01";
pub const LOOP_SOUND: &str = "thmfrc_trntdr";
/// The event a release raises.
pub const POWER_USED: &str = "PowerUsed";
/// The draw's texture offset, as the geometry sets it.
pub const TEXTURE_OFFSET: [f32; 3] = [0.0, 0.0, 0.0];

/// The torrent's phase word.
pub const CHARGING: i32 = 0;
pub const RUNNING: i32 = 1;
pub const FADING: i32 = 2;
pub const FIZZLED: i32 = -1;
pub const FINISHED: i32 = -2;

/// One stream record's history.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StreamPoint {
    pub at: [f32; 3],
    /// The point's remaining life, in ticks.
    pub life: i32,
    /// The point's channel level.
    pub level: f32,
}

/// One force stream.
#[derive(Debug, Clone, PartialEq)]
pub struct Stream {
    /// The newest point.
    pub head: usize,
    pub points: [StreamPoint; STREAM_POINTS],
    /// The spawn countdown, in ticks.
    pub delay: i32,
    /// The stream's two random tints.
    pub tint0: f32,
    pub tint1: f32,
    /// The channel the hits raise and its ceiling.
    pub channel_left: f32,
    pub channel_right: f32,
    /// The stream's own level, as the light reads it.
    pub level: f32,
    /// The stream's aim and its spread.
    pub aim: Vec3,
    pub spread: f32,
}

impl Stream {
    /// The newest point.
    pub fn newest(&self) -> StreamPoint {
        self.points[self.head]
    }

    /// The stream's tint bytes.
    pub fn colour(&self) -> [u8; 4] {
        [
            round_i32(self.tint0 * 255.0) as u8,
            round_i32(self.tint1 * 255.0) as u8,
            0xff,
            round_i32((2.0 - self.level) * self.level * 255.0) as u8,
        ]
    }
}

/// What a torrent step did.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TorrentStep {
    pub phase: i32,
    pub charge: f32,
    pub level: f32,
    pub released: bool,
    pub finished: bool,
}

/// A torrent in flight.
#[derive(Debug, Clone, PartialEq)]
pub struct Torrent {
    pub phase: i32,
    pub charge: f32,
    pub rate: f32,
    /// The shown level: `0.4·(2 − charge)·charge` while charging, then the
    /// fall to zero.
    pub level: f32,
    /// The caster's light scale, which the emissive multiplies.
    pub light: f32,
    pub streams: Vec<Stream>,
    /// The caster's hand.
    pub hand: Vec3,
    /// Whether the power has been reported used.
    pub power_used: bool,
    /// The hit scan's reach scale (0.25 static, 0.5 a character).
    pub hit_scale: f32,
    /// The hit scan's additive reach.
    pub hit_margin: f32,
    /// The share the stream point keeps toward a hit contact.
    pub hit_share: f32,
    /// Whether the last scan touched a body.
    pub hit: bool,
    /// The aim, a unit vector.
    pub aim: Vec3,
}

impl Torrent {
    pub fn new(tick_ms: u32, hand: Vec3, aim: Vec3, light: f32) -> Self {
        let rate = f64::from(tick_ms) * 0.001 * CHARGE_RATE;
        Self {
            phase: CHARGING,
            charge: 0.0,
            rate: rate as f32,
            level: 0.0,
            light,
            streams: Vec::new(),
            hand,
            aim,
            power_used: false,
            hit_scale: 0.25,
            hit_margin: 0.0,
            hit_share: 0.5,
            hit: false,
        }
    }

    /// The sixteen births: a ring of directions stepped by 1.2566371 rad,
    /// each scaled by 16, its height by 1.5 less 30 cm.
    pub fn spawn(&mut self, tints: impl Iterator<Item = (f32, f32)>) {
        let mut tints = tints;
        let mut angle = 0.0f32;
        self.streams.clear();
        for _ in 0..STREAMS {
            let dir = [angle.cos(), angle.sin(), 0.0];
            let local = Vec3::new(
                dir[0] * BIRTH_RADIUS,
                dir[1] * BIRTH_RADIUS,
                dir[2] * BIRTH_HEIGHT - BIRTH_DROP,
            );
            let (tint0, tint1) = tints.next().unwrap_or((0.0, 0.0));
            let mut stream = Stream {
                head: 0,
                points: [StreamPoint {
                    at: [0.0; 3],
                    life: 0,
                    level: 0.0,
                }; STREAM_POINTS],
                delay: 8,
                tint0,
                tint1,
                channel_left: 0.0,
                channel_right: 0.0,
                level: 0.0,
                aim: self.aim,
                spread: 0.0,
            };
            let origin = self.hand + local;
            for point in &mut stream.points {
                point.at = [origin.x, origin.y, origin.z];
            }
            self.streams.push(stream);
            angle += 1.256_637_1;
        }
    }

    /// The spawn delay a generator draw leaves: 8 less the draw's 0 to 2.
    pub fn spawn_delay(roll: f32) -> i32 {
        8 - (roll * 3.0) as i32
    }

    /// One tick.
    pub fn step(&mut self, hand: Vec3, aim: Vec3, roll: f32) -> TorrentStep {
        self.hand = hand;
        self.aim = aim;
        if self.streams.is_empty() && self.phase == CHARGING {
            self.spawn(std::iter::empty());
        }
        match self.phase {
            CHARGING => self.step_charging(roll),
            RUNNING | FADING => self.step_running(hand, aim, roll),
            FIZZLED => self.step_fizzling(),
            _ => TorrentStep {
                phase: self.phase,
                charge: self.charge,
                level: self.level,
                released: false,
                finished: self.phase == FINISHED,
            },
        }
    }

    fn step_charging(&mut self, roll: f32) -> TorrentStep {
        self.charge += self.rate;
        self.level = LEVEL_CURVE * (2.0 - self.charge) * self.charge;
        for stream in &mut self.streams {
            stream.level = self.level;
            advance(stream, self.hand, self.aim, roll, 0.1);
        }
        let mut released = false;
        if self.charge > 1.0 {
            self.charge = 1.0;
            self.phase = RUNNING;
            self.level = LEVEL_CURVE;
            self.power_used = true;
            released = true;
            for stream in &mut self.streams {
                stream.aim = self.aim;
                let local = self.aim * STREAM_STEP;
                let at = Vec3::from_array(stream.newest().at) + local;
                if let Some(point) = stream.points.get_mut(stream.head) {
                    point.at = [at.x, at.y, at.z];
                }
            }
        }
        TorrentStep {
            phase: self.phase,
            charge: self.charge,
            level: self.level,
            released,
            finished: false,
        }
    }

    fn step_running(&mut self, hand: Vec3, aim: Vec3, roll: f32) -> TorrentStep {
        for stream in &mut self.streams {
            stream.level = self.level;
            if stream.delay > 0 {
                stream.delay -= 1;
            }
            if stream.delay < 1 {
                respawn(stream, hand, aim, roll);
                stream.delay = Self::spawn_delay(roll);
            }
            advance(stream, hand, aim, roll, 0.05);
            for point in &mut stream.points {
                if point.life > 0 {
                    point.life -= 1;
                }
            }
        }
        if self.phase == FADING {
            self.level -= LEVEL_FALL;
            if self.level < LEVEL_FLOOR {
                self.level = 0.0;
                self.phase = FINISHED;
            }
        }
        let finished = self.phase == FINISHED;
        TorrentStep {
            phase: self.phase,
            charge: self.charge,
            level: self.level,
            released: false,
            finished,
        }
    }

    fn step_fizzling(&mut self) -> TorrentStep {
        self.charge -= self.rate * 2.0;
        let mut finished = false;
        if self.charge < 0.0 {
            self.charge = 0.0;
            self.phase = FINISHED;
            finished = true;
        }
        self.level = 0.0;
        TorrentStep {
            phase: self.phase,
            charge: self.charge,
            level: self.level,
            released: false,
            finished,
        }
    }

    /// Asks a running torrent to fade.
    pub fn release(&mut self) {
        if self.phase == RUNNING {
            self.phase = FADING;
        }
    }

    /// The two sounds the start plays.
    pub fn start_sounds(&self) -> [&'static str; 2] {
        [CHARGE_SOUND, LOOP_SOUND]
    }

    /// The reach of a stream point: the point's age (its life lost) times
    /// the stream's scale, plus the additive term.
    pub fn reach(point: &StreamPoint, scale: f32) -> f32 {
        let age = STREAM_LIFE_TICKS - point.life;
        (age as f32) * scale + REACH_ADD
    }

    /// The push a point inside a stream's reach takes.
    pub fn push(&self, at: Vec3, direction: Vec3, dist_sq: f32, reach: f32) -> Option<Vec3> {
        let span = reach * reach;
        if dist_sq > span {
            return None;
        }
        let step = (1.0 - dist_sq / span) * PUSH_SHARE;
        Some(at + direction * step)
    }

    /// Raises a stream's two channels by a hit's remaining-life share.
    pub fn hit(stream: &mut Stream, life: i32, samples: i32) {
        let fraction = life as f32 / samples as f32;
        stream.channel_left += fraction * CHANNEL_LEFT;
        if stream.channel_right < CHANNEL_RIGHT_MAX {
            stream.channel_right += fraction * CHANNEL_RIGHT;
        }
    }

    /// The light strength one stream shows.
    pub fn light_strength(&self, stream: &Stream) -> f32 {
        self.light * stream.level * LIGHT_CURVE * (2.0 - self.charge) * self.charge * LIGHT_CHARGE
    }

    /// The draw's texture offset.
    pub fn texture_offset(&self) -> [f32; 3] {
        TEXTURE_OFFSET
    }

    /// The light's whole set: its position, its 2000 strength uniform and
    /// one colour block a stream.
    pub fn light_set(&self) -> TorrentLightSet {
        let mut blocks = Vec::with_capacity(self.streams.len());
        for stream in &self.streams {
            blocks.push([LIGHT_TINT[0], LIGHT_TINT[1], LIGHT_TINT[2]].map(|c| {
                c * self.light
                    * stream.level
                    * LIGHT_CURVE
                    * (2.0 - self.charge)
                    * self.charge
                    * LIGHT_CHARGE
            }));
        }
        TorrentLightSet {
            position: self.light_position(),
            strength: LIGHT_UNIFORM,
            blocks,
        }
    }

    /// The light's position: the mean of the stream heads, 120 cm on each
    /// axis.
    pub fn light_position(&self) -> Vec3 {
        if self.streams.is_empty() {
            return self.hand;
        }
        let first = Vec3::from_array(self.streams[0].newest().at);
        let mut min = first;
        let mut max = first;
        for stream in &self.streams {
            let at = Vec3::from_array(stream.newest().at);
            min = min.min(at);
            max = max.max(at);
        }
        // The middle of the streams' bounds, 120 cm on each axis.
        (min + max) * 0.5 + Vec3::splat(LIGHT_RADIUS)
    }

    /// The stream points as a frame between two ticks draws them.
    pub fn drawn_points(&self, alpha: f32) -> Vec<Vec3> {
        let mut out = Vec::new();
        for stream in &self.streams {
            for step in 0..STREAM_POINTS {
                let index = (stream.head + STREAM_POINTS - step) % STREAM_POINTS;
                let point = stream.points[index];
                let previous = stream.points[(index + 1) % STREAM_POINTS];
                let a = Vec3::from_array(previous.at);
                let b = Vec3::from_array(point.at);
                out.push(a + (b - a) * alpha);
            }
        }
        out
    }

    /// A stream's ribbon: 12 quads through its history.
    pub fn ribbon(&self, stream: &Stream) -> Vec<[f32; 3]> {
        let mut out = Vec::with_capacity((STREAM_QUADS + 1) * 2);
        for step in 0..=STREAM_QUADS {
            let index = (stream.head + STREAM_POINTS - step) % STREAM_POINTS;
            let point = stream.points[index];
            let next = stream.points[(index + 1) % STREAM_POINTS];
            let along = Vec3::from_array(next.at) - Vec3::from_array(point.at);
            let side = along.cross(Vec3::Z).normalize_or_zero();
            let width = point.level * 4.0 + 0.5;
            out.push([
                point.at[0] + side.x * width,
                point.at[1] + side.y * width,
                point.at[2] + side.z * width,
            ]);
            out.push([
                point.at[0] - side.x * width,
                point.at[1] - side.y * width,
                point.at[2] - side.z * width,
            ]);
        }
        out
    }
}

/// The torrent's GI light: one light fed by every stream's colour block.
#[derive(Debug, Clone, PartialEq)]
pub struct TorrentLightSet {
    pub position: Vec3,
    /// The light's strength uniform, 2000.
    pub strength: f32,
    pub blocks: Vec<[f32; 3]>,
}

/// Advances one stream's history: the newest point is pushed toward the
/// hand's offset and the older ones follow it.
fn advance(stream: &mut Stream, hand: Vec3, aim: Vec3, _roll: f32, share: f32) {
    let head = stream.head;
    let target = hand + aim * (BIRTH_RADIUS * 0.5);
    let newest = Vec3::from_array(stream.points[head].at);
    let moved = newest + (target - newest) * share;
    stream.points[head].at = [moved.x, moved.y, moved.z];
    stream.points[head].life = STREAM_LIFE_TICKS;
    stream.points[head].level = stream.level;
    stream.head = (head + STREAM_POINTS - 1) % STREAM_POINTS;
}

/// Puts a run stream back at the hand with a spread of 0.3 of a random
/// angle around the aim.
fn respawn(stream: &mut Stream, hand: Vec3, aim: Vec3, roll: f32) {
    let angle = roll * std::f32::consts::TAU;
    let local = Vec3::new(angle.cos() * 0.3, angle.sin() * 0.3, 1.0);
    let direction = (aim + local * 0.3).normalize_or_zero();
    let at = hand + direction * BIRTH_RADIUS;
    let head = stream.head;
    stream.points[head].at = [at.x, at.y, at.z];
    stream.points[head].life = STREAM_LIFE_TICKS;
    stream.points[head].level = 1.0;
    stream.level = 1.0;
    stream.aim = direction;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn torrent() -> Torrent {
        let mut torrent = Torrent::new(12, Vec3::ZERO, Vec3::X, 1.0);
        torrent.spawn([(0.5, 0.25)].into_iter().cycle().take(STREAMS));
        torrent
    }

    #[test]
    fn the_streams_are_born_on_a_sphere() {
        let torrent = torrent();
        assert_eq!(torrent.streams.len(), 16);
        assert_eq!(torrent.streams[0].newest().at, [16.0, 0.0, -30.0]);
        assert_eq!(Torrent::spawn_delay(0.0), 8);
        assert_eq!(Torrent::spawn_delay(0.5), 7);
        assert_eq!(Torrent::spawn_delay(0.9), 6);
    }

    #[test]
    fn the_level_follows_the_charge_and_the_fade() {
        let mut torrent = torrent();
        let step = torrent.step(Vec3::ZERO, Vec3::X, 0.0);
        assert_eq!(step.phase, CHARGING);
        assert!(step.level > 0.0);
        torrent.charge = 1.0;
        let step = torrent.step(Vec3::ZERO, Vec3::X, 0.0);
        assert!(step.released);
        assert_eq!(step.phase, RUNNING);
        assert_eq!(step.level, 0.4);
        torrent.release();
        let step = torrent.step(Vec3::ZERO, Vec3::X, 0.0);
        assert!((step.level - 0.39).abs() < 1e-6);
        torrent.level = 0.02;
        let step = torrent.step(Vec3::ZERO, Vec3::X, 0.0);
        assert!(step.finished);
    }

    #[test]
    fn a_hit_raises_the_two_channels() {
        let mut stream = torrent().streams[0].clone();
        Torrent::hit(&mut stream, 33, 66);
        assert!((stream.channel_left - 0.0025).abs() < 1e-6);
        assert!((stream.channel_right - 0.005).abs() < 1e-6);
        stream.channel_right = 1.79;
        Torrent::hit(&mut stream, 66, 66);
        assert!((stream.channel_right - 1.8).abs() < 1e-6);
        stream.channel_right = 1.8;
        Torrent::hit(&mut stream, 66, 66);
        assert_eq!(stream.channel_right, 1.8);
    }

    #[test]
    fn a_point_inside_the_reach_is_pushed() {
        let torrent = torrent();
        let push = torrent
            .push(Vec3::ZERO, Vec3::X, 0.0, 10.0)
            .expect("a push");
        assert!((push.x - 0.01).abs() < 1e-6);
        assert!(torrent.push(Vec3::ZERO, Vec3::X, 101.0, 10.0).is_none());
    }

    #[test]
    fn the_light_reads_the_charge_and_the_stream_level() {
        let mut torrent = torrent();
        torrent.charge = 0.5;
        torrent.level = 0.3;
        let mut stream = torrent.streams[0].clone();
        stream.level = 0.3;
        let strength = torrent.light_strength(&stream);
        assert!((strength - 1.0 * 0.3 * 0.25 * 1.5 * 0.5 * 1.2).abs() < 1e-6);
        let at = torrent.light_position();
        let mut min = Vec3::from_array(torrent.streams[0].newest().at);
        let mut max = min;
        for stream in &torrent.streams {
            let p = Vec3::from_array(stream.newest().at);
            min = min.min(p);
            max = max.max(p);
        }
        assert!((at - ((min + max) * 0.5 + Vec3::splat(120.0))).length() < 1e-4);
    }

    #[test]
    fn the_light_set_feeds_every_stream() {
        let mut torrent = torrent();
        torrent.charge = 0.5;
        for stream in &mut torrent.streams {
            stream.level = 0.5;
        }
        let set = torrent.light_set();
        assert_eq!(set.strength, 2000.0);
        assert_eq!(set.blocks.len(), 16);
        let expected = 0.12 * 1.0 * 0.5 * 0.25 * 1.5 * 0.5 * 1.2;
        assert!((set.blocks[0][0] - expected).abs() < 1e-6);
        assert_eq!(torrent.texture_offset(), [0.0, 0.0, 0.0]);
    }

    #[test]
    fn the_start_sounds_and_the_power_used_are_recorded() {
        let mut torrent = torrent();
        assert_eq!(torrent.start_sounds(), ["thmfrc_cu01", "thmfrc_trntdr"]);
        assert!(!torrent.power_used);
        torrent.charge = 1.0;
        torrent.step(Vec3::ZERO, Vec3::X, 0.0);
        assert!(torrent.power_used);
        torrent.release();
        assert_eq!(torrent.phase, FADING);
    }

    #[test]
    fn a_frame_draws_between_the_history_points() {
        let mut torrent = torrent();
        let head = torrent.streams[0].head;
        torrent.streams[0].points[head].at = [10.0, 0.0, 0.0];
        torrent.streams[0].points[(head + 1) % STREAM_POINTS].at = [0.0, 0.0, 0.0];
        let points = torrent.drawn_points(0.5);
        assert_eq!(points.len(), STREAM_POINTS * STREAMS);
        assert_eq!(points[0].x, 5.0);
    }

    #[test]
    fn a_ribbon_runs_through_the_history() {
        let torrent = torrent();
        let ribbon = torrent.ribbon(&torrent.streams[0]);
        assert_eq!(ribbon.len(), (STREAM_QUADS + 1) * 2);
    }
}

/// The close a torrent keeps for a hit point: the nearest within 6400 cm²
/// joins that bucket, and one further than 25600 cm² opens a new one until
/// sixteen exist.
pub const LIGHT_NEAR_SQ: f32 = 6400.0;
pub const LIGHT_FAR_SQ: f32 = 25600.0;
pub const LIGHT_BUCKETS: usize = 16;

/// One light bucket of a hit point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightBucket {
    pub at: Vec3,
    /// The summed level.
    pub level: f32,
    /// How many hits the bucket holds.
    pub weight: i32,
}

/// The hit points' merged light buckets.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LightBuckets {
    pub buckets: Vec<LightBucket>,
}

impl LightBuckets {
    /// Merges one hit at a position with a charge: the level is
    /// `(2 − charge)·charge`. Returns whether a bucket was opened.
    pub fn merge(&mut self, at: Vec3, charge: f32) -> bool {
        let level = (2.0 - charge) * charge;
        if self.buckets.is_empty() {
            self.buckets.push(LightBucket {
                at,
                level,
                weight: 1,
            });
            return true;
        }
        let mut nearest = -1i32;
        let mut best = LIGHT_FAR_SQ;
        for (index, bucket) in self.buckets.iter().enumerate() {
            let d = bucket.at - at;
            let distance_sq = d.x * d.x + d.y * d.y + d.z * d.z;
            if distance_sq < best {
                best = distance_sq;
                nearest = index as i32;
            }
        }
        if self.buckets.len() > LIGHT_BUCKETS - 1 || best <= LIGHT_NEAR_SQ {
            if 0 <= nearest {
                let bucket = &mut self.buckets[nearest as usize];
                bucket.weight += 1;
                let share = 1.0 / bucket.weight as f32;
                bucket.at += (at - bucket.at) * share;
                bucket.level += level;
            }
            false
        } else {
            self.buckets.push(LightBucket {
                at,
                level,
                weight: 1,
            });
            true
        }
    }
}

/// A character state the torrent hits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TorrentCharacter {
    /// The inverse mass the torrent reads.
    pub inverse_mass: f32,
    /// The balance/guard value.
    pub guard: f32,
    /// How many hits the character has taken.
    pub hits: i32,
    /// The character's state word; 0x10 is a stance the hit knocks out.
    pub state: u32,
    /// The stun the hit sets.
    pub stun: f32,
    /// The stance timer the hit clears.
    pub stance: f32,
    /// The struck part's flag word.
    pub part_flags: u32,
    /// The hostility the hit adds toward the caster.
    pub anger: f32,
}

/// What a torrent character hit changed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TorrentCharacterHit {
    pub knock: Vec3,
    pub stun: f32,
    pub anger: f32,
    pub hits: i32,
    pub knocked: bool,
}

/// The torrent's hit on a character: the guard drops by the inverse mass
/// over 2000, the hit count rises, an impulse over one is normalised, capped
/// at 30 and doubled into the knock, a stance state loses its flag and its
/// timer, the anger gains 0.01, and the stun is `0.08·(1 − inv mass)`
/// floored at zero and — the original's inverted clamp — set to 0.008 above
/// 0.08.
pub fn torrent_character_hit(
    character: &mut TorrentCharacter,
    impulse: Vec3,
) -> TorrentCharacterHit {
    if 0.25 < character.guard {
        character.guard =
            (f64::from(character.guard) - f64::from(character.inverse_mass) * 0.0005) as f32;
    }
    character.hits += 1;
    let speed = impulse.length();
    let mut knock = Vec3::ZERO;
    if speed > 1.0 {
        let direction = impulse / speed;
        let capped = if speed > 30.0 { 30.0 } else { speed };
        knock = direction * capped * 2.0;
        if character.state == 0x10 {
            character.part_flags |= 0x80;
            character.stance = 0.0;
        }
    }
    character.anger += 0.01;
    let mut stun = ((1.0 - f64::from(character.inverse_mass)) * 0.08) as f32;
    if stun < 0.0 {
        stun = 0.0;
    } else if 0.08f64 < f64::from(stun) {
        stun = 0.008;
    }
    character.stun = stun;
    TorrentCharacterHit {
        knock,
        stun,
        anger: 0.01,
        hits: character.hits,
        knocked: speed > 1.0,
    }
}

/// A torrent's aim: the hand's direction, its yaw clamped to 45° around the
/// body's aim, and the pitch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TorrentAim {
    pub yaw: f32,
    pub pitch: f32,
    pub direction: Vec3,
}

/// The torrent's aim from the hand, the view's origin and the body's aim
/// yaw: the planar yaw of the hand's direction, limited to a quarter turn
/// around the body's, and the pitch is the negated arc sine of the height.
pub fn torrent_aim(hand: Vec3, origin: Vec3, body_yaw: f32) -> TorrentAim {
    let d = (hand - origin).normalize_or_zero();
    let mut yaw = crate::nav_camera::planar_yaw(d);
    let mut delta = yaw - body_yaw;
    if delta < -std::f32::consts::PI {
        delta += std::f32::consts::TAU;
    } else if std::f32::consts::PI < delta {
        delta -= std::f32::consts::TAU;
    }
    if std::f32::consts::FRAC_PI_4 < delta {
        yaw = body_yaw + std::f32::consts::FRAC_PI_4;
    } else if delta < -std::f32::consts::FRAC_PI_4 {
        yaw = body_yaw - std::f32::consts::FRAC_PI_4;
    }
    if yaw < -std::f32::consts::PI {
        yaw += std::f32::consts::TAU;
    } else if std::f32::consts::PI < yaw {
        yaw -= std::f32::consts::TAU;
    }
    let pitch = -d.y.clamp(-1.0, 1.0).asin();
    TorrentAim {
        yaw,
        pitch,
        direction: Vec3::new(
            yaw.sin() * pitch.cos(),
            -pitch.sin(),
            yaw.cos() * pitch.cos(),
        ),
    }
}

/// A torrent's contact setup: the body's frame, the push (0.2) and the
/// share (0.5) it uses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TorrentContact {
    pub origin: Vec3,
    pub push: f32,
    pub share: f32,
    /// Whether a hit callback was set; the setup clears it.
    pub callback: bool,
}

/// The link the draw callback installs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TorrentLink {
    pub geometry: bool,
    pub effect_flag: u32,
    pub callback: bool,
}

/// What destroying a torrent releases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TorrentTeardown {
    pub stopped: bool,
    pub sounds_released: usize,
    pub light_released: bool,
    pub streams_released: usize,
}

impl Torrent {
    /// The contact a body's hit begins with.
    pub fn contact(&mut self, origin: Vec3) -> TorrentContact {
        self.hit_scale = 0.25;
        self.hit_margin = 0.0;
        self.hit_share = 0.5;
        self.hit = false;
        TorrentContact {
            origin,
            push: self.hit_scale,
            share: self.hit_share,
            callback: false,
        }
    }

    /// Links the geometry and installs the draw callback.
    pub fn link(&mut self) -> TorrentLink {
        TorrentLink {
            geometry: true,
            effect_flag: 0x1_0000,
            callback: true,
        }
    }

    /// Destroys the torrent: `stop` stops a running loop first; the two
    /// sound handles and the GI light are released and the streams dropped.
    pub fn destroy(&mut self, stop: bool) -> TorrentTeardown {
        let stopped = stop && !self.streams.is_empty();
        let streams = self.streams.len();
        self.streams.clear();
        self.power_used = false;
        self.phase = FINISHED;
        TorrentTeardown {
            stopped,
            sounds_released: 2,
            light_released: true,
            streams_released: streams,
        }
    }

    /// The effect's frame: its translation at the hand.
    pub fn frame_matrix(&self) -> [[f32; 4]; 4] {
        [
            [1.0, 0.0, 0.0, self.hand.x],
            [0.0, 1.0, 0.0, self.hand.y],
            [0.0, 0.0, 1.0, self.hand.z],
            [0.0, 0.0, 0.0, 1.0],
        ]
    }

    /// The torrent's draw descriptions, one a view.
    pub fn draw_calls(&self, views: &[[[f32; 4]; 4]]) -> Vec<crate::effects::draw::DrawCall> {
        crate::effects::draw::torrent_geometry_draws(
            views,
            self.frame_matrix(),
            self.texture_offset(),
        )
    }
}

#[cfg(test)]
mod hit_tests {
    use super::*;

    fn torrent() -> Torrent {
        let mut torrent = Torrent::new(12, Vec3::ZERO, Vec3::X, 1.0);
        torrent.spawn([(0.5, 0.25)].into_iter().cycle().take(STREAMS));
        torrent
    }

    #[test]
    fn a_close_hit_merges_and_a_far_one_opens_a_bucket() {
        let mut buckets = LightBuckets::default();
        assert!(buckets.merge(Vec3::ZERO, 0.5));
        assert_eq!(buckets.buckets[0].level, 0.75);
        assert!(!buckets.merge(Vec3::new(10.0, 0.0, 0.0), 0.5));
        assert_eq!(buckets.buckets.len(), 1);
        assert_eq!(buckets.buckets[0].weight, 2);
        assert_eq!(buckets.buckets[0].level, 1.5);
        assert_eq!(buckets.buckets[0].at, Vec3::new(5.0, 0.0, 0.0));
        assert!(buckets.merge(Vec3::new(0.0, 200.0, 0.0), 0.5));
        assert_eq!(buckets.buckets.len(), 2);
    }

    #[test]
    fn a_full_bucket_list_merges_its_nearest_whatever_the_distance() {
        let mut buckets = LightBuckets::default();
        for index in 0..LIGHT_BUCKETS {
            assert!(buckets.merge(Vec3::new(index as f32 * 200.0, 0.0, 0.0), 0.5));
        }
        assert_eq!(buckets.buckets.len(), LIGHT_BUCKETS);
        assert!(!buckets.merge(Vec3::new(0.0, 0.0, 0.0), 0.5));
        assert_eq!(buckets.buckets.len(), LIGHT_BUCKETS);
        assert_eq!(buckets.buckets[0].weight, 2);
    }

    #[test]
    fn a_torrent_hit_knocks_stuns_and_angers() {
        let mut character = TorrentCharacter {
            inverse_mass: 0.2,
            guard: 0.3,
            hits: 0,
            state: 0x10,
            stun: 0.0,
            stance: 2.0,
            part_flags: 0,
            anger: 0.0,
        };
        let hit = torrent_character_hit(&mut character, Vec3::new(100.0, 0.0, 0.0));
        assert_eq!(hit.knock, Vec3::new(60.0, 0.0, 0.0));
        assert_eq!(hit.hits, 1);
        assert!((character.guard - (0.3 - 0.2 * 0.0005)).abs() < 1e-6);
        assert_eq!(character.part_flags, 0x80);
        assert_eq!(character.stance, 0.0);
        assert!((character.stun - 0.064).abs() < 1e-6);
        assert_eq!(character.anger, 0.01);
        assert!(hit.knocked);
    }

    #[test]
    fn a_slow_torrent_hit_still_counts_and_the_stun_clamp_is_inverted() {
        let mut character = TorrentCharacter {
            inverse_mass: 2.0,
            guard: 0.1,
            hits: 4,
            state: 0,
            stun: 0.0,
            stance: 0.0,
            part_flags: 0,
            anger: 0.0,
        };
        let hit = torrent_character_hit(&mut character, Vec3::new(0.5, 0.0, 0.0));
        assert_eq!(hit.knock, Vec3::ZERO);
        assert_eq!(hit.hits, 5);
        assert_eq!(character.stun, 0.0);
        let mut high = TorrentCharacter {
            inverse_mass: -0.5,
            ..character
        };
        torrent_character_hit(&mut high, Vec3::new(0.5, 0.0, 0.0));
        assert_eq!(high.stun, 0.008);
    }

    #[test]
    fn the_aim_is_clamped_around_the_body() {
        let aim = torrent_aim(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO, 0.0);
        assert!(aim.yaw.abs() < 1e-6);
        assert!(aim.direction.z > 0.99);
        let side = torrent_aim(Vec3::new(10.0, 0.0, 10.0), Vec3::ZERO, 0.0);
        assert!((side.yaw - std::f32::consts::FRAC_PI_4).abs() < 2e-3);
        let up = torrent_aim(Vec3::new(0.0, 10.0, 10.0), Vec3::ZERO, 0.0);
        assert!((up.pitch + std::f32::consts::FRAC_PI_4).abs() < 1e-5);
        assert!(up.direction.y > 0.7);
    }

    #[test]
    fn the_contact_and_destroy_keep_the_numbers() {
        let mut torrent = torrent();
        let contact = torrent.contact(Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(
            (contact.push, contact.share, contact.callback),
            (0.25, 0.5, false)
        );
        assert!(torrent.link().callback);
        assert_eq!(
            torrent
                .draw_calls(&[[
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0]
                ]])
                .len(),
            1
        );
        let teardown = torrent.destroy(true);
        assert!(teardown.stopped);
        assert_eq!(teardown.sounds_released, 2);
        assert!(teardown.light_released);
        assert_eq!(teardown.streams_released, 16);
    }
}

/// A body the torrent's scan can touch: its capsule, bounding radius, the
/// points that move and the impact gathered.
#[derive(Debug, Clone, PartialEq)]
pub struct TorrentBody {
    pub centre: Vec3,
    pub second: Vec3,
    pub bounding: f32,
    pub points: Vec<Vec3>,
    pub impact: f64,
}

/// What a torrent scan touched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TorrentScan {
    pub hit: bool,
    pub pushes: usize,
}

/// The static contact's surface push: a stream point inside a body's sphere
/// moves to its surface and reports the contact normal and distance.
pub fn push_stream_to_surface(point: Vec3, centre: Vec3, radius: f32) -> Option<(Vec3, Vec3, f32)> {
    let d = point - centre;
    let distance_sq = d.length_squared();
    if distance_sq < 0.000_000_119_208_998_228_259_58 || radius * radius <= distance_sq {
        return None;
    }
    let distance = distance_sq.sqrt();
    let normal = d / distance;
    let pushed = centre + normal * radius;
    Some((pushed, normal, radius - distance))
}

/// The torrent's push on a limb: the mass share `mass/(mass + 0.5)` times
/// the effect's scale, 0.3, the age's `(2 − s)·s` and the point's weight,
/// with the overshoot's clamp and the recoil along the point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TorrentLimbPush {
    pub impulse: Vec3,
    pub recoil: Vec3,
}

/// The limb push of a torrent contact.
#[allow(clippy::too_many_arguments)]
pub fn torrent_limb_push(
    direction: Vec3,
    point_offset: Vec3,
    mass: f32,
    scale: f32,
    age: i32,
    max_age: i32,
    weight: f32,
    share: f32,
) -> TorrentLimbPush {
    let factor = ((f64::from(mass) / f64::from(mass + 0.5)) * f64::from(scale) * 0.3) as f32;
    let fraction = (age as f64 / max_age as f64) as f32;
    let factor = factor * (2.0 - fraction) * fraction;
    let length_sq = point_offset.length_squared();
    if length_sq <= 0.0 {
        return TorrentLimbPush {
            impulse: Vec3::ZERO,
            recoil: Vec3::ZERO,
        };
    }
    let overlap = 1.0 - direction.dot(point_offset) / length_sq;
    if overlap <= 0.0 {
        return TorrentLimbPush {
            impulse: Vec3::ZERO,
            recoil: Vec3::ZERO,
        };
    }
    let overlap = if overlap > 1.0 { 1.0 } else { overlap };
    TorrentLimbPush {
        impulse: direction * (factor * overlap * weight),
        recoil: (point_offset * -1.0) * ((1.0 - weight) * factor * share),
    }
}

impl Torrent {
    /// The hit scan over the streams' history: every live point whose reach
    /// (`age·scale + margin + 10`) meets the body pushes its points by
    /// `(1 − d²/r²)·0.01`, leaves 0.5 of the stream point on the contact,
    /// raises the stream's channels and records the hit.
    pub fn scan_hits(&mut self, body: &mut TorrentBody) -> TorrentScan {
        let mut pushes = 0;
        self.hit = false;
        for stream in &mut self.streams {
            let mut index = (stream.head + 1) % STREAM_POINTS;
            loop {
                index = (index + STREAM_POINTS - 1) % STREAM_POINTS;
                if stream.points[index].life < 1 {
                    break;
                }
                let point = stream.points[index];
                let age = STREAM_LIFE_TICKS - point.life;
                let reach = age as f32 * self.hit_scale + self.hit_margin + 10.0;
                let reach_sq = reach * reach;
                let centre_d = Vec3::from_array(point.at) - body.centre;
                let span = reach + body.bounding;
                if centre_d.length_squared() > span * span {
                    continue;
                }
                if crate::effects::blast::point_segment_distance_sq(
                    Vec3::from_array(point.at),
                    body.centre,
                    body.second,
                ) > reach_sq
                {
                    continue;
                }
                let previous = stream.points[(index + 1) % STREAM_POINTS];
                let motion = Vec3::from_array(point.at) - Vec3::from_array(previous.at);
                for body_point in &mut body.points {
                    let d = Vec3::from_array(point.at) - *body_point;
                    let distance_sq = d.length_squared();
                    if distance_sq <= reach_sq {
                        let fraction = 1.0 - distance_sq / reach_sq;
                        *body_point += motion * (fraction * 0.01);
                        body.impact += f64::from(fraction) * 0.00001;
                        let at = Vec3::from_array(point.at);
                        let moved = at + (*body_point - at) * self.hit_share;
                        stream.points[index].at = [moved.x, moved.y, moved.z];
                        pushes += 1;
                        self.hit = true;
                    }
                }
                let life_share = point.life as f32 / STREAM_LIFE_TICKS as f32;
                stream.level += life_share * 0.005;
                if stream.channel_right < CHANNEL_RIGHT_MAX {
                    stream.channel_right += life_share * CHANNEL_RIGHT;
                }
                if index == stream.head {
                    break;
                }
            }
        }
        TorrentScan {
            hit: self.hit,
            pushes,
        }
    }
}

#[cfg(test)]
mod scan_tests {
    use super::*;

    fn torrent() -> Torrent {
        let mut torrent = Torrent::new(12, Vec3::ZERO, Vec3::X, 1.0);
        torrent.spawn([(0.5, 0.25)].into_iter().cycle().take(STREAMS));
        torrent
    }

    #[test]
    fn a_stream_point_on_a_body_pushes_its_points() {
        let mut torrent = torrent();
        let mut body = TorrentBody {
            centre: Vec3::ZERO,
            second: Vec3::ZERO,
            bounding: 1.0,
            points: vec![Vec3::new(0.0, 0.0, 1.0)],
            impact: 0.0,
        };
        for stream in &mut torrent.streams {
            for (slot, point) in stream.points.iter_mut().enumerate() {
                point.at = [0.0, 0.0, if slot == 0 { 0.0 } else { -1.0 }];
                point.life = STREAM_LIFE_TICKS;
            }
        }
        let scan = torrent.scan_hits(&mut body);
        assert!(scan.hit);
        assert!(scan.pushes >= 1);
        // The newest point moved a centimetre up from its previous one, so
        // the body point at z = 1 is pushed the same way.
        assert!(body.points[0].z > 1.0);
        assert!(body.impact > 0.0);
    }

    #[test]
    fn a_surface_push_moves_the_point_to_the_radius() {
        let (point, normal, depth) =
            push_stream_to_surface(Vec3::new(0.0, 0.0, 2.0), Vec3::ZERO, 5.0).expect("a push");
        assert_eq!(point, Vec3::new(0.0, 0.0, 5.0));
        assert_eq!(normal, Vec3::Z);
        assert_eq!(depth, 3.0);
        assert!(push_stream_to_surface(Vec3::new(0.0, 0.0, 6.0), Vec3::ZERO, 5.0).is_none());
    }

    #[test]
    fn a_limb_takes_the_mass_share_and_the_age_curve() {
        let push = torrent_limb_push(
            Vec3::X,
            Vec3::new(10.0, 0.0, 0.0),
            0.5,
            0.5,
            33,
            66,
            1.0,
            0.5,
        );
        // 0.5/1.0 · 0.5 · 0.3 · (2 − 0.5) · 0.5 · 0.9 = 0.050625.
        assert!((push.impulse.x - 0.050625).abs() < 1e-6);
        assert_eq!(push.recoil, Vec3::ZERO);
        let outside = torrent_limb_push(Vec3::X, Vec3::ZERO, 0.5, 0.5, 33, 66, 1.0, 0.5);
        assert_eq!(outside.impulse, Vec3::ZERO);
    }
}
