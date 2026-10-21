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

    /// Raises a stream's two channels by a hit's age share.
    pub fn hit(stream: &mut Stream, age: i32, samples: i32) {
        let fraction = age as f32 / samples as f32;
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
        let mut sum = Vec3::ZERO;
        for stream in &self.streams {
            sum += Vec3::from_array(stream.newest().at);
        }
        sum / self.streams.len() as f32 + Vec3::splat(LIGHT_RADIUS)
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
        assert!((at - Vec3::new(121.0, 120.0, 90.0)).length() < 1e-4);
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
