//! A company emblem chosen by name.
//!
//! The name `klean` selects background 52 and design 81, and stores the four
//! layer bytes 7, 0, 0 and 6. A picture name keeps a leading 0 and writes the
//! index's tens and ones after it, so 52 is `emblembkg052` and 81 is
//! `emblemdes081`. Any other name selects nothing. A candidate's listed price
//! was not read.
//!
//! Evidence: the name compare, the two picture loaders, and the four bytes
//! stored with that name.

/// Which picture an index names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmblemKind {
    Background,
    Design,
}

/// The pictures and layer bytes a name selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Emblem {
    pub background: u32,
    pub design: u32,
    /// Four bytes stored with the preset. What each byte selects was not read.
    pub layers: [u8; 4],
}

/// The emblem for `name`, if that name is a preset.
pub fn preset_emblem(name: &str) -> Option<Emblem> {
    if name == "klean" {
        Some(Emblem {
            background: 52,
            design: 81,
            layers: [7, 0, 0, 6],
        })
    } else {
        None
    }
}

/// Picture name. The hundreds digit stays 0. The next two are the index
/// divided by 10 and the remainder.
pub fn emblem_texture(kind: EmblemKind, index: u32) -> String {
    let stem = match kind {
        EmblemKind::Background => "emblembkg",
        EmblemKind::Design => "emblemdes",
    };
    let tens = b'0'.wrapping_add((index / 10) as u8);
    let ones = b'0'.wrapping_add((index % 10) as u8);
    format!("{stem}0{}{}", tens as char, ones as char)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn klean_selects_background_52_and_design_81() {
        let emblem = preset_emblem("klean").unwrap();
        assert_eq!(emblem.background, 52);
        assert_eq!(emblem.design, 81);
        assert_eq!(emblem.layers, [7, 0, 0, 6]);
        assert_eq!(
            emblem_texture(EmblemKind::Background, emblem.background),
            "emblembkg052"
        );
        assert_eq!(
            emblem_texture(EmblemKind::Design, emblem.design),
            "emblemdes081"
        );
        assert!(preset_emblem("Klean").is_none());
        assert!(preset_emblem("").is_none());
    }
}
