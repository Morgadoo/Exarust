//! Tests of the contacts, the classifier and the target pass.
//! (Included from contacts.rs: `#[path = "contacts_test.rs"] mod tests;`)

use super::*;

fn contact(concern: f32, hostility: f32) -> ContactRecord {
    ContactRecord {
        concern,
        hostility,
        ..ContactRecord::default()
    }
}

#[test]
fn a_full_list_replaces_its_lightest_contact() {
    let mut list = vec![contact(5.0, 0.0); 12];
    assert_eq!(admit(&list[..11], &contact(0.0, 0.0)), Some(11));
    list[7].concern = 1.0;
    // Slot 7 weighs 1: a contact of weight 2 replaces it, one of 0.5 does not.
    assert_eq!(admit(&list, &contact(2.0, 0.0)), Some(7));
    assert_eq!(admit(&list, &contact(0.5, 0.0)), None);
    // The first slot's sighting counts by its size, the others' by sign.
    list[0].sighting = -1.0;
    list[3].sighting = -1.0;
    assert_eq!(admit(&list, &contact(0.0, 0.0)), Some(3));
}

#[test]
fn heat_past_the_threshold_engages() {
    let mut m = Morale::default();
    let at = Standing::default();
    // Tolerance 0: the threshold is 2.
    let mut c = contact(1.0, 1.5);
    assert_eq!(classify(&mut c, &mut m, at, || false), (3, 2.5));
    assert_eq!((c.level, c.marks), (3, 2));
    // Unreachable: engaging is level 2.
    let mut c = ContactRecord {
        flags: 0x10,
        ..contact(1.0, 1.5)
    };
    assert_eq!(classify(&mut c, &mut m, at, || false).0, 2);
    // Tolerance 0.5: 7·0.25 + 2.5 = 4.25; heat 4 is under it (and a concern
    // of 1 is no watch), 4.3 passes it, and an alarm past 1.5 engages at 4
    // even with stress.
    let mut c = ContactRecord {
        tolerance: 0.5,
        ..contact(1.0, 3.0)
    };
    assert_eq!(classify(&mut c, &mut m, at, || false).0, 0);
    c.hostility = 3.3;
    assert_eq!(classify(&mut c, &mut m, at, || false).0, 3);
    let mut c = ContactRecord {
        tolerance: 0.5,
        alarm: 1.6,
        ..contact(1.0, 3.3)
    };
    m.stress = 10.0;
    assert_eq!(classify(&mut c, &mut m, at, || false).0, 3);
    // A negative tolerance lowers the threshold to 2 + b.
    let mut m = Morale::default();
    let mut c = ContactRecord {
        tolerance: -0.5,
        ..contact(0.0, 1.6)
    };
    assert_eq!(classify(&mut c, &mut m, at, || false).0, 3);
}

#[test]
fn stress_and_alarm_flee() {
    let at = Standing::default();
    // Stress 4 (base 1 + 4 − courage 1) and alarm 0.8 flee with a way out.
    let mut m = Morale {
        courage: 1.0,
        base: 1.0,
        stress: 4.0,
        ..Morale::default()
    };
    assert_eq!(stress(&m), 4.0);
    let mut c = ContactRecord {
        alarm: 0.8,
        ..contact(0.0, 0.0)
    };
    assert_eq!(classify(&mut c, &mut m, at, || true), (4, 0.8));
    assert_eq!((m.alarm_sum, m.alarm_count), (0.8, 1));
    // Without one, the same contact is only noticed.
    let mut c = ContactRecord {
        alarm: 0.8,
        noticed: 2.0,
        attention: 0.5,
        ..contact(0.0, 0.0)
    };
    assert_eq!(classify(&mut c, &mut m, at, || false), (0, 1.5));
    // The contact it already flees keeps the flight at any alarm, raised to
    // the alarm it fled at.
    m.fled_alarm = 2.0;
    let fleeing = Standing {
        level: 4,
        is_focus: true,
    };
    let mut c = ContactRecord {
        alarm: 0.1,
        ..contact(0.0, 0.0)
    };
    assert_eq!(classify(&mut c, &mut m, fleeing, || true), (4, 2.0));
    // Concern past 8 less the stress flees; under it, watches.
    let mut c = contact(4.5, 0.0);
    c.tolerance = 1.0;
    assert_eq!(classify(&mut c, &mut m, at, || true).0, 4);
    let mut calm = Morale::default();
    assert_eq!(classify(&mut c, &mut calm, at, || true), (1, 4.5));
    // Courage past the stress leaves 0.
    let brave = Morale {
        courage: 5.0,
        stress: 1.0,
        ..Morale::default()
    };
    assert_eq!(stress(&brave), 0.0);
}

