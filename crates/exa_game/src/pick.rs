//! What a click finds under the cursor, and whether it is close enough to use.
//!
//! A hinge that is not blocked is a door or a lever. A fitting of kind 3 is
//! a wheel, and kind 5 is a crank unless it is held. A dead character is a
//! corpse and a living one a person. A loose prop whose mobility is under
//! 0.08 is too heavy to move, unless it is tagged bulky. A prop tagged as
//! needing strength is refused when the player's strength is under 0.9. A
//! usable item on a prop stays usable. No body at all is nothing, even when
//! the item was usable.
//!
//! A point farther than 70 cm is turned toward before it is used. A point is
//! within reach when the point the pick hit is at most 245 cm high (the
//! world's height), and its offset from the player's hip is at most 300 cm
//! up or down and 300 cm across the ground. On a door the offset is the
//! door's and the limits are 300 cm high and 350 cm across; a wheel takes
//! its own offset too, with the plain limits. A comparison the original
//! makes with a NaN passes.
//!
//! Evidence: the cursor classification and the reach test of the walk-to-use
//! checks, read in the instructions (`context/re/player-control-hand.md`).

use bevy::math::Vec3;

/// What the click will do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    Nothing,
    Loose,
    Usable,
    Hinge,
    Corpse,
    Person,
    Wheel,
    Crank,
    Scene,
}

impl Pick {
    /// A prop or item the hand can move: the original's class bit 1 (a
    /// loose prop, or a usable item on a body).
    pub fn movable(self) -> bool {
        matches!(self, Pick::Loose | Pick::Usable)
    }
}

/// A hinge on the object. Blocked hinges are left alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hinge {
    pub blocked: bool,
}

/// A special fitting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fitting {
    Wheel,
    /// Held cranks are not turned.
    Crank {
        held: bool,
    },
}

/// The body the object carries, if the pick can see one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Body {
    Character { dead: bool },
    Prop,
    Scene,
    Other,
}

/// The object under the cursor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Target {
    pub has_record: bool,
    pub hinge: Option<Hinge>,
    pub fitting: Option<Fitting>,
    pub usable: bool,
    pub body: Option<Body>,
    /// The prop's mobility (larger moves more).
    pub mobility: f32,
    /// Skips the refusal of a prop whose mobility is under 0.08.
    pub bulky: bool,
    /// The prop needs strength: under 0.9 refuses it.
    pub heavy: bool,
    pub strength: f32,
}

impl Default for Target {
    fn default() -> Self {
        Self {
            has_record: false,
            hinge: None,
            fitting: None,
            usable: false,
            body: None,
            mobility: 1.0,
            bulky: false,
            heavy: false,
            strength: 1.0,
        }
    }
}

/// The interaction the click starts.
pub fn classify(target: &Target) -> Pick {
    let mut kind = Pick::Nothing;
    if target.has_record {
        if let Some(hinge) = target.hinge {
            return if hinge.blocked {
                Pick::Nothing
            } else {
                Pick::Hinge
            };
        }
        if let Some(fitting) = target.fitting {
            match fitting {
                Fitting::Wheel => return Pick::Wheel,
                Fitting::Crank { held: false } => return Pick::Crank,
                Fitting::Crank { held: true } => {}
            }
        }
        if target.usable {
            kind = Pick::Usable;
        }
    }
    let Some(body) = target.body else {
        return Pick::Nothing;
    };
    match body {
        Body::Character { dead: true } => Pick::Corpse,
        Body::Character { dead: false } => Pick::Person,
        Body::Scene => Pick::Scene,
        Body::Other => kind,
        Body::Prop => {
            if target.heavy {
                if target.strength < 0.9 {
                    return Pick::Nothing;
                }
            } else if !target.bulky && target.mobility < 0.08 {
                return Pick::Nothing;
            }
            if kind == Pick::Nothing {
                Pick::Loose
            } else {
                kind
            }
        }
    }
}

/// A use farther than 70 cm turns the body toward the point first.
pub fn turns_before_use(distance_cm: f32) -> bool {
    distance_cm.is_finite() && distance_cm > 70.0
}

/// What the reach is measured to: the point the pick hit, a door, or a
/// wheel (the last two by their own place).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReachTo {
    Point,
    Door,
    Wheel,
}

