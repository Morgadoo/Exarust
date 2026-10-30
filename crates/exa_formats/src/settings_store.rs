//! The binary settings store (`<user folder><project>.set`): a tree of
//! nested chunks, each a u32 tag and a u32 size that counts its own 8-byte
//! header, addressed by tag paths.
//!
//! Rules (the original's, `docs/formats/SETTINGS.md`, Settings store):
//! - a lookup walks the path one tag per level: at each level it scans the
//!   siblings in order, enters the first chunk with the tag, and stops at a
//!   sibling of size 0;
//! - a set finds the deepest chunk on the path that exists, then either
//!   resizes the leaf in place (every enclosing chunk's size moves by the
//!   payload's change) or appends the missing chunks at the end of that
//!   deepest chunk (or of the store), adding their bytes to it and to every
//!   chunk around it;
//! - a store read back whose first word is 0 fails its integrity check and
//!   is discarded; an empty store is not written;
//! - the program's exit hooks are a chain: registering puts a hook at the
//!   head, and the settings hook saves and then calls the hook registered
//!   before it.

use std::path::Path;

use crate::{Error, Result};

/// The tag path the player's key bindings are stored under.
pub const BINDINGS_PATH: [u32; 2] = [0x6440_a401, 0x6440_cb01];

/// The message the original logs when a store fails its integrity check.
pub const INTEGRITY_MESSAGE: &str = "Set file integrity check failed.";

/// A chunk header's size.
const HEADER: usize = 8;

/// The settings store.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SettingsStore {
    data: Vec<u8>,
}

/// What reading the store's file gave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedStore {
    pub store: SettingsStore,
    /// The file's first word was 0, so its contents were dropped.
    pub integrity_failed: bool,
}

/// One hook of an exit chain: it runs with the chain's context.
pub type ExitHook<C> = Box<dyn FnMut(&mut C)>;

/// The program's exit hooks: each registration goes to the head of the
/// chain, and a hook calls the one registered before it when it is done.
/// [`ExitHooks::run`] walks the chain from the last registered hook down,
/// which is what the original's own hooks (each ending by calling the saved
/// previous hook) give.
#[derive(Default)]
pub struct ExitHooks<C> {
    hooks: Vec<ExitHook<C>>,
}

impl<C> ExitHooks<C> {
    /// An empty chain.
    pub fn new() -> Self {
        Self { hooks: Vec::new() }
    }

    /// Registers `hook` at the head of the chain.
    pub fn register(&mut self, hook: impl FnMut(&mut C) + 'static) {
        self.hooks.push(Box::new(hook));
    }

    /// How many hooks are registered.
    pub fn len(&self) -> usize {
        self.hooks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.hooks.is_empty()
    }

    /// Runs the chain, latest hook first.
    pub fn run(&mut self, context: &mut C) {
        for hook in self.hooks.iter_mut().rev() {
            hook(context);
        }
    }
}

impl SettingsStore {
    /// An empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// The store over raw bytes, unchecked.
    pub fn from_bytes(data: Vec<u8>) -> Self {
        Self { data }
    }

    /// The store as the original reads its file: contents whose first word
    /// is 0 fail the check and leave the store empty. A file shorter than a
    /// word reads its missing bytes as 0.
    pub fn load(bytes: Vec<u8>) -> LoadedStore {
        if !bytes.is_empty() {
            let mut word = [0u8; 4];
            for (w, b) in word.iter_mut().zip(&bytes) {
                *w = *b;
            }
            if u32::from_le_bytes(word) == 0 {
                return LoadedStore {
                    store: Self::new(),
                    integrity_failed: true,
                };
            }
        }
        LoadedStore {
            store: Self { data: bytes },
            integrity_failed: false,
        }
    }

    /// The bytes to write back, or `None` for an empty store, which the
    /// original does not write.
    pub fn save(&self) -> Option<&[u8]> {
        (!self.data.is_empty()).then_some(&self.data[..])
    }

