//! The brain's contacts, how each is classified, and the target pass that
//! picks what the character does about them.
//!
//! - **Admitting a contact.** A brain keeps at most twelve. A new one takes
//!   a free slot; with all twelve taken it replaces the slot of least
//!   weight (ten times its sighting plus its concern; the first slot's
//!   sighting counted by size), unless it weighs less itself.
//! - **Classifying.** The stress is the stress base plus the stress, less
//!   the courage, at least 0. An alarm above a half adds to the alarm mean.
//!   With stress above 3 and an alarm above 0.75 (0 for the contact it
//!   already flees, whose alarm is raised to the remembered one), and a way
//!   out, the contact is fled (level 4, scored by its alarm). With the
//!   tolerance b, the threshold is `7b² + (2 + b)` (`2 + b` for b ≤ 0);
//!   concern plus hostility above the threshold plus the stress, or above
//!   the threshold with an alarm above 1.5, engages (level 3, 2 for an
//!   unreachable contact, scored by concern plus hostility, marked
//!   engaged). Otherwise a concern above 1 flees when it passes 8 less the
//!   stress and a way out exists, else watches (level 1, scored by the
//!   concern). Otherwise it is noticed (level 0, scored
//!   `6·(concern + hostility) + noticed − attention`, 3 more for the
//!   contact it already attends).
//! - **The target pass.** One brain a tick: the foe it flees stays a
//!   flight at the remembered alarm; the foe it fights stays an engagement
//!   while concern plus hostility pass the raw stress plus 2. Every seen
//!   character and every nearby one is classified; a higher level wins, or
//!   at the same level the higher score (the notice score for level 0).
//!   A sleeper that classifies anyone above a notice wakes. The alarm mean
//!   (over the root of its count) doubled raises the stress. A flight
//!   starts the flee state; nothing above a notice drops the foe and
//!   attends the best noticed; anything else enters combat at the level
//!   (watch, or 3 for 2 and above).
//!
//! Evidence: the original's contact classifier, its contact integration and
//! its target pass, read from the instructions (sums in their order, the
//! alarm mean in double).

use bevy::math::Vec3;

/// What a brain keeps of one contact.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ContactRecord {
    /// Who it is (the caller's id).
    pub id: u64,
    /// Bit 0x10 ([`UNREACHABLE`]): its route was found bad, so engaging it
    /// is level 2; bit 4 (noticed) keeps it through the pass's clean-up.
    pub flags: u32,
    pub noticed: f32,
    /// How much it was seen: the admission weight's part [guess: name].
    pub sighting: f32,
    pub attention: f32,
    pub concern: f32,
    pub hostility: f32,
    /// Its tolerance over 100.
    pub tolerance: f32,
    /// Its alarm rate.
    pub alarm: f32,
    /// A threat term the perception fills [?].
    pub threat: f32,
    /// Bit 2: engaged; bit 1: cleared while the brain engages [?].
    pub marks: u32,
    pub level: i32,
    /// What the noticed level rises toward: the last perception's
    /// noticeability.
    pub noticeability: f32,
    /// Where the concern is heading.
    pub concern_target: f32,
    /// How long it has been seen (ticks).
    pub ticks_seen: i32,
    /// A per-tick decaying value [guess: familiarity].
    pub decay: f32,
    /// Its bad-route mark holds until this clock (ms).
    pub unreachable_until_ms: u32,
    /// A float the integration subtracts from hostility [guess: the
    /// contact's +0x28 word].
    pub soothe: f32,
    /// The perception's stamp word the passive branch clears [?].
    pub stamp: f32,
    /// The other is dead (its body's death bit).
    pub dead: bool,
    /// The priority the combat state writes when it weighs the contact.
    pub priority: f32,
}

/// A brain holds this many contacts.
pub const MOST_CONTACTS: usize = crate::combatant::MAX_CONTACTS;

/// The contact flag a bad route sets: engaging the contact is level 2.
pub const UNREACHABLE: u32 = 0x10;

