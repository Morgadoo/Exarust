//! The Force domain's casts, as the original's Force unit routines run
//! them: the eleven-row power table, the player's and the AI's front ends,
//! each power's cast, what each cast creates, the four AI evaluators, the
//! barricade's placement and the reservation a saved persistent power takes
//! back on load. The effects themselves (blast, torrent, domes, shell,
//! kinesis …) are their own simulations; the barricade's is
//! [`super::barricade`].
//!
//! - **Table** ([`FORCE_POWERS`]): blast 0x201 at 0.10, bolt 0x202 at
//!   0.10, torrent 0x10203 at 0.10 a second, wave 0x204 at 0.20, barrier
//!   0x10205 at 0.07 a second, barricade 0x20206 and bubble 0x20207 at
//!   0.25, shell 0x20208 at 0.35, kinesis 0x10209 at 0.02 a second, throw
//!   0x20A at 0.10 and strikes 0x2020B at 0.30.
//! - **Charging powers** (blast, bolt, wave, torrent, barrier, kinesis,
//!   throw, strikes): pressing the power that charges releases it (0).
//!   Blast and bolt need 1.2 times the cost in focus (in double
//!   precision), barrier twice, the others the cost; then the state gate
//!   ([`super::casting::body_gate`]) and the effect starts (1).
//! - **Persistent powers** (bubble, shell, strikes, barricade): a running
//!   one is told to end (2) — the barricade only when the cursor's ray
//!   meets its wall; with no slot free the cast is refused.
//! - **Kinesis** releases instead while the controls drag something.
//!   **Throw** from a charging kinesis hands its held body to a throw;
//!   otherwise it is refused while the controls drag, and a pick that
//!   finds nothing plays the failure sound but still returns 1.
//!   **Strikes** need a weapon in hand and start cast motion 0x25 before
//!   they are made, with a cast hook answering 0 to bits 0x29; a refused
//!   motion makes nothing but still returns 1.
//! - **AI evaluators**: blast 1 with 1.2 times the cost in focus, facing
//!   within 1, the target within 6 m and on its feet; barrier 0x10 with
//!   half the focus above the cost, facing 1 or more, the target within
//!   3 m and on its feet; shell and strikes
//!   ([`super::casting::persistent_advice`]). The AI front end prices the
//!   row as the player's does, without the status test, and answers
//!   0x400_0000 for an id the table lacks.
//! - **Barricade placement** ([`place_barricade`]): the ground under the
//!   cursor (15 cm back along the hit's normal unless the surface asks
//!   otherwise), 10 cm up; a probe down of 210 cm that meets the floor
//!   moves it there, and the wall's middle is 100 cm above. The wall faces
//!   away from the caster on the level and spans 300 cm to each side until
//!   the level trims it; its placing sound plays at volume 0.3 and pitch
//!   0.8 + 0.4·r.
//!
//! Evidence: the Force table and AI table, the eleven casts, the two front
//! ends, the creators and the barricade creator, read from `Exanima.exe`
//! (`context/re/thaum-casts.md`).

use bevy::math::Vec3;

use super::casting::{
    AiView, CastBody, CastHook, ID_MASK, TickScale, WEAPON_IN_HAND, advice, body_gate, code,
    free_slots, front_end_blocked, not_above, not_below, persistent_advice, price, start_cast,
    within,
};

/// What a Force row's cast does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForceKind {
    Blast,
    Bolt,
    Torrent,
    Wave,
    Barrier,
    Barricade,
    Bubble,
    Shell,
    Kinesis,
    Throw,
    Strikes,
}

/// One row of the Force table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ForceRow {
    pub id: u32,
    pub cost: f32,
    pub kind: ForceKind,
}

/// The Force table, in the original's order.
pub const FORCE_POWERS: [ForceRow; 11] = [
    ForceRow {
        id: 0x201,
        cost: 0.1,
        kind: ForceKind::Blast,
    },
    ForceRow {
        id: 0x202,
        cost: 0.1,
        kind: ForceKind::Bolt,
    },
    ForceRow {
        id: 0x10203,
        cost: 0.1,
        kind: ForceKind::Torrent,
    },
    ForceRow {
        id: 0x204,
        cost: 0.2,
        kind: ForceKind::Wave,
    },
    ForceRow {
        id: 0x10205,
        cost: 0.07,
        kind: ForceKind::Barrier,
    },
    ForceRow {
        id: 0x20206,
        cost: 0.25,
        kind: ForceKind::Barricade,
    },
    ForceRow {
        id: 0x20207,
        cost: 0.25,
        kind: ForceKind::Bubble,
    },
    ForceRow {
        id: 0x20208,
        cost: 0.35,
        kind: ForceKind::Shell,
    },
    ForceRow {
        id: 0x10209,
        cost: 0.02,
        kind: ForceKind::Kinesis,
    },
    ForceRow {
        id: 0x20A,
        cost: 0.1,
        kind: ForceKind::Throw,
    },
    ForceRow {
        id: 0x2020B,
        cost: 0.3,
        kind: ForceKind::Strikes,
    },
];

/// The power ids of kinesis and of strikes, which the casts name.
pub const KINESIS: u32 = 0x10209;
pub const STRIKES: u32 = 0x2020B;
pub const BARRICADE: u32 = 0x20206;

