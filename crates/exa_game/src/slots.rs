//! Save slots on disk: the save folder's `ExanimaNNN.rsg` / `ArenaNNN.rsg`
//! files, their checkpoints, the most-recently-used order kept in
//! `Exanima.set`, deleting a slot, and the permadeath rule.
//!
//! The original scans the folder with `FindNextFile`, keeps the first file
//! name per number, opens each package and drops (deletes both files of) a
//! slot whose `#playerdata` its mode refuses. New slots take the lowest free
//! number and go to the front of the list. Loading a slot moves it to the
//! front; loading from a checkpoint then copies the checkpoint over the save.
//! A checkpoint is only written when none exists, and a dead player's slot is
//! dropped from the list when it has no checkpoint (the files are left on
//! disk: the original's permadeath routine only edits the list).
//!
//! Evidence: `docs/formats/SAVES.md`.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use exa_formats::save_chunk;
use exa_formats::save_package::{self, PLAYER_DATA, PackageImage, SlotKind};
use exa_formats::settings_store::SettingsStore;

use crate::session::{self, Mode, SaveError, Session, SlotInfo};

/// The settings store file the slot order lives in, in the user folder.
pub const STORE_FILE: &str = "Exanima.set";

/// One mode's save slots in one folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slots {
    dir: PathBuf,
    mode: Mode,
}

/// A listed slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slot {
    pub number: u32,
    /// The file the scan found; loading checks both, the preferred one
    /// first.
    pub kind: SlotKind,
    /// What the save's player data says.
    pub info: SlotInfo,
}

