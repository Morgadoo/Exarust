//! Ports registered by the second oracle lane (batch d): the fight AI's
//! and the force effects' routines that call a few helpers.

#[allow(unused_imports)]
use super::{Case, In, Out, Port};
use bevy::math::Vec3;

/// An object's bytes of `size`, zero but for `fields` (offset, bytes): the
/// out buffer of a spec laid out by offset.
fn object(size: usize, fields: &[(usize, &[u8])]) -> Vec<u8> {
    let mut out = vec![0u8; size];
    for &(at, bytes) in fields {
        out[at..at + bytes.len()].copy_from_slice(bytes);
    }
    out
}

/// A vector's twelve bytes.
fn v3(v: Vec3) -> Vec<u8> {
    Out::default().vec3(v).0
}

/// The gait helper's look smoothing: the AI's look and sway, the body's
/// heading vector rewritten when the look lags it by more than `degrees`.
fn smooth_look(b: &Case) -> Vec<u8> {
    let mut gaze = exa_ai::motion::Gaze::at(0.0);
    gaze.look = b.f32("ai.look");
    gaze.sway = b.f32("ai.sway");
    let body = b.vec3("body");
    let yaw = exa_physics::engine_math::heading(body);
    let written = exa_ai::motion::smooth_look(&mut gaze, b.f32("heading"), yaw, b.f32("degrees"));
    let mut out = object(
        0x8E8,
        &[
            (0x8D0, &gaze.look.to_le_bytes()),
            (0x8E4, &gaze.sway.to_le_bytes()),
        ],
    );
    out.extend(object(0xA54, &[(0xA48, &v3(written.unwrap_or(body)))]));
    out
}

/// A contact's place score as the original writes it: two weights at 4
/// and 8 (the close and danger terms), the approach and strike terms, the
/// bearing and the distance.
fn place_score(p: &exa_ai::combat::PlaceScore) -> Vec<u8> {
    let mut out = vec![0u8; 4];
    for v in [
        p.close,
        p.danger,
        p.approach,
        p.strike,
        p.bearing,
        p.distance_cm,
    ] {
        out.extend(v.to_le_bytes());
    }
    out
}

/// A force blast's place score: the caster's chest point and the blast's
/// aim, released when its count is above zero.
fn blast_place(b: &Case) -> Vec<u8> {
    let blast = exa_ai::combat::BlastStep {
        chest: b.vec3("caster"),
        aim: Vec3::new(b.f32("blast.ax"), b.f32("blast.ay"), b.f32("blast.az")),
        released: i32::from_le_bytes(b.field("blast.released").try_into().expect("4 bytes")) > 0,
        at: Vec3::ZERO,
    };
    let mut out = place_score(&exa_ai::combat::blast_place(blast, b.vec3("at")));
    out.push(1);
    out
}

/// A barricade's place score (its avoidance slot): the wall's centre, axis
/// and half width, the fighter's chest, step length, reach word, scale and
/// feral level, its caution and lunge reach, the place asked (F569).
fn barricade_place(b: &Case) -> Vec<u8> {
    let wall = exa_ai::combat::WallStep {
        centre: Vec3::new(b.f32("wall.cx"), b.f32("wall.cy"), b.f32("wall.cz")),
        along: Vec3::new(b.f32("node.ax"), b.f32("node.ay"), b.f32("node.az")),
        half_width: b.f32("wall.half"),
    };
    let body = exa_ai::combat::Body {
        chest: b.vec3("body"),
        scale: b.f32("body.scale"),
        reach_cm: b.f32("body.reach"),
        step_cm: b.f32("body.step"),
        feral: b.f32("crec.feral"),
        ..exa_ai::combat::Body::default()
    };
    place_score(&exa_ai::combat::barricade_place(
        wall,
        b.vec3("at"),
        &body,
        b.f32("ai.caution"),
        b.f32("ai.keep"),
    ))
}

/// The named field's bytes as a little-endian `i32`.
fn i32_of(b: &Case, name: &str) -> i32 {
    i32::from_le_bytes(b.field(name).try_into().expect("4 bytes"))
}

