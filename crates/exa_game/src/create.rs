//! Character creation's record movement.
//!
//! A new record takes the appearance: four words, a count and a parts list,
//! then a tail and its list. The source keeps its words and loses both
//! counts and lists. Evidence: the 228-byte `0x10012bd20`.

/// A character creation record's appearance.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CharacterAppearance {
    pub words: [u64; 4],
    pub count: i32,
    pub parts: Vec<u32>,
    pub tail: i32,
    pub tail_parts: Vec<u32>,
}

/// Moves the appearance out of `source` into a new record.
pub fn move_appearance(source: &mut CharacterAppearance) -> CharacterAppearance {
    let moved = CharacterAppearance {
        words: source.words,
        count: source.count,
        parts: std::mem::take(&mut source.parts),
        tail: source.tail,
        tail_parts: std::mem::take(&mut source.tail_parts),
    };
    source.count = 0;
    source.tail = 0;
    moved
}

/// The creation screen's appearance labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreationLabel {
    None,
    HumanFemale,
    HumanMale,
    Ham,
    Om,
    Ml,
}

impl CreationLabel {
    /// The literal the original stores, or none.
    pub fn code(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::HumanFemale => Some("HF"),
            Self::HumanMale => Some("HM"),
            Self::Ham => Some("HAM"),
            Self::Om => Some("OM"),
            Self::Ml => Some("ML"),
        }
    }
}

/// Picks the appearance label from the record's word.
pub fn creation_label(word: i32) -> CreationLabel {
    match word & 0xff00 {
        0 => {
            if word & 1 != 0 {
                CreationLabel::HumanFemale
            } else {
                CreationLabel::HumanMale
            }
        }
        0x200 => CreationLabel::Ham,
        0x300 => CreationLabel::Om,
        0x400 => CreationLabel::Ml,
        _ => CreationLabel::None,
    }
}

/// Clears the first used creation slot and the slots after it.
pub fn clear_creation_slots(slots: &mut [[u8; 8]; 6]) -> Option<usize> {
    let first = slots
        .iter()
        .position(|slot| u16::from_le_bytes([slot[0], slot[1]]) != 0)?;
    let slot = &mut slots[first];
    slot[0] = 0;
    slot[1] = 0;
    for byte in &mut slot[3..] {
        *byte = 0;
    }
    if first < 5 {
        for later in &mut slots[first + 1..5] {
            *later = [0; 8];
        }
    }
    Some(first)
}

/// What settling the creation appearance did.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreationSettle {
    pub stored: Option<f32>,
    pub pending: bool,
    pub last: i32,
    pub callback: bool,
}

/// The distance under which nothing is written.
const SETTLE_EPSILON: f32 = f32::from_bits(0x33ff_ffd7);

fn unchanged(last: i32) -> CreationSettle {
    CreationSettle {
        stored: None,
        pending: false,
        last,
        callback: false,
    }
}

fn debounce(value: f32, last: i32, clock: i32) -> CreationSettle {
    if last + 2 > clock {
        CreationSettle {
            stored: Some(value),
            pending: true,
            last,
            callback: false,
        }
    } else {
        CreationSettle {
            stored: Some(value),
            pending: false,
            last: clock,
            callback: true,
        }
    }
}

/// Settles an appearance value toward its target.
pub fn settle_appearance(source: f32, target: f32, last: i32, clock: i32) -> CreationSettle {
    let value = (0.5 - source) * 2.0;
    if (value - target).abs() < SETTLE_EPSILON {
        return unchanged(last);
    }
    debounce(value, last, clock)
}

/// Copies an appearance value to its target.
pub fn copy_appearance(source: f32, target: f32, last: i32, clock: i32) -> CreationSettle {
    if (source - target).abs() < SETTLE_EPSILON {
        return unchanged(last);
    }
    debounce(source, last, clock)
}

/// The creation change's inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationChangeInput {
    pub enabled: bool,
    pub female: bool,
    pub word: i32,
    pub has_child_object: bool,
    pub global_flag: bool,
}

/// What the creation change did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationChange {
    pub word: i32,
    pub label: CreationLabel,
    pub text_set: bool,
    pub child_call: bool,
    pub global_call: bool,
    pub first_pass: bool,
    pub second_pass: bool,
    pub callback: bool,
    pub finalized: bool,
}