impl ContactRecord {
    /// The classifier's view of a combatant's contact with `id`: its
    /// noticeability is the admission's sighting, and the attention the
    /// original keeps beside the noticed level is not in the combatant
    /// (0 here).
    pub fn of(c: &crate::combatant::Contact, id: u64) -> Self {
        Self {
            id,
            flags: c.flags,
            noticed: c.noticed,
            sighting: c.noticeability,
            attention: 0.0,
            concern: c.concern,
            hostility: c.hostility,
            tolerance: c.tolerance,
            alarm: c.alarm,
            threat: c.threat,
            marks: 0,
            level: 0,
            noticeability: c.noticeability,
            concern_target: c.concern_target,
            ticks_seen: 0,
            decay: 0.0,
            unreachable_until_ms: c.unreachable_until_ms,
            soothe: 0.0,
            stamp: 0.0,
            dead: false,
            priority: 0.0,
        }
    }
}

/// Where a new contact goes: a free slot, the lightest slot it outweighs,
/// or nowhere.
pub fn admit(list: &[ContactRecord], new: &ContactRecord) -> Option<usize> {
    if list.len() < MOST_CONTACTS {
        return Some(list.len());
    }
    let weight = |c: &ContactRecord| c.sighting * 10.0 + c.concern;
    let mut least = (list[0].sighting * 10.0).abs() + list[0].concern;
    let mut at = 0;
    for (i, c) in list.iter().enumerate().skip(1) {
        let w = weight(c);
        if least > w {
            least = w;
            at = i;
        }
    }
    if least > weight(new) { None } else { Some(at) }
}

/// The brain's morale.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Morale {
    /// Courage: (confidence + 1)/2 from the record; halved when it starts
    /// to flee, doubled when cornered.
    pub courage: f32,
    /// A stress term the classifier adds [?: who sets it].
    pub base: f32,
    /// Stress: raised by 2 on a flight and by the alarm mean.
    pub stress: f32,
    /// The alarm sum of this pass and how many added to it.
    pub alarm_sum: f32,
    pub alarm_count: i32,
    /// The alarm the flight started at.
    pub fled_alarm: f32,
}

/// The stress the classifier reads: base plus stress less the courage, at
/// least 0.
pub fn stress(m: &Morale) -> f32 {
    let s = (m.base + m.stress) - m.courage * 1.0;
    if s < 0.0 { 0.0 } else { s }
}

/// Where the brain stands toward the contact being classified.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Standing {
    /// The brain's level (4 while it flees, 3 while it fights).
    pub level: i32,
    /// The contact is the one it flees or attends.
    pub is_focus: bool,
}

/// Classifies `c`, which keeps its level; returns the level and its score.
/// `way_out` answers whether a flight from the contact has somewhere to go.
pub fn classify(
    c: &mut ContactRecord,
    morale: &mut Morale,
    at: Standing,
    mut way_out: impl FnMut() -> bool,
) -> (i32, f32) {
    if at.level == 3 {
        c.marks &= !1;
    }
    c.level = 0;
    if 0.5 < c.alarm {
        // The original also clears the threat of an alarm past 10, but on
        // its working copy after storing it: the stored record keeps it.
        morale.alarm_sum += c.alarm;
        morale.alarm_count += 1;
    }
    let heat = c.hostility + c.concern;
    let s = stress(morale);
    let mut fear = 0.75f32;
    if at.level == 4 {
        if at.is_focus {
            fear = 0.0;
            if morale.fled_alarm > c.alarm {
                c.alarm = morale.fled_alarm;
            }
        } else {
            fear = 0.75;
        }
    }
    if s > 3.0 && fear < c.alarm && way_out() {
        c.level = 4;
        return (4, c.alarm);
    }
    let b = c.tolerance;
    let threshold = if 0.0 < b {
        b * b * 7.0 + (2.0 + b)
    } else {
        2.0 + b
    };
    if threshold + s < heat || (threshold < heat && 1.5 < c.alarm) {
        // The original tests the level's bit 2 first, but the level was
        // just cleared, so the engaged mark always sets.
        c.marks |= 2;
        c.level = if c.flags & UNREACHABLE != 0 { 2 } else { 3 };
        return (c.level, heat);
    }
    if 1.0 < c.concern {
        if 8.0 - s < c.concern && way_out() {
            c.level = 4;
            return (4, c.alarm);
        }
        c.level = 1;
        return (1, c.concern);
    }
    let mut score = (c.concern + c.hostility) * 6.0 + (c.noticed - c.attention);
    if at.is_focus {
        score += 3.0;
    }
    (0, score)
}

