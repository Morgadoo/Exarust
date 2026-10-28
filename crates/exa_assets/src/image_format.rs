//! The texture loader's image-reader dispatch.
//!
//! The original keeps two registries of image sources: a list of image
//! providers searched by name first, and a list of file readers by the name's
//! extension. A name a provider holds is read by that provider; otherwise the
//! loader reads the file's bytes and hands them to the reader its extension
//! names, showing a "File format error" message, "Unrecognised file
//! extension: …", when no reader is registered for it. A reader that cannot
//! decode the bytes fails the load without a message.
//!
//! Exarust's installation is the provider: [`exa_rpk::GameData::texture`]
//! finds an image by its bare name and its `.rfi` extension before this
//! dispatch runs. `.rfi` is the only file reader Exarust has, so the registry
//! holds that one reader; every other extension is reported and its record
//! falls back to the original's grey placeholder. A name without a dot stands
//! for an image a provider holds (the texture packs' bare names): the
//! provider's own reader decodes it, which here means the `.rfi` reader when
//! the bytes carry its magic.

use std::fmt;

use exa_formats::FileKind;

/// An image file reader Exarust can dispatch to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// The engine's image format (`exa_formats::rfi`).
    Rfi,
}

impl ImageFormat {
    /// The extensions the reader is registered under, in lookup order.
    pub const EXTENSIONS: &'static [&'static str] = &["rfi"];

    /// The extension of the file this format reads.
    pub fn extension(self) -> &'static str {
        match self {
            ImageFormat::Rfi => "rfi",
        }
    }
}

/// Why a named image cannot be handed to the `.rfi` decoder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageReadError {
    /// No reader is registered for the name's extension. The original
    /// shows `Unrecognised file extension: <extension>` (at most 15 bytes,
    /// as written in the name) under the title `File format error`.
    UnrecognisedExtension(String),
    /// A provider's image (a name without a dot) whose bytes are not an
    /// image the loader knows. The original's provider reader fails without
    /// a message; Exarust's warning says `File format error`.
    Format,
}

impl fmt::Display for ImageReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImageReadError::UnrecognisedExtension(extension) => {
                write!(f, "Unrecognised file extension: {extension}")
            }
            ImageReadError::Format => f.write_str("File format error"),
        }
    }
}

impl std::error::Error for ImageReadError {}

/// The extension a name is dispatched by: `None` without a dot, else the
/// bytes after the last dot (a leading dot counts too). An extension longer
/// than the original's 16-byte field is not copied at all, so it is empty,
/// as is the extension of a name ending in a dot; an empty extension matches
/// no reader. The reader lookup folds ASCII case.
pub fn extension_of(name: &str) -> Option<&str> {
    let dot = name.rfind('.')?;
    let extension = &name[dot + 1..];
    Some(if extension.len() > 16 { "" } else { extension })
}

/// The extension as the original's message prints it: the first 15 bytes,
/// in the name's own case.
fn reported_extension(extension: &str) -> String {
    let mut end = extension.len().min(15);
    while !extension.is_char_boundary(end) {
        end -= 1;
    }
    extension[..end].to_owned()
}

/// The reader a named image goes to, before its bytes are decoded. A name
/// with an extension goes to the reader registered for it, whether or not
/// the bytes match; a name without one is dispatched by its magic. This is
/// the original's order: the extension registry first, then the file
/// reader's own format check.
pub fn reader_for(name: &str, bytes: &[u8]) -> Result<ImageFormat, ImageReadError> {
    if let Some(extension) = extension_of(name) {
        return if extension.eq_ignore_ascii_case(ImageFormat::Rfi.extension()) {
            Ok(ImageFormat::Rfi)
        } else {
            Err(ImageReadError::UnrecognisedExtension(reported_extension(
                extension,
            )))
        };
    }
    match FileKind::detect(bytes) {
        FileKind::Image => Ok(ImageFormat::Rfi),
        _ => Err(ImageReadError::Format),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bytes of a minimal image header: the engine's image magic and
    /// nothing else, enough for the magic dispatch and not for decoding.
    const IMAGE_MAGIC: [u8; 4] = [0xC6, 0x3D, 0x2D, 0x1D];

    #[test]
    fn the_extension_is_the_part_after_the_last_dot() {
        assert_eq!(extension_of("wall01n.rfi"), Some("rfi"));
        assert_eq!(extension_of("a.b.rfi"), Some("rfi"));
        assert_eq!(extension_of("wall01n"), None);
        // A leading dot starts an extension too; a trailing dot leaves it
        // empty.
        assert_eq!(extension_of(".rfi"), Some("rfi"));
        assert_eq!(extension_of("wall01n."), Some(""));
        // Sixteen bytes fit the field; seventeen are not copied at all.
        assert_eq!(
            extension_of("wall01n.abcdefghijklmnop"),
            Some("abcdefghijklmnop")
        );
        assert_eq!(extension_of("wall01n.abcdefghijklmnopq"), Some(""));
    }

    #[test]
    fn a_known_extension_picks_the_rfi_reader_whatever_the_bytes() {
        assert_eq!(
            reader_for("wall01n.rfi", b"not an image"),
            Ok(ImageFormat::Rfi)
        );
        // The extension comparison folds case, as the lowercased stored name
        // does.
        assert_eq!(reader_for("WALL01N.RFI", b""), Ok(ImageFormat::Rfi));
    }

    #[test]
    fn an_unknown_extension_reports_its_name_and_no_reader() {
        assert_eq!(
            reader_for("emblem.dds", b"DDS "),
            Err(ImageReadError::UnrecognisedExtension("dds".into()))
        );
        // The message keeps the name's case and at most 15 bytes.
        assert_eq!(
            reader_for("a.b.TGA", b""),
            Err(ImageReadError::UnrecognisedExtension("TGA".into()))
        );
        assert_eq!(
            reader_for("x.abcdefghijklmnop", b""),
            Err(ImageReadError::UnrecognisedExtension(
                "abcdefghijklmno".into()
            ))
        );
        // An empty extension matches no reader, whatever the bytes.
        assert_eq!(
            reader_for("wall01n.", &IMAGE_MAGIC),
            Err(ImageReadError::UnrecognisedExtension(String::new()))
        );
        assert_eq!(
            ImageReadError::UnrecognisedExtension("dds".into()).to_string(),
            "Unrecognised file extension: dds"
        );
    }

    #[test]
    fn a_bare_name_is_dispatched_by_its_magic() {
        assert_eq!(reader_for("wall01n", &IMAGE_MAGIC), Ok(ImageFormat::Rfi));
        assert_eq!(reader_for("spatter01", &IMAGE_MAGIC), Ok(ImageFormat::Rfi));
        // A name without an extension and without image magic is the file
        // reader's format error.
        assert_eq!(
            reader_for("notes", b"plain text"),
            Err(ImageReadError::Format)
        );
        assert_eq!(
            reader_for("notes", &[0x01, 0x0C, 0xBF, 0xAF]),
            Err(ImageReadError::Format)
        );
        assert_eq!(ImageReadError::Format.to_string(), "File format error");
    }
}
