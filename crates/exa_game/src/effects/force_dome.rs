//! The force domes: the ripple list and phases shared by the Barrier and the
//! Bubble, and the two effects themselves.
//!
//! - **Ripples** ([`Ripples`]): up to eight. A strike
//!   ([`Ripples::strike`]) raises the dome's peak (strictly larger values
//!   only), is capped at 5, and goes to the ripple whose target lies nearest
//!   within the reach (squared distances, strictly nearer); that ripple
//!   counts the strike, adds its point to the sum and keeps the larger
//!   strength. With none in reach and room left a new ripple opens at the
//!   point with the strike's strength. Each tick ([`Ripples::tick`]) the
//!   reach becomes `30 + 1.25·count`; a struck ripple's sum is scaled by
//!   one over its count, and its target jumps there while it is not shown
//!   yet, otherwise moves half way; then every ripple steps
//!   ([`crate::thaum::step_ripple`]: strength less 0.05 in double, shown
//!   value toward it by at most 0.04, place 0.05 toward the target) and one
//!   whose two values are both under 0.07 is replaced by the last.
//! - **The cast** ([`Dome::cast`]): state 1 (growing) from the setup's
//!   start time, reach 30, the node scaled to the radius; the cast sound
//!   plays at the caster's x and z and the higher of its height plus 10 and
//!   the ground plus 100, its volume the effect's (0.2 Barrier, 0.3 Bubble)
//!   and its pitch `0.6 + 0.4·r`.
//! - **The frame** ([`Dome::frame`]): the level of detail is 500 over the
//!   squared camera distance; the shader's eight ripple points are each
//!   ripple's place and shown value (the free slots' values go to 0); while
//!   growing or shrinking the shown share is the previous and the current
//!   share blended by the frame's fraction; the drawn fade is
//!   `max(1 − shown share, 0)` under state 3 and the shown share from 3.
//!   At state 4 the node leaves the scene, once.
//! - **The Barrier** ([`Barrier`]): a cap on the caster's facing side. Its
//!   centre stands at the caster's x and z, its height a damped follow of
//!   the higher of the caster's height plus 10 and the ground plus 100
//!   (`h += (h − h_prev)·0.95 + (target − h)·0.003`, double). While under
//!   state 2 it drains the caster's focus by the power's rate (half of it
//!   once standing). Growing runs only while under 800 ms have passed
//!   (`share = ms·0.00125`, so the share stops just short of 1 and the
//!   `PowerUsed` branch at 1 is never reached by the tick); the drawn radius
//!   is `(2 − share)·share·0.9·radius`. Ending flips the share and keeps the
//!   clock it would have had; shrinking runs at `ms·0.002` with a drawn
//!   radius of `(1 − share²)·0.9·radius`, and passes 1 to state 4. Its
//!   draw scales the cap 85 by 85 by 25 cm, pushed 75 cm along the facing
//!   (bounding radius `√(25² + 85²)`), its glow is four times the power's
//!   hit value, and the hum follows 80 cm ahead.
//! - **The Bubble** ([`Bubble`]): a full sphere. Growing at `ms/1200` passes
//!   1 to standing; ending flips the share, plays the break sound (volume
//!   0.3, pitch `0.6 + 0.2·r`) and shrinks at `ms/700` to state 4. Its glow
//!   is the ripples' summed shown values times 0.04, `(2 − x)·x/2` while at
//!   most 1 and 0.5 above.
//! - **Keeping out**: each body part (a sphere in the dome's frame) is
//!   tested against the shell. The Barrier skips its caster; a part within
//!   the radius plus its own, not below half the radius, inside the drawn
//!   radius sideways and outside the inner surface is pushed out
//!   ([`Barrier::part_push`]); the Bubble pushes its caster's parts back in
//!   ([`Bubble::own_part_push`]) and other parts out where the growing or
//!   shrinking shell already covers them ([`Bubble::other_part_push`]).
//!   Each push opens or feeds a ripple on the shell, and a struck character
//!   is shoved 40 cm, plays a hit sound and loses balance
//!   ([`Dome::contact`], [`Barrier::contact`]).
//!
//! Both meshes are double-sided: every face reversed, then every face as
//! built ([`dome_mesh`]).
//!
//! Evidence: the dome's cast, mesh, frame, strike, contact and ripple tick,
//! the Barrier's mesh, cast, tick, pose snapshot, placement, frame and
//! keep-out, and the Bubble's cast, mesh, tick and keep-out, read from
//! `Exanima.exe`. The rate, the hit value and the radius are set by the
//! powers, outside this group.

// NaN-sensitive compares are kept as the original makes them.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use bevy::prelude::*;

use exa_assets::polyhedra::Polyhedron;

use crate::arena::Mt19937;
use crate::effects::{distance_squared, level_of_detail};
use crate::nav_camera::planar_yaw;
use crate::thaum::engine::{Roll, lerp, normalize};
use crate::thaum::{
    BarrierPose, advance_barrier_pose, barrier_lerp, dome_extent, step_ripple,
    update_barrier_position,
};

/// At most eight ripples.
pub const MOST_RIPPLES: usize = 8;
/// A strike is capped at this strength.
pub const MOST_STRIKE: f32 = 5.0;
/// The reach a cast starts with, cm.
pub const START_REACH_CM: f32 = 30.0;
/// The glow per unit of shown ripple, and its cap above one.
pub const GLOW_PER_SHOWN: f32 = 0.04;
pub const GLOW_CAP: f32 = 0.5;
/// The level of detail's numerator.
pub const LOD_BASE: f32 = 500.0;
/// The cast sound's volume: the Barrier's and the Bubble's.
pub const BARRIER_CAST_VOLUME: f32 = crate::thaum::BARRIER_CAST_VOLUME;
pub const BUBBLE_CAST_VOLUME: f32 = f32::from_bits(0x3e99_999a);
/// The Barrier's grow and shrink rates a millisecond, and its grow window.
pub const BARRIER_GROW_PER_MS: f64 = 0.001_25;
pub const BARRIER_SHRINK_PER_MS: f64 = 0.002;
pub const BARRIER_GROW_MS: u64 = 800;
/// The ending's clock: the share times this many ms.
pub const BARRIER_END_MS: f32 = 500.0;
/// The Bubble's grow and shrink rates a millisecond.
pub const BUBBLE_GROW_PER_MS: f64 = 0.000_833_333_333_333_333_4;
pub const BUBBLE_SHRINK_PER_MS: f64 = 0.001_428_571_428_571_428_6;
pub const BUBBLE_END_MS: f32 = 700.0;
/// The Barrier's drawn cap and its push along the facing, cm.
pub const BARRIER_DRAW_SCALE: Vec3 = Vec3::new(85.0, 85.0, 25.0);
pub const BARRIER_FORWARD_CM: f32 = 75.0;
/// The Barrier's hum stands this far ahead of the cap's centre.
pub const BARRIER_HUM_AHEAD_CM: f32 = 80.0;
/// A struck character is shoved this hard.
pub const CONTACT_PUSH: f32 = 40.0;
/// The Bubble's break sound volume.
pub const BREAK_VOLUME: f32 = f32::from_bits(0x3e99_999a);

