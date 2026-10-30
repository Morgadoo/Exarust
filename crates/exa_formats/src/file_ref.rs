//! The original's file plumbing: the shared folder table, compact file
//! references (a folder index and a name kept inline up to 16 characters),
//! the name rules built on them (entry stem, extension swap, full path), the
//! directory prefixes it creates for a path, whole-file reads and writes (by
//! path, by reference and over the search path), and the registry of file
//! formats.
//!
//! Paths are the original's text: bytes with `\` between folders. Rules:
//! `docs/formats/SETTINGS.md` (File plumbing).

use std::path::{Path, PathBuf};

use crate::mem_block::MemBlock;
use crate::text;

/// Longest name a file reference keeps inline.
pub const INLINE_NAME: usize = 16;

/// The folder table every file reference indexes. Entries keep a trailing
/// separator and a use count; an index is 1-based, 0 meaning "no folder".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FolderTable {
    entries: Vec<Folder>,
}

/// One registered folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Folder {
    pub path: Vec<u8>,
    pub uses: u32,
}

impl FolderTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// The registered folders, oldest first.
    pub fn folders(&self) -> &[Folder] {
        &self.entries
    }

    /// The folder at a 1-based index.
    pub fn get(&self, index: u32) -> Option<&Folder> {
        (index as usize)
            .checked_sub(1)
            .and_then(|i| self.entries.get(i))
    }

    /// Registers `path` and returns its 1-based index. A path without a
    /// drive colon is taken relative to `base`. An entry whose stored length
    /// equals the length of `path` as given and whose bytes match the
    /// (possibly prefixed) path over that length gains a use instead; since
    /// entries store a trailing separator, a path given without one is never
    /// matched and is added again, as in the original.
    pub fn register(&mut self, path: &[u8], base: &[u8]) -> u32 {
        let len = path.len();
        let full: Vec<u8> = if find_byte(b':', path) < 0 {
            [base, path].concat()
        } else {
            path.to_vec()
        };
        for (i, entry) in self.entries.iter_mut().enumerate() {
            if entry.path.len() == len && full.get(..len) == Some(&entry.path[..]) {
                entry.uses += 1;
                return i as u32 + 1;
            }
        }
        let mut stored = full;
        if stored.last() != Some(&b'\\') {
            stored.push(b'\\');
        }
        self.entries.push(Folder {
            path: stored,
            uses: 1,
        });
        self.entries.len() as u32
    }

    /// Forgets every folder.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// A file name as a reference keeps it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileName {
    /// Up to 16 bytes, zero-padded.
    Inline([u8; INLINE_NAME]),
    /// A longer name (usually a whole path).
    Long(Vec<u8>),
}

impl FileName {
    /// A name of up to 16 bytes is kept inline, a longer one on its own.
    pub fn new(name: &[u8]) -> Self {
        if name.len() <= INLINE_NAME {
            let mut inline = [0; INLINE_NAME];
            inline[..name.len()].copy_from_slice(name);
            Self::Inline(inline)
        } else {
            Self::Long(name.to_vec())
        }
    }

    /// The name's bytes: an inline name up to its first zero.
    pub fn text(&self) -> &[u8] {
        match self {
            Self::Inline(b) => {
                let end = b.iter().position(|&c| c == 0).unwrap_or(INLINE_NAME);
                &b[..end]
            }
            Self::Long(v) => v,
        }
    }
}

/// A file reference: a folder index (0 for none) and a name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRef {
    pub folder: u32,
    pub name: FileName,
}

impl FileRef {
    pub fn new(folder: u32, name: &[u8]) -> Self {
        Self {
            folder,
            name: FileName::new(name),
        }
    }

    /// The path a whole-file read opens: the folder's path and the name, or
    /// the name alone without a folder. An index past the table opens the
    /// name alone here (the original reads past its table).
    pub fn open_path(&self, table: &FolderTable) -> Vec<u8> {
        let mut path = match self.folder {
            0 => Vec::new(),
            i => table.get(i).map(|f| f.path.clone()).unwrap_or_default(),
        };
        path.extend_from_slice(self.name.text());
        path
    }

    /// The full path: a long name holding a separator after its first byte
    /// is a path already; otherwise the folder's path (`base` without a
    /// folder, nothing for an index past the table) and the name.
    pub fn full_path(&self, table: &FolderTable, base: &[u8]) -> Vec<u8> {
        if let FileName::Long(name) = &self.name
            && find_last_byte(b'\\', name) > 0
        {
            return name.clone();
        }
        let mut path = match self.folder {
            0 => base.to_vec(),
            i => table.get(i).map(|f| f.path.clone()).unwrap_or_default(),
        };
        path.extend_from_slice(self.name.text());
        path
    }

