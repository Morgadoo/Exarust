//! Every fighter's voice, as the original's character init builds it: the
//! four sets of its voice (the race's first template and the record's voice
//! number while it is not feral, the second template and its undead voice
//! when it is, the record's sex choosing the gendered name), its wound set
//! by race, and its gains (the voice volume, a sound gain of 1, the voice
//! pitch). A fighter without a record (the duel's) speaks as a new record
//! does: a human male of voice 0. The pain sounds play through it
//! (`exa_audio`'s pain sounds).

use bevy::prelude::*;
use exa_assets::{Game, PackKind};
use exa_character::record::CharacterData;
use exa_combat::Fighter;
use exa_combat::pain::wound_set;
use exa_combat::voice::CharacterVoice;
use exa_formats::characters::CharacterRecord;
use exa_formats::races::{self, Race, VoiceSlot};

pub struct CharacterVoicePlugin;

impl Plugin for CharacterVoicePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, give_voices);
    }
}

/// A character's voice from its record, its race and its stats' voice
/// volume and pitch; `exists` says whether the sound pack holds a set.
pub fn voice_of(
    record: &CharacterRecord,
    race: &Race,
    volume: f32,
    pitch: f32,
    exists: impl Fn(&str) -> bool,
) -> CharacterVoice {
    // The living voice unless the feral level is above 0 (a NaN level is
    // not at or under 0 and takes the undead voice, as the init's test).
    let names = if record.feral <= 0.0 {
        race.voice.names(
            record.voice_index(),
            record.female,
            VoiceSlot::First,
            exists,
        )
    } else {
        race.voice.names(
            record.undead_voice,
            record.female,
            VoiceSlot::Second,
            exists,
        )
    };
    CharacterVoice {
        grunt: names.grunt,
        groan_short: names.groan_short,
        groan_heavy: names.groan_heavy,
        scream: names.scream,
        wound: wound_set(record.race).map(str::to_string),
        volume,
        gain: SOUND_GAIN,
        pitch,
        clock: Default::default(),
    }
}

/// The character's sound gain, 1 from the constructor.
const SOUND_GAIN: f32 = 1.0;

/// Gives each new fighter its voice, once.
fn give_voices(
    mut commands: Commands,
    game: Option<Res<Game>>,
    fighters: Query<(Entity, &Fighter, Option<&CharacterData>), Added<Fighter>>,
    mut table: Local<Option<Vec<Race>>>,
) {
    if fighters.is_empty() {
        return;
    }
    let Some(game) = game else { return };
    let races = table.get_or_insert_with(|| crate::scene_bodies::game_races(&game));
    let sounds = game.0.pack(PackKind::Sound);
    let exists = |name: &str| {
        sounds.is_some_and(|p| p.get(&format!("{}01", name.to_ascii_lowercase())).is_some())
    };
    let fresh = CharacterRecord::new();
    let builtin = races::builtin_human();
    for (entity, fighter, data) in &fighters {
        let record = data.map_or(&fresh, |d| &d.record);
        let race = races::of_character(races, record.race).unwrap_or(&builtin);
        let voice = voice_of(
            record,
            race,
            fighter.stats.voice_volume,
            fighter.stats.voice_pitch,
            exists,
        );
        commands.entity(entity).insert(voice);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A living voice takes the first template numbered from one, a feral
    /// one the second numbered from zero; a female voice the female names
    /// of a gendered race; the wound set follows the race byte.
    #[test]
    fn the_voice_follows_the_record() {
        let mut race = races::builtin_human();
        race.voice.first.male = "vhm".into();
        race.voice.first.female = "vhf".into();
        race.voice.second.male = "vhmda".into();
        race.voice.second.female = "vhfda".into();
        race.voice.gendered = true;
        race.voice.numbered = true;
        let mut r = CharacterRecord::new();
        let v = voice_of(&r, &race, 1.5, 0.95, |_| true);
        assert_eq!(v.groan_short, "vhm01_GroanS");
        assert_eq!(v.groan_heavy, "vhm01_GroanH");
        assert_eq!(v.grunt, "vhm01_GruntS");
        assert_eq!(v.wound.as_deref(), Some("WndFls"));
        assert_eq!((v.volume, v.gain, v.pitch), (1.5, 1.0, 0.95));
        r.female = true;
        r.feral = 0.5;
        r.undead_voice = 2;
        r.race = 1;
        let v = voice_of(&r, &race, 1.0, 1.0, |_| true);
        assert_eq!(v.groan_short, "vhfda02_GroanS");
        assert_eq!(v.wound.as_deref(), Some("SkelHit"));
        // A missing heavy groan shares the short one (first template).
        r.feral = 0.0;
        let v = voice_of(&r, &race, 1.0, 1.0, |n| !n.ends_with("GroanH"));
        assert_eq!(v.groan_heavy, "vhf01_GroanS");
    }
}
