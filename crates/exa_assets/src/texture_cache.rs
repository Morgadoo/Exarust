//! The texture manager: records keyed by a 16-byte, case-folded name.
//!
//! The original keeps its textures in a sorted array searched by name and
//! inserts new ones at the search point. Each record has a reference count;
//! the last release frees it, which is the only eviction (there is no size
//! budget). A second creation request for a live name returns the same
//! record and only replaces the creation word, so the last request's
//! sampler flags win.
//!
//! Exarust keys a record by the name and its colour space: the space is
//! part of the creation word, and Bevy needs separate images for the sRGB
//! and linear variants of one name (the original's single-word record would
//! silently reuse the first upload).

use bevy::prelude::*;

use crate::texture::{ColorSpace, TextureSettings, TextureTarget};

/// A texture name as the original stores it: sixteen bytes, upper-case
/// letters folded to lower case, the rest of a longer name cut off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TextureName(pub [u8; 16]);

impl TextureName {
    pub fn new(name: &str) -> Self {
        let mut bytes = [0u8; 16];
        for (slot, byte) in bytes.iter_mut().zip(name.bytes()) {
            *slot = byte.to_ascii_lowercase();
        }
        Self(bytes)
    }

    /// The name as text, up to the first zero.
    pub fn as_str(&self) -> &str {
        let end = self.0.iter().position(|&b| b == 0).unwrap_or(16);
        std::str::from_utf8(&self.0[..end]).unwrap_or("")
    }
}

impl PartialEq<str> for TextureName {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other.to_ascii_lowercase()
    }
}

/// What a cache entry is keyed by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TextureKey {
    pub name: TextureName,
    pub space: ColorSpace,
}

impl TextureKey {
    pub fn new(name: &str, space: ColorSpace) -> Self {
        Self {
            name: TextureName::new(name),
            space,
        }
    }
}

/// One texture record: what the original keeps between the name key and the
/// GL object.
#[derive(Debug, Clone)]
pub struct TextureRecord {
    pub key: TextureKey,
    /// The creation/sampler word (see [`crate::texture::flags`]).
    pub flags: u32,
    /// Whether the image has an alpha channel worth alpha-testing.
    pub alpha: bool,
    /// The uploaded image, once the loader has finished it.
    pub image: Option<Handle<Image>>,
    /// The original's reference count; the record is freed at zero.
    pub references: u32,
}

impl TextureRecord {
    pub fn target(&self) -> TextureTarget {
        TextureTarget::from_flags(self.flags)
    }

    pub fn is_ready(&self) -> bool {
        self.image.is_some()
    }
}

/// A name-keyed texture cache with the original's lookup, insert and release.
#[derive(Default)]
pub struct TextureCache {
    entries: Vec<TextureRecord>,
    settings: TextureSettings,
}

/// What [`TextureCache::acquire`] found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Acquired {
    /// Whether this call created the record (the caller must load and
    /// [`TextureCache::finish`] it).
    pub created: bool,
}

impl TextureCache {
    pub fn new(settings: TextureSettings) -> Self {
        Self {
            entries: Vec::new(),
            settings,
        }
    }

    pub fn settings(&self) -> &TextureSettings {
        &self.settings
    }

    pub fn set_settings(&mut self, settings: TextureSettings) {
        self.settings = settings;
    }

    /// The records in name order, as the manager's array holds them.
    pub fn entries(&self) -> &[TextureRecord] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The binary search the manager runs: `Ok(index)` on a hit,
    /// `Err(index)` at the position a new record goes.
    fn search(&self, key: &TextureKey) -> Result<usize, usize> {
        self.entries.binary_search_by(|record| record.key.cmp(key))
    }

    /// Finds a record and takes a reference, or creates one with a single
    /// reference. A new record has no image yet.
    ///
    /// On a hit the creation word is replaced by `flags`, as the original
    /// does before it looks at the image.
    pub fn acquire(&mut self, key: TextureKey, flags: u32) -> Acquired {
        match self.search(&key) {
            Ok(index) => {
                let record = &mut self.entries[index];
                record.references += 1;
                record.flags = flags;
                Acquired { created: false }
            }
            Err(index) => {
                self.entries.insert(
                    index,
                    TextureRecord {
                        key,
                        flags,
                        alpha: false,
                        image: None,
                        references: 1,
                    },
                );
                Acquired { created: true }
            }
        }
    }

