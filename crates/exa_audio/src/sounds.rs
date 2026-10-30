//! The named sound sets of the body and its blows: the swing voice (with
//! its clothing rustle), the second wind's heal, the hit-sound classes of armour and flesh, and the
//! clothing and golem foley loops.
//!
//! Evidence: the part's sound set names (`SwingHA`, `StmHeal01`, `HitGlm`,
//! `FoleyPlH`, `Golem_Foley`) and the pack's numbered variants: `swingha`
//! 6, `stmheal` 1, `bdycol` 6, `skelhit` 6, `hitclt` 1, `hitltr` 4,
//! `hitchn` 3, `hitplt` 8, `hithbr` 6, `hitglm` 6, `foleyclt` 6, `foleyltr`
//! 6, `foleychn` 6, `foleyplt` 7, `foleyhbr` 6, `foleyplh` 9,
//! `golem_foley` 6. The character class's first setup loads the part
//! impact's class table in the item byte's own order (0 `FlsCol`, 1
//! `HitClt`, 2 `HitLtr`, 3 `HitChn`, 4 `HitPlt`, 5 `HitHbr`, 6 `HitGlm`)
//! and the foley's (0 `FoleyFls`, 1 `FoleyClt`, 2 `FoleyLtr`, 3
//! `FoleyChn`, 4 `FoleyPlt`, 5 `FoleyHbr`); the armour builder copies the
//! byte into the layer as is, and the part's hit-sound routine and its
//! foley routine index those tables by it. Class 0 never reaches the part
//! table: it takes the flesh default `bdycol` (`SkelHit` for a skeleton
//! race). The pack has no `foleyfls`, and class 0 and 1 never win the
//! foley (their factor is 0).

use bevy::prelude::*;

use bevy::platform::collections::HashMap;
use exa_assets::Game;
use exa_combat::Fighter;
use exa_combat::sounds::{FoleyKind, HitSound, foley};
use exa_combat::swing_voice::{SwingVoiceCue, fade_voice, swing_rustle};

use crate::environment::voice_flags;
use crate::{AudioEnvironment, SoundBank, SoundSet, VoiceHandle};

/// The swing voice's set (`SwingHA`), six variants.
pub const SWING: SoundSet<'static> = SoundSet::new("swingha", 6);
/// The second wind's heal (`StmHeal01`), one sound.
pub const HEAL: SoundSet<'static> = SoundSet::new("stmheal", 1);
/// The flesh hit default (`BdyCol`).
pub const HIT_FLESH: SoundSet<'static> = SoundSet::new("bdycol", 6);
/// The skeleton hit default (`SkelHit`).
pub const HIT_SKELETON: SoundSet<'static> = SoundSet::new("skelhit", 6);
/// The golem's hit sound (`HitGlm`).
pub const HIT_GOLEM: SoundSet<'static> = SoundSet::new("hitglm", 6);
/// The part impact's armour classes 1 to 5: cloth (`hitclt`), leather
/// (`hitltr`), mail (`hitchn`), plate (`hitplt`) and the sixth class
/// (`hithbr`); class 6 is [`HIT_GOLEM`].
pub const HIT_CLOTH: SoundSet<'static> = SoundSet::new("hitclt", 1);
pub const HIT_LEATHER: SoundSet<'static> = SoundSet::new("hitltr", 4);
pub const HIT_MAIL: SoundSet<'static> = SoundSet::new("hitchn", 3);
pub const HIT_PLATE: SoundSet<'static> = SoundSet::new("hitplt", 8);
pub const HIT_HBR: SoundSet<'static> = SoundSet::new("hithbr", 6);
/// The clothing foley (`FoleyPlH`) and the golem's (`Golem_Foley`).
pub const FOLEY_PLAYER: SoundSet<'static> = SoundSet::new("foleyplh", 9);
pub const FOLEY_GOLEM: SoundSet<'static> = SoundSet::new("golem_foley", 6);
/// The armour foley loops by hit-sound class, the original's table order:
/// 0 `FoleyFls` (not in the pack), 1 cloth, 2 leather, 3 mail, 4 plate,
/// 5 the sixth class.
pub const FOLEY_CLASSES: [Option<SoundSet<'static>>; 6] = [
    None,
    Some(SoundSet::new("foleyclt", 6)),
    Some(SoundSet::new("foleyltr", 6)),
    Some(SoundSet::new("foleychn", 6)),
    Some(SoundSet::new("foleyplt", 7)),
    Some(SoundSet::new("foleyhbr", 6)),
];

