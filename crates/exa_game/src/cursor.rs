//! The world cursor's shape and colour, picked from the player's state as
//! the original picks them; `exa_ui::cursor` draws it. Rules: `docs/UI.md`
//! (World cursor).

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_ui::{CursorShape, WorldCursor};

use crate::bindings::{Action, Bindings};
use crate::hand::Hand;
use crate::player::{
    AIM_HEIGHT, CURSOR_RUN_FROM, ChaseCamera, CursorGround, Player, PlayerBody, window_point,
};

/// The player's state as the cursor sees it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Condition {
    #[default]
    Standing,
    /// Lying after a fall, getting up, or dormant.
    Down,
    /// A lethal wound's collapse, before death.
    Collapsing,
    Dead,
}

impl Condition {
    /// The cursor's view of the player's condition word, by the masks the
    /// original tests: dead, collapsing, down (bit 0x200: down, getting up,
    /// dormant), else standing.
    pub fn of(word: exa_combat::Condition) -> Self {
        if word.is_dead() {
            Condition::Dead
        } else if word.dying() {
            Condition::Collapsing
        } else if word.off_feet() {
            Condition::Down
        } else {
            Condition::Standing
        }
    }
}

/// What the cursor's shape depends on.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CursorState {
    pub combat: bool,
    pub condition: Condition,
    /// The right button walks to the cursor (held over the view): the
    /// horizontal distance to its point (m).
    pub walking_to: Option<f32>,
    /// Ctrl (interact) is held.
    pub interact: bool,
    /// The scene point under the cursor is within reach of the player
    /// (the hand's reach, `crate::hand`).
    pub in_reach: bool,
    /// The left button's press was refused: the cross until its release.
    pub refused: bool,
}

/// The cursor's shape, as the original picks it.
///
/// In combat mode: chevrons while walking to the cursor (two beyond 3 m),
/// else the pointer while Ctrl is held, else the ring, also while down or
/// collapsing; the pointer once dead. Outside it: the cross while down,
/// collapsing or dead, or while a press of the left button is refused,
/// chevrons while walking to the cursor, else the pointer over a point
/// within reach and the cross over one beyond it.
pub fn shape(state: CursorState) -> CursorShape {
    let chevrons = |d: f32| {
        if d > CURSOR_RUN_FROM {
            CursorShape::DoubleChevron
        } else {
            CursorShape::Chevron
        }
    };
    let walking = state
        .walking_to
        .filter(|_| state.condition == Condition::Standing);
    if state.combat {
        match (state.condition, walking) {
            (Condition::Dead, _) => CursorShape::Pointer,
            (Condition::Collapsing, _) => CursorShape::Ring,
            (_, Some(d)) => chevrons(d),
            _ if state.interact => CursorShape::Pointer,
            _ => CursorShape::Ring,
        }
    } else {
        match (state.condition, walking) {
            _ if state.refused => CursorShape::Cross,
            (Condition::Standing, Some(d)) => chevrons(d),
            (Condition::Standing, None) if state.in_reach => CursorShape::Pointer,
            _ => CursorShape::Cross,
        }
    }
}

pub struct CursorPlugin;

