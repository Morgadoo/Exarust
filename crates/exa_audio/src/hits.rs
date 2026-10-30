//! Sounds of hits: a wound's armour layers and a held weapon's ring at the
//! contact point, a wound's pain sounds (the wound set and the pain voice),
//! and a body's impact on a limb: the struck bone's hit sound, or the
//! character's foley when its root is crowded
//! ([`exa_combat::impact::ImpactSounded`]).
//!
//! [inf] The foley is taken from the character's armour as for a body that
//! is not a golem, as the hit sound is.
//!
//! Both are one-shots from numbered sets, so they go through the mixer's
//! voice pool and source ranking rather than straight to the backend.

use bevy::prelude::*;
use exa_assets::Game;
use exa_combat::Fighter;
use exa_combat::impact::ImpactSounded;
use exa_combat::pain::PainSounded;
use exa_combat::sounds::{LayerSounded, foley, part_hit_sound};
use exa_combat::voice::{Answer, CharacterVoice};
use exa_physics::body::BodyTicks;

use crate::environment::AudioEnvironment;
use crate::sounds::{foley_set, hit_set};
use crate::{SoundBank, SoundSet};

/// Sword impacts, the fallback when a weapon names no set.
pub(crate) const SWORD_IMPACTS: SoundSet<'static> = SoundSet::new("wh_sworda", 6);
/// Human voices, each with its grunts (the installed-sets check).
#[cfg(test)]
pub(crate) const VOICES: SoundSet<'static> = SoundSet::new("vhm", 9);
#[cfg(test)]
pub(crate) const GRUNTS: usize = 6;

/// Plays a wound's armour layers ([`exa_combat::sounds::LayerSounded`]):
/// each layer's class set at the blow, at the ring's volume and pitch, on
/// the plain one-shot flags. The striking weapon's ring on the same wound
/// comes as a [`exa_combat::impact::WeaponSounded`] ([`weapon_sounds`]).
pub(crate) fn layer_sounds(
    game: Option<Res<Game>>,
    mut rings: MessageReader<LayerSounded>,
    mut bank: ResMut<SoundBank>,
    mut sources: ResMut<Assets<AudioSource>>,
    mut env: ResMut<AudioEnvironment>,
    mut sets: Local<bevy::platform::collections::HashMap<&'static str, u32>>,
) {
    let Some(game) = game else { return };
    for ring in rings.read() {
        let id = *sets
            .entry(ring.set)
            .or_insert_with(|| bank.load_set(&game, &mut sources, ring.set));
        env.play_set(&mut bank, id, ring.point, ring.volume, ring.pitch, 0);
    }
}

/// Plays a wound's pain sounds ([`exa_combat::pain::PainSounded`]): the
/// character's wound set at the blow, and its pain voice through its
/// voice's request ([`CharacterVoice::speak`]: the line's set, its waits
/// and the character's gains) at its place. A fighter without a voice
/// makes neither; a set the pack lacks plays nothing and leaves the wait.
#[allow(clippy::too_many_arguments)]
pub(crate) fn pain_sounds(
    game: Option<Res<Game>>,
    mut pains: MessageReader<PainSounded>,
    mut voices: Query<(&mut CharacterVoice, &GlobalTransform)>,
    ticks: Option<Res<BodyTicks>>,
    mut bank: ResMut<SoundBank>,
    mut sources: ResMut<Assets<AudioSource>>,
    mut env: ResMut<AudioEnvironment>,
    mut sets: Local<bevy::platform::collections::HashMap<String, u32>>,
) {
    let Some(game) = game else { return };
    let now = ticks.as_ref().map_or(0, |t| t.millis()) as u32;
    for pain in pains.read() {
        let Ok((mut voice, at)) = voices.get_mut(pain.fighter) else {
            continue;
        };
        let mut set_of = |name: &str, bank: &mut SoundBank| {
            *sets
                .entry(name.to_string())
                .or_insert_with(|| bank.load_set(&game, &mut sources, name))
        };
        if let (Some(loud), Some(name)) = (pain.sounds.wound, voice.wound.clone()) {
            let id = set_of(&name, &mut bank);
            if id != SoundBank::EMPTY_SET {
                env.play_set(&mut bank, id, pain.point, loud.volume, loud.pitch, 0);
            }
            debug!(
                "pain: wound set {name} (loaded {})",
                id != SoundBank::EMPTY_SET
            );
        }
        let Some((line, loud)) = pain.sounds.voice else {
            continue;
        };
        let Some(name) = voice.set(line).map(str::to_string) else {
            continue;
        };
        let id = set_of(&name, &mut bank);
        let answer = voice.speak(
            line,
            id != SoundBank::EMPTY_SET,
            now,
            loud.volume,
            loud.pitch,
        );
        debug!("pain: voice {name} line {line:?}: {answer:?}");
        if let Answer::Played(spoken) = answer {
            env.play_set(
                &mut bank,
                id,
                at.translation(),
                spoken.volume,
                spoken.pitch,
                0,
            );
        }
    }
}