    /// Takes a reference without creating a record: the original's lookup
    /// used where a missing name is not an error.
    pub fn find(&mut self, key: &TextureKey) -> Option<&TextureRecord> {
        let index = self.search(key).ok()?;
        let record = &mut self.entries[index];
        record.references += 1;
        Some(record)
    }

    pub fn get(&self, key: &TextureKey) -> Option<&TextureRecord> {
        self.search(key).ok().map(|index| &self.entries[index])
    }

    pub fn get_mut(&mut self, key: &TextureKey) -> Option<&mut TextureRecord> {
        match self.search(key) {
            Ok(index) => Some(&mut self.entries[index]),
            Err(_) => None,
        }
    }

    /// Stores the uploaded image (the loader's finish step).
    pub fn finish(&mut self, key: &TextureKey, image: Handle<Image>, alpha: bool) {
        if let Some(record) = self.get_mut(key) {
            record.image = Some(image);
            record.alpha = alpha;
        }
    }

    /// Gives up a reference; the record is freed when the last one goes.
    /// `true` when this call freed the record.
    pub fn release(&mut self, key: &TextureKey) -> bool {
        let Ok(index) = self.search(key) else {
            return false;
        };
        let record = &mut self.entries[index];
        record.references = record.references.saturating_sub(1);
        if record.references == 0 {
            self.entries.remove(index);
            return true;
        }
        false
    }

    /// References held by a record, for tests and diagnostics.
    pub fn references(&self, key: &TextureKey) -> u32 {
        self.get(key).map_or(0, |record| record.references)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(name: &str) -> TextureKey {
        TextureKey::new(name, ColorSpace::Srgb)
    }

    #[test]
    fn names_fold_case_and_cut_at_sixteen_bytes() {
        assert_eq!(TextureName::new("GrassPalA01D").as_str(), "grasspala01d");
        assert_eq!(
            &TextureName::new("A_Very_Long_Texture_Name").0,
            b"a_very_long_text"
        );
        // The comparison is the 16 stored bytes, not the text.
        assert_eq!(key("Wall01D"), key("wall01d"));
        assert_ne!(key("wall01d"), key("wall01n"));
    }

    #[test]
    fn the_cache_stays_sorted_and_counts_references() {
        let mut cache = TextureCache::default();
        for name in ["stone01d", "brick02d", "wood03d"] {
            assert!(cache.acquire(key(name), 0).created);
        }
        let names: Vec<&str> = cache
            .entries()
            .iter()
            .map(|e| e.key.name.as_str())
            .collect();
        assert_eq!(names, ["brick02d", "stone01d", "wood03d"]);
        // A second acquire is a hit: one more reference, no new record.
        assert!(!cache.acquire(key("stone01d"), 0x70000).created);
        assert_eq!(cache.len(), 3);
        assert_eq!(cache.references(&key("stone01d")), 2);
        assert_eq!(cache.get(&key("stone01d")).unwrap().flags, 0x70000);
        // Release frees only the last reference, and the array closes up.
        assert!(!cache.release(&key("stone01d")));
        assert!(cache.release(&key("stone01d")));
        assert!(cache.get(&key("stone01d")).is_none());
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn a_missing_name_can_be_looked_up_without_creating_it() {
        let mut cache = TextureCache::default();
        assert!(cache.find(&key("nope")).is_none());
        assert!(cache.is_empty());
        cache.acquire(key("wall01n"), 0);
        assert!(cache.find(&key("wall01n")).is_some());
        assert_eq!(cache.references(&key("wall01n")), 2);
        assert!(!cache.release(&key("missing")));
    }

    #[test]
    fn the_colour_space_is_part_of_the_key() {
        let mut cache = TextureCache::default();
        cache.acquire(TextureKey::new("wall01", ColorSpace::Srgb), 0);
        cache.acquire(TextureKey::new("wall01", ColorSpace::Linear), 0);
        assert_eq!(cache.len(), 2);
        // Ordered by name, then by space (sRGB before linear).
        assert_eq!(cache.entries()[0].key.space, ColorSpace::Srgb);
        assert_eq!(cache.entries()[1].key.space, ColorSpace::Linear);
    }

    #[test]
    fn the_finish_step_stores_the_image() {
        let mut cache = TextureCache::default();
        let key = key("wall01d");
        cache.acquire(key, 0);
        assert!(!cache.get(&key).unwrap().is_ready());
        let image = Handle::default();
        cache.finish(&key, image.clone(), true);
        let record = cache.get(&key).unwrap();
        assert!(record.is_ready());
        assert!(record.alpha);
        assert_eq!(record.image.as_ref(), Some(&image));
    }
}
