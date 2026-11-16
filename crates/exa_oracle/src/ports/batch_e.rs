//! Ports registered by the third oracle lane (batch e): class C rows run
//! with their helpers followed or stubbed.

#[allow(unused_imports)]
use super::{Case, In, Out, Port};

/// An object's bytes of `size`, zero but for `fields` (offset, bytes).
fn object(size: usize, fields: &[(usize, &[u8])]) -> Vec<u8> {
    let mut out = vec![0u8; size];
    for &(at, bytes) in fields {
        out[at..at + bytes.len()].copy_from_slice(bytes);
    }
    out
}

/// The named field's bytes as a little-endian `u32`.
fn u32_of(b: &Case, name: &str) -> u32 {
    u32::from_le_bytes(b.field(name).try_into().expect("4 bytes"))
}

/// Three words' bytes.
fn words(v: [f32; 3]) -> Vec<u8> {
    v.iter().flat_map(|w| w.to_le_bytes()).collect()
}

/// The head look's angles: the kept yaw and the Euler angles the head's
/// matrix is rebuilt from (the matrix routines stubbed: the head's angles
/// come from `clip`, the built ones are recorded), helpless when the
/// condition flags have any of 0xAC0.
fn head_look(b: &Case) -> Vec<u8> {
    let flags = u32_of(b, "body.flags");
    let mut yaw = b.f32("body.yaw");
    let built = exa_physics::body::locomotion::head_look_angles(
        b.vec3("clip"),
        b.vec3("dir"),
        b.f32("weight"),
        &mut yaw,
        flags & 0xAC0 != 0,
        b.f32("body.offset"),
    );
    let mut out = object(
        0x11B4,
        &[
            (0xD0C, &yaw.to_le_bytes()),
            (0xDF4, &flags.to_le_bytes()),
            (0x11B0, b.field("body.offset")),
        ],
    );
    out.extend(words(built.to_array()));
    out
}

/// A light's update in the linear (mode byte set) or legacy workflow: the
/// node flags (no GI colour, no second colour), the stored colour,
/// multiplier, intensity, radius, running sum, pending bits and last
/// radius; the light's words as the update leaves them.
fn light_update(b: &Case) -> Vec<u8> {
    use exa_world::light_source::LightSource;
    let flags = u32_of(b, "light.flags");
    let mut light = LightSource::new(0, b.f32("light.intensity"));
    light.intensity = b.f32("light.intensity");
    light.multiplier = b.f32("light.mult");
    light.radius = b.f32("light.radius");
    light.last_radius = b.f32("light.last");
    light.offset = [b.f32("light.ox"), b.f32("light.oy"), b.f32("light.oz")];
    light.colour = [b.f32("light.red"), b.f32("light.green"), b.f32("light.blue")];
    light.dirty = u32_of(b, "light.dirty");
    let u = light.update(flags, b.u8("mode.linear") != 0);
    object(
        0x2A8,
        &[
            (0xC, &flags.to_le_bytes()),
            (0x1C8, &words(u.gi)),
            (0x1D8, &words(u.colour)),
            (0x1E8, &words(u.second)),
            (0x1FC, &words(light.offset)),
            (0x208, &u.luma.to_le_bytes()),
            (0x210, &words(light.colour)),
            (0x21C, &light.multiplier.to_le_bytes()),
            (0x220, &light.intensity.to_le_bytes()),
            (0x224, &light.radius.to_le_bytes()),
            (0x268, &light.dirty.to_le_bytes()),
            (0x278, &words(u.node_colour)),
            (0x2A4, &light.last_radius.to_le_bytes()),
        ],
    )
}

/// The batch's ports.
pub(super) const PORTS: &[Port] = &[
    Port {
        name: "head_look",
        run: head_look,
    },
    Port {
        name: "light_update",
        run: light_update,
    },
];