/// One candidate the pass weighs: its level and score.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Candidate {
    pub id: u64,
    pub level: i32,
    pub score: f32,
}

/// What the pass decided.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Outcome {
    /// Flee `id`; the remembered alarm is at least `alarm`.
    Flee { id: u64, alarm: f32 },
    /// Nothing above a notice: drop the foe, attend `noticed` if any.
    Calm { noticed: Option<u64> },
    /// Fight `id` at `level` (the watch, or 3).
    Fight { id: u64, level: i32 },
}

/// The pass's running best: the strongest level and the best noticed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Best {
    pub level: i32,
    pub strong: Option<u64>,
    pub strong_score: f32,
    pub noticed: Option<u64>,
    pub noticed_score: f32,
}

impl Best {
    /// The pass's start: the foe it flees, or the foe it fights while its
    /// heat passes the raw stress plus 2.
    /// `attended` is the score of what it already attends, if anything.
    pub fn start(
        foe: Option<(u64, &ContactRecord)>,
        level: i32,
        fresh: bool,
        morale: &Morale,
        attended: Option<f32>,
    ) -> Self {
        let mut best = Best {
            level: 0,
            strong: None,
            strong_score: 0.0,
            noticed: None,
            noticed_score: attended.unwrap_or(0.0),
        };
        if let Some((id, c)) = foe {
            if level == 4 {
                best.level = 4;
                best.strong_score = morale.fled_alarm;
                best.strong = Some(id);
            } else if !fresh {
                let heat = c.concern + c.hostility;
                let raw = (morale.base + morale.stress) - morale.courage;
                if raw + 2.0 < heat {
                    best.level = 3;
                    best.strong = Some(id);
                    best.strong_score = heat;
                }
            }
        }
        best
    }

    /// Weighs one candidate.
    pub fn weigh(&mut self, c: Candidate) {
        if c.level < 0 {
            return;
        }
        if self.level < c.level || Some(c.id) == self.strong {
            self.strong_score = c.score;
            self.strong = Some(c.id);
            self.noticed = Some(c.id);
            self.level = c.level;
        } else if c.level == self.level {
            if c.level == 0 {
                if self.noticed_score < c.score {
                    self.noticed_score = c.score;
                    self.noticed = Some(c.id);
                }
            } else if self.strong_score < c.score {
                self.strong_score = c.score;
                self.strong = Some(c.id);
                self.noticed = Some(c.id);
            }
        }
    }

    /// What the pass does with its best.
    pub fn outcome(&self) -> Outcome {
        match (self.level, self.strong) {
            (4, Some(id)) => Outcome::Flee {
                id,
                alarm: self.strong_score,
            },
            (l, Some(id)) if l >= 1 => Outcome::Fight {
                id,
                level: if l < 2 { l } else { 3 },
            },
            _ => Outcome::Calm {
                noticed: self.noticed,
            },
        }
    }
}

/// The pass's alarm mean: the sum over the root of its count (in double)
/// when more than one added, doubled, raises the stress.
pub fn settle_alarm(m: &mut Morale) {
    if 1 < m.alarm_count {
        m.alarm_sum = (f64::from(m.alarm_sum) / f64::from(m.alarm_count).sqrt()) as f32;
    }
    let doubled = m.alarm_sum * 2.0;
    if m.stress < doubled {
        m.stress = doubled;
    }
}

/// A sleeper that classified anyone above a notice wakes.
pub fn wakes(asleep: bool, best_level: i32) -> bool {
    asleep && 0 < best_level
}

/// The smallest float the integration counts as seen.
pub const TINY: f32 = f32::from_bits(0x33ff_ffd7);
/// The notice scale a quiet brain takes (the f32 of 0.99).
pub const QUIET_NOTICE: f32 = f32::from_bits(0x3f7d_70a4);
/// The alarm a brain without a focus or foe keeps while it is not fleeing.
pub const CALM_ALARM: f32 = 0.75;
/// The satisfaction above which the readiness rises instead of falling.
pub const READINESS_STEP: f32 = 0.0125;
/// A wake knocks its sleeper out (the knockout call's delay).
pub const KNOCKOUT: i32 = 0;
/// A greeting reaches this far (cm).
pub const GREETING_CM: f32 = 600.0;
/// The role a waking sleeper drops.
pub const SLEEP_ROLE_ID: u32 = 44;