/// Whether the point the pick hit, `hit_height_cm` high in the world, is
/// close enough to use without walking closer, its offset (or the door's,
/// or the wheel's) from the player's hip being `offset_cm`.
pub fn within_reach(hit_height_cm: f32, offset_cm: Vec3, to: ReachTo) -> bool {
    let (height_cap, across) = match to {
        ReachTo::Door => (300.0_f32, 350.0_f32),
        ReachTo::Point | ReachTo::Wheel => (245.0, 300.0),
    };
    // Each test fails only on an ordered comparison: NaN passes.
    if height_cap < hit_height_cm {
        return false;
    }
    if offset_cm.y.abs() > 300.0 {
        return false;
    }
    // The offset's height is zeroed before its squared length is taken.
    let flat = Vec3::new(offset_cm.x, 0.0, offset_cm.z);
    let across2 = flat.x * flat.x + flat.y * flat.y + flat.z * flat.z;
    let beyond = across2 > across * across;
    !beyond
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prop(mobility: f32) -> Target {
        Target {
            has_record: true,
            body: Some(Body::Prop),
            mobility,
            ..Target::default()
        }
    }

    #[test]
    fn a_click_names_the_thing() {
        let mut door = Target {
            has_record: true,
            hinge: Some(Hinge { blocked: false }),
            body: Some(Body::Prop),
            ..Target::default()
        };
        assert_eq!(classify(&door), Pick::Hinge);
        door.hinge = Some(Hinge { blocked: true });
        assert_eq!(classify(&door), Pick::Nothing);

        let wheel = Target {
            has_record: true,
            fitting: Some(Fitting::Wheel),
            ..Target::default()
        };
        assert_eq!(classify(&wheel), Pick::Wheel);
        let crank = Target {
            has_record: true,
            fitting: Some(Fitting::Crank { held: false }),
            ..Target::default()
        };
        assert_eq!(classify(&crank), Pick::Crank);

        let mut person = Target {
            body: Some(Body::Character { dead: false }),
            ..Target::default()
        };
        assert_eq!(classify(&person), Pick::Person);
        person.body = Some(Body::Character { dead: true });
        assert_eq!(classify(&person), Pick::Corpse);

        assert_eq!(classify(&prop(0.07)), Pick::Nothing);
        assert_eq!(classify(&prop(0.08)), Pick::Loose);
        let mut bulky = prop(0.07);
        bulky.bulky = true;
        assert_eq!(classify(&bulky), Pick::Loose);
        let mut heavy = prop(2.0);
        heavy.heavy = true;
        heavy.strength = 0.89;
        assert_eq!(classify(&heavy), Pick::Nothing);
        heavy.strength = 0.9;
        assert_eq!(classify(&heavy), Pick::Loose);

        let mut vial = prop(0.2);
        vial.usable = true;
        assert_eq!(classify(&vial), Pick::Usable);
        assert!(Pick::Usable.movable() && Pick::Loose.movable());
        assert!(!Pick::Corpse.movable());
        vial.body = None;
        assert_eq!(classify(&vial), Pick::Nothing);

        let scene = Target {
            body: Some(Body::Scene),
            ..Target::default()
        };
        assert_eq!(classify(&scene), Pick::Scene);
        assert!(!turns_before_use(70.0));
        assert!(turns_before_use(70.1));
    }

    /// 245 cm high and 300 cm across hold; a hair beyond either does not; a
    /// door holds to 300 cm high and 350 cm across; NaN passes every test.
    #[test]
    fn the_reach_measures_the_hit_height_and_the_offset() {
        let across = |x: f32| Vec3::new(x, 0.0, 0.0);
        assert!(within_reach(245.0, across(300.0), ReachTo::Point));
        assert!(!within_reach(245.1, across(300.0), ReachTo::Point));
        assert!(!within_reach(200.0, across(300.1), ReachTo::Point));
        assert!(!within_reach(
            200.0,
            Vec3::new(0.0, -300.1, 0.0),
            ReachTo::Point
        ));
        assert!(within_reach(
            200.0,
            Vec3::new(180.0, 300.0, 240.0),
            ReachTo::Point
        ));
        assert!(!within_reach(
            200.0,
            Vec3::new(180.0, 0.0, 240.1),
            ReachTo::Point
        ));
        assert!(within_reach(300.0, across(350.0), ReachTo::Door));
        assert!(!within_reach(300.1, across(10.0), ReachTo::Door));
        assert!(!within_reach(250.0, across(10.0), ReachTo::Wheel));
        assert!(within_reach(f32::NAN, across(f32::NAN), ReachTo::Point));
        // The height is the world's, not the offset's.
        assert!(!within_reach(500.0, across(0.0), ReachTo::Point));
    }
}