    /// The 16-byte entry stem: an inline name cut at its last dot (a dot at
    /// the start is kept); a long name from after its last separator to its
    /// last dot, zero-padded, or its first 16 bytes from there when that is
    /// 16 or more. Without a dot the length wraps around the name, which
    /// drops its last byte, as in the original.
    pub fn entry_stem(&self) -> [u8; INLINE_NAME] {
        let mut out = [0; INLINE_NAME];
        match &self.name {
            FileName::Inline(b) => {
                out = *b;
                let dot = find_last_byte(b'.', b);
                if dot > 0 {
                    out[dot as usize..].fill(0);
                }
            }
            FileName::Long(name) => {
                let start = find_last_byte(b'\\', name) + 1;
                let dot = find_last_byte(b'.', name);
                let mut len = dot - start;
                if len < 1 {
                    len += name.len() as i32;
                }
                let take = len.clamp(0, INLINE_NAME as i32) as usize;
                for (i, slot) in out.iter_mut().take(take).enumerate() {
                    *slot = name.get(start as usize + i).copied().unwrap_or(0);
                }
            }
        }
        out
    }
}

/// `name` with its extension replaced by `ext` (up to 16 bytes, cut at a
/// zero): a dot after the first byte and after the last separator keeps
/// the name up to and including it; otherwise a dot and the extension are
/// appended.
pub fn with_extension(name: &[u8], ext: &[u8]) -> Vec<u8> {
    let ext = &ext[..ext.len().min(INLINE_NAME)];
    let ext = &ext[..ext.iter().position(|&c| c == 0).unwrap_or(ext.len())];
    let dot = find_last_byte(b'.', name);
    if dot > 0 && find_last_byte(b'\\', name) < dot {
        return [&name[..dot as usize + 1], ext].concat();
    }
    [name, b".", ext].concat()
}

/// The directories the original creates, in order, to make every folder of
/// `path` (`sep` between folders): the first window starts two bytes after a
/// drive colon (or at the second byte), each later one after the separator
/// just found, and the walk stops when a window opens on a separator or the
/// path ends. Paths of fewer than three bytes create nothing.
pub fn directory_prefixes(path: &[u8], sep: u8) -> Vec<Vec<u8>> {
    let total = path.len() as i32 + 1;
    let mut out = Vec::new();
    if total <= 3 {
        return out;
    }
    let mut start = (find_byte(b':', path) + 3).max(1);
    loop {
        let window = path.get(start as usize - 1..).unwrap_or(&[]);
        let window = &window[..window.len().min((total - start).max(0) as usize)];
        let k = find_byte(sep, window);
        let end = if k >= 0 { start + k } else { total };
        out.push(path[..(end - 1).clamp(0, path.len() as i32) as usize].to_vec());
        start = end + 1;
        if !(k > 0 && start < total) {
            break;
        }
    }
    out
}

/// Reads a whole file. A missing or unreadable file gives `None`; an empty
/// one gives an empty block.
pub fn read_whole_file(path: &Path) -> Option<Vec<u8>> {
    std::fs::read(path).ok()
}

/// Writes a whole block, creating or overwriting the file. An empty block
/// makes an empty file. Whether it all reached the file.
pub fn write_whole_file(path: &Path, bytes: &[u8]) -> bool {
    std::fs::write(path, bytes).is_ok()
}

/// The message the original logs when a file cannot be opened.
pub const OPEN_FAILED: &str = "Failed to open file: ";
/// The message it logs when a file's contents cannot be read.
pub const READ_FAILED: &str = "Unable to read contents of file: ";
/// The log topic its file errors go to.
pub const IO_TOPIC: &str = "File IO";

/// Why a whole-file read left no block: the file could not be opened, or its
/// contents could not be read (in which case the original clears the block).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileIssue {
    Open,
    Read,
}

impl FileIssue {
    /// The line the original logs for this failure and the path or name it
    /// tried, in its error log's third level.
    pub fn message(self, name: &[u8]) -> String {
        let prefix = match self {
            Self::Open => OPEN_FAILED,
            Self::Read => READ_FAILED,
        };
        format!("{prefix}{}", String::from_utf8_lossy(name))
    }
}