/// The cast motions the Force powers play.
pub const BLAST_MOTION: u32 = 0x22;
pub const STRIKES_MOTION: u32 = 0x25;
pub const BARRICADE_MOTION: u32 = 0x24;

/// The cast hook Force Strikes and the barricade install once their cast
/// motion starts: 0 to an event carrying bits 0x29, so no other cast starts
/// until the motion is replaced.
pub const STRIKES_HOOK: CastHook = CastHook { keeps: 0x29 };
pub const BARRICADE_HOOK: CastHook = CastHook { keeps: 0x29 };

/// The blast family's tuning, as each creator leaves it on the blast
/// simulation (the bolt and the wave change the blast's defaults); how
/// [`crate::effects::blast`] reads each field.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlastSetup {
    /// The id its cast motion stores.
    pub power: u32,
    /// Added to each thrown chunk's reach every tick.
    pub growth: f32,
    /// Added to each release direction's forward part before it is turned
    /// into the aim's frame.
    pub forward: f32,
    /// A chunk's life, in seconds.
    pub life: f32,
    /// The share of their move thrown chunks keep each tick.
    pub retention: f32,
    /// The chunks' radius growth's scale.
    pub swell: f32,
    /// The release sound's volume.
    pub release_volume: f32,
    /// Added to the bounds and the contact reach (cm).
    pub margin: f32,
    /// The geometry's texture offset's depth.
    pub texture_scale: f32,
    /// The push a limb of a body other than a character takes.
    pub body_push: f32,
    /// The push a character's limb takes (over its body's mass halved), and
    /// the push a simulated object's points take.
    pub character_push: f32,
    /// The GI light's strength.
    pub light: f32,
    /// The bolt: a struck limb wears the chunk (and halves a non-character
    /// limb's share of it), and a static body is met by its sample-point
    /// walk, which zeroes a struck chunk's weight and wears it.
    pub piercing: bool,
    pub charge_sound: &'static str,
    pub release_sound: &'static str,
}

/// The blast's defaults.
pub const BLAST_SETUP: BlastSetup = BlastSetup {
    power: 0x201,
    growth: 0.4,
    forward: 5.0,
    life: 0.7,
    retention: 0.96,
    swell: 1.0,
    release_volume: 0.3,
    margin: 0.0,
    texture_scale: 1.0,
    body_push: 0.5,
    character_push: 0.8,
    light: 0.6,
    piercing: false,
    charge_sound: "thmfrc_cu01",
    release_sound: "thmfrc_bb",
};

/// The bolt: a blast with 0x202's motion and its own tuning.
pub const BOLT_SETUP: BlastSetup = BlastSetup {
    power: 0x202,
    growth: 0.01,
    forward: 80.0,
    life: 1.0,
    retention: 0.99,
    swell: 0.75,
    release_volume: 0.4,
    margin: 20.0,
    texture_scale: 0.3,
    body_push: 0.5,
    character_push: 0.5,
    light: 0.6,
    piercing: true,
    charge_sound: "thmfrc_cu01",
    release_sound: "thmfrc_blt",
};

/// The wave: the blast's defaults with its release volume, texture scale
/// and swell.
pub const WAVE_SETUP: BlastSetup = BlastSetup {
    release_volume: 0.5,
    texture_scale: 1.25,
    swell: 1.25,
    release_sound: "thmfrc_wav",
    ..BLAST_SETUP
};

/// The domes' and the barricade's sounds.
pub const DOME_SOUND: &str = "thmfrc_bi";
pub const BARRIER_HUM: &str = "thmfrc_bc02";
pub const BARRICADE_HUM: &str = "thmfrc_bc01";
pub const PLACE_SOUND: &str = "thmfrc_ss";
pub const BUBBLE_BREAK: &str = "thmfrc_brf";
pub const TORRENT_SOUND: &str = "thmfrc_trntdr";
pub const KINESIS_SOUND: &str = "thmfrc_knsst";
pub const THROW_SOUND: &str = "thmfrc_throw";

/// What a Force cast makes, with the cost it was priced at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ForceStart {
    /// A blast-family effect in the caster's hand (blast, bolt, wave; the
    /// wave starts from the body).
    Blast { setup: BlastSetup, cost: f32 },
    /// The torrent, from the hand.
    Torrent { cost: f32 },
    /// The barrier dome, 100 cm above the caster, facing its heading; the
    /// dome's radius is 100.
    Barrier { cost: f32 },
    /// The bubble dome around the caster; its radius is 120.
    Bubble { cost: f32 },
    /// The barricade, at a placed point.
    Barricade { cost: f32 },
    /// The shell.
    Shell { cost: f32 },
    /// Kinesis.
    Kinesis { cost: f32 },
    /// A throw: of what a charging kinesis holds, or of the pick under the
    /// cursor.
    Throw { cost: f32, from_kinesis: bool },
    /// Strikes, whose motion did (true) or did not start.
    Strikes { cost: f32, motion: bool },
    /// Throw found nothing to pick: the failure sound plays.
    ThrowMissed,
}

/// What a Force cast reads beyond the body.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ForceWorld {
    /// The controls drag something.
    pub controls_busy: bool,
    /// A possession runs (the player's front end refuses).
    pub possessing: bool,
    /// The cursor's ray meets the running barricade's wall.
    pub cursor_on_barricade: bool,
    /// A throw's pick under the cursor finds something it can throw.
    pub throw_pick: bool,
    /// A charging kinesis can hand its held body to a throw.
    pub kinesis_holds: bool,
}

