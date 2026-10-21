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
use exa_formats::save_package::{
    self, Flavour, NAME_LEN, PLAYER_DATA, PackageImage, PlayerData, SlotKind,
};
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

/// The sections a save writes besides `#playerdata`, in the original's
/// save order: the world character database, the player data, the world
/// object database and the current level.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SaveWorld<'a> {
    /// The character database export; written only when it has records.
    pub world_characters: Option<&'a [u8]>,
    /// The object database export and the data manager's table; written
    /// only when the database has records.
    pub world_objects: Option<(&'a [u8], &'a [u8])>,
    /// The current level's locale and its exported bytes.
    pub level: Option<(&'a str, &'a [u8])>,
    /// The original's "saving disabled" flag: a set flag skips the whole
    /// save.
    pub disabled: bool,
}

/// A save package opened and decoded as a load does, before the session
/// layer restores its own payload.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedSave {
    /// The package with any version conversion already applied.
    pub package: PackageImage,
    /// The mode's player data, with the full mask applied.
    pub data: PlayerData,
    /// Which of the slot's two files was opened.
    pub kind: SlotKind,
    /// The story loader's forced locale after a mid-transition save
    /// (`exanima05`), if any.
    pub forced_locale: Option<[u8; NAME_LEN]>,
    /// The locale the load uses: the forced one, else the header's.
    pub level_locale: [u8; NAME_LEN],
    /// Whether the package holds the level's section; when it does not, the
    /// original loads `<locale>.rfc` from the game archive.
    pub has_level_section: bool,
    /// Whether the package holds a `#worldobjdb` section (else the object
    /// database is cleared).
    pub has_world_objects: bool,
    /// Whether the package holds a `#worldchrdb` section (else the
    /// character database is cleared).
    pub has_world_characters: bool,
    /// Whether an old arena save was converted and written back.
    pub converted: bool,
}

impl LoadedSave {
    /// The locale the load uses as text, cut at its first zero, for the
    /// `<locale>.rfc` archive fallback.
    pub fn level_locale_name(&self) -> String {
        save_package::name_text(&self.level_locale)
    }

    /// The player's character record, as the character screen loads it: the
    /// placement's reference names a record in the save's world character
    /// database. None when the reference points at the other database, the
    /// section is missing, or the record is not there.
    pub fn player_record(&self) -> Option<exa_formats::characters::CharacterRecord> {
        let reference = self.data.character_reference()?;
        if reference & save_package::CHARACTER_IN_WORLD_DB == 0 {
            return None;
        }
        let id = reference & exa_formats::characters::ID_MASK;
        let section = self.package.section(save_package::WORLD_CHARACTER_DB)?;
        let payload = save_package::first_chunk_payload(section)?;
        let records = exa_formats::characters::read_character_database(payload).ok()?;
        records
            .into_iter()
            .find(|(record_id, _)| record_id & exa_formats::characters::ID_MASK == id)
            .and_then(|(_, record)| record.ok())
    }
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

    /// Whether a slot file can be opened for reading, the original's
    /// existence test (`CreateFile` with read access); a directory or an
    /// unreadable file is not there.
    fn opens(&self, number: u32, kind: SlotKind) -> bool {
        let path = self.file(number, kind);
        fs::metadata(&path)
            .map(|meta| meta.is_file())
            .unwrap_or(false)
            && fs::File::open(path).is_ok()
    }

