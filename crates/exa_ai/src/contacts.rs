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
//! Evidence: the original's contact classifier and its target pass, read
//! from the instructions (sums in their order, the alarm mean in double).

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

#[cfg(test)]
#[path = "contacts_test.rs"]
mod tests;
