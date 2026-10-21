//! Save slots on disk: the save folder's `ExanimaNNN.rsg` / `ArenaNNN.rsg`
//! files, their checkpoints, the most-recently-used order kept in
//! `Exanima.set`, deleting a slot, and the permadeath rule.
//!
//! The original scans the folder with `FindNextFile`, keeps the first listed
//! file per number, opens each package (which resets a file with a bad
//! magic to an empty package) and runs the mode's player-data reader on it:
//! the story deletes both files of a slot whose signature is older than its
//! own, the arena deletes nothing, and a slot that does not read is only
//! left out. New slots take the lowest free number and go to the front of
//! the list. Loading a slot moves it to the front and opens the first file
//! that exists in the asked order; loading from a checkpoint then copies the
//! checkpoint over the save. A checkpoint is only written when none exists;
//! a death deletes the save and drops the slot from the list when it has no
//! checkpoint, so a checkpoint is what a dead player comes back from.
//!
//! Evidence: `docs/formats/SAVES.md`.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use exa_formats::save_chunk;
use exa_formats::save_package::{self, PLAYER_DATA, PackageImage, PlayerData, SlotKind};
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

/// What a scan does with one slot file.
enum Verdict {
    /// List it.
    Keep(SlotInfo),
    /// Delete both of its files.
    Delete,
    /// Leave it out and on disk.
    Skip,
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