/// Applies the sex option to the record word and refreshes the screen.
pub fn apply_creation_change(input: CreationChangeInput) -> CreationChange {
    if !input.enabled {
        return CreationChange {
            word: input.word,
            label: CreationLabel::None,
            text_set: false,
            child_call: false,
            global_call: false,
            first_pass: false,
            second_pass: false,
            callback: false,
            finalized: true,
        };
    }
    let word = if input.female {
        input.word | 1
    } else {
        input.word & !1
    };
    CreationChange {
        word,
        label: creation_label(word),
        text_set: true,
        child_call: input.has_child_object,
        global_call: !input.global_flag,
        first_pass: true,
        second_pass: true,
        callback: true,
        finalized: true,
    }
}

/// What the creation look shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationLook {
    pub values: [i32; 4],
    pub letter: Option<u8>,
}

/// Updates the sex toggles and the face variant caption.
pub fn update_creation_look(female: bool, variant: i32) -> CreationLook {
    if female {
        return CreationLook {
            values: [16; 4],
            letter: None,
        };
    }
    let letter = if variant < 0 {
        None
    } else {
        Some((variant + 0x41) as u8)
    };
    CreationLook {
        values: [0; 4],
        letter,
    }
}

/// Scales three normalised values to bytes.
pub fn appearance_bytes(values: [f32; 3]) -> [u8; 3] {
    values.map(|value| (value * 255.0).round_ties_even() as i64 as u8)
}

/// Scales two normalised values to 16-bit words.
pub fn appearance_words(values: [f32; 2]) -> [u16; 2] {
    values.map(|value| (value * 65535.0).round_ties_even() as i64 as u16)
}

/// What stepping the creation variant did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationStep {
    pub word: i32,
    pub changed: bool,
    pub caption: Option<u8>,
    pub default_caption: bool,
    pub refreshed: bool,
    pub finalized: bool,
}

/// Steps the face variant down one.
pub fn step_creation_variant(word: i32, has_widget: bool) -> CreationStep {
    let variant = (word >> 20) & 0xf;
    if variant < 1 {
        return CreationStep {
            word,
            changed: false,
            caption: None,
            default_caption: false,
            refreshed: false,
            finalized: true,
        };
    }
    let next = variant - 1;
    let word = (word & !0x00f0_0000) | (next << 20);
    let mut caption = None;
    let mut default_caption = false;
    if has_widget {
        let caption_variant = next - 1;
        if caption_variant < 0 {
            default_caption = true;
        } else {
            caption = Some((caption_variant + 0x41) as u8);
        }
    }
    CreationStep {
        word,
        changed: true,
        caption,
        default_caption,
        refreshed: true,
        finalized: true,
    }
}

/// Steps the face variant up one, to a maximum of six.
pub fn advance_creation_variant(word: i32, has_widget: bool) -> CreationStep {
    let variant = (word >> 20) & 0xf;
    if variant > 5 {
        return CreationStep {
            word,
            changed: false,
            caption: None,
            default_caption: false,
            refreshed: false,
            finalized: true,
        };
    }
    let next = variant + 1;
    let word = (word & !0x00f0_0000) | (next << 20);
    let caption = if has_widget {
        Some(((next - 1) + 0x41) as u8)
    } else {
        None
    };
    CreationStep {
        word,
        changed: true,
        caption,
        default_caption: false,
        refreshed: true,
        finalized: true,
    }
}

/// The creation object's opening defaults.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreationDefaults {
    pub count: i32,
    pub stride: i32,
    pub slots: [[i32; 4]; 2],
    pub first: i32,
    pub second: i32,
    pub value: f32,
    pub flags: i32,
}

/// The two option entries and the fixed words the creation object opens with.
pub fn creation_defaults() -> CreationDefaults {
    CreationDefaults {
        count: 2,
        stride: 16,
        slots: [[0x100101, 0x5fff, 0, 0], [0x201, 0x5fff, 0, 0]],
        first: 0x100101,
        second: 0x201,
        value: 4.0,
        flags: 0x510000,
    }
}

/// A binding behind a creation signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationBinding {
    pub id: i32,
    pub parts: [u8; 4],
}

/// A creation signature to match against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationSignature {
    pub id: i32,
    pub parts: [u8; 4],
    pub binding: Option<CreationBinding>,
}

/// Matches a query against a signature.
pub fn signature_matches(query: i32, signature: &CreationSignature) -> bool {
    if query == signature.id {
        return true;
    }
    match signature.binding {
        Some(binding) => binding.id == signature.id && binding.parts == signature.parts,
        None => false,
    }
}

