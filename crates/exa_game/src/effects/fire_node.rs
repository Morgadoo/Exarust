//! The fire node: the flame material's geometry with a fire row and the
//! light and loop sound a burning held item carries.
//!
//! The node is a geometry node with one sub-object (kind 35044) drawn with
//! the object's `_FlameMat01` (registered from a default material record
//! when the library lacks it), and a [`super::fire_row::FlameRow`] bound to
//! it. Its light is a point light (kind 3, flags 0x14_0100, emitter flags
//! 0x9000), warm white (1, 0.68, 0.43), 800 cm, softness 4, size 1,
//! intensity 0.01 to start and 5 at full, attached to the node and
//! handed to the row; the row's sound is `FireS02`.
//!
//! Evidence: the node builder and its light-and-sound pass, read from
//! `Exanima.exe`. The row's own simulation is [`super::fire_row`].

use super::fire_row;

/// The flame geometry's material.
pub const MATERIAL: &str = "_FlameMat01";
/// The node's sub-object kind word.
pub const SUB_OBJECT_KIND: i32 = 35_044;
/// The light's kind word: a point light.
pub const LIGHT_KIND: i32 = fire_row::LIGHT_KIND;
/// The light's flags and extra words, as the node sets them.
pub const LIGHT_FLAGS: u32 = fire_row::LIGHT_FLAGS;
pub const LIGHT_EMITTER_FLAGS: u32 = fire_row::LIGHT_EMITTER_FLAGS;
pub const LIGHT_EXTRA_ONE: bool = fire_row::LIGHT_EXTRA_ONE;
pub const LIGHT_EXTRA_TWO: u32 = fire_row::LIGHT_EXTRA_TWO;
/// The light's colour, its radius in cm, its softness, its start and full
/// intensity.
pub const LIGHT_COLOUR: [f32; 3] = fire_row::LIGHT_COLOUR;
pub const LIGHT_RADIUS_CM: f32 = fire_row::LIGHT_RADIUS_CM;
pub const LIGHT_SOFTNESS: f32 = fire_row::LIGHT_SOFTNESS;
pub const LIGHT_START_INTENSITY: f32 = fire_row::LIGHT_START_INTENSITY;
pub const LIGHT_FULL_INTENSITY: f32 = fire_row::LIGHT_BASE_INTENSITY;
/// The light is one unit large.
pub const LIGHT_SIZE: f32 = 1.0;
/// The loop sound.
pub const LOOP_SOUND: &str = "FireS02";

/// What a fire node adds to the world: its light and its loop.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FireLight {
    pub colour: [f32; 3],
    pub radius_cm: f32,
    pub softness: f32,
    pub intensity: f32,
    pub kind: i32,
    pub flags: u32,
    pub emitter_flags: u32,
    pub size: f32,
}

/// The flame node's light, as the node creates it: intensity 0.01, 800 cm,
/// softness 4.
pub fn flame_light() -> FireLight {
    FireLight {
        colour: LIGHT_COLOUR,
        radius_cm: LIGHT_RADIUS_CM,
        softness: LIGHT_SOFTNESS,
        intensity: LIGHT_START_INTENSITY,
        kind: LIGHT_KIND,
        flags: LIGHT_FLAGS,
        emitter_flags: LIGHT_EMITTER_FLAGS,
        size: LIGHT_SIZE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_flame_node_is_the_flame_material() {
        assert_eq!(MATERIAL, "_FlameMat01");
        assert_eq!(SUB_OBJECT_KIND, 35_044);
    }

    #[test]
    fn the_flame_light_is_warm_and_eight_metres_across() {
        let light = flame_light();
        assert_eq!(light.colour, [1.0, 0.68, 0.43]);
        assert_eq!(light.radius_cm, 800.0);
        assert_eq!(light.softness, 4.0);
        assert_eq!(light.intensity, 0.01);
        assert_eq!(light.kind, 3);
        assert_eq!(light.flags, 0x0014_0100);
        assert_eq!(light.emitter_flags, 0x9000);
        assert_eq!(light.size, 1.0);
        // 0.68 and 0.43 are the original's float literals.
        assert_eq!(light.colour[1], f32::from_bits(0x3f2e_147b));
        assert_eq!(light.colour[2], f32::from_bits(0x3edc_28f6));
        assert_eq!(LOOP_SOUND, "FireS02");
        assert_eq!(LIGHT_FULL_INTENSITY, 5.0);
    }
}
