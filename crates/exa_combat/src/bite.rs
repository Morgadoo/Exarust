//! A weapon that bites sticks in the part it hit.
//!
//! The damage routine opens a bite after a wound on flesh when the bite
//! depth β is above 0.1 (compared in double) and the striker holds a weapon
//! (its weapon class has a hands bit, 0x30): fists and claws never stick,
//! whatever their race, and a blow turned, clashed or blocked opens none.
//!
//! The bite appends a link to the weapon's body. The depth is capped at 1
//! before the flags, the strength and the lifetime. The flags compare that
//! capped depth with 0.4. The slack word is 12 times the depth, the
//! smoothing the zone smoothing plus 0.5, and the pull 1. The lifetime unit
//! n is the link-iteration count of the weapon's tree root, the holder's
//! body (20 for a living character; a feral one ramps to 15). The count is
//! 3 times the product of 83, n and the depth, rounded to nearest; the
//! strain is twice that product. The self point is the contact on the
//! weapon, the parent point the contact on the struck part, and the axis
//! the strike's velocity (cm a tick) in the weapon's frame. The weapon's
//! layer becomes 0xC0; the struck part, its parent and each child gain bit
//! 0x40.
//!
//! The solver then pulls the link in every iteration ([`BiteHold`]): the
//! strain falls by (q/|d|)·2 + 1 a pass (q = 1 + the gap in the weapon's
//! frame along the axis), so a bite of depth β holds at most 2·β seconds
//! for a character, and less for a feral one, whose n is smaller while the
//! solver still runs 20 iterations a tick. Below zero the grab breaks, its
//! layers are lowered and every breakable grab of that weapon on that part
//! goes.
//!
//! Evidence: the bite link built after an armed hit, its call in the
//! damage routine, the layer routine, and the plain link iteration's grab
//! branch.

use bevy::prelude::*;
use exa_physics::body::{BodySystems, BodyTicks, GrabPull, Ragdoll, SolverStage, SolverStages};

/// Flags when the capped depth is not above 0.4.
const SHALLOW: u32 = 0x11000;
/// Flags when the capped depth is above 0.4.
const DEEP: u32 = 0x211000;
/// The weapon's layer while it is stuck.
const WEAPON_LAYER: u32 = exa_physics::body::STUCK_WEAPON_LAYER;
/// The bit added to the struck part, its parent and its children.
const PART_BIT: u32 = exa_physics::body::GRAB_LAYER;
/// Body ticks in one second, the lifetime's unit.
const TICKS: i64 = 83;
/// The depth a bite must pass, compared in double.
pub const BITE_FROM: f64 = 0.1;
/// The weapon class's hands bits: a held weapon.
pub const HANDS: u8 = 0x30;

/// Whether a wound of bite depth `bite` by a striker of weapon class
/// `weapon_class` sticks the weapon: the depth above 0.1 in double, and a
/// held weapon.
pub fn bites(bite: f32, weapon_class: u8) -> bool {
    f64::from(bite) > BITE_FROM && weapon_class & HANDS != 0
}

/// The link a sticking weapon appends, and the layers it sets.
#[derive(Debug, Clone, PartialEq)]
pub struct Stick {
    pub flags: u32,
    pub strength: f32,
    pub smoothing: f32,
    pub count: i32,
    pub span: f32,
    pub weapon_layer: u32,
    /// The strength the solver divides. The image stores 1 here.
    pub pull: f32,
}

/// The layers after bit 0x40 is added to the struck bodies.
#[derive(Debug, Clone, PartialEq)]
pub struct StuckLayers {
    pub parent: Option<u32>,
    pub part: u32,
    pub children: Vec<u32>,
}

/// The stick link for an armed bite `depth`, zone `smoothing` and lifetime
/// `unit` (the holder's link-iteration count). An unarmed blow, and a depth
/// of 0.1 or less, sticks nothing.
pub fn stick(armed: bool, depth: f32, smoothing: f32, unit: i32) -> Option<Stick> {
    if !armed || f64::from(depth) <= BITE_FROM {
        return None;
    }
    let held = depth.min(1.0);
    // The capped depth widened to a double against 0.4.
    let flags = if f64::from(held) > 0.4 { DEEP } else { SHALLOW };
    let ticks = (TICKS * i64::from(unit)) as f32;
    let scaled = ticks * held;
    Some(Stick {
        flags,
        strength: held * 12.0,
        smoothing: smoothing + 0.5,
        // The processor's conversion, to nearest (ties to even).
        count: (scaled.round_ties_even() as i32).wrapping_mul(3),
        span: scaled * 2.0,
        weapon_layer: WEAPON_LAYER,
        pull: 1.0,
    })
}

