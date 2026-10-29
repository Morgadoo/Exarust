//! Wounds that bleed, as the original queues them.
//!
//! A blow bleeds when the struck fighter's race bleeds and the wound's
//! size R (the blow's lasting damage on the struck part, its multiplier
//! applied, before the red bar's 0.75: [`crate::damage::Outcome::wound`])
//! exceeds 0.05. The wound then waits on the victim ([`WoundQueue`], at
//! most seven; a wound struck while all seven wait is dropped) with the
//! body that struck (the blade, or the fist), where on it the blow landed
//! (as weights in that body's tetrahedron, [`tetra_weights`]), the contact
//! point and R.
//!
//! Every body tick counts each waiting wound down; seven ticks after the
//! blow (84 ms) the first is taken out, one a tick, and bleeds
//! ([`Bleed`]): the striker's weights give where that point of it is now,
//! and how it moved on from the contact point sets the smear. The radius is
//! 30·R + 10 cm. The fallen keep counting, so the killing blow bleeds on
//! the corpse. Nothing bleeds on the striker: a fist stays clean.
//!
//! The weapon that deals a bleeding wound draws blood at once
//! ([`BladeBlood`]): half the wound's size R, up to 1, never washed off.
//! The blood stains the blade from its tip (`exa_character::blood`). Fists
//! draw none.
//!
//! A bleeding wound also sprays at once ([`Spray`]): a jet of droplets at
//! the contact point, on the struck part, running against the striker's
//! swing. Its strength is 120·R (200 once R exceeds 1.67,
//! [`spray_strength`]) and it sends round(0.3 × strength) droplets
//! ([`jet_droplets`]): 36·R, at most 60. The droplets fly and fall in
//! `exa_character::spray`.

use bevy::math::Affine3A;
use bevy::prelude::*;
use exa_physics::body::BodyTicks;

/// A wound bleeds when its size R exceeds this (compared in double
/// precision, as the original does).
pub const BLEED_FROM: f64 = 0.05;
/// Wounds that wait on a fighter at most; a wound struck while all of them
/// wait is dropped.
pub const WOUND_SLOTS: usize = 7;
/// Body ticks from the blow to the wound's bleeding (84 ms at 83 Hz).
pub const WOUND_DELAY: i32 = 7;
/// Blood a weapon draws from each bleeding wound it deals, as a share of
/// the wound's size R.
pub const BLADE_BLOOD_PER_WOUND: f32 = 0.5;
/// A bleeding wound's spray strength per unit of its size R.
pub const SPRAY_PER_WOUND: f32 = 120.0;
/// Above this size R (compared in double precision) a wound sprays at
/// [`SPRAY_FULL`] instead.
pub const SPRAY_FULL_FROM: f64 = 1.67;
/// The strongest spray.
pub const SPRAY_FULL: f32 = 200.0;
/// Droplets a jet sends per unit of its strength (a double).
pub const DROPLETS_PER_STRENGTH: f64 = 0.3;

/// The spray strength of a bleeding wound of size R: 120·R, or 200 when R
/// exceeds 1.67 (R widened to double first).
pub fn spray_strength(size: f32) -> f32 {
    if f64::from(size) > SPRAY_FULL_FROM {
        SPRAY_FULL
    } else {
        size * SPRAY_PER_WOUND
    }
}

/// The droplets a jet of strength `strength` sends: 0.3 × strength in
/// double precision, rounded to the nearest whole number, halves to even
/// (36·R for a wound of size R, 60 at most).
pub fn jet_droplets(strength: f32) -> i32 {
    (f64::from(strength) * DROPLETS_PER_STRENGTH).round_ties_even() as i32
}

/// A jet of blood from a bleeding wound, sent at the blow.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct Spray {
    /// The struck fighter's body model root.
    pub body: Entity,
    /// The struck body part's bone.
    pub bone: usize,
    /// Where the blow landed, in world space (metres).
    pub contact: Vec3,
    /// How the striking point moved at the blow (m/s): the jet runs
    /// against it.
    pub swing: Vec3,
    /// The droplets the jet sends ([`jet_droplets`]).
    pub droplets: i32,
}