/// One Force cast of a priced row.
pub fn cast_force(
    row: &ForceRow,
    cost: f32,
    body: &mut CastBody,
    world: &ForceWorld,
) -> (i32, Option<ForceStart>) {
    let charging = body.casting == row.id;
    let release = |body: &mut CastBody| {
        body.release = true;
        (code::RELEASED, None)
    };
    match row.kind {
        ForceKind::Blast | ForceKind::Bolt => {
            if charging {
                return release(body);
            }
            if f64::from(cost) * 1.2 > f64::from(body.focus) {
                return (code::REFUSED, None);
            }
            let gate = body_gate(body);
            if gate < 0 {
                return (gate, None);
            }
            let setup = if row.kind == ForceKind::Blast {
                BLAST_SETUP
            } else {
                BOLT_SETUP
            };
            (code::STARTED, Some(ForceStart::Blast { setup, cost }))
        }
        ForceKind::Wave | ForceKind::Torrent | ForceKind::Barrier => {
            if charging {
                return release(body);
            }
            let need = if row.kind == ForceKind::Barrier {
                cost * 2.0
            } else {
                cost
            };
            // `COMISS focus, need`: only an ordered shortfall refuses.
            if need > body.focus {
                return (code::REFUSED, None);
            }
            let gate = body_gate(body);
            if gate < 0 {
                return (gate, None);
            }
            let start = match row.kind {
                ForceKind::Wave => ForceStart::Blast {
                    setup: WAVE_SETUP,
                    cost,
                },
                ForceKind::Torrent => ForceStart::Torrent { cost },
                _ => ForceStart::Barrier { cost },
            };
            (code::STARTED, Some(start))
        }
        ForceKind::Bubble | ForceKind::Shell => {
            let free = match free_slots(body, row.id) {
                Ok(free) => free,
                Err(ended) => return (ended, None),
            };
            if free < 1 {
                return (code::REFUSED, None);
            }
            if charging {
                return release(body);
            }
            if !within(cost, body.focus) {
                return (code::REFUSED, None);
            }
            let gate = body_gate(body);
            if gate < 0 {
                return (gate, None);
            }
            let start = if row.kind == ForceKind::Bubble {
                ForceStart::Bubble { cost }
            } else {
                ForceStart::Shell { cost }
            };
            (code::STARTED, Some(start))
        }
        ForceKind::Barricade => {
            let mut free = body.slots;
            for power in &mut body.powers {
                if power.id & super::casting::PERSISTENT != 0 {
                    free -= 1;
                }
                if power.id == row.id && world.cursor_on_barricade {
                    power.state = 2;
                    return (code::ENDED, None);
                }
            }
            if free < 1 || !within(cost, body.focus) {
                return (code::REFUSED, None);
            }
            let gate = body_gate(body);
            if gate < 0 {
                return (gate, None);
            }
            (code::STARTED, Some(ForceStart::Barricade { cost }))
        }
        ForceKind::Kinesis => {
            if charging || world.controls_busy {
                return release(body);
            }
            if !within(cost, body.focus) {
                return (code::REFUSED, None);
            }
            let gate = body_gate(body);
            if gate < 0 {
                return (gate, None);
            }
            (code::STARTED, Some(ForceStart::Kinesis { cost }))
        }
        ForceKind::Throw => {
            if charging {
                return release(body);
            }
            if !within(cost, body.focus) {
                return (code::REFUSED, None);
            }
            if body.casting == KINESIS && body.powers.iter().any(|p| p.id == KINESIS) {
                return if world.kinesis_holds {
                    (
                        code::STARTED,
                        Some(ForceStart::Throw {
                            cost,
                            from_kinesis: true,
                        }),
                    )
                } else {
                    (code::REFUSED, None)
                };
            }
            let gate = body_gate(body);
            if gate < 0 {
                return (gate, None);
            }
            if world.controls_busy {
                return (code::REFUSED, None);
            }
            let start = if world.throw_pick {
                ForceStart::Throw {
                    cost,
                    from_kinesis: false,
                }
            } else {
                ForceStart::ThrowMissed
            };
            (code::STARTED, Some(start))
        }
        ForceKind::Strikes => {
            let free = match free_slots(body, row.id) {
                Ok(free) => free,
                Err(ended) => return (ended, None),
            };
            if free < 1 {
                return (code::REFUSED, None);
            }
            if charging {
                return release(body);
            }
            if body.weapon_class & WEAPON_IN_HAND == 0 || !within(cost, body.focus) {
                return (code::REFUSED, None);
            }
            let gate = body_gate(body);
            if gate < 0 {
                return (gate, None);
            }
            let motion = start_cast(body, STRIKES_MOTION, STRIKES);
            (code::STARTED, Some(ForceStart::Strikes { cost, motion }))
        }
    }
}

/// The row whose id equals `asked` under the front ends' mask.
pub fn force_row(asked: u32) -> Option<&'static ForceRow> {
    FORCE_POWERS.iter().find(|row| asked & ID_MASK == row.id)
}

