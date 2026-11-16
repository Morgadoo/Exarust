//! Ports registered by the second oracle lane (batch c): colours, render
//! and camera maths, the GUI's helpers, game effects and the fight AI's
//! small routines.

#[allow(unused_imports)]
use super::{Case, In, Out, Port};

/// The named field's bytes as a little-endian `u16`.
fn u16_of(b: &Case, name: &str) -> u16 {
    let f = b.field(name);
    u16::from_le_bytes([f[0], f[1]])
}

/// The named field's bytes as a little-endian `u32`.
fn u32_of(b: &Case, name: &str) -> u32 {
    let f = b.field(name);
    u32::from_le_bytes([f[0], f[1], f[2], f[3]])
}

/// An object's bytes of `size`, zero but for `fields` (offset, bytes): the
/// out buffer of a spec laid out by offset.
fn object(size: usize, fields: &[(usize, &[u8])]) -> Vec<u8> {
    let mut out = vec![0u8; size];
    for &(at, bytes) in fields {
        out[at..at + bytes.len()].copy_from_slice(bytes);
    }
    out
}

/// Four floats to RGBA8 bytes (the swatch write).
fn rgba8_pack(b: &Case) -> Vec<u8> {
    let c = [b.f32("c.r"), b.f32("c.g"), b.f32("c.b"), b.f32("c.a")];
    Out::default()
        .bytes(&exa_ui::colour_picker::packed_bytes(c))
        .0
}

/// RGBA8 bytes to four floats (the swatch read).
fn rgba8_unpack(b: &Case) -> Vec<u8> {
    let c = [b.u8("c.r"), b.u8("c.g"), b.u8("c.b"), b.u8("c.a")];
    Out::default().f32s(&exa_ui::colour_picker::unpacked(c)).0
}

/// A packed word's three colour bytes to floats.
fn rgb8_unpack(b: &Case) -> Vec<u8> {
    let word = u32::from_le_bytes([b.u8("c.r"), b.u8("c.g"), b.u8("c.b"), b.u8("c.a")]);
    Out::default()
        .f32s(&exa_formats::colour::unpack_rgb(word))
        .0
}

/// Three floats to the first three bytes; the fourth is kept.
fn rgb8_pack(b: &Case) -> Vec<u8> {
    let word = exa_formats::colour::pack_rgb([b.f32("c.r"), b.f32("c.g"), b.f32("c.b")]);
    let bytes = word.to_le_bytes();
    Out::default()
        .bytes(&[bytes[0], bytes[1], bytes[2], b.u8("out.a")])
        .0
}

/// A company entrant's starting health from its saved word: red and
/// yellow both set (the body laid out by offset: yellow, red, the max, the
/// record pointer).
fn start_health(b: &Case) -> Vec<u8> {
    let max = b.f32("body.max");
    let h = exa_game::arena_match::health_from_word(u16_of(b, "rec.word"), max);
    let h = h.to_le_bytes();
    object(
        0x10A8,
        &[(0xD58, &h), (0xD5C, &h), (0x103C, &max.to_le_bytes())],
    )
}

/// The chase camera preset's field of view scale.
fn fov_scale(b: &Case) -> Vec<u8> {
    let scale = exa_game::nav_camera::fov_scale(b.f32("value"));
    object(0x150, &[(0x148, &scale.to_le_bytes())])
}

/// A contact's concern store with its event bits.
fn contact_set_concern(b: &Case) -> Vec<u8> {
    let mut c = exa_ai::combatant::Contact::new(bevy::prelude::Entity::PLACEHOLDER);
    c.concern = b.f32("contact.concern");
    c.events = u32_of(b, "contact.events");
    c.set_concern(b.f32("concern"));
    object(
        0x58,
        &[
            (0x1C, &c.concern.to_le_bytes()),
            (0x50, &c.events.to_le_bytes()),
        ],
    )
}

/// The run-time library's `Random` real from one twister word.
fn rtl_random_unit(b: &Case) -> Vec<u8> {
    let draw = u32_of(b, "draw.v");
    Out::default()
        .bytes(&exa_formats::rtl_random::scale_unit(draw).to_le_bytes())
        .0
}

/// The scene presets' sRGB-to-linear of three channels in place.
fn srgb_linearise_in_place(b: &Case) -> Vec<u8> {
    let c = [b.f32("c.r"), b.f32("c.g"), b.f32("c.b")];
    Out::default()
        .f32s(&c.map(exa_world::presets::srgb_to_linear))
        .0
}