/// The blood a held weapon has drawn, from 0 (clean) to 1, on the entity
/// holding its model (the striker of its hits). Each bleeding wound it
/// deals adds half the wound's size R at the blow ([`BladeBlood::draw`]);
/// nothing washes it off. Fists have none, so a punch never bloodies them.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub struct BladeBlood(pub f32);

impl BladeBlood {
    /// Draws the blood of a bleeding wound of size `size`: R·0.5 more, kept
    /// within 0..=1 by the weapon-blood list's own rule
    /// ([`crate::weapon_blood::add_blood`]).
    pub fn draw(&mut self, size: f32) {
        self.0 = crate::weapon_blood::add_blood(self.0, size * BLADE_BLOOD_PER_WOUND);
    }
}

/// Whether a wound of size R bleeds: R above 0.05, R widened to double
/// first (so the single-precision 0.05, a hair above it, bleeds).
pub fn bleeds(size: f32) -> bool {
    f64::from(size) > BLEED_FROM
}

/// The radius of a wound of size R, in metres: 30·R + 10 cm (11.5 cm at
/// the bleeding threshold, 19 cm at R = 0.3, 40 cm at R = 1).
pub fn wound_radius(size: f32) -> f32 {
    (30.0 * size + 10.0) / 100.0
}

/// The tetrahedron of a striking body that moves rigidly with `frame`: its
/// origin and the ends of its three axes.
pub fn frame_tetra(frame: &Affine3A) -> [Vec3; 4] {
    let o = Vec3::from(frame.translation);
    let m = frame.matrix3;
    [
        o,
        o + Vec3::from(m.x_axis),
        o + Vec3::from(m.y_axis),
        o + Vec3::from(m.z_axis),
    ]
}

/// The weights of `point` in the tetrahedron `tetra`: four numbers that sum
/// to 1 and give the point back as Σ wₖ·tₖ ([`tetra_point`]), wherever the
/// tetrahedron moves. `None` for a flat tetrahedron.
pub fn tetra_weights(point: Vec3, tetra: &[Vec3; 4]) -> Option<[f32; 4]> {
    let edges = Mat3::from_cols(
        tetra[1] - tetra[0],
        tetra[2] - tetra[0],
        tetra[3] - tetra[0],
    );
    let det = edges.determinant();
    if !det.is_finite() || det.abs() <= 1e-12 {
        return None;
    }
    let [a, b, c] = (edges.inverse() * (point - tetra[0])).to_array();
    Some([1.0 - a - b - c, a, b, c])
}

/// The point the weights `weights` give in the tetrahedron `tetra`.
pub fn tetra_point(weights: &[f32; 4], tetra: &[Vec3; 4]) -> Vec3 {
    tetra.iter().zip(weights).map(|(p, w)| *p * *w).sum()
}

/// A wound waiting to bleed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PendingWound {
    /// The body that struck: the blade (the entity holding the weapon
    /// model), or the fist.
    pub striker: Entity,
    /// Where on the striker the blow landed, as weights in its tetrahedron
    /// ([`frame_tetra`]); none when its frame was unknown.
    pub weights: Option<[f32; 4]>,
    /// Where the blow landed, in world space (metres), at the blow.
    pub contact: Vec3,
    /// Body ticks left before it bleeds.
    pub delay: i32,
    /// The wound's size R.
    pub size: f32,
}

/// The wounds waiting to bleed on a fighter (on its body model root), in
/// the order they were struck.
#[derive(Component, Debug, Clone, Default)]
pub struct WoundQueue(Vec<PendingWound>);

impl WoundQueue {
    /// Queues a wound of size `size` that `striker` dealt at `contact`, to
    /// bleed [`WOUND_DELAY`] ticks from now. Returns false, and drops it,
    /// when [`WOUND_SLOTS`] wounds already wait.
    pub fn push(
        &mut self,
        striker: Entity,
        weights: Option<[f32; 4]>,
        contact: Vec3,
        size: f32,
    ) -> bool {
        if self.0.len() >= WOUND_SLOTS {
            return false;
        }
        self.0.push(PendingWound {
            striker,
            weights,
            contact,
            delay: WOUND_DELAY,
            size,
        });
        true
    }