/// The player's Force front end: refuses while the body's powers are
/// blocked or a possession runs, prices the row and casts it.
pub fn force_front_end(
    asked: u32,
    body: &mut CastBody,
    world: &ForceWorld,
) -> (i32, Option<ForceStart>) {
    if front_end_blocked(body, world.possessing) {
        return (code::REFUSED, None);
    }
    let Some(row) = force_row(asked) else {
        return (code::REFUSED, None);
    };
    let cost = price(row.id, row.cost, asked, body, TickScale::Thousandth);
    cast_force(row, cost, body, world)
}

/// The AI's Force front end: the advice word for `asked`.
pub fn force_ai_front_end(asked: u32, view: &AiView) -> u32 {
    let Some(row) = force_row(asked) else {
        return advice::UNAVAILABLE;
    };
    let cost = price(row.id, row.cost, asked, &view.caster, TickScale::Thousandth);
    match row.kind {
        ForceKind::Blast => blast_advice(cost, view),
        ForceKind::Barrier => barrier_advice(cost, view),
        ForceKind::Shell | ForceKind::Strikes => persistent_advice(row.id, cost, view),
        _ => advice::UNAVAILABLE,
    }
}

/// Blast: 1 when the AI should cast it. Each comparison refuses only on its
/// ordered branch, so a not-a-number passes (`COMISD`/`COMISS` then `JP`).
pub fn blast_advice(cost: f32, view: &AiView) -> u32 {
    let enough = not_above(f64::from(cost) * 1.2, f64::from(view.caster.focus));
    let use_it = enough
        && not_above(view.facing, 1.0)
        && not_above(view.distance_sq, 360_000.0)
        && view.target_condition & super::casting::OFF_FEET == 0;
    u32::from(use_it)
}

/// Barrier: 0x10 when the AI should cast it.
pub fn barrier_advice(cost: f32, view: &AiView) -> u32 {
    let use_it = not_below(view.caster.focus * 0.5, cost)
        && not_below(view.facing, 1.0)
        && not_above(view.distance_sq, 90_000.0)
        && view.target_condition & super::casting::OFF_FEET == 0;
    if use_it { advice::USE } else { 0 }
}

/// A persistent Force power restored with a saved body: its cost from the
/// table, held in the reserve (three quarters with the technique). Only
/// shell and strikes keep an object to restore.
pub fn restore_force_power(id: u32, body: &mut CastBody) -> Option<f32> {
    let row = FORCE_POWERS.iter().find(|row| row.id == id)?;
    if !matches!(row.kind, ForceKind::Shell | ForceKind::Strikes) {
        return None;
    }
    let held = if body.techniques & super::casting::RESERVE_TECHNIQUE != 0 {
        row.cost * 0.75
    } else {
        row.cost
    };
    body.reserved += held;
    Some(row.cost)
}

/// Where the cursor's ray met the level for a barricade.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroundHit {
    pub point: Vec3,
    pub normal: Vec3,
    /// The surface keeps the hit point as it is.
    pub exact: bool,
}

/// A barricade's place and frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarricadePlace {
    /// The wall's middle.
    pub centre: Vec3,
    /// Away from the caster, on the level.
    pub facing: Vec3,
    /// How far the wall runs to each side before the level trims it (cm).
    pub reach: [f32; 2],
    /// The placing sound's pitch.
    pub pitch: f32,
}

/// The placing sound's volume.
pub const PLACE_VOLUME: f32 = 0.3;

/// Places a barricade from the cursor's ground hit. `floor_below` probes
/// down from a point as far as the given reach and returns how far down
/// the floor is; `roll` is the engine's generator draw for the sound's
/// pitch.
pub fn place_barricade(
    hit: GroundHit,
    caster: Vec3,
    floor_below: impl FnOnce(Vec3, f32) -> Option<f32>,
    roll: f32,
) -> BarricadePlace {
    let mut point = hit.point;
    if !hit.exact {
        point += hit.normal * -15.0;
    }
    point.y += 10.0;
    if let Some(depth) = floor_below(point, 210.0) {
        point += Vec3::NEG_Y * depth;
        point.y += 100.0;
    }
    let mut away = point - caster;
    away.y = 0.0;
    let facing = away.normalize_or_zero();
    BarricadePlace {
        centre: point,
        facing,
        reach: [300.0, 300.0],
        pitch: roll * 0.4 + 0.8,
    }
}

/// What a throw's pick under the cursor found.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThrowTarget {
    /// The node is part of the level.
    pub fixed: bool,
    /// It has a moving body.
    pub moving: bool,
    /// The body is a character's.
    pub character: bool,
    /// The character's condition word.
    pub condition: u32,
    /// The node may be thrown whatever its weight.
    pub any_weight: bool,
    /// The body's mobility.
    pub mobility: f32,
    /// How many hold it already.
    pub held: i32,
}

/// Whether a throw takes the pick: a moving body off the level, a
/// character only when dead, anything else only when it may be thrown
/// whatever its weight or its mobility is 0.01 or more, and nobody holds
/// it. A throw that takes it plays `thmfrc_knsst` at it (volume 0.3,
/// pitch 0.8 + 0.4·r).
pub fn can_throw(target: &ThrowTarget) -> bool {
    if target.fixed || !target.moving {
        return false;
    }
    if target.character {
        if target.condition != super::casting::FALLEN_OR_DYING {
            return false;
        }
    } else if !target.any_weight && target.mobility < 0.01 {
        return false;
    }
    target.held < 1
}

/// A charging kinesis hands its held body to a throw when it holds one
/// nobody else holds; the kinesis then lets go and ends.
pub fn kinesis_hands_over(target_held: Option<i32>) -> bool {
    target_held.is_some_and(|held| held < 1)
}

