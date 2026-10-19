//! What a click finds under the cursor, and whether it is close enough to use.
//!
//! A hinge that is not blocked is a door or a lever. A fitting of kind 3 is
//! a wheel, and kind 5 is a crank unless it is held. A dead character is a
//! corpse and a living one a person. A loose prop lighter than 0.08 is
//! ignored, unless it is tagged bulky. A heavy prop is ignored when strength
//! is under 0.9. A usable item on a prop stays usable. No body at all is
//! nothing, even when the item was usable.
//!
//! A point farther than 70 cm is turned toward before it is used. It is
//! within reach when its height is at most 245 cm, the vertical gap is at
//! most 300 cm and the horizontal distance is at most 300 cm. A door raises
//! the height cap to 300 cm and the horizontal reach to 350 cm.
//!
//! Evidence: the cursor classification and the walk-to-use checks.

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
    pub mass: f32,
    /// Skips the refusal of a prop lighter than 0.08.
    pub bulky: bool,
    /// The prop is heavy: strength under 0.9 refuses it.
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
            mass: 1.0,
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
            } else if !target.bulky && target.mass < 0.08 {
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

/// Whether the point is close enough to use without walking closer.
pub fn within_reach(
    height_cm: f32,
    vertical_delta_cm: f32,
    horizontal_cm: f32,
    hinge: bool,
) -> bool {
    if !height_cm.is_finite() || !vertical_delta_cm.is_finite() || !horizontal_cm.is_finite() {
        return false;
    }
    let (height_cap, reach) = if hinge {
        (300.0, 350.0)
    } else {
        (245.0, 300.0)
    };
    height_cm <= height_cap && vertical_delta_cm.abs() <= 300.0 && horizontal_cm <= reach
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prop(mass: f32) -> Target {
        Target {
            has_record: true,
            body: Some(Body::Prop),
            mass,
            ..Target::default()
        }
    }

    #[test]
    fn a_click_names_the_thing_and_whether_it_is_in_reach() {
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
        let mut heavy = prop(2.0);
        heavy.heavy = true;
        heavy.strength = 0.89;
        assert_eq!(classify(&heavy), Pick::Nothing);
        heavy.strength = 0.9;
        assert_eq!(classify(&heavy), Pick::Loose);

        let mut vial = prop(0.2);
        vial.usable = true;
        assert_eq!(classify(&vial), Pick::Usable);
        vial.body = None;
        assert_eq!(classify(&vial), Pick::Nothing);

        let scene = Target {
            body: Some(Body::Scene),
            ..Target::default()
        };
        assert_eq!(classify(&scene), Pick::Scene);

        assert!(!turns_before_use(70.0));
        assert!(turns_before_use(70.1));
        assert!(within_reach(245.0, 0.0, 300.0, false));
        assert!(!within_reach(245.1, 0.0, 300.0, false));
        assert!(!within_reach(200.0, 0.0, 300.1, false));
        assert!(within_reach(300.0, 0.0, 350.0, true));
        assert!(!within_reach(300.0, 300.1, 100.0, true));
    }
}
