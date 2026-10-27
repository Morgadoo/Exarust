//! The save game's settings chunk and its easing curve.
//!
//! The chunk is the header `0xECADCA01`, a twelve-byte tag and two dwords;
//! the old flag is set when the tag's version float is below one million.
//! The curve is two quadratics joined at 0.5. The slot file number is the
//! original's padded decimal (zero is `000`, a negative is `00`).
//! Evidence: the retail routines behind it (`docs/formats/OTHER.md`,
//! Saves).

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

/// The saved settings word a load clears.
pub const LOAD_SETTINGS_WORD: u32 = 0;

/// The twelve-byte settings tag a load leaves in place: a zero, the version
/// `1_100_000.0` and a zero.
pub const LOAD_SETTINGS_TAG: [f32; 3] = [0.0, 1_100_000.0, 0.0];

/// What the save load resets globally. `old` is the old-tag flag (clear),
/// `tag` the twelve-byte tag, `saved` the settings chunk's saved word and
/// `sun_height` the scene's sun height (zero).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoadReset {
    pub old: bool,
    pub tag: [f32; 3],
    pub saved: u32,
    pub sun_height: f32,
}

/// The globals the load resets before it opens a level: the old-tag flag
/// clear, the default tag, a zero saved word and the scene's sun height
/// zero.
pub fn load_reset() -> LoadReset {
    LoadReset {
        old: false,
        tag: LOAD_SETTINGS_TAG,
        saved: LOAD_SETTINGS_WORD,
        sun_height: 0.0,
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

/// Formats a save number as a zero-padded three-character decimal, exactly
/// as the original's formatter does: zero is the constant `"000"`; otherwise
/// the decimal digits are produced from the least significant up and the
/// string is prefixed with `"00"` when shorter than two characters, or `"0"`
/// when shorter than three. A negative value produces no digits, so it comes
/// out as `"00"`; the scanner never produces one.
pub fn format_save_number(value: i32) -> String {
    if value == 0 {
        return "000".to_string();
    }
    let mut text = String::new();
    let mut value = value;
    while value > 0 {
        let digit = (value % 10) as u8;
        text.insert(0, char::from(b'0' + digit));
        value /= 10;
    }
    match text.len() {
        0 | 1 => text.insert_str(0, "00"),
        2 => text.insert(0, '0'),
        _ => {}
    }
    text
}

/// The story mode's value in the engine's mode word.
pub const MODE_STORY: u32 = 4;

/// The arena mode's value in the engine's mode word.
pub const MODE_ARENA: u32 = 8;

/// The `Exanima.set` key the story's most-recently-used slot order is kept
/// under. A guess in the sense that the key's owner is not traced; the key
/// itself is the manager's.
pub const STORY_ORDER_KEY: u32 = 0x6431_ac03;

/// The arena's most-recently-used slot order key.
pub const ARENA_ORDER_KEY: u32 = 0x6431_ac13;

/// A player below this health is dead for the save rules. The original
/// widens the single-precision health to double and compares it with the
/// double 0.01, so the single nearest 0.01 (just below it) already counts
/// as dead.
pub const DEAD_HEALTH: f64 = 0.01;

/// The most-recently-used slot order the scanner installs: the stored
/// numbers that are still slots, in their stored order, then every other
/// slot in the order the scan found it. A stored number that is no longer
/// a slot is ignored, and a repeat is only taken once.
pub fn slot_order(numbers: &[u32], stored: &[u32]) -> Vec<u32> {
    let mut out = Vec::with_capacity(numbers.len());
    for &wanted in stored {
        if out.contains(&wanted) {
            continue;
        }
        if numbers.contains(&wanted) {
            out.push(wanted);
        }
    }
    for &number in numbers {
        if !out.contains(&number) {
            out.push(number);
        }
    }
    out
}

/// What the manager stores after an ordering change: the list only when it
/// holds at least two slots, since the original skips the write otherwise.
pub fn order_to_store(numbers: &[u32]) -> Option<Vec<u32>> {
    if numbers.len() < 2 {
        None
    } else {
        Some(numbers.to_vec())
    }
}

/// The number a new slot takes: the lowest number from 1 through the list's
/// length that no slot uses, or one past the length when all are taken.
/// Numbers above the length are not considered, as the original's mark
/// array is only as long as the list.
pub fn free_slot_number(numbers: &[u32]) -> u32 {
    let mut used = vec![false; numbers.len()];
    for &number in numbers {
        if number >= 1 && number as usize <= numbers.len() {
            used[number as usize - 1] = true;
        }
    }
    match used.iter().position(|&taken| !taken) {
        Some(index) => index as u32 + 1,
        None => numbers.len() as u32 + 1,
    }
}

/// Inserts a new slot at the front of the list.
pub fn insert_slot(numbers: &mut Vec<u32>, number: u32) {
    numbers.insert(0, number);
}

/// Removes the slot at `index`, shifting the rest down. Returns its number.
pub fn delete_slot(numbers: &mut Vec<u32>, index: usize) -> Option<u32> {
    if index < numbers.len() {
        Some(numbers.remove(index))
    } else {
        None
    }
}

/// The slot a death drops: the current one when no checkpoint file exists.
/// The original only removes the slot from the list; the files stay.
pub fn dead_slot_without_checkpoint(current: u32, checkpoint_exists: bool) -> Option<u32> {
    if current > 0 && !checkpoint_exists {
        Some(current)
    } else {
        None
    }
}

/// Whether a save is written: unless the caller skips the check, the
/// health widened to double must not be below the double 0.01. A NaN health
/// is unordered, which the original's branch treats as "not below", so it
/// saves; write it as the negation of that test, not `>=`.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn save_allowed(health: f32, forced: bool) -> bool {
    forced || !(f64::from(health) < DEAD_HEALTH)
}

/// Whether a checkpoint is written: only when there is none yet.
pub fn checkpoint_due(checkpoint_exists: bool) -> bool {
    !checkpoint_exists
}

/// Whether the `CheckpointSave` script function saves: only when it is
/// called for the player, and only while checkpoints are enabled. It always
/// reports success for the player; the checkpoint file itself is written
/// when none exists.
pub fn checkpoint_script_runs(for_player: bool, enabled: bool) -> bool {
    for_player && enabled
}

/// The game clock: whole seconds saved at load, plus the milliseconds since
/// then at one second each.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GameClock {
    saved: u32,
    base: u32,
}

