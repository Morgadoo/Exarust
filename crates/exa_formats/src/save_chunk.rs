//! The save game's settings chunk and its easing curve.
//!
//! The chunk is the header `0xECADCA01`, a twelve-byte tag and two dwords;
//! the old flag is set when the tag's version float is below one million.
//! The curve is two quadratics joined at 0.5.
//! Evidence: the 90-byte `0x10017d160`, the 140-byte `0x10017d030` and the
//! 104-byte `0x10017d0c0`.

/// The settings chunk's header.
pub const SAVE_SETTINGS_ID: u32 = 0xecad_ca01;

/// Which twelve-byte tag the chunk carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveTag {
    Small,
    Large,
}

/// A written settings chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SaveSettings {
    pub id: u32,
    pub tag: SaveTag,
    pub first: i32,
    pub second: i32,
}

/// Writes the settings chunk header, tag and dwords.
pub fn write_save_settings(tag: SaveTag, first: i32, second: i32) -> SaveSettings {
    SaveSettings {
        id: SAVE_SETTINGS_ID,
        tag,
        first,
        second,
    }
}

/// A read settings chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SaveSettingsRead {
    pub accepted: bool,
    pub old: bool,
    pub first: i32,
    pub second: i32,
}

/// Reads the settings chunk, accepting only its header.
pub fn read_save_settings(id: u32, tag_value: f32, first: i32, second: i32) -> SaveSettingsRead {
    if id != SAVE_SETTINGS_ID {
        return SaveSettingsRead {
            accepted: false,
            old: false,
            first: 0,
            second: 0,
        };
    }
    SaveSettingsRead {
        accepted: true,
        old: 1_000_000.0 > tag_value,
        first,
        second,
    }
}

/// The save game's quadratic easing curve.
pub fn ease_save_curve(x: f32) -> f32 {
    let scaled = x * 2.0;
    if x < 0.5 {
        (scaled * scaled) * 0.5
    } else {
        (((scaled - 3.0) * (scaled - 1.0)) - 1.0) * -0.5
    }
}

/// Whether the save needs conversion.
pub fn save_needs_conversion(
    mode: i32,
    pending: i32,
    legacy: bool,
    entry: bool,
    read: bool,
    tag: u32,
) -> bool {
    if mode != 4 || pending > 0 {
        return false;
    }
    if legacy {
        return true;
    }
    if !entry || !read {
        return false;
    }
    tag != 0xdbac_0d06
}

/// A save record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SaveRecord {
    pub kind: i32,
    pub first: u64,
    pub second: u64,
}

/// The written record table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveRecordTable {
    pub count: i32,
    pub records: Vec<SaveRecord>,
}

/// Writes the count and the records in wire order.
pub fn write_save_records(records: &[SaveRecord]) -> SaveRecordTable {
    SaveRecordTable {
        count: records.len() as i32,
        records: records.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_save_curve_eases_both_halves() {
        assert_eq!(ease_save_curve(0.25), 0.125);
        assert_eq!(ease_save_curve(0.5), 0.5);
        assert_eq!(ease_save_curve(0.75), 0.875);
    }

    #[test]
    fn the_save_settings_chunk_round_trips() {
        let written = write_save_settings(SaveTag::Large, 3, 4);
        assert_eq!(written.id, SAVE_SETTINGS_ID);
        assert_eq!(written.tag, SaveTag::Large);
        assert_eq!(written.first, 3);
        assert_eq!(written.second, 4);

        let read = read_save_settings(SAVE_SETTINGS_ID, 999_999.0, 3, 4);
        assert!(read.accepted);
        assert!(read.old);
        assert_eq!(read.first, 3);
        assert_eq!(read.second, 4);

        let current = read_save_settings(SAVE_SETTINGS_ID, 1_000_000.0, 3, 4);
        assert!(!current.old);

        let rejected = read_save_settings(0x1234_5678, 0.0, 3, 4);
        assert!(!rejected.accepted);
        assert_eq!(rejected.first, 0);
        assert_eq!(rejected.second, 0);
    }

    #[test]
    fn the_save_version_gate_and_record_table() {
        assert!(save_needs_conversion(4, 0, true, false, false, 0));
        assert!(!save_needs_conversion(4, 0, false, true, true, 0xdbac_0d06));
        assert!(save_needs_conversion(4, 0, false, true, true, 0));
        assert!(!save_needs_conversion(3, 0, true, true, true, 0));
        assert!(!save_needs_conversion(4, 1, true, true, true, 0));
        assert!(!save_needs_conversion(4, 0, false, false, true, 0));
        assert!(!save_needs_conversion(4, 0, false, true, false, 0));

        let records = [
            SaveRecord {
                kind: 1,
                first: 2,
                second: 3,
            },
            SaveRecord {
                kind: 4,
                first: 5,
                second: 6,
            },
        ];
        let table = write_save_records(&records);
        assert_eq!(table.count, 2);
        assert_eq!(table.records, records);
    }
}
