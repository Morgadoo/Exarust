//! Sounds of hits: a weapon impact and a pain grunt at the contact point,
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
use exa_combat::Hit;
use exa_combat::impact::ImpactSounded;
use exa_combat::sounds::{foley, part_hit_sound};

use crate::environment::AudioEnvironment;
use crate::sounds::{foley_set, hit_set};
use crate::{SoundBank, SoundSet};

/// Sword impacts, the fallback when a weapon names no set.
pub(crate) const SWORD_IMPACTS: SoundSet<'static> = SoundSet::new("wh_sworda", 6);
/// Human voices, each with its grunts.
pub(crate) const VOICES: SoundSet<'static> = SoundSet::new("vhm", 9);
pub(crate) const GRUNTS: usize = 6;

#[allow(clippy::too_many_arguments)]
pub(crate) fn hit_sounds(
    game: Option<Res<Game>>,
    mut hits: MessageReader<Hit>,
    fighters: Query<&Fighter>,
    mut bank: ResMut<SoundBank>,
    mut sources: ResMut<Assets<AudioSource>>,
    mut env: ResMut<AudioEnvironment>,
    mut impacts: Local<Option<u32>>,
    mut weapon_sets: Local<[Option<u32>; 9]>,
    mut part_sets: Local<[Option<u32>; 7]>,
    mut grunt_sets: Local<[Option<u32>; VOICES.count]>,
) {
    let Some(game) = game else { return };
    for hit in hits.read() {
        // The weapon's own hit set, by its record's class.
        let class = usize::from(hit.weapon.hit_sound);
        let weapon = if let Some(name) = hit.weapon.hit_sound_name() {
            *weapon_sets
                .get_mut(class)
                .unwrap_or(&mut None)
                .get_or_insert_with(|| bank.load_set(&game, &mut sources, name))
        } else {
            *impacts.get_or_insert_with(|| bank.load_set(&game, &mut sources, SWORD_IMPACTS.prefix))
        };
        env.play_set(&mut bank, weapon, hit.point, 1.0, 1.0, 0);
        // The struck part's material: its outer covering layer's class, or
        // the flesh default.
        if let Ok(fighter) = fighters.get(hit.target)
            && let Some(armour) = hit.bone.checked_sub(1).and_then(|i| fighter.armour.get(i))
            && let Some(sound) = part_hit_sound(armour, false, false)
            && let Some(set) = hit_set(sound)
        {
            let class = usize::from(sound_class(sound));
            let id = *part_sets[class]
                .get_or_insert_with(|| bank.load_set(&game, &mut sources, set.prefix));
            env.play_set(&mut bank, id, hit.point, 1.0, 1.0, 0);
        }
        let voice = bank.pick(VOICES.count);
        let grunt = match grunt_sets[voice] {
            Some(set) => set,
            None => {
                let base = format!("{}_grunts", VOICES.name(voice));
                let grunt_set = SoundSet::new(&base, GRUNTS);
                let set = bank.load_set(&game, &mut sources, grunt_set.prefix);
                grunt_sets[voice] = Some(set);
                set
            }
        };
        env.play_set(&mut bank, grunt, hit.point, 0.8, 1.0, 0);
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

/// The table index a part's material takes.
fn sound_class(sound: exa_combat::sounds::HitSound) -> u8 {
    use exa_combat::sounds::HitSound;
    match sound {
        HitSound::Flesh | HitSound::Skeleton => 0,
        HitSound::Golem => 6,
        HitSound::Class(c) => c.min(6),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::voice_flags;
    use crate::sounds::{FOLEY_PLAYER, HIT_FLESH};
    use exa_combat::damage::{Layer, PartArmour};

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