/// Where a bite holds: the contact as weights on the weapon part's and the
/// struck part's particles, and the strike's velocity in the weapon's frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BitePoints {
    pub on_weapon: [f32; 4],
    pub on_struck: [f32; 4],
    pub axis: Vec3,
}

impl Default for BitePoints {
    fn default() -> Self {
        Self {
            on_weapon: [1.0, 0.0, 0.0, 0.0],
            on_struck: [1.0, 0.0, 0.0, 0.0],
            axis: Vec3::ZERO,
        }
    }
}

/// The strike's velocity `velocity` (cm a tick) in the frame of a weapon
/// part whose rotation is `frame`: its dot with each row of the matrix, as
/// the original turns it before storing the axis.
pub fn grab_axis(frame: Mat3, velocity: Vec3) -> Vec3 {
    frame.transpose() * velocity
}

/// The grab the body solver pulls, opened from a stick link between a
/// weapon of link mobility `weapon` and a struck part of `struck`: the
/// shares are the original's share routine over the two.
pub fn grab_of(link: &Stick, points: &BitePoints, weapon: f32, struck: f32) -> GrabPull {
    GrabPull {
        strain: link.span,
        flags: link.flags,
        strength: link.pull,
        axis: points.axis,
        on_weapon: points.on_weapon,
        on_struck: points.on_struck,
        shares: exa_physics::body::grab_shares(weapon, struck),
    }
}

/// One weapon stuck in one part, as the solver stage holds it.
#[derive(Debug, Clone)]
pub struct HeldBite {
    /// The weapon's part (its node entity) and the struck part's entity:
    /// the stage finds their bodies by them.
    pub weapon: Entity,
    pub struck: Entity,
    pub grab: GrabPull,
    /// The body tick it opened on.
    pub opened: u64,
}

/// A bite that broke in the solver, for the log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrokenBite {
    pub weapon: Entity,
    pub struck: Entity,
    pub opened: u64,
}

/// Holds the bites inside the body solver's link passes, after every
/// body's own links (where the original's weapon body runs its links).
#[derive(Default)]
pub struct BiteHold {
    pub bites: Vec<HeldBite>,
    /// The bites that broke since the last [`log_broken_bites`].
    pub broken: Vec<BrokenBite>,
}

/// The body and part index of the part `entity` among `bodies`.
fn find_part(bodies: &[&mut Ragdoll], entity: Entity) -> Option<(usize, usize)> {
    bodies.iter().enumerate().find_map(|(b, body)| {
        body.parts
            .iter()
            .position(|p| p.entity == entity)
            .map(|p| (b, p))
    })
}

/// One grab pass of a bite between two parts of `bodies`; whether it broke.
fn pull(
    bodies: &mut [&mut Ragdoll],
    weapon: (usize, usize),
    struck: (usize, usize),
    grab: &mut GrabPull,
) -> bool {
    let ((wb, wp), (sb, sp)) = (weapon, struck);
    // A struck body that is disabled (the original's state bit 0x800)
    // breaks the grab.
    let disabled = bodies[sb].disabled();
    if wb == sb {
        return bodies[wb].pull_grab(wp, sp, grab, disabled);
    }
    let (low, high) = (wb.min(sb), wb.max(sb));
    let (left, right) = bodies.split_at_mut(high);
    let (a, b) = (&mut *left[low], &mut *right[0]);
    let (weapon_body, struck_body) = if wb < sb { (a, b) } else { (b, a) };
    Ragdoll::pull_grab_across(weapon_body, wp, struck_body, sp, grab, disabled)
}

impl SolverStage for BiteHold {
    fn links(&mut self, bodies: &mut [&mut Ragdoll], _: usize) {
        let mut i = 0;
        while i < self.bites.len() {
            let (weapon, struck) = (self.bites[i].weapon, self.bites[i].struck);
            let (Some(w), Some(s)) = (find_part(bodies, weapon), find_part(bodies, struck)) else {
                i += 1;
                continue;
            };
            if !pull(bodies, w, s, &mut self.bites[i].grab) {
                i += 1;
                continue;
            }
            // The break drops every breakable grab of this weapon part on
            // that part, and lowers the grab layers of the struck part and
            // its neighbours and the weapon's bit.
            let opened = self.bites[i].opened;
            let same = |b: &HeldBite| b.weapon == weapon && b.struck == struck;
            let before = self.bites[..i].iter().filter(|b| same(b)).count();
            self.bites.retain(|b| !same(b));
            i -= before;
            bodies[s.0].lower_grab_layer(s.1);
            if let Some(layer) = bodies[w.0].layer(w.1) {
                bodies[w.0].set_layer(w.1, layer & !PART_BIT);
            }
            self.broken.push(BrokenBite {
                weapon,
                struck,
                opened,
            });
        }
    }