/// The set a limb impact plays and its gain, or none: the foley's set at
/// the volume times the foley's own, else the bone's hit sound's.
pub(crate) fn impact_play(
    sound: &ImpactSounded,
    fighter: &Fighter,
) -> Option<(SoundSet<'static>, f32)> {
    if sound.foley {
        let loop_ = foley(&fighter.armour, false, false, false);
        return foley_set(loop_.kind).map(|set| (set, sound.volume * loop_.volume));
    }
    let armour = sound
        .bone
        .checked_sub(1)
        .and_then(|i| fighter.armour.get(i))?;
    hit_set(part_hit_sound(armour, false, false)?).map(|set| (set, sound.volume))
}

pub(crate) fn impact_sounds(
    game: Option<Res<Game>>,
    mut sounded: MessageReader<ImpactSounded>,
    fighters: Query<&Fighter>,
    mut bank: ResMut<SoundBank>,
    mut sources: ResMut<Assets<AudioSource>>,
    mut env: ResMut<AudioEnvironment>,
    mut sets: Local<bevy::platform::collections::HashMap<&'static str, u32>>,
) {
    let Some(game) = game else { return };
    for sound in sounded.read() {
        let Some((set, gain)) = fighters
            .get(sound.target)
            .ok()
            .and_then(|f| impact_play(sound, f))
        else {
            continue;
        };
        let id = *sets
            .entry(set.prefix)
            .or_insert_with(|| bank.load_set(&game, &mut sources, set.prefix));
        env.play_set(&mut bank, id, sound.point, gain, sound.pitch, 0);
    }
}

/// Plays a held weapon's own impact sound at the item's point: the
/// weapon's named hit set, or the sword fallback, at the branch's volume
/// and pitch.
pub(crate) fn weapon_sounds(
    game: Option<Res<Game>>,
    mut sounded: MessageReader<exa_combat::impact::WeaponSounded>,
    mut bank: ResMut<SoundBank>,
    mut sources: ResMut<Assets<AudioSource>>,
    mut env: ResMut<AudioEnvironment>,
    mut sets: Local<bevy::platform::collections::HashMap<Option<String>, u32>>,
) {
    let Some(game) = game else { return };
    for sound in sounded.read() {
        let key = sound.set.clone();
        let id = *sets.entry(key).or_insert_with(|| {
            let name = sound.set.as_deref().unwrap_or(SWORD_IMPACTS.prefix);
            bank.load_set(&game, &mut sources, name)
        });
        env.play_set(&mut bank, id, sound.point, sound.volume, sound.pitch, 0);
    }
}

/// The set a block's ring plays: the struck item's root part's collision
/// sound set, none when its material names none.
pub(crate) fn ring_set(sounds: &exa_physics::prop::BodySounds) -> Option<&str> {
    sounds.impact.first()?.as_deref()
}