/// The set a part's material plays, or none when the class has no set:
/// the layer's class indexes the part table in the item byte's order (1
/// cloth, 2 leather, 3 mail, 4 plate, 5 `HitHbr`, 6 golem). Class 0 is the
/// flesh default, which [`exa_combat::sounds::part_hit_sound`] gives as
/// [`HitSound::Flesh`] or [`HitSound::Skeleton`].
pub fn hit_set(sound: HitSound) -> Option<SoundSet<'static>> {
    match sound {
        HitSound::Flesh | HitSound::Class(0) => Some(HIT_FLESH),
        HitSound::Skeleton => Some(HIT_SKELETON),
        HitSound::Golem | HitSound::Class(6) => Some(HIT_GOLEM),
        HitSound::Class(1) => Some(HIT_CLOTH),
        HitSound::Class(2) => Some(HIT_LEATHER),
        HitSound::Class(3) => Some(HIT_MAIL),
        HitSound::Class(4) => Some(HIT_PLATE),
        HitSound::Class(5) => Some(HIT_HBR),
        HitSound::Class(_) => None,
    }
}

/// The set a foley loop plays, or none.
pub fn foley_set(kind: FoleyKind) -> Option<SoundSet<'static>> {
    match kind {
        FoleyKind::None => None,
        FoleyKind::Class(class) => FOLEY_CLASSES.get(usize::from(class)).copied().flatten(),
        FoleyKind::Player => Some(FOLEY_PLAYER),
        FoleyKind::Golem => Some(FOLEY_GOLEM),
    }
}

/// A request to play the second wind's heal sound at a body.
#[derive(Message, Debug, Clone, Copy)]
pub struct HealSound {
    pub position: Vec3,
}

/// Plays a heal request at the original's volume 0.1 and pitch 1.
pub(crate) fn heal_sounds(
    game: Option<Res<Game>>,
    mut requests: MessageReader<HealSound>,
    mut bank: ResMut<SoundBank>,
    mut sources: ResMut<Assets<AudioSource>>,
    mut env: ResMut<AudioEnvironment>,
    mut set: Local<Option<u32>>,
) {
    let Some(game) = game else { return };
    for request in requests.read() {
        let id = *set.get_or_insert_with(|| bank.load_set(&game, &mut sources, HEAL.prefix));
        env.play_set(&mut bank, id, request.position, 0.1, 1.0, 0);
    }
}

/// A named one-shot at a point (m) at a set volume and pitch: the original's
/// point sound, which a picked-up object's use sound goes through (volume
/// 0.25, pitch 1). `flags` are the voice flags the caller adds (the story
/// events' `0x400`, and `0x4460` for the wail played at the origin; 0 for
/// the others).
#[derive(Message, Debug, Clone, PartialEq)]
pub struct PointSounded {
    pub name: String,
    pub position: Vec3,
    pub volume: f32,
    pub pitch: f32,
    pub flags: u32,
}

/// A door's lock sound levels from the two draws, in the order the original
/// takes them: the pitch first, `0.9 + 0.15·draw`, then the volume,
/// `0.15 + 0.1·draw`, each in double precision and narrowed. Returns
/// `(volume, pitch)`.
pub fn lock_sound_levels(pitch_draw: f64, volume_draw: f64) -> (f32, f32) {
    let pitch = (pitch_draw * 0.15 + 0.9) as f32;
    let volume = (volume_draw * 0.1 + 0.15) as f32;
    (volume, pitch)
}

/// Plays the named one-shots: a door's lock sound
/// ([`exa_physics::mechanism::LockSounded`]) at its drawn levels, and the
/// point sounds at theirs. The lock sound is a numbered set (`doorb_lko01`
/// …, as the door's init loads it); a point sound is one file. Each name is
/// loaded once and kept; a sound the packs do not hold plays nothing.
#[allow(clippy::too_many_arguments)]
pub(crate) fn named_sounds(
    game: Option<Res<Game>>,
    mut locks: MessageReader<exa_physics::mechanism::LockSounded>,
    mut points: MessageReader<PointSounded>,
    mut bank: ResMut<SoundBank>,
    mut sources: ResMut<Assets<AudioSource>>,
    mut env: ResMut<AudioEnvironment>,
    mut loaded: Local<HashMap<String, Option<u32>>>,
    mut lock_sets: Local<HashMap<String, u32>>,
) {
    let Some(game) = game else {
        locks.clear();
        points.clear();
        return;
    };
    for lock in locks.read() {
        let pitch_draw = bank.rng().unit_f64();
        let volume_draw = bank.rng().unit_f64();
        let (volume, pitch) = lock_sound_levels(pitch_draw, volume_draw);
        let set = *lock_sets
            .entry(lock.name.clone())
            .or_insert_with(|| bank.load_set(&game, &mut sources, &lock.name));
        env.play_set(&mut bank, set, lock.position, volume, pitch, 0);
        info!(
            "mechanism sound: {} volume {volume:.4} pitch {pitch:.4} at ({:.2}, {:.2}, {:.2})",
            lock.name, lock.position.x, lock.position.y, lock.position.z
        );
    }
    for point in points.read() {
        let buffer = *loaded
            .entry(point.name.clone())
            .or_insert_with(|| bank.load_buffer(&game, &mut sources, &point.name));
        if let Some(buffer) = buffer {
            env.play_buffer(
                buffer,
                point.position,
                point.volume,
                point.pitch,
                point.flags,
            );
        }
    }
}