/// A ripple on the shell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ripple {
    /// Where it shows.
    pub place: Vec3,
    /// Where it eases to.
    pub target: Vec3,
    /// Its shown strength and its strength.
    pub shown: f32,
    pub strength: f32,
    /// The sum of this tick's strike points and their count.
    pub sum: Vec3,
    pub strikes: i32,
}

/// The dome's ripple list.
#[derive(Debug, Clone, PartialEq)]
pub struct Ripples {
    pub list: Vec<Ripple>,
    /// The reach a strike merges within, cm.
    pub reach: f32,
    /// The largest strike since the keep-out last reset it.
    pub peak: f32,
    /// The shader's eight points: place and shown value.
    pub points: [[f32; 4]; MOST_RIPPLES],
}

impl Default for Ripples {
    fn default() -> Self {
        Self {
            list: Vec::with_capacity(MOST_RIPPLES),
            reach: START_REACH_CM,
            peak: 0.0,
            points: [[0.0; 4]; MOST_RIPPLES],
        }
    }
}

impl Ripples {
    /// The keep-out's reset of the peak before it tests a body.
    pub fn begin_body(&mut self) {
        self.peak = 0.0;
    }

    /// A strike at `point` (in the dome's frame).
    pub fn strike(&mut self, point: Vec3, strength: f32) {
        if strength > self.peak {
            self.peak = strength;
        }
        let strength = if strength > MOST_STRIKE {
            MOST_STRIKE
        } else {
            strength
        };
        let mut best = None;
        let mut nearest = self.reach * self.reach;
        for (index, ripple) in self.list.iter().enumerate() {
            let d = distance_squared(ripple.target, point);
            if d < nearest {
                nearest = d;
                best = Some(index);
            }
        }
        match best {
            Some(index) => {
                let ripple = &mut self.list[index];
                ripple.strikes += 1;
                ripple.sum += point;
                if strength > ripple.strength {
                    ripple.strength = strength;
                }
            }
            None if self.list.len() < MOST_RIPPLES => self.list.push(Ripple {
                place: point,
                target: point,
                shown: 0.0,
                strength,
                sum: point,
                strikes: 1,
            }),
            None => {}
        }
    }

    /// One tick: the reach, the strikes' targets, then each ripple's step.
    pub fn tick(&mut self) {
        self.reach = dome_extent(self.list.len() as u32);
        for ripple in &mut self.list {
            if ripple.strikes > 0 {
                let mean = ripple.sum * (1.0 / ripple.strikes as f32);
                ripple.target = if ripple.shown > 0.0 {
                    lerp(ripple.target, mean, 0.5)
                } else {
                    mean
                };
                ripple.strikes = 0;
            }
            ripple.sum = Vec3::ZERO;
        }
        let mut index = 0;
        while index < self.list.len() {
            let ripple = &mut self.list[index];
            match step_ripple(
                ripple.shown,
                ripple.strength,
                ripple.place.to_array(),
                ripple.target.to_array(),
            ) {
                Some(step) => {
                    ripple.shown = step.shown;
                    ripple.strength = step.strength;
                    ripple.place = Vec3::from_array(step.at);
                    index += 1;
                }
                None => {
                    self.list.swap_remove(index);
                }
            }
        }
    }

    /// The frame's copy of the ripples into the shader's points, and the
    /// summed shown values.
    pub fn copy_points(&mut self) -> f32 {
        let mut total = 0.0;
        for (slot, ripple) in self.points.iter_mut().zip(&self.list) {
            *slot = [ripple.place.x, ripple.place.y, ripple.place.z, ripple.shown];
            total += ripple.shown;
        }
        for slot in self.points.iter_mut().skip(self.list.len()) {
            slot[3] = 0.0;
        }
        total
    }
}

/// The ripples' glow: `x = total·0.04` through `(2 − x)·x/2`, a half above 1.
pub fn ripple_glow(total: f32) -> f32 {
    let x = total * GLOW_PER_SHOWN;
    if x <= 1.0 {
        (2.0 - x) * x * 0.5
    } else {
        GLOW_CAP
    }
}

/// The dome's state word.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Phase {
    Standing = 0,
    Growing = 1,
    Ending = 2,
    Shrinking = 3,
    Gone = 4,
}

/// A sound the dome plays: its place, volume and pitch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sound {
    pub at: Vec3,
    pub volume: f32,
    pub pitch: f32,
}

/// The shared dome.
#[derive(Debug, Clone, PartialEq)]
pub struct Dome {
    /// Its radius, cm, and its centre.
    pub radius: f32,
    pub at: Vec3,
    pub phase: Phase,
    /// The clock the phase counts from, ms.
    pub since_ms: u32,
    /// The share, the share before this tick, and the frame's blend.
    pub share: f32,
    pub previous_share: f32,
    pub shown_share: f32,
    pub ripples: Ripples,
    /// The geometry's glow.
    pub glow: f32,
    /// The shader's texture position, two draws at the mesh build.
    pub tex_pos: [f32; 2],
    /// The game clock before which no hit sound plays.
    pub quiet_until_ms: u32,
    /// The node has left the scene.
    pub removed: bool,
    /// The scene node the dome attaches to.
    pub node: DomeNode,
    /// The dome's GI light object exists.
    pub light: bool,
    /// The dome's render material exists.
    pub material: bool,
}

/// The scene node a dome attaches to: its flags word, its owner back
/// pointer and the dome's frame step as its hook.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DomeNode {
    pub attached: bool,
    /// The node's flags word: the attach sets 0x10000.
    pub flags: u32,
    /// The node's owner slot points back at the dome.
    pub owner: bool,
    /// The node runs the dome's frame step.
    pub frame_hook: bool,
}

/// What destroying a dome freed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DomeTeardown {
    /// The class's clear hook ran (the original's real delete only).
    pub cleared: bool,
    /// The GI light object was freed.
    pub light: bool,
    /// The material was released and freed.
    pub material: bool,
    /// The object freed itself.
    pub object: bool,
}

/// What a dome's frame hands the draw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DomeFrame {
    pub lod: f32,
    /// The shader's `Fade`.
    pub fade: f32,
    /// The hum's volume and pitch.
    pub hum: [f32; 2],
    /// The node leaves the scene this frame.
    pub remove: bool,
}

