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

/// The save's world character database, read as the load's character
/// database importer reads it: the `#worldchrdb` section's first chunk into
/// the world tier. `None` when the section or the chunk is missing or does
/// not read; the original then clears the live database.
pub fn world_characters(package: &PackageImage) -> Option<exa_formats::character_db::CharacterDb> {
    let section = package.section(save_package::WORLD_CHARACTER_DB)?;
    let payload = save_package::first_chunk_payload(section)?;
    exa_formats::character_db::CharacterDb::from_rdb(exa_formats::character_db::WORLD_TAG, payload)
        .ok()
}

/// The save's world object database, read as the load's object database
/// importer reads it: the `#worldobjdb` section's first chunk into the world
/// tier and its second chunk (the data manager's table) into the string
/// library the records resolve their text through. `None` when the section
/// or its first chunk is missing or does not read; the original then clears
/// the live database.
pub fn world_objects(package: &PackageImage) -> Option<exa_formats::objdb::ObjectDb> {
    let section = package.section(save_package::WORLD_OBJECT_DB)?;
    let (objects, strings) = save_package::world_object_payloads(section)?;
    let world = exa_formats::objdb::Store::parse(objects).ok()?;
    let strings = strings
        .and_then(|bytes| exa_formats::objstrings::Library::parse(bytes).ok())
        .unwrap_or_default();
    let mut db = exa_formats::objdb::ObjectDb::default();
    db.world = world;
    db.strings = strings;
    Some(db)
}

/// The two world databases as a save exports them, ready for
/// [`SaveWorld`]: the character database's save form and the object
/// database's world tier with its string library. A database with no
/// record exports nothing, and its section is not written.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorldExport {
    /// The world character database: flushed, then saved (magic, table and
    /// payload), the payload of the `#worldchrdb` section's one chunk.
    pub characters: Option<Vec<u8>>,
    /// The world object tier and the string library, the payloads of the
    /// `#worldobjdb` section's two chunks.
    pub objects: Option<(Vec<u8>, Vec<u8>)>,
}

impl WorldExport {
    /// The sections a save writes from this export, with the current level
    /// and the save-disable flag.
    pub fn save_world<'a>(
        &'a self,
        level: Option<(&'a str, &'a [u8])>,
        disabled: bool,
    ) -> SaveWorld<'a> {
        SaveWorld {
            world_characters: self.characters.as_deref(),
            world_objects: self
                .objects
                .as_ref()
                .map(|(objects, strings)| (objects.as_slice(), strings.as_slice())),
            level,
            disabled,
        }
    }
}