#[test]
fn a_notice_scores_heat_and_attention() {
    let mut m = Morale::default();
    let mut c = ContactRecord {
        noticed: 1.0,
        attention: 0.25,
        ..contact(0.5, 0.25)
    };
    let focus = Standing {
        level: 0,
        is_focus: true,
    };
    assert_eq!(
        classify(&mut c, &mut m, focus, || false),
        (0, 0.75 * 6.0 + 0.75 + 3.0)
    );
    // The engaging brain clears the contact's first mark.
    let mut c = ContactRecord {
        marks: 1,
        ..contact(0.0, 0.0)
    };
    let fighting = Standing {
        level: 3,
        is_focus: false,
    };
    classify(&mut c, &mut m, fighting, || false);
    assert_eq!(c.marks, 0);
}

#[test]
fn the_pass_keeps_the_strongest() {
    let m = Morale::default();
    let mut best = Best::start(None, 0, false, &m, None);
    best.weigh(Candidate {
        id: 1,
        level: 0,
        score: 2.0,
    });
    best.weigh(Candidate {
        id: 2,
        level: 0,
        score: 3.0,
    });
    assert_eq!(best.outcome(), Outcome::Calm { noticed: Some(2) });
    best.weigh(Candidate {
        id: 3,
        level: 1,
        score: 1.0,
    });
    best.weigh(Candidate {
        id: 4,
        level: 1,
        score: 1.5,
    });
    assert_eq!(best.outcome(), Outcome::Fight { id: 4, level: 1 });
    best.weigh(Candidate {
        id: 5,
        level: 2,
        score: 0.1,
    });
    assert_eq!(best.outcome(), Outcome::Fight { id: 5, level: 3 });
    best.weigh(Candidate {
        id: 6,
        level: 4,
        score: 0.9,
    });
    assert_eq!(best.outcome(), Outcome::Flee { id: 6, alarm: 0.9 });
    // Its current pick is renewed even at a lower level.
    best.weigh(Candidate {
        id: 6,
        level: 1,
        score: 0.2,
    });
    assert_eq!(best.outcome(), Outcome::Fight { id: 6, level: 1 });
}

#[test]
fn the_pass_starts_from_the_foe() {
    let m = Morale {
        fled_alarm: 1.25,
        ..Morale::default()
    };
    let foe = contact(1.0, 1.5);
    let best = Best::start(Some((9, &foe)), 4, false, &m, None);
    assert_eq!(
        (best.level, best.strong, best.strong_score),
        (4, Some(9), 1.25)
    );
    // A fought foe stays while its heat passes the raw stress plus 2.
    let best = Best::start(Some((9, &foe)), 3, false, &m, None);
    assert_eq!((best.level, best.strong_score), (3, 2.5));
    let cool = contact(1.0, 0.5);
    assert_eq!(Best::start(Some((9, &cool)), 3, false, &m, None).level, 0);
    assert_eq!(Best::start(Some((9, &foe)), 3, true, &m, None).level, 0);
    assert_eq!(
        Best::start(None, 0, false, &m, Some(4.0)).noticed_score,
        4.0
    );
}

#[test]
fn the_alarm_mean_raises_the_stress() {
    let mut m = Morale {
        alarm_sum: 3.0,
        alarm_count: 4,
        stress: 1.0,
        ..Morale::default()
    };
    settle_alarm(&mut m);
    assert_eq!((m.alarm_sum, m.stress), (1.5, 3.0));
    let mut one = Morale {
        alarm_sum: 0.6,
        alarm_count: 1,
        stress: 2.0,
        ..Morale::default()
    };
    settle_alarm(&mut one);
    assert_eq!(one.stress, 2.0);
    assert!(wakes(true, 1) && !wakes(true, 0) && !wakes(false, 3));
}

#[test]
fn a_combatant_contact_is_classified() {
    let mut c = crate::combatant::Contact::new(bevy::prelude::Entity::PLACEHOLDER);
    c.concern = 1.0;
    c.hostility = 1.5;
    c.noticeability = 0.25;
    let mut r = ContactRecord::of(&c, 7);
    assert_eq!((r.id, r.sighting, r.attention), (7, 0.25, 0.0));
    let mut m = Morale::default();
    assert_eq!(
        classify(&mut r, &mut m, Standing::default(), || false),
        (3, 2.5)
    );
}