/// Where a dome sits: 100 cm above the caster's ground point, facing its
/// heading (the barrier), its radius 100 for the barrier and 120 for the
/// bubble.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DomePlace {
    pub centre: Vec3,
    pub radius: f32,
    pub facing: Option<Vec3>,
}

/// The barrier's or the bubble's place.
pub fn place_dome(kind: ForceKind, ground: Vec3, heading: Vec3) -> Option<DomePlace> {
    let centre = Vec3::new(ground.x, ground.y + 100.0, ground.z);
    match kind {
        ForceKind::Barrier => Some(DomePlace {
            centre,
            radius: 100.0,
            facing: Some(heading),
        }),
        ForceKind::Bubble => Some(DomePlace {
            centre,
            radius: 120.0,
            facing: None,
        }),
        _ => None,
    }
}

/// The pitch of the sound the shell and strikes play as they start
/// (`thmfrc_ss` at volume 0.3): 0.6 + 0.4·r.
pub fn start_pitch(roll: f32) -> f32 {
    roll * 0.4 + 0.6
}

/// The material every blast-family node wears.
pub const FORCE_MATERIAL: &str = "_ForceMat01";
/// The material the domes and the barricade wear.
pub const FORCE_BARRIER_MATERIAL: &str = "_ForceBarrier01";
/// The material the shell (and its decorative copy) wears.
pub const FORCE_AURA_MATERIAL: &str = "_ForceAura01";
/// The four parameters each material binds.
pub const MATERIAL_PARAMS: [f32; 4] = [0.25, 0.25, 0.25, 1.0];
/// The blast material's parameter block size.
pub const FORCE_PARAM_SIZE: usize = 0xf0;
/// The barrier material's parameter block size.
pub const BARRIER_PARAM_SIZE: usize = 0xb8;
/// The aura material's parameter block size.
pub const AURA_PARAM_SIZE: usize = 0xf0;
/// The animated noise texture the barrier and the aura bind.
pub const NOISE_TEXTURE: &str = "_ForceNoise";
/// The noise field's width, height and octaves (one 8-bit channel).
pub const NOISE_SIZE: (u32, u32, u32) = (256, 256, 4);
/// The noise texture's parameter word (mipmapped when bit 0x40 is set).
pub const NOISE_WORD: u32 = 0x2f0;
/// The same word when either of the two display switches is on.
pub const NOISE_WORD_REDUCED: u32 = 0x33;
/// The flags every created Force node starts with.
pub const FORCE_NODE: u32 = 0x2100_0000;
/// The shared emissive program (blast and torrent).
pub const GI_PROGRAM: &str = "gi_multipoint";
/// The emissive program's light binding.
pub const GI_LIGHT: &str = "Light";
/// The emissive program's attenuation binding.
pub const GI_ATTEN: &str = "Atten";

/// What one Force creator attaches, in the original's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatorSpec {
    /// The material, if the power makes a node.
    pub material: Option<&'static str>,
    /// The material's parameter block size.
    pub param_size: usize,
    /// The shared emissive program.
    pub gi: Option<&'static str>,
    /// The sounds the creator loads, in its order.
    pub sounds: &'static [&'static str],
    /// The power whose active object the creator makes, if any.
    pub active: Option<u32>,
}

const NONE: CreatorSpec = CreatorSpec {
    material: None,
    param_size: 0,
    gi: None,
    sounds: &[],
    active: None,
};

/// The creator of one Force power.
pub fn creator_spec(kind: ForceKind) -> CreatorSpec {
    match kind {
        ForceKind::Blast => CreatorSpec {
            material: Some(FORCE_MATERIAL),
            param_size: FORCE_PARAM_SIZE,
            gi: Some(GI_PROGRAM),
            sounds: &[BLAST_SETUP.charge_sound, BLAST_SETUP.release_sound],
            active: None,
        },
        ForceKind::Bolt => CreatorSpec {
            material: Some(FORCE_MATERIAL),
            param_size: FORCE_PARAM_SIZE,
            gi: Some(GI_PROGRAM),
            sounds: &[BLAST_SETUP.charge_sound, BOLT_SETUP.release_sound],
            active: None,
        },
        ForceKind::Wave => CreatorSpec {
            material: Some(FORCE_MATERIAL),
            param_size: FORCE_PARAM_SIZE,
            gi: Some(GI_PROGRAM),
            sounds: &[BLAST_SETUP.charge_sound, WAVE_SETUP.release_sound],
            active: None,
        },
        ForceKind::Torrent => CreatorSpec {
            material: Some(FORCE_MATERIAL),
            param_size: FORCE_PARAM_SIZE,
            gi: Some(GI_PROGRAM),
            sounds: &[BLAST_SETUP.charge_sound, TORRENT_SOUND],
            active: None,
        },
        ForceKind::Barrier => CreatorSpec {
            material: Some(FORCE_BARRIER_MATERIAL),
            param_size: BARRIER_PARAM_SIZE,
            gi: Some(GI_PROGRAM),
            sounds: &[DOME_SOUND, BARRIER_HUM, PLACE_SOUND],
            active: None,
        },
        ForceKind::Bubble => CreatorSpec {
            material: Some(FORCE_BARRIER_MATERIAL),
            param_size: BARRIER_PARAM_SIZE,
            gi: Some(GI_PROGRAM),
            sounds: &[DOME_SOUND, BARRIER_HUM, PLACE_SOUND, BUBBLE_BREAK],
            active: None,
        },
        ForceKind::Barricade => CreatorSpec {
            material: Some(FORCE_BARRIER_MATERIAL),
            param_size: BARRIER_PARAM_SIZE,
            gi: Some(GI_PROGRAM),
            sounds: &[DOME_SOUND, BARRICADE_HUM, PLACE_SOUND],
            active: None,
        },
        ForceKind::Shell => CreatorSpec {
            material: Some(FORCE_AURA_MATERIAL),
            param_size: AURA_PARAM_SIZE,
            gi: None,
            sounds: &[PLACE_SOUND],
            active: Some(0x20208),
        },
        ForceKind::Kinesis => CreatorSpec {
            // The kinesis creator makes the active object; it loads no
            // sound of its own.
            active: Some(0x10209),
            ..NONE
        },
        ForceKind::Throw => CreatorSpec {
            sounds: &[KINESIS_SOUND, THROW_SOUND],
            active: Some(0x20A),
            ..NONE
        },
        ForceKind::Strikes => CreatorSpec {
            sounds: &[PLACE_SOUND],
            active: Some(0x2020B),
            ..NONE
        },
    }
}