    /// Whether a checkpoint file for the slot exists. The original's test
    /// only opens the file for reading, so an unreadable file is not a
    /// checkpoint; a directory is not one either (Windows' create call
    /// without the backup flag fails on it).
    pub fn has_checkpoint(&self, number: u32) -> bool {
        self.opens(number, SlotKind::Checkpoint)
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
            Ok(mut data) => {
                // The scan's load mask is 0: the header and the character
                // are read, no chunk is dispatched.
                data.apply_mask(0);
                let mut info = session::slot_info_of(&data);
                info.player_name = session::player_name(package, &data);
                Verdict::Keep(info)
            }
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
    /// directory order; the **first** file for a number decides that
    /// number's fate, even when it leaves the slot out (the original
    /// appends the number before asking the mode's reader, so a skipped
    /// number's checkpoint is not tried afterwards). A slot the story
    /// refuses for its old signature is deleted, both files; one that does
    /// not read is left out and left on disk.
    ///
    /// The original also requires the player's record to be found in the
    /// character databases; Exarust's own saves carry no placement, so a
    /// readable header is enough here.
    pub fn list(&self) -> io::Result<Vec<Slot>> {
        self.scan(self.candidates()?)
    }

    /// Applies the scan to slot files already listed in the order the
    /// folder returned them. Split out so tests can pin the order.
    fn scan(&self, candidates: Vec<(u32, SlotKind)>) -> io::Result<Vec<Slot>> {
        let mut valid: Vec<Slot> = Vec::new();
        let mut seen: Vec<u32> = Vec::new();
        for (number, kind) in candidates {
            if seen.contains(&number) {
                continue;
            }
            seen.push(number);
            let path = self.file(number, kind);
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

    /// Opens and decodes the first existing file of a slot, as the original's
    /// full load does: the `.rsg` first (the `.rcp` first when
    /// `prefer_checkpoint`), a file that exists but does not read is an
    /// error and the other file is not tried. The mode's player data is
    /// read with the full mask; an old arena save is converted (its
    /// `arenahub` section dropped, its placement moved to the start, the
    /// skipped chunks removed) and the package is written back. A story
    /// save written mid-transition reports `exanima05` as its forced
    /// locale.
    ///
    /// The original also imports the world databases and the level here;
    /// those payloads belong to other units and stay in the package.
    pub fn load_package(
        &self,
        number: u32,
        prefer_checkpoint: bool,
    ) -> Result<LoadedSave, SaveError> {
        self.load_package_masked(number, prefer_checkpoint, !0)
    }

    /// The character screen's load, as the original's continue and recruit
    /// flows do it: the save is preferred, the checkpoint is taken only when
    /// asked for or when the save is missing, and the player data is read
    /// with mask `0x100010` (the journal and the global experience only).
    /// The character record the original then loads into the creator is not
    /// restored here; [`SlotInfo::player_name`] exposes its name.
    pub fn load_character_screen(
        &self,
        number: u32,
        prefer_checkpoint: bool,
    ) -> Result<LoadedSave, SaveError> {
        let save = self.opens(number, SlotKind::Save);
        let checkpoint = self.has_checkpoint(number);
        // The original's branch: with a checkpoint and either an explicit
        // checkpoint request or no save, open the checkpoint; otherwise a
        // save is required.
        let kind = if checkpoint && (prefer_checkpoint || !save) {
            SlotKind::Checkpoint
        } else if save {
            SlotKind::Save
        } else {
            return Err(SaveError::Malformed);
        };
        // The character-screen path does not move the slot to the front;
        // only the full load does.
        self.load_file(number, kind, PlayerData::CHARACTER_SCREEN_MASK)
    }

    fn load_package_masked(
        &self,
        number: u32,
        prefer_checkpoint: bool,
        mask: u32,
    ) -> Result<LoadedSave, SaveError> {
        self.touch(number);
        let order = if prefer_checkpoint {
            [SlotKind::Checkpoint, SlotKind::Save]
        } else {
            [SlotKind::Save, SlotKind::Checkpoint]
        };
        let kind = order
            .into_iter()
            .find(|kind| self.opens(number, *kind))
            .ok_or(SaveError::Malformed)?;
        let loaded = self.load_file(number, kind, mask);
        // A failed full load from a checkpoint switches the package back to
        // the save name, which the original's open creates when it is
        // missing. It does not copy the checkpoint over.
        if loaded.is_err() && kind == SlotKind::Checkpoint {
            let _ = Self::open(&self.file(number, SlotKind::Save));
        }
        loaded
    }

    /// Opens and decodes a slot file: the mode's signature check, the player
    /// data with `mask`, an arena version conversion written back, and the
    /// level's availability.
    fn load_file(&self, number: u32, kind: SlotKind, mask: u32) -> Result<LoadedSave, SaveError> {
        let path = self.file(number, kind);
        // Opening may reset a damaged header, so read the package from
        // memory and not from the bytes read before the open.
        let mut package = Self::open(&path).map_err(|_| SaveError::Malformed)?;
        let section = package.section(PLAYER_DATA).ok_or(SaveError::Malformed)?;
        let signature = section
            .get(0..4)
            .map(|word| u32::from_le_bytes(word.try_into().unwrap()))
            .ok_or(SaveError::Malformed)?;
        let flavour = match self.mode {
            Mode::Story if signature < save_package::STORY_SIGNATURE => {
                return Err(SaveError::Malformed);
            }
            Mode::Story => Flavour::Story,
            Mode::Arena if signature != save_package::ARENA_SIGNATURE => {
                return Err(SaveError::Malformed);
            }
            Mode::Arena => Flavour::Arena,
        };
        let mut data = PlayerData::parse(section, flavour).map_err(|_| SaveError::Malformed)?;
        data.apply_mask(mask);
        let mut converted = false;
        if flavour == Flavour::Arena && data.arena_data_is_old() {
            data.convert_arena();
            package.remove_section(&save_package::name("arenahub"));
            package.put_section(PLAYER_DATA, &data.write_ordered());
            fs::write(&path, package.to_bytes()).map_err(|_| SaveError::Malformed)?;
            converted = true;
        }
        let forced_locale = if flavour == Flavour::Story {
            data.story_transition_locale()
        } else {
            None
        };
        let level_locale = forced_locale.unwrap_or_else(|| data.header.locale());
        let has_level_section = package.has_section(&level_locale);
        let has_world_objects = package.has_section(save_package::WORLD_OBJECT_DB);
        let has_world_characters = package.has_section(save_package::WORLD_CHARACTER_DB);
        Ok(LoadedSave {
            package,
            data,
            kind,
            forced_locale,
            level_locale,
            has_level_section,
            has_world_objects,
            has_world_characters,
            converted,
        })
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
        let loaded = self.load_package(number, prefer_checkpoint)?;
        let session = session::read_package(&loaded.package)?;
        if loaded.kind == SlotKind::Checkpoint {
            let _ = fs::copy(
                self.file(number, SlotKind::Checkpoint),
                self.file(number, SlotKind::Save),
            );
        }
        Ok((session, loaded.kind))
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
    /// character placement when the session has none. Writes only
    /// `#playerdata`; [`Slots::save_full`] writes the other sections too.
    pub fn save(&self, number: u32, session: &Session) -> io::Result<()> {
        self.save_full(number, session, &SaveWorld::default())?;
        Ok(())
    }

    /// Writes every section of a save in the original's order: the world
    /// character database when it has records, `#playerdata`, the world
    /// object database when it has records, then the current level under
    /// its locale name. Sections already in the file that are not written
    /// this time stay where they are. Returns whether the save ran; the
    /// original's "saving disabled" flag skips it entirely.
    pub fn save_full(
        &self,
        number: u32,
        session: &Session,
        world: &SaveWorld<'_>,
    ) -> io::Result<bool> {
        if world.disabled {
            return Ok(false);
        }
        let path = self.file(number, SlotKind::Save);
        let mut package = match fs::read(&path) {
            Ok(bytes) => PackageImage::read(&bytes).unwrap_or_default(),
            Err(_) => PackageImage::new(),
        };
        if let Some(characters) = world.world_characters {
            package.put_section(
                save_package::WORLD_CHARACTER_DB,
                &save_package::world_character_db(characters),
            );
        }
        let previous = package.section(PLAYER_DATA).map(<[u8]>::to_vec);
        package.put_section(
            PLAYER_DATA,
            &session::player_section_after(session, previous.as_deref()),
        );
        if let Some((objects, data_manager)) = world.world_objects {
            package.put_section(
                save_package::WORLD_OBJECT_DB,
                &save_package::world_object_db(objects, data_manager),
            );
        }
        if let Some((locale, level)) = world.level {
            package.put_section(&save_package::name(locale), level);
        }
        fs::write(path, package.to_bytes())?;
        Ok(true)
    }

    /// Creates a slot the way the original does: the lowest free number,
    /// at the front of the list, with an empty package file (the save
    /// follows when the game saves). Returns the number.
    pub fn create_empty(&self) -> io::Result<u32> {
        let mut numbers = self.numbers()?;
        let number = save_chunk::free_slot_number(&numbers);
        save_chunk::insert_slot(&mut numbers, number);
        self.store_order(&numbers)?;
        Self::open(&self.file(number, SlotKind::Save))?;
        Ok(number)
    }

    /// Creates a slot and writes a session into it. The original's new-slot
    /// path only creates the empty package ([`Slots::create_empty`]) and
    /// saves later; this convenience does both.
    pub fn create(&self, session: &Session) -> io::Result<u32> {
        let number = self.create_empty()?;
        self.save(number, session)?;
        Ok(number)
    }

    /// Deletes the slot at `index` in the list, as the menu's delete does.
    /// Returns the number deleted.
    pub fn delete_index(&self, index: usize) -> io::Result<Option<u32>> {
        let numbers = self.numbers()?;
        let Some(number) = numbers.get(index).copied() else {
            return Ok(None);
        };
        self.delete(number)?;
        Ok(Some(number))
    }

    /// Deletes both of a slot's files, the save first as the original does.
    /// A file that cannot be deleted does not stop the other attempt; the
    /// first error is returned after both.
    pub fn delete(&self, number: u32) -> io::Result<()> {
        let mut first = None;
        for kind in [SlotKind::Save, SlotKind::Checkpoint] {
            match fs::remove_file(self.file(number, kind)) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => {
                    if first.is_none() {
                        first = Some(error);
                    }
                }
            }
        }
        match first {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    /// Writes a checkpoint when there is none: saves the slot, then copies
    /// the save over the checkpoint. Returns whether it wrote one.
    pub fn checkpoint(&self, number: u32, session: &Session) -> io::Result<bool> {
        self.checkpoint_full(number, session, &SaveWorld::default())
    }

    /// A checkpoint over a full save ([`Slots::save_full`]). With the
    /// "saving disabled" flag the save part is skipped but the old save is
    /// still copied over the checkpoint, as the original does.
    pub fn checkpoint_full(
        &self,
        number: u32,
        session: &Session,
        world: &SaveWorld<'_>,
    ) -> io::Result<bool> {
        if !save_chunk::checkpoint_due(self.has_checkpoint(number)) {
            return Ok(false);
        }
        self.save_full(number, session, world)?;
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
        self.save_and_checkpoint_full(number, session, &SaveWorld::default())?;
        Ok(())
    }

    /// A manual save with the world sections. Returns whether the save
    /// itself ran; the checkpoint refresh happens either way.
    pub fn save_and_checkpoint_full(
        &self,
        number: u32,
        session: &Session,
        world: &SaveWorld<'_>,
    ) -> io::Result<bool> {
        let saved = self.save_full(number, session, world)?;
        fs::copy(
            self.file(number, SlotKind::Save),
            self.file(number, SlotKind::Checkpoint),
        )?;
        Ok(saved)
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
    /// permadeath rule runs instead. Returns whether it saved. The
    /// "saving disabled" flag is checked before the health, so a disabled
    /// save neither writes nor drops the slot.
    pub fn save_checked(&self, number: u32, session: &Session, health: f32) -> io::Result<bool> {
        self.save_checked_full(number, session, &SaveWorld::default(), health)
    }

    /// A checked save with the world sections of [`Slots::save_full`].
    pub fn save_checked_full(
        &self,
        number: u32,
        session: &Session,
        world: &SaveWorld<'_>,
        health: f32,
    ) -> io::Result<bool> {
        if world.disabled {
            return Ok(false);
        }
        if !save_chunk::save_allowed(health, false) {
            self.drops_after_death(number, health)?;
            return Ok(false);
        }
        self.save_full(number, session, world)
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

    /// Whether a level is loadable: the save's section for the locale, or
    /// otherwise the game archive's lookup (the caller reports whether
    /// `<locale>.rfc` exists). The original asks the save first.
    pub fn level_available(
        &self,
        number: u32,
        locale: &str,
        archive_has: impl FnOnce(&str) -> bool,
    ) -> bool {
        self.has_saved_level(number, locale) || archive_has(locale)
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
        self.transition_save_full(number, locale, session, &SaveWorld::default())
    }

    /// The transition save with the world sections of
    /// [`Slots::save_full`].
    pub fn transition_save_full(
        &self,
        number: u32,
        locale: &str,
        session: &Session,
        world: &SaveWorld<'_>,
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
        self.save_full(number, session, world)?;
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
    use exa_formats::save_package::PlayerHeader;

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
        // A disabled save is skipped before the health check: the dead
        // player's save is neither written nor dropped.
        let disabled = SaveWorld {
            disabled: true,
            ..SaveWorld::default()
        };
        assert!(
            !slots
                .save_checked_full(number, &later, &disabled, 0.0)
                .unwrap()
        );
        assert!(slots.file(number, SlotKind::Save).exists());
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
        // The archive fallback only runs when the save has no section.
        assert!(slots.level_available(number, "exanima02", |locale| locale == "exanima02"));
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
        // The save's own section is enough; the archive is not asked.
        assert!(slots.level_available(number, "exanima02", |_| false));
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

    /// The original's story reader, reduced to what decides whether a save
    /// loads: the section found by its exact name, a signature at or above
    /// the story's, the 52-byte header, the first chunk taken as the
    /// character whatever its id, then chunk ids while more than eight
    /// bytes remain. Returns the character reference and the chunk ids.
    fn original_story_reader(bytes: &[u8]) -> Option<(u32, Vec<u32>)> {
        let word = |at: usize| -> Option<u32> {
            Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
        };
        if word(0)? != save_package::PACKAGE_MAGIC {
            return None;
        }
        let count = word(4)? as usize;
        let (offset, size) = (1..count).find_map(|index| {
            let at = 8 + index * 24;
            (bytes.get(at + 8..at + 24)? == &PLAYER_DATA[..])
                .then(|| Some((word(at)? as usize, word(at + 4)? as usize)))
                .flatten()
        })?;
        let section = bytes.get(offset..offset + size)?;
        let read = |at: usize| -> Option<u32> {
            Some(u32::from_le_bytes(
                section.get(at..at + 4)?.try_into().ok()?,
            ))
        };
        if read(0)? < save_package::STORY_SIGNATURE {
            return None;
        }
        let reference = read(60)?;
        let mut at = 60 + read(56)? as usize;
        let mut ids = Vec::new();
        while at + 8 < section.len() {
            ids.push(read(at)?);
            at += 8 + read(at + 4)? as usize;
        }
        Some((reference, ids))
    }

    #[test]
    fn a_session_saved_over_a_retail_save_stays_readable_by_the_original() {
        let dir = folder("retail");
        let slots = Slots::story(&dir);
        // A retail-shaped story save: a world character database, player
        // data with a placement, discovery, journal and experience, and the
        // level.
        let mut data = PlayerData::story(save_package::name("exanima01"));
        data.character = Some(
            save_package::CharacterPlacement {
                reference: save_package::CHARACTER_IN_WORLD_DB | 1,
                position: [10.0, 0.0, 20.0],
                heading: 0.5,
                condition: 0,
                chunks: Vec::new(),
            }
            .write(),
        );
        data.set_chunk(save_package::DISCOVERY_CHUNK, vec![0; 4]);
        data.set_chunk(save_package::GLOBAL_XP_CHUNK, 44u32.to_le_bytes().to_vec());
        let mut package = PackageImage::new();
        package.put_section(
            save_package::WORLD_CHARACTER_DB,
            &save_package::world_character_db(&[0; 8]),
        );
        package.put_section(PLAYER_DATA, &data.write());
        package.put_section(&save_package::name("exanima01"), b"level");
        let path = slots.file(1, SlotKind::Save);
        fs::write(&path, package.to_bytes()).unwrap();
        let (reference, ids) = original_story_reader(&package.to_bytes()).unwrap();
        assert_eq!(reference, 0x4000_0001);
        assert_eq!(
            ids,
            [save_package::DISCOVERY_CHUNK, save_package::GLOBAL_XP_CHUNK]
        );

        // Exarust saves its session over it.
        let mut session = Session::story();
        session.clock = 9;
        slots.save(1, &session).unwrap();
        let bytes = fs::read(&path).unwrap();
        let (reference, ids) = original_story_reader(&bytes).unwrap();
        // The placement is still first, the retail chunks are kept in the
        // writer's order (the journal before the experience), and the
        // Exarust chunk is one more id the original skips.
        assert_eq!(reference, 0x4000_0001);
        assert_eq!(
            ids,
            [
                save_package::DISCOVERY_CHUNK,
                save_package::JOURNAL_CHUNK,
                save_package::GLOBAL_XP_CHUNK,
                save_package::EXARUST_CHUNK
            ]
        );
        let package = PackageImage::read(&bytes).unwrap();
        assert!(package.has_section(save_package::WORLD_CHARACTER_DB));
        assert_eq!(
            package.section(&save_package::name("exanima01")),
            Some(&b"level"[..])
        );
        assert_eq!(slots.load(1, false).unwrap().0.clock, 9);

        // A save Exarust writes from nothing has no placement: the
        // original would take its first chunk (the empty discovery chunk)
        // as the character and fail to find the record. That is the
        // documented gap until Exarust exports a character.
        let fresh = session::write(&session);
        let (reference, _) = original_story_reader(&fresh).unwrap();
        assert_eq!(
            reference, 0,
            "the empty discovery payload read as a reference"
        );
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
        // A directory with the checkpoint's name is not a checkpoint; the
        // original's create call fails on it.
        fs::create_dir(story.file(9, SlotKind::Checkpoint)).unwrap();
        assert!(!story.has_checkpoint(9));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_first_file_for_a_number_decides_even_when_it_is_skipped() {
        let dir = folder("scan");
        let slots = Slots::story(&dir);
        // An empty package (no player data) at number 5, and a readable
        // checkpoint for the same number.
        fs::write(
            slots.file(5, SlotKind::Save),
            PackageImage::new().to_bytes(),
        )
        .unwrap();
        fs::write(
            slots.file(5, SlotKind::Checkpoint),
            session::write(&Session::story()),
        )
        .unwrap();
        // The save comes first: the number is skipped and its checkpoint is
        // not looked at.
        assert!(
            slots
                .scan(vec![
                    (5, SlotKind::Save),
                    (5, SlotKind::Checkpoint),
                    (6, SlotKind::Checkpoint)
                ])
                .unwrap()
                .iter()
                .all(|slot| slot.number == 6)
        );
        // The checkpoint comes first: the slot is listed from it.
        let listed = slots
            .scan(vec![(5, SlotKind::Checkpoint), (5, SlotKind::Save)])
            .unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(
            (listed[0].number, listed[0].kind),
            (5, SlotKind::Checkpoint)
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_new_slot_is_an_empty_package_until_it_is_saved() {
        let dir = folder("empty");
        let slots = Slots::arena(&dir);
        let number = slots.create_empty().unwrap();
        assert_eq!(number, 1);
        let bytes = fs::read(slots.file(number, SlotKind::Save)).unwrap();
        assert_eq!(bytes, PackageImage::new().to_bytes());
        // The empty package has no player data, so the scan leaves it out
        // (the original saves into it in the same flow).
        assert!(slots.numbers().unwrap().is_empty());
        // A convenience create writes the session straight away.
        let number = slots.create(&arena("Ash")).unwrap();
        assert_eq!(number, 1);
        assert_eq!(slots.numbers().unwrap(), vec![1]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_full_save_writes_every_section_in_the_writers_order() {
        let dir = folder("full");
        let slots = Slots::story(&dir);
        let mut session = Session::story();
        session.clock = 8;
        let world = SaveWorld {
            world_characters: Some(b"characters"),
            world_objects: Some((b"objects", b"manager")),
            level: Some(("exanima01", b"level")),
            disabled: false,
        };
        assert!(slots.save_full(1, &session, &world).unwrap());
        let bytes = fs::read(slots.file(1, SlotKind::Save)).unwrap();
        let package = PackageImage::read(&bytes).unwrap();
        let names: Vec<String> = package
            .entries()
            .iter()
            .map(|entry| save_package::name_text(&entry.name))
            .collect();
        assert_eq!(
            names,
            [
                "#worldchrdb".to_string(),
                "#playerdata".to_string(),
                "#worldobjdb".to_string(),
                "exanima01".to_string()
            ]
        );
        assert_eq!(
            package
                .entries()
                .iter()
                .map(|entry| entry.offset)
                .collect::<Vec<_>>(),
            [0x2000, 0x4000, 0x6000, 0x8000]
        );
        // The chunks are framed, and the player data reads back.
        assert_eq!(
            package.section(save_package::WORLD_OBJECT_DB),
            Some(&save_package::world_object_db(b"objects", b"manager")[..])
        );
        assert_eq!(session::slot_info(&bytes).unwrap().clock, 8);
        let loaded = slots.load_package(1, false).unwrap();
        assert!(loaded.has_level_section);
        assert!(loaded.has_world_characters);
        assert!(loaded.has_world_objects);
        assert_eq!(loaded.level_locale, save_package::name("exanima01"));
        // The "saving disabled" flag skips the whole save.
        fs::remove_file(slots.file(1, SlotKind::Save)).unwrap();
        let disabled = SaveWorld {
            disabled: true,
            ..world
        };
        assert!(!slots.save_full(1, &session, &disabled).unwrap());
        assert!(!slots.file(1, SlotKind::Save).exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_old_arena_save_is_converted_and_written_back() {
        let dir = folder("oldarena");
        let slots = Slots::arena(&dir);
        let number = slots.create(&arena("Ash")).unwrap();
        let path = slots.file(number, SlotKind::Save);
        let mut package = PackageImage::read(&fs::read(&path).unwrap()).unwrap();
        let mut data = PlayerData::parse(
            package.section(PLAYER_DATA).unwrap(),
            save_package::Flavour::Arena,
        )
        .unwrap();
        data.set_chunk(save_package::HIREABLES_CHUNK, vec![1]);
        data.set_chunk(save_package::SHOP_STOCK_CHUNK, vec![2]);
        data.character = Some(
            save_package::CharacterPlacement {
                reference: save_package::CHARACTER_IN_WORLD_DB | 1,
                position: [10.0, 5.0, -20.0],
                heading: 1.0,
                condition: 0,
                chunks: Vec::new(),
            }
            .write(),
        );
        let PlayerHeader::Arena(header) = &mut data.header else {
            unreachable!()
        };
        header.data_version = 4;
        package.put_section(PLAYER_DATA, &data.write());
        package.put_section(&save_package::name("arenahub"), b"hub");
        fs::write(&path, package.to_bytes()).unwrap();

        let loaded = slots.load_package(number, false).unwrap();
        assert!(loaded.converted);
        assert!(!loaded.package.has_section(&save_package::name("arenahub")));
        assert!(loaded.data.chunk(save_package::HIREABLES_CHUNK).is_none());
        assert!(loaded.data.chunk(save_package::SHOP_STOCK_CHUNK).is_none());
        assert_eq!(
            loaded.data.placement().unwrap().unwrap().position,
            save_package::OLD_ARENA_POSITION
        );
        assert_eq!(loaded.data.header.clock(), 12);
        // The converted player data was written back, and the session still
        // reads through the ordinary load.
        let saved = PackageImage::read(&fs::read(&path).unwrap()).unwrap();
        assert!(!saved.has_section(&save_package::name("arenahub")));
        assert_eq!(slots.load(number, false).unwrap().0.company.name, "Ash");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_mid_transition_story_save_loads_as_exanima05() {
        let dir = folder("transition-load");
        let slots = Slots::story(&dir);
        let number = slots.create(&Session::story()).unwrap();
        let path = slots.file(number, SlotKind::Save);
        let mut package = PackageImage::read(&fs::read(&path).unwrap()).unwrap();
        let mut data = PlayerData::parse(
            package.section(PLAYER_DATA).unwrap(),
            save_package::Flavour::Story,
        )
        .unwrap();
        data.set_chunk(
            save_package::TRANSITION_CHUNK,
            [2u32, 1, 0, 0].map(u32::to_le_bytes).concat(),
        );
        package.put_section(PLAYER_DATA, &data.write());
        fs::write(&path, package.to_bytes()).unwrap();
        let loaded = slots.load_package(number, false).unwrap();
        assert_eq!(loaded.forced_locale, Some(save_package::name("exanima05")));
        assert_eq!(loaded.level_locale, save_package::name("exanima05"));
        assert_eq!(loaded.level_locale_name(), "exanima05");
        assert!(!loaded.has_level_section);
        assert!(!loaded.has_world_characters);
        assert!(!loaded.has_world_objects);
        assert!(!loaded.converted);
        // State 1 does not force a level.
        data.set_chunk(
            save_package::TRANSITION_CHUNK,
            [1u32, 0, 0, 0].map(u32::to_le_bytes).concat(),
        );
        package.put_section(PLAYER_DATA, &data.write());
        fs::write(&path, package.to_bytes()).unwrap();
        assert_eq!(
            slots.load_package(number, false).unwrap().forced_locale,
            None
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_character_screen_prefers_the_save_and_keeps_only_journal_and_xp() {
        let dir = folder("screen");
        let slots = Slots::story(&dir);
        let session = Session::story();
        let number = slots.create(&session).unwrap();
        // Give the save a discovery chunk and a global experience chunk.
        let path = slots.file(number, SlotKind::Save);
        let mut package = PackageImage::read(&fs::read(&path).unwrap()).unwrap();
        let mut data = PlayerData::parse(
            package.section(PLAYER_DATA).unwrap(),
            save_package::Flavour::Story,
        )
        .unwrap();
        data.set_chunk(save_package::DISCOVERY_CHUNK, vec![0; 8]);
        data.set_chunk(save_package::GLOBAL_XP_CHUNK, 44u32.to_le_bytes().to_vec());
        package.put_section(PLAYER_DATA, &data.write_ordered());
        fs::write(&path, package.to_bytes()).unwrap();
        // A checkpoint with a different clock (written directly so the save
        // keeps its own clock).
        let mut checkpoint = session.clone();
        checkpoint.clock = 99;
        fs::write(
            slots.file(number, SlotKind::Checkpoint),
            session::write(&checkpoint),
        )
        .unwrap();
        // The save is preferred though the checkpoint is fresher.
        let loaded = slots.load_character_screen(number, false).unwrap();
        assert_eq!(loaded.kind, SlotKind::Save);
        assert_eq!(loaded.data.header.clock(), 0);
        let ids: Vec<u32> = loaded.data.chunks.iter().map(|chunk| chunk.id).collect();
        assert_eq!(
            ids,
            [save_package::JOURNAL_CHUNK, save_package::GLOBAL_XP_CHUNK]
        );
        // Asked for, the checkpoint is read instead.
        let loaded = slots.load_character_screen(number, true).unwrap();
        assert_eq!(loaded.kind, SlotKind::Checkpoint);
        assert_eq!(loaded.data.header.clock(), 99);
        // Without a save only the checkpoint is left.
        fs::remove_file(slots.file(number, SlotKind::Save)).unwrap();
        let loaded = slots.load_character_screen(number, false).unwrap();
        assert_eq!(loaded.kind, SlotKind::Checkpoint);
        assert_eq!(loaded.data.header.clock(), 99);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_load_exposes_the_placed_player_record() {
        use exa_formats::characters::{CharacterRecord, WORLD_DB};
        let dir = folder("record");
        let slots = Slots::story(&dir);
        let number = slots.create(&Session::story()).unwrap();
        let mut record = CharacterRecord::new();
        record.id = 5;
        record.name = "Ash".to_string();
        let record = record.to_bytes().unwrap();
        let mut db = Vec::new();
        db.extend_from_slice(&0xDBAC_CD00u32.to_le_bytes());
        db.extend_from_slice(&16u32.to_le_bytes());
        db.extend_from_slice(&5u32.to_le_bytes());
        db.extend_from_slice(&0u32.to_le_bytes());
        db.extend_from_slice(&0u32.to_le_bytes());
        db.extend_from_slice(&(record.len() as u32).to_le_bytes());
        db.extend_from_slice(&record);
        let path = slots.file(number, SlotKind::Save);
        let mut package = PackageImage::read(&fs::read(&path).unwrap()).unwrap();
        package.put_section(
            save_package::WORLD_CHARACTER_DB,
            &save_package::world_character_db(&db),
        );
        let mut data = PlayerData::parse(
            package.section(PLAYER_DATA).unwrap(),
            save_package::Flavour::Story,
        )
        .unwrap();
        data.character = Some(
            save_package::CharacterPlacement {
                reference: WORLD_DB | 5,
                position: [1.0, 2.0, 3.0],
                heading: 0.0,
                condition: 0,
                chunks: Vec::new(),
            }
            .write(),
        );
        package.put_section(PLAYER_DATA, &data.write_ordered());
        fs::write(&path, package.to_bytes()).unwrap();
        let loaded = slots.load_package(number, false).unwrap();
        let record = loaded.player_record().unwrap();
        assert_eq!(record.name, "Ash");
        assert_eq!(record.id, 5);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_failed_checkpoint_load_leaves_a_fresh_save_file() {
        let dir = folder("checkpoint-fail");
        let slots = Slots::story(&dir);
        // Only a checkpoint, and it is not a readable session.
        fs::write(slots.file(3, SlotKind::Checkpoint), b"garbage").unwrap();
        assert_eq!(slots.load(3, false), Err(SaveError::Malformed));
        let save = fs::read(slots.file(3, SlotKind::Save)).unwrap();
        assert_eq!(save, PackageImage::new().to_bytes());
        // Opening the checkpoint reset its bad header; the save was created
        // fresh as well, and the checkpoint was not copied over it.
        assert_eq!(fs::read(slots.file(3, SlotKind::Checkpoint)).unwrap(), save);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_checkpoint_writes_every_section_and_copies_the_file() {
        let dir = folder("checkpoint-full");
        let slots = Slots::story(&dir);
        let number = slots.create_empty().unwrap();
        let world = SaveWorld {
            world_characters: Some(b"characters"),
            world_objects: Some((b"objects", b"manager")),
            level: Some(("exanima01", b"level")),
            disabled: false,
        };
        assert!(
            slots
                .checkpoint_full(number, &Session::story(), &world)
                .unwrap()
        );
        let save = fs::read(slots.file(number, SlotKind::Save)).unwrap();
        let checkpoint = fs::read(slots.file(number, SlotKind::Checkpoint)).unwrap();
        assert_eq!(save, checkpoint);
        let package = PackageImage::read(&save).unwrap();
        assert!(package.has_section(save_package::WORLD_CHARACTER_DB));
        assert!(package.has_section(save_package::WORLD_OBJECT_DB));
        assert!(package.has_section(&save_package::name("exanima01")));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn deleting_by_index_removes_both_files_and_leaves_the_order() {
        let dir = folder("delete-index");
        let slots = Slots::arena(&dir);
        let first = slots.create(&arena("Ash")).unwrap();
        let second = slots.create(&arena("Bram")).unwrap();
        assert_eq!(slots.numbers().unwrap(), vec![second, first]);
        assert_eq!(slots.delete_index(0).unwrap(), Some(second));
        assert!(!slots.file(second, SlotKind::Save).exists());
        assert!(!slots.file(second, SlotKind::Checkpoint).exists());
        assert_eq!(slots.numbers().unwrap(), vec![first]);
        assert_eq!(slots.delete_index(5).unwrap(), None);
        let _ = fs::remove_dir_all(&dir);
    }
}