/// What the observer sees of its targets this tick.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FoeSight {
    /// The body's foe.
    pub foe: Option<u64>,
    /// The eye can see the foe.
    pub sees_foe: bool,
    /// The foe's aim point (cm).
    pub foe_point: Vec3,
    /// The foe's ground height (cm).
    pub foe_ground_y: f32,
    /// The foe's room.
    pub foe_room: i32,
    /// A non-character focus's point (cm).
    pub focus: Option<Vec3>,
}

/// The brain's aggregates across its contacts.
#[derive(Debug, Clone, PartialEq)]
pub struct Integration {
    /// The brain's level (4 while fleeing).
    pub level: i32,
    /// The alarm a flight started at.
    pub fled_alarm: f32,
    /// The target is fresh (visible this tick).
    pub fresh: bool,
    /// The sum of the contacts' concern.
    pub concern_sum: f32,
    /// The scale the attention rises by.
    pub notice_scale: f32,
    /// The concern sum plus the worst hostility.
    pub total: f32,
    /// The worst hostility.
    pub anger: f32,
    /// How ready the brain is, 0 through 1.
    pub readiness: f32,
    /// A per-tick value that damps the soothe term.
    pub decay: f32,
    pub alarm: f32,
    pub stress: f32,
    pub courage: f32,
    /// What it attends.
    pub attention: Option<u64>,
    /// The scripted focus.
    pub scripted: Option<u64>,
    /// The soothe scale the perception sets.
    pub soothe_scale: f32,
    /// The body is running, and its locomotion flags.
    pub running: bool,
    pub run_flags: u32,
    /// The body's position overflowed the navigation grid.
    pub over_map: bool,
    /// The brain perceives (its live state).
    pub active: bool,
    /// The room it stands in.
    pub room: i32,
}

impl Default for Integration {
    fn default() -> Self {
        Self {
            level: 0,
            fled_alarm: 0.0,
            fresh: false,
            concern_sum: 0.0,
            notice_scale: QUIET_NOTICE,
            total: 0.0,
            anger: 0.0,
            readiness: 0.0,
            decay: 0.0,
            alarm: 0.0,
            stress: 0.0,
            courage: 0.0,
            attention: None,
            scripted: None,
            soothe_scale: 1.0,
            running: false,
            run_flags: 0,
            over_map: false,
            active: true,
            room: 0,
        }
    }
}

/// What the contact integration leaves besides the contacts: the body's
/// target point, when it has one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Integrated {
    pub target: Option<Vec3>,
    /// The foe's room, which a flight remembers.
    pub room: i32,
}