/// The material's own sRGB-to-linear (`powf`) against the same routine.
fn srgb_linearise_powf(b: &Case) -> Vec<u8> {
    let c = [b.f32("c.r"), b.f32("c.g"), b.f32("c.b")];
    Out::default().vec3(exa_render::material::linear_rgb(c)).0
}

/// The scene environment's sRGB-to-linear into another colour.
fn srgb_linearise_into(b: &Case) -> Vec<u8> {
    let c = [b.f32("c.r"), b.f32("c.g"), b.f32("c.b")];
    Out::default().f32s(&exa_world::scene_env::linearise(c)).0
}

/// The shadow box's orthographic matrix, column-major.
fn ortho_matrix(b: &Case) -> Vec<u8> {
    let m = exa_render::sun_fit::orthographic_matrix(b.vec3("min"), b.vec3("max"));
    Out::default().f32s(&m.to_cols_array()).0
}

/// The light's view matrix from the origin, travel direction and up hint.
fn light_view_matrix(b: &Case) -> Vec<u8> {
    let m = exa_render::sun_fit::light_view_matrix(b.vec3("dir"), b.vec3("up"), b.vec3("origin"));
    Out::default().f32s(&m.to_cols_array()).0
}

/// A view-space point to window pixels and depth through the viewport's
/// integer size and its camera's projection scales.
fn view_to_screen(b: &Case) -> Vec<u8> {
    let i32_of = |n: &str| u32_of(b, n) as i32;
    let size = (i32_of("view.w") as f32, i32_of("view.h") as f32);
    let p = [b.f32("p.x"), b.f32("p.y"), b.f32("p.z")];
    let s = exa_ui::viewport::view_to_screen(p, size, (b.f32("cam.sx"), b.f32("cam.sy")));
    Out::default().f32s(&s).0
}

/// The item card's rescale of its icon-area height (the unit global set
/// by the stubbed base rescale).
fn desc_rescale(b: &Case) -> Vec<u8> {
    let i32_of = |n: &str| u32_of(b, n) as i32;
    let h =
        exa_ui::popups::desc_rescale(i32_of("card.height"), i32_of("parent"), i32_of("gui.unit"));
    object(0x178, &[(0x170, &h.to_le_bytes())])
}

/// The loyalty marker's trust change and limit test (the relation from
/// the stubbed lookup).
fn trust_holds(b: &Case) -> Vec<u8> {
    let mut trust = b.f32("marker.trust");
    let learnt = u32_of(b, "owner.learnt") & 0x10000 != 0;
    let holds = exa_game::thaum::power::mind::trust_holds(
        &mut trust,
        b.f32("amount"),
        b.f32("rel.v"),
        learnt,
    );
    let mut out = object(0x38, &[(0x30, &trust.to_le_bytes())]);
    out.push(u8::from(holds));
    out
}

/// A drawable node's shadow-depth scales from its render flags (the cast
/// scale first in the node, then the receive scale).
fn shadow_bias_scales(b: &Case) -> Vec<u8> {
    let (receive, cast) = exa_render::lights::shadow_bias_scales(u32_of(b, "node.flags"));
    let flags = u32_of(b, "node.flags").to_le_bytes();
    object(
        0x230,
        &[
            (0x204, &flags),
            (0x224, &cast.to_le_bytes()),
            (0x228, &receive.to_le_bytes()),
        ],
    )
}

/// The bloom filter's weights for the array's count, the untouched rest of
/// the array kept.
fn bloom_weights(b: &Case, linear: bool) -> Vec<u8> {
    let n = (u32_of(b, "arr.n") as i32).max(0) as usize;
    let weights = exa_render::post::bloom_weights(n, b.f32("level"), linear);
    // the array record (16 bytes) and the level, then the 40 weights
    let mut data = b.bytes[20..180].to_vec();
    for (i, w) in weights.iter().enumerate() {
        data[4 * i..4 * i + 4].copy_from_slice(&w.to_le_bytes());
    }
    data
}

fn bloom_weights_linear(b: &Case) -> Vec<u8> {
    bloom_weights(b, true)
}

fn bloom_weights_legacy(b: &Case) -> Vec<u8> {
    bloom_weights(b, false)
}

/// The parchment's hit test at a click (the GUI unit 37): the texel the
/// stubbed sample was asked for, the form and control id on a hit, and the
/// answer.
fn parchment_hit(b: &Case) -> Vec<u8> {
    let i32_of = |n: &str| u32_of(b, n) as i32;
    let (tx, ty) = exa_ui::popups::parchment_texel((i32_of("x"), i32_of("y")), 37);
    let alpha = b.u8("probe.alpha");
    let hit = exa_ui::popups::parchment_hit(alpha, i32_of("control.id"));
    let id = hit.unwrap_or(i32_of("hit_id.id"));
    let answer = if hit.is_some() {
        exa_ui::popups::PARCHMENT_HIT
    } else {
        0
    };
    let mut out = vec![alpha, 0, 0, 0];
    out.extend(tx.to_le_bytes());
    out.extend(ty.to_le_bytes());
    out.extend([0; 8]);
    out.extend(id.to_le_bytes());
    out.extend(answer.to_le_bytes());
    out
}