    fn coupled(&self, bodies: &[&mut Ragdoll], coupled: &mut [bool]) {
        for bite in &self.bites {
            for entity in [bite.weapon, bite.struck] {
                if let Some((b, _)) = find_part(bodies, entity) {
                    coupled[b] = true;
                }
            }
        }
    }
}

/// Add bit 0x40 to the struck part, its parent when it has one, and each child.
pub fn stuck_layers(parent: Option<u32>, part: u32, children: &[u32]) -> StuckLayers {
    StuckLayers {
        parent: parent.map(|layer| layer | PART_BIT),
        part: part | PART_BIT,
        children: children.iter().map(|layer| layer | PART_BIT).collect(),
    }
}

/// A wound that sticks the striker's weapon in the struck part: the damage
/// routine's bite, opened before the next tick's solver.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct BiteTaken {
    pub attacker: Entity,
    /// The weapon's body (the held item's root entity).
    pub striker: Entity,
    pub target: Entity,
    pub bone: usize,
    /// The contact (world, m).
    pub point: Vec3,
    /// The strike's velocity (world, cm a tick).
    pub velocity: Vec3,
    /// The bite depth β.
    pub bite: f32,
    /// The zone smoothing: 0.1 a swing, 0 a stab, 0.5 unarmed.
    pub smoothing: f32,
}