#[cfg(test)]
mod tests {
    use super::super::casting::{ActivePower, CASTING};
    use super::*;

    fn standing(focus: f32) -> CastBody {
        CastBody {
            focus,
            slots: 1,
            ..CastBody::default()
        }
    }

    #[test]
    fn each_creator_attaches_its_material_and_sounds() {
        let blast = creator_spec(ForceKind::Blast);
        assert_eq!(blast.material, Some(FORCE_MATERIAL));
        assert_eq!(blast.param_size, 0xf0);
        assert_eq!(blast.gi, Some(GI_PROGRAM));
        assert_eq!(blast.sounds, ["thmfrc_cu01", "thmfrc_bb"]);
        assert_eq!(
            creator_spec(ForceKind::Bolt).sounds,
            ["thmfrc_cu01", "thmfrc_blt"]
        );
        assert_eq!(
            creator_spec(ForceKind::Wave).sounds,
            ["thmfrc_cu01", "thmfrc_wav"]
        );
        assert_eq!(
            creator_spec(ForceKind::Torrent).sounds,
            ["thmfrc_cu01", "thmfrc_trntdr"]
        );
        assert_eq!(
            creator_spec(ForceKind::Barrier).material,
            Some(FORCE_BARRIER_MATERIAL)
        );
        assert_eq!(creator_spec(ForceKind::Barrier).param_size, 0xb8);
        assert_eq!(creator_spec(ForceKind::Bubble).sounds[3], "thmfrc_brf");
        assert_eq!(creator_spec(ForceKind::Barricade).sounds[1], "thmfrc_bc01");
        let shell = creator_spec(ForceKind::Shell);
        assert_eq!(shell.material, Some(FORCE_AURA_MATERIAL));
        assert_eq!(shell.gi, None);
        assert_eq!(shell.active, Some(0x20208));
        assert_eq!(creator_spec(ForceKind::Kinesis).active, Some(0x10209));
        assert_eq!(creator_spec(ForceKind::Strikes).active, Some(0x2020B));
        assert_eq!(MATERIAL_PARAMS, [0.25, 0.25, 0.25, 1.0]);
        assert_eq!(NOISE_SIZE, (256, 256, 4));
        assert_eq!((NOISE_WORD, NOISE_WORD_REDUCED), (0x2f0, 0x33));
        assert_eq!(FORCE_NODE, 0x2100_0000);
        assert_eq!((GI_LIGHT, GI_ATTEN), ("Light", "Atten"));
    }

    #[test]
    fn the_table_holds_eleven_rows() {
        assert_eq!(FORCE_POWERS.len(), 11);
        assert_eq!(force_row(0x204).map(|r| r.cost), Some(0.2));
        assert_eq!(force_row(0x10205).map(|r| r.cost), Some(0.07));
        assert!(force_row(0x205).is_none());
        // The mask keeps the id's high bits, so a stray bit 0x10_0000 drops.
        assert_eq!(force_row(0x10_0201).map(|r| r.id), Some(0x201));
    }

    #[test]
    fn blast_needs_one_point_two_times_its_cost() {
        let world = ForceWorld::default();
        let mut short = standing(0.119);
        assert_eq!(force_front_end(0x201, &mut short, &world).0, code::REFUSED);
        // 0.12 in single precision is below 0.1 × 1.2 in double precision.
        let mut exact = standing(0.12);
        assert_eq!(force_front_end(0x201, &mut exact, &world).0, code::REFUSED);
        let mut body = standing(0.1201);
        let (result, start) = force_front_end(0x201, &mut body, &world);
        assert_eq!(result, code::STARTED);
        assert_eq!(
            start,
            Some(ForceStart::Blast {
                setup: BLAST_SETUP,
                cost: 0.1
            })
        );
        body.casting = 0x201;
        assert_eq!(force_front_end(0x201, &mut body, &world).0, code::RELEASED);
        assert!(body.release);
    }