/// One tick over the contacts: the aim point and freshness, hostility
/// calming, the concern rising toward its target, the noticed level
/// following the noticeability, the attention, the bad-route marks
/// expiring, and the aggregates the brain reads.
pub fn integrate(
    state: &mut Integration,
    contacts: &mut [ContactRecord],
    sight: &FoeSight,
    now_ms: u32,
) -> Integrated {
    let mut out = Integrated {
        target: None,
        room: state.room,
    };
    match sight.foe {
        None => {
            if let Some(point) = sight.focus {
                out.target = Some(point);
                state.fresh = true;
            } else if state.level == 4 {
                state.fled_alarm = (f64::from(state.fled_alarm) * 0.99) as f32;
            } else {
                state.fled_alarm = CALM_ALARM;
            }
        }
        Some(_) => {
            if sight.sees_foe {
                out.target = Some(Vec3::new(
                    sight.foe_point.x,
                    sight.foe_ground_y,
                    sight.foe_point.z,
                ));
                out.room = sight.foe_room;
                state.room = sight.foe_room;
                state.fresh = true;
            } else {
                state.fresh = false;
            }
        }
    }
    state.anger = 0.0;
    state.decay = (f64::from(state.decay) - 0.001) as f32;
    if state.decay < 0.0 {
        state.decay = 0.0;
    }
    let drift = ((1.0 - state.decay) as f64 * 5e-05) as f32;
    let one_minus = 1.0 - state.decay;
    let soothe_scale = state.soothe_scale;
    let scripted = state.scripted;
    let attention = state.attention;
    let active = state.active;
    let mut concern_sum = 0.0f32;
    let mut noticed_sum = 0.0f32;
    let mut anger = 0.0f32;
    for c in contacts.iter_mut() {
        if active {
            c.ticks_seen = c.ticks_seen.wrapping_add(1);
            if c.sighting > TINY && !c.dead {
                if state.over_map {
                    let drop = (f64::from(one_minus) * f64::from(c.soothe - 100.0) * 0.0001) as f32;
                    c.hostility -= drop;
                } else if 0.0 < c.soothe {
                    c.hostility -= (c.soothe * drift) * soothe_scale * one_minus;
                }
            }
            if c.concern < c.concern_target {
                let value =
                    (f64::from(c.concern_target - c.concern) * 0.005 + f64::from(c.concern)) as f32;
                c.concern = value;
            } else {
                c.concern = (f64::from(c.concern) * 0.9997) as f32;
            }
            c.decay = (f64::from(c.decay) * 0.999) as f32;
            concern_sum += c.concern;
            c.hostility = (f64::from(c.hostility) * 0.999) as f32;
            if anger < c.hostility {
                anger = c.hostility;
            }
            if c.noticeability < c.noticed {
                c.noticed = (f64::from(c.noticed) - 0.01) as f32;
            } else {
                c.noticed = (f64::from(c.noticeability) * 0.005 + f64::from(c.noticed)) as f32;
            }
            noticed_sum += c.noticed;
            if attention == Some(c.id) {
                if scripted != Some(c.id) && state.level < 1 {
                    c.attention += (f64::from(state.notice_scale) * 0.0006) as f32;
                }
            } else if 0.0 < c.attention {
                c.attention = (f64::from(c.attention) - 0.002) as f32;
            }
            if c.flags & UNREACHABLE != 0 && c.unreachable_until_ms < now_ms {
                c.flags &= !UNREACHABLE;
            }
        } else {
            c.concern = (f64::from(c.concern) * 0.9997) as f32;
            concern_sum += c.concern;
            c.ticks_seen = 0;
            c.hostility = (f64::from(c.hostility) * 0.999) as f32;
            if anger < c.hostility {
                anger = c.hostility;
            }
            if 0.0 < c.noticed {
                c.noticed = (f64::from(c.noticed) - 0.001) as f32;
            }
            if 0.0 < c.attention {
                c.attention = (f64::from(c.attention) - 0.001) as f32;
            }
            c.sighting = 0.0;
            c.stamp = 0.0;
        }
    }
    state.concern_sum = concern_sum;
    state.notice_scale = if noticed_sum <= 1.0 {
        QUIET_NOTICE
    } else {
        (noticed_sum / (noticed_sum + 100.0)) * 100.0
    };
    state.anger = anger;
    state.total = anger + concern_sum;
    let mut interest = concern_sum;
    if TINY < concern_sum {
        let squared = concern_sum * concern_sum;
        interest = (squared / (squared + 10.0)) * 40.0;
    }
    if state.running {
        let floor = if state.run_flags & 4 == 0 { 5.0 } else { 15.0 };
        interest = interest.max(floor);
    }
    let stress = ((state.stress + state.alarm) - state.courage) * 10.0;
    interest = interest.max(stress);
    if interest <= state.readiness {
        if READINESS_STEP < state.readiness {
            state.readiness -= READINESS_STEP;
        } else {
            state.readiness += 0.0;
        }
    } else {
        state.readiness += (interest - state.readiness) * 0.01;
    }
    out
}

/// A sighting the pass weighs: a character (or object) and its fresh
/// contact record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sighting {
    pub contact: ContactRecord,
    /// It is a character (kind 1), not an object.
    pub character: bool,
}