impl Slots {
    /// The story's slots in `dir`.
    pub fn story(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            mode: Mode::Story,
        }
    }

    /// The arena's slots in `dir`.
    pub fn arena(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            mode: Mode::Arena,
        }
    }

    /// The folder.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The file name prefix.
    pub fn prefix(&self) -> &'static str {
        match self.mode {
            Mode::Story => save_package::PREFIX_STORY,
            Mode::Arena => save_package::PREFIX_ARENA,
        }
    }

    /// The `Exanima.set` key the order is kept under.
    pub fn order_key(&self) -> u32 {
        match self.mode {
            Mode::Story => save_chunk::STORY_ORDER_KEY,
            Mode::Arena => save_chunk::ARENA_ORDER_KEY,
        }
    }

    /// A slot's file.
    pub fn file(&self, number: u32, kind: SlotKind) -> PathBuf {
        self.dir
            .join(save_package::slot_file_name(self.prefix(), number, kind))
    }

    /// Whether a checkpoint file for the slot exists. The original's test
    /// only opens the file; its magic read has no effect on the result.
    pub fn has_checkpoint(&self, number: u32) -> bool {
        fs::metadata(self.file(number, SlotKind::Checkpoint))
            .map(|meta| meta.is_file())
            .unwrap_or(false)
    }

    /// Every candidate found by file name, in directory order, the first
    /// file per number only.
    fn candidates(&self) -> io::Result<Vec<(u32, SlotKind)>> {
        let mut out: Vec<(u32, SlotKind)> = Vec::new();
        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            let Some((number, kind)) = save_package::parse_slot_file(&name, self.prefix()) else {
                continue;
            };
            if out.iter().any(|(seen, _)| *seen == number) {
                continue;
            }
            out.push((number, kind));
        }
        Ok(out)
    }

    /// The stored order, when the settings store has one.
    fn stored_order(&self) -> Option<Vec<u32>> {
        let bytes = fs::read(self.dir.join(STORE_FILE)).ok()?;
        let loaded = SettingsStore::load(bytes);
        let raw = loaded.store.get(&[self.order_key()])?;
        Some(
            raw.as_chunks::<4>()
                .0
                .iter()
                .map(|word| u32::from_le_bytes(*word))
                .collect(),
        )
    }

    /// Writes the order, leaving it alone for fewer than two slots as the
    /// original does.
    fn store_order(&self, numbers: &[u32]) -> io::Result<()> {
        let Some(order) = save_chunk::order_to_store(numbers) else {
            return Ok(());
        };
        let path = self.dir.join(STORE_FILE);
        let mut store = match fs::read(&path) {
            Ok(bytes) => SettingsStore::load(bytes).store,
            Err(_) => SettingsStore::new(),
        };
        let payload: Vec<u8> = order.iter().flat_map(|n| n.to_le_bytes()).collect();
        let _ = store.set(&[self.order_key()], &payload);
        if let Some(bytes) = store.save() {
            fs::write(path, bytes)?;
        }
        Ok(())
    }

    /// The valid slots, in the most-recently-used order. A candidate the
    /// mode refuses is deleted, both files with their checkpoint.
    pub fn list(&self) -> io::Result<Vec<Slot>> {
        let mut valid = Vec::new();
        let mut invalid = Vec::new();
        for (number, kind) in self.candidates()? {
            let info = fs::read(self.file(number, kind))
                .ok()
                .and_then(|bytes| session::slot_info(&bytes).ok())
                .filter(|info| self.accepts(info));
            match info {
                Some(info) => valid.push(Slot { number, kind, info }),
                None => invalid.push(number),
            }
        }
        for number in invalid {
            self.delete(number)?;
        }
        let numbers: Vec<u32> = valid.iter().map(|slot| slot.number).collect();
        let stored = self.stored_order().unwrap_or_default();
        let ordered = save_chunk::slot_order(&numbers, &stored);
        Ok(ordered
            .into_iter()
            .filter_map(|number| valid.iter().find(|slot| slot.number == number).cloned())
            .collect())
    }

    fn accepts(&self, info: &SlotInfo) -> bool {
        matches!(
            (self.mode, info.flavour),
            (Mode::Story, save_package::Flavour::Story)
                | (Mode::Arena, save_package::Flavour::Arena)
        )
    }

    /// The valid slot numbers, in the list's order.
    pub fn numbers(&self) -> io::Result<Vec<u32>> {
        Ok(self.list()?.into_iter().map(|slot| slot.number).collect())
    }

    /// Reads a slot, preferring the checkpoint or the save. Loading from a
    /// checkpoint copies it back over the save, as the original does once
    /// the level is up.
    pub fn load(
        &self,
        number: u32,
        prefer_checkpoint: bool,
    ) -> Result<(Session, SlotKind), SaveError> {
        let order = if prefer_checkpoint {
            [SlotKind::Checkpoint, SlotKind::Save]
        } else {
            [SlotKind::Save, SlotKind::Checkpoint]
        };
        for kind in order {
            let path = self.file(number, kind);
            let Ok(bytes) = fs::read(&path) else {
                continue;
            };
            let Ok(session) = session::read(&bytes) else {
                continue;
            };
            self.touch(number);
            if kind == SlotKind::Checkpoint {
                let _ = fs::copy(&path, self.file(number, SlotKind::Save));
            }
            return Ok((session, kind));
        }
        Err(SaveError::Malformed)
    }

    /// Moves a slot to the front of the stored order.
    fn touch(&self, number: u32) {
        let Ok(mut numbers) = self.numbers() else {
            return;
        };
        numbers.retain(|seen| *seen != number);
        numbers.insert(0, number);
        let _ = self.store_order(&numbers);
    }

    /// Writes a session to a slot's save file, keeping the sections that are
    /// already there (the original updates the package in place) and the
    /// character placement when the session has none.
    pub fn save(&self, number: u32, session: &Session) -> io::Result<()> {
        let path = self.file(number, SlotKind::Save);
        let mut package = match fs::read(&path) {
            Ok(bytes) => PackageImage::read(&bytes).unwrap_or_default(),
            Err(_) => PackageImage::new(),
        };
        let previous = package.section(PLAYER_DATA).map(<[u8]>::to_vec);
        package.put_section(
            PLAYER_DATA,
            &session::player_section_after(session, previous.as_deref()),
        );
        fs::write(path, package.to_bytes())
    }

    /// Creates a slot: the lowest free number, at the front of the list,
    /// written to `<prefix>NNN.rsg`.
    pub fn create(&self, session: &Session) -> io::Result<u32> {
        let mut numbers = self.numbers()?;
        let number = save_chunk::free_slot_number(&numbers);
        save_chunk::insert_slot(&mut numbers, number);
        self.save(number, session)?;
        self.store_order(&numbers)?;
        Ok(number)
    }

    /// Deletes both of a slot's files.
    pub fn delete(&self, number: u32) -> io::Result<()> {
        for kind in [SlotKind::Save, SlotKind::Checkpoint] {
            match fs::remove_file(self.file(number, kind)) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    /// Writes a checkpoint when there is none: saves the slot, then copies
    /// the save over the checkpoint. Returns whether it wrote one.
    pub fn checkpoint(&self, number: u32, session: &Session) -> io::Result<bool> {
        if !save_chunk::checkpoint_due(self.has_checkpoint(number)) {
            return Ok(false);
        }
        self.save(number, session)?;
        fs::copy(
            self.file(number, SlotKind::Save),
            self.file(number, SlotKind::Checkpoint),
        )?;
        Ok(true)
    }

    /// A manual save: writes the session, then copies it over the
    /// checkpoint whether or not one exists (the original's save button
    /// refreshes the checkpoint unconditionally).
    pub fn save_and_checkpoint(&self, number: u32, session: &Session) -> io::Result<()> {
        self.save(number, session)?;
        fs::copy(
            self.file(number, SlotKind::Save),
            self.file(number, SlotKind::Checkpoint),
        )?;
        Ok(())
    }

    /// The permadeath rule: a dead player's slot is dropped from the list
    /// when it has no checkpoint. The files stay, as the original's routine
    /// only edits the list.
    pub fn drops_after_death(&self, number: u32, health: f32) -> bool {
        if save_chunk::save_allowed(health, false) {
            return false;
        }
        save_chunk::dead_slot_without_checkpoint(number, self.has_checkpoint(number)).is_some()
    }

    /// The player-data signature the mode writes now.
    pub fn signature(&self) -> u32 {
        match self.mode {
            Mode::Story => save_package::STORY_SIGNATURE,
            Mode::Arena => save_package::ARENA_SIGNATURE,
        }
    }

    /// Whether the folder has at least one valid slot.
    pub fn any(&self) -> io::Result<bool> {
        Ok(!self.list()?.is_empty())
    }

    /// Whether a slot's save holds a section for this locale, i.e. the level
    /// was visited. The original falls back to the game archive's
    /// `<locale>.rfc` when it is not.
    pub fn has_saved_level(&self, number: u32, locale: &str) -> bool {
        let Ok(bytes) = fs::read(self.file(number, SlotKind::Save)) else {
            return false;
        };
        let Ok(package) = PackageImage::read(&bytes) else {
            return false;
        };
        package.has_section(&save_package::name(locale))
    }

    /// The transition save: when the save has a story in progress (an
    /// `exanima04` section) and already holds the target locale, save the
    /// session and copy it over the checkpoint, which is refreshed. Returns
    /// whether it saved.
    pub fn transition_save(
        &self,
        number: u32,
        locale: &str,
        session: &Session,
    ) -> io::Result<bool> {
        let path = self.file(number, SlotKind::Save);
        let Ok(bytes) = fs::read(&path) else {
            return Ok(false);
        };
        let Ok(package) = PackageImage::read(&bytes) else {
            return Ok(false);
        };
        if !package.has_section(&save_package::name("exanima04"))
            || !package.has_section(&save_package::name(locale))
        {
            return Ok(false);
        }
        self.save(number, session)?;
        fs::copy(&path, self.file(number, SlotKind::Checkpoint))?;
        Ok(true)
    }

    /// The start-up legacy pass: every `.rsg` whose player data signature is
    /// below the mode's current one is deleted. A file whose signature
    /// cannot be read is left alone, and checkpoints are never touched.
    pub fn remove_legacy(&self) -> io::Result<usize> {
        let mut numbers = Vec::new();
        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if let Some((number, SlotKind::Save)) =
                save_package::parse_slot_file(&name, self.prefix())
            {
                numbers.push(number);
            }
        }
        let mut removed = 0;
        for number in numbers {
            let path = self.file(number, SlotKind::Save);
            let Ok(bytes) = fs::read(&path) else {
                continue;
            };
            let Ok(package) = PackageImage::read(&bytes) else {
                continue;
            };
            let Some(section) = package.section(PLAYER_DATA) else {
                continue;
            };
            let Some(word) = section.get(0..4) else {
                continue;
            };
            let signature = u32::from_le_bytes(word.try_into().unwrap());
            if signature < self.signature() {
                fs::remove_file(&path)?;
                removed += 1;
            }
        }
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::career::Profession;
    use exa_formats::save_package::PlayerData;

    fn folder(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("exarust-slots-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn arena(name: &str) -> Session {
        let mut session = Session::career(name);
        session.company.hire("Bram", Profession::Combatant).unwrap();
        session.clock = 12;
        session
    }

    #[test]
    fn a_created_slot_takes_the_lowest_free_number() {
        let dir = folder("create");
        let slots = Slots::arena(&dir);
        assert_eq!(slots.create(&arena("Ash")).unwrap(), 1);
        assert_eq!(slots.create(&arena("Bram")).unwrap(), 2);
        assert_eq!(slots.create(&arena("Cid")).unwrap(), 3);
        assert_eq!(slots.numbers().unwrap(), vec![3, 2, 1]);
        assert!(slots.file(2, SlotKind::Save).exists());
        // The second slot keeps its number when the first is deleted.
        slots.delete(1).unwrap();
        assert_eq!(slots.numbers().unwrap(), vec![3, 2]);
        assert_eq!(slots.create(&arena("Dee")).unwrap(), 1);
        assert_eq!(slots.numbers().unwrap(), vec![1, 3, 2]);
        let (session, kind) = slots.load(2, false).unwrap();
        assert_eq!(kind, SlotKind::Save);
        assert_eq!(session.company.name, "Bram");
        assert_eq!(session.clock, 12);
        // Loading moved slot 2 to the front, and the order survives a
        // fresh folder handle.
        assert_eq!(slots.numbers().unwrap(), vec![2, 1, 3]);
        let reloaded = Slots::arena(&dir);
        assert_eq!(reloaded.numbers().unwrap(), vec![2, 1, 3]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_checkpoint_is_written_once_and_copied_back_over_the_save() {
        let dir = folder("checkpoint");
        let slots = Slots::arena(&dir);
        let number = slots.create(&arena("Ash")).unwrap();
        assert!(!slots.has_checkpoint(number));
        let mut later = arena("Ash");
        later.clock = 99;
        assert!(slots.checkpoint(number, &later).unwrap());
        assert!(slots.has_checkpoint(number));
        assert!(!slots.checkpoint(number, &later).unwrap());
        // Change the save, then load the checkpoint: it wins and overwrites
        // the save.
        slots.save(number, &arena("Ash")).unwrap();
        let (loaded, kind) = slots.load(number, true).unwrap();
        assert_eq!(kind, SlotKind::Checkpoint);
        assert_eq!(loaded.clock, 99);
        let (loaded, kind) = slots.load(number, false).unwrap();
        assert_eq!(kind, SlotKind::Save);
        assert_eq!(loaded.clock, 99);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_invalid_slot_is_deleted_and_other_modes_are_left_out() {
        let dir = folder("invalid");
        let slots = Slots::story(&dir);
        let number = slots.create(&Session::story()).unwrap();
        // A file of the right name that is not a package.
        fs::write(slots.file(7, SlotKind::Save), b"garbage").unwrap();
        // A valid arena save in the story folder is not a story slot.
        let arena = Slots::arena(&dir);
        let arena_number = arena.create(&arena_session()).unwrap();
        let listed = slots.list().unwrap();
        assert_eq!(
            listed.iter().map(|slot| slot.number).collect::<Vec<_>>(),
            vec![number]
        );
        assert!(!slots.file(7, SlotKind::Save).exists());
        assert!(arena.file(arena_number, SlotKind::Save).exists());
        let _ = fs::remove_dir_all(&dir);
    }

    fn arena_session() -> Session {
        Session::career("Rival")
    }

    #[test]
    fn a_dead_player_without_a_checkpoint_is_dropped_from_the_list() {
        let dir = folder("death");
        let slots = Slots::arena(&dir);
        let number = slots.create(&arena("Ash")).unwrap();
        assert!(!slots.drops_after_death(number, 1.0));
        assert!(!slots.drops_after_death(number, 0.01));
        assert!(slots.drops_after_death(number, 0.009));
        slots.checkpoint(number, &arena("Ash")).unwrap();
        assert!(!slots.drops_after_death(number, 0.0));
        assert!(slots.file(number, SlotKind::Save).exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_legacy_pass_deletes_only_older_saves() {
        let dir = folder("legacy");
        let slots = Slots::story(&dir);
        let current = slots.create(&Session::story()).unwrap();
        let older = slots.create(&Session::story()).unwrap();
        let mut package = PackageImage::new();
        package.put_section(PLAYER_DATA, &0x6F5A_EA13u32.to_le_bytes());
        fs::write(slots.file(older, SlotKind::Save), package.to_bytes()).unwrap();
        // A checkpoint is left alone even when it is old.
        fs::copy(
            slots.file(older, SlotKind::Save),
            slots.file(older, SlotKind::Checkpoint),
        )
        .unwrap();
        assert_eq!(slots.remove_legacy().unwrap(), 1);
        assert!(slots.file(current, SlotKind::Save).exists());
        assert!(!slots.file(older, SlotKind::Save).exists());
        assert!(slots.file(older, SlotKind::Checkpoint).exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_transition_save_refreshes_the_checkpoint_when_the_level_is_saved() {
        let dir = folder("transition");
        let slots = Slots::story(&dir);
        let session = Session::story();
        let number = slots.create(&session).unwrap();
        // A fresh save has no level sections.
        assert!(!slots.has_saved_level(number, "exanima02"));
        assert!(
            !slots
                .transition_save(number, "exanima02", &session)
                .unwrap()
        );
        // Add the story marker and the target level as an empty package
        // section would be.
        let mut package =
            PackageImage::read(&fs::read(slots.file(number, SlotKind::Save)).unwrap()).unwrap();
        package.put_section(&save_package::name("exanima04"), &[]);
        package.put_section(&save_package::name("exanima02"), &[]);
        fs::write(slots.file(number, SlotKind::Save), package.to_bytes()).unwrap();
        assert!(slots.has_saved_level(number, "exanima02"));
        let mut later = session;
        later.clock = 55;
        assert!(slots.transition_save(number, "exanima02", &later).unwrap());
        assert!(slots.has_checkpoint(number));
        let (loaded, kind) = slots.load(number, true).unwrap();
        assert_eq!(kind, SlotKind::Checkpoint);
        assert_eq!(loaded.clock, 55);
        // Without the progress marker nothing is written.
        let mut bare = PackageImage::new();
        bare.put_section(&save_package::name("exanima02"), &[]);
        fs::write(slots.file(number, SlotKind::Save), bare.to_bytes()).unwrap();
        assert!(!slots.transition_save(number, "exanima02", &later).unwrap());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn saving_keeps_the_character_placement_a_save_already_has() {
        let dir = folder("character");
        let slots = Slots::arena(&dir);
        let session = arena("Ash");
        let number = slots.create(&session).unwrap();
        let path = slots.file(number, SlotKind::Save);
        let mut package = PackageImage::read(&fs::read(&path).unwrap()).unwrap();
        let mut data = PlayerData::parse(
            package.section(PLAYER_DATA).unwrap(),
            save_package::Flavour::Arena,
        )
        .unwrap();
        data.character = Some(vec![7, 7, 7]);
        package.put_section(PLAYER_DATA, &data.write());
        fs::write(&path, package.to_bytes()).unwrap();
        // A save without a character of its own keeps the old placement.
        let mut later = arena("Ash");
        later.clock = 3;
        slots.save(number, &later).unwrap();
        let package = PackageImage::read(&fs::read(&path).unwrap()).unwrap();
        let data = PlayerData::parse(
            package.section(PLAYER_DATA).unwrap(),
            save_package::Flavour::Arena,
        )
        .unwrap();
        assert_eq!(data.character, Some(vec![7, 7, 7]));
        assert_eq!(data.header.clock(), 3);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_manual_save_refreshes_an_existing_checkpoint() {
        let dir = folder("manual");
        let slots = Slots::arena(&dir);
        let number = slots.create(&arena("Ash")).unwrap();
        let mut first = arena("Ash");
        first.clock = 10;
        slots.save_and_checkpoint(number, &first).unwrap();
        assert!(slots.has_checkpoint(number));
        let mut second = arena("Ash");
        second.clock = 20;
        slots.save_and_checkpoint(number, &second).unwrap();
        let (loaded, kind) = slots.load(number, true).unwrap();
        assert_eq!(kind, SlotKind::Checkpoint);
        assert_eq!(loaded.clock, 20);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_slot_names_come_from_the_mode_prefix() {
        let dir = folder("names");
        let story = Slots::story(&dir);
        let arena = Slots::arena(&dir);
        assert_eq!(story.prefix(), "Exanima");
        assert_eq!(arena.prefix(), "Arena");
        assert_eq!(
            story.file(3, SlotKind::Save).file_name().unwrap(),
            "Exanima003.rsg"
        );
        assert_eq!(
            arena.file(12, SlotKind::Checkpoint).file_name().unwrap(),
            "Arena012.rcp"
        );
        assert_eq!(story.order_key(), exa_formats::save_chunk::STORY_ORDER_KEY);
        assert_eq!(arena.order_key(), exa_formats::save_chunk::ARENA_ORDER_KEY);
        let _ = fs::remove_dir_all(&dir);
    }
}