/// The fight AI's keep-off clamp of a step offset (chest points in x and
/// z, the foe's scale).
fn keep_off(b: &Case) -> Vec<u8> {
    use bevy::math::Vec3;
    let me = Vec3::new(b.f32("me.x"), 0.0, b.f32("me.z"));
    let foe = Vec3::new(b.f32("foe.x"), 0.0, b.f32("foe.z"));
    let mut off = b.vec3("off");
    exa_ai::combat::keep_off(me, foe, b.f32("foe.scale"), &mut off);
    Out::default().vec3(off).0
}

/// A blast's or a wave's assessment: the viewer's body (chest, look,
/// facing), the effect's place and release word, the relation from the
/// stubbed lookup; the record's words, then the original's answer (0 when
/// seen or left out, −1 out of sight).
fn assess(b: &Case, wave: bool) -> Vec<u8> {
    use bevy::math::Vec3;
    use exa_game::effects::danger;
    let v = |p: &str| {
        Vec3::new(
            b.f32(&format!("body.{p}x")),
            b.f32(&format!("body.{p}y")),
            b.f32(&format!("body.{p}z")),
        )
    };
    let viewer = danger::Viewer {
        chest: v("c"),
        look: v("l"),
        facing: v("f"),
    };
    let at = Vec3::new(b.f32("eff.x"), b.f32("eff.y"), b.f32("eff.z"));
    let released = (u32_of(b, "eff.released") as i32) > 0;
    let mut seen = danger::Seen {
        flags: u32_of(b, "seen.flags"),
        alarm: b.f32("seen.alarm"),
        near: b.f32("seen.near"),
        reach: b.f32("seen.reach"),
        relation: b.f32("seen.relation"),
        fear: b.f32("seen.fear"),
        urge: b.f32("seen.urge"),
        face: b.f32("seen.face"),
    };
    let relation = b.f32("rel.v");
    let shown = if wave {
        danger::assess_wave(at, released, &viewer, relation, &mut seen)
    } else {
        danger::assess_blast(at, released, &viewer, false, relation, &mut seen)
    };
    let mut out = object(
        0x44,
        &[
            (0x00, &seen.flags.to_le_bytes()),
            (0x10, &seen.alarm.to_le_bytes()),
            (0x14, &seen.near.to_le_bytes()),
            (0x20, &seen.reach.to_le_bytes()),
            (0x28, &seen.relation.to_le_bytes()),
            (0x30, &seen.fear.to_le_bytes()),
            (0x3C, &seen.urge.to_le_bytes()),
            (0x40, &seen.face.to_le_bytes()),
        ],
    );
    out.extend(if shown { 0i32 } else { -1 }.to_le_bytes());
    out
}

fn assess_blast(b: &Case) -> Vec<u8> {
    assess(b, false)
}

fn assess_wave(b: &Case) -> Vec<u8> {
    assess(b, true)
}

/// A key track's Catmull-Rom position at a time: the keys up to the
/// controller's last index; the node keeps its position when no key
/// brackets the time.
fn catmull_position(b: &Case) -> Vec<u8> {
    use exa_formats::key_track::{KeyTrack, PlayMode, PositionKey};
    let last = (u32_of(b, "ctl.last") as i32).clamp(0, 5) as usize;
    let keys = (0..=last)
        .map(|i| PositionKey {
            time: b.f32(&format!("keys.t{i}")),
            position: [
                b.f32(&format!("keys.x{i}")),
                b.f32(&format!("keys.y{i}")),
                b.f32(&format!("keys.z{i}")),
            ],
        })
        .collect();
    let track = KeyTrack {
        mode: PlayMode::Free,
        duration: 0.0,
        keys,
    };
    let kept = [b.f32("out.x"), b.f32("out.y"), b.f32("out.z")];
    let p = track.position_at(b.f32("time")).unwrap_or(kept);
    Out::default().f32s(&p).0
}