    #[test]
    fn barrier_needs_twice_and_prices_by_the_tick() {
        let world = ForceWorld::default();
        let mut body = standing(1.0);
        let (result, start) = force_front_end(0x10205, &mut body, &world);
        assert_eq!(result, code::STARTED);
        let Some(ForceStart::Barrier { cost }) = start else {
            panic!("{start:?}");
        };
        assert_eq!(cost, 12.0f32 * 0.07 * 0.001);
        let mut poor = standing(cost * 2.0 - 1e-6);
        assert_eq!(force_front_end(0x10205, &mut poor, &world).0, code::REFUSED);
    }

    #[test]
    fn a_running_persistent_power_ends() {
        let world = ForceWorld::default();
        let mut body = standing(1.0);
        body.powers.push(ActivePower {
            id: 0x20208,
            state: 1,
        });
        assert_eq!(force_front_end(0x20208, &mut body, &world).0, code::ENDED);
        assert_eq!(body.powers[0].state, 2);
        // The one slot is taken: a bubble is refused.
        assert_eq!(force_front_end(0x20207, &mut body, &world).0, code::REFUSED);
    }

    #[test]
    fn the_barricade_ends_only_under_the_cursor() {
        let mut body = standing(1.0);
        body.slots = 2;
        body.powers.push(ActivePower {
            id: BARRICADE,
            state: 1,
        });
        let away = ForceWorld::default();
        let (result, start) = force_front_end(BARRICADE, &mut body, &away);
        assert_eq!(result, code::STARTED);
        assert_eq!(start, Some(ForceStart::Barricade { cost: 0.25 }));
        let on = ForceWorld {
            cursor_on_barricade: true,
            ..ForceWorld::default()
        };
        assert_eq!(force_front_end(BARRICADE, &mut body, &on).0, code::ENDED);
    }

    #[test]
    fn kinesis_releases_while_the_controls_drag() {
        let busy = ForceWorld {
            controls_busy: true,
            ..ForceWorld::default()
        };
        let mut body = standing(1.0);
        assert_eq!(force_front_end(KINESIS, &mut body, &busy).0, code::RELEASED);
        let mut body = standing(1.0);
        let (result, start) = force_front_end(KINESIS, &mut body, &ForceWorld::default());
        assert_eq!(result, code::STARTED);
        assert_eq!(
            start,
            Some(ForceStart::Kinesis {
                cost: 12.0f32 * 0.02 * 0.001
            })
        );
    }

    #[test]
    fn a_throw_takes_what_kinesis_holds() {
        let mut body = standing(1.0);
        body.casting = KINESIS;
        body.condition = CASTING;
        body.powers.push(ActivePower {
            id: KINESIS,
            state: 1,
        });
        let holds = ForceWorld {
            kinesis_holds: true,
            ..ForceWorld::default()
        };
        assert_eq!(
            force_front_end(0x20A, &mut body, &holds),
            (
                code::STARTED,
                Some(ForceStart::Throw {
                    cost: 0.1,
                    from_kinesis: true
                })
            )
        );
        assert_eq!(
            force_front_end(0x20A, &mut body, &ForceWorld::default()).0,
            code::REFUSED
        );
        let mut idle = standing(1.0);
        assert_eq!(
            force_front_end(0x20A, &mut idle, &ForceWorld::default()),
            (code::STARTED, Some(ForceStart::ThrowMissed))
        );
    }

    #[test]
    fn strikes_need_a_weapon() {
        let world = ForceWorld::default();
        let mut fists = standing(1.0);
        fists.weapon_class = 0x80;
        assert_eq!(
            force_front_end(STRIKES, &mut fists, &world).0,
            code::REFUSED
        );
        let mut armed = standing(1.0);
        armed.weapon_class = 0x10;
        let (result, start) = force_front_end(STRIKES, &mut armed, &world);
        assert_eq!(result, code::STARTED);
        assert_eq!(
            start,
            Some(ForceStart::Strikes {
                cost: 0.3,
                motion: true
            })
        );
        assert_eq!(armed.cast_motion, Some(STRIKES_MOTION));
        let mut held = standing(1.0);
        held.weapon_class = 0x20;
        held.cast_hook = Some(crate::thaum::casting::CastHook { keeps: 0x20 });
        assert_eq!(
            force_front_end(STRIKES, &mut held, &world).1,
            Some(ForceStart::Strikes {
                cost: 0.3,
                motion: false
            })
        );
    }

    #[test]
    fn blocked_powers_refuse() {
        let mut body = standing(1.0);
        body.status_bits = super::super::casting::POWERS_BLOCKED;
        assert_eq!(
            force_front_end(0x201, &mut body, &ForceWorld::default()).0,
            code::REFUSED
        );
        let mut body = standing(1.0);
        let possessing = ForceWorld {
            possessing: true,
            ..ForceWorld::default()
        };
        assert_eq!(
            force_front_end(0x201, &mut body, &possessing).0,
            code::REFUSED
        );
    }

