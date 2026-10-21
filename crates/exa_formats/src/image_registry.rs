//! The image loader registry: the original's table of formats, keyed by the
//! data's own tag and by file extension, through which an image is loaded.
//!
//! The engine registers `.rfi` ("Rayform Image") at start-up; other formats
//! came with their own readers. This module keeps the same shape, with the
//! loaders as plain Rust functions over bytes: the registry holds entries,
//! lookup is by extension or by the tag word, and a load returns owned image
//! data or an error. No game data is needed to register one.
//!
//! The tag word of the shipped Rayform reader is Exarust's own key (the
//! original's numeric key is not part of any file, and no shipped data
//! carries it): [`rfi::TAG`].

use crate::rfi::{ImageData, RFI_MAGIC, Rfi};
use crate::{Error, Result};

/// One registered image format.
#[derive(Clone, Copy)]
pub struct ImageFormat {
    /// The registry's numeric key, looked up against a file tag.
    pub tag: u32,
    /// The extension without a dot, lower case (`"rfi"`).
    pub extension: &'static str,
    /// The name the loader list shows ("Rayform Image").
    pub description: &'static str,
    /// Reads the bytes.
    pub loader: fn(&[u8]) -> Result<ImageData>,
}

impl std::fmt::Debug for ImageFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImageFormat")
            .field("tag", &self.tag)
            .field("extension", &self.extension)
            .field("description", &self.description)
            .finish_non_exhaustive()
    }
}

/// The loader table.
#[derive(Debug, Clone, Default)]
pub struct LoaderRegistry {
    entries: Vec<ImageFormat>,
}

impl LoaderRegistry {
    /// An empty table.
    pub fn new() -> Self {
        Self::default()
    }

    /// A table with the engine's start-up formats registered: the Rayform
    /// Image reader for `.rfi`.
    pub fn standard() -> Self {
        let mut registry = Self::new();
        assert!(registry.register(crate::rfi::format()));
        registry
    }

    /// Registers one format. A later registration of the same tag or the
    /// same extension replaces the old entry (the original's register call
    /// overwrites a key). Returns `false` when `format.loader` is not a
    /// Rayform reader but its extension claims `rfi`, which would shadow
    /// the start-up reader; such an entry is refused.
    pub fn register(&mut self, format: ImageFormat) -> bool {
        if format.extension.eq_ignore_ascii_case("rfi") && format.tag != crate::rfi::TAG {
            return false;
        }
        self.entries
            .retain(|e| e.tag != format.tag && !e.extension.eq_ignore_ascii_case(format.extension));
        self.entries.push(format);
        true
    }

    /// The entry registered under an extension (with or without a dot,
    /// case-insensitive).
    pub fn by_extension(&self, extension: &str) -> Option<&ImageFormat> {
        let extension = extension.trim_start_matches('.');
        self.entries
            .iter()
            .find(|e| e.extension.eq_ignore_ascii_case(extension))
    }

    /// The entry registered under a numeric tag.
    pub fn by_tag(&self, tag: u32) -> Option<&ImageFormat> {
        self.entries.iter().find(|e| e.tag == tag)
    }

    /// Loads an image of the named extension.
    pub fn load(&self, extension: &str, bytes: &[u8]) -> Result<ImageData> {
        let entry = self.by_extension(extension).ok_or_else(|| {
            Error::Unsupported(format!("no image loader for extension {extension:?}"))
        })?;
        (entry.loader)(bytes)
    }

    /// Loads an image whose reader is registered under `tag`.
    pub fn load_tag(&self, tag: u32, bytes: &[u8]) -> Result<ImageData> {
        let entry = self
            .by_tag(tag)
            .ok_or_else(|| Error::Unsupported(format!("no image loader for tag {tag:#010x}")))?;
        (entry.loader)(bytes)
    }

    /// Every registered extension, in registration order.
    pub fn extensions(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.entries.iter().map(|e| e.extension)
    }

    /// The number of formats.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Reads a `.rfi` file into owned image data.
pub fn load_rfi(bytes: &[u8]) -> Result<ImageData> {
    ImageData::copy_of(&Rfi::parse(bytes)?)
}

/// The tag the Rayform Image reader registers under: the file magic's low
/// word, so a file's own magic identifies its reader class. (The original's
/// numeric key is not stored in any file; this key is Exarust's.)
pub const fn rfi_tag() -> u32 {
    RFI_MAGIC
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rfi::RFI_MAGIC;

    fn entry(extension: &'static str) -> ImageFormat {
        ImageFormat {
            tag: 0x1234,
            extension,
            description: "test",
            loader: |_| Err(Error::Unsupported("test".into())),
        }
    }

    #[test]
    fn the_standard_table_has_the_rayform_reader() {
        let registry = LoaderRegistry::standard();
        assert_eq!(registry.len(), 1);
        assert_eq!(registry.extensions().collect::<Vec<_>>(), ["rfi"]);
        let rfi = registry.by_extension("RFI").unwrap();
        assert_eq!(rfi.description, "Rayform Image");
        assert_eq!(rfi.tag, RFI_MAGIC);
        assert_eq!(rfi_tag(), RFI_MAGIC);
        assert!(registry.by_extension(".rfi").is_some());
        assert!(registry.by_tag(RFI_MAGIC).is_some());
        assert!(registry.by_extension("dds").is_none());
    }

    #[test]
    fn registering_replaces_the_same_key() {
        let mut registry = LoaderRegistry::standard();
        assert!(!registry.register(entry("rfi")));
        assert_eq!(registry.len(), 1);
        assert!(registry.register(entry("png")));
        assert_eq!(registry.len(), 2);
        assert!(registry.register(entry("png")));
        assert_eq!(registry.len(), 2);
        assert_eq!(registry.extensions().collect::<Vec<_>>(), ["rfi", "png"]);
    }

    #[test]
    fn loading_an_unregistered_extension_is_an_error() {
        let registry = LoaderRegistry::standard();
        assert!(registry.load("dds", &[]).is_err());
        assert!(registry.load_tag(0, &[]).is_err());
    }

    #[test]
    fn a_short_rfi_is_an_error_not_a_panic() {
        let registry = LoaderRegistry::standard();
        assert!(registry.load("rfi", &[0; 4]).is_err());
        let mut header = Vec::new();
        for v in [RFI_MAGIC, 2, 2, 1, 0x0100_C600, 0, 0x1000_0000, 12] {
            header.extend_from_slice(&v.to_le_bytes());
        }
        header.extend_from_slice(&[1; 12]);
        let image = registry.load("rfi", &header).unwrap();
        assert_eq!(image.pixels.len(), 12);
    }
}