/// One frame of a light's flicker or fade timer (the generator's two draws
/// from the case, the bundle refresh and colour update stubbed): the light's
/// words, then its node's shown fixture scale and the fixture part's.
fn flicker_step(b: &Case) -> Vec<u8> {
    let mut light = exa_world::light_source::LightSource::new(u32_of(b, "light.flags"), 0.0);
    light.intensity = b.f32("light.intensity");
    light.dirty = u32_of(b, "light.dirty");
    light.base_intensity = b.f32("light.base");
    light.speed = b.f32("light.speed");
    light.amplitude = b.f32("light.amplitude");
    light.emissive = b.f32("light.emissive");
    light.scale = b.f32("light.scale");
    light.flicker = b.f32("light.flicker");
    let draws = [b.f32("rng.r0"), b.f32("rng.r1")];
    let mut n = 0;
    let mut random = || {
        let r = draws[n.min(1)];
        n += 1;
        r
    };
    light.flicker_step(&mut random, b.f32("clock.frame_ms"));
    let le = |v: f32| v.to_le_bytes();
    let mut out = object(
        0x2E0,
        &[
            (0x20C, &light.flags.to_le_bytes()),
            (0x220, &le(light.intensity)),
            (0x268, &light.dirty.to_le_bytes()),
            (0x2C8, &le(light.base_intensity)),
            (0x2CC, &le(light.speed)),
            (0x2D0, &le(light.amplitude)),
            (0x2D4, &le(light.emissive)),
            (0x2D8, &le(light.scale)),
            (0x2DC, &le(light.flicker)),
        ],
    );
    let kind = u32_of(b, "node.kind");
    let fixture = kind & 0x5F == 9;
    let shown = if fixture {
        light.fixture_scale
    } else {
        b.f32("node.shown")
    };
    out.extend(object(
        0x248,
        &[(0x08, &kind.to_le_bytes()), (0x244, &le(shown))],
    ));
    let part_flags = u32_of(b, "part.flags") | if fixture { 0x10 } else { 0 };
    let part_fixture = if fixture {
        light.fixture_scale
    } else {
        b.f32("part.fixture")
    };
    out.extend(object(
        0xF8,
        &[(0xF0, &part_flags.to_le_bytes()), (0xF4, &le(part_fixture))],
    ));
    out
}

/// The body tick's two focus settles, the reserve's then the pool's (the
/// status broadcast between them stubbed).
fn focus_settle(b: &Case) -> Vec<u8> {
    use exa_game::thaum::casting;
    let mut body = casting::CastBody {
        techniques: u32_of(b, "body.techniques"),
        focus: b.f32("body.focus"),
        capacity: b.f32("body.capacity"),
        settle_rate: b.f32("body.settle_rate"),
        spent: b.f32("body.spent"),
        reserved: b.f32("body.reserved"),
        capacity_rate: b.f32("body.capacity_rate"),
        regen_ramp: b.f32("body.regen_ramp"),
        regen_bonus: b.f32("body.regen_bonus"),
        ..Default::default()
    };
    casting::settle_capacity(&mut body);
    casting::settle_focus(&mut body);
    let le = |v: f32| v.to_le_bytes();
    object(
        0x1308,
        &[
            (0xC40, &body.techniques.to_le_bytes()),
            (0xD74, &le(body.focus)),
            (0xD78, &le(body.capacity)),
            (0xD7C, &le(body.settle_rate)),
            (0xD80, &le(body.spent)),
            (0xD84, &le(body.reserved)),
            (0xD88, &le(body.capacity_rate)),
            (0xDF8, &u32_of(b, "body.flags").to_le_bytes()),
            (0x1014, &le(body.regen_ramp)),
            (0x1304, &le(body.regen_bonus)),
        ],
    )
}

/// The view's projection half extents (stored negated) and its view range
/// from the camera's field of view, near and far distances.
fn frustum_half_extents(b: &Case) -> Vec<u8> {
    let (w, h) = exa_render::view::frustum_half_extents(
        b.f32("cam.fov"),
        b.f32("view.aspect"),
        b.f32("cam.near"),
    );
    let range = exa_render::view::view_range(b.f32("cam.far"));
    object(
        0x130,
        &[
            (0x20, &b.f32("view.aspect").to_le_bytes()),
            (0x120, &range.to_le_bytes()),
            // stored negated (the port answers them positive)
            (0x124, &(-w).to_le_bytes()),
            (0x128, &(-h).to_le_bytes()),
        ],
    )
}