impl Dome {
    /// The cast: the dome grows from `start_ms` around `at`; the cast sound
    /// plays at the caster's place raised to the higher of its height plus
    /// 10 and the ground plus 100.
    pub fn cast(
        radius: f32,
        at: Vec3,
        start_ms: u32,
        caster: Vec3,
        ground_y: f32,
        volume: f32,
        random: &mut Mt19937,
    ) -> (Self, Sound) {
        let dome = Self {
            radius,
            at,
            phase: Phase::Growing,
            since_ms: start_ms,
            share: 0.0,
            previous_share: 0.0,
            shown_share: 0.0,
            ripples: Ripples::default(),
            glow: 0.0,
            tex_pos: [random.unit() as f32, random.unit() as f32],
            quiet_until_ms: 0,
            removed: false,
            node: DomeNode::default(),
            light: false,
            material: false,
        };
        let sound = Sound {
            at: Vec3::new(caster.x, sound_height(caster.y, ground_y), caster.z),
            volume,
            pitch: (random.unit() * 0.4 + 0.6) as f32,
        };
        (dome, sound)
    }

    /// Attach the dome to its scene node: the node takes flag 0x10000,
    /// points its owner slot back at the dome and runs the dome's frame
    /// step. The cast stores the node and copies its frame for the
    /// light.
    pub fn attach_node(&mut self) {
        self.node.attached = true;
        self.node.flags |= 0x0001_0000;
        self.node.owner = true;
        self.node.frame_hook = true;
    }

    /// Destroy the dome, as the class's destructor does: a real delete
    /// (`owned`) first runs the class's clear hook; the light object and
    /// the render material are freed when they exist; the base object
    /// leaves the scene; and an owned object frees itself. Returns what it
    /// had.
    pub fn destroy(&mut self, owned: bool) -> DomeTeardown {
        let teardown = DomeTeardown {
            cleared: owned,
            light: self.light,
            material: self.material,
            object: owned,
        };
        self.light = false;
        self.material = false;
        self.removed = true;
        self.node.attached = false;
        self.node.owner = false;
        self.node.frame_hook = false;
        teardown
    }

    /// The share the frame shows at `alpha`, while growing or shrinking.
    fn blend_share(&mut self, alpha: f32) {
        if matches!(self.phase, Phase::Growing | Phase::Shrinking) {
            self.shown_share = (1.0 - alpha) * self.previous_share + self.share * alpha;
        }
    }

    /// The shader's fade: `max(1 − shown, 0)` under state 3, else the shown
    /// share.
    pub fn fade(&self) -> f32 {
        if self.phase < Phase::Shrinking {
            let left = 1.0 - self.shown_share;
            if left <= 0.0 { 0.0 } else { left }
        } else {
            self.shown_share
        }
    }

    /// The common frame: the level of detail, the ripple points, the share
    /// blend and the removal. `None` once removed.
    fn frame_common(&mut self, alpha: f32, camera: Vec3, centre: Vec3) -> Option<(f32, f32, bool)> {
        if self.removed {
            return None;
        }
        let lod = level_of_detail(LOD_BASE, camera, centre);
        let total = self.ripples.copy_points();
        self.blend_share(alpha);
        if self.phase == Phase::Gone {
            self.removed = true;
            return Some((lod, total, true));
        }
        Some((lod, total, false))
    }

    /// The Bubble's frame (the dome's own): glow from the ripples and the
    /// hum's volume and pitch from the phase.
    pub fn frame(&mut self, alpha: f32, camera: Vec3) -> Option<DomeFrame> {
        let (lod, total, remove) = self.frame_common(alpha, camera, self.at)?;
        self.glow = ripple_glow(total);
        let mut gain = 1.0;
        let mut hum = [0.2f32, 0.75];
        match self.phase {
            Phase::Standing => hum = [f32::from_bits(0x3e4c_cccd), 0.75],
            Phase::Growing => {
                gain = self.shown_share;
                if self.shown_share < 0.25 {
                    let x = self.shown_share * 4.0;
                    let x = x * (2.0 - x);
                    hum = [x * 0.2, 1.0 - x * 0.25];
                }
            }
            Phase::Ending => {}
            _ => {
                gain = 1.0 - self.shown_share;
                hum = [
                    (2.0 - (1.5 - self.shown_share * 3.5).abs()) * 0.4,
                    (1.0 - self.shown_share) * 0.75,
                ];
            }
        }
        let gain = gain * self.glow.abs();
        hum[0] += gain * 1.0;
        hum[1] += gain * 2.25;
        Some(DomeFrame {
            lod,
            fade: self.fade(),
            hum,
            remove,
        })
    }

    /// The response of a character whose parts the dome pushed: `offset` is
    /// the character's place less the dome's, `scale` its size factor.
    /// `caster` when the character is the dome's caster (only the Bubble
    /// tests it). A limp body (flag 1, a guess at its meaning) only plays the
    /// hit sound, when its flag 0x10000
    /// is set.
    pub fn contact(&mut self, hit: &DomeHit, now_ms: u32, roll: &mut Roll) -> Option<Contact> {
        let volume = if f64::from(self.ripples.peak) > 0.15 {
            let r = roll.unit();
            (f64::from(self.ripples.peak.sqrt()) * (f64::from(r) * 0.05 + 0.1)) as f32
        } else {
            0.0
        };
        let mut contact = Contact::default();
        if hit.limp {
            if hit.struck_flag && self.quiet_until_ms < now_ms && f64::from(volume) > 0.05 {
                contact.sound = Some([volume, (f64::from(roll.unit()) * 0.6 + 0.6) as f32]);
                self.quiet_until_ms = now_ms.wrapping_add(400);
            }
            return Some(contact);
        }
        let d2 = hit.offset.length_squared_zyx();
        if hit.caster {
            let edge = self.radius - hit.scale * 30.0;
            if !(edge * edge < d2) {
                return None;
            }
            contact.shove = Some(flat(hit.offset * (-1.0 / d2.sqrt())));
        } else {
            let edge = hit.scale * 20.0 + self.radius;
            if !(d2 < edge * edge) {
                return None;
            }
            let d = d2.sqrt();
            contact.shove = Some(flat(hit.offset * (1.0 / d)));
            contact.knocks = true;
            contact.balance_distance = Some(d);
        }
        if self.quiet_until_ms < now_ms && f64::from(volume) > 0.05 {
            contact.sound = Some([volume, (f64::from(roll.unit()) * 0.4 + 0.6) as f32]);
            self.quiet_until_ms = now_ms.wrapping_add(400);
        }
        Some(contact)
    }
}

/// A character the dome pushed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DomeHit {
    /// The character's place less the dome's centre.
    pub offset: Vec3,
    /// The character's size factor.
    pub scale: f32,
    /// The character is the dome's caster.
    pub caster: bool,
    /// The body's flag 1 is set (guess: a limp body), in which case only
    /// its flag 0x10000 (`struck_flag`) lets the hit sound play and the
    /// character's own struck call run.
    pub limp: bool,
    pub struck_flag: bool,
}

