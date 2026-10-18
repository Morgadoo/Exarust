//! What one character makes of another: range, closeness, noticeability,
//! hostility, and whether it notices, watches, engages or flees.
//!
//! A contact is seen out to 12 m, 6 m while asleep, and 18 m once it is
//! already tracked. Closeness is at most 0.99. Hostility toward a negative
//! attitude grows each tick and is then scaled by 0.999, so with neutral
//! traits it settles near 0.0375 times the attitude's size. The contact is
//! engaged when concern plus hostility clears the tolerance threshold.
//!
//! The view weight is 1 straight ahead and 0 straight behind. The recorded
//! product used minus signs, which invert that result, so the signs here
//! follow the stated weights.
//!
//! Evidence: the perception and target-selection notes. The attack chosen
//! once a fight has started was not read.

use bevy::prelude::*;

/// How the character treats the contact this pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Notice,
    Watch,
    Engage,
    Flee,
}

/// One other character, as the observer currently rates it.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct Mind {
    /// Attitude toward the other. Negative is hostile.
    pub attitude: f32,
    /// Trait t1, the record's courage-like value.
    pub t1: f32,
    /// Trait t2, the record's aggression-like value.
    pub t2: f32,
    pub calm: f32,
    /// Tolerance, 0 to 100. The threshold uses tolerance / 100.
    pub tolerance: f32,
    pub hostility: f32,
    pub concern: f32,
    pub concern_target: f32,
    pub alarm: f32,
    /// Stress already summed. The unidentified stress term is not filled here.
    pub stress: f32,
    pub flee_route: bool,
    /// Distance to the other, in centimetres. Vertical offsets under 150 cm
    /// are left out by the caller.
    pub distance_cm: f32,
    pub tracked: bool,
    pub asleep: bool,
    pub hidden: bool,
    /// Perception light, 0 to 1, used only while the other is hidden.
    pub light: f32,
    pub frenzy: bool,
    /// The other is in the motion state that drops it from the visible pass.
    pub other_unconscious: bool,
}

impl Mind {
    /// Neutral traits, the given attitude, and the other standing on top of us.
    pub fn new(attitude: f32) -> Self {
        Self {
            attitude,
            t1: 0.0,
            t2: 0.0,
            calm: 0.0,
            tolerance: 0.0,
            hostility: 0.0,
            concern: 0.0,
            concern_target: 0.0,
            alarm: 0.0,
            stress: 0.0,
            flee_route: false,
            distance_cm: 0.0,
            tracked: false,
            asleep: false,
            hidden: false,
            light: 1.0,
            frenzy: false,
            other_unconscious: false,
        }
    }

    /// How far this observer can still detect, in centimetres.
    pub fn radius(&self) -> f32 {
        let mut radius = 1200.0;
        if self.asleep {
            radius *= 0.5;
        }
        if self.hidden {
            let light = self.light.clamp(0.0, 1.0);
            radius *= 1.0 - 0.9 * (1.0 - light);
        }
        if self.tracked {
            radius += 600.0;
        }
        radius
    }

    /// Closeness, or 0 when the other is outside the radius.
    pub fn closeness(&self) -> f32 {
        if !self.distance_cm.is_finite() || self.distance_cm > self.radius() {
            return 0.0;
        }
        (1.0 - self.distance_cm / 1200.0).min(0.99)
    }
}

/// One tick of hostility and concern for a visible contact.
pub fn tick(mind: &mut Mind) {
    let visible = mind.closeness() > 0.0 && !mind.other_unconscious;
    if visible && mind.attitude < 0.0 {
        let calm = mind.calm.clamp(0.0, 1.0);
        if mind.frenzy {
            mind.hostility -= (1.0 - calm) * (mind.attitude - 100.0) * 1e-4;
        } else {
            let p = -mind.t2 / 100.0;
            let q = (6.0 + mind.t2 / 100.0 + mind.t1 / 100.0) / 8.0;
            mind.hostility += -mind.attitude * (1.0 - calm) * (1.0 - p) * 5e-5 * q;
        }
    }
    mind.hostility *= 0.999;
    if mind.concern_target > mind.concern {
        mind.concern += (mind.concern_target - mind.concern) * 0.005;
    } else {
        mind.concern *= 0.9997;
    }
}