/// The weapon body's part nearest the contact.
fn nearest_part(body: &Ragdoll, at: Vec3) -> Option<usize> {
    (0..body.parts.len())
        .filter_map(|i| body.origin_at(i).map(|o| (i, o.distance_squared(at))))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

/// Opens the bites the wounds asked for: the stick link from the depth,
/// the zone smoothing and the holder's link-iteration count, the grab at
/// the contact with the strike's axis, and the layers. A held model with no
/// body of its own (Exarust builds item bodies only for models with rigid
/// nodes) sticks through the part that carries it, the holder's hand
/// [Exarust stand-in for the original's weapon body]. A struck body
/// without the bone opens nothing.
pub fn open_bites(
    mut taken: MessageReader<BiteTaken>,
    mut bodies: Query<&mut Ragdoll>,
    stages: Option<ResMut<SolverStages>>,
    ticks: Option<Res<BodyTicks>>,
    parents: Query<&ChildOf>,
) {
    let Some(mut stages) = stages else {
        taken.clear();
        return;
    };
    let tick = ticks.map_or(0, |t| t.0);
    for bite in taken.read() {
        let unit = bodies
            .get(bite.attacker)
            .map_or(exa_physics::body::LIVING_LINK_ITERATIONS, |b| {
                b.link_iterations()
            });
        let Some(link) = stick(true, bite.bite, bite.smoothing, unit) else {
            continue;
        };
        // The weapon's own body; a held model that has none (no rigid node)
        // sticks through the holder's part that carries it, its hand.
        let (holder, hand) = if bodies.contains(bite.striker) {
            (bite.striker, None)
        } else {
            let carrier = parents.iter_ancestors(bite.striker).find(|&e| {
                bodies
                    .get(bite.attacker)
                    .is_ok_and(|b| b.parts.iter().any(|p| p.entity == e))
            });
            (bite.attacker, carrier)
        };
        let Ok([mut weapon, mut struck]) = bodies.get_many_mut([holder, bite.target]) else {
            info!(
                "bite on {:?} (depth {:.3}): the weapon {:?} has no body",
                bite.target, bite.bite, bite.striker
            );
            continue;
        };
        let wp = match hand {
            Some(e) => weapon.parts.iter().position(|p| p.entity == e),
            None if holder == bite.striker => nearest_part(&weapon, bite.point),
            None => None,
        };
        let Some(wp) = wp else {
            info!(
                "bite on {:?}: no part carries the weapon {:?}",
                bite.target, bite.striker
            );
            continue;
        };
        let Some(sp) = struck.parts.iter().position(|p| p.bone == bite.bone) else {
            info!("bite on {:?}: no part for bone {}", bite.target, bite.bone);
            continue;
        };
        let (Some(on_weapon), Some(on_struck)) = (
            weapon.weights_now(wp, bite.point),
            struck.weights_now(sp, bite.point),
        ) else {
            info!("bite on {:?}: a part is degenerate", bite.target);
            continue;
        };
        let frame = Mat3::from(weapon.parts[wp].settled_transform().matrix3);
        let points = BitePoints {
            on_weapon,
            on_struck,
            axis: grab_axis(frame, bite.velocity),
        };
        let (Some(wm), Some(sm)) = (weapon.link_mobility(wp), struck.link_mobility(sp)) else {
            continue;
        };
        let grab = grab_of(&link, &points, wm, sm);
        struck.raise_grab_layer(sp);
        weapon.set_layer(wp, link.weapon_layer);
        info!(
            "bite: {:?}'s weapon sticks in {:?} bone {} at tick {tick}: depth {:.3}, flags {:#x}, \
             strain {:.1} (unit {unit}), smoothing {:.2}, held by {}",
            bite.attacker,
            bite.target,
            bite.bone,
            bite.bite,
            link.flags,
            link.span,
            link.smoothing,
            if hand.is_some() {
                "the holder's hand"
            } else {
                "the weapon's body"
            }
        );
        let hold = HeldBite {
            weapon: weapon.parts[wp].entity,
            struck: struck.parts[sp].entity,
            grab,
            opened: tick,
        };
        if let Some(stage) = stages.get_mut::<BiteHold>() {
            stage.bites.push(hold);
        }
    }
}

/// Logs the bites the solver broke since the last call.
pub fn log_broken_bites(stages: Option<ResMut<SolverStages>>, ticks: Option<Res<BodyTicks>>) {
    let Some(mut stages) = stages else {
        return;
    };
    let Some(stage) = stages.get_mut::<BiteHold>() else {
        return;
    };
    let tick = ticks.map_or(0, |t| t.0);
    for broken in stage.broken.drain(..) {
        info!(
            "bite released at tick {tick}: weapon part {:?} from part {:?}, held {} ticks",
            broken.weapon,
            broken.struck,
            tick.saturating_sub(broken.opened)
        );
    }
}

/// Installs the bite stage into the solver's stages.
pub fn install_stage(stages: Option<ResMut<SolverStages>>) {
    if let Some(mut stages) = stages
        && stages.get::<BiteHold>().is_none()
    {
        stages.add(BiteHold::default());
    }
}

/// The bite's stage, its opening after the hits and its break log.
pub fn plugin(app: &mut App) {
    app.add_message::<BiteTaken>()
        .add_systems(Startup, install_stage)
        .add_systems(FixedUpdate, log_broken_bites.after(BodySystems));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bite_above_a_tenth_sticks_the_weapon() {
        assert!(stick(false, 1.0, 0.1, 1).is_none());
        // The test is the double 0.1: the single 0.1 widened is above it.
        assert!(stick(true, 0.099_999_99, 0.1, 1).is_none());
        assert!(stick(true, 0.1, 0.1, 1).is_some());
        let low = stick(true, 0.25, 0.1, 1).unwrap();
        assert_eq!(low.flags, 0x11000);
        assert_eq!(low.strength.to_bits(), 0x4040_0000);
        assert_eq!(low.smoothing.to_bits(), 0x3f19_999a);
        // 83 × 0.25 = 20.75 converts to nearest: 21, times 3.
        assert_eq!(low.count, 63);
        assert_eq!(low.span.to_bits(), 0x4226_0000);
        assert_eq!(low.weapon_layer, 0xC0);
        let edge = stick(true, 0.4, 0.0, 1).unwrap();
        assert_eq!(edge.flags, 0x211000);
        assert_eq!(edge.strength.to_bits(), 0x4099_999a);
        assert_eq!(edge.smoothing.to_bits(), 0x3f00_0000);
        assert_eq!(edge.count, 99);
        assert_eq!(edge.span.to_bits(), 0x4284_cccd);
        let full = stick(true, 1.0, 0.1, 1).unwrap();
        assert_eq!(full.count, 249);
        assert_eq!(full.span.to_bits(), 0x4326_0000);
        assert_eq!(full.strength.to_bits(), 0x4140_0000);
        let deep = stick(true, 3.0, 0.5, 1).unwrap();
        assert_eq!(deep.count, full.count);
        assert_eq!(deep.span.to_bits(), full.span.to_bits());
        assert_eq!(deep.strength.to_bits(), full.strength.to_bits());
        assert_eq!(deep.smoothing.to_bits(), 0x3f80_0000);
        let two = stick(true, 0.11, 0.0, 2).unwrap();
        assert_eq!(two.flags, 0x11000);
        assert_eq!(two.count, 54);
        assert_eq!(two.strength.to_bits(), 0x3fa8_f5c2);
        assert_eq!(two.span.to_bits(), 0x4212_147b);
        let marked = stuck_layers(Some(0), 0x3, &[1, 0x40]);
        assert_eq!(marked.parent, Some(0x40));
        assert_eq!(marked.part, 0x43);
        assert_eq!(marked.children, vec![0x41, 0x40]);
        assert!(stuck_layers(None, 0, &[]).parent.is_none());
    }

    #[test]
    fn the_lifetime_unit_is_the_holders_link_iteration_count() {
        // A living character's count, round(1660 / 83) = 20: β = 0.25 gives
        // a product of 415, the count 1245 and the strain 830; at most 830
        // passes, 20 a tick, so about 41 ticks, half a second.
        let living = stick(true, 0.25, 0.1, 20).unwrap();
        assert_eq!(living.count, 1245);
        assert_eq!(living.span, 830.0);
        // A fully feral body ramps to 15: 311.25, rounded to 311.
        let feral = stick(true, 0.25, 0.1, 15).unwrap();
        assert_eq!(feral.count, 933);
        assert_eq!(feral.span, 622.5);
        // The deepest bite, 1 after the cap: 3320 passes for a character,
        // 166 ticks, two seconds.
        let full = stick(true, 1.7, 0.0, 20).unwrap();
        assert_eq!(full.count, 4980);
        assert_eq!(full.span, 3320.0);
        assert_eq!(full.flags, 0x211000);
    }

    #[test]
    fn only_a_held_weapon_bites_and_only_past_a_tenth() {
        // One- and two-handed, polearm and dual wield have a hands bit.
        for class in [0x10, 0x20, 0x30, 0x90] {
            assert!(bites(0.5, class), "{class:#x}");
        }
        // Fists (0x80), claws (0xC0) and nothing held never stick.
        for class in [0x00, 0x80, 0xC0] {
            assert!(!bites(0.5, class), "{class:#x}");
        }
        // The single 0.1 is just above the double 0.1, so it bites.
        assert!(bites(0.1, 0x10));
        assert!(!bites(0.099_999_99, 0x10));
        assert!(!bites(f32::NAN, 0x10));
    }

    #[test]
    fn the_axis_is_the_velocity_dotted_with_the_frames_rows() {
        let frame = Mat3::from_cols(Vec3::Y, -Vec3::X, Vec3::Z);
        let axis = grab_axis(frame, Vec3::new(3.0, 4.0, 5.0));
        assert_eq!(axis, Vec3::new(4.0, -3.0, 5.0));
    }

    fn body(entity: u32, x: f32) -> Ragdoll {
        let at = bevy::math::Affine3A::from_translation(Vec3::new(x, 1.0, 0.0));
        Ragdoll::new(&[exa_physics::body::PartSetup {
            bone: entity as usize,
            entity: Entity::from_raw_u32(entity).unwrap(),
            parent: None,
            world: at,
            centre: Vec3::ZERO,
            half: Vec3::splat(0.05),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: Vec::new(),
            spheres: None,
        }])
    }

    fn gap(a: &Ragdoll, b: &Ragdoll) -> f32 {
        (a.parts[0].positions[0] - b.parts[0].positions[0]).length()
    }

    fn held(grab: GrabPull) -> HeldBite {
        HeldBite {
            weapon: Entity::from_raw_u32(1).unwrap(),
            struck: Entity::from_raw_u32(2).unwrap(),
            grab,
            opened: 7,
        }
    }

    fn solve(weapon: &mut Ragdoll, struck: &mut Ragdoll, stage: BiteHold) -> BiteHold {
        let mut stages: Vec<Box<dyn SolverStage>> = vec![Box::new(stage)];
        exa_physics::body::solve_with(
            &mut [&mut *weapon, &mut *struck],
            &[] as &[[Vec3; 3]],
            &mut stages,
        );
        let stage = stages.pop().unwrap();
        let any: Box<dyn std::any::Any> = stage;
        *any.downcast::<BiteHold>().unwrap()
    }

    #[test]
    fn a_bite_holds_a_weapon_body_to_the_part_it_struck() {
        let (mut weapon, mut struck) = (body(1, 0.0), body(2, 0.08));
        let before = gap(&weapon, &struck);
        let link = stick(true, 0.25, 0.1, 20).expect("a depth past a tenth sticks");
        let grab = grab_of(&link, &BitePoints::default(), 1.0, 1.0);
        let stage = solve(
            &mut weapon,
            &mut struck,
            BiteHold {
                bites: vec![held(grab)],
                broken: Vec::new(),
            },
        );
        let after = gap(&weapon, &struck);
        assert!(after < before * 0.25, "{after} stayed near {before}");
        assert_eq!(stage.bites.len(), 1);
        assert!(stage.bites[0].grab.strain < link.span);
        assert!(stage.broken.is_empty());

        // No bite: the bodies keep their gap.
        let (mut a, mut b) = (body(1, 0.0), body(2, 0.08));
        let open = gap(&a, &b);
        solve(&mut a, &mut b, BiteHold::default());
        let still = gap(&a, &b);
        assert!((still - open).abs() < 0.001, "{still} moved from {open}");
    }

    #[test]
    fn a_spent_bite_breaks_and_lowers_the_layers() {
        let (mut weapon, mut struck) = (body(1, 0.0), body(2, 0.08));
        struck.raise_grab_layer(0);
        weapon.set_layer(0, 0xC0);
        let link = stick(true, 0.25, 0.1, 20).unwrap();
        let mut grab = grab_of(&link, &BitePoints::default(), 1.0, 1.0);
        // A strain one pass can spend: the 8 cm gap takes 1/8·2 + 1.
        grab.strain = 1.0;
        let twin = grab.clone();
        let stage = solve(
            &mut weapon,
            &mut struck,
            BiteHold {
                bites: vec![held(grab), held(twin)],
                broken: Vec::new(),
            },
        );
        // Both grabs of that weapon on that part go with the break.
        assert!(stage.bites.is_empty());
        assert_eq!(
            stage.broken,
            vec![BrokenBite {
                weapon: Entity::from_raw_u32(1).unwrap(),
                struck: Entity::from_raw_u32(2).unwrap(),
                opened: 7,
            }]
        );
        assert_eq!(struck.layer(0), Some(0));
        assert_eq!(weapon.layer(0), Some(0x80));
    }

    /// A held model without a body of its own sticks through the hand that
    /// carries it: the bite opens on the holder's hand part with the
    /// holder's link-iteration count, raises the struck part's grab bit and
    /// sets the hand's layer to the stuck weapon's.
    #[test]
    fn a_bodiless_weapon_sticks_through_its_hand() {
        let mut app = App::new();
        app.add_message::<BiteTaken>()
            .init_resource::<SolverStages>()
            .add_systems(Update, open_bites);
        app.world_mut()
            .resource_mut::<SolverStages>()
            .add(BiteHold::default());
        let hand = app.world_mut().spawn_empty().id();
        let mut holder = body(1, 0.0);
        holder.parts[0].entity = hand;
        holder.set_link_iterations(15);
        let attacker = app.world_mut().spawn(holder).id();
        let blade = app.world_mut().spawn(ChildOf(hand)).id();
        let target = app.world_mut().spawn(body(2, 0.08)).id();
        app.world_mut().write_message(BiteTaken {
            attacker,
            striker: blade,
            target,
            bone: 2,
            point: Vec3::new(0.04, 1.0, 0.0),
            velocity: Vec3::new(5.0, 0.0, 0.0),
            bite: 0.25,
            smoothing: 0.1,
        });
        app.update();
        let stages = app.world().resource::<SolverStages>();
        let stage = stages.get::<BiteHold>().unwrap();
        assert_eq!(stage.bites.len(), 1);
        let bite = &stage.bites[0];
        assert_eq!(bite.weapon, hand);
        // Unit 15: 83 × 15 × 0.25 = 311.25, the strain twice that.
        assert_eq!(bite.grab.strain, 622.5);
        assert_eq!(bite.grab.flags, 0x11000);
        let struck = app.world().get::<Ragdoll>(target).unwrap();
        assert_eq!(struck.layer(0), Some(0x40));
        let holder = app.world().get::<Ragdoll>(attacker).unwrap();
        assert_eq!(holder.layer(0), Some(0xC0));
    }
}