impl Plugin for CursorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            pick_cursor
                .after(crate::player::chase_camera)
                .before(exa_ui::DrawWorldCursor),
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn pick_cursor(
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<Bindings>,
    ground: Res<CursorGround>,
    hand: Option<Res<Hand>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<ChaseCamera>>,
    players: Query<(&GlobalTransform, &Children), With<Player>>,
    bodies: Query<(&PlayerBody, Option<&exa_combat::Condition>)>,
    mut cursor: ResMut<WorldCursor>,
) {
    let found = players.iter().find_map(|(at, children)| {
        children
            .iter()
            .find_map(|c| bodies.get(c).ok())
            .map(|body| (at.translation(), body))
    });
    let (Ok(window), Ok((camera, eye)), Some((feet, (pb, word)))) =
        (windows.single(), cameras.single(), found)
    else {
        if cursor.active {
            *cursor = WorldCursor::default();
        }
        return;
    };
    let condition = Condition::of(word.copied().unwrap_or_default());
    let aim = feet + Vec3::Y * AIM_HEIGHT;
    // The chevrons show while the right button walks to the cursor, however
    // near it is, in combat mode too; a released walk finishing its stride
    // shows the other shapes (the original's keys-walking bit).
    let walking_to = ground
        .0
        .map(|c| (c - aim).xz().length())
        .filter(|_| pb.walking_to_cursor());
    let in_reach = !pb.combat && hand.as_ref().is_some_and(|h| !h.out_of_reach);
    let state = CursorState {
        combat: pb.combat,
        condition,
        walking_to,
        interact: bindings.pressed(&keys, Action::Interact),
        in_reach,
        refused: hand.as_ref().is_some_and(|h| h.refused()),
    };
    let next = WorldCursor {
        active: true,
        shape: shape(state),
        combat: pb.combat,
        player: window_point(camera, eye, window, aim),
    };
    if (cursor.active, cursor.shape, cursor.combat) != (next.active, next.shape, next.combat) {
        debug!(
            "world cursor: {:?}, {}",
            next.shape,
            if next.combat { "combat" } else { "exploring" }
        );
    }
    if *cursor != next {
        *cursor = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn combat(condition: Condition) -> CursorState {
        CursorState {
            combat: true,
            condition,
            ..default()
        }
    }

    #[test]
    fn combat_mode_shows_the_ring() {
        assert_eq!(shape(combat(Condition::Standing)), CursorShape::Ring);
        // Also while down or collapsing (seen in the original's fights).
        assert_eq!(shape(combat(Condition::Down)), CursorShape::Ring);
        assert_eq!(shape(combat(Condition::Collapsing)), CursorShape::Ring);
        // After death, the pointer (three deaths in the original).
        assert_eq!(shape(combat(Condition::Dead)), CursorShape::Pointer);
        let ctrl = CursorState {
            interact: true,
            ..combat(Condition::Standing)
        };
        assert_eq!(shape(ctrl), CursorShape::Pointer);
        let ctrl_down = CursorState {
            interact: true,
            ..combat(Condition::Down)
        };
        assert_eq!(shape(ctrl_down), CursorShape::Pointer);
    }

    #[test]
    fn walking_to_the_cursor_shows_chevrons() {
        for combat in [false, true] {
            let at = |d: f32| CursorState {
                combat,
                walking_to: Some(d),
                interact: true,
                in_reach: true,
                ..default()
            };
            // One chevron up to 300 cm, two beyond; also over a point
            // too near to walk to.
            assert_eq!(shape(at(0.2)), CursorShape::Chevron);
            assert_eq!(shape(at(0.5)), CursorShape::Chevron);
            assert_eq!(shape(at(3.0)), CursorShape::Chevron);
            assert_eq!(shape(at(3.01)), CursorShape::DoubleChevron);
            // Not while down.
            let down = CursorState {
                condition: Condition::Down,
                ..at(5.0)
            };
            assert_ne!(shape(down), CursorShape::DoubleChevron);
        }
    }

    #[test]
    fn outside_combat_the_pointer_or_the_cross() {
        let standing = |in_reach| CursorState {
            in_reach,
            ..default()
        };
        assert_eq!(shape(standing(true)), CursorShape::Pointer);
        assert_eq!(shape(standing(false)), CursorShape::Cross);
        for condition in [Condition::Down, Condition::Collapsing, Condition::Dead] {
            let state = CursorState {
                condition,
                in_reach: true,
                ..default()
            };
            assert_eq!(shape(state), CursorShape::Cross, "{condition:?}");
        }
    }

    /// The cursor reads the player's condition word: dead (0xA00), the
    /// collapse (0x800), down for bit 0x200 (down, getting up, dormant),
    /// else standing (attacking, walking, weak, shocked included).
    #[test]
    fn the_condition_word_picks_the_cursors_condition() {
        use exa_combat::Condition as Word;
        for (word, want) in [
            (Word::DEAD, Condition::Dead),
            (Word::COLLAPSING, Condition::Collapsing),
            (Word::DOWN, Condition::Down),
            (Word::GETTING_UP, Condition::Down),
            (Word::DORMANT, Condition::Down),
            (Word::NORMAL, Condition::Standing),
            (Word::ATTACKING, Condition::Standing),
            (Word::WEAK, Condition::Standing),
            (Word::SHOCKED, Condition::Standing),
        ] {
            assert_eq!(Condition::of(word), want, "{word:?}");
        }
    }

    /// A refused press shows the cross outside combat mode, even over a
    /// point within reach and while walking to the cursor.
    #[test]
    fn a_refused_press_shows_the_cross() {
        let refused = CursorState {
            in_reach: true,
            walking_to: Some(1.0),
            refused: true,
            ..default()
        };
        assert_eq!(shape(refused), CursorShape::Cross);
    }

    #[test]
    fn the_aim_plane_is_80_cm_up() {
        assert_eq!(AIM_HEIGHT, 0.8);
    }
}
