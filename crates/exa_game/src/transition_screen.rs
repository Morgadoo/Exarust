//! The transition screen in the running game: the black form the game flow
//! opens over a level change, a death, a respawn, a revival or the return
//! to the title.
//!
//! The screen's state machine is [`exa_ui::transition::TransitionScreen`]'s
//! and the flow runs it ([`crate::flow::Flow::transition_tick`]): it fades
//! to black, runs the flow's action there, holds two frames and the hold
//! time, and fades back in. This module draws its alpha byte as the form's
//! black background, over the game's view, the HUD and the GUI forms and
//! under the mouse cursor (the original raises the form above the others
//! when it opens).
//!
//! Rules: `docs/UI.md` (The transition screen).

use bevy::prelude::*;
use exa_ui::blend::linear_opacity;

/// The transition screen's alpha byte this frame; 0 when no transition is up.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TransitionOverlay(pub u8);

#[derive(Component)]
struct TransitionNode;

/// Over the HUD (900), the death fade (895) and the splash (950); under the
/// cursor (1000).
const Z: i32 = 960;

/// Draws the transition screen's black form.
pub struct TransitionScreenPlugin;

impl Plugin for TransitionScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TransitionOverlay>()
            .add_systems(Startup, spawn)
            .add_systems(Update, draw);
    }
}

fn spawn(mut commands: Commands) {
    commands.spawn((
        Name::new("TransitionScreen"),
        TransitionNode,
        Node {
            position_type: PositionType::Absolute,
            left: px(0.0),
            top: px(0.0),
            width: percent(100.0),
            height: percent(100.0),
            ..default()
        },
        BackgroundColor(Color::NONE),
        GlobalZIndex(Z),
        Visibility::Hidden,
    ));
}

/// The form's colour for an alpha byte: the original clears it to opaque
/// black and draws it at the screen's alpha, blended in its gamma space.
pub fn overlay_colour(alpha: u8) -> Color {
    Color::BLACK.with_alpha(linear_opacity([0.0; 3], f32::from(alpha) / 255.0))
}

fn draw(
    overlay: Res<TransitionOverlay>,
    mut nodes: Query<(&mut BackgroundColor, &mut Visibility), With<TransitionNode>>,
) {
    let Ok((mut colour, mut visibility)) = nodes.single_mut() else {
        return;
    };
    if overlay.0 == 0 {
        if *visibility != Visibility::Hidden {
            *visibility = Visibility::Hidden;
        }
        return;
    }
    let next = overlay_colour(overlay.0);
    if colour.0 != next {
        colour.0 = next;
    }
    if *visibility != Visibility::Inherited {
        *visibility = Visibility::Inherited;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Black at the byte's opacity: clear at 0, opaque at 255.
    #[test]
    fn the_form_is_black_at_the_screens_alpha() {
        assert_eq!(overlay_colour(0).alpha(), 0.0);
        assert_eq!(overlay_colour(255).alpha(), 1.0);
        let half = overlay_colour(128).alpha();
        assert!(half > 0.0 && half < 1.0);
        let srgb = overlay_colour(200).to_srgba();
        assert_eq!((srgb.red, srgb.green, srgb.blue), (0.0, 0.0, 0.0));
    }
}