    /// Every slot file found by name, in directory order.
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
            if let Some(found) = save_package::parse_slot_file(&name, self.prefix()) {
                out.push(found);
            }
        }
        Ok(out)
    }

    /// Opens a package file as the original does: a missing file is created,
    /// and a file shorter than nine bytes or with another magic gets a fresh
    /// package header (magic, count 1, the sentinel) written over its start,
    /// without truncating it. A file with the magic whose directory does not
    /// fit reads as an empty package here and is left as it is.
    pub fn open(path: &Path) -> io::Result<PackageImage> {
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(error) => return Err(error),
        };
        let magic_ok = bytes.len() >= 9 && bytes[0..4] == save_package::PACKAGE_MAGIC.to_le_bytes();
        if !magic_ok {
            let fresh = PackageImage::new().to_bytes();
            let mut out = bytes;
            if out.len() < fresh.len() {
                out.resize(fresh.len(), 0);
            }
            out[..fresh.len()].copy_from_slice(&fresh);
            fs::write(path, out)?;
            return Ok(PackageImage::new());
        }
        Ok(PackageImage::read(&bytes).unwrap_or_default())
    }

    /// What the mode's player-data reader makes of a package during a scan
    /// (it reads no chunks then): the story refuses a signature older than
    /// its own, which deletes the slot, and reads any other as a story; the
    /// arena takes only its exact signature and refuses nothing.
    fn verdict(&self, package: &PackageImage) -> Verdict {
        let Some(section) = package.section(PLAYER_DATA) else {
            return Verdict::Skip;
        };
        let Some(word) = section.get(0..4) else {
            return Verdict::Skip;
        };
        let signature = u32::from_le_bytes(word.try_into().unwrap());
        let flavour = match self.mode {
            Mode::Story if signature < save_package::STORY_SIGNATURE => return Verdict::Delete,
            Mode::Story => save_package::Flavour::Story,
            Mode::Arena if signature != save_package::ARENA_SIGNATURE => return Verdict::Skip,
            Mode::Arena => save_package::Flavour::Arena,
        };
        match PlayerData::parse(section, flavour) {
            Ok(data) => Verdict::Keep(session::slot_info_of(&data)),
            Err(_) => Verdict::Skip,
        }
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

    /// The valid slots, in the most-recently-used order. Files are taken in
    /// directory order; a number already listed is skipped. A slot the
    /// story refuses for its old signature is deleted, both files; one that
    /// does not read is left out and left on disk.
    ///
    /// The original also requires the player's record to be found in the
    /// character databases; Exarust's own saves carry no placement, so a
    /// readable header is enough here.
    pub fn list(&self) -> io::Result<Vec<Slot>> {
        let mut valid: Vec<Slot> = Vec::new();
        for (number, kind) in self.candidates()? {
            if valid.iter().any(|slot| slot.number == number) {
                continue;
            }
            let path = self.file(number, kind);
            if !path.is_file() {
                continue;
            }
            match self.verdict(&Self::open(&path)?) {
                Verdict::Keep(info) => valid.push(Slot { number, kind, info }),
                Verdict::Delete => self.delete(number)?,
                Verdict::Skip => {}
            }
        }
        let numbers: Vec<u32> = valid.iter().map(|slot| slot.number).collect();
        let stored = self.stored_order().unwrap_or_default();
        let ordered = save_chunk::slot_order(&numbers, &stored);
        Ok(ordered
            .into_iter()
            .filter_map(|number| valid.iter().find(|slot| slot.number == number).cloned())
            .collect())
    }

    /// The valid slot numbers, in the list's order.
    pub fn numbers(&self) -> io::Result<Vec<u32>> {
        Ok(self.list()?.into_iter().map(|slot| slot.number).collect())
    }

    /// Reads a slot. The slot first moves to the front of the order; then
    /// the first of its two files that exists, the checkpoint first when
    /// asked, is opened and read; a file that exists but does not read is
    /// an error, the other file is not tried. Loading from a checkpoint
    /// copies it back over the save, as the original does once the level is
    /// up.
    pub fn load(
        &self,
        number: u32,
        prefer_checkpoint: bool,
    ) -> Result<(Session, SlotKind), SaveError> {
        self.touch(number);
        let order = if prefer_checkpoint {
            [SlotKind::Checkpoint, SlotKind::Save]
        } else {
            [SlotKind::Save, SlotKind::Checkpoint]
        };
        let kind = order
            .into_iter()
            .find(|kind| self.file(number, *kind).is_file())
            .ok_or(SaveError::Malformed)?;
        let path = self.file(number, kind);
        Self::open(&path).map_err(|_| SaveError::Malformed)?;
        let bytes = fs::read(&path).map_err(|_| SaveError::Malformed)?;
        let session = session::read(&bytes)?;
        if kind == SlotKind::Checkpoint {
            let _ = fs::copy(&path, self.file(number, SlotKind::Save));
        }
        Ok((session, kind))
    }

    /// Moves a listed slot to the front of the stored order; a number that
    /// is not listed changes nothing.
    fn touch(&self, number: u32) {
        let Ok(mut numbers) = self.numbers() else {
            return;
        };
        let Some(at) = numbers.iter().position(|seen| *seen == number) else {
            return;
        };
        numbers.remove(at);
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

    /// The permadeath rule, run when the player is dead (health below 0.01
    /// in double): the slot's save file is deleted, and the slot is dropped
    /// from the list (it no longer has a file) unless a checkpoint exists,
    /// which the next load then comes back from. Returns whether the slot
    /// was dropped; a living player changes nothing.
    pub fn drops_after_death(&self, number: u32, health: f32) -> io::Result<bool> {
        if number == 0 || save_chunk::save_allowed(health, false) {
            return Ok(false);
        }
        match fs::remove_file(self.file(number, SlotKind::Save)) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        Ok(save_chunk::dead_slot_without_checkpoint(number, self.has_checkpoint(number)).is_some())
    }

    /// A save that checks the player's health first, as the save the arena
    /// and the death screens ask for: a dead player is not saved and the
    /// permadeath rule runs instead. Returns whether it saved.
    pub fn save_checked(&self, number: u32, session: &Session, health: f32) -> io::Result<bool> {
        if !save_chunk::save_allowed(health, false) {
            self.drops_after_death(number, health)?;
            return Ok(false);
        }
        self.save(number, session)?;
        Ok(true)
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

    /// The transition save: when the save has **neither** an `exanima04`
    /// section **nor** one for the target locale (the early levels, entered
    /// for the first time), save the session and copy it over the
    /// checkpoint, which is refreshed. A save with either section is left
    /// alone. Returns whether it saved.
    pub fn transition_save(
        &self,
        number: u32,
        locale: &str,
        session: &Session,
    ) -> io::Result<bool> {
        let path = self.file(number, SlotKind::Save);
        let package = fs::read(&path)
            .ok()
            .and_then(|bytes| PackageImage::read(&bytes).ok())
            .unwrap_or_default();
        if package.has_section(&save_package::name("exanima04"))
            || package.has_section(&save_package::name(locale))
        {
            return Ok(false);
        }
        self.save(number, session)?;
        fs::copy(&path, self.file(number, SlotKind::Checkpoint))?;
        Ok(true)
    }

    /// The start-up legacy pass: every `.rsg` is opened (a bad magic gets a
    /// fresh header, as any open does) and deleted when its player data
    /// signature is below the mode's current one, compared unsigned. A file
    /// without player data is left alone, and checkpoints are never touched.
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
            let package = Self::open(&path)?;
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
    fn a_scan_deletes_only_old_story_signatures() {
        let dir = folder("invalid");
        let slots = Slots::story(&dir);
        let number = slots.create(&Session::story()).unwrap();
        // A file of the right name that is not a package: left out, left on
        // disk, and opening it wrote a fresh package header over its start.
        fs::write(slots.file(7, SlotKind::Save), b"garbage").unwrap();
        // An older story signature: deleted, checkpoint included.
        let mut old = PackageImage::new();
        old.put_section(PLAYER_DATA, &0x6F5A_EA13u32.to_le_bytes());
        fs::write(slots.file(8, SlotKind::Save), old.to_bytes()).unwrap();
        fs::write(slots.file(8, SlotKind::Checkpoint), old.to_bytes()).unwrap();
        // A package without player data: left out and left alone.
        fs::write(
            slots.file(9, SlotKind::Save),
            PackageImage::new().to_bytes(),
        )
        .unwrap();
        // An arena player data under a story name: its signature is above
        // the story's, so the story reads it and lists it.
        let mut arena_data = PackageImage::new();
        arena_data.put_section(
            PLAYER_DATA,
            &PlayerData::arena(save_package::name("arenahub")).write(),
        );
        fs::write(slots.file(10, SlotKind::Save), arena_data.to_bytes()).unwrap();
        let mut listed = slots.numbers().unwrap();
        listed.sort_unstable();
        assert_eq!(listed, vec![number, 10]);
        let reset = fs::read(slots.file(7, SlotKind::Save)).unwrap();
        assert_eq!(reset.len(), 32);
        assert_eq!(&reset[0..4], &save_package::PACKAGE_MAGIC.to_le_bytes());
        assert!(!slots.file(8, SlotKind::Save).exists());
        assert!(!slots.file(8, SlotKind::Checkpoint).exists());
        assert!(slots.file(9, SlotKind::Save).exists());

        // The arena takes its exact signature and deletes nothing.
        let arena = Slots::arena(&dir);
        let mut story_data = PackageImage::new();
        story_data.put_section(
            PLAYER_DATA,
            &PlayerData::story(save_package::name("exanima01")).write(),
        );
        fs::write(arena.file(3, SlotKind::Save), story_data.to_bytes()).unwrap();
        fs::write(arena.file(4, SlotKind::Save), old.to_bytes()).unwrap();
        let arena_number = arena.create(&arena_session()).unwrap();
        assert_eq!(arena.numbers().unwrap(), vec![arena_number]);
        assert!(arena.file(3, SlotKind::Save).exists());
        assert!(arena.file(4, SlotKind::Save).exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_number_is_listed_once_from_the_first_file_that_reads() {
        let dir = folder("first");
        let slots = Slots::story(&dir);
        // Only a checkpoint: the slot is listed from it.
        fs::write(
            slots.file(5, SlotKind::Checkpoint),
            session::write(&Session::story()),
        )
        .unwrap();
        let listed = slots.list().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(
            (listed[0].number, listed[0].kind),
            (5, SlotKind::Checkpoint)
        );
        // With both files the number still appears once.
        fs::write(
            slots.file(5, SlotKind::Save),
            session::write(&Session::story()),
        )
        .unwrap();
        assert_eq!(slots.numbers().unwrap(), vec![5]);
        let _ = fs::remove_dir_all(&dir);
    }

    fn arena_session() -> Session {
        Session::career("Rival")
    }

    #[test]
    fn a_death_deletes_the_save_and_a_checkpoint_brings_it_back() {
        let dir = folder("death");
        let slots = Slots::arena(&dir);
        let number = slots.create(&arena("Ash")).unwrap();
        // A living player changes nothing.
        assert!(!slots.drops_after_death(number, 1.0).unwrap());
        assert!(!slots.drops_after_death(number, 0.010_001).unwrap());
        assert!(slots.file(number, SlotKind::Save).exists());
        // Without a checkpoint the save goes and the slot is dropped.
        assert!(slots.drops_after_death(number, 0.01).unwrap());
        assert!(!slots.file(number, SlotKind::Save).exists());
        assert!(slots.numbers().unwrap().is_empty());

        // With a checkpoint the save still goes, but the slot stays listed
        // from its checkpoint, and loading copies it back.
        let number = slots.create(&arena("Bram")).unwrap();
        let mut marked = arena("Bram");
        marked.clock = 77;
        assert!(slots.checkpoint(number, &marked).unwrap());
        assert!(!slots.drops_after_death(number, 0.0).unwrap());
        assert!(!slots.file(number, SlotKind::Save).exists());
        let listed = slots.list().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].kind, SlotKind::Checkpoint);
        let (loaded, kind) = slots.load(number, false).unwrap();
        assert_eq!(kind, SlotKind::Checkpoint);
        assert_eq!(loaded.clock, 77);
        assert!(slots.file(number, SlotKind::Save).exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_checked_save_of_a_dead_player_writes_nothing() {
        let dir = folder("checked");
        let slots = Slots::arena(&dir);
        let number = slots.create(&arena("Ash")).unwrap();
        let mut later = arena("Ash");
        later.clock = 5;
        assert!(slots.save_checked(number, &later, 0.5).unwrap());
        assert_eq!(slots.load(number, false).unwrap().0.clock, 5);
        later.clock = 6;
        assert!(!slots.save_checked(number, &later, 0.0).unwrap());
        assert!(!slots.file(number, SlotKind::Save).exists());
        // An unchecked save writes whatever the health.
        slots.save(number, &later).unwrap();
        assert_eq!(slots.load(number, false).unwrap().0.clock, 6);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_load_opens_the_first_file_that_exists_and_does_not_fall_back() {
        let dir = folder("prefer");
        let slots = Slots::story(&dir);
        let number = slots.create(&Session::story()).unwrap();
        slots.checkpoint(number, &Session::story()).unwrap();
        // A save that exists but is not a session is an error even though
        // the checkpoint reads.
        let mut package = PackageImage::new();
        package.put_section(
            PLAYER_DATA,
            &PlayerData::story(save_package::name("exanima01")).write(),
        );
        fs::write(slots.file(number, SlotKind::Save), package.to_bytes()).unwrap();
        assert_eq!(slots.load(number, false), Err(SaveError::Malformed));
        assert_eq!(slots.load(number, true).unwrap().1, SlotKind::Checkpoint);
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
    fn a_transition_save_runs_only_on_a_first_visit_before_the_fourth_level() {
        let dir = folder("transition");
        let slots = Slots::story(&dir);
        let session = Session::story();
        let number = slots.create(&session).unwrap();
        // A fresh save has neither the target level nor `exanima04`: the
        // transition saves and refreshes the checkpoint.
        assert!(!slots.has_saved_level(number, "exanima02"));
        let mut later = session.clone();
        later.clock = 55;
        assert!(slots.transition_save(number, "exanima02", &later).unwrap());
        assert!(slots.has_checkpoint(number));
        let (loaded, kind) = slots.load(number, true).unwrap();
        assert_eq!(kind, SlotKind::Checkpoint);
        assert_eq!(loaded.clock, 55);
        // A level the save already holds: nothing is written.
        let mut package =
            PackageImage::read(&fs::read(slots.file(number, SlotKind::Save)).unwrap()).unwrap();
        package.put_section(&save_package::name("exanima02"), &[]);
        fs::write(slots.file(number, SlotKind::Save), package.to_bytes()).unwrap();
        assert!(slots.has_saved_level(number, "exanima02"));
        later.clock = 66;
        assert!(!slots.transition_save(number, "exanima02", &later).unwrap());
        // Once `exanima04` is in the save no transition saves, even to a
        // new level.
        assert!(slots.transition_save(number, "exanima03", &later).unwrap());
        package.put_section(&save_package::name("exanima04"), &[]);
        fs::write(slots.file(number, SlotKind::Save), package.to_bytes()).unwrap();
        assert!(!slots.transition_save(number, "exanima05", &later).unwrap());
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
        data.character = Some(vec![7, 7, 7, 7]);
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
        assert_eq!(data.character, Some(vec![7, 7, 7, 7]));
        assert_eq!(data.header.clock(), 3);
        let info = session::slot_info(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(info.player_record, Some(0x0707_0707));
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
