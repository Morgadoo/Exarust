//! The world clock across a save and a load.
//!
//! The original keeps two words: the seconds a save carried and the tick
//! they were restored at. The clock at any later tick is the saved seconds
//! plus the whole seconds since that tick (the difference of the
//! zero-extended 32-bit tick words, divided signed and rounded toward
//! zero). The player-data readers restore the pair when they read a save,
//! and the writers stamp the current seconds into the header, so the world
//! time a script, a dialogue line or an elixir reads continues across
//! saves.
//!
//! [`restored`] and [`stamped`] are that pair over Exarust's
//! [`GameClock`]; [`RestoreWorldClock`] carries a loaded save's seconds to
//! the running game and [`WorldClockPlugin`] applies it at the tick the
//! load happened on. A session start sends zero, so a fresh level's world
//! time begins at its load rather than at the process's start.
//!
//! Evidence: the save unit's clock getter and its setter, and the two
//! player-data readers that call the setter (`docs/formats/SAVES.md`, The
//! game clock).

use bevy::prelude::*;
use exa_combat::remedy::WorldClock;
use exa_formats::save_chunk::GameClock;
use exa_physics::body::BodyTicks;

/// The world clock a load restores from a save's seconds, its base at the
/// tick that read them.
pub fn restored(saved_seconds: u32, ticks_ms: u32) -> WorldClock {
    WorldClock(GameClock::restore(saved_seconds, ticks_ms))
}

/// The seconds a save stamps: the world clock at the save's tick.
pub fn stamped(clock: &WorldClock, ticks_ms: u32) -> u32 {
    clock.0.now(ticks_ms)
}

/// A loaded save's world clock, sent to the running game. The host sends it
/// after reading a session's player data (the original's setter runs inside
/// the reader) and at a session start with zero.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RestoreWorldClock {
    /// The seconds the save carried.
    pub saved_seconds: u32,
}

/// Applies [`RestoreWorldClock`] to the world clock at the tick it is read.
pub struct WorldClockPlugin;

impl Plugin for WorldClockPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldClock>()
            .add_message::<RestoreWorldClock>()
            .add_systems(Update, apply_restore);
    }
}

/// The game time the messages are applied at; without the fixed clock the
/// process starts at zero, as a fresh [`GameClock`] does.
fn game_millis(ticks: Option<&BodyTicks>) -> u32 {
    ticks.map_or(0, |ticks| ticks.millis() as u32)
}

fn apply_restore(
    mut messages: MessageReader<RestoreWorldClock>,
    mut clock: ResMut<WorldClock>,
    ticks: Option<Res<BodyTicks>>,
) {
    let now = game_millis(ticks.as_deref());
    for message in messages.read() {
        *clock = restored(message.saved_seconds, now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_clock_keeps_the_saved_seconds_and_counts_whole_seconds_since_the_load() {
        let clock = restored(8684, 1_000);
        assert_eq!(stamped(&clock, 1_000), 8684);
        assert_eq!(stamped(&clock, 1_999), 8684);
        assert_eq!(stamped(&clock, 2_000), 8685);
        assert_eq!(stamped(&clock, 61_000), 8744);
        // A tick before the base counts back (a signed divide), and the sum
        // wraps to 32 bits.
        assert_eq!(stamped(&clock, 500), 8684);
        assert_eq!(stamped(&clock, 0), 8683);
        let edge = restored(0, 1_000);
        assert_eq!(stamped(&edge, 0), u32::MAX);
        // A NaN clock cannot arise; a restored zero behaves as a new clock.
        assert_eq!(stamped(&restored(0, 0), 12_345), 12);
    }

    #[test]
    fn retail_saves_keep_their_clock_when_a_save_folder_is_given() {
        // A real-data check: `EXANIMA_SAVES` names a folder of the user's
        // own saves. The header's clock is the base a load restores, and
        // stamping at the restoring tick writes the same word back. It
        // skips when the variable is unset; nothing is written.
        let Ok(dir) = std::env::var("EXANIMA_SAVES") else {
            return;
        };
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return;
        };
        for entry in entries.flatten() {
            let bytes = std::fs::read(entry.path()).unwrap();
            let Ok(package) = exa_formats::save_package::PackageImage::read(&bytes) else {
                continue;
            };
            let Some(section) = package.section(exa_formats::save_package::PLAYER_DATA) else {
                continue;
            };
            let signature = u32::from_le_bytes(section[0..4].try_into().unwrap());
            let flavour = if signature == exa_formats::save_package::ARENA_SIGNATURE {
                exa_formats::save_package::Flavour::Arena
            } else {
                exa_formats::save_package::Flavour::Story
            };
            let Ok(data) = exa_formats::save_package::PlayerData::parse(section, flavour) else {
                continue;
            };
            let saved = data.header.clock();
            let clock = restored(saved, 1_000);
            assert_eq!(stamped(&clock, 1_000), saved);
            // A minute later the world clock is a minute further on.
            assert_eq!(stamped(&clock, 61_000), saved.wrapping_add(60));
            assert!(saved > 0, "{}", entry.file_name().to_string_lossy());
        }
    }

    #[test]
    fn a_restore_message_sets_the_running_clock_at_its_tick() {
        let mut app = App::new();
        app.add_plugins(WorldClockPlugin);
        // Without the fixed clock the message is applied at tick zero.
        app.world_mut()
            .write_message(RestoreWorldClock { saved_seconds: 120 });
        app.update();
        let clock = *app.world().resource::<WorldClock>();
        assert_eq!(clock.0.saved(), 120);
        assert_eq!(clock.0.base(), 0);
        assert_eq!(stamped(&clock, 5_000), 125);

        // With the body clock the base is the tick the message was read at.
        app.insert_resource(BodyTicks(250));
        app.world_mut()
            .write_message(RestoreWorldClock { saved_seconds: 7 });
        app.update();
        let clock = *app.world().resource::<WorldClock>();
        assert_eq!(
            clock.0.base(),
            250 * exa_physics::body::GAME_MS_PER_TICK as u32
        );
        assert_eq!(clock.0.saved(), 7);
    }
}