/// The Loyalty target's tick: its state, trust, strength and tick count,
/// the relation answered by the relation routine.
fn loyalty_control(b: &Case) -> Vec<u8> {
    use exa_game::thaum::power::mind::LoyaltyTarget;
    let mut t = LoyaltyTarget::new(0);
    t.state = i32_of(b, "t.state");
    t.trust = b.f32("t.trust");
    t.strength = b.f32("t.strength");
    t.ticks = i32_of(b, "t.ticks") as u32;
    let word = t.control(i32_of(b, "event"), b.f32("rel.relation"));
    let mut out = object(
        0x40,
        &[
            (0x18, &t.state.to_le_bytes()),
            (0x30, &t.trust.to_le_bytes()),
            (0x38, &t.strength.to_le_bytes()),
            (0x3C, &t.ticks.to_le_bytes()),
        ],
    );
    out.extend(word.to_le_bytes());
    out
}

/// An idle state's attention to the attended contact: its notice score
/// against `level`, the yaw from the body to the contact's eye point, the
/// held focus; the look and the state word the original sets on success.
fn attend(b: &Case) -> Vec<u8> {
    use exa_ai::walker;
    let score = walker::notice_score(
        b.f32("ai.concern"),
        b.f32("ai.hostility"),
        b.f32("ai.noticed"),
        b.f32("ai.attention"),
    );
    let attended = walker::Attended {
        score: Some(score),
        yaw: exa_ai::motion::yaw_of(b.vec3("eye") - b.vec3("body")),
        focus_held: b.u8("ai.held") != 0,
    };
    let mut turn = exa_ai::motion::Turning {
        heading: 0.0,
        look: b.f32("ai.look"),
        offset: 0.0,
        sway: 0.0,
        sway_at_ms: 0,
    };
    let took = walker::attend(&mut turn, Some(attended), b.f32("level"));
    let state = if took { 1 } else { i32_of(b, "ai.state") };
    let mut out = object(
        0x8B8,
        &[
            (0x54, &turn.look.to_le_bytes()),
            (0x288, &i32_of(b, "ai.count").to_le_bytes()),
            (0x2A0, b.field("ai.noticed")),
            (0x2A8, b.field("ai.attention")),
            (0x2AC, b.field("ai.concern")),
            (0x2B4, b.field("ai.hostility")),
            (0x87E, b.field("ai.held")),
            (0x8B4, &state.to_le_bytes()),
        ],
    );
    out.push(u8::from(took));
    out
}

/// A barricade as the AI's sight assesses it: the wall's point nearest
/// the viewer's chest, the urge, the near, reach and fear words and the
/// facing term written to the seen record; 0 when seen, −1 when not.
fn assess_barricade(b: &Case) -> Vec<u8> {
    use exa_game::effects::danger;
    let wall = danger::Wall {
        centre: Vec3::new(b.f32("wall.cx"), b.f32("wall.cy"), b.f32("wall.cz")),
        along: Vec3::new(b.f32("node.ax"), b.f32("node.ay"), b.f32("node.az")),
        half_width: b.f32("wall.half"),
    };
    let viewer = danger::Viewer {
        chest: b.vec3("body"),
        // The spec names the body's facing axis (the first) `l*` and its
        // look axis `f*`: the viewer takes them as the blast's spec does.
        look: Vec3::new(b.f32("body.fx"), b.f32("body.fy"), b.f32("body.fz")),
        facing: Vec3::new(b.f32("body.lx"), b.f32("body.ly"), b.f32("body.lz")),
    };
    let mut seen = danger::Seen {
        flags: u32::from_le_bytes(b.field("seen.flags").try_into().expect("4 bytes")),
        alarm: b.f32("seen.alarm"),
        near: b.f32("seen.near"),
        reach: b.f32("seen.reach"),
        relation: b.f32("seen.relation"),
        fear: b.f32("seen.fear"),
        urge: b.f32("seen.urge"),
        face: b.f32("seen.face"),
    };
    let seen_it = danger::assess_barricade(
        &wall,
        i32_of(b, "wall.state") == 1,
        i32_of(b, "wall.strong") == 1,
        &viewer,
        b.f32("rel.relation"),
        &mut seen,
    );
    let mut out = object(
        0x44,
        &[
            (0, &seen.flags.to_le_bytes()),
            (0x10, &seen.alarm.to_le_bytes()),
            (0x14, &seen.near.to_le_bytes()),
            (0x20, &seen.reach.to_le_bytes()),
            (0x28, &seen.relation.to_le_bytes()),
            (0x30, &seen.fear.to_le_bytes()),
            (0x3C, &seen.urge.to_le_bytes()),
            (0x40, &seen.face.to_le_bytes()),
        ],
    );
    out.extend(if seen_it { 0i32 } else { -1 }.to_le_bytes());
    out
}