/// What a contact asks of the character.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Contact {
    /// The shove's direction (flattened, not renormalised); its strength is
    /// [`CONTACT_PUSH`].
    pub shove: Option<Vec3>,
    /// The hit sound's volume and pitch.
    pub sound: Option<[f32; 2]>,
    /// The character counts a knock and its brace is released.
    pub knocks: bool,
    /// The distance its balance drain is taken at.
    pub balance_distance: Option<f32>,
}

/// The balance a pushed character keeps: above 0.25 it loses `step` within
/// 100 cm, `(d − 100)·0.0001 + step` beyond (double precision); 0.002 for
/// the dome, 0.001 for the Barrier.
pub fn balance_after(balance: f32, distance: f32, step: f64) -> f32 {
    if !(balance > 0.25) {
        return balance;
    }
    let loss = if distance > 100.0 {
        f64::from(distance - 100.0) * 0.0001 + step
    } else {
        step
    };
    (f64::from(balance) - loss) as f32
}

fn flat(v: Vec3) -> Vec3 {
    Vec3::new(v.x, 0.0, v.z)
}

trait Length {
    fn length_squared_zyx(self) -> f32;
}

impl Length for Vec3 {
    /// `(x² + y²) + z²`, the order the keep-out sums.
    fn length_squared_zyx(self) -> f32 {
        let xy = self.y * self.y + self.x * self.x;
        self.z * self.z + xy
    }
}

/// The cast sound's and the Barrier's height: the higher of the caster's
/// height plus 10 and the ground plus 100.
pub fn sound_height(caster_y: f32, ground_y: f32) -> f32 {
    let body = caster_y + 10.0;
    let floor = ground_y + 100.0;
    if body < floor { floor } else { body }
}

/// A body part pushed by a shell: the push (in the dome's frame unless
/// noted), the shell point the ripple opens at and its strength.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PartPush {
    pub push: Vec3,
    pub ripple_at: Vec3,
    pub strength: f32,
}

/// The Barrier.
#[derive(Debug, Clone, PartialEq)]
pub struct Barrier {
    pub dome: Dome,
    /// The followed height and its value a tick before.
    pub height: f32,
    pub previous_height: f32,
    /// The drawn radius, cm.
    pub drawn_radius: f32,
    /// The power's focus rate a tick and its hit value.
    pub rate: f32,
    pub hit: f32,
    /// The place and facing at the last two snapshots and the tick's own
    /// (the snapshot's next pair), with the snapshot's frame.
    pub pose: BarrierPose,
    /// The texture's random offsets.
    pub tex_seed: [f32; 2],
}

/// What a Barrier tick asks of the world.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BarrierTick {
    /// Focus drained from the caster this tick.
    pub drain: Option<f32>,
    /// The power was used (the `PowerUsed` event).
    pub power_used: bool,
}

impl Barrier {
    /// The cast: volume 0.2, the dome around the caster's place at the
    /// followed height.
    pub fn cast(
        radius: f32,
        caster: Vec3,
        ground_y: f32,
        start_ms: u32,
        random: &mut Mt19937,
    ) -> (Self, Sound) {
        let height = sound_height(caster.y, ground_y);
        let at = Vec3::new(caster.x, height, caster.z);
        let (dome, sound) = Dome::cast(
            radius,
            at,
            start_ms,
            caster,
            ground_y,
            BARRIER_CAST_VOLUME,
            random,
        );
        let tex_seed = dome.tex_pos;
        let barrier = Self {
            dome,
            height,
            previous_height: height,
            drawn_radius: 0.0,
            rate: 0.0,
            hit: 0.0,
            pose: BarrierPose {
                first: [0.0; 3],
                first_previous: [0.0; 3],
                second: [0.0; 3],
                second_previous: [0.0; 3],
                next_first: at.to_array(),
                next_second: [0.0; 3],
                frame: 0,
            },
            tex_seed,
        };
        (barrier, sound)
    }

    /// One tick at the game clock `now_ms`, the caster at `caster` over the
    /// ground at `ground_y`, facing `facing`.
    pub fn tick(&mut self, caster: Vec3, ground_y: f32, facing: Vec3, now_ms: u32) -> BarrierTick {
        let mut out = BarrierTick::default();
        self.dome.ripples.tick();
        let target = sound_height(caster.y, ground_y);
        let previous = self.previous_height;
        self.previous_height = self.height;
        let carried = f64::from(self.height - previous) * 0.95 + f64::from(self.height);
        self.height = (f64::from(target - self.height) * 0.003 + carried) as f32;
        self.dome.at = Vec3::new(caster.x, self.height, caster.z);
        self.pose.next_first = self.dome.at.to_array();
        self.pose.next_second = facing.to_array();
        self.dome.previous_share = self.dome.share;
        let dome = &mut self.dome;
        if dome.phase < Phase::Ending {
            out.drain = Some(if dome.phase == Phase::Growing {
                self.rate
            } else {
                self.rate * 0.5
            });
        }
        let elapsed = i64::from(now_ms) - i64::from(dome.since_ms);
        if dome.phase == Phase::Shrinking {
            dome.share = (elapsed as f64 * BARRIER_SHRINK_PER_MS) as f32;
            if dome.share > 1.0 {
                dome.phase = Phase::Gone;
                dome.share = 1.0;
            }
            let s = dome.share;
            self.drawn_radius = (f64::from(1.0 - s * s) * (f64::from(dome.radius) * 0.9)) as f32;
        } else if dome.phase != Phase::Standing {
            if u64::from(dome.since_ms) + BARRIER_GROW_MS > u64::from(now_ms) {
                dome.share = (elapsed as f64 * BARRIER_GROW_PER_MS) as f32;
                if dome.share > 1.0 {
                    dome.share = 1.0;
                    if dome.phase == Phase::Growing {
                        dome.phase = Phase::Standing;
                        out.power_used = true;
                    }
                }
                let s = (2.0 - dome.share) * dome.share;
                self.drawn_radius = (f64::from(s) * (f64::from(dome.radius) * 0.9)) as f32;
            }
            if dome.phase == Phase::Ending {
                dome.phase = Phase::Shrinking;
                dome.share = 1.0 - dome.share;
                dome.previous_share = dome.share;
                dome.shown_share = dome.share;
                let back = (dome.share * BARRIER_END_MS).round_ties_even() as i64 as u32;
                dome.since_ms = now_ms.wrapping_sub(back);
            }
        }
        out
    }

    /// The pose snapshot before a tick ([`advance_barrier_pose`]): the
    /// current place and facing become the previous, the tick's become the
    /// current, and the frame word is stored.
    pub fn snapshot(&mut self, frame: i32) {
        self.pose = advance_barrier_pose(&self.pose, frame);
    }

    /// The place the frame draws at the frame's fraction `t`: from the
    /// previous snapshot toward the current one.
    pub fn frame_place(&self, t: f32) -> Vec3 {
        Vec3::from_array(barrier_lerp(self.pose.first_previous, self.pose.first, t))
    }

