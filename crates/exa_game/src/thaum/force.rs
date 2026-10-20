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
//!   they are made; a refused motion still returns 1.
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
    AiView, CastBody, ID_MASK, TickScale, WEAPON_IN_HAND, advice, body_gate, code, free_slots,
    front_end_blocked, persistent_advice, price, start_cast, within,
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

/// The blast family's tuning, as each creator leaves it on the blast
/// simulation (the bolt and the wave change the blast's defaults).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlastSetup {
    /// The id its cast motion stores.
    pub power: u32,
    /// Added to each chunk's radius every tick.
    pub growth: f32,
    /// What a struck body's push gains.
    pub push: f32,
    /// Scales the chunk count.
    pub chunks: f32,
    /// The share of their speed chunks keep each tick.
    pub retention: f32,
    /// The speed term's scale.
    pub speed: f32,
    /// The light's strength.
    pub light: f32,
    /// Added to the bounds (cm).
    pub margin: f32,
    /// Further values the blast's tick reads, in the creator's order (their
    /// roles are not traced here).
    pub scale: f32,
    pub impact: f32,
    pub impact_share: f32,
    pub after: f32,
    /// A bolt flies through the level's static check.
    pub piercing: bool,
    pub charge_sound: &'static str,
    pub release_sound: &'static str,
}

/// The blast's defaults.
pub const BLAST_SETUP: BlastSetup = BlastSetup {
    power: 0x201,
    growth: 0.4,
    push: 5.0,
    chunks: 0.7,
    retention: 0.96,
    speed: 1.0,
    light: 0.3,
    margin: 0.0,
    scale: 1.0,
    impact: 0.5,
    impact_share: 0.8,
    after: 0.6,
    piercing: false,
    charge_sound: "thmfrc_cu01",
    release_sound: "thmfrc_bb",
};

/// The bolt: a blast with 0x202's motion and its own tuning.
pub const BOLT_SETUP: BlastSetup = BlastSetup {
    power: 0x202,
    growth: 0.01,
    push: 80.0,
    chunks: 1.0,
    retention: 0.99,
    speed: 0.75,
    light: 0.4,
    margin: 20.0,
    scale: 0.3,
    impact: 0.5,
    impact_share: 0.5,
    after: 0.6,
    piercing: true,
    charge_sound: "thmfrc_cu01",
    release_sound: "thmfrc_blt",
};

/// The wave: the blast's defaults with its light, scale and speed.
pub const WAVE_SETUP: BlastSetup = BlastSetup {
    light: 0.5,
    scale: 1.25,
    speed: 1.25,
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
            if need > body.focus || need.is_nan() || body.focus.is_nan() {
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

/// Blast: 1 when the AI should cast it.
pub fn blast_advice(cost: f32, view: &AiView) -> u32 {
    let use_it = f64::from(cost) * 1.2 <= f64::from(view.caster.focus)
        && view.facing <= 1.0
        && view.distance_sq <= 360_000.0
        && view.target_condition & super::casting::OFF_FEET == 0;
    u32::from(use_it)
}

/// Barrier: 0x10 when the AI should cast it.
pub fn barrier_advice(cost: f32, view: &AiView) -> u32 {
    let use_it = view.caster.focus * 0.5 >= cost
        && view.facing >= 1.0
        && view.distance_sq <= 90_000.0
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
        held.cast_hook = Some(0);
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