/// What the target pass did.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PassReport {
    pub outcome: Outcome,
    /// The brain starts a flight at this alarm.
    pub fled_alarm: f32,
    /// The brain leaves combat for its activity.
    pub leave: bool,
    /// The brain enters combat at this level.
    pub enter: Option<i32>,
    /// The brain targets this contact.
    pub target: Option<u64>,
    /// The encounter script of this contact.
    pub encounter: Option<u64>,
    /// The greeting of this contact.
    pub greet: Option<u64>,
    /// A sleeper classified someone and wakes (dropping the sleep role).
    pub wake: bool,
    pub remove_sleep_role: bool,
    /// The contact the round-robin dialogue refresh asks for.
    pub refresh: Option<usize>,
    /// The new attention target.
    pub attention: Option<u64>,
    /// The new level.
    pub level: i32,
    /// The stress the alarm mean raised to.
    pub stress: f32,
    /// The sleeper is knocked out.
    pub knockout: bool,
}

impl Default for Outcome {
    fn default() -> Self {
        Outcome::Calm { noticed: None }
    }
}

/// The target pass over a brain's contacts.
///
/// The contacts are cleared of their noticed and sighting marks, the foe
/// it flees stays a flight and the foe it fights stays an engagement while
/// its heat passes the raw stress plus 2, a scripted focus is weighed
/// first, then every visible character and every nearby one. A sleeper
/// that classifies anyone above a notice wakes and is knocked out, the
/// unkept contacts are cleaned, the alarm mean raises the stress, and the
/// pass starts a flight, attends what it noticed or enters combat. The
/// round-robin cursor then offers the next seen character for the
/// dialogue refresh.
pub struct PassInput<'a> {
    /// The brain's contact list, updated in place.
    pub contacts: &'a mut Vec<ContactRecord>,
    pub morale: &'a mut Morale,
    pub level: i32,
    pub fresh: bool,
    pub foe: Option<u64>,
    pub asleep: bool,
    pub in_combat: bool,
    /// The score of what the brain already attends.
    pub attended: Option<f32>,
    /// The scripted focus's sighting.
    pub scripted: Option<Sighting>,
    /// The round-robin cursor.
    pub cursor: usize,
    /// What the brain attended before the pass.
    pub attended_id: Option<u64>,
    /// The greeting's gates: the notice scale and the readiness.
    pub greet_gates: (f32, f32),
}