/// Notice, watch, engage or flee.
pub fn decide(mind: &Mind) -> Decision {
    let b = mind.tolerance / 100.0;
    let threshold = if b == 0.0 { 2.0 } else { 7.0 * b * b + b + 2.0 };
    let stress = if mind.stress.is_finite() {
        mind.stress.max(0.0)
    } else {
        0.0
    };
    let concern = mind.concern;
    let hostility = mind.hostility;
    let alarm = mind.alarm;
    let flee = mind.flee_route
        && ((stress > 3.0 && alarm > 0.75) || (concern > 1.0 && concern > 8.0 - stress));
    if flee {
        return Decision::Flee;
    }
    let heat = concern + hostility;
    if heat > threshold + stress || (heat > threshold && alarm > 1.5) {
        return Decision::Engage;
    }
    if concern > 1.0 {
        return Decision::Watch;
    }
    Decision::Notice
}

/// How noticeable the other is. `gaze` and `forward` are dots of the other's
/// look and facing with the direction toward the observer.
pub fn noticeability(
    closeness: f32,
    gaze: f32,
    forward: f32,
    dead: bool,
    down: bool,
    attacking: bool,
) -> f32 {
    let c = closeness.clamp(0.0, 0.99);
    if dead {
        return 3.0 * c * c;
    }
    if down {
        return 24.0 * c;
    }
    let f = gaze.clamp(-1.0, 1.0);
    let g = forward.clamp(-1.0, 1.0);
    let mut amount = ((f / 2.0 + 0.5).powi(2) / 2.0 + 1.0)
        * (g / 2.0 + 0.5 + 2.0)
        * (2.0 * c.powi(4) + (1.0 - c * c) * c);
    if attacking {
        amount += 10.0;
    }
    amount
}

/// The observer's view of a direction. 1 straight ahead, 0 straight behind.
pub fn view_weight(look_dot: f32, forward_dot: f32) -> f32 {
    let look = look_dot.clamp(-1.0, 1.0);
    let forward = forward_dot.clamp(-1.0, 1.0);
    0.25 * ((1.0 + look) / 2.0).powi(2) + 0.75 * (1.0 + forward) / 2.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settled(attitude: f32) -> Mind {
        let mut mind = Mind::new(attitude);
        for _ in 0..20_000 {
            tick(&mut mind);
        }
        mind
    }

    #[test]
    fn hostility_settles_and_the_choice_follows_it() {
        let mut mind = Mind::new(-100.0);
        tick(&mut mind);
        assert!(
            (mind.hostility - 0.00374625).abs() < 1e-6,
            "{}",
            mind.hostility
        );
        let demon = settled(-100.0);
        assert!(
            (demon.hostility - 3.74625).abs() < 0.01,
            "{}",
            demon.hostility
        );
        assert_eq!(decide(&demon), Decision::Engage);
        let zombie = settled(-20.0);
        assert!(
            (zombie.hostility - 0.74925).abs() < 0.01,
            "{}",
            zombie.hostility
        );
        assert_eq!(decide(&zombie), Decision::Notice);
        let mut wary = settled(-100.0);
        wary.tolerance = 100.0;
        assert_eq!(decide(&wary), Decision::Notice);
        let mut watched = Mind::new(0.0);
        watched.concern = 1.5;
        assert_eq!(decide(&watched), Decision::Watch);
        let mut running = Mind::new(0.0);
        running.stress = 4.0;
        running.alarm = 0.8;
        running.flee_route = true;
        assert_eq!(decide(&running), Decision::Flee);
    }

    #[test]
    fn range_closeness_and_the_view_follow_the_facing() {
        let mut mind = Mind::new(-100.0);
        assert!((mind.closeness() - 0.99).abs() < 1e-6);
        mind.distance_cm = 1200.0;
        assert!(mind.closeness().abs() < 1e-6);
        mind.distance_cm = 1201.0;
        assert_eq!(mind.closeness(), 0.0);
        mind.tracked = true;
        mind.distance_cm = 1800.0;
        assert!(mind.radius() >= 1800.0);
        mind.tracked = false;
        mind.asleep = true;
        mind.distance_cm = 700.0;
        assert_eq!(mind.closeness(), 0.0);
        mind.asleep = false;
        mind.hidden = true;
        mind.light = 0.0;
        assert!((mind.radius() - 120.0).abs() < 1e-3);
        assert!((view_weight(1.0, 1.0) - 1.0).abs() < 1e-6);
        assert!(view_weight(-1.0, -1.0).abs() < 1e-6);
        let side = noticeability(0.5, 0.0, 0.0, false, false, false);
        assert!((side - 1.40625).abs() < 1e-4, "{side}");
        assert!((noticeability(0.5, 0.0, 0.0, true, false, false) - 0.75).abs() < 1e-6);
        assert!((noticeability(0.5, 0.0, 0.0, false, true, false) - 12.0).abs() < 1e-6);
        assert!((noticeability(0.5, 0.0, 0.0, false, false, true) - 11.40625).abs() < 1e-4);
    }
}
