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
//! A placed object with a fire effect creates the same node
//! ([`placed_flame`]): the `_FlameMat01` material, the light and loop, the
//! node's 0x9000 and 0x14_0100 flags and the two words set to 1. The
//! object's colour-ramp byte becomes the flame's two colour values through
//! [`flame_ramp`]. The object-to-descriptor conversion's template words
//! (0x11/0x12/0x13/0x14/0x22/0x23/0x32/0x421/0x521 by kind) are the model
//! factory's, outside this node.
//!
//! Evidence: the node builder, its light-and-sound pass and the placed
//! object's flame creation, read from `Exanima.exe`. The node's own GL draw
//! is not ported; the row's simulation is [`super::fire_row`].

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

/// The node flags and second flag word a placed object's flame is built
/// with.
pub const NODE_FLAGS: u32 = 0x9000;
pub const NODE_EXTRA_FLAGS: u32 = 0x0014_0100;
/// The node's render word and the effect word its creation sets.
pub const NODE_RENDER_ONE: u32 = 1;
pub const NODE_EFFECT_ONE: u32 = 1;

/// The flame's two colour-ramp bytes from a placed object's record byte:
/// below 0x47 the pair (20, 200); below 0x5A `(b − 10)·2` and `(b + 10)·2`;
/// from 0x5A `(b·2 − 100)·2` and 200. The arithmetic is the original's
/// single-byte one and wraps.
pub fn flame_ramp(byte: u8) -> (u8, u8) {
    if byte < 0x47 {
        (0x14, 0xc8)
    } else if byte < 0x5a {
        (
            byte.wrapping_sub(10).wrapping_mul(2),
            byte.wrapping_add(10).wrapping_mul(2),
        )
    } else {
        (byte.wrapping_mul(2).wrapping_sub(100).wrapping_mul(2), 0xc8)
    }
}

/// What a placed object's flame is: its node's material, its light, its loop
/// and its flags.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlacedFlame {
    pub material: &'static str,
    pub sub_object_kind: i32,
    pub light: FireLight,
    pub sound: &'static str,
    pub node_flags: u32,
    pub node_extra_flags: u32,
    pub render_one: u32,
    pub effect_one: u32,
}

/// The flame a placed object creates: `_FlameMat01`, a point light at
/// intensity 0.01 to start and 5 at full, the node's 0x9000 and 0x14_0100
/// words and the `FireS02` loop.
pub fn placed_flame() -> PlacedFlame {
    PlacedFlame {
        material: MATERIAL,
        sub_object_kind: SUB_OBJECT_KIND,
        light: flame_light(),
        sound: LOOP_SOUND,
        node_flags: NODE_FLAGS,
        node_extra_flags: NODE_EXTRA_FLAGS,
        render_one: NODE_RENDER_ONE,
        effect_one: NODE_EFFECT_ONE,
    }
}

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

    #[test]
    fn a_placed_object_builds_the_same_flame_node() {
        let flame = placed_flame();
        assert_eq!(flame.material, "_FlameMat01");
        assert_eq!(flame.sub_object_kind, 35_044);
        assert_eq!(flame.light, flame_light());
        assert_eq!(flame.sound, "FireS02");
        assert_eq!(
            (flame.node_flags, flame.node_extra_flags),
            (0x9000, 0x14_0100)
        );
        assert_eq!((flame.render_one, flame.effect_one), (1, 1));
    }

    #[test]
    fn the_flame_ramp_follows_the_records_byte() {
        assert_eq!(flame_ramp(0x00), (20, 200));
        assert_eq!(flame_ramp(0x46), (20, 200));
        // 0x47: (0x3d)·2 = 122 and 0x51·2 = 162.
        assert_eq!(flame_ramp(0x47), (122, 162));
        // 0x59: 79·2 = 158 and 99·2 = 198.
        assert_eq!(flame_ramp(0x59), (158, 198));
        // 0x5A: 180 − 100 = 80, ×2 = 160 and the fixed 200.
        assert_eq!(flame_ramp(0x5a), (160, 200));
        // The byte's arithmetic wraps: 0x7f gives 308 → 52.
        assert_eq!(flame_ramp(0x7f), (52, 200));
    }
}
