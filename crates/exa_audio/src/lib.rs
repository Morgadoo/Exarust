//! Audio from `Sound.rpk` (RIFF/WAVE PCM): music and ambience per level,
//! weapon impacts and pain sounds on hits, footsteps for moving characters.
//! The original mixes through OpenAL Soft with EFX reverb; reverb zones come
//! later (`docs/PLAN.md`, Phase 5).

use std::collections::HashMap;
use std::sync::Arc;

use bevy::audio::{PlaybackMode, Volume};
use bevy::prelude::*;
use exa_assets::{Game, PackKind};
use exa_combat::Hit;
use exa_physics::CharacterBody;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SoundBank>()
            .add_systems(Update, (hit_sounds, footsteps, start_music));
    }
}

/// Decoded sounds by name.
#[derive(Resource, Default)]
pub struct SoundBank {
    loaded: HashMap<String, Option<Handle<AudioSource>>>,
    seed: u64,
}

impl SoundBank {
    pub fn get(
        &mut self,
        game: &Game,
        sources: &mut Assets<AudioSource>,
        name: &str,
    ) -> Option<Handle<AudioSource>> {
        let key = name.to_ascii_lowercase();
        if let Some(h) = self.loaded.get(&key) {
            return h.clone();
        }
        let handle = game
            .0
            .pack(PackKind::Sound)
            .and_then(|p| p.get(&key))
            .map(|bytes| {
                sources.add(AudioSource {
                    bytes: Arc::from(bytes),
                })
            });
        if handle.is_none() {
            warn!("sound {name:?} not found");
        }
        self.loaded.insert(key, handle.clone());
        handle
    }

    /// A pseudo-random index in `0..n` (xorshift; audio variety only).
    pub fn pick(&mut self, n: usize) -> usize {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        if self.seed == 0 {
            self.seed = 0x9E37_79B9_7F4A_7C15;
        }
        (self.seed % n.max(1) as u64) as usize
    }
}

/// The looping music or ambience to play (set by the level loader).
#[derive(Resource, Clone, Debug)]
pub struct LevelMusic(pub String);

/// Music for a level name, following the original's track naming.
pub fn music_for_level(level: &str) -> &'static str {
    let l = level.to_ascii_lowercase();
    if l.starts_with("arenahub") {
        "msc_hub01"
    } else if l.starts_with("arenao") {
        "msc_arenao"
    } else if l.starts_with("arena") {
        "msc_arena3"
    } else if l.starts_with("exanima0") || l.starts_with("exanimac") {
        "msc_uw01amb"
    } else {
        "ambntra01"
    }
}

#[derive(Component)]
struct MusicPlayer;

fn start_music(
    mut commands: Commands,
    game: Option<Res<Game>>,
    music: Option<Res<LevelMusic>>,
    mut bank: ResMut<SoundBank>,
    mut sources: ResMut<Assets<AudioSource>>,
    playing: Query<Entity, With<MusicPlayer>>,
) {
    let (Some(game), Some(music)) = (game, music) else {
        return;
    };
    if !music.is_changed() {
        return;
    }
    for e in &playing {
        commands.entity(e).despawn();
    }
    if let Some(h) = bank.get(&game, &mut sources, &music.0) {
        commands.spawn((
            AudioPlayer(h),
            PlaybackSettings {
                mode: PlaybackMode::Loop,
                volume: Volume::Linear(0.45),
                ..default()
            },
            MusicPlayer,
        ));
    }
}

fn hit_sounds(
    mut commands: Commands,
    game: Option<Res<Game>>,
    mut hits: MessageReader<Hit>,
    mut bank: ResMut<SoundBank>,
    mut sources: ResMut<Assets<AudioSource>>,
) {
    let Some(game) = game else { return };
    for hit in hits.read() {
        let impact = format!("wh_sworda{:02}", 1 + bank.pick(6));
        let voice = format!("vhm{:02}_grunts{:02}", 1 + bank.pick(9), 1 + bank.pick(6));
        for (name, volume) in [(impact, 1.0), (voice, 0.8)] {
            if let Some(h) = bank.get(&game, &mut sources, &name) {
                commands.spawn((
                    AudioPlayer(h),
                    PlaybackSettings::DESPAWN
                        .with_spatial(true)
                        .with_volume(Volume::Linear(volume)),
                    Transform::from_translation(hit.point),
                ));
            }
        }
    }
}

#[derive(Component, Default)]
struct StepClock(f32);

fn footsteps(
    mut commands: Commands,
    time: Res<Time>,
    game: Option<Res<Game>>,
    mut bank: ResMut<SoundBank>,
    mut sources: ResMut<Assets<AudioSource>>,
    mut walkers: Query<(
        Entity,
        &CharacterBody,
        &GlobalTransform,
        Option<&mut StepClock>,
    )>,
) {
    let Some(game) = game else { return };
    for (e, body, gt, clock) in &mut walkers {
        let Some(mut clock) = clock else {
            commands.entity(e).insert(StepClock::default());
            continue;
        };
        let speed = body.desired_velocity.length();
        if speed < 0.2 || !body.grounded {
            clock.0 = 0.0;
            continue;
        }
        clock.0 -= time.delta_secs();
        if clock.0 > 0.0 {
            continue;
        }
        // About two steps per metre.
        clock.0 = (0.75 / speed).clamp(0.25, 0.7);
        let name = if speed > 2.5 {
            format!("stphvy{:02}", 1 + bank.pick(6))
        } else {
            format!("stpmed{:02}", 1 + bank.pick(6))
        };
        if let Some(h) = bank.get(&game, &mut sources, &name) {
            commands.spawn((
                AudioPlayer(h),
                PlaybackSettings::DESPAWN
                    .with_spatial(true)
                    .with_volume(Volume::Linear(0.6)),
                Transform::from_translation(gt.translation()),
            ));
        }
    }
}