/// A block read from a file ([`read_referenced_file`],
/// [`read_from_search_path`]); a failure carries what the original logs.
pub type FileRead = Result<MemBlock, FileIssue>;

/// The original's text path as a filesystem path (its bytes are ANSI text).
fn path_of(name: &[u8]) -> PathBuf {
    PathBuf::from(String::from_utf8_lossy(name).into_owned())
}

/// Whether a path names something that exists (`GetFileAttributesA` says so
/// for a file or a folder).
pub fn file_exists(name: &[u8]) -> bool {
    path_of(name).metadata().is_ok()
}

/// Reads a whole file named by `name`: on an open failure the original logs
/// it and leaves the block alone; on a read failure it logs it and clears
/// the block.
pub fn read_file(name: &[u8]) -> FileRead {
    let mut file = std::fs::File::open(path_of(name)).map_err(|_| FileIssue::Open)?;
    let mut bytes = Vec::new();
    use std::io::Read;
    file.read_to_end(&mut bytes).map_err(|_| FileIssue::Read)?;
    Ok(MemBlock::from_bytes(bytes))
}

/// Writes a whole block to `name`, creating or overwriting the file. An
/// empty block makes an empty file (and counts as written). Whether all of
/// it reached the file.
pub fn write_file(name: &[u8], block: &MemBlock) -> bool {
    write_whole_file(&path_of(name), block.bytes())
}

/// The resource type the executable's own payloads are stored as
/// (`RT_RCDATA`).
pub const RT_RCDATA: u16 = 10;

/// Reads the named resource of `kind` into `block`, as the original reads
/// its executable's `RT_RCDATA` payloads: the platform's resource directory
/// finds the name (an inline name up to its first zero, a long name whole)
/// under the type, and on success the block takes the locked bytes and its
/// resource kind; a missing resource leaves the block as it was. The lookup
/// is the platform part (the Win32 resource calls); the rule and the block
/// are Exarust's.
pub fn read_resource(
    block: &mut MemBlock,
    name: &FileName,
    kind: u16,
    lookup: impl FnOnce(&[u8], u16) -> Option<Vec<u8>>,
) -> bool {
    match lookup(name.text(), kind) {
        Some(bytes) => {
            block.adopt_resource(bytes);
            true
        }
        None => false,
    }
}

/// Reads the whole file a reference names: the folder's path and the name,
/// or the name alone without a folder.
pub fn read_referenced_file(table: &FolderTable, file: &FileRef) -> FileRead {
    read_file(&file.open_path(table))
}

/// Writes a block to the full path a reference gives (a long name holding a
/// separator is a path already; otherwise the folder's path and the name).
pub fn write_referenced_file(
    table: &FolderTable,
    file: &FileRef,
    base: &[u8],
    block: &MemBlock,
) -> bool {
    write_file(&file.full_path(table, base), block)
}

/// Reads the whole file `name` from the newest folder still in use where it
/// exists. The first folder where it exists ends the search even when its
/// read fails, as in the original. On an open failure nothing is read and
/// the original logs the name; on a read failure the block is cleared.
pub fn read_from_search_path(table: &FolderTable, name: &FileName) -> FileRead {
    let issue = std::cell::Cell::new(None);
    let read = read_from_folders(table, name, file_exists, |path| match read_file(path) {
        Ok(block) => Some(block.into_bytes()),
        Err(failure) => {
            issue.set(Some(failure));
            None
        }
    });
    match read {
        Some(bytes) => Ok(MemBlock::from_bytes(bytes)),
        None => match issue.get() {
            Some(failure) => Err(failure),
            None => Err(FileIssue::Open),
        },
    }
}

/// Reads `name` from the newest registered folder, among those still in
/// use, where it exists. The first folder where it exists ends the search
/// even when the read fails.
pub fn read_from_folders(
    table: &FolderTable,
    name: &FileName,
    exists: impl Fn(&[u8]) -> bool,
    read: impl Fn(&[u8]) -> Option<Vec<u8>>,
) -> Option<Vec<u8>> {
    for folder in table.entries.iter().rev() {
        if folder.uses == 0 {
            continue;
        }
        let path = [&folder.path[..], name.text()].concat();
        if exists(&path) {
            return read(&path);
        }
    }
    None
}

/// The registry of file formats: a 16-byte key, a handler, a description
/// and a use count per format.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FormatRegistry<H> {
    entries: Vec<Format<H>>,
}