/// The creation rail's laid-out values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationRail {
    pub view: i32,
    pub thumb: Option<u16>,
    pub start: Option<u16>,
}

/// Lays out the option rail from the count.
pub fn lay_out_creation_rail(
    count: i32,
    unit: i32,
    width: i32,
    before: i32,
    compact: bool,
) -> CreationRail {
    let view = width - before - unit * count;
    if compact {
        CreationRail {
            view,
            thumb: None,
            start: None,
        }
    } else {
        CreationRail {
            view,
            thumb: Some(((count - 4) * unit) as u16),
            start: Some((width - (count - 2) * unit) as u16),
        }
    }
}

/// A creation continue button's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationContinue {
    pub label: &'static str,
    pub mirrored: bool,
    pub enabled: bool,
}

/// Picks the continue label and flags from the mode.
pub fn creation_continue_button(mode: i32, threshold: i32) -> CreationContinue {
    let save = mode == 2;
    CreationContinue {
        label: if save {
            "Continue from save"
        } else {
            "Continue from checkpoint"
        },
        mirrored: save,
        enabled: mode >= threshold,
    }
}

/// A creation part node reduced to what the search reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartNode {
    pub kind: i32,
    pub wanted_count: i32,
    pub attachments: Vec<i32>,
    pub children: Vec<PartNode>,
}

/// Searches the part tree for the `0xFCA0` tag.
pub fn has_creation_part(node: &PartNode) -> bool {
    if node.kind & 0x89 == 0x89 && node.wanted_count >= 0 && node.attachments.contains(&0xfca0) {
        return true;
    }
    node.children.iter().any(has_creation_part)
}

/// What selecting a creation frame did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationFrame {
    pub changed: bool,
    pub current: i32,
    pub part_found: bool,
    pub refreshed: bool,
}

/// Selects the requested frame one lower.
pub fn select_creation_frame(requested: i32, current: i32, part_found: bool) -> CreationFrame {
    if requested < 1 || current == requested - 1 {
        return CreationFrame {
            changed: false,
            current,
            part_found: false,
            refreshed: false,
        };
    }
    CreationFrame {
        changed: true,
        current: requested - 1,
        part_found,
        refreshed: true,
    }
}

/// What refreshing the creation label did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationRefresh {
    pub ran: bool,
    pub built: bool,
    pub drew: bool,
    pub last: i32,
}

/// Refreshes the creation label at most once per 300 units.
pub fn refresh_creation_label(clock: i32, last: i32, has_widget: bool) -> CreationRefresh {
    if clock - last < 300 {
        return CreationRefresh {
            ran: false,
            built: false,
            drew: false,
            last,
        };
    }
    CreationRefresh {
        ran: true,
        built: !has_widget,
        drew: true,
        last: clock,
    }
}

/// What rebuilding the creation list did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationList {
    pub built: bool,
    pub index: i32,
    pub replaced: bool,
    pub stored: bool,
    pub caption: Option<u8>,
    pub drew: bool,
    pub finalized: bool,
}

/// Rebuilds the option list for an index.
pub fn rebuild_creation_list(
    index: i32,
    old_exists: bool,
    new_count: i32,
    has_caption_widget: bool,
    drawing_allowed: bool,
) -> CreationList {
    if new_count <= 0 {
        return CreationList {
            built: false,
            index,
            replaced: false,
            stored: false,
            caption: None,
            drew: false,
            finalized: true,
        };
    }
    CreationList {
        built: true,
        index,
        replaced: old_exists,
        stored: true,
        caption: has_caption_widget.then_some((index + 0x41) as u8),
        drew: drawing_allowed,
        finalized: true,
    }
}

/// What rebuilding the second creation list did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationSecondList {
    pub banded: bool,
    pub built: bool,
    pub index: i32,
    pub replaced: bool,
    pub stored: bool,
    pub caption: Option<u8>,
    pub drew: bool,
    pub finalized: bool,
}