    /// The node's place a tick takes from its setup's frame: the fraction
    /// is the setup's frame less the snapshot's plus its sub-frame part
    /// ([`update_barrier_position`]).
    pub fn tick_place(&self, setup_frame: i32, sub_frame: f32) -> Vec3 {
        let (at, _) = update_barrier_position(
            self.pose.first_previous,
            self.pose.first,
            setup_frame,
            self.pose.frame,
            sub_frame,
        );
        Vec3::from_array(at)
    }

    /// The frame: the cap's place, facing, centre and texture position,
    /// then the dome's frame values with the Barrier's glow and hum. `node_time`
    /// is the node's own clock word the texture's second coordinate follows.
    pub fn frame(&mut self, alpha: f32, camera: Vec3, node_time: f32) -> Option<BarrierFrame> {
        let at = self.frame_place(alpha);
        let facing = normalize(Vec3::from_array(barrier_lerp(
            self.pose.second_previous,
            self.pose.second,
            alpha,
        )));
        let yaw = planar_yaw(facing);
        self.dome.tex_pos = [
            (f64::from(yaw) * 0.159_154_943_091_895_35 + f64::from(self.tex_seed[0]) + 0.5) as f32,
            (f64::from(node_time) * 0.001 + f64::from(self.tex_seed[1])) as f32,
        ];
        let centre = at + facing * BARRIER_FORWARD_CM;
        self.dome.glow = self.hit * 4.0;
        let (lod, _, remove) = self.dome.frame_common(alpha, camera, centre)?;
        let shown = self.dome.shown_share;
        let mut hum = [0.2f32, 0.75];
        match self.dome.phase {
            Phase::Standing => hum = [f32::from_bits(0x3e4c_cccd), 0.75],
            Phase::Growing if shown < 0.25 => {
                let x = shown * 4.0;
                let x = x * (2.0 - x);
                hum = [x * 0.2, 1.0 - x * 0.25];
            }
            Phase::Growing | Phase::Ending => {}
            _ => {
                hum = [
                    (1.0 - (0.5 - shown * 1.5).abs()) * 0.4,
                    (1.0 - shown) * 0.75,
                ];
            }
        }
        hum[0] += (self.hit * 4.0).abs();
        hum[1] += (self.hit * 9.0).abs();
        Some(BarrierFrame {
            dome: DomeFrame {
                lod,
                fade: self.dome.fade(),
                hum,
                remove,
            },
            at,
            facing,
            centre,
            bound_radius: bound_radius(),
            hum_at: centre + facing * BARRIER_HUM_AHEAD_CM,
        })
    }

    /// A body part's push, `local` its centre in the cap's frame (z along
    /// the facing) and `radius` its own; `mass` the part's body's mass
    /// figure. The push is in the cap's frame.
    pub fn part_push(&self, local: Vec3, radius: f32, mass: f32) -> Option<PartPush> {
        let r = self.dome.radius;
        let d2 = local.length_squared_zyx();
        let reach = r + radius;
        if reach * reach < d2 || r * 0.5 > local.z + radius {
            return None;
        }
        let outer = radius + 10.0;
        let flat2 = local.y * local.y + local.x * local.x;
        let side = self.drawn_radius - radius;
        let inner = (r - 10.0) - outer;
        if side * side < flat2 || inner * inner > d2 {
            return None;
        }
        let d = d2.sqrt();
        let surface = local * ((r - 10.0) / d);
        let mut push = local - surface;
        let l2 = push.length_squared_zyx();
        if l2 < EPSILON {
            return None;
        }
        let l = l2.sqrt();
        push *= (f64::from(outer / l - 1.0) * 0.1) as f32;
        if flat2 > EPSILON {
            let k = (0.3 / f64::from(flat2.sqrt())) as f32;
            push.x += local.x * k;
            push.y += local.y * k * 0.25;
        }
        push *= mass / (mass + 1.0);
        Some(PartPush {
            push,
            ripple_at: local * (r / d),
            strength: l * 0.5,
        })
    }

    /// The response of a character after its parts were pushed (the
    /// Barrier never tests its caster).
    pub fn contact(&mut self, hit: &DomeHit, now_ms: u32, roll: &mut Roll) -> Option<Contact> {
        let mut contact = Contact::default();
        let quiet = self.dome.quiet_until_ms < now_ms;
        if hit.limp {
            if hit.struck_flag && quiet {
                let pitch = (f64::from(roll.unit()) * 0.6 + 0.6) as f32;
                let volume = (f64::from(roll.unit()) * 0.2 + 0.3) as f32;
                contact.sound = Some([volume, pitch]);
                self.dome.quiet_until_ms = now_ms.wrapping_add(400);
            }
            return Some(contact);
        }
        let d2 = hit.offset.length_squared_zyx();
        let edge = hit.scale * 20.0 + self.dome.radius;
        if !(d2 < edge * edge) {
            return None;
        }
        let d = d2.sqrt();
        contact.shove = Some(flat(hit.offset * (1.0 / d)));
        contact.knocks = true;
        contact.balance_distance = Some(d);
        if quiet {
            let pitch = (f64::from(roll.unit()) * 0.4 + 0.6) as f32;
            let volume = (f64::from(roll.unit()) * 0.2 + 0.1) as f32;
            contact.sound = Some([volume, pitch]);
            let gap = ((u64::from(roll.step()) * 200) >> 32) as u32;
            self.dome.quiet_until_ms = gap.wrapping_add(now_ms).wrapping_add(300);
        }
        Some(contact)
    }
}

/// The smallest squared length the keep-out divides by.
const EPSILON: f32 = f32::from_bits(0x33ff_ffd7);

/// The Barrier cap's bounding radius: the length of its 85 cm width and
/// 25 cm depth, `√(25² + 85²)`.
pub fn bound_radius() -> f32 {
    let x = BARRIER_DRAW_SCALE.x;
    let z = BARRIER_DRAW_SCALE.z;
    (z * z + x * x).sqrt()
}

/// What a Barrier's frame hands the draw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarrierFrame {
    pub dome: DomeFrame,
    /// The cap's place, facing and drawn centre (75 cm ahead).
    pub at: Vec3,
    pub facing: Vec3,
    pub centre: Vec3,
    pub bound_radius: f32,
    /// The hum's place.
    pub hum_at: Vec3,
}

/// The Bubble.
#[derive(Debug, Clone, PartialEq)]
pub struct Bubble {
    pub dome: Dome,
}

impl Bubble {
    /// The cast: volume 0.3, the dome around `at`.
    pub fn cast(
        radius: f32,
        at: Vec3,
        start_ms: u32,
        caster: Vec3,
        ground_y: f32,
        random: &mut Mt19937,
    ) -> (Self, Sound) {
        let (dome, sound) = Dome::cast(
            radius,
            at,
            start_ms,
            caster,
            ground_y,
            BUBBLE_CAST_VOLUME,
            random,
        );
        (Self { dome }, sound)
    }