    /// Writes the store back as the original's exit save does: an empty
    /// store is not written at all. Whether the file holds the store
    /// afterwards (true when there was nothing to write).
    pub fn save_to(&self, path: &Path) -> bool {
        match self.save() {
            Some(bytes) => crate::file_ref::write_whole_file(path, bytes),
            None => true,
        }
    }

    /// The raw bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.data
    }

    /// The payload at `path`, when every tag on it exists. A leaf whose size
    /// is below its header or runs past the store gives `None`.
    pub fn get(&self, path: &[u32]) -> Option<&[u8]> {
        let found = self.descend(path);
        if path.is_empty() || found.matched != path.len() {
            return None;
        }
        let at = found.at?;
        let size = self.size_at(at)? as usize;
        self.data.get(at + HEADER..at.checked_add(size)?)
    }

    /// Stores `payload` at `path`, creating the chunks that are missing.
    /// An empty path changes nothing; a tree whose sizes do not nest is an
    /// error.
    pub fn set(&mut self, path: &[u32], payload: &[u8]) -> Result<()> {
        let count = path.len();
        if count == 0 {
            return Ok(());
        }
        let n = payload.len() as i64;
        let found = self.descend(path);
        let matched = found.matched;
        let mut grow = ((count - matched) * HEADER) as i64 + n;
        let total = self.data.len();
        let write_at = match found.at {
            None => {
                self.data.resize(total + grow as usize, 0);
                total
            }
            Some(node) => {
                let node_size = self.size_at(node).ok_or_else(malformed)? as usize;
                let node_end = node + node_size;
                if node_size < HEADER || node_end > total {
                    return Err(malformed());
                }
                let ancestors = self.ancestors(node)?;
                if matched == count {
                    let new_end = node + HEADER + payload.len();
                    let delta = n - (node_size - HEADER) as i64;
                    for &a in &ancestors {
                        self.add_size(a, delta);
                    }
                    if delta < 0 {
                        self.data.copy_within(node_end..total, new_end);
                        self.data.truncate((total as i64 + delta) as usize);
                    } else if delta > 0 {
                        self.data.resize((total as i64 + delta) as usize, 0);
                        self.data.copy_within(node_end..total, new_end);
                    }
                    grow = delta;
                    node
                } else {
                    for &a in &ancestors {
                        self.add_size(a, grow);
                    }
                    self.add_size(node, grow);
                    self.data.resize(total + grow as usize, 0);
                    self.data
                        .copy_within(node_end..total, node_end + grow as usize);
                    node_end
                }
            }
        };
        let mut at = write_at;
        let mut put = |data: &mut Vec<u8>, bytes: &[u8]| {
            data[at..at + bytes.len()].copy_from_slice(bytes);
            at += bytes.len();
        };
        // The missing chunks between the deepest existing one and the leaf.
        for (i, &tag) in path.iter().enumerate().take(count - 1).skip(matched) {
            let size = grow - ((i - matched) * HEADER) as i64;
            put(&mut self.data, &tag.to_le_bytes());
            put(&mut self.data, &(size as u32).to_le_bytes());
        }
        put(&mut self.data, &path[count - 1].to_le_bytes());
        put(&mut self.data, &((n + HEADER as i64) as u32).to_le_bytes());
        put(&mut self.data, payload);
        Ok(())
    }

    /// Walks `path`: how many of its tags were found and the header of the
    /// deepest chunk found (the leaf when all were). A header that does not
    /// fit in the store ends the walk.
    fn descend(&self, path: &[u32]) -> Descent {
        let mut matched = 0;
        let mut deepest = None;
        let Some(&first) = path.first() else {
            return Descent { matched, at: None };
        };
        let mut tag = first;
        let mut cursor = 0usize;
        let mut end = self.data.len();
        while cursor < end {
            let (Some(here), Some(size)) = (self.tag_at(cursor), self.size_at(cursor)) else {
                break;
            };
            if here == tag {
                matched += 1;
                if matched == path.len() {
                    return Descent {
                        matched,
                        at: Some(cursor),
                    };
                }
                deepest = Some(cursor);
                tag = path[matched];
                end = cursor.saturating_add(size as usize);
                cursor += HEADER;
            } else {
                if size == 0 {
                    break;
                }
                cursor = cursor.saturating_add(size as usize);
            }
        }
        Descent {
            matched,
            at: deepest,
        }
    }

    /// The headers of every chunk that encloses the chunk at `node`, outermost
    /// first.
    fn ancestors(&self, node: usize) -> Result<Vec<usize>> {
        let mut out = Vec::new();
        let mut p = 0usize;
        while p < node {
            let size = self.size_at(p).ok_or_else(malformed)? as usize;
            if node < p + size {
                out.push(p);
                p += HEADER;
            } else {
                if size == 0 {
                    return Err(malformed());
                }
                p += size;
            }
        }
        Ok(out)
    }

    fn tag_at(&self, at: usize) -> Option<u32> {
        self.word(at)
    }

    fn size_at(&self, at: usize) -> Option<u32> {
        self.word(at.checked_add(4)?)
    }

    fn word(&self, at: usize) -> Option<u32> {
        let b = self.data.get(at..at.checked_add(4)?)?;
        Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn add_size(&mut self, at: usize, delta: i64) {
        if let Some(size) = self.size_at(at) {
            let new = (size as i64).wrapping_add(delta) as u32;
            self.data[at + 4..at + 8].copy_from_slice(&new.to_le_bytes());
        }
    }
}