/// Plays the first block of a swing on a held shield or torch
/// ([`exa_combat::block::ShieldRung`]): the item's own material collision
/// set at the blow, at the ring's volume and pitch, on the plain one-shot
/// flags. An item whose material names no set stays silent, as the
/// original's body with no impact sound does.
pub(crate) fn shield_sounds(
    game: Option<Res<Game>>,
    mut rungs: MessageReader<exa_combat::block::ShieldRung>,
    items: Query<&exa_physics::prop::BodySounds>,
    mut bank: ResMut<SoundBank>,
    mut sources: ResMut<Assets<AudioSource>>,
    mut env: ResMut<AudioEnvironment>,
    mut sets: Local<bevy::platform::collections::HashMap<String, u32>>,
) {
    let Some(game) = game else { return };
    for rung in rungs.read() {
        let Some(name) = items.get(rung.item).ok().and_then(ring_set) else {
            continue;
        };
        let id = *sets
            .entry(name.to_owned())
            .or_insert_with(|| bank.load_set(&game, &mut sources, name));
        env.play_set(&mut bank, id, rung.point, rung.volume, rung.pitch, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::voice_flags;
    use crate::sounds::{FOLEY_PLAYER, HIT_FLESH};
    use exa_combat::damage::{Layer, PartArmour};

    /// A block rings the struck item's root part's set; an item without a
    /// material set rings nothing.
    #[test]
    fn a_block_rings_the_items_root_set() {
        let sounds = exa_physics::prop::BodySounds {
            impact: vec![Some("wodcol".into()), Some("mtlcol".into())],
            scrapes: Vec::new(),
        };
        assert_eq!(ring_set(&sounds), Some("wodcol"));
        let silent = exa_physics::prop::BodySounds {
            impact: vec![None],
            scrapes: Vec::new(),
        };
        assert_eq!(ring_set(&silent), None);
        assert_eq!(ring_set(&exa_physics::prop::BodySounds::default()), None);
    }

    /// A hit submits two one-shots on the default one-shot flags, at full and
    /// 0.8 gain, and they take voices from the pool.
    #[test]
    fn a_hit_submits_two_one_shots() {
        let mut env = AudioEnvironment::new();
        let mut bank = SoundBank::default();
        let impacts = bank.insert_test_set(vec![1, 2]);
        let grunts = bank.insert_test_set(vec![3, 4]);
        env.play_set(&mut bank, impacts, Vec3::new(1.0, 2.0, 3.0), 1.0, 1.0, 0);
        env.play_set(&mut bank, grunts, Vec3::new(1.0, 2.0, 3.0), 0.8, 1.0, 0);
        let first = env.voice(0).unwrap();
        assert_eq!(first.flags, crate::environment::DEFAULT_ONE_SHOT_FLAGS);
        assert_eq!(first.gain, 1.0);
        assert_eq!(first.position, Vec3::new(1.0, 2.0, 3.0));
        let second = env.voice(1).unwrap();
        assert_eq!(second.gain, 0.8);
        assert_eq!(second.flags & voice_flags::ACTIVE, 0);
    }

    /// A limb impact plays the bone's hit sound at its volume; crowded, the
    /// foley's set at the volume times the foley's, and nothing without a
    /// foley.
    #[test]
    fn a_limb_impact_picks_the_hit_sound_or_the_foley() {
        let mut fighter = Fighter::default();
        let mut sound = ImpactSounded {
            target: Entity::PLACEHOLDER,
            bone: 1,
            point: Vec3::ZERO,
            volume: 0.4,
            pitch: 0.74,
            foley: false,
        };
        assert_eq!(impact_play(&sound, &fighter), Some((HIT_FLESH, 0.4)));
        sound.foley = true;
        assert_eq!(impact_play(&sound, &fighter), None);
        let leather = Layer {
            material: 3,
            coverage: 1.0,
            ..Default::default()
        };
        fighter.armour = [PartArmour {
            outer: Some(leather),
            ..Default::default()
        }; 18];
        let loop_ = foley(&fighter.armour, false, false, false);
        assert!(loop_.volume > 0.0);
        assert_eq!(
            impact_play(&sound, &fighter),
            Some((FOLEY_PLAYER, 0.4 * loop_.volume))
        );
    }
}