/// Runs the pass. `visible` then `nearby` are weighed in order; `distance`
/// gives a character's distance for the greeting; `way_out` answers
/// whether a flight has somewhere to go.
#[allow(clippy::too_many_arguments)]
pub fn target_pass(
    input: PassInput<'_>,
    visible: &[Sighting],
    nearby: &[Sighting],
    mut distance: impl FnMut(u64) -> f32,
    mut way_out: impl FnMut() -> bool,
) -> PassReport {
    let scripted_id = input.scripted.map(|s| s.contact.id);
    let PassInput {
        contacts,
        morale,
        level,
        fresh,
        foe,
        asleep,
        in_combat,
        attended,
        scripted,
        cursor,
        greet_gates,
        attended_id,
    } = input;
    for c in contacts.iter_mut() {
        c.noticed = 0.0;
        c.sighting = 0.0;
        c.stamp = 0.0;
        c.flags &= !crate::combatant::CONTACT_NOTICED;
    }
    let foe_record = foe.and_then(|id| contacts.iter().find(|c| c.id == id).copied());
    let mut best = Best::start(
        foe_record.as_ref().map(|c| (c.id, c)),
        level,
        fresh,
        morale,
        attended,
    );
    let mut report = PassReport {
        level: best.level,
        attention: None,
        stress: 0.0,
        ..PassReport::default()
    };
    let weigh = |sighting: &Sighting,
                 best: &mut Best,
                 contacts: &mut Vec<ContactRecord>,
                 morale: &mut Morale,
                 way_out: &mut dyn FnMut() -> bool|
     -> (Option<usize>, i32, f32) {
        let mut record = sighting.contact;
        let at = Standing {
            level,
            is_focus: Some(record.id) == foe,
        };
        let (l, score) = classify(&mut record, morale, at, &mut *way_out);
        record.level = l;
        let index = if let Some(i) = contacts.iter().position(|c| c.id == record.id) {
            contacts[i] = record;
            Some(i)
        } else if contacts.len() < MOST_CONTACTS {
            contacts.push(record);
            Some(contacts.len() - 1)
        } else {
            admit(contacts, &record).inspect(|&i| {
                contacts[i] = record;
            })
        };
        if l >= 0 {
            best.weigh(Candidate {
                id: record.id,
                level: l,
                score,
            });
        }
        (index, l, score)
    };
    if let Some(scripted) = &scripted {
        let (_, level, score) = weigh(
            scripted,
            &mut best,
            contacts,
            morale,
            &mut way_out as &mut dyn FnMut() -> bool,
        );
        // The scripted focus becomes what the brain attends outright; its
        // strong score is the 0 the original leaves unset, so any sighting
        // of the same level wins the tie.
        best.level = level;
        best.strong = Some(scripted.contact.id);
        best.strong_score = 0.0;
        best.noticed = Some(scripted.contact.id);
        best.noticed_score = score.min(scripted.contact.concern);
    }
    for sighting in visible.iter().chain(nearby.iter()) {
        weigh(
            sighting,
            &mut best,
            contacts,
            morale,
            &mut way_out as &mut dyn FnMut() -> bool,
        );
    }
    if asleep && 0 < best.level {
        report.wake = true;
        report.remove_sleep_role = true;
        report.knockout = true;
    }
    let strong = best.strong;
    for c in contacts.iter_mut() {
        c.ticks_seen = 0;
        if c.flags & crate::combatant::CONTACT_NOTICED == 0 && Some(c.id) != strong {
            c.level = 0;
            c.noticed = 0.0;
            c.noticeability = 0.0;
            c.sighting = 0.0;
            c.stamp = 0.0;
        }
        if c.level < 1 {
            c.priority = 0.0;
        }
    }
    settle_alarm(morale);
    report.stress = morale.stress;
    report.outcome = best.outcome();
    report.level = best.level;
    match best.outcome() {
        Outcome::Flee { id, alarm } => {
            report.attention = Some(id);
            report.fled_alarm = alarm.max(report.fled_alarm);
            report.leave = false;
        }
        Outcome::Calm { noticed } => match noticed {
            None => {
                // The scripted focus stays attended even with nothing seen.
                report.attention = if attended_id == scripted_id {
                    attended_id
                } else {
                    None
                };
            }
            Some(id) => {
                let changed = attended_id != Some(id);
                report.attention = Some(id);
                if changed
                    && let Some(sighting) = visible
                        .iter()
                        .chain(nearby.iter())
                        .find(|s| s.contact.id == id)
                    && sighting.character
                {
                    report.encounter = Some(id);
                    let (notice, readiness) = greet_gates;
                    if notice <= 6.0 && readiness <= 1.0 && distance(id) <= GREETING_CM {
                        report.greet = Some(id);
                    }
                }
                report.leave = in_combat;
            }
        },
        Outcome::Fight { id, level } => {
            let changed = attended_id != Some(id);
            report.attention = Some(id);
            if changed
                && let Some(sighting) = visible
                    .iter()
                    .chain(nearby.iter())
                    .find(|s| s.contact.id == id)
                && sighting.character
            {
                report.encounter = Some(id);
                let (notice, readiness) = greet_gates;
                if notice <= 6.0 && readiness <= 1.0 && distance(id) <= GREETING_CM {
                    report.greet = Some(id);
                }
            }
            report.target = Some(id);
            report.enter = if in_combat { None } else { Some(level) };
            // A level char enters at 3 for an engagement (2 and above),
            // else at its own level; already fighting it keeps the level.
            report.level = if in_combat || level < 2 { level } else { 3 };
        }
    }
    report.refresh = refresh_slot(contacts, cursor);
    report
}

/// The round-robin's next contact: from the cursor it steps at least once,
/// wrapping at the second-to-last slot, and stops at the first it has
/// seen; a character there asks for the dialogue refresh. The last slot is
/// never stepped onto by the wrap, as in the original.
pub fn refresh_slot(contacts: &[ContactRecord], cursor: usize) -> Option<usize> {
    if contacts.is_empty() {
        return None;
    }
    let mut cursor = cursor;
    if contacts.len() <= cursor {
        cursor = 0;
    }
    let mut at = cursor + 1;
    loop {
        if at < contacts.len() - 1 {
            at += 1;
        } else {
            at = 0;
        }
        if TINY < contacts[at].sighting || cursor == at {
            break;
        }
    }
    if TINY < contacts[at].sighting {
        Some(at)
    } else {
        None
    }
}

#[cfg(test)]
#[path = "contacts_test.rs"]
mod tests;
