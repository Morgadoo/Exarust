//! The HUD's values and the low-health heartbeat, taken from the player as
//! the original takes them; `exa_ui::hud` and `exa_ui::heartbeat` draw them.
//! Rules: `docs/UI.md` (Health bars, Heartbeat).

use bevy::audio::{AudioSink, AudioSinkPlayback, PlaybackMode, Volume};
use bevy::prelude::*;
use exa_assets::Game;
use exa_audio::SoundBank;
use exa_combat::block::ShieldHold;
use exa_combat::{Fighter, Weakened};
use exa_items::catalogue::ItemCatalogue;
use exa_items::inventory_window::carries_compass;
use exa_items::possession::Possessions;
use exa_ui::hud_status::StatusView;
use exa_ui::{CompassView, Heartbeat, HudBars};

use crate::player::{ChaseCamera, Player, PlayerBody};
use crate::thaum::cast::PowerCaster;

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
                (pick_hud, pick_compass, pick_status, heartbeat)
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

/// The player's health, focus and combat mode for the HUD: the focus and
/// the capacity it regenerates to are the caster's, full for a body that
/// casts nothing. A shield in the player's off hand ([`ShieldHold`], the
/// original's shield pointer on the character) shows the block-stamina
/// gauge in combat mode, filled to its arm's stamina. The power bar's flag
/// is the original's: set when the bar opens, cleared when its closing
/// animation has run ([`power_bar_up`]).
fn pick_hud(
    players: Query<&Children, With<Player>>,
    bodies: Query<(
        &PlayerBody,
        Option<&Fighter>,
        Option<&PowerCaster>,
        Option<&ShieldHold>,
    )>,
    view: Option<Res<crate::flow::GameView>>,
    tree: Option<Res<crate::power_screen::PowerScreen>>,
    mut hud: ResMut<HudBars>,
) {
    if view.is_some_and(|view| !view.0) {
        if hud.active {
            *hud = HudBars::default();
        }
        return;
    }
    let found = players
        .iter()
        .find_map(|children| children.iter().find_map(|c| bodies.get(c).ok()));
    let next = match found {
        Some((body, fighter, caster, shield)) => {
            let health = fighter
                .map(|f| f.health)
                .unwrap_or_else(|| exa_combat::Health::of_race(1.0));
            let (focus, focus_cap) = caster.map_or((1.0, 1.0), |caster| {
                (caster.body.focus, caster.body.capacity)
            });
            HudBars {
                active: true,
                combat: body.combat,
                yellow: health.yellow,
                red: health.red,
                max: health.max,
                focus,
                focus_cap,
                shield: shield.is_some(),
                stamina: shield.map_or(1.0, |hold| hold.guard.stamina),
                power_bar: tree.as_deref().is_some_and(power_bar_up),
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

/// How long a healing charge's step and a quick-bar cast hold the bars
/// shown (ms).
pub const BARS_HOLD_MS: i64 = 1000;

/// The original's bars hold on the frame's world: the bars stay shown for
/// `ms` of game time from now ([`exa_ui::hud::HudShowUntil::show_for`]).
pub fn hold_bars(world: &mut World, ms: i64) {
    let Some(now) = world
        .get_resource::<Time<Virtual>>()
        .map(|time| time.elapsed().as_millis() as i64)
    else {
        return;
    };
    if let Some(mut until) = world.get_resource_mut::<exa_ui::hud::HudShowUntil>() {
        until.show_for(now, ms);
    }
}

/// The original's power-bar flag: set by the bar's opening toggle and
/// cleared by the bar draw once its 900 ms closing has run, so it stays up
/// while the bar is open or still closing.
pub fn power_bar_up(screen: &crate::power_screen::PowerScreen) -> bool {
    screen.bar_open || screen.tree.bar_flags != 0
}

/// The compass's visibility and turn: the player's own pack carrying a
/// usable item of the compass's kind (4, 11) shows it, and the chase
/// camera's backward axis (the third row of its frame, as the original
/// reads it) turns it. The interface fade is the power tree's window fade
/// (its opening rise, held at 1 while open, its 256 ms closing fall), which
/// the original's tree draw leaves in the GUI's fade word: 0 before the
/// tree first opens and once it has faded out, so the compass hides while
/// the tree is up.
fn pick_compass(
    players: Query<&Children, With<Player>>,
    bodies: Query<&Possessions>,
    catalogue: Option<Res<ItemCatalogue>>,
    cameras: Query<&Transform, With<ChaseCamera>>,
    hud: Res<HudBars>,
    tree: Option<Res<crate::power_screen::PowerScreen>>,
    mut compass: ResMut<CompassView>,
) {
    let carried = catalogue.as_ref().is_some_and(|catalogue| {
        players.iter().any(|children| {
            children
                .iter()
                .filter_map(|c| bodies.get(c).ok())
                .any(|possessions| carries_compass(possessions, |id| catalogue.usable(id)))
        })
    });
    let back = cameras
        .iter()
        .next()
        .map_or([0.0, 1.0], |t| [t.back().x, t.back().z]);
    let next = CompassView {
        visible: hud.active && carried,
        back,
        fade: tree
            .as_deref()
            .filter(|screen| screen.open)
            .map_or(0.0, |screen| screen.tree.window_alpha),
    };
    if *compass != next {
        *compass = next;
    }
}

/// The HUD's power marker and skill-ready alert: the HUD is always on in
/// Exarust (it has no switch for it), the player is the HUD's, and the
/// marker shows while the player's caster knows powers and none of them is
/// a new pick ([`exa_ui::game_gui::status_marker`] over the known pairs'
/// flags). The alert reads the player's record as the original's status
/// routine reads its player record: it shows while the six skills' levels
/// sum below fifteen and none holds a technique ahead of its level
/// ([`exa_ui::game_gui::ready_alert`]). A player body without a record
/// (a viewer scene) has no alert.
fn pick_status(
    players: Query<&Children, With<Player>>,
    casters: Query<&PowerCaster, With<PlayerBody>>,
    records: Query<&exa_character::record::CharacterData, With<PlayerBody>>,
    hud: Res<HudBars>,
    mut status: ResMut<StatusView>,
) {
    let flags: Vec<u32> = players
        .iter()
        .flat_map(|children| children.iter())
        .find_map(|child| casters.get(child).ok())
        .map(|caster| caster.record.pairs.iter().map(|pair| pair.flags).collect())
        .unwrap_or_default();
    let next = StatusView {
        enabled: true,
        player: hud.active,
        marker: exa_ui::game_gui::status_marker(&flags),
        ready: players
            .iter()
            .flat_map(|children| children.iter())
            .find_map(|child| records.get(child).ok())
            .is_some_and(|data| exa_ui::game_gui::ready_alert(&data.record.skills)),
    };
    if *status != next {
        *status = next;
    }
}

/// Starts the heartbeat on a blow whose low-health check falls on the
/// player's body, plays its sound and ends both after 7 s, or when its
/// 1 s fade-out ([`Heartbeat::start_fade`]) has run.
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
    mut sinks: Query<&mut AudioSink, With<HeartbeatSound>>,
) {
    let real_now = real_time.elapsed_secs_f64();
    if let Some(started) = beat.started
        && real_now - started > HEARTBEAT_FOR
    {
        beat.stop();
        if let Some(sound) = state.sound.take() {
            commands.entity(sound).despawn();
        }
        debug!("heartbeat ends");
    }
    // The fade-out's arm of the original's border draw: the sound's gain
    // falls from 0.3 over 1 s, then the heartbeat stops.
    match beat.fade_gain(real_now) {
        Some(Some(gain)) => {
            if let Some(mut sink) = state.sound.and_then(|sound| sinks.get_mut(sound).ok()) {
                sink.set_volume(Volume::Linear(gain));
            }
        }
        Some(None) => {
            beat.stop();
            if let Some(sound) = state.sound.take() {
                commands.entity(sound).despawn();
            }
            debug!("heartbeat faded out");
        }
        None => {}
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

    /// A started fade-out lowers nothing at once and stops the heartbeat
    /// (border and sound) once its second has run, before the 7 s end.
    #[test]
    fn the_heartbeat_fades_out_over_a_second_and_stops() {
        let (mut app, player, _) = beating(true);
        assert!(weaken(&mut app, player));
        let started = app.world().resource::<Heartbeat>().started.unwrap();
        assert!(
            app.world_mut()
                .resource_mut::<Heartbeat>()
                .start_fade(started)
        );
        let gain = app.world().resource::<Heartbeat>().fade_gain(started + 0.5);
        assert!((gain.unwrap().unwrap() - 0.15).abs() < 1e-6);
        // A second and a half on the fade has ended (the real clock's first
        // update only sets its start).
        for ms in [0, 1500] {
            app.world_mut()
                .resource_mut::<Time<Real>>()
                .update_with_duration(std::time::Duration::from_millis(ms));
        }
        assert!(app.world().resource::<Time<Real>>().elapsed_secs_f64() >= 1.5);
        app.update();
        let beat = *app.world().resource::<Heartbeat>();
        assert_eq!((beat.started, beat.fading), (None, None));
        // A heartbeat that is not running takes no fade.
        assert!(!app.world_mut().resource_mut::<Heartbeat>().start_fade(5.0));
    }

    /// The player's off-hand shield feeds the block-stamina gauge (its
    /// arm's stamina), and the power bar's flag stays up while the bar is
    /// open or still closing.
    #[test]
    fn the_hud_takes_the_shield_and_the_power_bar() {
        let run = |shield: Option<f32>, bar: Option<(bool, u8)>| {
            let mut app = App::new();
            app.init_resource::<HudBars>().add_systems(Update, pick_hud);
            let mut body = app
                .world_mut()
                .spawn((PlayerBody::new(0.0), Fighter::default()));
            if let Some(stamina) = shield {
                let item = Entity::PLACEHOLDER;
                body.insert(ShieldHold {
                    item,
                    guard: exa_combat::damage::ShieldGuard {
                        stamina,
                        ..default()
                    },
                });
            }
            let body = body.id();
            app.world_mut().spawn(Player).add_child(body);
            if let Some((open, flags)) = bar {
                let mut screen = crate::power_screen::PowerScreen::default();
                screen.bar_open = open;
                screen.tree.bar_flags = flags;
                app.insert_resource(screen);
            }
            app.update();
            *app.world().resource::<HudBars>()
        };
        let bare = run(None, None);
        assert!(bare.active && !bare.shield && !bare.power_bar);
        let guarded = run(Some(0.4), None);
        assert!(guarded.shield);
        assert_eq!(guarded.stamina, 0.4);
        assert!(run(None, Some((true, 0))).power_bar);
        // Put away, still closing: up; closed to the end: down.
        assert!(run(None, Some((false, 1))).power_bar);
        assert!(!run(None, Some((false, 0))).power_bar);
    }

    /// The bars hold: a second of game time from now.
    #[test]
    fn a_hold_shows_the_bars_for_a_second_of_game_time() {
        let mut world = World::new();
        world.init_resource::<exa_ui::hud::HudShowUntil>();
        let mut time = Time::<Virtual>::default();
        time.advance_by(std::time::Duration::from_millis(2500));
        world.insert_resource(time);
        hold_bars(&mut world, BARS_HOLD_MS);
        assert_eq!(
            *world.resource::<exa_ui::hud::HudShowUntil>(),
            exa_ui::hud::HudShowUntil(3500)
        );
    }

    /// The compass's fade follows the open power tree's window fade and is
    /// 0 with the tree shut.
    #[test]
    fn the_compass_fades_with_the_power_tree_window() {
        let run = |open: Option<(bool, f32)>| {
            let mut app = App::new();
            app.init_resource::<HudBars>()
                .init_resource::<CompassView>()
                .add_systems(Update, pick_compass);
            if let Some((open, window)) = open {
                let mut screen = crate::power_screen::PowerScreen::default();
                screen.open = open;
                screen.tree.window_alpha = window;
                app.insert_resource(screen);
            }
            app.update();
            app.world().resource::<CompassView>().fade
        };
        assert_eq!(run(None), 0.0);
        assert_eq!(run(Some((false, 1.0))), 0.0);
        assert_eq!(run(Some((true, 0.25))), 0.25);
        assert_eq!(run(Some((true, 1.0))), 1.0);
    }

    /// The power marker shows while the player's caster knows powers and
    /// none is a new pick; without a record the alert stays off.
    #[test]
    fn the_power_marker_reads_the_players_known_powers() {
        let run = |powers: &[u32], new_pick: bool| {
            let mut app = App::new();
            app.init_resource::<HudBars>()
                .init_resource::<StatusView>()
                .add_systems(Update, (pick_hud, pick_status).chain());
            let mut caster = PowerCaster::new(powers);
            if new_pick && let Some(pair) = caster.record.pairs.first_mut() {
                pair.flags |= crate::thaum::tree::NEW;
            }
            let body = app.world_mut().spawn((PlayerBody::new(0.0), caster)).id();
            app.world_mut().spawn(Player).add_children(&[body]);
            app.update();
            *app.world().resource::<StatusView>()
        };
        let known = run(&[0x201], false);
        assert_eq!(
            known,
            StatusView {
                enabled: true,
                player: true,
                marker: true,
                ready: false,
            }
        );
        assert!(!run(&[], false).marker);
        assert!(!run(&[0x201], true).marker);
    }

    /// The skill-ready alert reads the player's record through the live
    /// status pick: on with levels to spend, off at fifteen levels and off
    /// while a skill holds a technique ahead of its level.
    #[test]
    fn the_skill_ready_alert_reads_the_players_record() {
        use exa_character::record::CharacterData;
        let run = |data: Option<CharacterData>| {
            let mut app = App::new();
            app.init_resource::<HudBars>()
                .init_resource::<StatusView>()
                .add_systems(Update, (pick_hud, pick_status).chain());
            let body = app
                .world_mut()
                .spawn((PlayerBody::new(0.0), PowerCaster::new(&[])))
                .id();
            if let Some(data) = data {
                app.world_mut().entity_mut(body).insert(data);
            }
            app.world_mut().spawn(Player).add_children(&[body]);
            app.update();
            app.world().resource::<StatusView>().ready
        };
        assert!(!run(None));
        let fresh = CharacterData::new();
        assert!(run(Some(fresh.clone())));
        let mut capped = fresh.clone();
        for skill in capped.record.skills.iter_mut().take(3) {
            skill.points = 5 * 5000;
        }
        assert_eq!(capped.total_levels(), 15);
        assert!(!run(Some(capped)));
        let mut ahead = fresh;
        ahead.record.skills[0].techniques[0] = 1;
        assert!(!run(Some(ahead)));
    }

    /// The focus bar shows the player's caster: its pool and its capacity.
    #[test]
    fn the_focus_bar_reads_the_players_caster() {
        let mut app = App::new();
        app.init_resource::<HudBars>().add_systems(Update, pick_hud);
        let mut caster = PowerCaster::new(&[]);
        caster.body.focus = 0.4;
        caster.body.capacity = 0.75;
        let body = app.world_mut().spawn((PlayerBody::new(0.0), caster)).id();
        app.world_mut().spawn(Player).add_children(&[body]);
        app.update();
        let hud = *app.world().resource::<HudBars>();
        assert!(hud.active);
        assert_eq!((hud.focus, hud.focus_cap), (0.4, 0.75));
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
