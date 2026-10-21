//! Sounds of hits: a weapon impact and a pain grunt at the contact point.
//!
//! Both are one-shots from numbered sets, so they go through the mixer's
//! voice pool and source ranking rather than straight to the backend.

use bevy::prelude::*;
use exa_assets::Game;
use exa_combat::Fighter;
use exa_combat::Hit;
use exa_combat::sounds::part_hit_sound;

use crate::environment::AudioEnvironment;
use crate::sounds::hit_set;
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
}