/// A fighter's swing voice as the mixer keeps it: the voice of its first
/// held item (allocated at its first swing), the clothing rustle's handle
/// and the foley clock (ms) the rustle waits on.
#[derive(Debug, Clone, Copy)]
pub(crate) struct HeldVoice {
    voice: Option<u32>,
    rustle: VoiceHandle,
    foley_next_ms: u32,
}

/// Plays the swing voice's cues ([`SwingVoiceCue`]): a start restarts the
/// fighter's held voice on a variant of `SwingHA`, silent, and rustles its
/// clothing foley once the foley clock allows (a fighter without a foley
/// set never rustles); an attack tick sets the voice's volume and pitch;
/// a tick that is not an attack fades a playing voice and asks it to stop
/// under 0.005. The voices of fighters that are gone are freed.
#[allow(clippy::too_many_arguments)]
pub(crate) fn swing_voices(
    game: Option<Res<Game>>,
    mut cues: MessageReader<SwingVoiceCue>,
    fighters: Query<&Fighter>,
    mut bank: ResMut<SoundBank>,
    mut sources: ResMut<Assets<AudioSource>>,
    mut env: ResMut<AudioEnvironment>,
    mut swing_set: Local<Option<u32>>,
    mut foley_sets: Local<HashMap<&'static str, u32>>,
    mut held: Local<HashMap<Entity, HeldVoice>>,
) {
    let Some(game) = game else { return };
    for cue in cues.read() {
        match *cue {
            SwingVoiceCue::Start {
                fighter,
                at,
                pitch,
                now_ms,
            } => {
                let Ok(f) = fighters.get(fighter) else {
                    continue;
                };
                let foley = foley(&f.armour, false, false, false);
                let foley_set = foley_set(foley.kind);
                let entry = held.entry(fighter).or_insert_with(|| HeldVoice {
                    voice: None,
                    rustle: env.new_handle(),
                    foley_next_ms: if foley_set.is_some() { 0 } else { u32::MAX },
                });
                if entry.voice.is_none() {
                    entry.voice = env.allocate();
                }
                let set = *swing_set
                    .get_or_insert_with(|| bank.load_set(&game, &mut sources, SWING.prefix));
                if let Some(id) = entry.voice {
                    env.retrigger_set(&mut bank, id, set, at, 0.0, pitch);
                }
                let Some(foley_set) = foley_set else {
                    continue;
                };
                if entry.foley_next_ms >= now_ms {
                    continue;
                }
                let r_volume = bank.random_unit();
                let r_pitch = bank.random_unit();
                let mut roll = bank.rng().0;
                if let Some(rustle) = swing_rustle(
                    foley.volume,
                    1.0,
                    now_ms,
                    entry.foley_next_ms,
                    r_volume,
                    r_pitch,
                    &mut roll,
                ) {
                    bank.rng().0 = roll;
                    entry.foley_next_ms = rustle.next_ms;
                    let id = *foley_sets
                        .entry(foley_set.prefix)
                        .or_insert_with(|| bank.load_set(&game, &mut sources, foley_set.prefix));
                    env.play_set_attached(
                        &mut bank,
                        id,
                        entry.rustle,
                        at,
                        rustle.gain,
                        rustle.pitch,
                    );
                }
            }
            SwingVoiceCue::Sound {
                fighter,
                at,
                volume,
                pitch,
            } => {
                let Some(id) = held.get(&fighter).and_then(|h| h.voice) else {
                    continue;
                };
                if let Some(voice) = env.voice_mut(id) {
                    voice.gain = volume;
                    voice.pitch = pitch;
                    voice.position = at;
                }
            }
            SwingVoiceCue::Fade { fighter } => {
                let Some(id) = held.get(&fighter).and_then(|h| h.voice) else {
                    continue;
                };
                if let Some(voice) = env.voice_mut(id)
                    && voice.flags & voice_flags::ACTIVE != 0
                {
                    let (gain, pitch, stop) = fade_voice(voice.gain, voice.pitch);
                    voice.gain = gain;
                    voice.pitch = pitch;
                    if stop {
                        voice.flags |= voice_flags::STOP;
                    }
                }
            }
        }
    }
    held.retain(|fighter, voice| {
        let alive = fighters.contains(*fighter);
        if !alive {
            if let Some(id) = voice.voice {
                env.free_voice(&mut bank, id);
            }
            env.free_handle(&mut bank, voice.rustle);
        }
        alive
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The part impact's classes follow the item byte's order (0 flesh, 1
    /// cloth, 2 leather, 3 mail, 4 plate, 5 `HitHbr`, 6 golem), and unknown
    /// classes have no set.
    #[test]
    fn the_hit_sets_follow_the_item_bytes_order() {
        assert_eq!(hit_set(HitSound::Flesh), Some(HIT_FLESH));
        assert_eq!(hit_set(HitSound::Skeleton), Some(HIT_SKELETON));
        assert_eq!(hit_set(HitSound::Golem), Some(HIT_GOLEM));
        let names: Vec<_> = (0..8)
            .map(|c| hit_set(HitSound::Class(c)).map(|s| s.prefix))
            .collect();
        assert_eq!(
            names,
            [
                Some("bdycol"),
                Some("hitclt"),
                Some("hitltr"),
                Some("hitchn"),
                Some("hitplt"),
                Some("hithbr"),
                Some("hitglm"),
                None
            ]
        );
    }

    /// A class's foley loop is the original's per-class set (no flesh set),
    /// a golem's its own, and no loop none.
    #[test]
    fn the_foley_sets_follow_the_item_bytes_order() {
        let names: Vec<_> = (0..7)
            .map(|c| foley_set(FoleyKind::Class(c)).map(|s| s.prefix))
            .collect();
        assert_eq!(
            names,
            [
                None,
                Some("foleyclt"),
                Some("foleyltr"),
                Some("foleychn"),
                Some("foleyplt"),
                Some("foleyhbr"),
                None
            ]
        );
        assert_eq!(foley_set(FoleyKind::None), None);
        assert_eq!(foley_set(FoleyKind::Player), Some(FOLEY_PLAYER));
        assert_eq!(foley_set(FoleyKind::Golem), Some(FOLEY_GOLEM));
    }

    /// A worn breastplate (item byte 4) rings plate, a mail shirt (3) mail
    /// and leather boots (2) leather; a full plate body loops `foleyplt`.
    #[test]
    fn worn_items_ring_their_own_class() {
        use exa_combat::damage::{Layer, PartArmour};
        use exa_combat::sounds::part_hit_sound;
        let part = |material| PartArmour {
            outer: Some(Layer {
                material,
                coverage: 1.0,
                ..Default::default()
            }),
            ..Default::default()
        };
        let set = |m| hit_set(part_hit_sound(&part(m), false, false).unwrap());
        assert_eq!(set(4), Some(HIT_PLATE));
        assert_eq!(set(3), Some(HIT_MAIL));
        assert_eq!(set(2), Some(HIT_LEATHER));
        assert_eq!(set(1), Some(HIT_CLOTH));
        assert_eq!(set(0), Some(HIT_FLESH));
        let loop_ = foley(&[part(4); 18], false, false, false);
        assert_eq!(loop_.kind, FoleyKind::Class(4));
        assert_eq!(foley_set(loop_.kind).map(|s| s.prefix), Some("foleyplt"));
    }

    /// The installed `Sound.rpk` holds every variant of the part impact's
    /// and the armour foley's sets and no more, and no `foleyfls` (skipped
    /// without `EXANIMA_DIR`).
    #[test]
    fn the_installed_class_sets_have_every_variant() {
        use exa_assets::{GameData, PackKind, locate_install};
        let Some(dir) = locate_install(None) else {
            return;
        };
        let game = GameData::open(&dir).unwrap();
        let sounds = game.pack(PackKind::Sound).unwrap();
        let sets = (0..7)
            .filter_map(|c| hit_set(HitSound::Class(c)))
            .chain([HIT_SKELETON])
            .chain(FOLEY_CLASSES.iter().flatten().copied());
        for set in sets {
            for i in 0..set.count {
                assert!(sounds.get(&set.name(i)).is_some(), "{}", set.name(i));
            }
            assert!(sounds.get(&set.name(set.count)).is_none(), "{set:?}");
        }
        assert!(sounds.get("foleyfls01").is_none());
    }

    /// The lock sound's levels: draws 0 give volume 0.15 and pitch 0.9,
    /// draws 0.5 volume 0.2 and pitch 0.975; the first draw is the pitch.
    #[test]
    fn the_lock_sound_draws_its_pitch_first() {
        assert_eq!(lock_sound_levels(0.0, 0.0), (0.15, 0.9));
        assert_eq!(lock_sound_levels(0.5, 0.5), (0.2, 0.975));
        assert_eq!(lock_sound_levels(1.0, 0.0), (0.15, 1.05));
        assert_eq!(lock_sound_levels(0.0, 1.0), (0.25, 0.9));
    }
}