/// Rebuilds the second option list, or stores only the index on a banded record.
pub fn rebuild_creation_second_list(
    index: i32,
    banded: bool,
    old_exists: bool,
    new_count: i32,
    has_caption_widget: bool,
    drawing_allowed: bool,
) -> CreationSecondList {
    if banded {
        return CreationSecondList {
            banded: true,
            built: false,
            index,
            replaced: false,
            stored: true,
            caption: None,
            drew: false,
            finalized: true,
        };
    }
    if new_count <= 0 {
        return CreationSecondList {
            banded: false,
            built: false,
            index,
            replaced: false,
            stored: false,
            caption: None,
            drew: false,
            finalized: true,
        };
    }
    CreationSecondList {
        banded: false,
        built: true,
        index,
        replaced: old_exists,
        stored: true,
        caption: has_caption_widget.then_some((index + 0x41) as u8),
        drew: drawing_allowed,
        finalized: true,
    }
}

/// The creation record's cleared fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationReset {
    pub words: [i32; 5],
    pub tail: i32,
    pub reset_size: i32,
}

/// Clears the record's five words and tail.
pub fn reset_creation_record() -> CreationReset {
    CreationReset {
        words: [0; 5],
        tail: 0,
        reset_size: 8,
    }
}

/// The three fixed creation slots.
pub fn creation_slot_defaults() -> [[u8; 8]; 3] {
    [
        [0xa8, 0x61, 1, 2, 3, 4, 8, 0],
        [0xa8, 0x61, 1, 2, 3, 4, 5, 0],
        [0xa8, 0x61, 1, 2, 3, 4, 6, 0],
    ]
}

/// The creation view's deferred callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationDeferred {
    pub fired: bool,
    pub stamp: i32,
    pub pending: bool,
    pub result: bool,
}

/// Runs the deferred callback when its stamp allows.
pub fn flush_creation_deferred(pending: bool, stamp: i32, clock: i32) -> CreationDeferred {
    if !pending || stamp + 1 >= clock {
        return CreationDeferred {
            fired: false,
            stamp,
            pending,
            result: true,
        };
    }
    CreationDeferred {
        fired: true,
        stamp: clock,
        pending: false,
        result: true,
    }
}

/// Releases the tip when present.
pub fn release_creation_tip(has_tip: bool) -> bool {
    has_tip
}

/// Stores the view and attaches its callback when present.
pub fn attach_creation_view(view: bool) -> bool {
    view
}

/// The creation link copy's debounce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationCopy {
    pub copied: bool,
    pub fired: bool,
    pub stamp: i32,
    pub pending: bool,
}

/// Copies the link word and runs its two-unit debounce.
pub fn copy_creation_link(stamp: i32, clock: i32) -> CreationCopy {
    if stamp + 2 <= clock {
        CreationCopy {
            copied: true,
            fired: true,
            stamp: clock,
            pending: false,
        }
    } else {
        CreationCopy {
            copied: true,
            fired: false,
            stamp,
            pending: true,
        }
    }
}

/// What attaching a creation body wrote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationAttach {
    pub body: bool,
    pub camera: bool,
    pub link: bool,
    pub pane_target: bool,
    pub link_target: bool,
}

/// Stores the body, its camera and link and targets the pane and link.
pub fn attach_creation_body(has_pane: bool, has_link: bool) -> CreationAttach {
    CreationAttach {
        body: true,
        camera: true,
        link: true,
        pane_target: has_pane,
        link_target: has_link,
    }
}

/// What sizing the creation view did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationResize {
    pub count: i32,
    pub size: i32,
    pub stored: bool,
    pub ticked: bool,
}

/// Sizes the view from the current frame.
pub fn resize_creation_view(frame: Option<i32>, unit: i32, has_global: bool) -> CreationResize {
    CreationResize {
        count: frame.map_or(1, |frame| frame + 1),
        size: unit * 12,
        stored: true,
        ticked: has_global,
    }
}

/// The creation view's four refresh calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationViewRefresh {
    pub link: bool,
    pub camera: bool,
    pub first: bool,
    pub second: bool,
}

/// Runs the view's link, camera and two ticks.
pub fn refresh_creation_view() -> CreationViewRefresh {
    CreationViewRefresh {
        link: true,
        camera: true,
        first: true,
        second: true,
    }
}

/// The creation flag's result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationFlag {
    pub word: i32,
    pub refreshed: bool,
    pub part_found: bool,
}