/// One registered format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Format<H> {
    pub key: [u8; INLINE_NAME],
    pub handler: H,
    pub description: String,
    pub uses: u32,
}

impl<H: Clone> FormatRegistry<H> {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// The formats in registry order.
    pub fn formats(&self) -> &[Format<H>] {
        &self.entries
    }

    /// Adds a format with no uses and returns its index. From the third
    /// entry on it goes in below the run of used formats at the end, never
    /// below index 1.
    pub fn add(&mut self, key: [u8; INLINE_NAME], handler: H, description: &str) -> usize {
        let new = Format {
            key,
            handler,
            description: description.to_string(),
            uses: 0,
        };
        self.entries.push(new.clone());
        let count = self.entries.len();
        let mut slot = count - 1;
        if count >= 3 {
            let mut i = count - 1;
            loop {
                i -= 1;
                if self.entries[i].uses < 1 {
                    break;
                }
                self.entries[i + 1] = self.entries[i].clone();
                slot = i;
                if i <= 1 {
                    break;
                }
            }
        }
        self.entries[slot] = new;
        slot
    }

    /// The first format whose key matches over 16 bytes, ignoring ASCII
    /// case.
    pub fn find(&self, key: &[u8; INLINE_NAME]) -> Option<usize> {
        self.entries
            .iter()
            .position(|f| f.key.eq_ignore_ascii_case(key))
    }

    /// The handler of the format at `index`.
    pub fn handler(&self, index: usize) -> Option<&H> {
        self.entries.get(index).map(|f| &f.handler)
    }

    /// Records a use of the format at `index`.
    pub fn note_use(&mut self, index: usize) {
        if let Some(f) = self.entries.get_mut(index) {
            f.uses += 1;
        }
    }

    /// Forgets every format.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// The 0-based index of the first `c` in `s`, or −1. The original's forward
/// byte search ([`crate::text::find_byte`]).
pub fn find_byte(c: u8, s: &[u8]) -> i32 {
    text::find_byte(c, s)
}

/// The 0-based index of the last `c` in `s`, or −1 ([`crate::text::last_byte`]).
pub fn find_last_byte(c: u8, s: &[u8]) -> i32 {
    if s.is_empty() {
        return -1;
    }
    text::last_byte(c, s)
}

#[cfg(test)]
mod tests {
    use super::*;

    impl FolderTable {
        /// Sets an entry's use count, which only the readers of a file
        /// decrement (their release is outside this pack).
        fn set_uses(&mut self, index: usize, uses: u32) {
            self.entries[index].uses = uses;
        }

        /// Adds an entry as it is: the tests need platform separators, which
        /// [`FolderTable::register`] (as the original) never stores.
        fn push_folder(&mut self, path: &[u8], uses: u32) {
            self.entries.push(Folder {
                path: path.to_vec(),
                uses,
            });
        }
    }

    #[test]
    fn folders_take_the_base_and_a_trailing_separator() {
        let mut t = FolderTable::new();
        assert_eq!(t.register(b"Resource", b"C:\\Game\\"), 1);
        assert_eq!(t.get(1).unwrap().path, b"C:\\Game\\Resource\\");
        assert_eq!(t.register(b"D:\\Data\\", b"C:\\Game\\"), 2);
        // Same length and bytes: a second use.
        assert_eq!(t.register(b"D:\\Data\\", b"C:\\Game\\"), 2);
        assert_eq!(t.get(2).unwrap().uses, 2);
        // Without its separator the same folder is added again.
        assert_eq!(t.register(b"D:\\Data", b""), 3);
        assert_eq!(t.folders().len(), 3);
    }

    #[test]
    fn a_reference_opens_its_folder_and_name() {
        let mut t = FolderTable::new();
        t.register(b"D:\\Data\\", b"");
        let r = FileRef::new(1, b"Exanima.ini");
        assert_eq!(r.open_path(&t), b"D:\\Data\\Exanima.ini");
        assert_eq!(FileRef::new(0, b"a.rpk").open_path(&t), b"a.rpk");
        assert_eq!(
            FileRef::new(0, b"a.rpk").full_path(&t, b"C:\\G\\"),
            b"C:\\G\\a.rpk"
        );
        let long = FileRef::new(1, b"C:\\Users\\me\\Exanima.ini");
        assert_eq!(long.full_path(&t, b""), b"C:\\Users\\me\\Exanima.ini");
        assert!(matches!(long.name, FileName::Long(_)));
        assert!(matches!(FileName::new(&[b'x'; 16]), FileName::Inline(_)));
    }