    /// One tick. Returns the break sound when the ending starts, and whether
    /// the grow finished (the caster's hook is then cleared).
    pub fn tick(&mut self, now_ms: u32, roll: &mut Roll) -> (Option<Sound>, bool) {
        self.dome.ripples.tick();
        let dome = &mut self.dome;
        dome.previous_share = dome.share;
        let elapsed = i64::from(now_ms) - i64::from(dome.since_ms);
        match dome.phase {
            Phase::Growing => {
                dome.share = (elapsed as f64 * BUBBLE_GROW_PER_MS) as f32;
                if dome.share > 1.0 {
                    dome.share = 1.0;
                    dome.phase = Phase::Standing;
                    return (None, true);
                }
            }
            Phase::Ending => {
                dome.phase = Phase::Shrinking;
                dome.share = 1.0 - dome.share;
                dome.previous_share = dome.share;
                dome.shown_share = dome.share;
                let back = (dome.share * BUBBLE_END_MS).round_ties_even() as i64 as u32;
                dome.since_ms = now_ms.wrapping_sub(back);
                let pitch = (f64::from(roll.unit()) * 0.2 + 0.6) as f32;
                return (
                    Some(Sound {
                        at: dome.at,
                        volume: BREAK_VOLUME,
                        pitch,
                    }),
                    false,
                );
            }
            Phase::Shrinking => {
                dome.share = (elapsed as f64 * BUBBLE_SHRINK_PER_MS) as f32;
                if dome.share > 1.0 {
                    dome.phase = Phase::Gone;
                    dome.share = 1.0;
                }
            }
            _ => {}
        }
        (None, false)
    }

    /// One of the caster's own parts, `local` its centre in the bubble's
    /// frame: a part reaching the inner surface (10 cm in) is pushed back in
    /// by a tenth of the overlap over the distance, and ripples with the
    /// overlap. The push is applied without the bubble's rotation.
    pub fn own_part_push(&self, local: Vec3, radius: f32) -> Option<PartPush> {
        let r = self.dome.radius;
        let d2 = local.length_squared_zyx();
        let inner = (r - 10.0) - radius;
        if inner * inner > d2 {
            return None;
        }
        let d = d2.sqrt();
        let overlap = ((r - 10.0) - d) - radius;
        Some(PartPush {
            push: local * (f64::from(overlap / d) * 0.1) as f32,
            ripple_at: local * (r / d),
            strength: -overlap,
        })
    }

    /// Whether another body (its bound centre and radius) can reach the
    /// bubble's parts test.
    pub fn reaches(&self, centre: Vec3, bound: f32) -> bool {
        let d2 = (centre - self.dome.at).length_squared_zyx();
        let edge = bound + self.dome.radius;
        !(edge * edge < d2)
    }

    /// Another body's part, `local` its centre in the bubble's frame and
    /// `mass` its body's mass figure: a part the shell covers (its bottom
    /// under `(2s² − 1)·r`, `s` the share, flipped while shrinking) is pushed
    /// out by `(0.1 − 0.01·min(overlap, 1))·d`, scaled by the mass factor.
    /// The push is in the bubble's frame (the caller turns it to the world).
    pub fn other_part_push(&self, local: Vec3, radius: f32, mass: f32) -> Option<PartPush> {
        let dome = &self.dome;
        let r = dome.radius;
        let s = if dome.phase < Phase::Shrinking {
            dome.share
        } else {
            1.0 - dome.share
        };
        let cover = (s * s * 2.0 - 1.0) * r;
        let factor = (f64::from(mass / (mass + 1.0)) * 0.05) as f32;
        let factor = if f64::from(factor) > 0.01 {
            f32::from_bits(0x3c23_d70a)
        } else {
            factor
        };
        let d2 = local.length_squared_zyx();
        let reach = r + radius;
        if reach * reach < d2 || cover < local.y - radius {
            return None;
        }
        let d = d2.sqrt();
        let overlap = radius - (d - r);
        let overlap = if overlap > 1.0 { 1.0 } else { overlap };
        let strength = ((0.1 - f64::from(overlap) * 0.01) * f64::from(d)) as f32;
        Some(PartPush {
            push: local * (strength / d * factor),
            ripple_at: local * (r / d),
            strength,
        })
    }
}

/// A dome's double-sided mesh: the polyhedron's points scaled to `radius`,
/// then every face reversed, then every face as built.
pub fn dome_mesh(poly: &Polyhedron, radius: f32) -> (Vec<[f32; 3]>, Vec<[u32; 3]>) {
    let points = poly.points.iter().map(|p| p.map(|c| c * radius)).collect();
    let reversed = poly.faces.iter().map(|&[a, b, c]| [c, b, a]);
    let faces = reversed.chain(poly.faces.iter().copied()).collect();
    (points, faces)
}

/// The Bubble's polyhedron: the unit icosahedron subdivided three times.
pub fn bubble_polyhedron() -> Polyhedron {
    let mut poly = Polyhedron::icosahedron(1.0);
    for _ in 0..3 {
        poly = poly.subdivided();
    }
    poly
}

/// The Barrier's polyhedron: the icosahedron's unit top cap with each
/// point's y and z exchanged, subdivided three times.
pub fn barrier_polyhedron() -> Polyhedron {
    let mut poly = Polyhedron::cap(1.0);
    for point in &mut poly.points {
        point.swap(1, 2);
    }
    for _ in 0..3 {
        poly = poly.subdivided();
    }
    poly
}

#[cfg(test)]
mod tests {
    use super::*;

    fn random() -> Mt19937 {
        Mt19937::new(5489)
    }

    #[test]
    fn a_strike_merges_with_the_nearest_target_and_opens_otherwise() {
        let mut r = Ripples::default();
        r.strike(Vec3::new(100.0, 0.0, 0.0), 9.0);
        assert_eq!(r.peak, 9.0);
        assert_eq!(r.list.len(), 1);
        // Capped at 5; the sum starts at the point with one strike.
        assert_eq!(r.list[0].strength, 5.0);
        assert_eq!(r.list[0].sum, Vec3::new(100.0, 0.0, 0.0));
        // Within the 30 cm reach of the target: summed, the larger kept.
        r.strike(Vec3::new(110.0, 0.0, 0.0), 1.0);
        assert_eq!(r.list.len(), 1);
        assert_eq!(r.list[0].strikes, 2);
        assert_eq!(r.list[0].sum, Vec3::new(210.0, 0.0, 0.0));
        assert_eq!(r.list[0].strength, 5.0);
        // The tick takes the mean as the target (not shown yet).
        r.tick();
        assert_eq!(r.list[0].target, Vec3::new(105.0, 0.0, 0.0));
        assert_eq!(r.reach, 31.25);
        // Far away: a second ripple.
        r.strike(Vec3::new(500.0, 0.0, 0.0), 1.0);
        assert_eq!(r.list.len(), 2);
    }