    #[test]
    fn the_ai_weighs_range_and_facing() {
        let mut view = AiView {
            caster: standing(0.1201),
            facing: 1.0,
            distance_sq: 360_000.0,
            ..AiView::default()
        };
        assert_eq!(force_ai_front_end(0x201, &view), 1);
        view.distance_sq = 360_001.0;
        assert_eq!(force_ai_front_end(0x201, &view), 0);
        view.distance_sq = 90_000.0;
        view.target_condition = 0x200;
        assert_eq!(force_ai_front_end(0x201, &view), 0);
        view.target_condition = 0;
        view.caster.focus = 2.0 * (12.0 * 0.07 * 0.001);
        assert_eq!(force_ai_front_end(0x10205, &view), advice::USE);
        view.facing = 0.99;
        assert_eq!(force_ai_front_end(0x10205, &view), 0);
        assert_eq!(force_ai_front_end(0x999, &view), advice::UNAVAILABLE);
        view.facing = 0.5;
        view.caster.focus = 0.7;
        assert_eq!(force_ai_front_end(0x20208, &view), advice::USE);
        view.caster.slots = 0;
        assert_eq!(force_ai_front_end(0x20208, &view), advice::UNAVAILABLE);
        assert_eq!(force_ai_front_end(0x20207, &view), advice::UNAVAILABLE);
    }

    #[test]
    fn a_not_a_number_focus_or_facing_passes_the_checks() {
        // A wave or torrent whose focus is not a number still starts: the
        // original's `COMISS focus, cost` jumps past its refuse branch.
        let mut body = standing(f32::NAN);
        assert_eq!(
            force_front_end(0x204, &mut body, &ForceWorld::default()).0,
            code::STARTED
        );
        // The same holds for the AI evaluators' focus, facing and distance.
        let view = AiView {
            caster: standing(f32::NAN),
            facing: f32::NAN,
            distance_sq: f32::NAN,
            ..AiView::default()
        };
        assert_eq!(force_ai_front_end(0x201, &view), 1);
        assert_eq!(force_ai_front_end(0x10205, &view), advice::USE);
        let shell = AiView {
            caster: standing(f32::NAN),
            facing: f32::NAN,
            ..AiView::default()
        };
        assert_eq!(force_ai_front_end(0x20208, &shell), advice::USE);
    }

    #[test]
    fn a_restored_shell_holds_its_cost() {
        let mut body = standing(1.0);
        assert_eq!(restore_force_power(0x20208, &mut body), Some(0.35));
        assert!((body.reserved - 0.35).abs() < 1e-7);
        body.techniques = super::super::casting::RESERVE_TECHNIQUE;
        assert_eq!(restore_force_power(0x2020B, &mut body), Some(0.3));
        assert!((body.reserved - (0.35 + 0.225)).abs() < 1e-6);
        assert_eq!(restore_force_power(0x20207, &mut body), None);
    }

    #[test]
    fn a_throw_takes_loose_bodies_and_corpses() {
        let loose = ThrowTarget {
            fixed: false,
            moving: true,
            character: false,
            condition: 0,
            any_weight: false,
            mobility: 0.01,
            held: 0,
        };
        assert!(can_throw(&loose));
        assert!(!can_throw(&ThrowTarget {
            mobility: 0.0099,
            ..loose
        }));
        assert!(can_throw(&ThrowTarget {
            mobility: 0.0,
            any_weight: true,
            ..loose
        }));
        assert!(!can_throw(&ThrowTarget { held: 1, ..loose }));
        let body = ThrowTarget {
            character: true,
            condition: 0xA00,
            ..loose
        };
        assert!(can_throw(&body));
        assert!(!can_throw(&ThrowTarget {
            condition: 0x200,
            ..body
        }));
        assert!(!can_throw(&ThrowTarget {
            fixed: true,
            ..loose
        }));
        assert!(kinesis_hands_over(Some(0)));
        assert!(!kinesis_hands_over(Some(1)));
        assert!(!kinesis_hands_over(None));
    }

    #[test]
    fn the_domes_stand_a_metre_up() {
        let barrier =
            place_dome(ForceKind::Barrier, Vec3::new(1.0, 2.0, 3.0), Vec3::Z).expect("a dome");
        assert_eq!(barrier.centre, Vec3::new(1.0, 102.0, 3.0));
        assert_eq!((barrier.radius, barrier.facing), (100.0, Some(Vec3::Z)));
        let bubble = place_dome(ForceKind::Bubble, Vec3::ZERO, Vec3::Z).expect("a dome");
        assert_eq!((bubble.radius, bubble.facing), (120.0, None));
        assert!(place_dome(ForceKind::Blast, Vec3::ZERO, Vec3::Z).is_none());
        assert_eq!(start_pitch(0.5), 0.8);
    }

    #[test]
    fn a_barricade_stands_on_the_floor_facing_away() {
        let hit = GroundHit {
            point: Vec3::new(100.0, 0.0, 0.0),
            normal: Vec3::Y,
            exact: false,
        };
        let place = place_barricade(
            hit,
            Vec3::new(0.0, 50.0, 0.0),
            |_, reach| (reach == 210.0).then_some(20.0),
            0.5,
        );
        assert_eq!(place.centre, Vec3::new(100.0, 75.0, 0.0));
        assert_eq!(place.facing, Vec3::X);
        assert_eq!(place.reach, [300.0, 300.0]);
        assert_eq!(place.pitch, 1.0);
        let wall = GroundHit {
            point: Vec3::new(0.0, 20.0, 50.0),
            normal: Vec3::X,
            exact: true,
        };
        let place = place_barricade(wall, Vec3::ZERO, |_, _| None, 0.0);
        assert_eq!(place.centre, Vec3::new(0.0, 30.0, 50.0));
        assert_eq!(place.facing, Vec3::Z);
        assert_eq!(place.pitch, 0.8);
    }
}
