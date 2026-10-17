//! The HUD's values and the low-health heartbeat, taken from the player as
//! the original takes them; `exa_ui::hud` and `exa_ui::heartbeat` draw them.
//! Rules: `docs/UI.md` (Health bars, Heartbeat).

use bevy::audio::{PlaybackMode, Volume};
use bevy::prelude::*;
use exa_assets::Game;
use exa_audio::SoundBank;
use exa_combat::{Fighter, Weakened};
use exa_ui::{Heartbeat, HudBars};

use crate::player::{Player, PlayerBody};

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

/// Whether the low-health check of a blow on the player (yellow left under
/// 0.05, alive: [`Weakened`]) starts the heartbeat, `since` seconds after
/// the last one (None: never started): at most once a minute.
pub fn starts_heartbeat(since: Option<f64>) -> bool {
    since.is_none_or(|s| s > HEARTBEAT_EVERY)
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

/// Starts the heartbeat on a blow whose low-health check falls on the
/// player's body, plays its sound and ends both after 7 s.
#[allow(clippy::too_many_arguments)]
fn heartbeat(
    mut commands: Commands,
    mut weakened: MessageReader<Weakened>,
    player_bodies: Query<(), With<PlayerBody>>,
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
    let now = game_time.elapsed_secs_f64();
    let player_weakened = weakened.read().any(|w| player_bodies.contains(w.fighter));
    if player_weakened && rule.allowed && starts_heartbeat(state.last.map(|t| now - t)) {
        state.last = Some(now);
        beat.started = Some(real_now);
        info!("heartbeat: yellow health {:.3}", hud.yellow);
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_low_blow_starts_the_heartbeat_once_a_minute() {
        assert!(starts_heartbeat(None));
        assert!(starts_heartbeat(Some(60.5)));
        // Not again within the minute.
        assert!(!starts_heartbeat(Some(59.0)));
        assert!(!starts_heartbeat(Some(60.0)));
    }

    /// An app running only the heartbeat, its rule allowing it or not, with
    /// the player's body and another fighter.
    fn beating(allowed: bool) -> (App, Entity, Entity) {
        let mut app = App::new();
        app.add_message::<Weakened>()
            .init_resource::<Time<Virtual>>()
            .init_resource::<Time<Real>>()
            .init_resource::<SoundBank>()
            .init_resource::<Assets<AudioSource>>()
            .init_resource::<Heartbeat>()
            .init_resource::<HeartbeatState>()
            .init_resource::<HudBars>()
            .insert_resource(HeartbeatRule { allowed })
            .add_systems(Update, heartbeat);
        let player = app.world_mut().spawn(PlayerBody::new(0.0)).id();
        let other = app.world_mut().spawn_empty().id();
        (app, player, other)
    }

    fn weaken(app: &mut App, fighter: Entity) -> bool {
        app.world_mut().write_message(Weakened { fighter });
        app.update();
        app.world().resource::<Heartbeat>().started.is_some()
    }

    /// The damage's low-health check on the player's body starts the
    /// heartbeat where it is allowed, whatever yellow the HUD shows (a blow
    /// that deals nothing to a player already under 0.05 too); on another
    /// fighter, or in the arena, nothing starts.
    #[test]
    fn the_players_low_health_check_starts_the_heartbeat() {
        let (mut app, player, other) = beating(true);
        assert!(!weaken(&mut app, other));
        assert!(weaken(&mut app, player));
        let (mut arena, player, _) = beating(false);
        assert!(!weaken(&mut arena, player));
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