/// The perception's words reach the classifier's record: closeness is this
/// tick's sighting, the attitude is the soothing term and the threat
/// estimate is the alarm.
#[test]
fn a_perceived_contact_carries_its_words() {
    let mut c = crate::combatant::Contact::new(bevy::prelude::Entity::PLACEHOLDER);
    c.closeness = 0.75;
    c.attitude = -40.0;
    c.threat = 1.25;
    c.concern_target = 0.5;
    let r = ContactRecord::of(&c, 9);
    assert_eq!((r.seen, r.soothe, r.alarm), (0.75, -40.0, 1.25));
    assert_eq!((r.threat, r.concern_target), (1.25, 0.5));
}

/// A contact seen this tick, with a hostile soothe term (the perceived
/// attitude), a rising concern and a sighting of 2 its noticed level rises
/// toward.
fn seen(id: u64) -> ContactRecord {
    ContactRecord {
        id,
        seen: 1.0,
        soothe: -20.0,
        concern: 1.0,
        concern_target: 2.0,
        sighting: 2.0,
        hostility: 1.0,
        ..ContactRecord::default()
    }
}

#[test]
fn the_contacts_sober_and_rise_toward_their_targets() {
    let mut state = Integration {
        soothe_scale: 1.0,
        ..Integration::default()
    };
    let mut contacts = vec![seen(5)];
    let sight = FoeSight {
        foe: Some(5),
        sees_foe: true,
        foe_point: Vec3::new(100.0, 50.0, 0.0),
        foe_ground_y: 10.0,
        foe_room: 3,
        focus: None,
    };
    let out = integrate(&mut state, &mut contacts, &sight, 1000);
    let c = contacts[0];
    // Hostility: 1 − (−20)·5e-05 = 1.001, then ×0.999.
    assert!((c.hostility - 0.999_999).abs() < 1e-6, "{}", c.hostility);
    // Concern rises 0.5 % of the gap, the noticed level 0.5 % of the
    // sighting.
    assert!((c.concern - 1.005).abs() < 1e-6);
    assert!((c.noticed - 0.01).abs() < 1e-6);
    assert!(state.fresh);
    assert_eq!(out.target, Some(Vec3::new(100.0, 10.0, 0.0)));
    assert_eq!(out.room, 3);
    // The sums: 1.005 of concern, 0.01 of notice (a quiet scale), and the
    // anger is the worst hostility.
    assert!((state.concern_sum - 1.005).abs() < 1e-6);
    assert!((state.notice_scale - QUIET_NOTICE).abs() < 1e-7);
    assert!((state.anger - c.hostility).abs() < 1e-6);
    // The interest is (s²/(s²+10))·40.
    let s = 1.005f32;
    let interest = (s * s / (s * s + 10.0)) * 40.0;
    assert!(
        (state.readiness - interest * 0.01).abs() < 1e-5,
        "{}",
        state.readiness
    );
}

#[test]
fn a_friendly_attitude_does_not_lower_hostility_without_the_decay() {
    // The assembly's branch takes the soothe term only when it is below
    // zero; a positive attitude leaves hostility to its own 0.999 decay.
    let mut state = Integration {
        soothe_scale: 1.0,
        ..Integration::default()
    };
    let mut contacts = vec![ContactRecord {
        id: 9,
        seen: 1.0,
        soothe: 20.0,
        hostility: 1.0,
        ..ContactRecord::default()
    }];
    integrate(&mut state, &mut contacts, &FoeSight::default(), 0);
    assert!((contacts[0].hostility - 0.999).abs() < 1e-6);
}

#[test]
fn a_contact_above_its_targets_falls_and_a_bad_route_expires() {
    let mut state = Integration::default();
    let mut contacts = vec![ContactRecord {
        id: 2,
        seen: 1.0,
        concern: 2.0,
        concern_target: 1.0,
        noticed: 1.0,
        sighting: 0.5,
        soothe: 0.0,
        hostility: 2.0,
        flags: UNREACHABLE,
        unreachable_until_ms: 1000,
        ..ContactRecord::default()
    }];
    let sight = FoeSight::default();
    integrate(&mut state, &mut contacts, &sight, 1000);
    let c = contacts[0];
    assert!((c.concern - 2.0 * 0.9997).abs() < 1e-6);
    assert!((c.noticed - 0.99).abs() < 1e-6);
    // No soothe: only the 0.999 decay applies.
    assert!((c.hostility - 2.0 * 0.999).abs() < 1e-6);
    // The mark lasts exactly to its clock, then clears.
    assert_eq!(c.flags & UNREACHABLE, UNREACHABLE);
    integrate(&mut state, &mut contacts, &sight, 1001);
    assert_eq!(contacts[0].flags & UNREACHABLE, 0);
    // Without a foe or a focus the calm alarm is three quarters.
    assert_eq!(state.fled_alarm, CALM_ALARM);
}