struct Descent {
    matched: usize,
    at: Option<usize>,
}

fn malformed() -> Error {
    Error::Malformed("settings store chunks do not nest".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(data: &[u8]) -> Vec<u32> {
        data.chunks(4)
            .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
            .collect()
    }

    #[test]
    fn a_new_path_is_written_whole() {
        let mut s = SettingsStore::new();
        s.set(&BINDINGS_PATH, &[1, 2, 3, 4]).unwrap();
        // Outer size 8 + 8 + 4 = 20; leaf 8 + 4 = 12.
        assert_eq!(
            words(s.bytes()),
            [0x6440_a401, 20, 0x6440_cb01, 12, 0x0403_0201]
        );
        assert_eq!(s.get(&BINDINGS_PATH), Some(&[1, 2, 3, 4][..]));
        assert_eq!(s.get(&[0x6440_a401]).map(<[u8]>::len), Some(12));
        assert_eq!(s.get(&[0x6440_a401, 7]), None);
    }

    #[test]
    fn a_sibling_goes_at_the_end_of_its_parent() {
        let mut s = SettingsStore::new();
        s.set(&[1, 2], &[0xaa; 4]).unwrap();
        s.set(&[9], &[0xbb; 4]).unwrap();
        s.set(&[1, 3], &[0xcc; 8]).unwrap();
        // Chunk 1 grows by 8 + 8 = 16 to 36, and chunk 9 moves after it.
        assert_eq!(
            words(s.bytes()),
            [
                1,
                36,
                2,
                12,
                0xaaaa_aaaa,
                3,
                16,
                0xcccc_cccc,
                0xcccc_cccc,
                9,
                12,
                0xbbbb_bbbb
            ]
        );
        assert_eq!(s.get(&[1, 3]), Some(&[0xcc; 8][..]));
        assert_eq!(s.get(&[9]), Some(&[0xbb; 4][..]));
    }

    #[test]
    fn a_leaf_resizes_and_its_ancestors_follow() {
        let mut s = SettingsStore::new();
        s.set(&[1, 2, 3], &[5; 4]).unwrap();
        s.set(&[8], &[6; 4]).unwrap();
        s.set(&[1, 2, 3], &[7; 12]).unwrap();
        assert_eq!(words(&s.bytes()[..24]), [1, 36, 2, 28, 3, 20]);
        assert_eq!(s.get(&[1, 2, 3]), Some(&[7; 12][..]));
        assert_eq!(s.get(&[8]), Some(&[6; 4][..]));
        s.set(&[1, 2, 3], &[9; 2]).unwrap();
        assert_eq!(words(&s.bytes()[..24]), [1, 26, 2, 18, 3, 10]);
        assert_eq!(s.get(&[1, 2, 3]), Some(&[9; 2][..]));
        assert_eq!(s.get(&[8]), Some(&[6; 4][..]));
        assert_eq!(s.bytes().len(), 26 + 12);
    }

    #[test]
    fn a_middle_chunk_takes_the_missing_levels() {
        let mut s = SettingsStore::new();
        s.set(&[1, 2], &[]).unwrap();
        s.set(&[1, 3, 4], &[1, 2]).unwrap();
        // Chunk 1 gains 8 (chunk 3) + 8 (chunk 4) + 2.
        assert_eq!(words(&s.bytes()[..16]), [1, 34, 2, 8]);
        assert_eq!(words(&s.bytes()[16..32]), [3, 18, 4, 10]);
        assert_eq!(s.get(&[1, 3, 4]), Some(&[1, 2][..]));
    }

    #[test]
    fn the_integrity_check_and_the_empty_store() {
        let failed = SettingsStore::load(vec![0, 0, 0, 0, 1, 2]);
        assert!(failed.integrity_failed);
        assert_eq!(failed.store.save(), None);
        let ok = SettingsStore::load(vec![1, 0, 0, 0, 8, 0, 0, 0]);
        assert!(!ok.integrity_failed);
        assert_eq!(ok.store.save(), Some(&[1, 0, 0, 0, 8, 0, 0, 0][..]));
        assert!(SettingsStore::load(vec![0]).integrity_failed);
        assert!(!SettingsStore::load(Vec::new()).integrity_failed);
    }

    #[test]
    fn a_sibling_of_size_zero_ends_the_scan() {
        // Tag 5 with size 0, then tag 6: the scan stops before 6.
        let mut b = Vec::new();
        for w in [5u32, 0, 6, 12, 42] {
            b.extend_from_slice(&w.to_le_bytes());
        }
        let s = SettingsStore::from_bytes(b);
        assert_eq!(s.get(&[6]), None);
        // So a set of tag 6 appends a new chunk after the store.
        let mut appended = s.clone();
        appended.set(&[6], &[1]).unwrap();
        assert_eq!(&appended.bytes()[20..], &[6, 0, 0, 0, 9, 0, 0, 0, 1]);
        // A leaf that runs past the store is refused.
        let mut bad = SettingsStore::from_bytes(
            [1u32, 100, 2, 50, 0]
                .iter()
                .flat_map(|w| w.to_le_bytes())
                .collect(),
        );
        assert!(bad.set(&[1, 2], &[1]).is_err());
        assert_eq!(bad.get(&[1, 2]), None);
    }

    #[test]
    fn the_exit_save_writes_a_store_and_skips_an_empty_one() {
        let dir = std::env::temp_dir().join(format!("exarust-store-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.set");
        // An empty store is not written: a file already there stays.
        std::fs::write(&path, b"kept").unwrap();
        assert!(SettingsStore::new().save_to(&path));
        assert_eq!(std::fs::read(&path).unwrap(), b"kept");
        // A store with contents is written whole.
        let mut store = SettingsStore::new();
        store.set(&BINDINGS_PATH, &[7; 4]).unwrap();
        assert!(store.save_to(&path));
        assert_eq!(std::fs::read(&path).unwrap(), store.bytes());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_exit_hooks_run_latest_first() {
        let mut hooks: ExitHooks<Vec<&'static str>> = ExitHooks::new();
        assert!(hooks.is_empty());
        hooks.register(|log| log.push("first"));
        hooks.register(|log| log.push("second"));
        hooks.register(|log| log.push("third"));
        assert_eq!(hooks.len(), 3);
        let mut log = Vec::new();
        hooks.run(&mut log);
        // Registration puts each hook at the head, so the last runs first.
        assert_eq!(log, ["third", "second", "first"]);
    }
}