/// The spirit form's frame hook: the form's value stepped by the game
/// clock, the strength the scene environment takes and the post effect's
/// screen fade; false when the form has ended.
fn spirit_form_step(b: &Case) -> Vec<u8> {
    use exa_game::mind::{SpiritForm, step_spirit_form};
    let stamp = u32::from_le_bytes(b.field("form.stamp").try_into().expect("4 bytes"));
    let now = u32::from_le_bytes(b.field("clock.now").try_into().expect("4 bytes"));
    let mut form = SpiritForm {
        state: i32_of(b, "form.state"),
        value: b.f32("form.value"),
        stamp: i64::from(stamp),
    };
    let step = step_spirit_form(&mut form, i64::from(now), b.f32("form.rate"));
    let (mut screen, mut strength) = (b.f32("pp.screen"), b.f32("env.strength"));
    if step.active {
        strength = step.strength;
        if let Some(s) = step.screen {
            screen = s;
        }
    }
    let mut out = object(
        0x50,
        &[
            (0x20, &(form.stamp as u32).to_le_bytes()),
            (0x28, &form.state.to_le_bytes()),
            (0x2C, &form.value.to_le_bytes()),
            (0x30, b.field("form.rate")),
        ],
    );
    out.extend(object(0xE4, &[(0xE0, &screen.to_le_bytes())]));
    out.extend(object(0x3C, &[(0x38, &strength.to_le_bytes())]));
    out.push(u8::from(step.active));
    out
}

/// A made copy's condition roll: its block (flags, two seeds, quality,
/// damage, dirt, the packed spreads), the clock word the entropy reads and
/// the engine generator's seed, written back after the roll.
fn made_condition_roll(b: &Case) -> Vec<u8> {
    let word = |name: &str| {
        let f = b.field(name);
        u32::from_le_bytes([f[0], f[1], f[2], f[3]])
    };
    let mut cond = exa_items::instance::MadeCondition {
        flags: word("cond.flags"),
        seeds: [word("cond.seed0"), word("cond.seed1")],
        quality: b.u8("cond.quality"),
        damage: b.u8("cond.damage"),
        dirt: b.u8("cond.dirt"),
        spreads: exa_items::instance::Spreads(b.u8("cond.spreads")),
    };
    let mut seed = word("seed.s");
    cond.roll(word("clock.now"), &mut seed);
    let mut out = Vec::with_capacity(24);
    out.extend(cond.flags.to_le_bytes());
    out.extend(cond.seeds[0].to_le_bytes());
    out.extend(cond.seeds[1].to_le_bytes());
    out.extend([cond.quality, cond.damage, cond.dirt, cond.spreads.0]);
    out.extend([0u8; 4]);
    out.extend(seed.to_le_bytes());
    out
}

pub(super) const PORTS: &[Port] = &[
    Port {
        name: "barricade_place",
        run: barricade_place,
    },
    Port {
        name: "made_condition_roll",
        run: made_condition_roll,
    },
    Port {
        name: "spirit_form_step",
        run: spirit_form_step,
    },
    Port {
        name: "assess_barricade",
        run: assess_barricade,
    },
    Port {
        name: "attend",
        run: attend,
    },
    Port {
        name: "loyalty_control",
        run: loyalty_control,
    },
    Port {
        name: "smooth_look",
        run: smooth_look,
    },
    Port {
        name: "blast_place",
        run: blast_place,
    },
];
