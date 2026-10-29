//! A weapon that bites sticks in the part it hit.
//!
//! An armed bite above 0.1 appends a link. The depth is capped at 1 before
//! the flags, the strength, and the lifetime, so a depth above 2 never
//! reaches the longer counts. The flags compare that capped depth, widened
//! to a double, with 0.4. Strength is 12 times the depth. Smoothing is the
//! zone smoothing plus 0.5. The count is 3 times the product of 83, the
//! unit and the depth, truncated toward zero. The span is twice that
//! product. The weapon's layer becomes 0xC0. The struck part, its parent
//! and each child gain bit 0x40.
//!
//! Evidence: the bite link built after an armed hit.

/// Flags when the widened depth is not above 0.4.
const SHALLOW: u32 = 0x11000;
/// Flags when the widened depth is above 0.4.
const DEEP: u32 = 0x211000;
/// The weapon's layer while it is stuck.
const WEAPON_LAYER: u32 = 0xC0;
/// The bit added to the struck part, its parent and its children.
const PART_BIT: u32 = 0x40;
/// Body ticks in one second, the lifetime's unit.
const TICKS: i64 = 83;

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
/// `unit`. An unarmed blow, and a depth of 0.1 or less, sticks nothing.
pub fn stick(armed: bool, depth: f32, smoothing: f32, unit: i32) -> Option<Stick> {
    if !armed || depth <= 0.1 {
        return None;
    }
    let held = depth.min(1.0);
    let flags = if f64::from(held) > 0.4 { DEEP } else { SHALLOW };
    let ticks = (TICKS * i64::from(unit)) as f32;
    let scaled = ticks * held;
    Some(Stick {
        flags,
        strength: held * 12.0,
        smoothing: smoothing + 0.5,
        count: (scaled as i32).wrapping_mul(3),
        span: scaled * 2.0,
        weapon_layer: WEAPON_LAYER,
        pull: 1.0,
    })
}

/// The grab the body solver pulls, opened from a stick link between a
/// weapon of link mobility `weapon` and a struck part of `struck`: the
/// shares are the original's share routine over the two.
pub fn grab_of(link: &Stick, weapon: f32, struck: f32) -> exa_physics::body::GrabPull {
    exa_physics::body::GrabPull {
        strain: link.span,
        flags: link.flags,
        strength: link.pull,
        axis: bevy::math::Vec3::ZERO,
        on_weapon: [1.0, 0.0, 0.0, 0.0],
        on_struck: [1.0, 0.0, 0.0, 0.0],
        shares: exa_physics::body::grab_shares(weapon, struck),
    }
}

/// Holds one bite inside the body solver's link passes.
pub struct BiteHold {
    pub grab: Option<exa_physics::body::GrabPull>,
}

impl exa_physics::body::SolverStage for BiteHold {
    fn links(&mut self, bodies: &mut [&mut exa_physics::body::Ragdoll], _: usize) {
        let Some(grab) = self.grab.as_mut() else {
            return;
        };
        // No Exarust body is disabled yet: the struck part never is.
        if let Some(body) = bodies.first_mut()
            && body.pull_grab(0, 1, grab, false)
        {
            let mut held = vec![(1, grab.clone())];
            body.release_grabs(0, 1, &mut held);
            self.grab = None;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bite_above_a_tenth_sticks_the_weapon() {
        assert!(stick(false, 1.0, 0.1, 1).is_none());
        assert!(stick(true, 0.1, 0.1, 1).is_none());
        let low = stick(true, 0.25, 0.1, 1).unwrap();
        assert_eq!(low.flags, 0x11000);
        assert_eq!(low.strength.to_bits(), 0x4040_0000);
        assert_eq!(low.smoothing.to_bits(), 0x3f19_999a);
        assert_eq!(low.count, 60);
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

    fn pair(gap: f32) -> exa_physics::body::Ragdoll {
        let at =
            |x: f32| bevy::math::Affine3A::from_translation(bevy::math::Vec3::new(x, 1.0, 0.0));
        exa_physics::body::Ragdoll::new(&[
            exa_physics::body::PartSetup {
                bone: 1,
                entity: bevy::prelude::Entity::PLACEHOLDER,
                parent: None,
                world: at(0.0),
                centre: bevy::math::Vec3::ZERO,
                half: bevy::math::Vec3::splat(0.05),
                origin: bevy::math::Vec3::ZERO,
                links: vec![],
                foot: None,
                pins: Vec::new(),
                spheres: None,
            },
            exa_physics::body::PartSetup {
                bone: 2,
                entity: bevy::prelude::Entity::PLACEHOLDER,
                parent: None,
                world: at(gap),
                centre: bevy::math::Vec3::ZERO,
                half: bevy::math::Vec3::splat(0.05),
                origin: bevy::math::Vec3::ZERO,
                links: vec![],
                foot: None,
                pins: Vec::new(),
                spheres: None,
            },
        ])
    }

    fn gap_of(body: &exa_physics::body::Ragdoll) -> f32 {
        (body.parts[0].positions[0] - body.parts[1].positions[0]).length()
    }

    #[test]
    fn a_bite_above_a_tenth_is_held_by_the_solver() {
        let mut held = pair(0.08);
        let before = gap_of(&held);
        let link = stick(true, 0.25, 0.1, 1).expect("a depth past a tenth sticks");
        let mut stages: Vec<Box<dyn exa_physics::body::SolverStage>> = vec![Box::new(BiteHold {
            grab: Some(grab_of(&link, 1.0, 1.0)),
        })];
        exa_physics::body::solve_with(
            &mut [&mut held],
            &[] as &[[bevy::math::Vec3; 3]],
            &mut stages,
        );
        let after = gap_of(&held);
        assert!(after < before * 0.25, "{after} stayed near {before}");

        let mut free = pair(0.08);
        let open = gap_of(&free);
        assert!(stick(true, 0.1, 0.1, 1).is_none());
        let mut stages: Vec<Box<dyn exa_physics::body::SolverStage>> =
            vec![Box::new(BiteHold { grab: None })];
        exa_physics::body::solve_with(
            &mut [&mut free],
            &[] as &[[bevy::math::Vec3; 3]],
            &mut stages,
        );
        let still = gap_of(&free);
        assert!((still - open).abs() < 0.001, "{still} moved from {open}");
    }
}
