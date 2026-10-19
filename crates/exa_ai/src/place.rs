//! Where a fighter wants to stand against another, from the combat
//! position score.
//!
//! The close term uses the other fighter's reach plus 150 cm. Inside that
//! limit, with `t` the fraction still left, it is `(2 − t) · t · 4`.
//! Outside a fight it doubles, and the strike term is 0. In a fight the
//! strike term uses the observer's reach and step: the square of distance
//! over the ideal while closer than the ideal, 1 until one step past it,
//! then a fade that is 0 at 600 cm. A large opponent's ideal is 50 cm
//! longer. Wanting melee doubles the strike term.

/// The strike term is 0 at this distance (cm).
pub const PLACE_FADE_CM: f32 = 600.0;
/// Added to the other fighter's reach for the close term (cm).
const CLOSE_PAST: f32 = 150.0;
/// Centimetres of strike ideal per unit of the opponent's size past 1.
const SIZE_CM: f32 = 10.0;
/// Extra strike ideal when the opponent is large (cm).
const LARGE_CM: f32 = 50.0;

/// Who the score is measured against.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    /// The other fighter's weapon reach (cm).
    pub other_reach_cm: f32,
    /// The observer's weapon reach (cm).
    pub my_reach_cm: f32,
    /// The other fighter's body size.
    pub other_size: f32,
    /// The observer's step (cm).
    pub my_step_cm: f32,
    /// The other fighter is large.
    pub other_large: bool,
    /// The observer wants melee, which doubles the strike term.
    pub melee: bool,
    /// A fight score. Otherwise the strike term is cleared and the close
    /// term doubles.
    pub fight: bool,
}

/// The two distance terms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaceScore {
    pub close: f32,
    pub strike: f32,
}

fn ideal(spot: Spot) -> f32 {
    let mut ideal = spot.my_reach_cm + (spot.other_size - 1.0) * SIZE_CM;
    if spot.other_large {
        ideal += LARGE_CM;
    }
    ideal
}

/// Score a candidate `distance_cm` from the other fighter.
pub fn score_place(distance_cm: f32, spot: Spot) -> PlaceScore {
    let limit = spot.other_reach_cm + CLOSE_PAST;
    let mut close = if distance_cm <= limit {
        let t = (limit - distance_cm) / limit;
        (2.0 - t) * t * 4.0
    } else {
        0.0
    };
    let ideal = ideal(spot);
    let mut strike = if distance_cm < ideal {
        let ratio = distance_cm / ideal;
        ratio * ratio
    } else {
        let end = ideal + spot.my_step_cm;
        if distance_cm < end {
            1.0
        } else {
            1.0 - (distance_cm - end) / (PLACE_FADE_CM - end)
        }
    };
    if spot.melee {
        strike *= 2.0;
    }
    if !spot.fight {
        close *= 2.0;
        strike = 0.0;
    }
    PlaceScore { close, strike }
}

/// The candidate is past the strike plateau, so the fighter keeps closing.
pub fn still_closing(distance_cm: f32, spot: Spot) -> bool {
    let score = score_place(distance_cm, spot);
    distance_cm > ideal(spot) + spot.my_step_cm && score.strike < 1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fight(large: bool, melee: bool) -> Spot {
        Spot {
            other_reach_cm: 50.0,
            my_reach_cm: 50.0,
            other_size: 1.0,
            my_step_cm: 80.0,
            other_large: large,
            melee,
            fight: true,
        }
    }

    #[test]
    fn a_fight_stands_on_the_plateau_and_closes_past_it() {
        let spot = fight(false, false);
        let near = score_place(25.0, spot);
        assert_eq!(near.strike, 0.25);
        let plateau = score_place(120.0, spot);
        assert_eq!(plateau.strike, 1.0);
        assert!(!still_closing(120.0, spot));
        let edge = score_place(130.0, spot);
        assert_eq!(edge.strike, 1.0);
        assert!(!still_closing(130.0, spot));
        let far = score_place(600.0, spot);
        assert_eq!(far.strike, 0.0);
        assert!(still_closing(600.0, spot));
        let limit = 50.0 + 150.0;
        let t = (limit - 25.0) / limit;
        assert_eq!(near.close, (2.0 - t) * t * 4.0);
        assert_eq!(score_place(limit + 1.0, spot).close, 0.0);
        let doubled = score_place(
            25.0,
            Spot {
                fight: false,
                ..spot
            },
        );
        assert_eq!(doubled.strike, 0.0);
        assert_eq!(doubled.close, near.close * 2.0);
        let big = score_place(100.0, fight(true, false));
        assert_eq!(big.strike, 1.0);
        let eager = score_place(25.0, fight(false, true));
        assert_eq!(eager.strike, 0.5);
    }
}
