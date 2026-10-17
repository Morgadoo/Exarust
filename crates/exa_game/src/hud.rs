//! The HUD's values and the low-health heartbeat, taken from the player as
//! the original takes them; `exa_ui::hud` and `exa_ui::heartbeat` draw them.
//! Rules: `docs/UI.md` (Health bars, Heartbeat).

use bevy::audio::{PlaybackMode, Volume};
use bevy::prelude::*;
use exa_assets::Game;
use exa_audio::SoundBank;
use exa_combat::Fighter;
use exa_combat::fighter::DEATH;
use exa_ui::{Heartbeat, HudBars};

use crate::player::{Player, PlayerBody};

/// A blow that leaves the player's yellow health below this (and alive)
/// starts the heartbeat.
pub const LOW_HEALTH: f32 = 0.05;
/// The heartbeat starts at most once in this long (s of game time).
pub const HEARTBEAT_EVERY: f64 = 60.0;
/// How long the heartbeat lasts (s of real time).
const HEARTBEAT_FOR: f64 = exa_ui::heartbeat::HEARTBEAT_MS as f64 / 1000.0;
/// The heartbeat's sound, looping at this gain until it ends.
pub const HEARTBEAT_SOUND: &str = "heartbeat";
const HEARTBEAT_GAIN: f32 = 0.3;

/// Whether the low-health heartbeat may start: the original's story mode
/// only, never in practice or the arena.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HeartbeatRule {
    pub allowed: bool,
}

/// Whether a level is played in the original's story mode, where the
/// heartbeat plays: every level but the arenas.
pub fn heartbeat_allowed(level: &str) -> bool {
    !level.to_ascii_lowercase().starts_with("arena")
}

/// Whether a blow that took the player's yellow health from `before` to
/// `after` starts the heartbeat, `since` seconds after the last one (None:
/// never started).
pub fn starts_heartbeat(before: f32, after: f32, since: Option<f64>) -> bool {
    after < before
        && (DEATH..LOW_HEALTH).contains(&after)
        && since.is_none_or(|s| s > HEARTBEAT_EVERY)
}

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HeartbeatRule>()
            .init_resource::<HeartbeatState>()
            .add_systems(
                Update,
                (pick_hud, heartbeat)
                    .chain()
                    .after(crate::player::chase_camera)
                    .before(exa_ui::DrawHud),
            );
    }
}

/// Values the heartbeat needs from frame to frame.
#[derive(Resource, Default)]
struct HeartbeatState {
    /// The player's yellow health last frame.
    yellow: Option<f32>,
    /// When the last heartbeat started (s of game time).
    last: Option<f64>,
    sound: Option<Entity>,
}

#[derive(Component)]
struct HeartbeatSound;

/// The player's health, focus and combat mode for the HUD. Exarust has no
/// powers and no shields yet: focus stays full, as the original's for a
/// character who casts nothing, and the stamina gauge never shows.
fn pick_hud(
    players: Query<&Children, With<Player>>,
    bodies: Query<(&PlayerBody, Option<&Fighter>)>,
    mut hud: ResMut<HudBars>,
) {
    let found = players
        .iter()
        .find_map(|children| children.iter().find_map(|c| bodies.get(c).ok()));
    let next = match found {
        Some((body, fighter)) => {
            let health = fighter
                .map(|f| f.health)
                .unwrap_or_else(|| exa_combat::Health::of_race(1.0));
            HudBars {
                active: true,
                combat: body.combat,
                yellow: health.yellow,
                red: health.red,
                max: health.max,
                ..default()
            }
        }
        None => HudBars::default(),
    };
    if *hud != next {
        if next.active && (hud.yellow, hud.red) != (next.yellow, next.red) {
            debug!(
                "player health: yellow {:.3}, red {:.3} of {:.3}",
                next.yellow, next.red, next.max
            );
        }
        *hud = next;
    }
}

/// Starts the heartbeat on a blow that leaves the player's yellow health
/// low, plays its sound and ends both after 7 s.
#[allow(clippy::too_many_arguments)]
fn heartbeat(
    mut commands: Commands,
    hud: Res<HudBars>,
    rule: Res<HeartbeatRule>,
    game_time: Res<Time<Virtual>>,
    real_time: Res<Time<Real>>,
    game: Option<Res<Game>>,
    mut bank: ResMut<SoundBank>,
    mut sources: ResMut<Assets<AudioSource>>,
    mut beat: ResMut<Heartbeat>,
    mut state: ResMut<HeartbeatState>,
) {
    let real_now = real_time.elapsed_secs_f64();
    if let Some(started) = beat.started
        && real_now - started > HEARTBEAT_FOR
    {
        beat.started = None;
        if let Some(sound) = state.sound.take() {
            commands.entity(sound).despawn();
        }
        debug!("heartbeat ends");
    }
    let yellow = hud.active.then_some(hud.yellow);
    let now = game_time.elapsed_secs_f64();
    if let (Some(before), Some(after), true) = (state.yellow, yellow, rule.allowed)
        && starts_heartbeat(before, after, state.last.map(|t| now - t))
    {
        state.last = Some(now);
        beat.started = Some(real_now);
        info!("heartbeat: yellow health {after:.3}");
        if let Some(sound) = state.sound.take() {
            commands.entity(sound).despawn();
        }
        if let Some(game) = game
            && let Some(clip) = bank.get(&game, &mut sources, HEARTBEAT_SOUND)
        {
            state.sound = Some(
                commands
                    .spawn((
                        HeartbeatSound,
                        AudioPlayer(clip),
                        PlaybackSettings {
                            mode: PlaybackMode::Loop,
                            volume: Volume::Linear(HEARTBEAT_GAIN),
                            ..default()
                        },
                    ))
                    .id(),
            );
        }
    }
    state.yellow = yellow;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_low_blow_starts_the_heartbeat_once_a_minute() {
        // A blow from 0.08 to 0.04: below 0.05 and alive.
        assert!(starts_heartbeat(0.08, 0.04, None));
        assert!(starts_heartbeat(0.04, 0.02, Some(60.5)));
        // Not again within the minute.
        assert!(!starts_heartbeat(0.04, 0.02, Some(59.0)));
        // Not above 0.05, not a killing blow, not without a blow.
        assert!(!starts_heartbeat(0.2, 0.05, None));
        assert!(!starts_heartbeat(0.04, 0.0, None));
        assert!(!starts_heartbeat(0.04, 0.009, None));
        assert!(!starts_heartbeat(0.04, 0.04, None));
        assert!(starts_heartbeat(0.04, 0.01, None));
    }

    #[test]
    fn the_heartbeat_plays_in_story_levels_only() {
        assert!(heartbeat_allowed("exanima01"));
        assert!(heartbeat_allowed("Exanima05"));
        assert!(!heartbeat_allowed("arena"));
        assert!(!heartbeat_allowed("arenae"));
        assert!(!heartbeat_allowed("ArenaHub"));
    }
}