impl GameClock {
    /// A clock at zero.
    pub fn new() -> Self {
        Self::default()
    }

    /// The clock a load restores, its base at the current tick.
    pub fn restore(saved: u32, ticks_ms: u32) -> Self {
        Self {
            saved,
            base: ticks_ms,
        }
    }

    /// The saved seconds.
    pub fn saved(&self) -> u32 {
        self.saved
    }

    /// The tick the saved value was restored at.
    pub fn base(&self) -> u32 {
        self.base
    }

    /// The clock at `ticks_ms`: the saved seconds plus the whole seconds
    /// since the base. The original subtracts the zero-extended 32-bit words
    /// in 64 bits and divides that **signed** difference by 1000, rounding
    /// toward zero, so a tick below the base gives a negative count (the
    /// clock runs back) rather than a huge one; the sum is kept to 32 bits.
    pub fn now(&self, ticks_ms: u32) -> u32 {
        let elapsed = (i64::from(ticks_ms) - i64::from(self.base)) / 1000;
        self.saved.wrapping_add(elapsed as u32)
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

    #[test]
    fn the_save_number_is_padded_to_three_digits() {
        assert_eq!(format_save_number(0), "000");
        assert_eq!(format_save_number(5), "005");
        assert_eq!(format_save_number(12), "012");
        assert_eq!(format_save_number(123), "123");
        assert_eq!(format_save_number(1234), "1234");
        // The original's digit loop does not run for a negative value, so
        // only the two-character pad is added.
        assert_eq!(format_save_number(-7), "00");
    }

    #[test]
    fn the_slot_order_takes_the_stored_front_then_the_rest() {
        assert_eq!(slot_order(&[3, 1, 2], &[]), vec![3, 1, 2]);
        assert_eq!(slot_order(&[3, 1, 2], &[2]), vec![2, 3, 1]);
        assert_eq!(slot_order(&[3, 1, 2], &[1, 3]), vec![1, 3, 2]);
        // Stored numbers that are gone and repeats are ignored.
        assert_eq!(slot_order(&[3, 1], &[9, 1, 1]), vec![1, 3]);
        assert_eq!(order_to_store(&[4]), None);
        assert_eq!(order_to_store(&[4, 2]), Some(vec![4, 2]));
        assert_eq!(order_to_store(&[]), None);
    }

    #[test]
    fn a_new_slot_takes_the_lowest_free_number() {
        assert_eq!(free_slot_number(&[]), 1);
        assert_eq!(free_slot_number(&[3, 1, 2]), 4);
        assert_eq!(free_slot_number(&[2, 3]), 1);
        assert_eq!(free_slot_number(&[1, 3]), 2);
        // A number above the list's length marks nothing.
        assert_eq!(free_slot_number(&[7]), 1);
        assert_eq!(free_slot_number(&[1, 2, 4]), 3);
    }

    #[test]
    fn slots_are_inserted_at_the_front_and_deleted_by_index() {
        let mut numbers = vec![5, 2];
        let free = free_slot_number(&numbers);
        insert_slot(&mut numbers, free);
        assert_eq!(numbers, vec![1, 5, 2]);
        assert_eq!(delete_slot(&mut numbers, 1), Some(5));
        assert_eq!(numbers, vec![1, 2]);
        assert_eq!(delete_slot(&mut numbers, 2), None);
    }

    #[test]
    fn a_dead_player_without_a_checkpoint_loses_the_slot() {
        assert_eq!(dead_slot_without_checkpoint(4, false), Some(4));
        assert_eq!(dead_slot_without_checkpoint(4, true), None);
        assert_eq!(dead_slot_without_checkpoint(0, false), None);
        // 0.01 as a single is 0.00999999977648…, below the double 0.01.
        assert!(!save_allowed(0.01, false));
        assert!(save_allowed(0.010_001, false));
        assert!(!save_allowed(0.009_999, false));
        assert!(save_allowed(f32::NAN, false));
        assert!(save_allowed(0.0, true));
        assert!(checkpoint_due(false));
        assert!(!checkpoint_due(true));
        assert!(checkpoint_script_runs(true, true));
        assert!(!checkpoint_script_runs(false, true));
        assert!(!checkpoint_script_runs(true, false));
        assert_eq!(MODE_STORY, 4);
        assert_eq!(MODE_ARENA, 8);
        assert_eq!(STORY_ORDER_KEY, 0x6431_ac03);
        assert_eq!(ARENA_ORDER_KEY, 0x6431_ac13);
    }

    #[test]
    fn the_game_clock_counts_whole_seconds_from_its_saved_base() {
        let clock = GameClock::restore(120, 5_000);
        assert_eq!(clock.now(5_000), 120);
        assert_eq!(clock.now(5_999), 120);
        assert_eq!(clock.now(6_000), 121);
        assert_eq!(clock.now(65_000), 180);
        assert_eq!(GameClock::new().now(12_345), 12);
        // A tick before the base is a negative difference, divided signed
        // and rounded toward zero: −500 ms is 0 s, −2000 and −2001 ms are
        // −2 s, and −1000 ms from a zero clock wraps the word to 2^32 − 1.
        assert_eq!(clock.now(4_500), 120);
        assert_eq!(clock.now(3_000), 118);
        assert_eq!(clock.now(2_999), 118);
        assert_eq!(GameClock::restore(0, 1_000).now(0), u32::MAX);
        // The millisecond counter wrapping past the base: 1000 − 4294966296
        // is −4294965296 ms, −4294965 s, and 120 − 4294965 wraps to
        // 4290672451.
        assert_eq!(
            GameClock::restore(120, 4_294_966_296).now(1_000),
            4_290_672_451
        );
    }

    #[test]
    fn the_load_resets_the_settings_tag_and_scene_height() {
        let reset = load_reset();
        assert!(!reset.old);
        assert_eq!(reset.tag, [0.0, 1_100_000.0, 0.0]);
        assert_eq!(reset.saved, 0);
        assert_eq!(reset.sun_height, 0.0);
    }
}