/// Sets or clears bit `0x10000` and stores the part result.
pub fn set_creation_flag(word: i32, flag: bool, part_found: bool) -> CreationFlag {
    CreationFlag {
        word: if flag {
            word | 0x1_0000
        } else {
            word & !0x1_0000
        },
        refreshed: true,
        part_found,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_character_record_moves_its_appearance() {
        let mut source = CharacterAppearance {
            words: [1, 2, 3, 4],
            count: 5,
            parts: vec![7, 8],
            tail: 9,
            tail_parts: vec![10],
        };
        let moved = move_appearance(&mut source);
        assert_eq!(
            moved,
            CharacterAppearance {
                words: [1, 2, 3, 4],
                count: 5,
                parts: vec![7, 8],
                tail: 9,
                tail_parts: vec![10],
            }
        );
        assert_eq!(
            source,
            CharacterAppearance {
                words: [1, 2, 3, 4],
                count: 0,
                parts: Vec::new(),
                tail: 0,
                tail_parts: Vec::new(),
            }
        );
    }

    #[test]
    fn the_creation_appearance_labels_its_bands() {
        assert_eq!(creation_label(0x001), CreationLabel::HumanFemale);
        assert_eq!(creation_label(0x001).code(), Some("HF"));
        assert_eq!(creation_label(0x000), CreationLabel::HumanMale);
        assert_eq!(creation_label(0x000).code(), Some("HM"));
        assert_eq!(creation_label(0x201), CreationLabel::Ham);
        assert_eq!(creation_label(0x201).code(), Some("HAM"));
        assert_eq!(creation_label(0x300), CreationLabel::Om);
        assert_eq!(creation_label(0x4ff), CreationLabel::Ml);
        assert_eq!(creation_label(0x4ff).code(), Some("ML"));
        assert_eq!(creation_label(0x500), CreationLabel::None);
        assert_eq!(creation_label(0x500).code(), None);
    }

    #[test]
    fn the_creation_slots_clear_from_the_first_used() {
        let mut slots = [[0u8; 8]; 6];
        slots[1] = [1, 2, 3, 4, 5, 6, 7, 8];
        slots[5] = [7; 8];
        assert_eq!(clear_creation_slots(&mut slots), Some(1));
        assert_eq!(slots[0], [0; 8]);
        assert_eq!(slots[1], [0, 0, 3, 0, 0, 0, 0, 0]);
        assert_eq!(slots[2], [0; 8]);
        assert_eq!(slots[3], [0; 8]);
        assert_eq!(slots[4], [0; 8]);
        assert_eq!(slots[5], [7; 8]);

        let mut empty = [[0u8; 8]; 6];
        assert_eq!(clear_creation_slots(&mut empty), None);
        assert!(empty.iter().all(|slot| slot == &[0u8; 8]));

        let mut last = [[0u8; 8]; 6];
        last[5] = [7; 8];
        assert_eq!(clear_creation_slots(&mut last), Some(5));
        assert_eq!(last[5], [0, 0, 7, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn the_creation_appearance_settles_toward_its_target() {
        let settled = settle_appearance(0.25, 0.5, 0, 0);
        assert_eq!(settled.stored, None);
        assert!(!settled.pending);
        assert!(!settled.callback);

        let pending = settle_appearance(0.25, 0.0, 100, 101);
        assert_eq!(pending.stored, Some(0.5));
        assert!(pending.pending);
        assert_eq!(pending.last, 100);
        assert!(!pending.callback);

        let fired = settle_appearance(0.25, 0.0, 100, 102);
        assert_eq!(fired.stored, Some(0.5));
        assert!(!fired.pending);
        assert_eq!(fired.last, 102);
        assert!(fired.callback);
    }

    #[test]
    fn the_creation_appearance_copies_with_the_same_debounce() {
        assert_eq!(copy_appearance(0.5, 0.5, 0, 0).stored, None);

        let pending = copy_appearance(0.5, 0.0, 100, 101);
        assert_eq!(pending.stored, Some(0.5));
        assert!(pending.pending);
        assert_eq!(pending.last, 100);
        assert!(!pending.callback);

        let fired = copy_appearance(0.5, 0.0, 100, 102);
        assert_eq!(fired.stored, Some(0.5));
        assert!(!fired.pending);
        assert_eq!(fired.last, 102);
        assert!(fired.callback);
    }

    #[test]
    fn the_creation_change_sets_the_sex_bit_and_refreshes() {
        let female = apply_creation_change(CreationChangeInput {
            enabled: true,
            female: true,
            word: 0,
            has_child_object: true,
            global_flag: false,
        });
        assert_eq!(female.word, 1);
        assert_eq!(female.label, CreationLabel::HumanFemale);
        assert!(female.text_set);
        assert!(female.child_call);
        assert!(female.global_call);
        assert!(female.first_pass);
        assert!(female.second_pass);
        assert!(female.callback);
        assert!(female.finalized);

        let male = apply_creation_change(CreationChangeInput {
            enabled: true,
            female: false,
            word: 1,
            has_child_object: false,
            global_flag: true,
        });
        assert_eq!(male.word, 0);
        assert_eq!(male.label, CreationLabel::HumanMale);
        assert!(!male.child_call);
        assert!(!male.global_call);

        let disabled = apply_creation_change(CreationChangeInput {
            enabled: false,
            female: true,
            word: 0x201,
            has_child_object: true,
            global_flag: false,
        });
        assert_eq!(disabled.word, 0x201);
        assert_eq!(disabled.label, CreationLabel::None);
        assert!(!disabled.text_set);
        assert!(!disabled.child_call);
        assert!(!disabled.global_call);
        assert!(!disabled.first_pass);
        assert!(!disabled.second_pass);
        assert!(!disabled.callback);
        assert!(disabled.finalized);
    }

    #[test]
    fn the_creation_look_toggles_and_letters_the_variant() {
        let female = update_creation_look(true, 0);
        assert_eq!(female.values, [16; 4]);
        assert_eq!(female.letter, None);

        let lettered = update_creation_look(false, 2);
        assert_eq!(lettered.values, [0; 4]);
        assert_eq!(lettered.letter, Some(b'C'));

        let plain = update_creation_look(false, -1);
        assert_eq!(plain.values, [0; 4]);
        assert_eq!(plain.letter, None);
    }

    #[test]
    fn the_creation_appearance_scales_to_bytes_and_words() {
        assert_eq!(appearance_bytes([0.0, 0.5, 1.0]), [0, 128, 255]);
        assert_eq!(appearance_words([0.0, 0.5]), [0, 32768]);
        assert_eq!(appearance_words([1.0, 0.0]), [65535, 0]);
    }

    #[test]
    fn the_creation_variant_steps_down_and_stops_at_zero() {
        let stepped = step_creation_variant(0x0020_0000, true);
        assert_eq!(stepped.word, 0x0010_0000);
        assert!(stepped.changed);
        assert_eq!(stepped.caption, Some(b'A'));
        assert!(!stepped.default_caption);
        assert!(stepped.refreshed);

        let to_zero = step_creation_variant(0x0010_0000, true);
        assert_eq!(to_zero.word, 0);
        assert!(to_zero.default_caption);
        assert_eq!(to_zero.caption, None);

        let stopped = step_creation_variant(0, true);
        assert_eq!(stopped.word, 0);
        assert!(!stopped.changed);
        assert!(!stopped.refreshed);

        let no_widget = step_creation_variant(0x0020_0000, false);
        assert_eq!(no_widget.word, 0x0010_0000);
        assert!(no_widget.changed);
        assert_eq!(no_widget.caption, None);
        assert!(!no_widget.default_caption);
        assert!(no_widget.refreshed);
    }

    #[test]
    fn the_creation_variant_advances_to_six() {
        let advanced = advance_creation_variant(0x0020_0000, true);
        assert_eq!(advanced.word, 0x0030_0000);
        assert!(advanced.changed);
        assert_eq!(advanced.caption, Some(b'C'));
        assert!(advanced.refreshed);

        let to_six = advance_creation_variant(0x0050_0000, true);
        assert_eq!(to_six.word, 0x0060_0000);
        assert_eq!(to_six.caption, Some(b'F'));

        let capped = advance_creation_variant(0x0060_0000, true);
        assert_eq!(capped.word, 0x0060_0000);
        assert!(!capped.changed);
        assert!(!capped.refreshed);

        let no_widget = advance_creation_variant(0x0020_0000, false);
        assert_eq!(no_widget.word, 0x0030_0000);
        assert_eq!(no_widget.caption, None);
        assert!(no_widget.refreshed);
    }

    #[test]
    fn the_creation_defaults_are_fixed() {
        let defaults = creation_defaults();
        assert_eq!(defaults.count, 2);
        assert_eq!(defaults.stride, 16);
        assert_eq!(
            defaults.slots,
            [[0x100101, 0x5fff, 0, 0], [0x201, 0x5fff, 0, 0]]
        );
        assert_eq!(defaults.first, 0x100101);
        assert_eq!(defaults.second, 0x201);
        assert_eq!(defaults.value, 4.0);
        assert_eq!(defaults.flags, 0x510000);
    }

    #[test]
    fn the_creation_signature_matches_its_binding() {
        let binding = CreationBinding {
            id: 7,
            parts: [1, 2, 3, 4],
        };
        let signature = CreationSignature {
            id: 7,
            parts: [1, 2, 3, 4],
            binding: Some(binding),
        };
        assert!(signature_matches(7, &signature));
        assert!(signature_matches(9, &signature));

        let wrong_part = CreationSignature {
            binding: Some(CreationBinding {
                id: 7,
                parts: [1, 2, 3, 5],
            }),
            ..signature
        };
        assert!(!signature_matches(9, &wrong_part));

        let wrong_id = CreationSignature {
            binding: Some(CreationBinding {
                id: 8,
                parts: [1, 2, 3, 4],
            }),
            ..signature
        };
        assert!(!signature_matches(9, &wrong_id));

        let missing = CreationSignature {
            binding: None,
            ..signature
        };
        assert!(!signature_matches(9, &missing));
    }

    #[test]
    fn the_creation_rail_lays_out_its_count() {
        let rail = lay_out_creation_rail(6, 4, 100, 10, false);
        assert_eq!(rail.view, 66);
        assert_eq!(rail.thumb, Some(8));
        assert_eq!(rail.start, Some(84));

        let compact = lay_out_creation_rail(6, 4, 100, 10, true);
        assert_eq!(compact.view, 66);
        assert_eq!(compact.thumb, None);
        assert_eq!(compact.start, None);
    }

    #[test]
    fn the_creation_continue_button_follows_its_mode() {
        let save = creation_continue_button(2, 3);
        assert_eq!(save.label, "Continue from save");
        assert!(save.mirrored);
        assert!(!save.enabled);

        let checkpoint = creation_continue_button(5, 3);
        assert_eq!(checkpoint.label, "Continue from checkpoint");
        assert!(!checkpoint.mirrored);
        assert!(checkpoint.enabled);

        assert!(creation_continue_button(3, 3).enabled);
    }

    #[test]
    fn the_creation_part_search_walks_attachments_and_children() {
        let leaf = |kind, wanted_count, attachments| PartNode {
            kind,
            wanted_count,
            attachments,
            children: Vec::new(),
        };

        assert!(has_creation_part(&leaf(0x89, 2, vec![0x100, 0xfca0])));
        assert!(!has_creation_part(&leaf(0x89, -1, vec![0xfca0])));
        assert!(!has_creation_part(&leaf(0x01, 2, vec![0xfca0])));

        let parent = PartNode {
            kind: 0,
            wanted_count: 0,
            attachments: Vec::new(),
            children: vec![leaf(0x89, 0, vec![0xfca0])],
        };
        assert!(has_creation_part(&parent));

        let empty = PartNode {
            kind: 0,
            wanted_count: 0,
            attachments: Vec::new(),
            children: Vec::new(),
        };
        assert!(!has_creation_part(&empty));
    }

    #[test]
    fn the_creation_frame_follows_the_request() {
        let changed = select_creation_frame(3, 5, true);
        assert!(changed.changed);
        assert_eq!(changed.current, 2);
        assert!(changed.part_found);
        assert!(changed.refreshed);

        let same = select_creation_frame(3, 2, true);
        assert!(!same.changed);
        assert_eq!(same.current, 2);
        assert!(!same.part_found);
        assert!(!same.refreshed);

        let none = select_creation_frame(0, 5, true);
        assert!(!none.changed);
        assert_eq!(none.current, 5);
    }

    #[test]
    fn the_creation_label_refresh_is_throttled() {
        let early = refresh_creation_label(100, 0, false);
        assert!(!early.ran);
        assert!(!early.built);
        assert!(!early.drew);
        assert_eq!(early.last, 0);

        let built = refresh_creation_label(400, 0, false);
        assert!(built.ran);
        assert!(built.built);
        assert!(built.drew);
        assert_eq!(built.last, 400);

        let existing = refresh_creation_label(400, 0, true);
        assert!(existing.ran);
        assert!(!existing.built);
        assert!(existing.drew);
    }

    #[test]
    fn the_creation_list_rebuilds_for_an_index() {
        let built = rebuild_creation_list(2, true, 3, true, true);
        assert!(built.built);
        assert_eq!(built.index, 2);
        assert!(built.replaced);
        assert!(built.stored);
        assert_eq!(built.caption, Some(b'C'));
        assert!(built.drew);
        assert!(built.finalized);

        let empty = rebuild_creation_list(2, true, 0, true, true);
        assert!(!empty.built);
        assert!(!empty.replaced);
        assert!(!empty.stored);
        assert_eq!(empty.caption, None);
        assert!(!empty.drew);
        assert!(empty.finalized);

        let suspended = rebuild_creation_list(2, false, 3, false, false);
        assert!(suspended.built);
        assert!(!suspended.replaced);
        assert_eq!(suspended.caption, None);
        assert!(!suspended.drew);
    }

    #[test]
    fn the_creation_second_list_handles_the_band() {
        let banded = rebuild_creation_second_list(1, true, true, 3, true, true);
        assert!(banded.banded);
        assert!(!banded.built);
        assert!(banded.stored);
        assert!(!banded.drew);

        let built = rebuild_creation_second_list(1, false, true, 3, true, true);
        assert!(!built.banded);
        assert!(built.built);
        assert_eq!(built.index, 1);
        assert!(built.replaced);
        assert!(built.stored);
        assert_eq!(built.caption, Some(b'B'));
        assert!(built.drew);
        assert!(built.finalized);

        let empty = rebuild_creation_second_list(1, false, true, 0, true, true);
        assert!(!empty.built);
        assert!(!empty.stored);
        assert_eq!(empty.caption, None);
        assert!(!empty.drew);
    }

    #[test]
    fn the_creation_record_opens_cleared_with_three_slots() {
        let reset = reset_creation_record();
        assert_eq!(reset.words, [0; 5]);
        assert_eq!(reset.tail, 0);
        assert_eq!(reset.reset_size, 8);

        let slots = creation_slot_defaults();
        assert_eq!(slots[0], [0xa8, 0x61, 1, 2, 3, 4, 8, 0]);
        assert_eq!(slots[1], [0xa8, 0x61, 1, 2, 3, 4, 5, 0]);
        assert_eq!(slots[2], [0xa8, 0x61, 1, 2, 3, 4, 6, 0]);
    }

    #[test]
    fn the_creation_view_ticks_and_attaches() {
        let idle = flush_creation_deferred(false, 10, 12);
        assert!(!idle.fired);
        assert_eq!(idle.stamp, 10);
        assert!(idle.result);

        let waiting = flush_creation_deferred(true, 10, 11);
        assert!(!waiting.fired);
        assert_eq!(waiting.stamp, 10);
        assert!(waiting.pending);

        let fired = flush_creation_deferred(true, 10, 12);
        assert!(fired.fired);
        assert_eq!(fired.stamp, 12);
        assert!(!fired.pending);
        assert!(fired.result);

        assert!(release_creation_tip(true));
        assert!(!release_creation_tip(false));
        assert!(attach_creation_view(true));
        assert!(!attach_creation_view(false));
    }

    #[test]
    fn the_creation_view_copies_attaches_and_sizes() {
        let pending = copy_creation_link(10, 11);
        assert!(pending.copied);
        assert!(!pending.fired);
        assert_eq!(pending.stamp, 10);
        assert!(pending.pending);

        let fired = copy_creation_link(10, 12);
        assert!(fired.fired);
        assert_eq!(fired.stamp, 12);
        assert!(!fired.pending);

        let attached = attach_creation_body(true, true);
        assert!(attached.body);
        assert!(attached.camera);
        assert!(attached.link);
        assert!(attached.pane_target);
        assert!(attached.link_target);

        let bare = attach_creation_body(false, false);
        assert!(!bare.pane_target);
        assert!(!bare.link_target);

        let sized = resize_creation_view(Some(3), 4, true);
        assert_eq!(sized.count, 4);
        assert_eq!(sized.size, 48);
        assert!(sized.stored);
        assert!(sized.ticked);

        let unframed = resize_creation_view(None, 4, false);
        assert_eq!(unframed.count, 1);
        assert!(!unframed.ticked);
    }

    #[test]
    fn the_creation_view_refreshes_and_flags() {
        let refresh = refresh_creation_view();
        assert_eq!(
            refresh,
            CreationViewRefresh {
                link: true,
                camera: true,
                first: true,
                second: true,
            }
        );

        let set = set_creation_flag(0, true, true);
        assert_eq!(set.word, 0x1_0000);
        assert!(set.refreshed);
        assert!(set.part_found);

        let clear = set_creation_flag(0x1_0000, false, false);
        assert_eq!(clear.word, 0);
        assert!(clear.refreshed);
        assert!(!clear.part_found);
    }
}
