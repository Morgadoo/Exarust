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