/// Exports the world databases as the save does before writing its
/// sections: the character database only when it holds a record (it is
/// flushed first, so every live character's current record is written,
/// then saved in its `0xDBACCD00` form), the object database only when its
/// world tier holds a record (the tier's writer, then the string library's
/// writer, the same library [`world_objects`] imports the second chunk
/// into). The scene's characters are flushed into the character database
/// by the caller first ([`crate::arena_ids::flush_scene_character`]), as
/// the original walks the scene's character list before the export.
pub fn export_world_databases(
    characters: &mut exa_formats::character_db::CharacterDb,
    objects: &exa_formats::objdb::ObjectDb,
) -> exa_formats::Result<WorldExport> {
    let characters = if characters.is_empty() {
        None
    } else {
        characters.flush()?;
        Some(characters.save())
    };
    let objects =
        (!objects.world.is_empty()).then(|| (objects.world.write(), objects.strings.write()));
    Ok(WorldExport {
        characters,
        objects,
    })
}

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
    /// The load mask the player data was read with: a full load's is every
    /// bit, the character screen's `0x100010`, the scan's 0.
    pub mask: u32,
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
        let mut db = self.world_characters()?;
        let record = db.get(id)?;
        let mut record = record.record().clone();
        // The slot view names records by their low id, as the placement does.
        record.id &= exa_formats::characters::ID_MASK;
        Some(record)
    }

    /// The save's world character database, imported as the load does.
    pub fn world_characters(&self) -> Option<exa_formats::character_db::CharacterDb> {
        world_characters(&self.package)
    }

    /// The save's world object database and its strings, imported as the
    /// load does.
    pub fn world_objects(&self) -> Option<exa_formats::objdb::ObjectDb> {
        world_objects(&self.package)
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

    /// Opens a package file as the original does: a missing file is created
    /// empty (the original opens with "open always"), and a file of eight
    /// bytes or fewer or with another magic reads as a fresh package. The
    /// open itself writes nothing; the next section write puts the header
    /// and the directory over the file's start and keeps the rest. A file
    /// with the magic whose directory does not fit reads as an empty
    /// package here.
    pub fn open(path: &Path) -> io::Result<PackageImage> {
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(false)
                    .open(path)?;
                Vec::new()
            }
            Err(error) => return Err(error),
        };
        Ok(PackageImage::open(&bytes))
    }

    /// The original's checkpoint copy: the save copied over the checkpoint.
    pub fn copy_save_over_checkpoint(&self, number: u32) -> io::Result<()> {
        fs::copy(
            self.file(number, SlotKind::Save),
            self.file(number, SlotKind::Checkpoint),
        )?;
        Ok(())
    }

    /// The original's copy-back: the checkpoint copied over the save, which
    /// a load from a checkpoint does once the level is up.
    pub fn copy_checkpoint_over_save(&self, number: u32) -> io::Result<()> {
        fs::copy(
            self.file(number, SlotKind::Checkpoint),
            self.file(number, SlotKind::Save),
        )?;
        Ok(())
    }

    /// What the mode's player-data reader makes of a package during a scan
    /// (it reads no chunks then): the story refuses a signature older than
    /// its own, which deletes the slot, and reads any other as a story; the
    /// arena takes only its exact signature and refuses nothing. The arena
    /// reader still converts an old save's header before the chunk loop:
    /// the `arenahub` section is deleted from the package here, as the
    /// scan does.
    fn verdict(&self, package: &mut PackageImage, path: &Path) -> Verdict {
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
                if flavour == Flavour::Arena && data.arena_data_is_old() {
                    data.convert_arena();
                    if package.remove_section(&save_package::name("arenahub")) {
                        // The loader's section delete rewrites the
                        // directory over the file.
                        let _ = fs::write(path, package.to_bytes());
                    }
                }
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
    /// directory order; a number already **listed** is not looked at again,
    /// but a number whose first file was left out or deleted is: the
    /// original writes the number into the list's next free place before
    /// asking the mode's reader and only moves past it when the reader
    /// keeps the slot, and its repeat test covers the kept places only, so
    /// a skipped save's checkpoint is still tried. A slot the story refuses
    /// for its old signature is deleted, both files; one that does not read
    /// is left out and left on disk.
    ///
    /// The original also requires the player's record to be found in the
    /// character databases; Exarust's own saves carry no placement, so a
    /// readable header is enough here.
    pub fn list(&self) -> io::Result<Vec<Slot>> {
        self.scan(self.candidates()?)
    }

    /// Applies the scan to slot files already listed in the order the
    /// folder returned them. Split out so tests can pin the order. A file
    /// that is gone by the time the scan reaches it (deleted with its slot
    /// earlier in the same scan) is passed over [guess: the original's
    /// directory walk does not return a file deleted before the walk
    /// reached it; the scan's open would otherwise create it empty].
    fn scan(&self, candidates: Vec<(u32, SlotKind)>) -> io::Result<Vec<Slot>> {
        let mut valid: Vec<Slot> = Vec::new();
        for (number, kind) in candidates {
            if valid.iter().any(|slot| slot.number == number) {
                continue;
            }
            let path = self.file(number, kind);
            if !path.is_file() {
                continue;
            }
            let mut package = Self::open(&path)?;
            match self.verdict(&mut package, &path) {
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
        self.open_first(number, prefer_checkpoint, mask)
    }

    /// Opens the first existing file of a slot in the asked order and
    /// decodes it with `mask`; a failed load from a checkpoint switches the
    /// package back to the save name.
    fn open_first(
        &self,
        number: u32,
        prefer_checkpoint: bool,
        mask: u32,
    ) -> Result<LoadedSave, SaveError> {
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
        let mut converted = false;
        if flavour == Flavour::Arena && data.arena_data_is_old() {
            // Convert and write back before the load mask hides chunks:
            // the file keeps every chunk the conversion did not drop.
            data.convert_arena();
            package.remove_section(&save_package::name("arenahub"));
            package.put_section(PLAYER_DATA, &data.write_ordered());
            fs::write(&path, package.to_bytes()).map_err(|_| SaveError::Malformed)?;
            converted = true;
        }
        data.apply_mask(mask);
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
            mask,
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
            let _ = self.copy_checkpoint_over_save(number);
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
            // The section write opens the package first, with its resets.
            Ok(bytes) => PackageImage::open(&bytes),
            Err(error) if error.kind() == io::ErrorKind::NotFound => PackageImage::new(),
            Err(error) => return Err(error),
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
        self.copy_save_over_checkpoint(number)?;
        Ok(true)
    }

    /// The `CheckpointSave` script function's save path: it does nothing
    /// unless it was called for the player with checkpoints enabled, and
    /// then writes a checkpoint only when the slot has none. Returns
    /// whether one was written.
    pub fn checkpoint_from_script(
        &self,
        number: u32,
        session: &Session,
        player: bool,
        enabled: bool,
    ) -> io::Result<bool> {
        if !save_chunk::checkpoint_script_runs(player, enabled) {
            return Ok(false);
        }
        self.checkpoint(number, session)
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
        self.copy_save_over_checkpoint(number)?;
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
        self.copy_save_over_checkpoint(number)?;
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

/// The slot manager's cached list, as the original keeps it beside the
/// folder: the slot count (−1 until the folder was scanned), the numbers in
/// the list's order with each slot's player name, and the current slot (−1
/// when the manager is made, 0 after a delete or a dropped death). The
/// entry points that need the list scan the folder only while the count is
/// negative; creating, selecting, touching and deleting then work on the
/// cached list and keep the stored order, so the folder is read once and
/// not on every question.
///
/// A scan whose folder cannot be listed leaves the count at −1, so the next
/// question scans again, as the original's directory walk that fails
/// leaves its count unset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotList {
    slots: Slots,
    count: i32,
    numbers: Vec<u32>,
    names: Vec<Option<String>>,
    current: i32,
}

impl SlotList {
    /// A manager over `slots`, not scanned yet: count −1, current −1.
    pub fn new(slots: Slots) -> Self {
        Self {
            slots,
            count: -1,
            numbers: Vec::new(),
            names: Vec::new(),
            current: -1,
        }
    }

    /// The folder and mode.
    pub fn slots(&self) -> &Slots {
        &self.slots
    }

    /// The cached count; −1 until a scan ran.
    pub fn count(&self) -> i32 {
        self.count
    }

    /// The current slot number: −1 at creation, 0 when none is chosen.
    pub fn current(&self) -> i32 {
        self.current
    }

    /// The cached numbers, in the list's order.
    pub fn numbers(&self) -> &[u32] {
        &self.numbers
    }

    /// The cached player names, one per number.
    pub fn names(&self) -> &[Option<String>] {
        &self.names
    }

    /// Scans the folder ([`Slots::list`]) into the cache. A folder that
    /// cannot be listed leaves the cache as it was.
    pub fn rescan(&mut self) -> io::Result<()> {
        let listed = self.slots.list()?;
        self.numbers = listed.iter().map(|slot| slot.number).collect();
        self.names = listed
            .into_iter()
            .map(|slot| slot.info.player_name)
            .collect();
        self.count = self.numbers.len() as i32;
        Ok(())
    }

    /// Scans only while the count is negative.
    fn ensure(&mut self) -> io::Result<()> {
        if self.count < 0 {
            self.rescan()?;
        }
        Ok(())
    }

    /// Whether any slot is listed: the cached count above zero, scanning
    /// first when there is none. A folder that cannot be listed has none.
    pub fn any(&mut self) -> bool {
        let _ = self.ensure();
        self.count > 0
    }

    /// The new-slot entry: the lowest free number
    /// ([`save_chunk::free_slot_number`]) goes to the front, the others
    /// move down a place with their names, and the front name becomes the
    /// current player's when there is one; without one the front keeps the
    /// name that was there (the original only overwrites the moved list's
    /// first name when a player record exists). The new number is the
    /// current slot, the order is stored, and the slot's save file is
    /// opened (created empty when missing). Returns the number.
    pub fn create(&mut self, player_name: Option<&str>) -> io::Result<u32> {
        self.ensure()?;
        let number = save_chunk::free_slot_number(&self.numbers);
        save_chunk::insert_slot(&mut self.numbers, number);
        let front = self.names.first().cloned().flatten();
        self.names.insert(0, front);
        if let Some(name) = player_name {
            self.names[0] = Some(name.to_owned());
        }
        self.count += 1;
        self.current = number as i32;
        self.slots.store_order(&self.numbers)?;
        Slots::open(&self.slots.file(number, SlotKind::Save))?;
        Ok(number)
    }

    /// The number [`SlotList::create`] would take now, scanning first when
    /// the list was never scanned.
    pub fn next_free_number(&mut self) -> io::Result<u32> {
        self.ensure()?;
        Ok(save_chunk::free_slot_number(&self.numbers))
    }

    /// Makes the listed slot at `index` the current one, as the character
    /// screen's load does before it reads; an index outside the cached
    /// list changes nothing.
    pub fn select(&mut self, index: usize) -> Option<u32> {
        let number = *self.numbers.get(index)?;
        self.current = number as i32;
        Some(number)
    }

    /// Moves a listed number to the front with its name and stores the
    /// order; a number that is not listed changes nothing and stores
    /// nothing.
    pub fn touch(&mut self, number: u32) {
        let Some(at) = self.numbers.iter().position(|&seen| seen == number) else {
            return;
        };
        let moved = self.numbers.remove(at);
        let name = self.names.remove(at);
        self.numbers.insert(0, moved);
        self.names.insert(0, name);
        let _ = self.slots.store_order(&self.numbers);
    }

    /// The menu's delete of the slot at `index`: it becomes the current
    /// slot, both files are deleted (the save first), it leaves the list
    /// and the current slot is 0 afterwards. The stored order is not
    /// rewritten. Returns the number deleted.
    pub fn delete_index(&mut self, index: usize) -> io::Result<Option<u32>> {
        let Some(&number) = self.numbers.get(index) else {
            return Ok(None);
        };
        self.current = number as i32;
        let deleted = self.slots.delete(number);
        self.numbers.remove(index);
        self.names.remove(index);
        self.count -= 1;
        self.current = 0;
        deleted.map(|()| Some(number))
    }

    /// The death rule over the current slot: with health below 0.01 (in
    /// double) and a current slot above zero, the save file is deleted;
    /// without a checkpoint the slot then leaves the list and the current
    /// slot is 0, with one it stays for the next load to come back from.
    /// Returns whether the slot left the list.
    pub fn drop_after_death(&mut self, health: f32) -> io::Result<bool> {
        if self.current <= 0 || save_chunk::save_allowed(health, false) {
            return Ok(false);
        }
        let number = self.current as u32;
        match fs::remove_file(self.slots.file(number, SlotKind::Save)) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        if self.slots.has_checkpoint(number) {
            return Ok(false);
        }
        if let Some(at) = self.numbers.iter().position(|&seen| seen == number) {
            self.numbers.remove(at);
            self.names.remove(at);
            self.count -= 1;
        }
        self.current = 0;
        Ok(true)
    }

    /// The full load of the current slot: it moves to the front of the
    /// cached list, then the slot's files are tried as [`Slots::load`]
    /// tries them (the checkpoint first when asked). Loading from a
    /// checkpoint copies it back over the save.
    pub fn load_current(&mut self, prefer_checkpoint: bool) -> Result<LoadedSave, SaveError> {
        if self.current <= 0 {
            return Err(SaveError::Malformed);
        }
        let number = self.current as u32;
        self.touch(number);
        let loaded = self.slots.open_first(number, prefer_checkpoint, !0)?;
        if loaded.kind == SlotKind::Checkpoint {
            let _ = self.slots.copy_checkpoint_over_save(number);
        }
        Ok(loaded)
    }

    /// Writes every section of a save into the current slot
    /// ([`Slots::save_full`]). No current slot writes nothing.
    pub fn save_current(&self, session: &Session, world: &SaveWorld<'_>) -> io::Result<bool> {
        if self.current <= 0 {
            return Ok(false);
        }
        self.slots.save_full(self.current as u32, session, world)
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
    fn a_slot_copies_its_save_and_checkpoint_both_ways() {
        let dir = folder("copies");
        let slots = Slots::arena(&dir);
        let number = slots.create(&arena("Ash")).unwrap();
        let mut later = arena("Ash");
        later.clock = 42;
        slots.save(number, &later).unwrap();
        slots.copy_save_over_checkpoint(number).unwrap();
        let checkpoint = fs::read(slots.file(number, SlotKind::Checkpoint)).unwrap();
        let package = PackageImage::read(&checkpoint).unwrap();
        let data = PlayerData::parse(
            package.section(PLAYER_DATA).unwrap(),
            save_package::Flavour::Arena,
        )
        .unwrap();
        assert_eq!(data.header.clock(), 42);
        // The copy-back overwrites the save with the checkpoint.
        slots.save(number, &arena("Bram")).unwrap();
        slots.copy_checkpoint_over_save(number).unwrap();
        assert_eq!(slots.load(number, false).unwrap().0.clock, 42);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn opening_creates_a_missing_file_empty_and_writes_nothing_else() {
        let dir = folder("open");
        let path = dir.join("Arena001.rsg");
        let package = Slots::open(&path).unwrap();
        assert!(package.entries().is_empty());
        assert_eq!(fs::read(&path).unwrap(), Vec::<u8>::new());
        fs::write(&path, b"garbage").unwrap();
        Slots::open(&path).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"garbage");
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
    fn the_checkpoint_script_needs_the_player_and_the_setting() {
        let dir = folder("script");
        let slots = Slots::story(&dir);
        let number = slots.create(&Session::story()).unwrap();
        // Not the player, or checkpoints off: nothing is written.
        assert!(
            !slots
                .checkpoint_from_script(number, &Session::story(), false, true)
                .unwrap()
        );
        assert!(!slots.has_checkpoint(number));
        assert!(
            !slots
                .checkpoint_from_script(number, &Session::story(), true, false)
                .unwrap()
        );
        // The player with checkpoints on writes one; a second call sees it.
        assert!(
            slots
                .checkpoint_from_script(number, &Session::story(), true, true)
                .unwrap()
        );
        assert!(slots.has_checkpoint(number));
        assert!(
            !slots
                .checkpoint_from_script(number, &Session::story(), true, true)
                .unwrap()
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_scan_deletes_only_old_story_signatures() {
        let dir = folder("invalid");
        let slots = Slots::story(&dir);
        let number = slots.create(&Session::story()).unwrap();
        // A file of the right name that is not a package: left out and left
        // on disk as it was (the open resets it in memory only).
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
        assert_eq!(fs::read(slots.file(7, SlotKind::Save)).unwrap(), b"garbage");
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
    fn a_skipped_number_is_tried_again_and_a_listed_one_is_not() {
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
        // The save comes first and is left out; the repeat test covers the
        // kept slots only, so the checkpoint lists the slot. A number with
        // no file left is passed over and not created.
        let listed = slots
            .scan(vec![
                (5, SlotKind::Save),
                (5, SlotKind::Checkpoint),
                (6, SlotKind::Checkpoint),
            ])
            .unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(
            (listed[0].number, listed[0].kind),
            (5, SlotKind::Checkpoint)
        );
        assert!(!slots.file(6, SlotKind::Checkpoint).exists());
        // The checkpoint comes first: the slot is listed from it and the
        // save is not looked at again.
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
        assert!(
            bytes.is_empty(),
            "the open creates the file and writes nothing"
        );
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

        // The scan runs the arena reader too: its version conversion
        // deletes the hub section from the package.
        assert!(
            slots
                .list()
                .unwrap()
                .iter()
                .any(|slot| slot.number == number)
        );
        let scanned = PackageImage::read(&fs::read(&path).unwrap()).unwrap();
        assert!(!scanned.has_section(&save_package::name("arenahub")));

        // The character screen's masked load must not strip the file's
        // chunks when it converts the old save.
        slots.load_character_screen(number, false).unwrap();
        let screened = PackageImage::read(&fs::read(&path).unwrap()).unwrap();
        let screened_data = PlayerData::parse(
            screened.section(PLAYER_DATA).unwrap(),
            save_package::Flavour::Arena,
        )
        .unwrap();
        assert!(screened_data.chunk(save_package::EXARUST_CHUNK).is_some());

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
        // The session layer applies the forced level too.
        assert_eq!(
            slots.load(number, false).unwrap().0.story.level(),
            "exanima05"
        );
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
    fn a_saves_world_databases_import_into_their_tiers() {
        use exa_formats::characters::CharacterRecord;
        use exa_formats::objdb::ObjectDb;
        let mut package = PackageImage::new();
        let mut db =
            exa_formats::character_db::CharacterDb::new(exa_formats::character_db::WORLD_TAG);
        let mut record = CharacterRecord::new();
        record.id = 5;
        record.name = "Ash".to_string();
        db.add(5, record);
        db.flush().unwrap();
        package.put_section(
            save_package::WORLD_CHARACTER_DB,
            &save_package::world_character_db(&db.save()),
        );
        let objects = ObjectDb::default();
        package.put_section(
            save_package::WORLD_OBJECT_DB,
            &save_package::world_object_db(&objects.world.write(), &objects.strings.write()),
        );
        let mut characters = world_characters(&package).unwrap();
        assert!(characters.contains(5));
        assert_eq!(characters.get(5).unwrap().record().name, "Ash");
        let objects = world_objects(&package).unwrap();
        assert!(objects.world.is_empty());
        // A package without the sections imports nothing (the original
        // clears the live databases then).
        assert!(world_characters(&PackageImage::new()).is_none());
        assert!(world_objects(&PackageImage::new()).is_none());
        // A broken table imports nothing rather than panicking.
        let mut broken = PackageImage::new();
        broken.put_section(
            save_package::WORLD_OBJECT_DB,
            &save_package::world_object_db(&[0xff; 6], &[]),
        );
        assert!(world_objects(&broken).is_none());
    }

    #[test]
    fn a_failed_checkpoint_load_leaves_an_empty_save_file() {
        let dir = folder("checkpoint-fail");
        let slots = Slots::story(&dir);
        // Only a checkpoint, and it is not a readable session.
        fs::write(slots.file(3, SlotKind::Checkpoint), b"garbage").unwrap();
        assert_eq!(slots.load(3, false), Err(SaveError::Malformed));
        // The save was created empty, the checkpoint was not copied over
        // it, and opening the checkpoint left its bad bytes alone.
        assert_eq!(fs::read(slots.file(3, SlotKind::Save)).unwrap(), b"");
        assert_eq!(
            fs::read(slots.file(3, SlotKind::Checkpoint)).unwrap(),
            b"garbage"
        );
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

    #[test]
    fn exported_world_databases_import_back_and_empty_ones_write_nothing() {
        use exa_formats::character_db::{CharacterDb, WORLD_TAG};
        use exa_formats::characters::CharacterRecord;
        use exa_formats::objdb::{ObjectDb, Store};
        let dir = folder("world-export");
        let slots = Slots::story(&dir);
        let mut characters = CharacterDb::new(WORLD_TAG);
        let mut objects = ObjectDb::default();
        let empty = export_world_databases(&mut characters, &objects).unwrap();
        assert_eq!(empty, WorldExport::default());
        let mut record = CharacterRecord::new();
        record.name = "Tobb".into();
        characters.add(3, record);
        objects.world = Store::from_records([(0x4000_0002, 0, &[1u8, 2, 3, 4][..])]);
        let text = objects.strings.add(b"rusty key");
        let export = export_world_databases(&mut characters, &objects).unwrap();
        let saved = export.characters.as_deref().unwrap();
        assert_eq!(&saved[0..4], &0xDBAC_CD00u32.to_le_bytes());
        // One record and the sentinel: a table of 32 bytes.
        assert_eq!(&saved[4..8], &32u32.to_le_bytes());
        assert!(
            slots
                .save_full(1, &Session::story(), &export.save_world(None, false))
                .unwrap()
        );
        let package = Slots::open(&slots.file(1, SlotKind::Save)).unwrap();
        let mut back = world_characters(&package).unwrap();
        assert_eq!(back.get(3).unwrap().record().name, "Tobb");
        let objects_back = world_objects(&package).unwrap();
        assert_eq!(objects_back.world.write(), objects.world.write());
        assert_eq!(objects_back.strings.get(text).as_deref(), Some("rusty key"));
        // A disabled save writes nothing at all.
        let dir2 = folder("world-export-disabled");
        let slots2 = Slots::story(&dir2);
        assert!(
            !slots2
                .save_full(1, &Session::story(), &export.save_world(None, true))
                .unwrap()
        );
        assert!(!slots2.file(1, SlotKind::Save).exists());
        let _ = fs::remove_dir_all(&dir);
        let _ = fs::remove_dir_all(&dir2);
    }

    #[test]
    fn the_cached_list_scans_once_and_works_on_its_cache() {
        let dir = folder("cache");
        let slots = Slots::arena(&dir);
        let mut list = SlotList::new(slots.clone());
        assert_eq!((list.count(), list.current()), (-1, -1));
        // An empty folder scans to zero slots.
        assert!(!list.any());
        assert_eq!(list.count(), 0);
        // Three new slots take 1, 2, 3, each at the front and current.
        assert_eq!(list.create(Some("Ash")).unwrap(), 1);
        assert_eq!(list.create(None).unwrap(), 2);
        assert_eq!(list.create(Some("Cid")).unwrap(), 3);
        assert_eq!(list.numbers(), &[3, 2, 1]);
        assert_eq!(list.current(), 3);
        // Without a player the new front keeps the name that was there.
        assert_eq!(
            list.names(),
            &[Some("Cid".into()), Some("Ash".into()), Some("Ash".into())]
        );
        assert!(slots.file(2, SlotKind::Save).is_file());
        // A file written behind the cache's back is not seen: the count is
        // set, so the folder is not read again.
        fs::write(slots.file(9, SlotKind::Save), session::write(&arena("Zed"))).unwrap();
        assert!(list.any());
        assert_eq!(list.count(), 3);
        // Touch moves a listed number to the front with its name and
        // stores the order; an unknown number changes nothing.
        list.touch(1);
        list.touch(42);
        assert_eq!(list.numbers(), &[1, 3, 2]);
        assert_eq!(list.names()[0], Some("Ash".into()));
        assert_eq!(slots.stored_order(), Some(vec![1, 3, 2]));
        // Select sets the current slot from the cached list only.
        assert_eq!(list.select(2), Some(2));
        assert_eq!(list.current(), 2);
        assert_eq!(list.select(3), None);
        // Delete by index removes both files and leaves current at 0.
        assert_eq!(list.delete_index(1).unwrap(), Some(3));
        assert_eq!(list.numbers(), &[1, 2]);
        assert_eq!((list.count(), list.current()), (2, 0));
        assert!(!slots.file(3, SlotKind::Save).exists());
        // The free number fills the gap at 3 only after 1 and 2: the list
        // is two long, so 3 is one past it.
        assert_eq!(list.create(None).unwrap(), 3);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_cached_death_drops_the_slot_only_without_a_checkpoint() {
        let dir = folder("cache-death");
        let slots = Slots::arena(&dir);
        let mut list = SlotList::new(slots.clone());
        let first = list.create(Some("Ash")).unwrap();
        slots.save(first, &arena("Ash")).unwrap();
        // A living player changes nothing; the single 0.01 widens to just
        // under the double 0.01, so it already counts as dead.
        assert!(!list.drop_after_death(0.0101).unwrap());
        assert!(slots.file(first, SlotKind::Save).is_file());
        // With a checkpoint the save goes, the slot stays current.
        slots.copy_save_over_checkpoint(first).unwrap();
        assert!(!list.drop_after_death(0.0).unwrap());
        assert!(!slots.file(first, SlotKind::Save).exists());
        assert_eq!((list.count(), list.current()), (1, first as i32));
        // The load of the current slot comes back from the checkpoint and
        // copies it over the save.
        let loaded = list.load_current(false).unwrap();
        assert_eq!(loaded.kind, SlotKind::Checkpoint);
        assert!(slots.file(first, SlotKind::Save).is_file());
        // Without a checkpoint the slot leaves the list.
        fs::remove_file(slots.file(first, SlotKind::Checkpoint)).unwrap();
        assert!(list.drop_after_death(0.0).unwrap());
        assert_eq!((list.count(), list.current()), (0, 0));
        assert!(list.load_current(false).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_folder_that_cannot_be_listed_stays_unscanned() {
        let dir = folder("cache-missing").join("absent");
        let mut list = SlotList::new(Slots::story(&dir));
        assert!(!list.any());
        assert_eq!(list.count(), -1);
        assert!(list.create(None).is_err());
    }
}