    #[test]
    fn the_entry_stem_and_extension_rules() {
        assert_eq!(
            &FileRef::new(0, b"humandiff.rfi").entry_stem()[..10],
            b"humandiff\0"
        );
        let long = FileRef::new(0, b"Textures\\Apparel\\cloak01.rfi");
        assert_eq!(&long.entry_stem()[..8], b"cloak01\0");
        // No dot: the wrapped length drops the last byte.
        assert_eq!(
            &FileRef::new(0, b"Textures\\Apparel\\cloak").entry_stem()[..5],
            b"cloa\0"
        );
        assert_eq!(with_extension(b"Exanima.rfp", b"set"), b"Exanima.set");
        assert_eq!(with_extension(b"Exanima", b"ini"), b"Exanima.ini");
        assert_eq!(with_extension(b"a.b\\c", b"x"), b"a.b\\c.x");
        assert_eq!(with_extension(b".hidden", b"x"), b".hidden.x");
    }

    #[test]
    fn the_directory_prefixes_of_a_path() {
        let p = |s: &[u8]| directory_prefixes(s, b'\\');
        assert_eq!(p(b"C:\\a\\b"), [b"C:\\a".to_vec(), b"C:\\a\\b".to_vec()]);
        assert_eq!(p(b"C:\\x\\"), [b"C:\\x".to_vec()]);
        assert_eq!(
            p(b"ab\\cd\\ef"),
            [b"ab".to_vec(), b"ab\\cd".to_vec(), b"ab\\cd\\ef".to_vec()]
        );
        // A separator at the second byte opens the first window: one folder.
        assert_eq!(p(b"a\\b"), [b"a".to_vec()]);
        assert!(p(b"ab").is_empty());
        assert_eq!(
            directory_prefixes(b"/home/u/.config", b'/'),
            [
                b"/home".to_vec(),
                b"/home/u".to_vec(),
                b"/home/u/.config".to_vec()
            ]
        );
    }

    #[test]
    fn the_newest_folder_with_the_file_wins() {
        let mut t = FolderTable::new();
        t.register(b"A\\", b"");
        t.register(b"B\\", b"");
        t.register(b"C\\", b"");
        let name = FileName::new(b"f");
        let exists = |p: &[u8]| p == b"A\\f" || p == b"B\\f";
        let got = read_from_folders(&t, &name, exists, |p| Some(p.to_vec()));
        assert_eq!(got.as_deref(), Some(&b"B\\f"[..]));
        // The first existing folder ends the search even when its read fails.
        assert_eq!(read_from_folders(&t, &name, exists, |_| None), None);
    }