    /// One body tick: every waiting wound counts down, and the first is
    /// taken out when its delay is up (at most one a tick).
    pub fn tick(&mut self) -> Option<PendingWound> {
        for w in &mut self.0 {
            w.delay -= 1;
        }
        (self.0.first()?.delay <= 0).then(|| self.0.remove(0))
    }

    /// The waiting wounds, first to bleed first.
    pub fn pending(&self) -> &[PendingWound] {
        &self.0
    }
}

/// A wound bleeding now, on the struck fighter's body model root `body`:
/// what paints its blood.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct Bleed {
    pub body: Entity,
    /// Where the blow landed, in world space (metres).
    pub contact: Vec3,
    /// How the struck point of the striker moved on from the contact point
    /// since the blow (metres): the smear runs along it. Zero when the
    /// striker's frame is unknown.
    pub travel: Vec3,
    /// The wound's radius (metres), 30·R + 10 cm.
    pub radius: f32,
}

/// Counts every fighter's waiting wounds down by the body ticks run since
/// the last frame, and sends each wound whose delay is up as a [`Bleed`],
/// with how its striker moved on as this frame draws it.
pub(crate) fn count_down_wounds(
    ticks: Option<Res<BodyTicks>>,
    mut seen: Local<Option<u64>>,
    mut queues: Query<(Entity, &mut WoundQueue)>,
    transforms: Query<&GlobalTransform>,
    mut bleeds: MessageWriter<Bleed>,
) {
    let Some(ticks) = ticks else { return };
    let last = seen.replace(ticks.0).unwrap_or(ticks.0);
    let elapsed = ticks.0.saturating_sub(last);
    if elapsed == 0 {
        return;
    }
    for (body, mut queue) in &mut queues {
        for _ in 0..elapsed {
            if queue.0.is_empty() {
                break;
            }
            let Some(w) = queue.tick() else { continue };
            let now = w
                .weights
                .zip(transforms.get(w.striker).ok())
                .map(|(weights, g)| tetra_point(&weights, &frame_tetra(&g.affine())));
            let travel = now.map_or(Vec3::ZERO, |p| p - w.contact);
            debug!(
                "wound on {body:?} bleeds: R {:.3}, travel {:.0} cm",
                w.size,
                travel.length() * 100.0
            );
            bleeds.write(Bleed {
                body,
                contact: w.contact,
                travel,
                radius: wound_radius(w.size),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R bleeds above 0.05 in double precision: the single-precision 0.05
    /// (0.050 000 000 7) bleeds, the next value down does not. 30·R + 10 cm:
    /// 11.5 cm at the threshold, 19 cm at 0.3, 40 cm at 1.
    #[test]
    fn wound_sizes() {
        assert!(!bleeds(0.049_999_996));
        assert!(bleeds(0.05));
        assert!(bleeds(0.0501));
        assert!(!bleeds(0.0));
        for (r, cm) in [(0.05, 11.5), (0.3, 19.0), (1.0, 40.0)] {
            assert!((wound_radius(r) * 100.0 - cm).abs() < 1e-4, "{r}");
        }
    }

    /// A blade draws half of each wound's R and holds at most 1: the chest
    /// cut (R 0.457) leaves 0.2285, three of them 0.6855, a fifth soaks it.
    #[test]
    fn blades_draw_half_of_each_wound() {
        let mut blood = BladeBlood::default();
        blood.draw(0.457);
        assert!((blood.0 - 0.2285).abs() < 1e-6);
        blood.draw(0.457);
        blood.draw(0.457);
        assert!((blood.0 - 0.6855).abs() < 1e-6);
        blood.draw(0.457);
        assert!((blood.0 - 0.914).abs() < 1e-6);
        blood.draw(0.457);
        assert_eq!(blood.0, 1.0);
        blood.draw(0.06);
        assert_eq!(blood.0, 1.0);
        // The smallest bleeding wound, just over 0.05, adds just over 0.025.
        let mut faint = BladeBlood::default();
        faint.draw(0.0501);
        assert!((faint.0 - 0.025_05).abs() < 1e-7);
    }

    /// A wound of size R sprays at 120·R and sends round(0.3 × 120·R)
    /// droplets: the chest cut (R 0.457) 54.84 → 16, R 1 → 36, the smallest
    /// bleeding wound (R 0.05) 6 → 2; above R 1.67 (in double precision,
    /// so the single-precision 1.67 is not above it) 200 → 60. Halves round
    /// to even: 15 → 4.5 → 4, 5 → 1.5 → 2.
    #[test]
    fn jets_send_droplets_by_the_wound() {
        assert!((spray_strength(0.457) - 54.84).abs() < 1e-4);
        assert_eq!(jet_droplets(spray_strength(0.457)), 16);
        assert_eq!(jet_droplets(spray_strength(1.0)), 36);
        assert_eq!(jet_droplets(spray_strength(0.05)), 2);
        assert_eq!(spray_strength(1.67), 1.67 * 120.0);
        assert_eq!(jet_droplets(spray_strength(1.67)), 60);
        assert_eq!(spray_strength(1.6701), 200.0);
        assert_eq!(spray_strength(5.0), 200.0);
        assert_eq!(jet_droplets(200.0), 60);
        assert_eq!(jet_droplets(15.0), 4);
        assert_eq!(jet_droplets(5.0), 2);
        assert_eq!(jet_droplets(25.0), 8);
    }

    /// The weights follow the striker: a point of a blade turned and moved
    /// is where the turned, moved blade has it.
    #[test]
    fn weights_ride_with_the_striker() {
        let before = Affine3A::from_rotation_translation(
            Quat::from_rotation_x(0.4),
            Vec3::new(1.0, 1.2, -0.3),
        );
        let local = Vec3::new(0.02, 0.7, -0.01);
        let point = before.transform_point3(local);
        let w = tetra_weights(point, &frame_tetra(&before)).unwrap();
        assert!((w.iter().sum::<f32>() - 1.0).abs() < 1e-5);
        assert!(tetra_point(&w, &frame_tetra(&before)).distance(point) < 1e-5);
        let after = Affine3A::from_rotation_translation(
            Quat::from_rotation_z(-0.8) * Quat::from_rotation_x(0.4),
            Vec3::new(1.1, 0.9, -0.2),
        );
        let moved = tetra_point(&w, &frame_tetra(&after));
        assert!(moved.distance(after.transform_point3(local)) < 1e-5);
        // A flat tetrahedron gives no weights.
        assert_eq!(tetra_weights(Vec3::ONE, &[Vec3::ZERO; 4]), None);
    }

    /// Seven wait at most; each bleeds seven ticks after it was struck, one
    /// a tick: two struck together bleed on the 7th and the 8th tick.
    #[test]
    fn the_queue_waits_seven_ticks_one_a_tick() {
        let mut q = WoundQueue::default();
        let s = Entity::PLACEHOLDER;
        for i in 0..WOUND_SLOTS {
            assert!(q.push(s, None, Vec3::X * i as f32, 0.1));
        }
        assert!(!q.push(s, None, Vec3::Y, 0.1));
        assert_eq!(q.pending().len(), 7);
        for tick in 1..7 {
            assert_eq!(q.tick(), None, "tick {tick}");
        }
        for (tick, i) in (7..14).zip(0..) {
            let w = q.tick().unwrap_or_else(|| panic!("tick {tick}"));
            assert_eq!(w.contact, Vec3::X * i as f32);
            assert_eq!(w.delay, 7 - tick, "tick {tick}");
        }
        assert!(q.pending().is_empty());
        assert_eq!(q.tick(), None);
        // A wound struck while others wait still waits its own 7 ticks.
        q.push(s, None, Vec3::ZERO, 0.1);
        q.tick();
        q.tick();
        q.push(s, None, Vec3::ONE, 0.1);
        let due: Vec<Option<Vec3>> = (0..6).map(|_| q.tick().map(|w| w.contact)).collect();
        assert_eq!(due, [None, None, None, None, Some(Vec3::ZERO), None]);
        assert_eq!(q.tick().map(|w| w.contact), Some(Vec3::ONE));
    }
}