#[test]
fn a_passive_brain_lets_its_contacts_fade() {
    let mut state = Integration {
        active: false,
        ..Integration::default()
    };
    let mut contacts = vec![ContactRecord {
        id: 1,
        seen: 1.0,
        sighting: 1.0,
        concern: 1.0,
        noticed: 1.0,
        attention: 1.0,
        hostility: 1.0,
        ..ContactRecord::default()
    }];
    integrate(&mut state, &mut contacts, &FoeSight::default(), 0);
    let c = contacts[0];
    assert_eq!(c.ticks_seen, 0);
    // This tick's sighting clears; the admission's sighting stays.
    assert_eq!((c.seen, c.sighting), (0.0, 1.0));
    assert!((c.concern - 0.9997).abs() < 1e-6);
    assert!((c.noticed - 0.999).abs() < 1e-6);
    assert!((c.attention - 0.999).abs() < 1e-6);
}

#[test]
fn a_noticed_character_is_met_greeted_and_entered_in_combat() {
    let mut contacts = vec![ContactRecord::default()];
    let mut morale = Morale::default();
    let sighting = Sighting {
        contact: ContactRecord {
            id: 1,
            sighting: 1.0,
            concern: 2.0,
            ..ContactRecord::default()
        },
        character: true,
    };
    let report = target_pass(
        PassInput {
            contacts: &mut contacts,
            morale: &mut morale,
            level: 0,
            fresh: false,
            foe: None,
            asleep: false,
            in_combat: false,
            attended: None,
            scripted: None,
            cursor: 0,
            attended_id: None,
            greet_gates: (0.99, 1.0),
        },
        &[sighting],
        &[],
        |_| 250_000.0,
        || false,
    );
    assert_eq!(report.outcome, Outcome::Fight { id: 1, level: 1 });
    assert_eq!(report.target, Some(1));
    assert_eq!(report.enter, Some(1));
    assert_eq!(report.level, 1);
    assert_eq!(report.encounter, Some(1));
    assert_eq!(report.greet, Some(1));
    assert_eq!(report.attention, Some(1));
}

#[test]
fn a_greeting_stays_inside_its_six_metres_and_its_gates() {
    let contacts = Vec::new();
    let morale = Morale::default();
    let sighting = Sighting {
        contact: ContactRecord {
            id: 1,
            sighting: 1.0,
            concern: 2.0,
            ..ContactRecord::default()
        },
        character: true,
    };
    let pass = |distance: f32, gates: (f32, f32)| {
        let distance2 = distance * distance;
        target_pass(
            PassInput {
                contacts: &mut contacts.clone(),
                morale: &mut morale.clone(),
                level: 0,
                fresh: false,
                foe: None,
                asleep: false,
                in_combat: false,
                attended: None,
                scripted: None,
                cursor: 0,
                attended_id: None,
                greet_gates: gates,
            },
            &[sighting],
            &[],
            |_| distance2,
            || false,
        )
    };
    assert_eq!(pass(600.0, (0.99, 1.0)).greet, Some(1));
    assert_eq!(pass(601.0, (0.99, 1.0)).greet, None);
    assert_eq!(pass(500.0, (6.01, 1.0)).greet, None);
    assert_eq!(pass(500.0, (0.99, 1.01)).greet, None);
    // A NaN gate or distance passes, as the original's unordered compares.
    assert_eq!(pass(f32::NAN, (f32::NAN, 1.0)).greet, Some(1));
    // A first meeting touches both before the greeting.
    assert_eq!(pass(500.0, (6.01, 1.0)).encounter, Some(1));
}