    /// A temporary folder for the file tests; the caller removes it.
    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("exarust-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_whole_file_read_names_its_folder_and_the_search_path() {
        let dir = temp_dir("file-ref-read");
        let folder = dir.to_string_lossy().into_owned().into_bytes();
        let mut t = FolderTable::new();
        // The table stores a trailing separator.
        t.register(&folder, b"");
        let stored = &t.folders()[0].path.clone();
        std::fs::write(
            format!("{}old.txt", String::from_utf8_lossy(stored)),
            b"old",
        )
        .unwrap();
        std::fs::write(
            format!("{}new.txt", String::from_utf8_lossy(stored)),
            b"new",
        )
        .unwrap();
        assert!(file_exists(&[stored.as_slice(), b"new.txt"].concat()));
        assert!(!file_exists(&[stored.as_slice(), b"gone.txt"].concat()));
        // By reference: the folder's path and the name.
        let file = FileRef::new(1, b"new.txt");
        let read = read_referenced_file(&t, &file).unwrap();
        assert_eq!(read.bytes(), b"new");
        assert_eq!(read.capacity(), 3);
        // Without a folder the name is a path of its own (relative here).
        let missing = read_referenced_file(&t, &FileRef::new(0, b"nowhere.txt"));
        assert_eq!(missing, Err(FileIssue::Open));
        assert_eq!(
            FileIssue::Open.message(b"nowhere.txt"),
            "Failed to open file: nowhere.txt"
        );
        assert_eq!(
            FileIssue::Read.message(b"x"),
            "Unable to read contents of file: x"
        );
        // A write by reference, an empty write and a failed open.
        let file = FileRef::new(1, b"out.txt");
        let block = MemBlock::from_bytes(vec![1, 2, 3]);
        assert!(write_referenced_file(&t, &file, b"", &block));
        assert_eq!(
            std::fs::read(format!("{}out.txt", String::from_utf8_lossy(stored))).unwrap(),
            [1, 2, 3]
        );
        assert!(write_referenced_file(&t, &file, b"", &MemBlock::new()));
        assert_eq!(
            std::fs::read(format!("{}out.txt", String::from_utf8_lossy(stored)))
                .unwrap()
                .len(),
            0
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_resource_read_fills_a_resource_block_or_leaves_it() {
        let mut block = MemBlock::from_bytes(vec![1, 2, 3]);
        let name = FileName::new(b"humandiff.rfi");
        let asked = std::cell::RefCell::new(None);
        let ok = read_resource(&mut block, &name, RT_RCDATA, |text, kind| {
            *asked.borrow_mut() = Some((text.to_vec(), kind));
            Some(vec![9, 8, 7, 6])
        });
        assert!(ok);
        assert_eq!(*asked.borrow(), Some((b"humandiff.rfi".to_vec(), 10)));
        assert_eq!(block.bytes(), &[9, 8, 7, 6]);
        assert_eq!(
            (block.size(), block.kind()),
            (4, crate::mem_block::Kind::Resource)
        );
        // Resource memory is only released, never reallocated.
        assert!(!block.resize(8));
        block.resize(0);
        assert_eq!(block.bytes(), b"");
        // A missing resource leaves the block untouched.
        let mut block = MemBlock::from_bytes(vec![1, 2, 3]);
        let missing = read_resource(&mut block, &name, RT_RCDATA, |_, _| None);
        assert!(!missing);
        assert_eq!(block.bytes(), &[1, 2, 3]);
        assert_eq!(block.kind(), crate::mem_block::Kind::Owned);
        // A long name is asked for whole; an inline name stops at its zero.
        let long = FileName::new(b"Textures\\Apparel\\cloak01.rfi");
        let mut seen = Vec::new();
        read_resource(&mut block, &long, RT_RCDATA, |text, _| {
            seen = text.to_vec();
            None
        });
        assert_eq!(seen, b"Textures\\Apparel\\cloak01.rfi");
    }

    #[test]
    fn the_search_path_takes_the_newest_folder_holding_the_file() {
        let dir = temp_dir("file-ref-search");
        let make = |name: &str| {
            let path = dir.join(name);
            std::fs::create_dir_all(&path).unwrap();
            let mut bytes = path.to_string_lossy().into_owned().into_bytes();
            bytes.push(std::path::MAIN_SEPARATOR as u8);
            bytes
        };
        let old = make("old");
        let new = make("new");
        let mut t = FolderTable::new();
        t.push_folder(&old, 1);
        t.push_folder(&new, 1);
        // Only the oldest folder holds it, then only the newest.
        std::fs::write(dir.join("old").join("f.txt"), b"one").unwrap();
        let name = FileName::new(b"f.txt");
        assert_eq!(read_from_search_path(&t, &name).unwrap().bytes(), b"one");
        std::fs::write(dir.join("new").join("f.txt"), b"two").unwrap();
        assert_eq!(read_from_search_path(&t, &name).unwrap().bytes(), b"two");
        // A folder with no uses left is skipped.
        t.set_uses(1, 0);
        assert_eq!(read_from_search_path(&t, &name).unwrap().bytes(), b"one");
        // No folder holds it: nothing is read and the open failed.
        assert_eq!(
            read_from_search_path(&t, &FileName::new(b"gone.txt")),
            Err(FileIssue::Open)
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn formats_go_in_below_the_used_run() {
        let key = |s: &[u8]| {
            let mut k = [0u8; 16];
            k[..s.len()].copy_from_slice(s);
            k
        };
        let mut r = FormatRegistry::new();
        assert_eq!(r.add(key(b"rfc"), 1, "Flux Content"), 0);
        assert_eq!(r.add(key(b"rfi"), 2, "image"), 1);
        r.note_use(1);
        // The third goes below the used entry at index 1.
        assert_eq!(r.add(key(b"rsq"), 3, "skeleton"), 1);
        assert_eq!(r.formats()[2].key, key(b"rfi"));
        assert_eq!(r.find(&key(b"RSQ")), Some(1));
        assert_eq!(r.handler(2), Some(&2));
        assert_eq!(r.find(&key(b"x")), None);
    }
}