    #[test]
    fn a_faded_ripple_is_replaced_by_the_last() {
        let mut r = Ripples::default();
        r.strike(Vec3::new(0.0, 0.0, 0.0), 0.06);
        r.strike(Vec3::new(500.0, 0.0, 0.0), 1.0);
        r.strike(Vec3::new(1000.0, 0.0, 0.0), 2.0);
        r.tick();
        assert_eq!(r.list.len(), 2);
        assert_eq!(r.list[0].target.x, 1000.0);
        // Strength less 0.05 in double precision.
        assert_eq!(r.list[0].strength, (2.0f64 - 0.05) as f32);
        assert_eq!(r.list[0].shown, f32::from_bits(0x3d23_d70a));
    }

    #[test]
    fn the_glow_and_the_shader_points_follow_the_shown_values() {
        let mut r = Ripples::default();
        r.strike(Vec3::new(1.0, 2.0, 3.0), 1.0);
        r.tick();
        let total = r.copy_points();
        assert_eq!(total, r.list[0].shown);
        assert_eq!(r.points[0][3], r.list[0].shown);
        assert_eq!(r.points[1][3], 0.0);
        let x = total * 0.04;
        assert_eq!(ripple_glow(total), (2.0 - x) * x * 0.5);
        assert_eq!(ripple_glow(30.0), 0.5);
    }

    #[test]
    fn the_cast_sound_stands_at_the_higher_of_body_and_floor() {
        let (dome, sound) = Dome::cast(
            150.0,
            Vec3::ZERO,
            10,
            Vec3::new(5.0, 120.0, 7.0),
            0.0,
            BARRIER_CAST_VOLUME,
            &mut random(),
        );
        assert_eq!(sound.at, Vec3::new(5.0, 130.0, 7.0));
        assert_eq!(dome.phase, Phase::Growing);
        assert_eq!(
            (dome.radius, dome.since_ms, dome.ripples.reach),
            (150.0, 10, 30.0)
        );
        assert_eq!(sound_height(20.0, 0.0), 100.0);
        assert_eq!(sound.volume, 0.2);
    }

    fn barrier() -> Barrier {
        let (mut b, _) = Barrier::cast(100.0, Vec3::new(0.0, 0.0, 0.0), 0.0, 1000, &mut random());
        b.rate = 0.002;
        b
    }

    #[test]
    fn the_node_attachment_takes_the_dome_and_its_step() {
        let mut dome = Dome::cast(
            30.0,
            Vec3::ZERO,
            0,
            Vec3::ZERO,
            0.0,
            BARRIER_CAST_VOLUME,
            &mut random(),
        )
        .0;
        assert_eq!(dome.node, DomeNode::default());
        dome.attach_node();
        assert!(dome.node.attached && dome.node.owner && dome.node.frame_hook);
        assert_eq!(dome.node.flags, 0x0001_0000);
        dome.attach_node();
        assert_eq!(dome.node.flags, 0x0001_0000, "the bit is set once");
        // A dome with a light and a material frees both on the real delete,
        // runs its clear hook and frees itself; a plain free keeps the
        // light and the material and only lets the base object go.
        dome.light = true;
        dome.material = true;
        assert_eq!(
            dome.destroy(true),
            DomeTeardown {
                cleared: true,
                light: true,
                material: true,
                object: true,
            }
        );
        assert!(dome.removed && !dome.light && !dome.material);
        assert!(!dome.node.attached);
        let mut plain = Dome::cast(
            30.0,
            Vec3::ZERO,
            0,
            Vec3::ZERO,
            0.0,
            BARRIER_CAST_VOLUME,
            &mut random(),
        )
        .0;
        plain.light = true;
        assert_eq!(
            plain.destroy(false),
            DomeTeardown {
                cleared: false,
                light: true,
                material: false,
                object: false,
            }
        );
        assert!(!plain.light && plain.removed);
    }

    #[test]
    fn the_barrier_grows_only_inside_its_800_ms_window() {
        let mut b = barrier();
        let t = b.tick(Vec3::ZERO, 0.0, Vec3::Z, 1400);
        assert_eq!(t.drain, Some(0.002));
        assert_eq!(b.dome.share, (400.0f64 * 0.00125) as f32);
        assert_eq!(
            b.drawn_radius,
            (f64::from((2.0 - 0.5f32) * 0.5) * (100.0f64 * 0.9)) as f32
        );
        b.tick(Vec3::ZERO, 0.0, Vec3::Z, 1799);
        let last = b.dome.share;
        assert!(last < 1.0);
        // From 800 ms on the share stays put: no PowerUsed from the tick.
        let t = b.tick(Vec3::ZERO, 0.0, Vec3::Z, 1900);
        assert!(!t.power_used);
        assert_eq!(b.dome.share, last);
        assert_eq!(b.dome.phase, Phase::Growing);
    }

    #[test]
    fn the_barrier_ends_from_its_share_and_shrinks_at_0_002() {
        let mut b = barrier();
        b.tick(Vec3::ZERO, 0.0, Vec3::Z, 1400);
        b.dome.phase = Phase::Ending;
        let t = b.tick(Vec3::ZERO, 0.0, Vec3::Z, 1500);
        assert_eq!(t.drain, None);
        assert_eq!(b.dome.phase, Phase::Shrinking);
        // The grow ran first (share 0.625), then the flip.
        let share = 1.0 - (500.0f64 * 0.00125) as f32;
        assert_eq!(b.dome.share, share);
        assert_eq!(
            b.dome.since_ms,
            1500 - (share * 500.0).round_ties_even() as u32
        );
        b.tick(Vec3::ZERO, 0.0, Vec3::Z, 2000);
        assert_eq!(b.dome.phase, Phase::Gone);
    }

    #[test]
    fn the_barrier_height_follows_with_its_spring() {
        let mut b = barrier();
        assert_eq!(b.height, 100.0);
        b.tick(Vec3::new(0.0, 200.0, 0.0), 0.0, Vec3::Z, 1012);
        // target 210: h = (210 − 100)·0.003 + (0·0.95 + 100).
        assert_eq!(b.height, (110.0f64 * 0.003 + 100.0) as f32);
        let h1 = b.height;
        b.tick(Vec3::new(0.0, 200.0, 0.0), 0.0, Vec3::Z, 1024);
        let carried = f64::from(h1 - 100.0) * 0.95 + f64::from(h1);
        assert_eq!(b.height, (f64::from(210.0 - h1) * 0.003 + carried) as f32);
    }