/// The form's wheel: the setter it calls (1 vertical, 2 horizontal, 0
/// none) and the value it passes.
fn wheel_scroll(b: &Case) -> Vec<u8> {
    use exa_ui::form::WheelScroll;
    let scroll = exa_ui::form::wheel_scroll(
        u32_of(b, "form.attributes"),
        u32_of(b, "delta") as i32,
        (b.f32("form.hscroll"), b.f32("form.vscroll")),
        (b.f32("form.hstep"), b.f32("form.vstep")),
    );
    let (which, value) = match scroll {
        WheelScroll::Vertical(v) => (1u32, v),
        WheelScroll::Horizontal(v) => (2, v),
        WheelScroll::None => (u32_of(b, "probe.which"), b.f32("probe.value")),
    };
    let mut out = which.to_le_bytes().to_vec();
    out.extend(value.to_le_bytes());
    out
}

/// A spot light's vertical shaping (y, z) from its spot values (q, p).
fn light_spot_shaping(b: &Case) -> Vec<u8> {
    let (q, p) = (b.f32("light.q"), b.f32("light.p"));
    let light = exa_formats::rfc::Light {
        flags: 0,
        color: [0.0; 3],
        multiplier: 0.0,
        intensity: 0.0,
        radius: 0.0,
        softness: 0.0,
        spot: Some([q, p]),
        ranged: false,
        animation: None,
    };
    let (y, z) = exa_world::vertical_shaping(&light);
    object(
        0x2E8,
        &[
            (0x240, &q.to_le_bytes()),
            (0x244, &p.to_le_bytes()),
            (0x24C, &y.to_le_bytes()),
            (0x250, &z.to_le_bytes()),
        ],
    )
}

/// A hit's rumble on both motors (starting at 0.7 and 0.35): the motor
/// words the vibration call gets, then the motors.
fn rumble_hit(b: &Case) -> Vec<u8> {
    let mut rumble = exa_game::gamepad::Rumble {
        first: 0.7,
        second: 0.35,
    };
    rumble.add(b.f32("amount"));
    let (first, second) = rumble.motors();
    // the first motor's word goes in the first argument register
    let mut out = u32::from(first).to_le_bytes().to_vec();
    out.extend(u32::from(second).to_le_bytes());
    out.extend(rumble.first.to_le_bytes());
    out.extend(rumble.second.to_le_bytes());
    out
}

pub(super) const PORTS: &[Port] = &[
    Port {
        name: "rgba8_pack",
        run: rgba8_pack,
    },
    Port {
        name: "rgba8_unpack",
        run: rgba8_unpack,
    },
    Port {
        name: "rgb8_unpack",
        run: rgb8_unpack,
    },
    Port {
        name: "rgb8_pack",
        run: rgb8_pack,
    },
    Port {
        name: "start_health",
        run: start_health,
    },
    Port {
        name: "fov_scale",
        run: fov_scale,
    },
    Port {
        name: "contact_set_concern",
        run: contact_set_concern,
    },
    Port {
        name: "rtl_random_unit",
        run: rtl_random_unit,
    },
    Port {
        name: "srgb_linearise_in_place",
        run: srgb_linearise_in_place,
    },
    Port {
        name: "srgb_linearise_powf",
        run: srgb_linearise_powf,
    },
    Port {
        name: "srgb_linearise_into",
        run: srgb_linearise_into,
    },
    Port {
        name: "ortho_matrix",
        run: ortho_matrix,
    },
    Port {
        name: "light_view_matrix",
        run: light_view_matrix,
    },
    Port {
        name: "view_to_screen",
        run: view_to_screen,
    },
    Port {
        name: "desc_rescale",
        run: desc_rescale,
    },
    Port {
        name: "trust_holds",
        run: trust_holds,
    },
    Port {
        name: "shadow_bias_scales",
        run: shadow_bias_scales,
    },
    Port {
        name: "bloom_weights_linear",
        run: bloom_weights_linear,
    },
    Port {
        name: "bloom_weights_legacy",
        run: bloom_weights_legacy,
    },
    Port {
        name: "parchment_hit",
        run: parchment_hit,
    },
    Port {
        name: "keep_off",
        run: keep_off,
    },
    Port {
        name: "assess_blast",
        run: assess_blast,
    },
    Port {
        name: "assess_wave",
        run: assess_wave,
    },
    Port {
        name: "catmull_position",
        run: catmull_position,
    },
    Port {
        name: "flicker_step",
        run: flicker_step,
    },
    Port {
        name: "focus_settle",
        run: focus_settle,
    },
    Port {
        name: "frustum_half_extents",
        run: frustum_half_extents,
    },
    Port {
        name: "wheel_scroll",
        run: wheel_scroll,
    },
    Port {
        name: "light_spot_shaping",
        run: light_spot_shaping,
    },
    Port {
        name: "rumble_hit",
        run: rumble_hit,
    },
    // PORTS-END
];
