//! The fade to black the original's session starts when the player falls
//! to its death: a black layer over the game's view, under the HUD's bars,
//! whose opacity rises from 0 to 255 over 3 s of game time and stays. The
//! game starts it ([`DeathFade`], `exa_game::fall`). Rules: `docs/UI.md`
//! (Death fade).

use bevy::prelude::*;

use crate::LoadingStatus;
use crate::blend::linear_opacity;
use crate::hud::DrawHud;

/// When the fade started (game time, seconds), set by the game.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct DeathFade {
    pub started: Option<f64>,
}

/// How long the fade takes (ms of game time).
pub const FADE_MS: f64 = 3000.0;

/// The black layer's opacity (of 255) `elapsed` ms after the fade started:
/// the share of the 3 s gone (in double precision, then single), at most
/// 1, times 255, rounded half to even.
pub fn opacity(elapsed: f64) -> u8 {
    let t = ((elapsed.max(0.0) * (1.0 / FADE_MS)) as f32).min(1.0);
    (t * 255.0).round_ties_even() as u8
}

#[derive(Component)]
struct FadeNode;

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<DeathFade>()
        .add_systems(Startup, spawn)
        .add_systems(Update, draw.in_set(DrawHud));
}

/// Over the heartbeat's border, under the HUD's bars, as the original
/// draws it between them.
const Z: i32 = 895;

fn spawn(mut commands: Commands) {
    commands.spawn((
        Name::new("DeathFade"),
        FadeNode,
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

fn draw(
    fade: Res<DeathFade>,
    time: Res<Time<Virtual>>,
    loading: Option<Res<LoadingStatus>>,
    mut nodes: Query<(&mut BackgroundColor, &mut Visibility), With<FadeNode>>,
) {
    let Ok((mut colour, mut visibility)) = nodes.single_mut() else {
        return;
    };
    let alpha = fade
        .started
        .filter(|_| loading.is_none())
        .map(|s| opacity((time.elapsed_secs_f64() - s) * 1000.0));
    let Some(alpha) = alpha.filter(|&a| a > 0) else {
        if *visibility != Visibility::Hidden {
            *visibility = Visibility::Hidden;
        }
        return;
    };
    // Black blended in the original's gamma space.
    let next = Color::BLACK.with_alpha(linear_opacity([0.0; 3], f32::from(alpha) / 255.0));
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

    /// 0 at the start, half at 1.5 s, full at 3 s and after.
    #[test]
    fn the_fade_takes_3_seconds() {
        assert_eq!(opacity(0.0), 0);
        assert_eq!(opacity(-5.0), 0);
        assert_eq!(opacity(12.0), 1);
        assert_eq!(opacity(1500.0), 128);
        assert_eq!(opacity(2988.0), 254);
        assert_eq!(opacity(3000.0), 255);
        assert_eq!(opacity(9000.0), 255);
    }
}