    #[test]
    fn the_barrier_frame_draws_the_cap_ahead_with_its_bound() {
        let mut b = barrier();
        b.tick(Vec3::ZERO, 0.0, Vec3::Z, 1012);
        b.snapshot(1);
        b.snapshot(2);
        // Two snapshots: the place stands still, the facing is +z.
        assert_eq!(b.frame_place(0.5), b.dome.at);
        assert_eq!(b.tick_place(4, 0.5), b.dome.at);
        let frame = b.frame(0.5, Vec3::new(0.0, 0.0, 1000.0), 0.0).unwrap();
        // The facing is the engine's approximate normalisation of +z.
        assert_eq!(frame.centre, frame.at + frame.facing * 75.0);
        assert!((frame.facing - Vec3::Z).length() < 1e-3);
        assert_eq!(frame.bound_radius, (625.0f32 + 7225.0).sqrt());
        assert_eq!(frame.hum_at, frame.centre + frame.facing * 80.0);
        // The growing hum under a quarter share: the blended share is half
        // of 12 ms of growth.
        let shown = 0.5 * 0.0 + b.dome.share * 0.5;
        let x = shown * 4.0;
        let x = x * (2.0 - x);
        assert_eq!(frame.dome.hum, [x * 0.2, 1.0 - x * 0.25]);
        assert_eq!(b.dome.glow, 0.0);
    }

    #[test]
    fn a_barrier_part_is_pushed_out_of_the_cap() {
        let mut b = barrier();
        b.drawn_radius = 90.0;
        // Straight ahead, 95 cm out, radius 5: inside reach, outside 80.
        let push = b.part_push(Vec3::new(0.0, 0.0, 95.0), 5.0, 1.0).unwrap();
        let surface = Vec3::new(0.0, 0.0, 95.0) * (90.0 / 95.0);
        let v = Vec3::new(0.0, 0.0, 95.0) - surface;
        let l = v.length();
        assert_eq!(push.strength, l * 0.5);
        assert_eq!(push.ripple_at, Vec3::new(0.0, 0.0, 95.0) * (100.0 / 95.0));
        let expected = v * ((f64::from(15.0 / l - 1.0) * 0.1) as f32) * 0.5;
        assert!((push.push - expected).length() < 1e-6);
        // Below half the radius, or sideways past the drawn radius: none.
        assert!(b.part_push(Vec3::new(0.0, 0.0, 40.0), 5.0, 1.0).is_none());
        assert!(b.part_push(Vec3::new(90.0, 0.0, 50.0), 5.0, 1.0).is_none());
    }

    #[test]
    fn the_bubble_grows_breaks_and_shrinks() {
        let (mut bubble, sound) =
            Bubble::cast(200.0, Vec3::ZERO, 0, Vec3::ZERO, 0.0, &mut random());
        assert_eq!(sound.volume, BUBBLE_CAST_VOLUME);
        let mut roll = Roll(1);
        bubble.tick(600, &mut roll);
        assert_eq!(bubble.dome.share, (600.0 * BUBBLE_GROW_PER_MS) as f32);
        assert_eq!(bubble.tick(1201, &mut roll), (None, true));
        assert_eq!(bubble.dome.phase, Phase::Standing);
        bubble.dome.phase = Phase::Ending;
        let (broke, _) = bubble.tick(2000, &mut roll);
        assert_eq!(broke.unwrap().volume, BREAK_VOLUME);
        assert_eq!(bubble.dome.share, 0.0);
        bubble.tick(2701, &mut roll);
        assert_eq!(bubble.dome.phase, Phase::Gone);
    }

    #[test]
    fn the_bubble_holds_its_caster_in_and_others_out() {
        let (bubble, _) = Bubble::cast(100.0, Vec3::ZERO, 0, Vec3::ZERO, 0.0, &mut random());
        // The caster's part 88 cm out with radius 5 reaches the 85 cm surface.
        let own = bubble
            .own_part_push(Vec3::new(88.0, 0.0, 0.0), 5.0)
            .unwrap();
        let overlap = (90.0f32 - 88.0) - 5.0;
        assert_eq!(own.strength, -overlap);
        assert_eq!(own.push.x, 88.0 * (f64::from(overlap / 88.0) * 0.1) as f32);
        assert!(
            bubble
                .own_part_push(Vec3::new(50.0, 0.0, 0.0), 5.0)
                .is_none()
        );
        // Another body's part on a growing shell: covered only below
        // (2s² − 1)·r, s = 0 at the cast.
        assert!(
            bubble
                .other_part_push(Vec3::new(98.0, 0.0, 0.0), 5.0, 1.0)
                .is_none()
        );
        let low = bubble
            .other_part_push(Vec3::new(0.0, -98.0, 0.0), 5.0, 1.0)
            .unwrap();
        let d = 98.0f32;
        let strength = ((0.1 - 1.0f64 * 0.01) * f64::from(d)) as f32;
        assert_eq!(low.strength, strength);
        let factor = f32::from_bits(0x3c23_d70a);
        assert_eq!(low.push.y, -98.0 * (strength / d * factor));
    }

    #[test]
    fn a_contact_shoves_and_drains_balance() {
        let (mut bubble, _) = Bubble::cast(100.0, Vec3::ZERO, 0, Vec3::ZERO, 0.0, &mut random());
        bubble.dome.ripples.peak = 4.0;
        let hit = DomeHit {
            offset: Vec3::new(100.0, 50.0, 0.0),
            scale: 1.0,
            caster: false,
            limp: false,
            struck_flag: false,
        };
        let mut roll = Roll(9);
        let c = bubble.dome.contact(&hit, 100, &mut roll).unwrap();
        let d = (100.0f32 * 100.0 + 50.0 * 50.0).sqrt();
        assert_eq!(c.shove, Some(Vec3::new(100.0 / d, 0.0, 0.0)));
        assert!(c.knocks);
        assert!(c.sound.is_some());
        assert_eq!(bubble.dome.quiet_until_ms, 500);
        assert_eq!(balance_after(1.0, 50.0, 0.002), (1.0f64 - 0.002) as f32);
        assert_eq!(
            balance_after(1.0, 150.0, 0.001),
            (1.0f64 - (50.0f64 * 0.0001 + 0.001)) as f32
        );
        assert_eq!(balance_after(0.2, 150.0, 0.001), 0.2);
    }

    #[test]
    fn both_meshes_are_double_sided_reversed_first() {
        let poly = bubble_polyhedron();
        assert_eq!(poly.faces.len(), 1280);
        let (points, faces) = dome_mesh(&poly, 100.0);
        assert_eq!(faces.len(), 2560);
        let [a, b, c] = poly.faces[0];
        assert_eq!(faces[0], [c, b, a]);
        assert_eq!(faces[1280], [a, b, c]);
        let p = points[0];
        assert!(((p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt() - 100.0).abs() < 0.01);
        let cap = barrier_polyhedron();
        assert!(cap.points.iter().all(|p| p[2] >= -1e-3));
    }
}
