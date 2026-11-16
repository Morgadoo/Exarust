//! The stab's aim and its arm drive.
//!
//! A stab does not only play its clip. While the clip runs, the fighter's
//! behaviour keeps a stab aim, and the body tick pulls the weapon onto the
//! line from a point by the shoulder to that aim and drives it along the
//! line, so the point of a long weapon arrives first.
//!
//! The AI's aim ([`StabAim`]) is its head tracking's state: it starts as the
//! stab starts on the yaw of the body's look smoothing, level, and the
//! combat state's sweep runs the head tracking ([`crate::aim::track_head`])
//! every tick a stab plays: it turns toward the AI's estimate by at most
//! π/100 a tick in yaw and in pitch, looks up by no more than 0.6 rad and
//! keeps within 45° of the body's facing, from an eye 20 cm to the side of
//! and 10 cm above the character frame's origin; the aim point lies 250 cm
//! out along it.
//!
//! The drive runs from clock 0.1 while the stab has a held weapon and has
//! not landed a blow: the body tick's handling pushes
//! ([`crate::weapon::handling_pushes`]) move the weapon point 0.9 up the
//! first held item and the right hand across onto the line from the anchor
//! to the aim point, and past clock 0.4 along it.
//!
//! Evidence: the AI behaviour's stab-aim slot and the body tick's stab arm
//! drive (REA, `Exanima.exe`).

use bevy::prelude::*;
use exa_anim::quick_direction;

/// The anchor the aim and the drive measure from, in the character
/// frame (cm): 20 cm to the side, 10 cm above the frame's origin.
pub const ANCHOR_CM: Vec3 = Vec3::new(-20.0, 10.0, 0.0);
/// The weapon point sits this share of the item's length up from its grip.
pub const ITEM_POINT_SHARE: f32 = 0.9;

/// A fighter's stab aim, which its behaviour keeps while a stab plays and
/// its body's stab drive reads.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub struct StabAim {
    /// The aim's yaw (radians, 0 along +z, toward +x).
    pub yaw: f32,
    /// The aim's pitch (radians, positive down).
    pub pitch: f32,
    /// The aim point the drive pulls toward (world, cm), once aimed.
    pub point_cm: Option<Vec3>,
}

impl StabAim {
    /// The aim as an attack starts: on the attack's aim yaw, level, with
    /// no point yet.
    pub fn start(aim_yaw: f32) -> Self {
        Self {
            yaw: aim_yaw,
            pitch: 0.0,
            point_cm: None,
        }
    }

    /// The aim's unit direction.
    pub fn direction(&self) -> Vec3 {
        quick_direction(self.pitch, self.yaw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The aim starts on the given yaw, level, with no point; its direction
    /// is the engine's quick direction of the pitch and yaw.
    #[test]
    fn the_aim_starts_level_on_its_yaw() {
        let aim = StabAim::start(0.5);
        assert_eq!((aim.yaw, aim.pitch, aim.point_cm), (0.5, 0.0, None));
        assert_eq!(aim.direction(), quick_direction(0.0, 0.5));
    }
}