#[test]
fn the_scripted_focus_is_attended_and_any_sighting_of_its_level_wins() {
    let mut contacts = Vec::new();
    let mut morale = Morale::default();
    let scripted = Sighting {
        contact: ContactRecord {
            id: 9,
            sighting: 1.0,
            ..ContactRecord::default()
        },
        character: true,
    };
    let seen = Sighting {
        contact: ContactRecord {
            id: 1,
            sighting: 1.0,
            concern: 1.5,
            ..ContactRecord::default()
        },
        character: true,
    };
    let report = target_pass(
        PassInput {
            contacts: &mut contacts,
            morale: &mut morale,
            level: 0,
            fresh: false,
            foe: None,
            asleep: false,
            in_combat: false,
            attended: None,
            scripted: Some(scripted),
            cursor: 0,
            attended_id: None,
            greet_gates: (0.99, 1.0),
        },
        &[seen],
        &[],
        |_| 250_000.0,
        || false,
    );
    // The watch wins the scripted focus's level 0 notice.
    assert_eq!(report.attention, Some(1));
    assert_eq!(report.outcome, Outcome::Fight { id: 1, level: 1 });
    assert_eq!(contacts.len(), 2);
    // With nothing seen, the scripted focus stays attended.
    let mut contacts = Vec::new();
    let report = target_pass(
        PassInput {
            contacts: &mut contacts,
            morale: &mut morale,
            level: 0,
            fresh: false,
            foe: None,
            asleep: false,
            in_combat: false,
            attended: Some(0.0),
            scripted: Some(Sighting {
                contact: ContactRecord {
                    id: 9,
                    ..ContactRecord::default()
                },
                character: true,
            }),
            cursor: 0,
            attended_id: Some(9),
            greet_gates: (0.99, 1.0),
        },
        &[],
        &[],
        |_| 250_000.0,
        || false,
    );
    assert_eq!(report.attention, Some(9));
    // The scripted focus is what the brain attends when nothing else is
    // seen (the original's noticed slot holds it).
    assert_eq!(report.outcome, Outcome::Calm { noticed: Some(9) });
}

#[test]
fn the_round_robin_looks_two_on_from_its_cursor() {
    let mut contacts = vec![
        ContactRecord {
            id: 1,
            ..ContactRecord::default()
        },
        ContactRecord {
            id: 2,
            seen: 1.0,
            ..ContactRecord::default()
        },
        ContactRecord {
            id: 3,
            seen: 1.0,
            ..ContactRecord::default()
        },
    ];
    // From the cursor the first slot looked at is two on: the seen second
    // contact is passed over for the third.
    assert_eq!(refresh_slot(&contacts, 0), (2, Some(2)));
    // With only the second seen, the walk wraps to the cursor and stops
    // there, unseen.
    contacts[2].seen = 0.0;
    assert_eq!(refresh_slot(&contacts, 0), (0, None));
    // From the second slot the walk wraps round to it.
    assert_eq!(refresh_slot(&contacts, 1), (1, Some(1)));
    // An object asks for no refresh, but the cursor still moves.
    contacts[1].object = true;
    assert_eq!(refresh_slot(&contacts, 1), (1, None));
}

/// The hostility drift is `(1 − restraint)·5e-5`, scaled by the soothing
/// scale; an object's hostility is not touched.
#[test]
fn the_restraint_slows_the_drift() {
    let mut state = Integration {
        restraint: 0.5,
        soothe_scale: 2.0,
        ..Integration::default()
    };
    let mut contacts = vec![
        seen(5),
        ContactRecord {
            object: true,
            ..seen(6)
        },
    ];
    integrate(&mut state, &mut contacts, &FoeSight::default(), 0);
    let drift = (0.5f64 * 5e-05) as f32;
    let risen = 1.0 + (20.0 * drift) * 2.0;
    assert_eq!(contacts[0].hostility, (f64::from(risen) * 0.999) as f32);
    assert_eq!(contacts[1].hostility, (1.0f64 * 0.999) as f32);
}

/// The pass clears the concern targets and this tick's sightings, keeps
/// the noticed levels, and cleans an unkept contact's level, concern
/// target, sighting and stamp.
#[test]
fn the_pass_clears_the_concern_targets_and_cleans_the_unkept() {
    let mut contacts = vec![ContactRecord {
        id: 4,
        noticed: 0.7,
        sighting: 0.3,
        concern_target: 2.0,
        seen: 1.0,
        level: 2,
        ..ContactRecord::default()
    }];
    let mut morale = Morale::default();
    let _ = target_pass(
        PassInput {
            contacts: &mut contacts,
            morale: &mut morale,
            level: 0,
            fresh: false,
            foe: None,
            asleep: false,
            in_combat: false,
            attended: None,
            scripted: None,
            cursor: 0,
            attended_id: None,
            greet_gates: (0.99, 1.0),
        },
        &[],
        &[],
        |_| 0.0,
        || false,
    );
    let c = contacts[0];
    assert_eq!(c.noticed, 0.7);
    assert_eq!(
        (c.level, c.sighting, c.concern_target, c.seen),
        (0, 0.0, 0.0, 0.0)
    );
}
