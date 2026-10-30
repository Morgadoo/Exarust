//! Character creation: the record, its starting kit, the two pages and the
//! flow shared by story, arena recruiting and versus.
//!
//! The creator edits a character record. Its gear is four hand items, a
//! worn list and a carried list; the screen offers the kit its mode and
//! background allow, lets the player spend the account's experience on the
//! six skills, names the character and commits it. The preset of each story
//! background is kept in the settings under its own key.
//!
//! Evidence: REA on the character screen's unit routines, their callers and
//! the record layout (`docs/formats/RFC.md`, Starts, characters and placed
//! objects; `docs/formats/OTHER.md`, Character creation).

use exa_audio::Lcg;
use exa_formats::characters::{
    ARMOUR, CLOSE_COMBAT, Carried, CharacterRecord, ItemRef, Pools, SHIELDS, SKILLS, Skill,
    TECHNIQUE_SLOTS,
};
use exa_ui::form::cvtss2si64;

use crate::experience::{LEVEL_STEP, global_xp_for_worth, global_xp_worth, training_skill};

/// The item id of the braies, the one worn item the story screen keeps.
pub const BRAIES: u32 = 8;
/// A hand item id that some modes treat as empty.
const FIST_ITEM: u32 = 0x1f;

/// One item reference: the item and its per-instance flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GearSlot {
    pub item: u32,
    pub flags: u32,
}

impl GearSlot {
    /// An item reference with no flags.
    pub fn new(item: u32) -> Self {
        Self { item, flags: 0 }
    }

    /// The record's form of the reference.
    pub fn item_ref(self) -> ItemRef {
        ItemRef::new(self.item, self.flags)
    }
}

/// A creator record's gear.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CreationGear {
    /// The four hand items (two loadouts).
    pub hands: [GearSlot; 4],
    /// Worn apparel, innermost first.
    pub worn: Vec<GearSlot>,
    /// Carried inventory items.
    pub carried: Vec<GearSlot>,
}

/// Moves the gear out of `source` into a new record.
///
/// The hand items are copied and stay in the source; both lists move.
pub fn move_creation_gear(source: &mut CreationGear) -> CreationGear {
    CreationGear {
        hands: source.hands,
        worn: std::mem::take(&mut source.worn),
        carried: std::mem::take(&mut source.carried),
    }
}

/// Writes the creator's fields into the record the character spawns from.
///
/// The hand slots, the worn list and the carried list keep each item with
/// its flags, and a new character's three pools and its seed word are set
/// full (the entry writes four `0xffff` halves: yellow, red, focus and the
/// save seed).
pub fn write_created_record(
    record: &mut CharacterRecord,
    name: &str,
    gear: &CreationGear,
    skills: &[Skill; SKILLS],
    full_health: bool,
) {
    record.name = name.to_string();
    record.hands = gear.hands.map(GearSlot::item_ref);
    record.worn = gear.worn.iter().map(|slot| slot.item_ref()).collect();
    record.inventory = gear
        .carried
        .iter()
        .map(|slot| Carried {
            item: slot.item,
            flags: slot.flags,
            extra: 0,
        })
        .collect();
    record.skills = *skills;
    if full_health {
        record.pools = Pools::FULL;
        record.seed = 0xffff;
    }
}

/// Keeps only the braies among the worn items, as the story preset filter
/// does.
fn keep_braies_only(worn: &mut Vec<GearSlot>) {
    worn.retain(|slot| slot.item == BRAIES);
}

/// Applies the mode filter to a creator's gear.
///
/// Story background 0 keeps the character in braies only and clears every
/// hand item. The arena keeps the first two hand items only if the preset
/// kit offers them, clears the last two and keeps braies only. Modes with
/// the 2 bit clear the fist placeholder from the first two hands. Every
/// branch empties the inventory. `arena_kit` is the ids the arena preset
/// offers.
pub fn filter_creation_gear(
    gear: &mut CreationGear,
    game_mode: i32,
    background: i32,
    arena_kit: &[u32],
) {
    if game_mode == 4 && background == 0 {
        for hand in &mut gear.hands {
            hand.item = 0;
        }
        keep_braies_only(&mut gear.worn);
        gear.carried.clear();
    } else if game_mode & 2 == 0 {
        if game_mode == 8 {
            for hand in gear.hands.iter_mut().take(2) {
                if hand.item != 0 && !arena_kit.contains(&hand.item) {
                    hand.item = 0;
                }
            }
            gear.hands[2].item = 0;
            gear.hands[3].item = 0;
            keep_braies_only(&mut gear.worn);
            gear.carried.clear();
        }
    } else {
        for hand in gear.hands.iter_mut().take(2) {
            if hand.item == FIST_ITEM {
                hand.item = 0;
            }
        }
        gear.carried.clear();
    }
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

/// The body form's constructor values: 172 wide with no border, 592 tall
/// (540 in the compact layout), twelve controls in its table, the caption
/// `VHM` and a scale of 1.0, the physique picker at 8, 64 and the tone
/// picker at 8, 328, 160 wide and 160 tall (112 compact). Before the table
/// is built the constructor writes the seventh record's fourth word: 0x7f,
/// or 0x71 compact (its role is not traced); the physique picker reports
/// to [`sync_physique`], the tone picker (style bit 0x80) to
/// [`skin_changed`], and the page's own change goes to the creation form.
/// The constructor writes one image atlas handle into control records 7, 9
/// and 10.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BaseFormData {
    pub width: i32,
    pub height: i32,
    pub border: i32,
    /// The control table's high index (so twelve controls).
    pub controls_high: usize,
    pub caption: &'static str,
    pub scale_bits: u32,
    pub physique: [i32; 2],
    /// The tone picker's left, top, width and height.
    pub tone: [i32; 4],
    /// The word the constructor writes into the control table first.
    pub table_word: u16,
    /// The bit the constructor sets in the tone picker's style word.
    pub tone_style: u32,
    /// The three control records the constructor gives the same image atlas
    /// (each record's own image list, which an image button draws from).
    pub atlas_controls: [usize; 3],
}

pub fn base_form(compact: bool) -> BaseFormData {
    BaseFormData {
        width: 0xac,
        height: if compact { 0x21c } else { 0x250 },
        border: 0,
        controls_high: 0xb,
        caption: "VHM",
        scale_bits: 1.0f32.to_bits(),
        physique: [8, 0x40],
        tone: [8, 0x148, 0xa0, if compact { 0x70 } else { 0xa0 }],
        table_word: if compact { 0x71 } else { 0x7f },
        tone_style: 0x80,
        atlas_controls: [7, 9, 10],
    }
}

/// The hair form's constructor values: 172 by 612 (548 compact) with no
/// border, seven controls in its table, one attribute word of 0x100 (its
/// role is not traced) and the hair-colour picker at 4, 24.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HairFormData {
    pub width: i32,
    pub height: i32,
    pub border: i32,
    /// The control table's high index (so seven controls).
    pub controls_high: usize,
    pub attribute: u32,
    pub picker: [i32; 2],
}

/// The hair page's style list (an icon view under the colour picker).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HairList {
    /// Left, top, width and height in the hair form.
    pub rect: [i32; 4],
    /// The bit set in the view's attribute word.
    pub attribute_bit: u32,
    /// The view's option word.
    pub options: u32,
    /// The icon manager's cell size (twelve GUI units), the view's row
    /// height.
    pub cell: i32,
    /// The icon manager's flag and mode words.
    pub icon_flags: u32,
    pub icon_mode: u32,
    /// The view's entry size, two above the cell.
    pub entry: i32,
    /// The view's row step: two GUI units above the cell.
    pub step: i32,
    /// The four colour bytes the view keeps (all 0xff).
    pub tint: [u8; 4],
}

/// Places the hair style list: 4 in, its top the colour picker's top plus
/// the picker's width plus 40 (as the constructor adds them), the picker's
/// width less 8 wide and the form's height less 216 tall, with a cell of
/// twelve `unit`s. A change of selection runs [`hair_style_changed`]; the
/// view's refresh runs [`hair_icon_refresh`].
pub fn hair_list(picker: [i32; 4], form_height: i32, unit: i32) -> HairList {
    let cell = unit.wrapping_mul(0xc);
    HairList {
        rect: [
            4,
            (i64::from(picker[2]) + i64::from(picker[1]) + 0x28) as i32,
            picker[2].wrapping_sub(8),
            (i64::from(form_height) - 0xd8) as i32,
        ],
        attribute_bit: 0x200,
        options: 0x3020_0800,
        cell,
        icon_flags: 0x100_e608,
        icon_mode: 0x103,
        entry: cell.wrapping_add(2),
        step: unit.wrapping_mul(2).wrapping_add(cell),
        tint: [0xff; 4],
    }
}

pub fn hair_form(compact: bool) -> HairFormData {
    HairFormData {
        width: 0xac,
        height: if compact { 0x224 } else { 0x264 },
        border: 0,
        controls_high: 6,
        attribute: 0x100,
        picker: [4, 0x18],
    }
}

/// What the physique picker's change does in the body form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysiqueSync {
    /// Too soon after the last regeneration: a pending flag is set and
    /// nothing is regenerated (what later clears it is not traced).
    pub pending: bool,
    /// The form's regeneration callback runs.
    pub regenerate: bool,
    /// The clock value of the last regeneration.
    pub last: u32,
}

/// The physique picker's change: its point goes into the record at once;
/// the regeneration waits while the clock is within two units of the last
/// one (the sum is worked in 64 bits).
pub fn sync_physique(
    target: &mut [f32; 2],
    point: [f32; 2],
    last: u32,
    clock: u32,
) -> PhysiqueSync {
    *target = point;
    if u64::from(last) + 2 > u64::from(clock) {
        PhysiqueSync {
            pending: true,
            regenerate: false,
            last,
        }
    } else {
        PhysiqueSync {
            pending: false,
            regenerate: true,
            last: clock,
        }
    }
}

/// The hair list's refresh: takes the icon view's cell height and its
/// scroll step to twelve units, and redraws it when the global the form
/// checks is set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HairIconRefresh {
    pub cell: i32,
    pub step: i32,
    pub regenerate: bool,
}

pub fn hair_icon_refresh(unit: i32, has_texture: bool) -> HairIconRefresh {
    HairIconRefresh {
        cell: unit * 0xc,
        step: unit * 0xc,
        regenerate: has_texture,
    }
}

/// The character record's fields the edit page shows.
#[derive(Debug, Clone, PartialEq)]
pub struct PageSource {
    pub physique: [f32; 2],
    /// −1…1, positive shorter.
    pub height: f32,
    pub age: f32,
    /// The two skin-colour words.
    pub skin: [u16; 2],
    /// The hair colour's red, green and blue bytes.
    pub hair_colour: [u8; 3],
    /// A float the fifth slider shows as it is (its role is not traced).
    pub slider: f32,
    /// The integer the sixth control shows (its role is not traced).
    pub number: i32,
    /// The voice number; the seventh control shows it as the letter `A` + n.
    pub voice: u32,
    /// The voice pitch, 0.9…1.07 (the slider's range).
    pub pitch: f32,
    /// The profession word: bit 0 female, 0x10000 the hair flag, bits
    /// 20–23 the appearance variant plus one, bits 8–15 the race band the
    /// caption's label reads ([`creation_label`]).
    pub word: u32,
    /// The page has a hair-colour picker (the hair page; the body page has
    /// none).
    pub hair_picker: bool,
}

/// What the character-edit page's update pushes into its controls.
#[derive(Debug, Clone, PartialEq)]
pub struct EditPageState {
    /// The physique picker's point.
    pub physique_point: [f32; 2],
    /// The skin picker's point: the words over 65535.
    pub skin_point: [f32; 2],
    /// The first and second buttons' chosen flags (male, female).
    pub male_on: bool,
    pub female_on: bool,
    /// The form's caption: `V` and the record's appearance label.
    pub caption: String,
    /// 0.5 − height × 0.5.
    pub height_slider: f32,
    pub age_slider: f32,
    pub slider: f32,
    pub number: i32,
    /// The seventh control's voice letter (the form also keeps the voice).
    pub voice_letter: u8,
    /// (pitch − 0.9) × 5.88235…, worked in double (the form also keeps
    /// the pitch).
    pub pitch_slider: f32,
    /// With a hair picker: its three channels over 255, worked in double,
    /// set into the picker (whose pick result is cleared first and then
    /// recomputed from the character's hair).
    pub hair_colour: Option<[f32; 3]>,
    /// With a hair picker: the hair buttons' chosen flags (without hair,
    /// with); both false without one.
    pub hair_off_on: bool,
    pub hair_on: bool,
    /// The variant label's letter, or a dash when the variant field is 0.
    pub variant_letter: Option<u8>,
}

/// The character-edit page's update (the base page's refresh, shared by
/// the body and hair pages).
pub fn edit_page(src: &PageSource) -> EditPageState {
    let female = src.word & 1 != 0;
    let hair = src.word & 0x10000 != 0;
    let variant = ((src.word >> 20) & 0xf) as i32 - 1;
    let label = creation_label(src.word as i32).code().unwrap_or("");
    EditPageState {
        physique_point: src.physique,
        skin_point: src
            .skin
            .map(|w| (f64::from(w) * 1.5259021896696422e-05) as f32),
        male_on: !female,
        female_on: female,
        caption: format!("V{label}"),
        height_slider: 0.5 - src.height * 0.5,
        age_slider: src.age,
        slider: src.slider,
        number: src.number,
        voice_letter: (src.voice as u8).wrapping_add(0x41),
        pitch_slider: ((f64::from(src.pitch) - 0.9) * 5.88235294117647) as f32,
        hair_colour: src.hair_picker.then(|| {
            src.hair_colour
                .map(|b| (f64::from(b) * 0.00392156862745098) as f32)
        }),
        hair_off_on: src.hair_picker && !hair,
        hair_on: src.hair_picker && hair,
        variant_letter: (variant >= 0).then(|| (variant as u8).wrapping_add(0x41)),
    }
}

/// What destroying the edit page does: when it holds a watched view it
/// removes the change listener it added to that view before the base
/// form's destroy runs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EditPageDestroy {
    pub hook_removed: bool,
    pub base: bool,
}

pub fn destroy_edit_page(has_view: bool) -> EditPageDestroy {
    EditPageDestroy {
        hook_removed: has_view,
        base: true,
    }
}

/// The skin picker's change: its two coordinates scale to the record's two
/// 16-bit colour words (rounded half to even); the body is regenerated and
/// the preview redrawn.
pub fn skin_changed(src: &mut PageSource, point: [f32; 2]) {
    src.skin = appearance_words(point);
}

/// The hair-colour picker's change: its three coordinates scale to the
/// record's colour bytes (rounded half to even); the body is regenerated
/// and the preview redrawn.
pub fn hair_colour_changed(src: &mut Vec<u8>, point: [f32; 3]) {
    src.clear();
    src.extend(appearance_bytes(point));
}

/// The hair list's change: a selection of 1 or more that is not the
/// record's style plus one sets the style (selection − 1), regenerates the
/// hair and refreshes the icon view. Returns whether it changed.
pub fn hair_style_changed(style: &mut u32, selection: i32) -> bool {
    if selection < 1 || i64::from(selection) - 1 == i64::from(*style) {
        return false;
    }
    *style = (selection - 1) as u32;
    true
}

/// The edit form's close: detaches the child editor's callback when there
/// is one, then runs the base close.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditClose {
    pub detach: bool,
    pub base: bool,
}

pub fn close_edit_form(has_child: bool) -> EditClose {
    EditClose {
        detach: has_child,
        base: true,
    }
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
    // The original zero-extends the stamp and adds two in 64 bits, so it
    // does not wrap; the clock is zero-extended too.
    if u64::from(last as u32) + 2 > u64::from(clock as u32) {
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
///
/// The distance is tested with an unordered compare: a shorter distance
/// than `epsilon` writes nothing, while a NaN distance takes the write
/// branch, as the original's `jp` does.
pub fn settle_appearance(source: f32, target: f32, last: i32, clock: i32) -> CreationSettle {
    let value = (0.5 - source) * 2.0;
    match (value - target).abs().partial_cmp(&SETTLE_EPSILON) {
        Some(std::cmp::Ordering::Less) => unchanged(last),
        // A NaN distance jumps to the write, as the unordered compare does.
        _ => debounce(value, last, clock),
    }
}

/// Copies an appearance value to its target.
pub fn copy_appearance(source: f32, target: f32, last: i32, clock: i32) -> CreationSettle {
    match (source - target).abs().partial_cmp(&SETTLE_EPSILON) {
        Some(std::cmp::Ordering::Less) => unchanged(last),
        // A NaN distance jumps to the write, as the unordered compare does.
        _ => debounce(source, last, clock),
    }
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
    let word = change_sex(input.word, if input.female { 2 } else { 1 }).word;
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
///
/// The caption is the variant one below the stored one, so variant 1 reads
/// "A"; variant 0 has no caption and shows a dash.
pub fn update_creation_look(female: bool, variant: i32) -> CreationLook {
    if female {
        return CreationLook {
            values: [16; 4],
            letter: None,
        };
    }
    let letter = if variant < 1 {
        None
    } else {
        Some((variant + 0x40) as u8)
    };
    CreationLook {
        values: [0; 4],
        letter,
    }
}

/// Scales three normalised values to bytes (the hair page's colour
/// change).
///
/// Each single-precision product converts with the processor's
/// `CVTSS2SI` into a 64-bit register (ties to even) and the low byte is
/// stored: a NaN or a value past 2⁶³ is the integer indefinite, whose low
/// byte is zero, and a large in-range value keeps its own low byte.
pub fn appearance_bytes(values: [f32; 3]) -> [u8; 3] {
    values.map(|value| cvtss2si64(value * 255.0) as u8)
}

/// Scales two normalised values to 16-bit words (the body page's skin
/// change): each value plus 0.0, times 65535, in single precision, with
/// the same 64-bit conversion and the low word kept.
pub fn appearance_words(values: [f32; 2]) -> [u16; 2] {
    values.map(|value| cvtss2si64((value + 0.0) * 65535.0) as u16)
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

/// The thaumaturge's starting powers, which the unknown story background
/// gives its record.
///
/// Two known powers of 16 bytes each (`0x101` and `0x201`, both with
/// `0x5fff`); the first power slot holds the first power with bit 20 set
/// (`0x100101`), the second the second power; the power word is
/// `0x510000` and the float beside the slots 4.0. The other eight slots are
/// cleared. A record with known powers opens the power tree when it is
/// committed ([`CommitKind::PowerTree`]).
pub fn creation_defaults() -> CreationDefaults {
    CreationDefaults {
        count: 2,
        stride: 16,
        slots: [[0x101, 0x5fff, 0, 0], [0x201, 0x5fff, 0, 0]],
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

/// Picks the continue toggle's label and flags from the chosen kind.
///
/// The button shows the chosen kind and is enabled while the kind is below
/// the availability mask (an unsigned compare). With both a checkpoint and
/// a save (mask 3) either kind can switch; with only one there is nothing
/// to switch to and the button is disabled.
pub fn creation_continue_button(kind: i32, availability: i32) -> CreationContinue {
    let save = kind == 2;
    CreationContinue {
        label: exa_ui::creation::continue_label(kind),
        mirrored: save,
        enabled: (kind as u32) < (availability as u32),
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
///
/// This is the voice preview button: the "label" built when absent is the
/// selected voice's sample, and the draw plays it (see [`throttle_voice`]
/// for the same 300-unit guard).
pub fn refresh_creation_label(clock: i32, last: i32, has_widget: bool) -> CreationRefresh {
    if !throttle_voice(clock, last).played {
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

/// What resetting a record's gear did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationGearReset {
    /// The hands kept their flags and lost their items.
    pub hands: bool,
    /// The braies were put back on.
    pub braies: bool,
    pub carried_cleared: bool,
}

/// Resets a record's gear: every hand item is cleared, the worn list
/// becomes just the braies (item 8, no flags) and the carried list is
/// emptied. The hand flags stay, and so does each slot's second word.
pub fn reset_creation_gear(gear: &mut CreationGear) -> CreationGearReset {
    for hand in &mut gear.hands {
        hand.item = 0;
    }
    gear.worn.clear();
    gear.worn.push(GearSlot::new(BRAIES));
    gear.carried.clear();
    CreationGearReset {
        hands: true,
        braies: true,
        carried_cleared: true,
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
    // Both words are zero-extended and added in 64 bits, so the stamp does
    // not wrap.
    if !pending || (stamp as u32 as u64) + 1 >= u64::from(clock as u32) {
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
    // The stamp is zero-extended and added in 64 bits, so it does not wrap.
    if u64::from(stamp as u32) + 2 <= u64::from(clock as u32) {
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

/// What applying the creation scale did.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreationApply {
    pub scale: f32,
    pub stored: bool,
    pub rebuild: bool,
    pub global_call: bool,
    pub built: bool,
    pub drew: bool,
    pub finalized: bool,
}

const CREATION_SCALE_A: f64 = f64::from_bits(0x3fc5_c28f_5c28_f5c3);
const CREATION_SCALE_B: f64 = f64::from_bits(0x3fec_cccc_cccc_cccd);

/// Stores the appearance scale and, when special, rebuilds and draws the label.
pub fn apply_creation_scale(
    source: f32,
    special: bool,
    global_flag: bool,
    has_widget: bool,
) -> CreationApply {
    let scale = (f64::from(source) * CREATION_SCALE_A + CREATION_SCALE_B) as f32;
    CreationApply {
        scale,
        stored: true,
        rebuild: special,
        global_call: special && !global_flag,
        built: special && !has_widget,
        drew: special,
        finalized: true,
    }
}

/// A creation list step's result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationListStep {
    pub rebuilt: bool,
    pub index: i32,
}

/// Steps the option list back or forward.
pub fn step_creation_list(index: i32, up: bool) -> CreationListStep {
    if !up && index <= 0 {
        return CreationListStep {
            rebuilt: false,
            index,
        };
    }
    CreationListStep {
        rebuilt: true,
        index: if up { index + 1 } else { index - 1 },
    }
}

/// The creation mode switch's branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreationModeAction {
    Special,
    Widget { enabled: bool },
}

/// Routes the mode to the special branch or the widget bit.
pub fn switch_creation_mode(mode: i32) -> CreationModeAction {
    if mode == 2 {
        CreationModeAction::Special
    } else {
        CreationModeAction::Widget { enabled: mode <= 0 }
    }
}

/// The creation mode toggle's result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationModeToggle {
    pub changed: bool,
    pub mode: i32,
    pub refreshed: bool,
}

/// Toggles between mode 1 and mode 2 by flag bits.
pub fn toggle_creation_mode(mode: i32, flags: i32) -> CreationModeToggle {
    if mode == 2 {
        if flags & 1 == 0 {
            return CreationModeToggle {
                changed: false,
                mode,
                refreshed: false,
            };
        }
        CreationModeToggle {
            changed: true,
            mode: 1,
            refreshed: true,
        }
    } else {
        if flags & 2 == 0 {
            return CreationModeToggle {
                changed: false,
                mode,
                refreshed: false,
            };
        }
        CreationModeToggle {
            changed: true,
            mode: 2,
            refreshed: true,
        }
    }
}

/// The form a creation close schedules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreationCloseForm {
    Special,
    Regular,
}

/// The creation close's inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationCloseInput {
    pub active: bool,
    /// The weapons panel is built.
    pub has_weapon_panel: bool,
    /// The skills panel is built.
    pub has_skill_panel: bool,
    /// The versus lobby form is built.
    pub has_lobby_form: bool,
    /// The versus equipment page's skills panel is built.
    pub has_versus_skills: bool,
    /// The equipment form on show (the apparel panel, or the form the
    /// versus equipment hook built) is set.
    pub has_equipment_form: bool,
    /// The body page (physique, height and skin tone) is built.
    pub has_body_form: bool,
    /// The hair page is built.
    pub has_hair_form: bool,
    pub mode: i32,
}

/// What the creation close did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationClose {
    pub scheduled: Option<CreationCloseForm>,
    pub released: [bool; 7],
    /// The form is set back to its 45-unit width (closing the skills panel,
    /// which widened it).
    pub width_restored: bool,
    pub widgets_reset: bool,
    pub ticked: bool,
    pub strings: bool,
    pub disabled: bool,
    pub cleared: bool,
}

/// Closes the creation screen, scheduling a form or releasing its children.
pub fn close_creation_screen(input: CreationCloseInput) -> CreationClose {
    if !input.active {
        return CreationClose {
            scheduled: Some(if input.mode == 10 {
                CreationCloseForm::Special
            } else {
                CreationCloseForm::Regular
            }),
            released: [false; 7],
            width_restored: false,
            widgets_reset: false,
            ticked: false,
            strings: false,
            disabled: false,
            cleared: true,
        };
    }
    let mut released = [false; 7];
    let mut width_restored = false;
    if input.has_weapon_panel {
        released[0] = true;
    } else if input.has_skill_panel {
        released[1] = true;
        width_restored = true;
    } else if input.has_lobby_form {
        released[2] = true;
    }
    released[3] = input.has_versus_skills;
    released[4] = input.has_equipment_form;
    let widgets_reset = input.mode != 12;
    if widgets_reset {
        released[5] = input.has_body_form;
        released[6] = input.has_hair_form;
    }
    CreationClose {
        scheduled: None,
        released,
        width_restored,
        widgets_reset,
        ticked: true,
        strings: true,
        disabled: true,
        cleared: true,
    }
}

/// What hitting a creation option did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationOptionHit {
    pub hit: bool,
    pub selected: bool,
    pub result: bool,
}

/// Hit-tests the option list and selects its last entry on a hit.
pub fn hit_creation_option(key: i32, has_list: bool, hits: bool) -> CreationOptionHit {
    let hit = key != 8 && has_list && hits;
    CreationOptionHit {
        hit,
        selected: hit,
        result: true,
    }
}

/// What ensuring the creation singleton did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationSingleton {
    pub created: bool,
    pub ready: bool,
}

/// Creates the singleton when absent.
pub fn ensure_creation_singleton(exists: bool) -> CreationSingleton {
    CreationSingleton {
        created: !exists,
        ready: true,
    }
}

/// The creation key's result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationKey {
    pub result: i32,
    pub forwarded: bool,
    pub closed: bool,
}

/// Routes the key to the base handler or closes with `0x40`.
pub fn creation_key(has_selection: bool, base: i32) -> CreationKey {
    if has_selection {
        CreationKey {
            result: base,
            forwarded: true,
            closed: false,
        }
    } else {
        CreationKey {
            result: 0x40,
            forwarded: false,
            closed: true,
        }
    }
}

/// What showing the creation form did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationShow {
    pub prepared: bool,
    pub dispatched: bool,
    pub refreshed: bool,
    pub built: bool,
}

/// Shows the form through its prepare, dispatch and refresh.
pub fn show_creation_form(build: bool) -> CreationShow {
    CreationShow {
        prepared: true,
        dispatched: true,
        refreshed: true,
        built: build,
    }
}

/// A captured creation state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationCapture {
    pub words: [u64; 6],
    pub value: i32,
    pub mode: i32,
    pub scheduled: bool,
    pub stored_cleared: bool,
}

/// Captures the six words, the value and the mode.
pub fn capture_creation(
    words: [u64; 6],
    current: i32,
    last: i32,
    saved: i32,
    mode: i32,
) -> CreationCapture {
    CreationCapture {
        words,
        value: current - last + saved,
        mode,
        scheduled: true,
        stored_cleared: true,
    }
}

/// Schedules the special form in mode 10, else the regular form.
pub fn close_creation_form(mode: i32) -> CreationCloseForm {
    if mode == 10 {
        CreationCloseForm::Special
    } else {
        CreationCloseForm::Regular
    }
}

/// A creation page or select result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationPage {
    pub mode: i32,
    pub index: i32,
    pub refreshed: bool,
}

/// Pages forward when the maximum exceeds the index.
pub fn page_creation_next(max: i32, index: i32) -> CreationPage {
    if max <= index {
        CreationPage {
            mode: 0,
            index,
            refreshed: false,
        }
    } else {
        CreationPage {
            mode: 1,
            index: index + 1,
            refreshed: true,
        }
    }
}

/// Selects the wanted index less one.
pub fn select_creation_index(wanted: i32, current: i32) -> CreationPage {
    let index = wanted - 1;
    if index == current {
        CreationPage {
            mode: 0,
            index: current,
            refreshed: false,
        }
    } else {
        CreationPage {
            mode: 1,
            index,
            refreshed: true,
        }
    }
}

/// A creation table selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationTable {
    pub index: i32,
    pub applied: bool,
}

/// Selects the source index less one in the table.
pub fn select_creation_table(index: i32) -> CreationTable {
    CreationTable {
        index: index - 1,
        applied: true,
    }
}

/// A started creation session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationSessionStart {
    pub value: i32,
    pub scheduled: bool,
    pub cleared: bool,
}

/// Starts the creation session with its value.
pub fn start_creation_session(value: i32) -> CreationSessionStart {
    CreationSessionStart {
        value,
        scheduled: true,
        cleared: true,
    }
}

/// The creation menu's chosen call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreationDispatch {
    Alternate,
    Primary,
}

/// Takes the alternate call when the byte is set, else the primary call.
pub fn dispatch_creation_menu(alternate: bool) -> CreationDispatch {
    if alternate {
        CreationDispatch::Alternate
    } else {
        CreationDispatch::Primary
    }
}

/// What starting the creation screen did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationStartup {
    pub reset: bool,
    pub second_reset: bool,
    pub created: bool,
    pub prepared: bool,
    pub mode: i32,
    pub refreshed: bool,
    pub built: bool,
    pub ticked: bool,
    pub stored: bool,
}

/// Starts the creation screen and stores its form.
pub fn start_creation_screen(
    had_form: bool,
    second_reset: bool,
    mode: i32,
    arena_flag: bool,
    build: bool,
) -> CreationStartup {
    let in_mode = mode == 12;
    CreationStartup {
        reset: true,
        second_reset,
        created: !had_form,
        prepared: true,
        mode: if in_mode {
            if arena_flag { 2 } else { 1 }
        } else {
            0
        },
        refreshed: in_mode,
        built: in_mode && build,
        ticked: in_mode,
        stored: true,
    }
}

/// What the creation apply step did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationApplyStep {
    pub linked: bool,
    pub camera: bool,
    pub stored: bool,
    pub part_found: bool,
    pub first: bool,
    pub second: bool,
}

/// Relinks, refreshes the camera, stores the part result and ticks.
pub fn apply_creation_step(has_target: bool, part_found: bool) -> CreationApplyStep {
    CreationApplyStep {
        linked: true,
        camera: true,
        stored: has_target,
        part_found: has_target && part_found,
        first: true,
        second: true,
    }
}

/// What the creation part refresh did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationPartRefresh {
    pub stored: bool,
    pub part_found: bool,
}

/// Stores the part result at the target when it exists.
pub fn refresh_creation_part(has_target: bool, part_found: bool) -> CreationPartRefresh {
    CreationPartRefresh {
        stored: has_target,
        part_found: has_target && part_found,
    }
}

/// What destroying the creation did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationDestroy {
    pub base_destroy: bool,
    pub ref_decremented: bool,
    pub released: bool,
    pub base: bool,
    pub freed: bool,
}

/// Destroys the creation, decrementing or releasing its child.
pub fn destroy_creation(flags: i32, has_child: bool, child_count: i32) -> CreationDestroy {
    CreationDestroy {
        base_destroy: flags > 0,
        ref_decremented: has_child && child_count > 1,
        released: has_child && child_count <= 1,
        base: true,
        freed: flags > 0,
    }
}

/// Copies the creation point.
pub fn copy_creation_point(point: [i32; 3]) -> [i32; 3] {
    point
}

/// What resuming the creation did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationResume {
    pub saved: bool,
    pub refreshed: bool,
    pub resumed: bool,
}

/// Saves, refreshes at the index and resumes.
pub fn resume_creation() -> CreationResume {
    CreationResume {
        saved: true,
        refreshed: true,
        resumed: true,
    }
}

/// Writes the three starting skill slots into `skills`.
///
/// Close Combat, Armour and Shields are set to five levels (25000 points)
/// and their first five techniques. Only those three slots' points and
/// first five technique bytes are written: Ranged Combat, Insight,
/// Concentration and each slot's sixth technique keep what they held. The
/// free creator does this when it opens and every time it builds the
/// equipment page of a mode without its own kit rules.
pub fn apply_starting_skills(skills: &mut [Skill; SKILLS]) {
    for (slot, skill) in [(CLOSE_COMBAT, 0), (ARMOUR, 1), (SHIELDS, 2)] {
        let bytes = creation_slot_defaults()[skill];
        let target = &mut skills[slot];
        target.points = u16::from_le_bytes([bytes[0], bytes[1]]);
        target.techniques[..5].copy_from_slice(&bytes[2..7]);
    }
}

/// The skills a new, empty record has once the starting slots are written
/// (see [`apply_starting_skills`]); the other three skills are empty.
pub fn starting_skills() -> [Skill; SKILLS] {
    let mut skills = [Skill::default(); SKILLS];
    apply_starting_skills(&mut skills);
    skills
}

/// The byte after a skill's points at `level`, read across the six slots
/// as the original's unchecked index reads it.
///
/// A level past the slot's six techniques reads the next slot's points and
/// techniques. Past the last slot the original reads the record's carried
/// count and alignment; that read cannot change the result (a level above
/// five clears nothing and no slot follows the last), so it is taken as
/// zero here.
fn technique_byte(skills: &[Skill; SKILLS], slot: usize, level: usize) -> u8 {
    let byte = slot * 8 + 2 + level;
    if byte >= SKILLS * 8 {
        return 0;
    }
    let skill = &skills[byte / 8];
    let offset = byte % 8;
    if offset < 2 {
        // The original reads a neighbouring slot's point bytes here.
        skill.points.to_le_bytes()[offset]
    } else {
        skill.techniques[offset - 2]
    }
}

/// Clears the first used skill and every skill after it.
///
/// The first used slot keeps its first technique byte, as the original's
/// five one-byte writes leave it; the later slots are cleared whole.
pub fn clear_skills_from_first_used(skills: &mut [Skill; SKILLS]) -> Option<usize> {
    let first = skills.iter().position(|skill| skill.points != 0)?;
    let skill = &mut skills[first];
    skill.points = 0;
    if skill.techniques.len() > 1 {
        skill.techniques[1..].fill(0);
    }
    for later in &mut skills[first + 1..] {
        *later = Skill::default();
    }
    Some(first)
}

/// What spending the account's experience on the skills did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillBudget {
    /// The skills after the budget was applied.
    pub skills: [Skill; SKILLS],
    /// The experience left after the skills were capped, before the
    /// training skill took its share.
    pub leftover: u32,
    /// The training skill that took the leftover, if any.
    pub training: Option<usize>,
}

/// Caps the six skills to a global experience budget.
///
/// A slot over the remaining budget is cut to it; when its technique at
/// the new level is chosen, the first such slot keeps that technique and
/// clears only the later ones. Whatever is left goes to the skill in
/// training (the first below level 5 with its next technique chosen) up
/// to its next level. A knight or proctor's budget is at least 10000.
pub fn set_skill_budget(skills: &[Skill; SKILLS], global: u32, background: i32) -> SkillBudget {
    let mut budget = if matches!(background, 1 | 2) {
        global.max(10_000)
    } else {
        global
    };
    let mut out = *skills;
    let mut keep_technique = true;
    for index in 0..SKILLS {
        let mut skill = out[index];
        let points = u32::from(skill.points);
        if budget < points {
            skill.points = budget as u16;
            let points = u32::from(skill.points);
            let mut level = points / u32::from(LEVEL_STEP);
            if keep_technique && technique_byte(&out, index, level as usize) != 0 {
                level += 1;
                keep_technique = false;
            }
            if level <= TECHNIQUE_SLOTS as u32 {
                let from = (level as usize).min(TECHNIQUE_SLOTS);
                skill.techniques[from..].fill(0);
            }
            budget = budget.wrapping_sub(points);
        } else {
            budget = budget.wrapping_sub(points);
        }
        out[index] = skill;
    }
    let leftover = budget;
    // The character unit's training-skill routine returns the same skill
    // and next level as the experience unit's rule (one-based there).
    let training = if leftover == 0 {
        None
    } else {
        training_skill(&out).map(|(index, next)| {
            let cap = next.saturating_mul(u32::from(LEVEL_STEP));
            let slot = &mut out[index];
            let raised = u32::from(slot.points).wrapping_add(leftover) as u16;
            slot.points = if cap < u32::from(raised) {
                cap as u16
            } else {
                raised
            };
            index
        })
    };
    SkillBudget {
        skills: out,
        leftover,
        training,
    }
}

/// What the skills are worth as global experience.
///
/// Each 5000 points is worth more than the last (the first at 4, then 5,
/// 6, …), the first skill's partial level is worth its remainder at the
/// current weight, and the total is quartered toward zero. The original
/// visits the slots in order, so only the first partial level counts, and
/// sums in 32 bits (six 16-bit skills cannot reach the wrap).
pub fn effective_global_experience(skills: &[Skill; SKILLS]) -> i32 {
    let step = i32::from(LEVEL_STEP);
    let mut total = 0i32;
    let mut weight = 4i32;
    for skill in skills {
        for _ in 0..i32::from(skill.points) / step {
            total = total.wrapping_add(weight.wrapping_mul(step));
            weight += 1;
        }
    }
    for skill in skills {
        let points = i32::from(skill.points);
        let rem = points - points / step * step;
        if rem != 0 {
            total = total.wrapping_add(rem.wrapping_mul(weight));
            break;
        }
    }
    total / 4
}

/// The new global experience a continued character's skills page shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewGlobal {
    /// The account's worth less the committed character's worth, or zero;
    /// the form keeps it for the arena commit.
    pub worth_left: u32,
    /// That worth converted back to experience, as shown.
    pub experience: i32,
}

/// The new global experience: the account's experience converted to worth
/// ([`global_xp_worth`]), less the worth of the last committed character
/// (zero when that covers it), converted back ([`global_xp_for_worth`]).
pub fn new_global_experience(account: u32, committed_worth: u32) -> NewGlobal {
    let worth_left = global_xp_worth(account).saturating_sub(committed_worth);
    NewGlobal {
        worth_left,
        experience: global_xp_for_worth(worth_left as i32),
    }
}

/// Which skills page the creator shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillPageMode {
    /// The arena's skills, read-only.
    Arena,
    /// The account's experience, spent on the skills.
    Global,
    /// A continued character's new experience, read-only.
    NewGlobal,
}

/// The skills page's captions and state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillPage {
    pub mode: SkillPageMode,
    /// The equipment page's second tab caption; a continued character
    /// keeps the one it had.
    pub tab: Option<&'static str>,
    /// The skills panel's heading; the arena's new character keeps the
    /// panel's own.
    pub heading: Option<&'static str>,
    /// The experience the panel shows (none on the arena's page).
    pub value: Option<u32>,
    /// What the form keeps for the arena commit (see [`NewGlobal`]).
    pub worth_left: u32,
    /// The arena's new character clears its skills from the first used.
    pub clear_skills: bool,
    /// The skills are capped to the account's experience first
    /// ([`set_skill_budget`]).
    pub apply_budget: bool,
    /// The player may spend experience on the skills.
    pub editable: bool,
    /// The page selector's value (4 for the arena, else 6).
    pub page: i32,
    /// The second preview panel was hidden first (it exists).
    pub second_panel_hidden: bool,
    /// The skills panel was made (none existed) rather than shown again.
    pub created: bool,
    /// The panel is shown, made or not.
    pub shown: bool,
    /// The panel's flag word gained `0x20000` when it was made.
    pub panel_flags: u32,
    /// The existing panel's record pointer was rebound to the edited record.
    pub record_rebound: bool,
}

/// Picks the skills page for the game and screen modes. `account` is the
/// account's global experience and `committed_worth` the worth of the last
/// committed character; `second_panel` says whether the second preview
/// panel exists and `existing` whether the skills panel was already made.
///
/// The second preview panel is hidden first when it exists. The skills
/// panel is made at [`skill_panel_place`] with its flag word gaining
/// `0x20000` and shown; an existing one is shown again with its record
/// pointer rebound to the edited record. The headed page then shows the
/// account's experience (or, on the arena's page, the character's skills
/// and their progress).
pub fn skill_page(
    game_mode: i32,
    screen_mode: i32,
    account: u32,
    committed_worth: u32,
    second_panel: bool,
    existing: bool,
) -> SkillPage {
    let continuing = screen_mode == 12;
    let created = !existing;
    let panel_flags = if created { 0x2_0000 } else { 0 };
    let new_global = new_global_experience(account, committed_worth);
    let tab = (!continuing).then_some(exa_ui::creation::SKILLS);
    if game_mode == 8 {
        SkillPage {
            mode: SkillPageMode::Arena,
            tab,
            heading: continuing.then_some(exa_ui::creation::CURRENT_SKILL_PROGRESS),
            value: None,
            worth_left: new_global.worth_left,
            clear_skills: !continuing,
            apply_budget: false,
            editable: false,
            page: 4,
            second_panel_hidden: second_panel,
            created,
            shown: true,
            panel_flags,
            record_rebound: existing,
        }
    } else if continuing {
        SkillPage {
            mode: SkillPageMode::NewGlobal,
            tab,
            heading: Some(exa_ui::creation::NEW_GLOBAL_EXPERIENCE),
            value: Some(new_global.experience as u32),
            worth_left: new_global.worth_left,
            clear_skills: false,
            apply_budget: false,
            editable: false,
            page: 6,
            second_panel_hidden: second_panel,
            created,
            shown: true,
            panel_flags,
            record_rebound: existing,
        }
    } else {
        SkillPage {
            mode: SkillPageMode::Global,
            tab,
            heading: Some(exa_ui::creation::GLOBAL_EXPERIENCE),
            value: Some(account),
            worth_left: new_global.worth_left,
            clear_skills: false,
            apply_budget: true,
            editable: true,
            page: 6,
            second_panel_hidden: second_panel,
            created,
            shown: true,
            panel_flags,
            record_rebound: existing,
        }
    }
}

/// A story background whose saved character seeds the creator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundClass {
    /// The unknown background: no template name.
    Unknown,
    Knight,
    Proctor,
    Villager,
}

/// The settings keys a background's preset is kept under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresetKeys {
    pub root: u32,
    pub character: u32,
}

/// A background preset's name and keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackgroundPreset {
    pub class: BackgroundClass,
    pub name: Option<&'static str>,
    pub keys: PresetKeys,
}

/// The first preset key (the unknown background); the arena's career
/// character is kept one below it and each named background one above.
const PRESET_KEY_BASE: u32 = 0x6431_bc02;
/// The character key under every preset: the same for the arena and all
/// four backgrounds.
const PRESET_CHARACTER_KEY: u32 = 0x6431_cc01;

/// The preset of a story background, or `None` past the three templates.
pub fn background_preset(background: i32) -> Option<BackgroundPreset> {
    let class = match background {
        0 => BackgroundClass::Unknown,
        1 => BackgroundClass::Knight,
        2 => BackgroundClass::Proctor,
        3 => BackgroundClass::Villager,
        _ => return None,
    };
    let name = match class {
        BackgroundClass::Unknown => None,
        BackgroundClass::Knight => Some("knight"),
        BackgroundClass::Proctor => Some("proctor"),
        BackgroundClass::Villager => Some("villager"),
    };
    let offset = background as u32;
    Some(BackgroundPreset {
        class,
        name,
        keys: PresetKeys {
            root: PRESET_KEY_BASE + offset,
            character: PRESET_CHARACTER_KEY,
        },
    })
}

/// The settings keys a committed character saves its preset under.
///
/// Story mode keeps one preset per background; the arena keeps its career
/// character. Other modes save nothing.
pub fn preset_save_keys(game_mode: i32, background: i32) -> Option<PresetKeys> {
    match game_mode {
        4 => background_preset(background).map(|preset| preset.keys),
        8 => Some(PresetKeys {
            root: PRESET_KEY_BASE - 1,
            character: PRESET_CHARACTER_KEY,
        }),
        _ => None,
    }
}

/// Where a preset's record came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresetSource {
    /// The settings held a preset.
    Saved,
    /// No preset was found: the creator made an empty record.
    NewDefault,
    /// No preset was found and the background's template is used instead.
    Template,
}

/// What happens to the record's gear when a background loads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundGear {
    /// The unknown background clears the hands and lists.
    Reset,
    /// A named background always wears its template's gear: moved into the
    /// loaded preset, or with the whole template when no preset was saved.
    Template,
}

/// What loading a story background does to the record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackgroundPlan {
    pub preset: BackgroundPreset,
    pub source: PresetSource,
    pub gear: BackgroundGear,
    /// The unknown background starts the thaumaturge's powers (see
    /// [`creation_defaults`]).
    pub creation_defaults: bool,
}

/// Plans the record for a story background from whether its preset loaded.
///
/// The unknown background takes its saved preset or a new default record,
/// clears its gear and starts its powers. A named background reads its
/// template record by name first; a saved preset then keeps its own body
/// and skills but takes the template's gear, and without a preset the
/// template becomes the record. The template itself is supplied by the
/// caller.
pub fn plan_background(background: i32, preset_loaded: bool) -> Option<BackgroundPlan> {
    let preset = background_preset(background)?;
    let source = match (background, preset_loaded) {
        (_, true) => PresetSource::Saved,
        (0, false) => PresetSource::NewDefault,
        (_, false) => PresetSource::Template,
    };
    let unknown = background == 0;
    Some(BackgroundPlan {
        preset,
        source,
        gear: if unknown {
            BackgroundGear::Reset
        } else {
            BackgroundGear::Template
        },
        creation_defaults: unknown,
    })
}

/// Where a freely created record starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreeCreateSource {
    /// Load the given background preset and share its gear.
    Preset(PresetKeys),
    /// Make an empty record.
    NewDefault,
    /// Keep the record the creator already had.
    KeepRecord,
}

/// What opening the free creator does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreeCreatePlan {
    pub source: FreeCreateSource,
    /// The three starting skill slots are written (see
    /// [`apply_starting_skills`]), into the kept record or the new one.
    pub reset_skills: bool,
    /// A loaded preset's gear is cleared so the player picks a kit.
    pub reset_gear: bool,
    /// A loaded preset loses its race byte and its known powers.
    pub clear_race_and_powers: bool,
    /// No record was found and a new one is made and marked.
    pub marks_new_character: bool,
}

/// Plans the free creator's record for the game mode.
///
/// Story and arena read their preset (the unknown background's key for
/// story, the career's for arena). A loaded preset has its gear reset and
/// loses its race byte and powers; a missing one leaves no record, so a
/// new, marked one is made with no starting kit or skills. Every other
/// mode keeps the record it had (or makes a new, marked one) and always
/// writes the three starting skill slots.
pub fn plan_free_create(
    game_mode: i32,
    record_present: bool,
    preset_loaded: bool,
) -> FreeCreatePlan {
    let keys = match game_mode {
        4 => Some(PresetKeys {
            root: PRESET_KEY_BASE,
            character: PRESET_CHARACTER_KEY,
        }),
        8 => Some(PresetKeys {
            root: PRESET_KEY_BASE - 1,
            character: PRESET_CHARACTER_KEY,
        }),
        _ => None,
    };
    if let Some(keys) = keys {
        return if preset_loaded {
            FreeCreatePlan {
                source: FreeCreateSource::Preset(keys),
                reset_skills: false,
                reset_gear: true,
                clear_race_and_powers: true,
                // The preset is read into a record made (and marked) for
                // it when the creator had none.
                marks_new_character: !record_present,
            }
        } else {
            FreeCreatePlan {
                source: FreeCreateSource::NewDefault,
                reset_skills: false,
                reset_gear: false,
                clear_race_and_powers: false,
                marks_new_character: true,
            }
        };
    }
    FreeCreatePlan {
        source: if record_present {
            FreeCreateSource::KeepRecord
        } else {
            FreeCreateSource::NewDefault
        },
        reset_skills: true,
        reset_gear: false,
        clear_race_and_powers: false,
        marks_new_character: !record_present,
    }
}

/// The screen mode the creator was opened in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreationScreenMode {
    Free,
    Story,
    Recruit,
    Continue,
}

impl CreationScreenMode {
    /// Reads the screen mode; an unknown value is the free creator.
    pub fn from_code(code: i32) -> Self {
        match code {
            4 => Self::Story,
            10 => Self::Recruit,
            12 => Self::Continue,
            _ => Self::Free,
        }
    }
}

/// Why the creator could not open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreationEntryError {
    /// Continue mode needs the save manager.
    SaveManagerMissing,
}

impl CreationEntryError {
    /// The message the original shows for this error.
    pub fn message(self) -> &'static str {
        match self {
            Self::SaveManagerMissing => exa_ui::creation::SAVE_MANAGER_MISSING,
        }
    }

    /// The message box's title.
    pub fn title(self) -> &'static str {
        match self {
            Self::SaveManagerMissing => exa_ui::creation::CHARACTER_SCREEN,
        }
    }

    /// The message box's code.
    pub fn code(self) -> i32 {
        match self {
            Self::SaveManagerMissing => 4,
        }
    }
}

/// Where the creator's record comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntrySource {
    /// A new default record (recruiting, or nothing else was found).
    NewDefault,
    /// The chosen story background's preset (see [`plan_background`]).
    Background(i32),
    /// Continuing: the record the screen already had, or a new default one.
    ContinueSave,
    /// The free creator's preset or default (see [`plan_free_create`]).
    FreeCreate,
    /// The record the screen already had.
    Existing,
}

/// The record source and filtering an entry performs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntryPlan {
    /// The screen mode after a failed continue falls back to the free one.
    pub mode: CreationScreenMode,
    /// The game mode (story 4, arena 8, versus 0x1010, …).
    pub game_mode: i32,
    pub source: EntrySource,
    /// The mode's gear filter ([`filter_creation_gear`]) runs.
    pub filter: bool,
    /// The four health and focus words are set full.
    pub full_health: bool,
    /// The story record's experience word is cleared.
    pub clear_experience: bool,
    /// The underwear pass runs: the record wears the braies unless its
    /// race band is set or a worn item covers the groin
    /// (`exa_character::record::CharacterData::ensure_underwear`).
    pub ensure_underwear: bool,
    /// The record is a new one the screen creates and marks as such (the
    /// record's new-character word set to 1 when it is made).
    pub marks_new_character: bool,
    pub background: i32,
}

/// Plans the record a character screen entry edits.
///
/// Continue mode needs the save manager and first lists the saves; an
/// invalid list falls back to the free creator. A valid one keeps the
/// screen's record (or a new default one) unfiltered and untouched.
/// Recruiting makes a new default record that is not filtered. Story mode
/// loads the chosen background's preset, clears its experience and
/// filters it. Other modes keep the record they had, or run the free
/// creator; when that finds no record (story or arena with no saved
/// preset, `preset_loaded` false) a new default record is made and not
/// filtered. Every record but a continued one is set to full health and
/// then gets the underwear pass.
pub fn plan_creation_entry(
    screen_mode: i32,
    game_mode: i32,
    background: i32,
    record_present: bool,
    save_manager_present: bool,
    saves_valid: bool,
    preset_loaded: bool,
) -> Result<EntryPlan, CreationEntryError> {
    let mut mode = CreationScreenMode::from_code(screen_mode);
    if mode == CreationScreenMode::Continue {
        if !save_manager_present {
            return Err(CreationEntryError::SaveManagerMissing);
        }
        if saves_valid {
            return Ok(EntryPlan {
                mode,
                game_mode,
                source: EntrySource::ContinueSave,
                filter: false,
                full_health: false,
                clear_experience: false,
                ensure_underwear: false,
                marks_new_character: !record_present,
                background,
            });
        }
        mode = CreationScreenMode::Free;
    }
    let (source, filter) = match mode {
        CreationScreenMode::Recruit => (EntrySource::NewDefault, false),
        CreationScreenMode::Story => (EntrySource::Background(background), true),
        _ if record_present => (EntrySource::Existing, true),
        // The free creator leaves no record only where it reads a preset.
        _ if matches!(game_mode, 4 | 8) && !preset_loaded => (EntrySource::NewDefault, false),
        _ => (EntrySource::FreeCreate, true),
    };
    Ok(EntryPlan {
        mode,
        game_mode,
        source,
        filter,
        full_health: true,
        clear_experience: mode == CreationScreenMode::Story,
        ensure_underwear: filter,
        marks_new_character: match mode {
            CreationScreenMode::Recruit => true,
            // The story loader makes (and marks) a record only when the
            // screen had none and it either reads a saved preset or starts
            // the unknown background; a named background without a preset
            // adopts its template, and an unlisted one gets an unmarked
            // blank record.
            CreationScreenMode::Story => {
                !record_present
                    && background_preset(background).is_some()
                    && (background == 0 || preset_loaded)
            }
            // The free creator (or the entry's own fallback) makes and
            // marks a record whenever none was there.
            _ => !record_present,
        },
        background,
    })
}

/// The two pages of the creator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreationScreenPage {
    /// The name, sex, face and hair.
    Appearance,
    /// The equipment and skills.
    Equipment,
}

/// What the final button does for a game mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitKind {
    Recruit,
    Begin,
    Ready,
    Create,
    /// The record knows thaumaturgy powers: the power tree opens, and its
    /// own commit saves the preset and starts the story.
    PowerTree,
}

/// The state the character screen's final button reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubmitInput {
    pub page: CreationScreenPage,
    pub name_len: usize,
    /// The record's known thaumaturgy powers: any opens the power tree
    /// instead of committing, and hides the final button's label.
    pub known_powers: i32,
    pub game_mode: i32,
    pub screen_mode: i32,
    pub continue_mode: bool,
    /// The versus script installed its equipment hook.
    pub equipment_hook: bool,
}

/// The message box the final button shows when the character has no name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NameWarning {
    pub title: &'static str,
    pub text: &'static str,
    /// The box's own code; the warning's is 0xc1.
    pub code: i32,
    /// The callback closes the warning and the creator with code 3.
    pub closes: bool,
}

/// The warning's box code.
pub const NAME_WARNING_CODE: i32 = 0xc1;

impl NameWarning {
    /// The warning the original shows for a missing name.
    pub const fn missing() -> Self {
        Self {
            title: exa_ui::creation::CHARACTER_NAME,
            text: exa_ui::creation::PLEASE_NAME,
            code: NAME_WARNING_CODE,
            closes: true,
        }
    }
}

/// What pressing the final button did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Submit {
    pub page: CreationScreenPage,
    /// The name is missing and the warning box is shown.
    pub missing_name: bool,
    /// The warning box the screen opens, when the name is missing.
    pub warning: Option<NameWarning>,
    /// The equipment page is built.
    pub build_equipment: bool,
    /// What the equipment page holds for the mode.
    pub panels: EquipmentPanels,
    pub button: Option<&'static str>,
    /// The screen flags that a versus lobby button needs.
    pub ready_flags: bool,
    /// The equipment page's tab state word gained `0x110`.
    pub tab_flags: bool,
    /// The equipment page was shown.
    pub form_shown: bool,
    /// The page state word takes 2: the advanced editor is scheduled and
    /// commits in place of the mode's own.
    pub advanced_editor: bool,
    pub commit: Option<CommitKind>,
}

/// The panels the equipment page builds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EquipmentPanels {
    pub apparel: bool,
    pub weapons: bool,
    pub skills: bool,
    /// The starting skill slots are written first
    /// ([`apply_starting_skills`]).
    pub starting_skills: bool,
    /// Versus with its script hooks: the script's equipment and lobby
    /// forms stand in for the panels it replaces.
    pub versus_script: bool,
}

/// The equipment page's panels for a game mode.
///
/// The arena and story show apparel and skills. Versus shows apparel, or
/// with the script's equipment hook its equipment form and a read-only
/// skills panel at 100000 experience. Every other mode rewrites the
/// starting skills and shows apparel and weapons.
pub fn equipment_panels(game_mode: i32, equipment_hook: bool) -> EquipmentPanels {
    match game_mode {
        4 | 8 => EquipmentPanels {
            apparel: true,
            skills: true,
            ..EquipmentPanels::default()
        },
        0x1010 => EquipmentPanels {
            apparel: !equipment_hook,
            skills: equipment_hook,
            versus_script: equipment_hook,
            ..EquipmentPanels::default()
        },
        _ => EquipmentPanels {
            apparel: true,
            weapons: true,
            starting_skills: true,
            ..EquipmentPanels::default()
        },
    }
}

/// Advances the creator's page or commits the character.
///
/// On the appearance page a non-continued character needs a name; a
/// missing one asks for it and changes nothing. A present one builds the
/// equipment page and, when the record knows no powers, labels the button
/// for the mode. On the equipment page a record with powers opens the
/// power tree; otherwise the mode decides what the commit does: a recruit
/// is handed to the arena's hook (or the form closes), versus readies its
/// lobby, and every other mode saves the preset, commits the record,
/// stores its worth and starts the game.
pub fn submit_creation(input: SubmitInput) -> Submit {
    if input.page == CreationScreenPage::Appearance {
        if !input.continue_mode && input.name_len < 1 {
            return Submit {
                page: input.page,
                missing_name: true,
                warning: Some(NameWarning::missing()),
                build_equipment: false,
                panels: EquipmentPanels::default(),
                button: None,
                ready_flags: false,
                tab_flags: false,
                form_shown: false,
                advanced_editor: false,
                commit: None,
            };
        }
        let button = if input.known_powers == 0 {
            Some(exa_ui::creation::commit_label(
                input.screen_mode,
                input.game_mode,
            ))
        } else {
            None
        };
        return Submit {
            page: CreationScreenPage::Equipment,
            missing_name: false,
            warning: None,
            build_equipment: true,
            panels: equipment_panels(input.game_mode, input.equipment_hook),
            button,
            ready_flags: input.known_powers == 0 && input.game_mode == 0x1010,
            tab_flags: true,
            form_shown: true,
            advanced_editor: false,
            commit: None,
        };
    }
    if input.known_powers > 0 {
        return Submit {
            page: input.page,
            missing_name: false,
            warning: None,
            build_equipment: false,
            panels: EquipmentPanels::default(),
            button: None,
            ready_flags: false,
            tab_flags: false,
            form_shown: false,
            advanced_editor: true,
            commit: Some(CommitKind::PowerTree),
        };
    }
    let commit = if input.screen_mode == 10 {
        CommitKind::Recruit
    } else if input.game_mode == 0x1010 {
        CommitKind::Ready
    } else if input.game_mode == 4 {
        CommitKind::Begin
    } else {
        CommitKind::Create
    };
    Submit {
        page: input.page,
        missing_name: false,
        warning: None,
        build_equipment: false,
        panels: EquipmentPanels::default(),
        button: None,
        ready_flags: false,
        tab_flags: false,
        form_shown: false,
        advanced_editor: false,
        commit: Some(commit),
    }
}

/// The labels the creator's header shows after a page switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageHeader {
    pub page: CreationScreenPage,
    pub first: &'static str,
    pub second: &'static str,
    /// The final button's label on the first page.
    pub final_button: &'static str,
}

/// Switches back to the appearance page.
pub fn appearance_page_header() -> PageHeader {
    let labels = exa_ui::creation::appearance_labels();
    PageHeader {
        page: CreationScreenPage::Appearance,
        first: labels.first,
        second: labels.second,
        final_button: labels.final_button,
    }
}

/// The header of the equipment page.
pub fn equipment_page_header(game_mode: i32, screen_mode: i32, versus_script: bool) -> PageHeader {
    let labels = exa_ui::creation::equipment_labels(game_mode, screen_mode, versus_script);
    PageHeader {
        page: CreationScreenPage::Equipment,
        first: labels.first,
        second: labels.second,
        final_button: labels.final_button,
    }
}

/// Where the page tab switch sends the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageDestination {
    /// Stay on the creator and show the appearance page.
    Appearance,
    /// Start the arena's recruit form.
    Arena,
    /// Leave the creator for the menu it came from, as the cancel key and
    /// an emptied save list do.
    Menu,
}

/// A panel the page switch hides on its way back to the appearance page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HiddenPanel {
    Weapons,
    Skills,
    /// The versus lobby form a script hook supplied.
    Lobby,
}

/// The equipment-page forms the creator has built. Each one stays built
/// once made, so a later visit sees every form an earlier one created.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BuiltPanels {
    pub weapons: bool,
    pub skills: bool,
    /// The versus lobby form (the lobby hook's).
    pub lobby: bool,
    /// The read-only versus skills panel (with the equipment hook).
    pub versus_skills: bool,
    /// The versus equipment form (the equipment hook's).
    pub versus_equipment: bool,
}

/// The result of the equipment page's tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageSwitch {
    pub page: CreationScreenPage,
    pub destination: PageDestination,
    pub header: Option<PageHeader>,
    /// The one main panel hidden, the first built of weapons, skills and
    /// the versus lobby form.
    pub hidden_first: Option<HiddenPanel>,
    /// The skills panel's slide argument, `0x2d`, when it was the panel
    /// hidden.
    pub skills_slide: Option<i32>,
    /// The versus skills panel was hidden (it exists).
    pub versus_skills_hidden: bool,
    /// The versus equipment form was hidden (it exists).
    pub versus_equipment_hidden: bool,
    /// A non-continued character's two appearance panels were shown again
    /// and both their tab state words set to `0x110`.
    pub appearance_panels_shown: bool,
    /// The appearance panels' two tab state words (both `0x110` when they
    /// were shown, both 0 for a continued character).
    pub appearance_tab_words: [u32; 2],
    /// The creator form was shown again with its page state cleared.
    pub form_shown: bool,
    /// The page state word was set back to 0 after the show.
    pub page_state_cleared: bool,
    /// The equipment tab's state word lost the `0x110` bits.
    pub tab_word_cleared: bool,
    /// The NEXT button's form was found and its disabled bit cleared.
    pub next_enabled: bool,
}

/// Switches from the equipment page back to the appearance page, or leaves
/// the creator from the appearance page: a recruit returns to the arena,
/// anyone else to the menu.
///
/// Only the page state 1 (the equipment page) comes back; any other state
/// leaves instead. On the way back only the first built of the weapons
/// panel, the skills panel (which also slides with `0x2d`) and the versus
/// lobby form is hidden; the versus skills panel and the versus equipment
/// form are then hidden when they exist. A non-continued character shows its
/// two appearance panels again and sets both tab state words to `0x110`.
/// The form is then shown, its page state word cleared, the tabs read NAME
/// and HAIR, the equipment tab's state word loses the `0x110` bits, the NEXT
/// button's form is captioned and enabled, and the appearance editor is
/// stored.
pub fn switch_creation_page(
    page: CreationScreenPage,
    screen_mode: i32,
    panels: BuiltPanels,
) -> PageSwitch {
    if page == CreationScreenPage::Equipment {
        let hidden_first = if panels.weapons {
            Some(HiddenPanel::Weapons)
        } else if panels.skills {
            Some(HiddenPanel::Skills)
        } else if panels.lobby {
            Some(HiddenPanel::Lobby)
        } else {
            None
        };
        let appearance_panels_shown = screen_mode != 12;
        PageSwitch {
            page: CreationScreenPage::Appearance,
            destination: PageDestination::Appearance,
            header: Some(appearance_page_header()),
            hidden_first,
            skills_slide: (hidden_first == Some(HiddenPanel::Skills)).then_some(0x2d),
            versus_skills_hidden: panels.versus_skills,
            versus_equipment_hidden: panels.versus_equipment,
            appearance_panels_shown,
            appearance_tab_words: if appearance_panels_shown {
                [0x110, 0x110]
            } else {
                [0, 0]
            },
            form_shown: true,
            page_state_cleared: true,
            tab_word_cleared: true,
            next_enabled: true,
        }
    } else {
        PageSwitch {
            page,
            destination: leave_creator(screen_mode),
            header: None,
            hidden_first: None,
            skills_slide: None,
            versus_skills_hidden: false,
            versus_equipment_hidden: false,
            appearance_panels_shown: false,
            appearance_tab_words: [0, 0],
            form_shown: false,
            page_state_cleared: false,
            tab_word_cleared: false,
            next_enabled: false,
        }
    }
}

/// Leaves the creator: a recruit returns to the arena's form, anyone else
/// to the menu. The cancel key and the page tab's appearance side both
/// end here.
pub fn leave_creator(screen_mode: i32) -> PageDestination {
    if screen_mode == 10 {
        PageDestination::Arena
    } else {
        PageDestination::Menu
    }
}

/// The account experience after an arena character is committed.
///
/// The worth left on the skills page, less `spent_before` (a word this
/// unit only reads), plus the worth of the last committed character, in
/// wrapping 32-bit arithmetic.
pub fn arena_global_experience(worth_left: i32, spent_before: i32, committed_worth: i32) -> i32 {
    worth_left
        .wrapping_sub(spent_before)
        .wrapping_add(committed_worth)
}

/// What the arena commit carries over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArenaCommit {
    pub skills: [Skill; SKILLS],
    pub global: i32,
    /// The form's continue kind, carried as it was.
    pub kind: i32,
    /// The arena's start is scheduled.
    pub transition: bool,
}

/// Prepares an arena character's commit: its six skills are copied, the
/// account experience is worked out and the arena's start runs.
pub fn commit_arena(
    skills: [Skill; SKILLS],
    worth_left: i32,
    spent_before: i32,
    committed_worth: i32,
    kind: i32,
) -> ArenaCommit {
    ArenaCommit {
        skills,
        global: arena_global_experience(worth_left, spent_before, committed_worth),
        kind,
        transition: true,
    }
}

/// A story commit's preset and worth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoryCommit {
    pub keys: Option<PresetKeys>,
    pub worth: i32,
    pub transition: bool,
}

/// Prepares a story character's commit: save its preset and start the mode.
pub fn commit_story(game_mode: i32, background: i32, skills: &[Skill; SKILLS]) -> StoryCommit {
    StoryCommit {
        keys: preset_save_keys(game_mode, background),
        worth: effective_global_experience(skills),
        transition: true,
    }
}

/// The two kinds of save the continue screen offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveKind {
    Checkpoint = 1,
    Save = 2,
}

impl SaveKind {
    /// Reads a choice; 2 is a save, anything else a checkpoint.
    pub fn from_code(code: i32) -> Self {
        if code == 2 {
            Self::Save
        } else {
            Self::Checkpoint
        }
    }

    /// The original's choice code.
    pub fn code(self) -> i32 {
        self as i32
    }
}

/// A save slot's availability mask: bit 0 a checkpoint, bit 1 a save.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SaveAvailability(pub u32);

impl SaveAvailability {
    /// Whether the kind may be selected.
    pub fn has(self, kind: SaveKind) -> bool {
        self.0 & (1 << (kind.code() - 1)) != 0
    }
}

/// A continue-screen selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SaveSelection {
    pub index: i32,
    pub kind: SaveKind,
    pub availability: SaveAvailability,
    /// The mask offered another kind than the chosen one.
    pub adjusted: bool,
    /// The screenshot/view refit runs through a deferred callback.
    pub deferred: bool,
}

/// Selects a save slot with its kind, adjusting the kind to what the slot
/// offers.
///
/// The save manager reports what the slot offers; a zero mask leaves the
/// kind and the screen alone. Otherwise the creator's record is shown
/// again, a save the slot lacks falls back to its checkpoint and a missing
/// checkpoint to its save, and the screen is refreshed.
pub fn select_save(index: i32, kind: SaveKind, availability: SaveAvailability) -> SaveSelection {
    let mut kind = kind;
    let mut adjusted = false;
    if availability.0 == 0 {
        return SaveSelection {
            index,
            kind,
            availability,
            adjusted,
            deferred: false,
        };
    }
    if kind == SaveKind::Save && !availability.has(SaveKind::Save) {
        kind = SaveKind::Checkpoint;
        adjusted = true;
    } else if kind != SaveKind::Save && !availability.has(SaveKind::Checkpoint) {
        kind = SaveKind::Save;
        adjusted = true;
    }
    SaveSelection {
        index,
        kind,
        availability,
        adjusted,
        deferred: true,
    }
}

/// Steps to the previous save, if any. Stepping, like picking from the
/// list, selects the new slot's checkpoint first.
pub fn previous_save(index: i32) -> Option<i32> {
    (index > 0).then_some(index - 1)
}

/// Steps to the next save, if any.
pub fn next_save(index: i32, count: i32) -> Option<i32> {
    // The list's last index is one below the save count.
    (i64::from(index) < i64::from(count) - 1).then_some(index + 1)
}

/// Selects the save at one-based list position `selected` unless it is
/// already the current one. The comparison is made in 64 bits and the
/// index passed on is the 32-bit one below `selected`, unchecked.
pub fn select_save_from_list(selected: i32, current: i32) -> Option<i32> {
    (i64::from(selected) - 1 != i64::from(current)).then_some(selected.wrapping_sub(1))
}

/// Toggles the save kind against the availability mask.
pub fn toggle_save_kind(kind: SaveKind, availability: SaveAvailability) -> Option<SaveKind> {
    match kind {
        SaveKind::Save => availability
            .has(SaveKind::Checkpoint)
            .then_some(SaveKind::Checkpoint),
        SaveKind::Checkpoint => availability.has(SaveKind::Save).then_some(SaveKind::Save),
    }
}

/// What deleting the selected save did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteSave {
    pub index: i32,
    /// No save is left; the continue screen opens its empty state.
    pub empty: bool,
}

/// Deletes the selected save; the selection moves to the last remaining.
pub fn delete_save(index: i32, count: i32) -> DeleteSave {
    let remaining = count.saturating_sub(1).max(0);
    let index = index.min(remaining - 1);
    DeleteSave {
        index,
        empty: index < 0,
    }
}

/// The continue screen's button and navigation state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinueLayout {
    pub button: CreationContinue,
    pub previous_disabled: bool,
    pub next_disabled: bool,
    /// The name shown, cut to fifteen characters (the original copies
    /// fifteen bytes of its single-byte strings).
    pub name: String,
}

/// Lays out the continue screen for a save index.
pub fn continue_layout(
    index: i32,
    count: i32,
    kind: SaveKind,
    availability: SaveAvailability,
    name: &str,
) -> ContinueLayout {
    ContinueLayout {
        button: creation_continue_button(kind.code(), availability.0 as i32),
        previous_disabled: index < 1,
        next_disabled: i64::from(index) >= i64::from(count) - 1,
        name: name.chars().take(15).collect(),
    }
}

/// The voice pitch a slider value produces: 0.9 to 1.07. The product and
/// sum are worked in double precision and stored as a float.
pub fn voice_pitch(slider: f32) -> f32 {
    (f64::from(slider) * 0.17 + 0.9) as f32
}

/// The two-digit number the voice names use: at least two digits, and a
/// lone zero for a negative input (the original formats no digits and then
/// prepends one zero).
pub fn two_digits(value: i32) -> String {
    if value < 0 {
        return "0".to_string();
    }
    if value < 10 {
        format!("0{value}")
    } else {
        value.to_string()
    }
}

/// The suffix every voice sample name ends with.
pub const VOICE_SUFFIX: &str = "_GruntS";

/// The voice name prefix a sex change sets: `V` and the appearance label
/// (`VHM`, `VHF`, …), or a bare `V` for a race with no label.
pub fn voice_prefix(label: CreationLabel) -> String {
    format!("V{}", label.code().unwrap_or(""))
}

/// The sound a voice selection plays.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoicePreview {
    /// The name the sound is looked up under.
    pub name: String,
    /// The same voice with the variant suffix, used for the alternative.
    pub variant_name: String,
    pub pitch_bits: u32,
}

/// Builds the sound names of a selected voice, played when its sample
/// exists: the prefix, the one-based voice number and `_GruntS`
/// (`VHM01_GruntS`); the alternative inserts `DA` before the number
/// (`VHMDA01_GruntS`). Both previews play at volume 0.4.
pub fn voice_preview(prefix: &str, index: i32, pitch: f32) -> VoicePreview {
    let number = two_digits(index.wrapping_add(1));
    VoicePreview {
        name: format!("{prefix}{number}{VOICE_SUFFIX}"),
        variant_name: format!("{prefix}DA{number}{VOICE_SUFFIX}"),
        pitch_bits: voice_pitch(pitch).to_bits(),
    }
}

/// Steps the voice selection down, stopping at zero.
pub fn previous_voice(index: i32) -> Option<i32> {
    (index > 0).then_some(index - 1)
}

/// Steps the voice selection up; there is no upper bound, the selection
/// just fails when no sample has the number.
pub fn next_voice(index: i32) -> i32 {
    index.wrapping_add(1)
}

/// The voice preview button's throttle: a preview at most once per 300
/// clock units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoiceThrottle {
    pub played: bool,
    pub last: i32,
}

/// Allows a voice preview when 300 clock units have passed.
///
/// Both words are zero-extended and subtracted in 64 bits, so a stamp
/// ahead of the clock gives a negative distance and blocks the preview.
pub fn throttle_voice(clock: i32, last: i32) -> VoiceThrottle {
    if i64::from(clock as u32) - i64::from(last as u32) >= 300 {
        VoiceThrottle {
            played: true,
            last: clock,
        }
    } else {
        VoiceThrottle {
            played: false,
            last,
        }
    }
}

/// The hair checkbox's state word from the record's flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HairButton {
    /// A female record: the box is hidden (state 0x10, the same bit the
    /// look hides the face variant controls with).
    Hidden,
    /// A male record with a face variant: state 0x60, which holds 0x40,
    /// the bit the sex radio reads as checked.
    Checked,
    /// A male record without a face variant: state 0.
    Clear,
}

impl HairButton {
    /// The state word the original stores on the checkbox.
    pub fn state(self) -> u32 {
        match self {
            Self::Hidden => 0x10,
            Self::Checked => 0x60,
            Self::Clear => 0,
        }
    }
}

/// Reads the hair checkbox's state: bit 0 (female) hides it, otherwise a
/// face variant checks it.
pub fn hair_button(sex_or_word: i32, variant_bits: i32) -> HairButton {
    if sex_or_word & 1 != 0 {
        HairButton::Hidden
    } else if variant_bits & 0xf0_0000 != 0 {
        HairButton::Checked
    } else {
        HairButton::Clear
    }
}

/// Sets or clears the hair visibility flag from the checkbox.
pub fn set_hair_visible(word: i32, checkbox_on: bool) -> i32 {
    if checkbox_on {
        word | 0x1_0000
    } else {
        word & !0x1_0000
    }
}

/// The sex change's record word, label and voice prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SexChange {
    pub word: i32,
    pub label: CreationLabel,
    /// The prefix the voice names are built from (see [`voice_prefix`]).
    pub voice_prefix: String,
    /// The look panel was refreshed.
    pub look_refreshed: bool,
    /// Both voice selections were reset to zero (and previewed).
    pub voice_reset: bool,
    /// Previews were muted while the two selections were made.
    pub muted: bool,
    /// The audio device was opened first (it was not ready yet).
    pub audio_opened: bool,
    /// The change callback ran.
    pub callback: bool,
}

/// Applies the sex radio choice: 2 is female.
///
/// The original only reacts to the radio button being checked. After the
/// word and prefix change it reselects voice 0 and variant voice 0 with
/// the preview muted, then runs the change callback.
pub fn change_sex(word: i32, radio: i32) -> SexChange {
    let word = if radio == 2 { word | 1 } else { word & !1 };
    let label = creation_label(word);
    SexChange {
        word,
        label,
        voice_prefix: voice_prefix(label),
        look_refreshed: false,
        voice_reset: false,
        muted: false,
        audio_opened: false,
        callback: false,
    }
}

/// Applies the sex radio only when it was checked, and reports the full
/// refresh: the look panel when it exists, the audio device opened when it
/// was not ready yet, both voice selections previewed under the mute flag,
/// and the callback.
pub fn sex_radio(
    word: i32,
    radio: i32,
    checked: bool,
    panel: bool,
    audio_ready: bool,
) -> Option<SexChange> {
    if !checked {
        return None;
    }
    let mut change = change_sex(word, radio);
    change.look_refreshed = panel;
    change.voice_reset = true;
    change.muted = true;
    change.audio_opened = !audio_ready;
    change.callback = true;
    Some(change)
}

/// A starting kit table. The original keeps seven: five offered only as
/// apparel, one only as weapons and one offered as apparel in versus and
/// as weapons in mode 2's alternative setup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KitTable {
    /// The apparel most modes offer.
    Default,
    /// Mode 2's apparel when its alternative setup flag is set.
    Intro,
    /// The arena recruit's apparel, also the kit the arena filter keeps
    /// hand items from.
    ArenaRecruit,
    /// The arena career character's apparel.
    ArenaContinue,
    /// The unknown story background's apparel.
    StoryUnknown,
    /// Versus apparel, and mode 2's weapons with its alternative setup.
    Versus,
    /// The weapons every other mode but story offers.
    Weapons,
}

impl KitTable {
    /// The entries the table holds.
    pub fn entries(self) -> usize {
        match self {
            Self::Default | Self::Weapons => 6,
            Self::Intro | Self::ArenaRecruit => 10,
            Self::ArenaContinue => 15,
            Self::StoryUnknown => 5,
            Self::Versus => 7,
        }
    }
}

/// The apparel kit table a mode offers, or none.
///
/// The arena offers the recruit's or the career's table; mode 2 its own or
/// its alternative one (`intro`); story only the unknown background's
/// (a named background's panel is built and left unfilled); versus its
/// own; every other mode the default table.
pub fn apparel_kit_table(
    game_mode: i32,
    screen_mode: i32,
    background: i32,
    intro: bool,
) -> Option<KitTable> {
    match game_mode {
        8 if screen_mode == 10 => Some(KitTable::ArenaRecruit),
        8 => Some(KitTable::ArenaContinue),
        2 if intro => Some(KitTable::Intro),
        4 => (background == 0).then_some(KitTable::StoryUnknown),
        0x1010 => Some(KitTable::Versus),
        _ => Some(KitTable::Default),
    }
}

/// The weapon kit table a mode offers, or none: story offers no weapons
/// (its panel is built empty), mode 2's alternative setup offers the
/// versus table, every other mode the weapon table.
pub fn weapon_kit_table(game_mode: i32, intro: bool) -> Option<KitTable> {
    match game_mode {
        4 => None,
        2 if intro => Some(KitTable::Versus),
        _ => Some(KitTable::Weapons),
    }
}

/// One starting kit entry: an item and the appearance it is given.
///
/// A table entry is 19 bytes: the item, the variant word, three tint bytes,
/// four bytes this unit does not read, then the four colour-layer bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KitItem {
    pub item: u32,
    /// The variant word.
    pub variant: u32,
    /// The three tint bytes. An alias's target takes all three; a made
    /// instance takes the second and third as its two dye bytes, each only
    /// when it is set.
    pub tint: [u8; 3],
    /// The four colour-layer bytes a made instance writes and the kit match
    /// compares.
    pub layers: [u8; 4],
}

impl KitItem {
    /// A plain item with no appearance.
    pub const fn plain(item: u32) -> Self {
        Self {
            item,
            variant: 0,
            tint: [0; 3],
            layers: [0; 4],
        }
    }
}

/// A database item record's fields the kit match reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KitRecord {
    /// The item's base id.
    pub base: u32,
    /// The first byte of each of the item's four colour layers, or none
    /// when the record has no colour-layer block.
    pub layers: Option<[u8; 4]>,
}

/// Whether a kit entry already covers an item.
///
/// The item matches by id, or its database record's base id matches the
/// entry and its four colour-layer bytes equal the entry's, compared in
/// layer order. A record without a colour-layer block never matches.
pub fn kit_item_matches(item: u32, record: Option<&KitRecord>, spec: &KitItem) -> bool {
    if item == spec.item {
        return true;
    }
    match record {
        Some(record) => record.base == spec.item && record.layers == Some(spec.layers),
        None => false,
    }
}

/// The kit entries missing from the worn list.
pub fn missing_apparel(
    worn: &[GearSlot],
    table: &[KitItem],
    record_of: impl Fn(u32) -> Option<KitRecord>,
) -> Vec<KitItem> {
    table
        .iter()
        .filter(|spec| {
            !worn
                .iter()
                .any(|slot| kit_item_matches(slot.item, record_of(slot.item).as_ref(), spec))
        })
        .copied()
        .collect()
}

/// The kit entries missing from the first two hand slots.
///
/// Weapons match by database base id, not by colour. An empty hand is
/// looked up like any other item, so `base_of` must answer for id 0.
pub fn missing_weapons(
    hands: &[GearSlot; 4],
    table: &[KitItem],
    base_of: impl Fn(u32) -> u32,
) -> Vec<KitItem> {
    table
        .iter()
        .filter(|spec| {
            !hands[..2]
                .iter()
                .any(|slot| base_of(slot.item) == spec.item)
        })
        .copied()
        .collect()
}

/// What a kit entry looks like once resolved against the item database.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedKitItem {
    pub item: u32,
    /// The appearance written on the resolved item, or none for a plain
    /// item.
    pub appearance: Option<KitAppearance>,
}

/// The byte a made instance writes at the end of each colour layer.
pub const KIT_LAYER_MARKER: u8 = 5;

/// The appearance a kit entry writes on the item it resolves to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KitAppearance {
    /// An alias's target record takes the variant in both of its variant
    /// words and the three tint bytes, each set through the record's own
    /// setter with a zero blend.
    Alias { variant: u32, tint: [u8; 3] },
    /// A made instance takes the variant word, the four colour-layer bytes
    /// (each layer ending with [`KIT_LAYER_MARKER`]) and the second and third
    /// tint bytes as its two dye bytes where they are set: a zero leaves the
    /// item's own byte. Its description is rebuilt only for a non-zero
    /// variant.
    Instance {
        variant: u32,
        layers: [u8; 4],
        dyes: [Option<u8>; 2],
    },
}

/// The route a kit entry takes through the item database.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KitRoute {
    /// The item record's flag bit 0 is clear: the entry is added as itself.
    Plain,
    /// The item has an appearance block and follows to an alias record.
    /// `marker` is that record's first name byte; only [`ALIAS_MARKER`]
    /// makes the alias apply, and any other byte leaves the entry as itself.
    Alias { marker: u8 },
    /// A fresh instance of the item.
    Instance,
}

/// The alias record's marker byte that makes a kit entry follow it.
pub const ALIAS_MARKER: u8 = b'@';

/// Resolves a kit entry by the route its database record takes.
///
/// A plain record adds the entry as itself. An alias whose marker is
/// [`ALIAS_MARKER`] gives the entry's variant and tint to the item the
/// database resolves it to (`created`); with nothing resolved, or any other
/// marker, the entry is added unchanged. An instance takes the entry's
/// variant, colour layers and set dyes on the item made for it, and with
/// nothing made nothing is added (item 0).
///
/// The record's sub-record layout belongs to the object database; only the
/// route, the marker byte and the made item cross this boundary.
pub fn resolve_kit_item(spec: &KitItem, route: KitRoute, created: Option<u32>) -> ResolvedKitItem {
    match (route, created) {
        (KitRoute::Alias { marker }, Some(item)) if marker == ALIAS_MARKER => ResolvedKitItem {
            item,
            appearance: Some(KitAppearance::Alias {
                variant: spec.variant,
                tint: spec.tint,
            }),
        },
        // Any other marker, or no target, leaves the entry as itself.
        (KitRoute::Alias { .. }, _) | (KitRoute::Plain, _) => ResolvedKitItem {
            item: spec.item,
            appearance: None,
        },
        (KitRoute::Instance, Some(item)) => ResolvedKitItem {
            item,
            appearance: Some(KitAppearance::Instance {
                variant: spec.variant,
                layers: spec.layers,
                dyes: [spec.tint[1], spec.tint[2]].map(|dye| (dye != 0).then_some(dye)),
            }),
        },
        // An instance that could not be made adds nothing.
        (KitRoute::Instance, None) => ResolvedKitItem {
            item: 0,
            appearance: None,
        },
    }
}

/// The creator icon sheet's frame size.
pub const CREATION_ICON_SIZE: i32 = 96;
/// The sheet's four 96-pixel cells are stacked vertically, cut at these
/// offsets largest first (a 96 by 384 image).
pub const CREATION_ICON_OFFSETS: [i32; 4] = [288, 192, 96, 0];

/// The icon size the creator's icon renderer is given: eight times the
/// GUI unit (32 at unit 4), with its entry height two more.
pub fn creation_icon_size(unit: i32) -> i32 {
    unit * 8
}

/// A creator panel's place in its form's client rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelPlace {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// The apparel panel: a quarter-unit inset, 0xac wide, 0x26c or 0x238
/// tall depending on the window's aspect flag.
pub fn apparel_panel_place(tall: bool) -> PanelPlace {
    PanelPlace {
        x: 4,
        y: 0x18,
        w: 0xac,
        h: if tall { 0x26c } else { 0x238 },
    }
}

/// What showing the equipment page's apparel panel did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApparelPanel {
    /// The versus equipment form was hidden first (it exists).
    pub versus_equipment_hidden: bool,
    /// The panel was made (none existed) rather than shown again.
    pub created: bool,
    pub shown: bool,
    /// The panel's item loop finished (the made panel only).
    pub finished: bool,
    /// The screen's current equipment form took the panel.
    pub current_form: bool,
    /// The kit entries the loop added.
    pub items: usize,
}

/// Shows the equipment page's apparel panel and fills it with the mode's
/// kit.
///
/// The versus equipment form is hidden first when it exists, and the form's
/// first tab is captioned APPAREL. A missing panel is made at
/// [`apparel_panel_place`], shown and bound as the current equipment form
/// at once, then filled from [`apparel_kit_table`]'s entries: every entry
/// the worn list does not already cover is resolved and added
/// ([`missing_apparel`], [`resolve_kit_item`]) and the panel finishes. A
/// named story background (no table) returns right after that binding, with
/// no items and no finish. A panel that already exists is shown again,
/// keeps its items and is bound as the current form; the table is not
/// consulted.
pub fn show_apparel_panel(
    has_versus_equipment: bool,
    existing: bool,
    kit_entries: Option<usize>,
    added: usize,
) -> ApparelPanel {
    let created = !existing;
    // Both routes bind the panel as the current equipment form: the made
    // one before the table is picked, the existing one after it is shown.
    let filled = created && kit_entries.is_some();
    ApparelPanel {
        versus_equipment_hidden: has_versus_equipment,
        created,
        shown: true,
        finished: filled,
        current_form: true,
        items: if filled { added } else { 0 },
    }
}

/// The weapons panel beside the apparel one.
pub fn weapon_panel_place(parent_width: i32, tall: bool) -> PanelPlace {
    PanelPlace {
        x: parent_width - 0xb0,
        y: 0x18,
        w: 0xac,
        h: if tall { 0x26c } else { 0x238 },
    }
}

/// The bit the weapons panel sets in its flag word.
pub const WEAPON_PANEL_FLAGS: u32 = 0x2_0000;

/// What showing the equipment page's weapons panel did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeaponPanel {
    /// The second preview panel was hidden first (it exists).
    pub second_panel_hidden: bool,
    /// The panel was made (none existed) rather than shown again.
    pub created: bool,
    pub shown: bool,
    /// The panel's item loop finished (the made panel only).
    pub finished: bool,
    /// The panel's flag word gained [`WEAPON_PANEL_FLAGS`] when it was
    /// made.
    pub flags: u32,
    /// The kit entries the loop added.
    pub items: usize,
}

/// Shows the equipment page's weapons panel and fills it with the mode's
/// kit.
///
/// The second preview panel is hidden first when it exists and the form's
/// second tab is captioned WEAPONS. A missing panel is made beside the
/// apparel one at [`weapon_panel_place`] with [`WEAPON_PANEL_FLAGS`] set,
/// shown, and filled from [`weapon_kit_table`]'s entries: every entry whose
/// item id none of the first two hands' base items equals is resolved and
/// added ([`missing_weapons`], [`resolve_kit_item`]), then the panel
/// finishes. An existing panel is only shown again. Story mode offers no
/// weapons, so its panel is made and left empty.
pub fn show_weapon_panel(
    has_second_panel: bool,
    existing: bool,
    kit_entries: Option<usize>,
    added: usize,
) -> WeaponPanel {
    let created = !existing;
    let empty = kit_entries.is_none_or(|entries| entries == 0);
    WeaponPanel {
        second_panel_hidden: has_second_panel,
        created,
        shown: true,
        finished: created,
        flags: if created { WEAPON_PANEL_FLAGS } else { 0 },
        items: if created && !empty { added } else { 0 },
    }
}

/// The skills panel: 0xd8 wide, 0x204 tall when recruiting and 0x294
/// otherwise, against the parent's right edge.
pub fn skill_panel_place(parent_width: i32, screen_mode: i32) -> PanelPlace {
    let w = 0xd8;
    let h = if screen_mode == 10 { 0x204 } else { 0x294 };
    PanelPlace {
        x: parent_width - w - 0xc,
        y: 0x18,
        w,
        h,
    }
}

/// The skill page's slide and tab positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillTabPlace {
    /// The skills panel's width after the slide.
    pub width: i32,
    /// The tab's horizontal offset.
    pub tab_x: i32,
    /// The tab's vertical offset.
    pub tab_y: i32,
}

/// Lays out the skills page's slide for its index.
///
/// The panel's width shrinks by one unit per index. A continued character
/// does not move the tab labels.
pub fn skill_tab_place(
    form_width: i32,
    panel_x: i32,
    unit: i32,
    index: i32,
    continuing: bool,
) -> SkillTabPlace {
    let (tab_x, tab_y) = if continuing {
        (0, 0)
    } else {
        // The original multiplies and subtracts in 32 bits and stores the
        // low 16 bits of each result, so the fields wrap like a word.
        let x = index.wrapping_sub(4).wrapping_mul(unit);
        let y = form_width.wrapping_sub(index.wrapping_sub(2).wrapping_mul(unit));
        (i32::from(x as u16 as i16), i32::from(y as u16 as i16))
    };
    SkillTabPlace {
        width: form_width - panel_x - unit * index,
        tab_x,
        tab_y,
    }
}

/// The continue screen's panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContinuePlace {
    /// The character view's left edge: 59 GUI units.
    pub view_x: i32,
    /// The carried items' list.
    pub list: PanelPlace,
    /// The details form under it.
    pub details: PanelPlace,
    /// The name box inside the details form, down to its bottom.
    pub name: PanelPlace,
    /// The item list's scroll step and its two state words.
    pub list_step: i32,
    pub list_flags: u32,
    pub list_word: u32,
    /// The name box's two state words.
    pub name_flags: u32,
    pub name_word: u32,
    /// The details form's own state word.
    pub details_word: i32,
}

/// Lays out the continue screen. `tall` is the same window aspect flag the
/// equipment panels read (clear in the original's short layout).
pub fn continue_place(unit: i32, tall: bool) -> ContinuePlace {
    let details = PanelPlace {
        x: 0,
        y: 0x148,
        w: 0xec,
        h: if tall { 0x140 } else { 0x104 },
    };
    ContinuePlace {
        view_x: unit * 0x3b,
        list: PanelPlace {
            x: 4,
            y: 0x74,
            w: 0xe4,
            h: 0xb4,
        },
        details,
        name: PanelPlace {
            x: 4,
            y: 0x28,
            w: 0xe4,
            h: details.h - 0x28,
        },
        list_step: 2,
        list_flags: 0x80,
        list_word: 0x0010_0010,
        name_flags: 0x200,
        name_word: 0x0600_0200,
        details_word: 0x100,
    }
}

/// Which creator the open action shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenCreator {
    /// The continue screen's initial choice.
    pub continue_kind: SaveKind,
    pub show_continue: bool,
    pub reset_to_portal: bool,
    pub update_input: bool,
}

/// Opens the creator form, choosing the continue screen's first slot.
pub fn open_creator(
    screen_mode: i32,
    availability: SaveAvailability,
    reset_to_portal: bool,
) -> OpenCreator {
    OpenCreator {
        continue_kind: if availability.0 == 2 {
            SaveKind::Save
        } else {
            SaveKind::Checkpoint
        },
        show_continue: screen_mode == 12,
        reset_to_portal: screen_mode == 12 && reset_to_portal,
        update_input: true,
    }
}

/// The equipment page's tab action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquipmentTab {
    /// The character tab stores the record and refreshes the figure.
    Character,
    /// Another tab shows or hides its form flag.
    Other { enabled: bool },
}

/// Reads the equipment page's tab index; 2 is the character tab.
pub fn equipment_tab(index: i32) -> EquipmentTab {
    if index == 2 {
        EquipmentTab::Character
    } else {
        EquipmentTab::Other {
            enabled: index != 0,
        }
    }
}

/// Which of the record's two voice words a selection wrote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceWord {
    /// Nothing was written.
    None,
    /// The record's voice word (the plain selection's).
    Voice,
    /// The record's variant voice word (the selection beside the voice).
    Variant,
}

/// What selecting a voice did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoiceChoice {
    /// The voice the screen keeps, when the selection took.
    pub index: Option<i32>,
    /// The record word the selection wrote.
    pub word: VoiceWord,
    /// The caption's letter: the index plus `A`, truncated to a byte. The
    /// plain selection's caption is the base form's seventh control, the
    /// variant's the ninth.
    pub caption: Option<u8>,
    /// The sample loaded and its preview plays (previews are not muted).
    pub plays: bool,
}

/// Selects a voice, taking it only when its sample exists.
///
/// The `_GruntS` sample named after the prefix and the one-based voice must
/// load for anything to happen: the screen keeps the index, the record's
/// voice word takes it, the caption shows `A` plus the index and the sample
/// previews at the voice's stored pitch (volume 0.4) unless previews are
/// muted.
pub fn select_voice(index: i32, sample_exists: bool, muted: bool) -> VoiceChoice {
    if !sample_exists {
        return VoiceChoice {
            index: None,
            word: VoiceWord::None,
            caption: None,
            plays: false,
        };
    }
    VoiceChoice {
        index: Some(index),
        word: VoiceWord::Voice,
        caption: Some(index.wrapping_add(0x41) as u8),
        plays: !muted,
    }
}

/// Selects the variant voice beside the voice.
///
/// A record whose race band (bits 8 through 15 of its profession word) is
/// set takes the index straight into the variant voice word, with no sample
/// and no preview. Otherwise the `DA` sample must load: the screen keeps
/// the index, the variant voice word takes it, the ninth control's caption
/// shows `A` plus the index, and the sample previews like the plain one.
pub fn select_variant_voice(
    index: i32,
    race_band: bool,
    sample_exists: bool,
    muted: bool,
) -> VoiceChoice {
    if race_band {
        return VoiceChoice {
            index: Some(index),
            word: VoiceWord::Variant,
            caption: None,
            plays: false,
        };
    }
    if !sample_exists {
        return VoiceChoice {
            index: None,
            word: VoiceWord::None,
            caption: None,
            plays: false,
        };
    }
    VoiceChoice {
        index: Some(index),
        word: VoiceWord::Variant,
        caption: Some(index.wrapping_add(0x41) as u8),
        plays: !muted,
    }
}

/// The record fields a loaded preset leaves behind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresetLoad {
    /// The profession word keeps its sex bit and clears its race byte.
    pub profession: i32,
    /// The known powers are cleared.
    pub known_powers: i32,
    /// The ten power slots are cleared.
    pub power_slots_cleared: bool,
}

/// Finishes a successful preset load.
///
/// The record's race byte is cleared, its known powers and power slots
/// are emptied; the rest of the record stays as the preset wrote it.
pub fn apply_loaded_preset(profession: i32) -> PresetLoad {
    PresetLoad {
        profession: profession & !0x0000_ff00,
        known_powers: 0,
        power_slots_cleared: true,
    }
}

/// What adopting a background's template did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TemplateUse {
    /// The record the creator had before was freed.
    pub replaced: bool,
    /// The template's known powers are cleared.
    pub known_powers: i32,
    /// Its ten power slots are cleared.
    pub power_slots_cleared: bool,
}

/// Adopts the template record when a named background has no saved preset.
pub fn use_template(has_record: bool) -> TemplateUse {
    TemplateUse {
        replaced: has_record,
        known_powers: 0,
        power_slots_cleared: true,
    }
}

/// Adding an item preview to the panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreviewPush {
    pub count: i32,
    /// The item selected after the push, if the previous last one was
    /// hovered.
    pub selected: Option<i32>,
    /// The item view's entry list was refreshed.
    pub refreshed: bool,
    /// The icon manager finished the new icon.
    pub finished: bool,
}

/// Adds a preview icon; a hovered last item moves the selection to the new
/// last one.
///
/// The icon is registered with the icon manager first, then the list grows
/// by one entry holding the new icon, the item view's entries are refreshed
/// and the manager finishes. The selection then follows the hover rule.
pub fn push_preview(count: i32, hovered: Option<i32>) -> PreviewPush {
    let count = count.wrapping_add(1);
    // The hovered index is zero-extended and the count sign-extended, both
    // in 64 bits.
    let selected = hovered
        .filter(|hovered| u64::from(*hovered as u32) as i64 + 1 == i64::from(count) - 1)
        .map(|_| count - 1);
    PreviewPush {
        count,
        selected,
        refreshed: true,
        finished: true,
    }
}

/// The row and name a pick in the continue screen's list shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContinueNameRow<'a> {
    /// The row the page keeps: the one-based pick less one, unchecked.
    pub row: i32,
    /// The row's name, which fills the details box's name control.
    pub text: Option<&'a str>,
}

/// The name a pick in the continue screen's list fills in.
///
/// The control reports a one-based row; the page stores it less one and the
/// name of the list entry at that row becomes the name control's text. The
/// original computes the index without a check; here a pick with no row
/// behind it stores the word and writes no text.
pub fn continue_name_row<'a>(selected: i32, rows: &[&'a str]) -> ContinueNameRow<'a> {
    let row = selected.wrapping_sub(1);
    let text = usize::try_from(row)
        .ok()
        .and_then(|row| rows.get(row))
        .copied();
    ContinueNameRow { row, text }
}

/// The two texts the continue page opens with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContinueTexts<'a> {
    /// The row the page keeps, when it is written (always the first).
    pub row: Option<i32>,
    /// The label list's selection and length: the first of the list's
    /// entries; missing when the list is empty.
    pub label: Option<[i32; 2]>,
    /// The label's text: the list's first display string, or none (the
    /// text is cleared) when the list is empty.
    pub label_text: Option<&'a str>,
    /// The details box's name: the first entry's name, or none (cleared).
    pub name_text: Option<&'a str>,
}

/// Fills the continue page's label and name box when it opens.
///
/// The list belongs to the save manager, which refreshes it first: its
/// display strings (`labels`, whose length is the list's count) and the
/// entries' names (`names`). With any entry the label selects the first of
/// `count` entries and shows its display string, and the pick handler runs
/// on that selection ([`continue_name_row`] with row one), so the page keeps
/// row 0 and the name box takes the first name. An empty list clears both
/// texts and leaves the kept row alone.
pub fn continue_texts<'a>(labels: &[&'a str], names: &[&'a str]) -> ContinueTexts<'a> {
    if labels.is_empty() {
        return ContinueTexts {
            row: None,
            label: None,
            label_text: None,
            name_text: None,
        };
    }
    let pick = continue_name_row(1, names);
    ContinueTexts {
        row: Some(pick.row),
        label: Some([1, labels.len() as i32]),
        label_text: labels.first().copied(),
        name_text: pick.text,
    }
}

/// The delete confirmation's text: the question and the character's name.
pub fn delete_confirm_text(question: &str, name: &str) -> String {
    format!("{question}{name}?")
}

/// Copies a character's name into the screen's string.
pub fn copy_creation_name(name: &str) -> String {
    name.to_string()
}

/// What the creator's icon sheet leaves behind, and its reload hook.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationIconHook {
    /// The sheet was built (the manager was missing).
    pub built: bool,
    /// A previous texture-reload hook exists and is called before every
    /// rebuild ([`build_creation_icons`]'s chain).
    pub previous_hook: bool,
    /// This builder took the reload hook's slot, after the previous one.
    pub installed: bool,
}

/// Gets the creator's icon sheet, building it on first use.
///
/// An existing manager is returned untouched: no rebuild and no chaining. A
/// missing one is built, the texture-reload hook already installed is kept
/// to be called first, and the builder takes the hook's slot so the sheet is
/// cut again when the textures reload [the hook's slot is a global this unit
/// only chains into].
pub fn ensure_creation_icons(existing: bool, previous_hook: bool) -> CreationIconHook {
    if existing {
        CreationIconHook {
            built: false,
            previous_hook,
            installed: false,
        }
    } else {
        CreationIconHook {
            built: true,
            previous_hook,
            installed: true,
        }
    }
}

/// The voice variants played last, so a preview does not repeat one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VoiceHistory {
    pub last: usize,
    pub before_last: usize,
}

/// Draws the voice variant a preview plays, or none for an empty sound.
///
/// One sample always plays the first and does not advance the engine's
/// generator; two or three avoid the last variant, and four or more avoid
/// the last two, redrawing until they do. Every played variant pushes the
/// history. The audio environment's own play routine does this, and only
/// once its device is open.
pub fn draw_voice_variant(
    rng: &mut Lcg,
    count: usize,
    history: &mut VoiceHistory,
) -> Option<usize> {
    let index = match count {
        0 => return None,
        1 => 0,
        2 | 3 => {
            let mut index = rng.below(count as u32) as usize;
            while index == history.last {
                index = rng.below(count as u32) as usize;
            }
            index
        }
        _ => {
            let mut index = rng.below(count as u32) as usize;
            while index == history.last || index == history.before_last {
                index = rng.below(count as u32) as usize;
            }
            index
        }
    };
    history.before_last = history.last;
    history.last = index;
    Some(index)
}

/// The character creation form's size and style word, as its constructor
/// sets them.
pub const CREATION_FORM_WIDTH: i32 = 1024;
pub const CREATION_FORM_HEIGHT: i32 = 768;
pub const CREATION_FORM_STYLE: u32 = 0x80_0020;
/// The form mode whose layout has ten elements and no editor sub-forms;
/// every other mode has seven and the two editors.
pub const TEN_ELEMENT_MODE: i32 = 0xc;
/// The preview viewport's left and top in its parent, the width it leaves
/// to the parent's other controls, and the height the form trims off it.
pub const PREVIEW_LEFT: i32 = 180;
pub const PREVIEW_TOP: i32 = 24;
pub const PREVIEW_WIDTH_LEFT: i32 = 360;
pub const PREVIEW_HEIGHT_TRIM: i32 = 4;
/// Two values the preview sets on its view (400.0 and 0.34), read here as
/// the fog's start and density [guess: the fields are not named].
pub const PREVIEW_FOG_FROM: f32 = 400.0;
pub const PREVIEW_FOG_DENSITY: f32 = 0.34;

/// The preview's floor scene.
pub const PREVIEW_SCENE: &str = "sgccflr01.rfc";
/// The preview camera's eye and target in centimetres (the eye is also the
/// scene's stored eye, the target its stored target).
pub const PREVIEW_EYE_CM: [f32; 3] = [-200.0, 210.0, 360.0];
pub const PREVIEW_TARGET_CM: [f32; 3] = [0.0, 60.0, 0.0];
/// The preview camera's field of view, degrees.
pub const PREVIEW_FOV_DEG: f32 = 50.0;
/// The preview view's scale.
pub const PREVIEW_SCALE: f32 = 1.0;
/// The fog mode the preview scene's environment takes (fade to black).
pub const PREVIEW_FOG_MODE: u32 = 2;
/// The word the preview writes into its view's render object.
pub const PREVIEW_RENDER_WORD: u32 = 0x4100;
/// The global the preview sets to 12 before it creates its view (its role
/// is not traced).
pub const PREVIEW_GLOBAL_MODE: i32 = 0xc;

/// The preview viewport's form words and switches, as its builder sets
/// them on the new view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreviewView {
    /// The view's border word.
    pub border: i32,
    /// Bits or-ed into the form-flag word.
    pub form_flags: u32,
    /// The style word: its bits 14 and 15 cleared and 0x90 set.
    pub style_clear: u32,
    pub style_set: u32,
    /// The view's bloom switch byte and its second switch byte, both
    /// cleared.
    pub bloom: bool,
    pub second_switch: bool,
    /// The mesh-smoothing switch is raised while the preview lives (the
    /// close lowers it), so the preview's hair is subdivided.
    pub smoothing: bool,
}

/// The preview view's words: border 2, form flags `0xf0080`, style
/// `(style & !0xc000) | 0x90`, both switches off and smoothing on.
pub fn preview_view() -> PreviewView {
    PreviewView {
        border: 2,
        form_flags: 0xf_0080,
        style_clear: 0xc000,
        style_set: 0x90,
        bloom: false,
        second_switch: false,
        smoothing: true,
    }
}

impl PreviewView {
    /// The view's style word over `style`.
    pub fn style(&self, style: u32) -> u32 {
        (style & !self.style_clear) | self.style_set
    }
}

/// What closing the creation form does, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreationFormClose {
    /// The mesh-smoothing switch and the form's other global are lowered.
    pub smoothing: bool,
    /// Each apparel-list item's id, in list order, is handed to the item
    /// refresh.
    pub refreshed: Vec<u32>,
    /// The child form, when present, is closed with argument 0.
    pub close_child: bool,
    /// The edit form's close then runs ([`close_edit_form`]).
    pub edit: EditClose,
}

/// The creation form's close: lower the switches, refresh every apparel
/// item, close the child form, refresh the game screen's item state, then
/// the edit form's close.
pub fn creation_form_close(
    apparel: &[u32],
    has_child: bool,
    has_editor: bool,
) -> CreationFormClose {
    CreationFormClose {
        smoothing: false,
        refreshed: apparel.to_vec(),
        close_child: has_child,
        edit: close_edit_form(has_editor),
    }
}

/// What the creation form's destroy does with its shared drop record (a
/// reference-counted record the preview view also holds): a count under 2
/// frees it; otherwise its owner is cleared and the count drops by one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropRecordRelease {
    Freed,
    Released { count: i32 },
}

pub fn release_drop_record(count: i32) -> DropRecordRelease {
    if count < 2 {
        DropRecordRelease::Freed
    } else {
        DropRecordRelease::Released { count: count - 1 }
    }
}

/// The creation form's shared change callback (both editors report to it),
/// step by step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewChangeStep {
    /// The busy counter rises.
    Busy,
    /// The character's skin and material textures are regenerated from the
    /// record.
    Regenerate,
    /// The preview's character is refreshed.
    Refresh,
    /// The hair picker's pick result is recomputed from the character's
    /// hair (only with a hair picker).
    HairPick,
    /// The texture loader's pass.
    Textures,
    /// The procedural material pass.
    Materials,
    /// The busy counter falls.
    Idle,
}

/// The change callback's steps in order.
pub fn preview_change(hair_picker: bool) -> Vec<PreviewChangeStep> {
    use PreviewChangeStep::*;
    let mut steps = vec![Busy, Regenerate, Refresh];
    if hair_picker {
        steps.push(HairPick);
    }
    steps.extend([Textures, Materials, Idle]);
    steps
}

/// The preview's body refresh: with a hair picker its pick result is
/// recomputed from the previewed character's hair.
pub fn refresh_preview_body(hair_picker: bool) -> bool {
    hair_picker
}

/// The drop into the form's mode container (the second drop routine):
/// item 8 is ignored; with a mode list the item is added and, when added,
/// placed.
pub fn mode_container_drop(item: u32, mode_list: bool) -> Option<EquipDrop> {
    (item != 8 && mode_list).then_some(EquipDrop {
        list: DropList::Mode,
        place: true,
    })
}

/// The preview viewport's frame: left, top, width and height in parent
/// units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreviewFrame {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// The preview viewport in a form whose parent is `parent` units: 180 in,
/// 24 down, the parent's width less 360 and its height less 24, then 4
/// more off the height. Nothing clamps a small parent.
pub fn preview_frame(parent: (i32, i32)) -> PreviewFrame {
    PreviewFrame {
        x: PREVIEW_LEFT,
        y: PREVIEW_TOP,
        width: parent.0 - PREVIEW_WIDTH_LEFT,
        height: parent.1 - PREVIEW_TOP - PREVIEW_HEIGHT_TRIM,
    }
}

/// The equipment page's three panels and its preview, in the parent form's
/// units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationFormLayout {
    pub preview: PreviewFrame,
    pub apparel: PanelPlace,
    pub weapons: PanelPlace,
    pub skills: PanelPlace,
}

/// Lays out the equipment page over the creator form.
///
/// The preview takes the frame's top left; the apparel panel sits at 4, 24
/// and the weapons panel against the right edge less 176 (both 172 wide and
/// 620 or 568 tall); the skills panel is 216 wide and 516 or 660 tall, 12
/// in from the right. `tall` is the window aspect flag the panels read; the
/// skills height follows the screen mode (recruiting 0x204, otherwise
/// 0x294). The two side panels share their order with the original's
/// constructors; nothing clamps a parent narrower than the panels.
pub fn creation_form_layout(
    parent: (i32, i32),
    screen_mode: i32,
    tall: bool,
) -> CreationFormLayout {
    CreationFormLayout {
        preview: preview_frame(parent),
        apparel: apparel_panel_place(tall),
        weapons: weapon_panel_place(parent.0, tall),
        skills: skill_panel_place(parent.0, screen_mode),
    }
}

/// What the creation form builds in a form mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationLayout {
    /// The layout table's element count.
    pub elements: usize,
    /// The two editor sub-forms (both report changes to the form; the
    /// second sits at the parent's width less 176, 32 down).
    pub editors: bool,
}

/// The layout of the creation form in `form_mode`: mode 12 takes the
/// ten-element table and no editors, every other mode the seven-element
/// table and both editors.
pub fn creation_layout(form_mode: i32) -> CreationLayout {
    if form_mode == TEN_ELEMENT_MODE {
        CreationLayout {
            elements: 10,
            editors: false,
        }
    } else {
        CreationLayout {
            elements: 7,
            editors: true,
        }
    }
}

/// The words the creation form's constructor writes into the ten-element
/// table before building it (form mode 12 only; their roles are not
/// traced).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationTableWords {
    /// 0 in game mode 4, else 0x110.
    pub first: u32,
    /// 0x10 in game mode 4, else 0x20.
    pub second: u32,
    /// Game mode 4 also points one record's callback at the shared
    /// handler, after setting that handler up.
    pub handler: bool,
}

/// The ten-element table's words for `game_mode`, in form mode 12; no
/// words in any other form mode.
pub fn creation_table_words(form_mode: i32, game_mode: i32) -> Option<CreationTableWords> {
    if form_mode != TEN_ELEMENT_MODE {
        return None;
    }
    Some(if game_mode == 4 {
        CreationTableWords {
            first: 0,
            second: 0x10,
            handler: true,
        }
    } else {
        CreationTableWords {
            first: 0x110,
            second: 0x20,
            handler: false,
        }
    })
}

/// The creation form's field the constructor sets to 50 (its role is not
/// traced).
pub const CREATION_FORM_FIFTY: i32 = 0x32;
/// Where the body editor sits in the creation form.
pub const BODY_EDITOR_AT: [i32; 2] = [4, 0x3c];
/// The hair editor sits at the parent's width less this, [`HAIR_EDITOR_TOP`]
/// down, with style bit [`HAIR_EDITOR_STYLE`].
pub const HAIR_EDITOR_RIGHT: i32 = 0xb0;
pub const HAIR_EDITOR_TOP: i32 = 0x20;
pub const HAIR_EDITOR_STYLE: u32 = 0x2_0000;

/// The list a drop onto the creation preview goes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropList {
    /// The mode's own item list, when the form has one.
    Mode,
    /// The apparel list.
    Apparel,
    /// The weapon list.
    Weapons,
    /// Game mode `0x1010`: the drop is passed to its hook, when set.
    Hook,
}

/// Where a dropped item goes, and whether the list then places the new
/// entry (a random spot in the list's area, nudged clear of the others).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EquipDrop {
    pub list: DropList,
    pub place: bool,
}

/// The game mode whose drops all go to the apparel list, and the one whose
/// drops go to its hook.
pub const APPAREL_GAME_MODE: i32 = 8;
pub const HOOK_GAME_MODE: i32 = 0x1010;
/// The item kinds the routing asks the dropped item's record for, in
/// order.
pub const APPAREL_KIND: u32 = 0x1000;
pub const WEAPON_KIND: u32 = 0x4000;
pub const OTHER_KIND: u32 = 0x8000;

/// Routes an `item` dropped on the creation preview. Item 8 is ignored.
/// With a mode list the drop goes there; in game mode 8 to the apparel
/// list; in game mode `0x1010` to its hook; otherwise the item's record
/// (`kinds`: `None` when it has none) is asked for the apparel kind, then
/// the weapon kind, then `0x8000`, which also goes to the weapon list but
/// without the placing step. Each list placing happens only when the item
/// was added.
pub fn equip_drop(
    item: u32,
    game_mode: i32,
    mode_list: bool,
    kinds: Option<&dyn Fn(u32) -> bool>,
) -> Option<EquipDrop> {
    let to = |list, place| Some(EquipDrop { list, place });
    if item == 8 {
        return None;
    }
    if mode_list {
        return to(DropList::Mode, true);
    }
    match game_mode {
        APPAREL_GAME_MODE => to(DropList::Apparel, true),
        HOOK_GAME_MODE => to(DropList::Hook, false),
        _ => {
            let has = kinds?;
            if has(APPAREL_KIND) {
                to(DropList::Apparel, true)
            } else if has(WEAPON_KIND) {
                to(DropList::Weapons, true)
            } else if has(OTHER_KIND) {
                to(DropList::Weapons, false)
            } else {
                None
            }
        }
    }
}

// ---------------------------------------------------------------------
// The preview: its icon list, material and dummy head
// ---------------------------------------------------------------------

/// The creator preview's icon list.
///
/// Every preview item is an icon the item view shows. Adding one grows the
/// list by one; the selection only moves when the entry the pointer was on
/// was the list's last one.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PreviewIcons {
    /// The icons, oldest first.
    pub icons: Vec<i32>,
    /// The entry the hover rule last selected.
    pub selected: Option<usize>,
}

/// Adds an icon to the preview list and reports whether the selection
/// follows it.
///
/// `hovered` is the zero-extended word the pointer was over; the original
/// selects the new last entry only when the hovered index plus one equals
/// the old count. The list's own refresh runs after the push.
pub fn add_preview_icon(list: &mut PreviewIcons, icon: i32, hovered: Option<i32>) -> PreviewPush {
    let pushed = push_preview(list.icons.len() as i32, hovered);
    list.icons.push(icon);
    if let Some(selected) = pushed.selected {
        list.selected = usize::try_from(selected).ok();
    }
    pushed
}

/// Releases the preview's icon renderer. The original's release ignores a
/// missing object, so this only reports whether one was there.
pub fn release_preview_icon(present: bool) -> bool {
    present
}

/// Applies the preview's material to every item of its list, in order.
///
/// The material is only applied when the preview's generator object exists;
/// then each listed item, from the first through the list's last, is given
/// the same material, and the applied items are returned in that order. An
/// empty list applies nothing.
pub fn apply_preview_material(present: bool, items: &[i32]) -> Vec<i32> {
    if present { items.to_vec() } else { Vec::new() }
}

/// Releases a preview item's material.
///
/// The material manager is called only when a material is set, and it is
/// handed the field's address, so the generator slot is cleared as well as
/// released.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreviewMaterialRelease {
    /// The material manager ran.
    pub released: bool,
    /// The generator slot was cleared.
    pub cleared: bool,
}

pub fn release_preview_material(present: bool) -> PreviewMaterialRelease {
    PreviewMaterialRelease {
        released: present,
        cleared: present,
    }
}

/// The model the dummy-head preview renders.
pub const DUMMY_HEAD_MODEL: &str = "dummyhead.rfc";
/// The generation options word the dummy-head preview passes: the bytes
/// `40 a0 a0 00` read as a little-endian word.
pub const DUMMY_HEAD_OPTIONS: i32 = 0x00a0_a040;
/// The preview's clear flag: the byte beside its generation object, set to
/// zero when the preview is built.
pub const DUMMY_HEAD_CLEAR_FLAG: u8 = 0;
/// The reference the dummy-head preview adds to its generation object.
pub const DUMMY_HEAD_REFERENCES: i32 = 1;

/// The callbacks the dummy-head preview installs on its icon renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DummyHeadHook {
    /// Applies the preview's material to every item.
    Material,
    /// Adds an icon to the list.
    Icon,
    /// Releases the material.
    Release,
}

/// What building the dummy-head preview sets up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DummyHeadPreview {
    /// The model's pack path.
    pub model: &'static str,
    pub options: i32,
    /// The icon size, taken from the icon view before the renderer is made.
    pub icon_size: i32,
    /// The entry height, two above the icon size.
    pub entry_height: i32,
    pub clear_flag: u8,
    pub references: i32,
    /// The icon renderer's callbacks in the order the original stores them.
    pub hooks: [DummyHeadHook; 3],
    /// The icon list was cleared first.
    pub reset: bool,
}

/// Builds the dummy-head preview for an icon size.
///
/// The original first releases the screen's previous icon renderer, clears
/// the icon list, reads the icon size from the icon view (storing it and
/// two more as the entry's size and height), creates the renderer, loads
/// `dummyhead.rfc`, makes the generation object from the options word,
/// adds one reference to it - which stops it shrinking with the record -
/// and installs the material, icon and release hooks. The model name is a
/// pack path; the loader is the asset pipeline's.
pub fn dummy_head_preview(icon_size: i32) -> DummyHeadPreview {
    DummyHeadPreview {
        model: DUMMY_HEAD_MODEL,
        options: DUMMY_HEAD_OPTIONS,
        icon_size,
        entry_height: icon_size + 2,
        clear_flag: DUMMY_HEAD_CLEAR_FLAG,
        references: DUMMY_HEAD_REFERENCES,
        hooks: [
            DummyHeadHook::Material,
            DummyHeadHook::Icon,
            DummyHeadHook::Release,
        ],
        reset: true,
    }
}

/// What refreshing the character preview calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterPreviewRefresh {
    /// The character's parts and its mesh were rebuilt.
    pub rebuilt: bool,
    /// The screen's label and panel texts were refreshed.
    pub labels: bool,
    /// The texture cache ran a load pass.
    pub textures: bool,
    /// The procedural material set was refreshed.
    pub materials: bool,
}

/// Refreshes the character preview after the record changed.
///
/// The original always makes the same four calls: the character generator's
/// rebuild, the game GUI's panel refresh, the texture cache's pass and the
/// procedural materials' pass.
pub fn refresh_character_preview() -> CharacterPreviewRefresh {
    CharacterPreviewRefresh {
        rebuilt: true,
        labels: true,
        textures: true,
        materials: true,
    }
}

/// What binding a panel to the creator's record does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationPanelBinding {
    /// The panel holds the character.
    pub character: bool,
    /// The panel holds the figure, from the character's figure field.
    pub figure: bool,
    /// The panel holds the record, from the character's record field.
    pub record: bool,
    /// The item list was pointed at the character's item list.
    pub item_list: bool,
    /// A second panel was pointed at the GUI's texture source.
    pub second_panel: bool,
}

/// Binds item panels to a character.
///
/// The character and figure are always stored, and the record the figure
/// shows. An item list, when present, takes the character's own item list;
/// a second panel, when present, takes the GUI's texture source. The
/// original writes through null checks, so a missing panel changes nothing.
pub fn bind_creation_panels(has_item_list: bool, has_second_panel: bool) -> CreationPanelBinding {
    CreationPanelBinding {
        character: true,
        figure: true,
        record: true,
        item_list: has_item_list,
        second_panel: has_second_panel,
    }
}

// ---------------------------------------------------------------------
// The power tree the creator opens, and its dialogs
// ---------------------------------------------------------------------

/// A callback the creator installs on the power tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatorTreeHook {
    None,
    /// The tree's commit: save the preset, store the character's worth and
    /// start the mode.
    Commit,
    /// The tree's close: the commit runs after a deferred close.
    Close,
}

/// What opening the power tree from the creator does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatorPowerTree {
    /// The in-game GUI is hidden while the tree is up.
    pub gui_hidden: bool,
    /// The creator form is hidden.
    pub creator_hidden: bool,
    pub tree_opened: bool,
    /// The hook the tree's commit field takes.
    pub commit: CreatorTreeHook,
    /// The hook the tree's close field takes.
    pub close: CreatorTreeHook,
}

/// Opens the power tree over the creator.
///
/// The original hides the in-game GUI, hides the creator form, makes the
/// tree (its table selector 0), installs the deferred-close callback in its
/// close field and the story commit in its commit field.
pub fn open_creator_power_tree() -> CreatorPowerTree {
    CreatorPowerTree {
        gui_hidden: true,
        creator_hidden: true,
        tree_opened: true,
        commit: CreatorTreeHook::Commit,
        close: CreatorTreeHook::Close,
    }
}

/// What closing the creator's power tree does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatorTreeClosed {
    /// The tree's own close ran.
    pub tree_closed: bool,
    /// The creator form is shown again.
    pub creator_shown: bool,
    /// The Steam input action set is cleared.
    pub input_cleared: bool,
}

/// Closes the power tree and brings the creator back.
pub fn close_creator_power_tree() -> CreatorTreeClosed {
    CreatorTreeClosed {
        tree_closed: true,
        creator_shown: true,
        input_cleared: true,
    }
}

/// The deferred reopen the tree's close callback schedules.
///
/// The close callback does not close the tree itself: it asks the game to
/// close it on the next frame with no arguments, so the commit the close
/// raised can run first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeReopen {
    pub deferred: bool,
    pub closes_tree: bool,
    /// The deferred call's first argument.
    pub arg: i32,
}

/// Schedules the power tree's close after a commit from the creator.
pub fn reopen_creator_power_tree() -> TreeReopen {
    TreeReopen {
        deferred: true,
        closes_tree: true,
        arg: 0,
    }
}

/// What the name warning's callback does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NameWarningClose {
    /// The creator form closes with code 3 (the warning's own answer).
    pub closed: bool,
    /// The close went through the Steam input layer rather than the form.
    pub via_steam_input: bool,
}

/// Closes the "Please name your character" warning.
///
/// The message box's callback closes the creator screen with code 3. With a
/// controller the close is the Steam input layer's, otherwise the form's;
/// both reach the same close.
pub fn close_name_warning(steam_input: bool) -> NameWarningClose {
    NameWarningClose {
        closed: true,
        via_steam_input: steam_input,
    }
}

/// What the reset-to-portal dialog does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResetPortalDialog {
    pub created: bool,
    /// The creator form is stored as the dialog's owner.
    pub owner_bound: bool,
    pub shown: bool,
    /// The two centre calls, both given -1 (already measured).
    pub centred: [i32; 2],
}

/// Opens the "reset to portal" dialog over the continue screen.
///
/// The original makes the dialog from its form table, stores the creator
/// form as its owner, shows it and centres it twice with -1, which keeps the
/// form's measured size.
pub fn reset_to_portal_dialog() -> ResetPortalDialog {
    ResetPortalDialog {
        created: true,
        owner_bound: true,
        shown: true,
        centred: [-1, -1],
    }
}

/// What the delete confirmation dialog shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteConfirmDialog {
    pub text: String,
    pub created: bool,
    pub owner_bound: bool,
    pub shown: bool,
    pub centred: [i32; 2],
}

/// Opens the confirmation that deletes the chosen save.
///
/// The dialog's own question is kept and the character's name and a `?`
/// are appended; the name is read from the record in fifteen bytes. The
/// dialog then takes the creator as its owner, is shown and centred like
/// the reset-to-portal one.
pub fn delete_confirm_dialog(question: &str, name: &str) -> DeleteConfirmDialog {
    let name: String = name.chars().take(15).collect();
    DeleteConfirmDialog {
        text: delete_confirm_text(question, &name),
        created: true,
        owner_bound: true,
        shown: true,
        centred: [-1, -1],
    }
}

// ---------------------------------------------------------------------
// The continue screen's list
// ---------------------------------------------------------------------

/// What the continue screen's character view needs.
#[derive(Debug, Clone, Copy)]
pub struct ContinueViewInput<'a> {
    pub screen_mode: i32,
    pub game_mode: i32,
    pub name: &'a str,
    /// The model loader answered (or no loading was needed).
    pub model_available: bool,
    /// The original's asynchronous model-loading hook exists.
    pub loader_hook: bool,
    /// The figure's item count.
    pub item_count: i32,
    /// The figure holds the item at its extra slot.
    pub extra_item: bool,
    /// The figure's first listed item's database record carries flag
    /// `0x100`; only then is that item refreshed on each pass.
    pub listed_flagged: bool,
    /// The preview panel exists.
    pub panel: bool,
    /// The second panel exists.
    pub second_panel: bool,
    /// The item list exists.
    pub item_list: bool,
    /// The record's flag word (bit 0 female, bits 20 through 23 the face
    /// variant).
    pub record_flags: i32,
}

/// What building the continue screen's character view did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinueView {
    /// The view control was cleared before the model was set.
    pub cleared_view: bool,
    /// The asynchronous loader reported nothing at least once, so the
    /// original slept a frame and asked again.
    pub waited: bool,
    pub model: bool,
    /// The view control took the model (empty or not).
    pub view_target: bool,
    /// The item list was attached to the record's carried list.
    pub item_list_attached: bool,
    /// The details box's name: the record's name cut to fifteen bytes.
    pub name: String,
    /// The header, when the screen is a continue page or versus.
    pub title: Option<&'static str>,
    /// The record was pushed into the figure (continue mode).
    pub record_applied: bool,
    /// Passes over the figure's items in continue mode.
    pub item_passes: usize,
    /// Of those passes, the ones that refreshed the first listed item.
    pub listed_passes: usize,
    /// Of those passes, the ones that also refreshed the extra item.
    pub extra_passes: usize,
    pub panels_bound: usize,
    /// The second panel's look was refreshed (it exists).
    pub look_refreshed: bool,
    /// Otherwise the hair button's state was written.
    pub hair_button: Option<HairButton>,
    /// The first panel's target received the figure and the GUI's textures.
    pub panel_targets: usize,
    pub panels_shown: usize,
}

/// Builds the continue screen's character view from a character.
///
/// The view control is cleared, the character is stored and the model is
/// loaded: with the asynchronous hook the original polls it, sleeping a
/// frame between empty answers. The loaded model is shown and an existing
/// item list is attached to the record's carried items. Continue mode
/// labels the header CONTINUE and applies the character to the figure, then
/// walks the figure's item count: on every pass the first listed item is
/// refreshed when its database record carries flag `0x100`, and the extra
/// item when it exists. Other modes put the
/// record's name (fifteen bytes) into the details box. Versus labels the
/// header VERSUS after that. When the preview panel exists, both panels are
/// bound to the character; the second panel's look is refreshed, or the
/// hair button's state is written when it does not exist, and the first
/// panel's targets are set to the figure and the GUI's textures before both
/// are shown.
///
/// The item passes are modelled as counts: the original's loop re-reads the
/// list's first entry on every pass (the instructions carry no index), so
/// the same item is refreshed each time.
pub fn continue_view(input: ContinueViewInput<'_>) -> ContinueView {
    let waited = input.loader_hook && !input.model_available;
    let mut item_passes = 0;
    let mut listed_passes = 0;
    let mut extra_passes = 0;
    let mut name = String::new();
    let mut title = None;
    if input.screen_mode == 12 {
        title = Some(exa_ui::creation::CONTINUE);
        if input.item_count > 0 {
            item_passes = input.item_count as usize;
            if input.listed_flagged {
                listed_passes = item_passes;
            }
            if input.extra_item {
                extra_passes = item_passes;
            }
        }
    } else {
        name = input.name.chars().take(15).collect();
    }
    if input.game_mode == 0x1010 {
        title = Some(exa_ui::creation::VERSUS);
    }
    let mut panels_bound = 0;
    let mut look_refreshed = false;
    let mut hair_state = None;
    let mut panel_targets = 0;
    let mut panels_shown = 0;
    if input.panel {
        panels_bound = 2;
        if input.second_panel {
            look_refreshed = true;
            panels_shown = 2;
        } else {
            hair_state = Some(hair_button(
                input.record_flags,
                input.record_flags & 0x00f0_0000,
            ));
            panels_shown = 1;
        }
        panel_targets = 2;
    }
    ContinueView {
        cleared_view: true,
        waited,
        model: input.model_available,
        view_target: true,
        item_list_attached: input.item_list,
        name,
        title,
        record_applied: input.screen_mode == 12,
        item_passes,
        listed_passes,
        extra_passes,
        panels_bound,
        look_refreshed,
        hair_button: hair_state,
        panel_targets,
        panels_shown,
    }
}

/// What opening the continue page does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinuePageOpen<'a> {
    pub gui_hidden: bool,
    pub view: ContinueView,
    pub button: CreationContinue,
    pub reset_dialog: bool,
    /// The label and name box the page fills in ([`continue_texts`]).
    pub texts: ContinueTexts<'a>,
    /// The view's left edge and the panels ([`continue_place`]).
    pub place: ContinuePlace,
}

/// The save manager's list a continue page opens on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContinueSaveList<'a> {
    /// The entries' display strings; their count is the list's.
    pub labels: &'a [&'a str],
    /// The entries' names, by row.
    pub names: &'a [&'a str],
}

/// Opens the continue page: hides the in-game GUI, builds the character
/// view, lays out the save or checkpoint button, fills the label and name
/// box from the save list and, when the save manager reports a reset to the
/// portal is needed, opens that dialog.
pub fn open_continue_page<'a>(
    input: ContinueViewInput<'_>,
    saves: ContinueSaveList<'a>,
    kind: SaveKind,
    availability: SaveAvailability,
    reset_to_portal: bool,
    unit: i32,
    tall: bool,
) -> ContinuePageOpen<'a> {
    ContinuePageOpen {
        gui_hidden: true,
        view: continue_view(input),
        button: creation_continue_button(kind.code(), availability.0 as i32),
        reset_dialog: reset_to_portal,
        texts: continue_texts(saves.labels, saves.names),
        place: continue_place(unit, tall),
    }
}

/// Re-selects the save the continue screen already has.
///
/// This is the reset-to-portal dialog's confirmation: after the save
/// manager resets the chosen character (a routine outside this unit) the
/// selection re-runs with the stored index and the dialog closes [guess:
/// the dialog is inferred from the owner field it reads, which only the
/// reset-to-portal and delete dialogs set].
pub fn reselect_save(index: i32) -> i32 {
    index
}

/// What stepping to another save writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SaveStep {
    pub index: i32,
    /// Stepping always offers the checkpoint first.
    pub kind: SaveKind,
}

/// Steps to the previous save, if any.
pub fn previous_save_step(index: i32) -> Option<SaveStep> {
    previous_save(index).map(|index| SaveStep {
        index,
        kind: SaveKind::Checkpoint,
    })
}

/// Steps to the next save, if any.
pub fn next_save_step(index: i32, count: i32) -> Option<SaveStep> {
    next_save(index, count).map(|index| SaveStep {
        index,
        kind: SaveKind::Checkpoint,
    })
}

/// What picking a save from the list writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SavePick {
    pub index: i32,
    pub kind: SaveKind,
}

/// Picks a one-based list row; it always offers the checkpoint first, and
/// picking the current row changes nothing.
pub fn pick_save_from_list(selected: i32, current: i32) -> Option<SavePick> {
    select_save_from_list(selected, current).map(|index| SavePick {
        index,
        kind: SaveKind::Checkpoint,
    })
}

/// What deleting the selected save does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteSelectedSave {
    /// The slot selected after the delete.
    pub index: i32,
    /// Nothing is left: the original leaves for the menu.
    pub closed: bool,
    /// A remaining slot was selected again.
    pub reselected: bool,
}

/// Deletes the selected save, clamps the selection to the last remaining
/// slot and selects it again; with no slot left the screen returns to the
/// menu. `count` is the save count after the delete.
pub fn delete_selected_save(index: i32, count: i32) -> DeleteSelectedSave {
    let index = index.min(count - 1);
    if index < 0 {
        DeleteSelectedSave {
            index,
            closed: true,
            reselected: false,
        }
    } else {
        DeleteSelectedSave {
            index,
            closed: false,
            reselected: true,
        }
    }
}

// ---------------------------------------------------------------------
// The creator's icon sheet
// ---------------------------------------------------------------------

/// The creator's icon sheet in the resource packs.
pub const CREATION_ICON_SHEET: &str = "charcreate.rfi";
/// The icon manager's word the sheet sets.
pub const CREATION_ICON_WORD: u32 = 0x203;
/// The icon manager's flags the sheet sets.
pub const CREATION_ICON_FLAGS: u32 = 0x0100_2008;
/// The sheet's height: four [`CREATION_ICON_SIZE`] cells stacked.
pub const CREATION_ICON_HEIGHT: i32 = 4 * CREATION_ICON_SIZE;
/// The flags the cell image is made with (the icon manager's own).
pub const CREATION_ICON_IMAGE_FLAGS: u32 = 0x2000_0000;
/// The mip level the Kaiser pass leaves each cell at: the whole 96 by 96
/// image is filtered each time, then cut from the sheet.
pub const CREATION_ICON_MIP: u32 = 0;

/// What building the creator's icon sheet did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationIconSheet {
    /// The manager existed already and was reset rather than made.
    pub reused: bool,
    pub word: u32,
    pub flags: u32,
    /// The icon size the manager is given, eight GUI units
    /// ([`creation_icon_size`]); the dummy-head preview reads it back.
    pub icon_size: i32,
    /// The sheet loaded.
    pub loaded: bool,
    /// The 96-pixel cells added, in the order the original cuts them, and
    /// the band each is cut from (288, 192, 96 and 0).
    pub cells: usize,
    pub bands: [i32; 4],
    /// The cell image's size and flags, and the mip level kept.
    pub cell_size: i32,
    pub image_flags: u32,
    pub mip: u32,
    /// The manager's reset call ran (it existed).
    pub reset: bool,
}

/// Builds or rebinds the creator's icon sheet.
///
/// A previous reload hook, when one exists, runs first (the chain
/// [`ensure_creation_icons`] installs). An existing manager is reset;
/// otherwise one is made with the 0x203 word and the `0x0100_2008` flags.
/// Its icon size is set to eight GUI units (32 at unit 4) either way. A
/// missing sheet adds no cells. A present one is decoded into a 96 by 96
/// image with the `0x2000_0000` flags and the sheet's own format, filtered
/// by the Kaiser pass and registered as four icons, cut at 288, 192, 96 and
/// 0 from the 96 by 384 sheet, then the manager finishes.
pub fn build_creation_icons(unit: i32, existing: bool, load_ok: bool) -> CreationIconSheet {
    CreationIconSheet {
        reused: existing,
        word: CREATION_ICON_WORD,
        flags: CREATION_ICON_FLAGS,
        icon_size: creation_icon_size(unit),
        loaded: load_ok,
        cells: if load_ok {
            CREATION_ICON_OFFSETS.len()
        } else {
            0
        },
        bands: if load_ok {
            CREATION_ICON_OFFSETS
        } else {
            [0; 4]
        },
        cell_size: CREATION_ICON_SIZE,
        image_flags: CREATION_ICON_IMAGE_FLAGS,
        mip: CREATION_ICON_MIP,
        reset: existing,
    }
}

// ---------------------------------------------------------------------
// Opening the creator form
// ---------------------------------------------------------------------

/// What opening the creator form does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatorFormOpen {
    /// The GUI's cursor word is set to -1 and the last task is removed.
    pub cursor_reset: bool,
    pub gui_refreshed: bool,
    pub record_bound: bool,
    pub form_shown: bool,
    pub centred: bool,
    /// The preview viewport is shown.
    pub viewport_shown: bool,
    /// The continue screen's first choice, in continue mode.
    pub continue_kind: Option<SaveKind>,
    pub reset_dialog: bool,
    /// The action set the screen asks for: three.
    pub input_actions: i32,
}

/// Opens the creator form and binds the record it edits.
///
/// The cursor word is cleared, the task's last entry is removed and the GUI
/// is refreshed. The form is made when absent, bound to the record, shown
/// and centred, and the preview viewport is shown. In continue mode the
/// first choice is a save when the slot offers exactly that (two), else a
/// checkpoint; the button is laid out and the reset-to-portal dialog opens
/// when the save manager asks for it. The screen then installs three Steam
/// input actions.
pub fn open_creator_form(
    screen_mode: i32,
    availability: SaveAvailability,
    reset_to_portal: bool,
) -> CreatorFormOpen {
    let opening = open_creator(screen_mode, availability, reset_to_portal);
    CreatorFormOpen {
        cursor_reset: true,
        gui_refreshed: true,
        record_bound: true,
        form_shown: true,
        centred: true,
        viewport_shown: true,
        continue_kind: opening.show_continue.then_some(opening.continue_kind),
        reset_dialog: opening.reset_to_portal,
        input_actions: 3,
    }
}

/// What showing the creator form for a record did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatorShow {
    /// The form was made because none existed.
    pub created: bool,
    /// The preview viewport was stored as the screen's view.
    pub viewport: bool,
    /// The view's word stored beside the viewport (0x1002).
    pub view_word: i32,
    /// The record was stored.
    pub record: bool,
    /// The caller's show hook ran.
    pub hook: bool,
}

/// Shows the creator on a record.
///
/// The form is made on first use, the viewport taken from its view and the
/// record stored. The screen's word beside the view is set to `0x1002`,
/// and a caller hook, when installed, is given the record.
pub fn show_creator_here(existing: bool, hook: bool) -> CreatorShow {
    CreatorShow {
        created: !existing,
        viewport: true,
        view_word: 0x1002,
        record: true,
        hook,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_formats::characters::RANGED_COMBAT;

    #[test]
    fn a_character_record_moves_its_gear() {
        let mut source = CreationGear {
            hands: [
                GearSlot { item: 1, flags: 2 },
                GearSlot::new(3),
                GearSlot::new(4),
                GearSlot::new(5),
            ],
            worn: vec![GearSlot::new(7), GearSlot::new(8)],
            carried: vec![GearSlot::new(10)],
        };
        let moved = move_creation_gear(&mut source);
        assert_eq!(
            moved,
            CreationGear {
                hands: [
                    GearSlot { item: 1, flags: 2 },
                    GearSlot::new(3),
                    GearSlot::new(4),
                    GearSlot::new(5),
                ],
                worn: vec![GearSlot::new(7), GearSlot::new(8)],
                carried: vec![GearSlot::new(10)],
            }
        );
        assert_eq!(
            source,
            CreationGear {
                hands: [
                    GearSlot { item: 1, flags: 2 },
                    GearSlot::new(3),
                    GearSlot::new(4),
                    GearSlot::new(5),
                ],
                worn: Vec::new(),
                carried: Vec::new(),
            }
        );
    }

    #[test]
    fn the_creator_writes_the_spawned_record() {
        let mut record = CharacterRecord::default();
        let gear = CreationGear {
            hands: [
                GearSlot::new(1),
                GearSlot {
                    item: 2,
                    flags: 0x0100_0000,
                },
                GearSlot::new(3),
                GearSlot::new(4),
            ],
            worn: vec![GearSlot::new(8)],
            carried: vec![GearSlot { item: 12, flags: 7 }],
        };
        let skills = starting_skills();
        write_created_record(&mut record, "Mira", &gear, &skills, true);
        assert_eq!(record.name, "Mira");
        assert_eq!(record.hands.map(|slot| slot.id), [1, 2, 3, 4]);
        assert_eq!(record.hands[1], ItemRef::new(2, 0x0100_0000));
        assert_eq!(record.worn, vec![ItemRef::new(8, 0)]);
        assert_eq!(
            record.inventory,
            vec![Carried {
                item: 12,
                flags: 7,
                extra: 0
            }]
        );
        assert_eq!(record.skills, skills);
        assert_eq!(record.pools, Pools::FULL);
        assert_eq!(record.seed, 0xffff);

        let mut kept = CharacterRecord {
            pools: Pools {
                yellow: 1,
                red: 2,
                focus: 3,
            },
            seed: 7,
            ..CharacterRecord::default()
        };
        write_created_record(&mut kept, "Mira", &gear, &[Skill::default(); SKILLS], false);
        assert_eq!(
            kept.pools,
            Pools {
                yellow: 1,
                red: 2,
                focus: 3
            }
        );
        assert_eq!(kept.seed, 7);
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
        assert_eq!(lettered.letter, Some(b'B'));

        let first = update_creation_look(false, 1);
        assert_eq!(first.letter, Some(b'A'));

        let plain = update_creation_look(false, 0);
        assert_eq!(plain.values, [0; 4]);
        assert_eq!(plain.letter, None);
    }

    #[test]
    fn the_creation_appearance_scales_to_bytes_and_words() {
        assert_eq!(appearance_bytes([0.0, 0.5, 1.0]), [0, 128, 255]);
        assert_eq!(appearance_words([0.0, 0.5]), [0, 32768]);
        assert_eq!(appearance_words([1.0, 0.0]), [65535, 0]);
        // CVTSS2SI's indefinite makes the low byte zero, not 0xff.
        assert_eq!(
            appearance_bytes([f32::NAN, f32::INFINITY, 1.5]),
            [0, 0, 126]
        );
        assert_eq!(appearance_words([f32::NAN, f32::NEG_INFINITY]), [0, 0]);
        // The conversion is 64-bit: 300000 · 65535 keeps its low word,
        // where a 32-bit conversion would give the indefinite's zero.
        assert_eq!(appearance_words([300_000.0, -0.0]), [0x7000, 0]);
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
    fn the_edit_page_pushes_the_record_into_its_controls() {
        let src = PageSource {
            physique: [0.25, 0.75],
            height: 0.5,
            age: 0.25,
            skin: [0x8000, 0xffff],
            hair_colour: [255, 51, 0],
            slider: 0.3,
            number: 3,
            voice: 2,
            pitch: 0.985,
            word: 0x0011_0001,
            hair_picker: true,
        };
        let page = edit_page(&src);
        assert_eq!(page.physique_point, [0.25, 0.75]);
        assert_eq!(page.skin_point[1], 1.0);
        assert!((page.skin_point[0] - 0.5000076).abs() < 1e-6);
        assert!(!page.male_on && page.female_on);
        assert_eq!(page.height_slider, 0.25);
        assert_eq!((page.age_slider, page.slider, page.number), (0.25, 0.3, 3));
        assert_eq!(page.voice_letter, b'C');
        assert!((page.pitch_slider - 0.5).abs() < 1e-6);
        assert_eq!(page.hair_colour, Some([1.0, 0.2, 0.0]));
        assert!(page.hair_on && !page.hair_off_on);
        // Race band 0 with bit 0: the human female label.
        assert_eq!(page.caption, "VHF");
        // Bits 20-23 hold 1: variant 0, the letter A.
        assert_eq!(page.variant_letter, Some(b'A'));
        // A zero variant field shows the dash; a male without hair flips
        // the buttons.
        let plain = edit_page(&PageSource {
            word: 0,
            ..src.clone()
        });
        assert_eq!(plain.variant_letter, None);
        assert!(plain.male_on && !plain.female_on && plain.hair_off_on);
        assert_eq!(plain.caption, "VHM");
        // The body page has no hair picker: no colour, no hair buttons; a
        // band without a label leaves the bare `V`.
        let body = edit_page(&PageSource {
            hair_picker: false,
            word: 0x0001_0500,
            ..src.clone()
        });
        assert_eq!(body.hair_colour, None);
        assert!(!body.hair_on && !body.hair_off_on);
        assert_eq!(body.caption, "V");
        // The height slider runs 1 (tallest, height -1) to 0 (shortest).
        let tall = edit_page(&PageSource {
            height: -1.0,
            word: 0,
            ..src
        });
        assert_eq!(tall.height_slider, 1.0);
    }

    #[test]
    fn the_edit_page_destroy_removes_its_listener_first() {
        assert_eq!(
            destroy_edit_page(true),
            EditPageDestroy {
                hook_removed: true,
                base: true
            }
        );
        assert!(!destroy_edit_page(false).hook_removed);
    }

    #[test]
    fn the_pickers_write_the_record_with_half_even_rounding() {
        let mut skin = PageSource {
            physique: [0.0; 2],
            height: 0.0,
            age: 0.0,
            skin: [0; 2],
            hair_colour: [0; 3],
            slider: 0.0,
            number: 0,
            voice: 0,
            pitch: 1.0,
            word: 0,
            hair_picker: false,
        };
        // 0.5 * 65535 = 32767.5 rounds to the even 32768; 0.25 * 65535 =
        // 16383.75 to 16384.
        skin_changed(&mut skin, [0.5, 0.25]);
        assert_eq!(skin.skin, [32768, 16384]);
        // 0.5 * 255 = 127.5 rounds to the even 128.
        let mut hair = vec![9];
        hair_colour_changed(&mut hair, [0.0, 0.5, 1.0]);
        assert_eq!(hair, [0, 128, 255]);
        let mut style = 2;
        assert!(hair_style_changed(&mut style, 4));
        assert_eq!(style, 3);
        // The same style again, and a selection below 1, do nothing.
        assert!(!hair_style_changed(&mut style, 4));
        assert!(!hair_style_changed(&mut style, 0));
        assert!(!hair_style_changed(&mut style, -1));
        assert_eq!(style, 3);
        assert!(close_edit_form(true).detach);
        assert!(close_edit_form(false).base && !close_edit_form(false).detach);
    }

    #[test]
    fn the_form_constructors_keep_their_sizes() {
        let base = base_form(false);
        assert_eq!(
            (base.width, base.height, base.controls_high),
            (172, 592, 11)
        );
        assert_eq!(base.physique, [8, 64]);
        assert_eq!(base.tone, [8, 328, 160, 160]);
        assert_eq!(base_form(true).height, 540);
        assert_eq!(base_form(true).tone[3], 112);
        assert_eq!(base.caption, "VHM");
        assert_eq!((base.table_word, base.tone_style), (0x7f, 0x80));
        assert_eq!(base.atlas_controls, [7, 9, 10]);
        assert_eq!(base_form(true).table_word, 0x71);
        // The style list under a 160-wide picker at 4, 24 in a 612 form.
        let list = hair_list([4, 24, 160, 112], 612, 4);
        assert_eq!(list.rect, [4, 224, 152, 396]);
        assert_eq!((list.cell, list.entry, list.step), (48, 50, 56));
        assert_eq!(
            (list.options, list.icon_flags, list.icon_mode),
            (0x3020_0800, 0x100_e608, 0x103)
        );
        assert_eq!((list.attribute_bit, list.tint), (0x200, [0xff; 4]));
        let hair = hair_form(false);
        assert_eq!(
            (hair.height, hair.controls_high, hair.picker),
            (612, 6, [4, 24])
        );
        assert_eq!(hair_form(true).height, 548);
        let h = hair_icon_refresh(4, true);
        assert_eq!((h.cell, h.step, h.regenerate), (48, 48, true));
    }

    #[test]
    fn the_physique_change_waits_two_clock_units_between_regenerations() {
        let mut target = [0.0; 2];
        // 100 + 2 > 101: too soon.
        let soon = sync_physique(&mut target, [0.4, 0.6], 100, 101);
        assert!(soon.pending && !soon.regenerate && soon.last == 100);
        assert_eq!(target, [0.4, 0.6]);
        // 100 + 2 > 102 is false: regenerate and take the clock.
        let due = sync_physique(&mut target, [0.5, 0.5], 100, 102);
        assert!(!due.pending && due.regenerate && due.last == 102);
        assert_eq!(target, [0.5, 0.5]);
        // The sum is worked in 64 bits: no wrap near the top of u32.
        let top = sync_physique(&mut target, [0.0; 2], u32::MAX, 5);
        assert!(top.pending);
    }

    #[test]
    fn the_creation_defaults_are_fixed() {
        let defaults = creation_defaults();
        assert_eq!(defaults.count, 2);
        assert_eq!(defaults.stride, 16);
        assert_eq!(
            defaults.slots,
            [[0x101, 0x5fff, 0, 0], [0x201, 0x5fff, 0, 0]]
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
        // With both kinds available either one can switch.
        let save = creation_continue_button(2, 3);
        assert_eq!(save.label, "Continue from save");
        assert!(save.mirrored);
        assert!(save.enabled);

        let checkpoint = creation_continue_button(1, 3);
        assert_eq!(checkpoint.label, "Continue from checkpoint");
        assert!(!checkpoint.mirrored);
        assert!(checkpoint.enabled);

        // A lone save or checkpoint has nothing to switch to.
        assert!(!creation_continue_button(2, 2).enabled);
        assert!(!creation_continue_button(1, 1).enabled);
        assert!(!creation_continue_button(5, 3).enabled);
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

        // Near the top of the clock the stamp plus two does not wrap.
        assert!(copy_creation_link(-1, 0).pending);
        assert!(copy_creation_link(-3, -1).fired);

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

    #[test]
    fn the_creation_scale_applies_and_rebuilds() {
        let plain = apply_creation_scale(0.0, false, false, false);
        assert!((plain.scale - 0.9).abs() < 1e-6);
        assert!(plain.stored);
        assert!(!plain.rebuild);
        assert!(!plain.global_call);
        assert!(!plain.built);
        assert!(!plain.drew);

        let special = apply_creation_scale(1.0, true, false, false);
        assert!((special.scale - 1.07).abs() < 1e-6);
        assert!(special.rebuild);
        assert!(special.global_call);
        assert!(special.built);
        assert!(special.drew);
        assert!(special.finalized);

        let existing = apply_creation_scale(1.0, true, true, true);
        assert!(!existing.global_call);
        assert!(!existing.built);
        assert!(existing.drew);
    }

    #[test]
    fn the_creation_list_steps_both_ways() {
        let back = step_creation_list(2, false);
        assert!(back.rebuilt);
        assert_eq!(back.index, 1);

        let blocked = step_creation_list(0, false);
        assert!(!blocked.rebuilt);
        assert_eq!(blocked.index, 0);

        let forward = step_creation_list(2, true);
        assert!(forward.rebuilt);
        assert_eq!(forward.index, 3);

        let zero_forward = step_creation_list(0, true);
        assert!(zero_forward.rebuilt);
        assert_eq!(zero_forward.index, 1);
    }

    #[test]
    fn the_creation_modes_switch_and_toggle() {
        assert_eq!(switch_creation_mode(2), CreationModeAction::Special);
        assert_eq!(
            switch_creation_mode(1),
            CreationModeAction::Widget { enabled: false }
        );
        assert_eq!(
            switch_creation_mode(0),
            CreationModeAction::Widget { enabled: true }
        );

        let down = toggle_creation_mode(2, 1);
        assert!(down.changed);
        assert_eq!(down.mode, 1);
        assert!(down.refreshed);

        let up = toggle_creation_mode(1, 2);
        assert!(up.changed);
        assert_eq!(up.mode, 2);
        assert!(up.refreshed);

        let refused = toggle_creation_mode(2, 2);
        assert!(!refused.changed);
        assert_eq!(refused.mode, 2);
        assert!(!refused.refreshed);
    }

    #[test]
    fn the_creation_close_schedules_or_releases() {
        let empty = CreationCloseInput {
            active: false,
            has_weapon_panel: false,
            has_skill_panel: false,
            has_lobby_form: false,
            has_versus_skills: false,
            has_equipment_form: false,
            has_body_form: false,
            has_hair_form: false,
            mode: 0,
        };

        let inactive = close_creation_screen(CreationCloseInput {
            has_weapon_panel: true,
            has_skill_panel: true,
            has_lobby_form: true,
            has_versus_skills: true,
            has_equipment_form: true,
            has_body_form: true,
            has_hair_form: true,
            mode: 10,
            ..empty
        });
        assert_eq!(inactive.scheduled, Some(CreationCloseForm::Special));
        assert_eq!(inactive.released, [false; 7]);
        assert!(!inactive.ticked);
        assert!(inactive.cleared);

        let regular = close_creation_screen(CreationCloseInput { mode: 9, ..empty });
        assert_eq!(regular.scheduled, Some(CreationCloseForm::Regular));

        let active = close_creation_screen(CreationCloseInput {
            active: true,
            has_skill_panel: true,
            has_lobby_form: true,
            has_equipment_form: true,
            has_body_form: true,
            mode: 12,
            ..empty
        });
        assert_eq!(active.scheduled, None);
        assert_eq!(
            active.released,
            [false, true, false, false, true, false, false]
        );
        assert!(active.width_restored);
        assert!(!active.widgets_reset);
        assert!(active.ticked);
        assert!(active.strings);
        assert!(active.disabled);
        assert!(active.cleared);

        let full = close_creation_screen(CreationCloseInput {
            active: true,
            has_weapon_panel: true,
            has_skill_panel: true,
            has_lobby_form: true,
            has_versus_skills: true,
            has_equipment_form: true,
            has_body_form: true,
            has_hair_form: true,
            mode: 3,
        });
        assert_eq!(full.released, [true, false, false, true, true, true, true]);
        assert!(full.widgets_reset);
        assert!(!full.width_restored);
    }

    #[test]
    fn the_creation_option_hits_and_the_singleton_opens() {
        let hit = hit_creation_option(3, true, true);
        assert!(hit.hit);
        assert!(hit.selected);
        assert!(hit.result);

        let key8 = hit_creation_option(8, true, true);
        assert!(!key8.hit);
        assert!(!key8.selected);
        assert!(key8.result);

        let no_list = hit_creation_option(3, false, true);
        assert!(!no_list.hit);
        assert!(no_list.result);

        let miss = hit_creation_option(3, true, false);
        assert!(!miss.hit);
        assert!(miss.result);

        let first = ensure_creation_singleton(false);
        assert!(first.created);
        assert!(first.ready);

        let later = ensure_creation_singleton(true);
        assert!(!later.created);
        assert!(later.ready);
    }

    #[test]
    fn the_creation_forms_close_key_show_and_capture() {
        assert_eq!(close_creation_form(10), CreationCloseForm::Special);
        assert_eq!(close_creation_form(9), CreationCloseForm::Regular);

        let forwarded = creation_key(true, 3);
        assert_eq!(forwarded.result, 3);
        assert!(forwarded.forwarded);
        assert!(!forwarded.closed);

        let closed = creation_key(false, 3);
        assert_eq!(closed.result, 0x40);
        assert!(!closed.forwarded);
        assert!(closed.closed);

        let shown = show_creation_form(true);
        assert!(shown.prepared);
        assert!(shown.dispatched);
        assert!(shown.refreshed);
        assert!(shown.built);

        let skipped = show_creation_form(false);
        assert!(!skipped.built);
        assert!(skipped.refreshed);

        let captured = capture_creation([1, 2, 3, 4, 5, 6], 10, 4, 7, 2);
        assert_eq!(captured.words, [1, 2, 3, 4, 5, 6]);
        assert_eq!(captured.value, 13);
        assert_eq!(captured.mode, 2);
        assert!(captured.scheduled);
        assert!(captured.stored_cleared);
    }

    #[test]
    fn the_creation_screen_pages_and_selects() {
        let page = page_creation_next(5, 2);
        assert_eq!(page.mode, 1);
        assert_eq!(page.index, 3);
        assert!(page.refreshed);

        let stopped = page_creation_next(2, 2);
        assert_eq!(stopped.mode, 0);
        assert_eq!(stopped.index, 2);
        assert!(!stopped.refreshed);

        let selected = select_creation_index(4, 2);
        assert_eq!(selected.mode, 1);
        assert_eq!(selected.index, 3);
        assert!(selected.refreshed);

        let same = select_creation_index(3, 2);
        assert_eq!(same.mode, 0);
        assert_eq!(same.index, 2);
        assert!(!same.refreshed);
    }

    #[test]
    fn the_creation_session_selects_and_dispatches() {
        let table = select_creation_table(4);
        assert_eq!(table.index, 3);
        assert!(table.applied);

        let session = start_creation_session(13);
        assert_eq!(session.value, 13);
        assert!(session.scheduled);
        assert!(session.cleared);

        assert_eq!(dispatch_creation_menu(true), CreationDispatch::Alternate);
        assert_eq!(dispatch_creation_menu(false), CreationDispatch::Primary);
    }

    #[test]
    fn the_creation_screen_starts_and_applies() {
        let arena = start_creation_screen(true, false, 12, true, true);
        assert!(arena.reset);
        assert!(!arena.second_reset);
        assert!(!arena.created);
        assert!(arena.prepared);
        assert_eq!(arena.mode, 2);
        assert!(arena.refreshed);
        assert!(arena.built);
        assert!(arena.ticked);
        assert!(arena.stored);

        let story = start_creation_screen(false, true, 12, false, false);
        assert!(story.created);
        assert!(story.second_reset);
        assert_eq!(story.mode, 1);
        assert!(!story.built);

        let other = start_creation_screen(true, false, 9, true, true);
        assert_eq!(other.mode, 0);
        assert!(!other.refreshed);
        assert!(!other.built);
        assert!(!other.ticked);
        assert!(other.stored);

        let applied = apply_creation_step(true, true);
        assert!(applied.linked);
        assert!(applied.camera);
        assert!(applied.stored);
        assert!(applied.part_found);
        assert!(applied.first);
        assert!(applied.second);

        let bare = apply_creation_step(false, true);
        assert!(!bare.stored);
        assert!(!bare.part_found);
        assert!(bare.linked);
        assert!(bare.first);
    }

    #[test]
    fn the_creation_part_refreshes_and_destroys() {
        let found = refresh_creation_part(true, true);
        assert!(found.stored);
        assert!(found.part_found);

        let absent = refresh_creation_part(false, true);
        assert!(!absent.stored);
        assert!(!absent.part_found);

        let decremented = destroy_creation(1, true, 2);
        assert!(decremented.base_destroy);
        assert!(decremented.ref_decremented);
        assert!(!decremented.released);
        assert!(decremented.base);
        assert!(decremented.freed);

        let released = destroy_creation(0, true, 1);
        assert!(!released.base_destroy);
        assert!(!released.ref_decremented);
        assert!(released.released);
        assert!(released.base);
        assert!(!released.freed);

        let none = destroy_creation(0, false, 0);
        assert!(!none.ref_decremented);
        assert!(!none.released);
        assert!(none.base);
    }

    #[test]
    fn the_creation_point_copies_and_resumes() {
        assert_eq!(copy_creation_point([1, 2, 3]), [1, 2, 3]);

        let resumed = resume_creation();
        assert!(resumed.saved);
        assert!(resumed.refreshed);
        assert!(resumed.resumed);
    }

    fn skill(points: u16, techniques: [u8; 6]) -> Skill {
        Skill { points, techniques }
    }

    fn gear(hands: [u32; 4], worn: &[u32], carried: &[u32]) -> CreationGear {
        CreationGear {
            hands: hands.map(GearSlot::new),
            worn: worn.iter().copied().map(GearSlot::new).collect(),
            carried: carried.iter().copied().map(GearSlot::new).collect(),
        }
    }

    #[test]
    fn the_story_filter_keeps_braies_only() {
        let mut story = gear([1, 2, 3, 4], &[7, 8, 9], &[10]);
        story.hands[0].flags = 7;
        filter_creation_gear(&mut story, 4, 0, &[247, 249]);
        assert_eq!(
            story.hands,
            [
                GearSlot { item: 0, flags: 7 },
                GearSlot::default(),
                GearSlot::default(),
                GearSlot::default()
            ]
        );
        assert_eq!(story.worn, vec![GearSlot::new(8)]);
        assert!(story.carried.is_empty());

        let mut knight = gear([1, 2, 3, 4], &[7, 8, 9], &[10]);
        filter_creation_gear(&mut knight, 4, 1, &[247, 249]);
        assert_eq!(knight, gear([1, 2, 3, 4], &[7, 8, 9], &[10]));
    }

    #[test]
    fn the_arena_filter_keeps_offered_hands_and_braies() {
        let mut arena = gear([247, 38, 3, 4], &[7, 8, 9], &[10]);
        filter_creation_gear(&mut arena, 8, 0, &[247, 249]);
        assert_eq!(
            arena.hands,
            [
                GearSlot::new(247),
                GearSlot::default(),
                GearSlot::default(),
                GearSlot::default()
            ]
        );
        assert_eq!(arena.worn, vec![GearSlot::new(8)]);
        assert!(arena.carried.is_empty());
    }

    #[test]
    fn the_other_modes_clear_the_fist_and_the_inventory() {
        let mut arena_mode = gear([0x1f, 0x1f, 3, 4], &[7, 8], &[10]);
        filter_creation_gear(&mut arena_mode, 10, 0, &[]);
        assert_eq!(
            arena_mode.hands,
            [
                GearSlot::default(),
                GearSlot::default(),
                GearSlot::new(3),
                GearSlot::new(4)
            ]
        );
        assert_eq!(arena_mode.worn, vec![GearSlot::new(7), GearSlot::new(8)]);
        assert!(arena_mode.carried.is_empty());

        let mut other = gear([8, 9, 3, 4], &[7, 8], &[10]);
        filter_creation_gear(&mut other, 0x1010, 0, &[]);
        assert_eq!(other, gear([8, 9, 3, 4], &[7, 8], &[10]));
    }

    #[test]
    fn the_starting_skills_are_three_full_melee_skills() {
        let skills = starting_skills();
        assert_eq!(skills[CLOSE_COMBAT], skill(25000, [1, 2, 3, 4, 8, 0]));
        assert_eq!(skills[1], Skill::default());
        assert_eq!(skills[ARMOUR], skill(25000, [1, 2, 3, 4, 5, 0]));
        assert_eq!(skills[SHIELDS], skill(25000, [1, 2, 3, 4, 6, 0]));
        assert_eq!(skills[4], Skill::default());
        assert_eq!(skills[5], Skill::default());

        // Written into a used record, the other slots and every sixth
        // technique stay as they were.
        let mut used = [skill(7000, [9, 9, 9, 9, 9, 9]); SKILLS];
        apply_starting_skills(&mut used);
        assert_eq!(used[CLOSE_COMBAT], skill(25000, [1, 2, 3, 4, 8, 9]));
        assert_eq!(used[RANGED_COMBAT], skill(7000, [9, 9, 9, 9, 9, 9]));
        assert_eq!(used[ARMOUR], skill(25000, [1, 2, 3, 4, 5, 9]));
        assert_eq!(used[SHIELDS], skill(25000, [1, 2, 3, 4, 6, 9]));
        assert_eq!(used[5], skill(7000, [9, 9, 9, 9, 9, 9]));
    }

    #[test]
    fn clearing_skills_keeps_the_first_used_first_technique() {
        let mut skills = [Skill::default(); SKILLS];
        skills[1] = skill(7000, [1, 2, 3, 4, 5, 6]);
        skills[3] = skill(5000, [7, 7, 7, 7, 7, 7]);
        assert_eq!(clear_skills_from_first_used(&mut skills), Some(1));
        assert_eq!(skills[1], skill(0, [1, 0, 0, 0, 0, 0]));
        assert_eq!(skills[2], Skill::default());
        assert_eq!(skills[3], Skill::default());

        // The first used slot keeps its first technique byte and loses the
        // other five; every later slot is cleared whole, the last included.
        let mut last = [Skill::default(); SKILLS];
        last[5] = skill(25000, [1, 2, 3, 4, 5, 6]);
        assert_eq!(clear_skills_from_first_used(&mut last), Some(5));
        assert_eq!(last[5], skill(0, [1, 0, 0, 0, 0, 0]));
        assert_eq!(last[0], Skill::default());

        let mut empty = [Skill::default(); SKILLS];
        assert_eq!(clear_skills_from_first_used(&mut empty), None);
    }

    #[test]
    fn the_skill_budget_caps_slots_and_keeps_a_chosen_technique() {
        let mut skills = [Skill::default(); SKILLS];
        skills[0] = skill(25000, [1, 2, 3, 4, 5, 6]);
        skills[1] = skill(25000, [1, 2, 3, 4, 5, 6]);
        let budget = set_skill_budget(&skills, 12000, 0);
        assert_eq!(budget.skills[0], skill(12000, [1, 2, 3, 0, 0, 0]));
        assert_eq!(budget.skills[1], Skill::default());
        assert_eq!(budget.leftover, 0);
        assert_eq!(budget.training, None);

        // A slot capped at level 6 reads past its techniques into the next
        // slot: the next slot's point byte is set, so this slot keeps its
        // techniques and the next one, capped later, loses its chosen one.
        let mut wide = [Skill::default(); SKILLS];
        wide[0] = skill(40000, [1, 2, 3, 4, 5, 6]);
        wide[1] = skill(5000, [7, 8, 0, 0, 0, 0]);
        let read = set_skill_budget(&wide, 30000, 0);
        assert_eq!(read.skills[0], skill(30000, [1, 2, 3, 4, 5, 6]));
        assert_eq!(read.skills[1], Skill::default());
        let mut last = [Skill::default(); SKILLS];
        last[5] = skill(40000, [1, 2, 3, 4, 5, 6]);
        // At level 5 the slot's own sixth technique decides.
        let five = set_skill_budget(&last, 25000, 0);
        assert_eq!(five.skills[5], skill(25000, [1, 2, 3, 4, 5, 6]));
        // At level 4 the chosen fifth technique is kept and only the sixth
        // is cleared.
        let four = set_skill_budget(&last, 20000, 0);
        assert_eq!(four.skills[5], skill(20000, [1, 2, 3, 4, 5, 0]));
    }

    #[test]
    fn the_skill_budget_gives_the_leftover_to_the_training_skill() {
        let mut skills = [Skill::default(); SKILLS];
        skills[0] = skill(1000, [1, 0, 0, 0, 0, 0]);
        let budget = set_skill_budget(&skills, 30000, 0);
        assert_eq!(budget.skills[0].points, 5000);
        assert_eq!(budget.leftover, 29000);
        assert_eq!(budget.training, Some(0));

        let knight = set_skill_budget(&skills, 5000, 1);
        assert_eq!(knight.leftover, 9000);
        assert_eq!(knight.skills[0].points, 5000);

        let proctor = set_skill_budget(&[Skill::default(); SKILLS], 9999, 2);
        assert_eq!(proctor.leftover, 10000);
        assert_eq!(proctor.training, None);
    }

    #[test]
    fn the_effective_experience_weighs_levels_and_the_first_remainder() {
        let mut one = [Skill::default(); SKILLS];
        one[0] = skill(25000, [0; 6]);
        assert_eq!(effective_global_experience(&one), 37500);

        let mut two = one;
        two[1] = skill(25000, [0; 6]);
        assert_eq!(effective_global_experience(&two), 106250);

        let mut partial = [Skill::default(); SKILLS];
        partial[0] = skill(7500, [0; 6]);
        assert_eq!(effective_global_experience(&partial), 8125);

        partial[1] = skill(7500, [0; 6]);
        assert_eq!(effective_global_experience(&partial), 15000);

        assert_eq!(effective_global_experience(&[Skill::default(); SKILLS]), 0);

        // Six full words: 78 levels at weights 4 to 81, then no remainder
        // past the first slot's 535.
        let full = [skill(u16::MAX, [0; 6]); SKILLS];
        assert_eq!(
            effective_global_experience(&full),
            (5000 * (4..82).sum::<i32>() + 535 * 82) / 4
        );
    }

    #[test]
    fn the_skills_page_follows_the_mode() {
        let arena = skill_page(8, 0, 20000, 0, true, false);
        assert_eq!(arena.mode, SkillPageMode::Arena);
        assert_eq!(arena.tab, Some("SKILLS"));
        assert_eq!(arena.heading, None);
        assert_eq!(arena.value, None);
        assert!(arena.clear_skills);
        assert!(!arena.editable);
        assert_eq!(arena.page, 4);

        let arena_continue = skill_page(8, 12, 20000, 0, true, false);
        assert_eq!(arena_continue.tab, None);
        assert_eq!(arena_continue.heading, Some("Current skill progress"));
        assert!(!arena_continue.clear_skills);

        let global = skill_page(4, 0, 12000, 0, true, false);
        assert_eq!(global.mode, SkillPageMode::Global);
        assert_eq!(global.heading, Some("Global experience"));
        assert_eq!(global.value, Some(12000));
        assert!(global.apply_budget);
        assert!(global.editable);
        assert_eq!(global.page, 6);

        // 30000 experience is worth 48750; less a committed 37500 leaves
        // 11250, which buys back 10000 experience.
        let continued = skill_page(4, 12, 30000, 37500, true, false);
        assert_eq!(continued.mode, SkillPageMode::NewGlobal);
        assert_eq!(continued.heading, Some("New global experience"));
        assert_eq!(continued.value, Some(10000));
        assert_eq!(continued.worth_left, 11250);
        assert!(!continued.editable);
        assert!(!continued.apply_budget);

        // Every page hides the second preview panel first and makes the
        // skills panel with its 0x20000 flag; the panel is shown either way.
        for page in [arena, arena_continue, global, continued] {
            assert!(page.second_panel_hidden);
            assert!(page.created && page.shown);
            assert_eq!(page.panel_flags, 0x2_0000);
            assert!(!page.record_rebound);
        }
        // A second visit shows the existing panel again with its record
        // pointer rebound and no new flags; a missing second preview panel
        // is not hidden.
        let again = skill_page(4, 0, 12000, 0, false, true);
        assert!(!again.second_panel_hidden);
        assert!(!again.created && again.shown);
        assert_eq!(again.panel_flags, 0);
        assert!(again.record_rebound);
        assert_eq!(again.value, Some(12000));
        // Of the three pages, the arena's reads the character's skills, not
        // the account's, so its experience word stays empty. 20000 experience
        // is worth 27500.
        assert_eq!(arena.value, None);
        assert_eq!(arena.worth_left, 27500);
    }

    #[test]
    fn the_new_global_experience_never_goes_negative() {
        assert_eq!(
            new_global_experience(30000, 14250),
            NewGlobal {
                worth_left: 34500,
                experience: 23500
            }
        );
        assert_eq!(new_global_experience(12000, 14250).worth_left, 0);
        assert_eq!(new_global_experience(100, 14250).experience, 0);
    }

    #[test]
    fn the_background_presets_name_their_keys() {
        let unknown = background_preset(0).unwrap();
        assert_eq!(unknown.class, BackgroundClass::Unknown);
        assert_eq!(unknown.name, None);
        assert_eq!(
            unknown.keys,
            PresetKeys {
                root: 0x6431_bc02,
                character: 0x6431_cc01
            }
        );

        let knight = background_preset(1).unwrap();
        assert_eq!(knight.name, Some("knight"));
        assert_eq!(knight.keys.root, 0x6431_bc03);
        // Every preset keeps its character under the same second key.
        assert_eq!(knight.keys.character, 0x6431_cc01);
        assert_eq!(background_preset(2).unwrap().name, Some("proctor"));
        assert_eq!(background_preset(3).unwrap().name, Some("villager"));
        assert_eq!(background_preset(4), None);
    }

    #[test]
    fn the_preset_save_keys_follow_the_mode() {
        assert_eq!(preset_save_keys(4, 0).unwrap().root, 0x6431_bc02);
        assert_eq!(preset_save_keys(4, 3).unwrap().root, 0x6431_bc05);
        assert_eq!(preset_save_keys(4, 3).unwrap().character, 0x6431_cc01);
        assert_eq!(
            preset_save_keys(8, 0).unwrap(),
            PresetKeys {
                root: 0x6431_bc01,
                character: 0x6431_cc01
            }
        );
        assert_eq!(preset_save_keys(0, 0), None);
        assert_eq!(preset_save_keys(4, 9), None);
    }

    #[test]
    fn the_background_plan_resets_only_the_unknown_background() {
        let unknown = plan_background(0, false).unwrap();
        assert_eq!(unknown.source, PresetSource::NewDefault);
        assert_eq!(unknown.gear, BackgroundGear::Reset);
        assert!(unknown.creation_defaults);

        let template = plan_background(1, false).unwrap();
        assert_eq!(template.source, PresetSource::Template);
        assert_eq!(template.gear, BackgroundGear::Template);
        assert!(!template.creation_defaults);

        let saved = plan_background(2, true).unwrap();
        assert_eq!(saved.source, PresetSource::Saved);
        // The saved proctor still wears the proctor template's gear.
        assert_eq!(saved.gear, BackgroundGear::Template);
        assert!(!saved.creation_defaults);
        assert_eq!(plan_background(4, true), None);
    }

    #[test]
    fn the_free_create_plan_picks_a_preset_or_default() {
        let story = plan_free_create(4, true, true);
        assert_eq!(
            story.source,
            FreeCreateSource::Preset(PresetKeys {
                root: 0x6431_bc02,
                character: 0x6431_cc01
            })
        );
        assert!(!story.reset_skills);
        assert!(story.reset_gear);
        assert!(story.clear_race_and_powers);
        assert!(!story.marks_new_character);
        // Read into a record made for it, the preset's record is marked.
        assert!(plan_free_create(4, false, true).marks_new_character);

        assert_eq!(
            plan_free_create(8, false, true).source,
            FreeCreateSource::Preset(PresetKeys {
                root: 0x6431_bc01,
                character: 0x6431_cc01
            })
        );

        // No preset leaves no record: the screen makes a plain marked one.
        let missing = plan_free_create(8, false, false);
        assert_eq!(missing.source, FreeCreateSource::NewDefault);
        assert!(missing.marks_new_character);
        assert!(!missing.reset_skills && !missing.reset_gear);

        let kept = plan_free_create(0, true, false);
        assert_eq!(kept.source, FreeCreateSource::KeepRecord);
        assert!(kept.reset_skills);
        assert!(!kept.reset_gear);
        assert!(!kept.marks_new_character);

        let fresh = plan_free_create(0x1010, false, false);
        assert_eq!(fresh.source, FreeCreateSource::NewDefault);
        assert!(fresh.reset_skills);
        assert!(fresh.marks_new_character);
    }

    #[test]
    fn the_entry_plan_loads_the_record_each_mode_edits() {
        assert_eq!(
            plan_creation_entry(12, 4, 0, false, false, true, true),
            Err(CreationEntryError::SaveManagerMissing)
        );

        let continued = plan_creation_entry(12, 4, 0, false, true, true, true).unwrap();
        assert_eq!(continued.source, EntrySource::ContinueSave);
        assert!(!continued.filter);
        assert!(!continued.full_health);

        let invalid = plan_creation_entry(12, 4, 0, false, true, false, true).unwrap();
        assert_eq!(invalid.mode, CreationScreenMode::Free);
        assert_eq!(invalid.source, EntrySource::FreeCreate);
        assert!(invalid.filter);
        assert!(invalid.full_health);

        // Recruiting skips the gear filter but still fills the pools.
        let recruit = plan_creation_entry(10, 8, 0, false, true, true, true).unwrap();
        assert_eq!(recruit.source, EntrySource::NewDefault);
        assert!(!recruit.filter);
        assert!(recruit.full_health);

        let story = plan_creation_entry(4, 4, 2, false, true, true, false).unwrap();
        assert_eq!(story.source, EntrySource::Background(2));
        assert!(story.filter);
        assert!(story.clear_experience);

        let existing = plan_creation_entry(0, 8, 0, true, true, true, false).unwrap();
        assert_eq!(existing.source, EntrySource::Existing);
        assert!(!existing.clear_experience);

        // The free story creator with no saved preset makes a plain record.
        let unsaved = plan_creation_entry(0, 4, 0, false, true, true, false).unwrap();
        assert_eq!(unsaved.source, EntrySource::NewDefault);
        assert!(!unsaved.filter);
        assert!(unsaved.full_health);
        let free = plan_creation_entry(0, 0x1010, 0, false, true, true, false).unwrap();
        assert_eq!(free.source, EntrySource::FreeCreate);
        assert!(free.filter);
        assert!(free.ensure_underwear);
        // The free creator makes the missing record and marks it.
        assert!(free.marks_new_character);
        assert!(unsaved.marks_new_character);
        assert!(!existing.marks_new_character);

        // Continue with no record marks a new one; the other entries that
        // make a record mark it too.
        let continued_blank = plan_creation_entry(12, 4, 0, false, true, true, true).unwrap();
        assert!(continued_blank.marks_new_character);
        assert!(!continued_blank.ensure_underwear);
        assert!(!recruit.ensure_underwear);
        assert!(recruit.marks_new_character);
        // A named background without a preset adopts its template.
        assert!(!story.marks_new_character);
        // A saved preset read into a fresh record marks it, and so does
        // the unknown background's blank record; a record already there is
        // reused unmarked.
        let saved = plan_creation_entry(4, 4, 2, false, true, true, true).unwrap();
        assert!(saved.marks_new_character);
        let unknown = plan_creation_entry(4, 4, 0, false, true, true, false).unwrap();
        assert!(unknown.marks_new_character);
        let reused = plan_creation_entry(4, 4, 0, true, true, true, true).unwrap();
        assert!(!reused.marks_new_character);
        // An unlisted background gets a blank, unmarked record.
        let unlisted = plan_creation_entry(4, 4, 7, false, true, true, true).unwrap();
        assert!(!unlisted.marks_new_character);
    }

    #[test]
    fn resetting_the_record_puts_the_braies_back_on() {
        let mut gear = CreationGear {
            hands: [
                GearSlot {
                    item: 3,
                    flags: 0x0100_0000,
                },
                GearSlot::new(4),
                GearSlot::new(5),
                GearSlot::new(6),
            ],
            worn: vec![GearSlot::new(21)],
            carried: vec![GearSlot::new(30)],
        };
        assert_eq!(
            reset_creation_gear(&mut gear),
            CreationGearReset {
                hands: true,
                braies: true,
                carried_cleared: true
            }
        );
        assert_eq!(
            gear.hands[0],
            GearSlot {
                item: 0,
                flags: 0x0100_0000
            }
        );
        assert_eq!(
            gear.hands[1..],
            [GearSlot::new(0), GearSlot::new(0), GearSlot::new(0)]
        );
        assert_eq!(gear.worn, vec![GearSlot::new(8)]);
        assert!(gear.carried.is_empty());
    }

    #[test]
    fn the_final_button_advances_or_commits() {
        let unnamed = submit_creation(SubmitInput {
            page: CreationScreenPage::Appearance,
            name_len: 0,
            known_powers: 0,
            game_mode: 4,
            screen_mode: 4,
            continue_mode: false,
            equipment_hook: false,
        });
        assert!(unnamed.missing_name);
        assert_eq!(unnamed.page, CreationScreenPage::Appearance);
        assert!(unnamed.button.is_none());
        assert_eq!(unnamed.warning, Some(NameWarning::missing()));
        assert_eq!(unnamed.warning.unwrap().code, 0xc1);
        assert_eq!(unnamed.warning.unwrap().title, "Character Name");
        assert_eq!(unnamed.warning.unwrap().text, "Please name your character");

        let named = submit_creation(SubmitInput {
            page: CreationScreenPage::Appearance,
            name_len: 1,
            known_powers: 0,
            game_mode: 4,
            screen_mode: 4,
            continue_mode: false,
            equipment_hook: false,
        });
        assert!(!named.missing_name);
        assert_eq!(named.page, CreationScreenPage::Equipment);
        assert!(named.build_equipment);
        assert_eq!(named.button, Some("BEGIN"));
        assert!(!named.ready_flags);
        assert!(named.tab_flags && named.form_shown);
        assert!(named.warning.is_none());
        assert_eq!(
            named.panels,
            EquipmentPanels {
                apparel: true,
                skills: true,
                ..EquipmentPanels::default()
            }
        );

        let ready = submit_creation(SubmitInput {
            page: CreationScreenPage::Appearance,
            name_len: 3,
            known_powers: 0,
            game_mode: 0x1010,
            screen_mode: 0,
            continue_mode: false,
            equipment_hook: false,
        });
        assert_eq!(ready.button, Some("READY"));
        assert!(ready.ready_flags);
        assert!(ready.panels.apparel);
        assert!(!ready.panels.skills);
        let scripted = equipment_panels(0x1010, true);
        assert!(!scripted.apparel);
        assert!(scripted.skills);
        assert!(scripted.versus_script);
        // The free creator rewrites its starting skills on every visit.
        assert_eq!(
            equipment_panels(0, false),
            EquipmentPanels {
                apparel: true,
                weapons: true,
                starting_skills: true,
                ..EquipmentPanels::default()
            }
        );

        let recruit = submit_creation(SubmitInput {
            page: CreationScreenPage::Equipment,
            name_len: 3,
            known_powers: 0,
            game_mode: 8,
            screen_mode: 10,
            continue_mode: false,
            equipment_hook: false,
        });
        assert_eq!(recruit.commit, Some(CommitKind::Recruit));

        let advanced = submit_creation(SubmitInput {
            page: CreationScreenPage::Equipment,
            name_len: 3,
            known_powers: 2,
            game_mode: 8,
            screen_mode: 10,
            continue_mode: false,
            equipment_hook: false,
        });
        assert_eq!(advanced.commit, Some(CommitKind::PowerTree));
        // The equipment page with powers moves its page state to 2 (the
        // advanced editor) instead of committing, while a record without
        // powers commits on the same page.
        assert!(advanced.advanced_editor);
        assert!(!advanced.build_equipment && !advanced.form_shown);
        assert!(!recruit.advanced_editor);

        let free = submit_creation(SubmitInput {
            page: CreationScreenPage::Equipment,
            name_len: 3,
            known_powers: 0,
            game_mode: 0,
            screen_mode: 0,
            continue_mode: false,
            equipment_hook: false,
        });
        assert_eq!(free.commit, Some(CommitKind::Create));
    }

    #[test]
    fn the_page_switch_goes_back_or_commits() {
        let arena_panels = BuiltPanels {
            skills: true,
            ..BuiltPanels::default()
        };
        let back = switch_creation_page(CreationScreenPage::Equipment, 10, arena_panels);
        assert_eq!(back.page, CreationScreenPage::Appearance);
        assert_eq!(back.destination, PageDestination::Appearance);
        let header = back.header.unwrap();
        assert_eq!(header.first, "NAME");
        assert_eq!(header.second, "HAIR");
        assert_eq!(header.final_button, "NEXT");
        // The arena built the skills panel: it hides and slides.
        assert_eq!(back.hidden_first, Some(HiddenPanel::Skills));
        assert_eq!(back.skills_slide, Some(0x2d));
        assert!(!back.versus_skills_hidden && !back.versus_equipment_hidden);
        assert!(back.form_shown && back.next_enabled);
        assert!(back.appearance_panels_shown);
        // Both appearance tab words take 0x110, the page state word clears
        // and the equipment tab word loses its 0x110 bits.
        assert_eq!(back.appearance_tab_words, [0x110, 0x110]);
        assert!(back.page_state_cleared && back.tab_word_cleared);

        // Weapons come first: the skills panel is neither hidden nor slid.
        let both = switch_creation_page(
            CreationScreenPage::Equipment,
            0,
            BuiltPanels {
                weapons: true,
                skills: true,
                ..BuiltPanels::default()
            },
        );
        assert_eq!(both.hidden_first, Some(HiddenPanel::Weapons));
        assert_eq!(both.skills_slide, None);

        // Versus with both script hooks hides the lobby and its two forms.
        let versus = switch_creation_page(
            CreationScreenPage::Equipment,
            0,
            BuiltPanels {
                lobby: true,
                versus_skills: true,
                versus_equipment: true,
                ..BuiltPanels::default()
            },
        );
        assert_eq!(versus.hidden_first, Some(HiddenPanel::Lobby));
        assert!(versus.versus_skills_hidden && versus.versus_equipment_hidden);

        let continued = switch_creation_page(CreationScreenPage::Equipment, 12, arena_panels);
        assert!(!continued.appearance_panels_shown);
        // A continued character's two tab words are left alone.
        assert_eq!(continued.appearance_tab_words, [0, 0]);
        assert!(continued.page_state_cleared && continued.tab_word_cleared);

        let arena =
            switch_creation_page(CreationScreenPage::Appearance, 10, BuiltPanels::default());
        assert_eq!(arena.destination, PageDestination::Arena);
        assert!(arena.header.is_none());
        assert_eq!(arena.hidden_first, None);
        // Leaving from the appearance page clears nothing.
        assert!(!arena.page_state_cleared && !arena.tab_word_cleared);
        assert_eq!(arena.appearance_tab_words, [0, 0]);

        let other = switch_creation_page(CreationScreenPage::Appearance, 4, BuiltPanels::default());
        assert_eq!(other.destination, PageDestination::Menu);
        assert_eq!(leave_creator(10), PageDestination::Arena);
        assert_eq!(leave_creator(12), PageDestination::Menu);
    }

    #[test]
    fn the_equipment_header_follows_the_versus_script() {
        let normal = equipment_page_header(8, 10, false);
        assert_eq!(normal.first, "APPAREL");
        assert_eq!(normal.second, "WEAPONS");
        assert_eq!(normal.final_button, "RECRUIT");

        let story = equipment_page_header(4, 4, false);
        assert_eq!(story.final_button, "BEGIN");

        let versus = equipment_page_header(0x1010, 0, true);
        assert_eq!(versus.first, "EQUIPMENT");
        assert_eq!(versus.second, "LOBBY");
        assert_eq!(versus.final_button, "READY");

        let no_script = equipment_page_header(0x1010, 0, false);
        assert_eq!(no_script.first, "APPAREL");
        assert_eq!(no_script.second, "WEAPONS");
    }

    #[test]
    fn the_arena_commit_returns_the_new_global_experience() {
        let mut skills = [Skill::default(); SKILLS];
        skills[0] = skill(25000, [0; 6]);
        // The committed worth is the stored one, not the skills' own.
        let commit = commit_arena(skills, 50000, 10000, 2000, 2);
        assert_eq!(commit.global, 42000);
        assert_eq!(commit.skills[0].points, 25000);
        assert_eq!(commit.kind, 2);
        assert!(commit.transition);
    }

    #[test]
    fn the_story_commit_saves_the_preset_and_weighs_the_skills() {
        let mut skills = [Skill::default(); SKILLS];
        skills[0] = skill(25000, [0; 6]);
        let story = commit_story(4, 2, &skills);
        assert_eq!(story.keys.unwrap().root, 0x6431_bc04);
        assert_eq!(story.worth, 37500);
        assert!(story.transition);

        assert_eq!(commit_story(0, 0, &skills).keys, None);
    }

    #[test]
    fn the_save_choices_adjust_and_step() {
        let checkpoint = SaveAvailability(1);
        let save = SaveAvailability(2);
        let both = SaveAvailability(3);

        assert!(select_save(3, SaveKind::Save, checkpoint).adjusted);
        assert_eq!(
            select_save(3, SaveKind::Save, checkpoint).kind,
            SaveKind::Checkpoint
        );
        assert!(select_save(3, SaveKind::Checkpoint, save).adjusted);
        assert_eq!(
            select_save(3, SaveKind::Checkpoint, save).kind,
            SaveKind::Save
        );
        assert!(!select_save(3, SaveKind::Save, both).adjusted);
        assert!(select_save(3, SaveKind::Save, both).deferred);
        assert!(!select_save(3, SaveKind::Save, SaveAvailability(0)).deferred);

        assert_eq!(previous_save(0), None);
        assert_eq!(previous_save(1), Some(0));
        assert_eq!(next_save(0, 3), Some(1));
        assert_eq!(next_save(2, 3), None);
        assert_eq!(next_save(0, 1), None);

        assert_eq!(
            toggle_save_kind(SaveKind::Save, checkpoint),
            Some(SaveKind::Checkpoint)
        );
        assert_eq!(toggle_save_kind(SaveKind::Save, save), None);
        assert_eq!(
            toggle_save_kind(SaveKind::Checkpoint, save),
            Some(SaveKind::Save)
        );
        assert_eq!(toggle_save_kind(SaveKind::Checkpoint, checkpoint), None);
    }

    #[test]
    fn deleting_a_save_moves_to_the_last_remaining() {
        assert_eq!(
            delete_save(1, 3),
            DeleteSave {
                index: 1,
                empty: false
            }
        );
        assert_eq!(
            delete_save(2, 3),
            DeleteSave {
                index: 1,
                empty: false
            }
        );
        assert_eq!(
            delete_save(0, 1),
            DeleteSave {
                index: -1,
                empty: true
            }
        );
        assert!(delete_save(0, 0).empty);
    }

    #[test]
    fn the_continue_layout_disables_its_ends_and_cuts_the_name() {
        let layout = continue_layout(
            0,
            3,
            SaveKind::Save,
            SaveAvailability(2),
            "A very long character name",
        );
        assert!(layout.previous_disabled);
        assert!(!layout.next_disabled);
        assert_eq!(layout.name, "A very long cha");
        assert_eq!(layout.button.label, "Continue from save");
        assert!(layout.button.mirrored);
        assert!(!layout.button.enabled);

        let last = continue_layout(2, 3, SaveKind::Checkpoint, SaveAvailability(3), "X");
        assert!(!last.previous_disabled);
        assert!(last.next_disabled);
        assert_eq!(last.button.label, "Continue from checkpoint");
        assert!(!last.button.mirrored);
        assert!(last.button.enabled);
    }

    #[test]
    fn the_voice_names_and_pitch_follow_the_selection() {
        assert_eq!(two_digits(0), "00");
        assert_eq!(two_digits(1), "01");
        assert_eq!(two_digits(9), "09");
        assert_eq!(two_digits(10), "10");
        assert_eq!(two_digits(100), "100");
        assert_eq!(two_digits(-1), "0");

        let preview = voice_preview("VHM", 0, 0.0);
        assert_eq!(preview.name, "VHM01_GruntS");
        assert_eq!(preview.variant_name, "VHMDA01_GruntS");
        assert_eq!(preview.pitch_bits, 0.9f32.to_bits());
        assert_eq!(voice_preview("VHF", 11, 0.0).name, "VHF12_GruntS");
        assert_eq!(voice_prefix(CreationLabel::Ham), "VHAM");
        assert_eq!(voice_prefix(CreationLabel::None), "V");

        assert_eq!(previous_voice(0), None);
        assert_eq!(previous_voice(1), Some(0));
        assert_eq!(next_voice(4), 5);
        assert_eq!(next_voice(i32::MAX), i32::MIN);

        assert_eq!(voice_pitch(0.0), 0.9);
        // 0.17 + 0.9 in doubles is 1.07 rounded once to a float.
        assert_eq!(voice_pitch(1.0), 1.07);
        // A slider of 0.001 gives 0x3f66718b in doubles; float arithmetic
        // would round to 0x3f66718a.
        assert_eq!(voice_pitch(0.001).to_bits(), 0x3f66_718b);
    }

    #[test]
    fn the_voice_preview_is_throttled_and_wraps() {
        let played = throttle_voice(300, 0);
        assert!(played.played);
        assert_eq!(played.last, 300);
        assert!(!throttle_voice(299, 0).played);
        assert!(!throttle_voice(0, -2).played);
        // A stamp ahead of the clock is a negative distance, not a wrap.
        assert!(!throttle_voice(0, -300).played);
        assert!(throttle_voice(-1, 0).played);
        assert!(!refresh_creation_label(0, -300, true).ran);
    }

    #[test]
    fn the_hair_button_and_sex_change_read_the_flags() {
        assert_eq!(hair_button(0, 0), HairButton::Clear);
        assert_eq!(hair_button(1, 0), HairButton::Hidden);
        assert_eq!(hair_button(1, 0).state(), 0x10);
        assert_eq!(hair_button(0, 0x10_0000), HairButton::Checked);
        assert_eq!(hair_button(0, 0x10_0000).state(), 0x60);
        assert_eq!(hair_button(0, 0x1_0000), HairButton::Clear);

        assert_eq!(set_hair_visible(0, true), 0x1_0000);
        assert_eq!(set_hair_visible(0x1_0000, false), 0);

        let female = change_sex(0, 2);
        assert_eq!(female.word, 1);
        assert_eq!(female.label, CreationLabel::HumanFemale);
        let male = change_sex(1, 1);
        assert_eq!(male.word, 0);
        assert_eq!(male.label, CreationLabel::HumanMale);
        assert_eq!(male.voice_prefix, "VHM");
        let ham = change_sex(0x200, 2);
        assert_eq!(ham.word, 0x201);
        assert_eq!(ham.label, CreationLabel::Ham);
        assert_eq!(ham.voice_prefix, "VHAM");

        assert_eq!(sex_radio(0, 2, false, true, false), None);
        let checked = sex_radio(0x200, 1, true, true, false).unwrap();
        assert_eq!(checked.word, 0x200);
        assert_eq!(checked.voice_prefix, "VHAM");
        assert!(checked.look_refreshed);
        assert!(checked.voice_reset && checked.muted && checked.audio_opened);
        assert!(checked.callback);
        let without_panel = sex_radio(0, 2, true, false, true).unwrap();
        // The audio device is only opened when it was not ready.
        assert!(!without_panel.audio_opened);
        assert!(!without_panel.look_refreshed);
        assert!(without_panel.voice_reset);
    }

    #[test]
    fn the_kit_tables_follow_the_modes() {
        assert_eq!(
            apparel_kit_table(8, 10, 0, false),
            Some(KitTable::ArenaRecruit)
        );
        assert_eq!(
            apparel_kit_table(8, 12, 0, false),
            Some(KitTable::ArenaContinue)
        );
        assert_eq!(apparel_kit_table(2, 0, 0, true), Some(KitTable::Intro));
        assert_eq!(apparel_kit_table(2, 0, 0, false), Some(KitTable::Default));
        assert_eq!(
            apparel_kit_table(4, 0, 0, false),
            Some(KitTable::StoryUnknown)
        );
        assert_eq!(apparel_kit_table(4, 0, 1, false), None);
        assert_eq!(
            apparel_kit_table(0x1010, 0, 0, false),
            Some(KitTable::Versus)
        );
        assert_eq!(apparel_kit_table(0, 0, 0, false), Some(KitTable::Default));

        assert_eq!(KitTable::ArenaRecruit.entries(), 10);
        assert_eq!(KitTable::ArenaContinue.entries(), 15);
        assert_eq!(KitTable::StoryUnknown.entries(), 5);
        assert_eq!(KitTable::Default.entries(), 6);
        assert_eq!(KitTable::Intro.entries(), 10);
        // The versus apparel and mode 2's alternative weapons share one
        // seven-entry table.
        assert_eq!(KitTable::Versus.entries(), 7);
        assert_eq!(KitTable::Weapons.entries(), 6);

        assert_eq!(weapon_kit_table(4, false), None);
        assert_eq!(weapon_kit_table(2, true), Some(KitTable::Versus));
        assert_eq!(weapon_kit_table(2, false), Some(KitTable::Weapons));
        assert_eq!(weapon_kit_table(8, true), Some(KitTable::Weapons));
    }

    #[test]
    fn the_kits_skip_what_is_already_held() {
        let five = KitItem::plain(5);
        let seven = KitItem {
            item: 7,
            variant: 2,
            tint: [4, 5, 6],
            layers: [1, 2, 3, 4],
        };

        assert!(kit_item_matches(5, None, &five));
        assert!(!kit_item_matches(6, None, &five));
        assert!(kit_item_matches(
            9,
            Some(&KitRecord {
                base: 5,
                layers: Some([0; 4])
            }),
            &five
        ));
        assert!(kit_item_matches(
            9,
            Some(&KitRecord {
                base: 7,
                layers: Some([1, 2, 3, 4])
            }),
            &seven
        ));
        // The fourth layer byte counts, and a record without a colour-layer
        // block never matches by base.
        assert!(!kit_item_matches(
            9,
            Some(&KitRecord {
                base: 7,
                layers: Some([1, 2, 3, 5])
            }),
            &seven
        ));
        assert!(!kit_item_matches(
            9,
            Some(&KitRecord {
                base: 7,
                layers: None
            }),
            &seven
        ));

        let worn = vec![GearSlot::new(5), GearSlot::new(9)];
        let record_of = |item: u32| match item {
            9 => Some(KitRecord {
                base: 7,
                layers: Some([1, 2, 3, 4]),
            }),
            _ => None,
        };
        assert!(missing_apparel(&worn, &[five, seven], record_of).is_empty());

        let hands = [
            GearSlot::new(5),
            GearSlot::new(8),
            GearSlot::new(6),
            GearSlot::new(7),
        ];
        let base_of = |item: u32| if item == 8 { 5 } else { item };
        assert_eq!(
            missing_weapons(&hands, &[five, seven], base_of),
            vec![seven]
        );
    }

    #[test]
    fn kit_items_resolve_by_database_flags() {
        let spec = KitItem {
            item: 12,
            variant: 9,
            tint: [2, 3, 4],
            layers: [5, 6, 7, 8],
        };

        let plain = resolve_kit_item(&spec, KitRoute::Plain, None);
        assert_eq!(
            plain,
            ResolvedKitItem {
                item: 12,
                appearance: None,
            }
        );

        // An instance is a new item the database made for the entry: the
        // four layer bytes (each layer then marked with five) and the
        // second and third tint bytes as dyes.
        assert_eq!(resolve_kit_item(&spec, KitRoute::Instance, None).item, 0);
        let instance = resolve_kit_item(&spec, KitRoute::Instance, Some(30));
        assert_eq!(instance.item, 30);
        assert_eq!(KIT_LAYER_MARKER, 5);
        assert_eq!(
            instance.appearance,
            Some(KitAppearance::Instance {
                variant: 9,
                layers: [5, 6, 7, 8],
                dyes: [Some(3), Some(4)],
            })
        );
        // A zero dye byte is left alone, keeping the item's own byte; the
        // first tint byte never reaches an instance.
        let no_dyes = KitItem {
            tint: [9, 0, 4],
            ..spec
        };
        assert_eq!(
            resolve_kit_item(&no_dyes, KitRoute::Instance, Some(30)).appearance,
            Some(KitAppearance::Instance {
                variant: 9,
                layers: [5, 6, 7, 8],
                dyes: [None, Some(4)],
            })
        );

        // An alias applies only with its marker byte; then its target takes
        // the variant and all three tint bytes.
        let alias = resolve_kit_item(
            &spec,
            KitRoute::Alias {
                marker: ALIAS_MARKER,
            },
            Some(21),
        );
        assert_eq!(alias.item, 21);
        assert_eq!(
            alias.appearance,
            Some(KitAppearance::Alias {
                variant: 9,
                tint: [2, 3, 4],
            })
        );
        assert_eq!(ALIAS_MARKER, b'@');

        // Any other marker leaves the entry as itself, and so does a marker
        // without a made item.
        let other = resolve_kit_item(&spec, KitRoute::Alias { marker: b'#' }, Some(21));
        assert_eq!(other.item, 12);
        assert!(other.appearance.is_none());
        let no_alias = resolve_kit_item(
            &spec,
            KitRoute::Alias {
                marker: ALIAS_MARKER,
            },
            None,
        );
        assert_eq!(no_alias.item, 12);
        assert!(no_alias.appearance.is_none());
    }

    #[test]
    fn the_creation_icons_are_four_sheet_cells() {
        assert_eq!(CREATION_ICON_SIZE, 96);
        assert_eq!(CREATION_ICON_OFFSETS, [288, 192, 96, 0]);
        assert_eq!(creation_icon_size(4), 32);
        assert_eq!(creation_icon_size(5), 40);
    }

    #[test]
    fn the_apparel_panel_is_made_once_and_bound() {
        // The first visit hides the versus equipment form, makes the panel,
        // fills it from the kit and binds it as the current equipment form.
        assert_eq!(
            show_apparel_panel(true, false, Some(5), 3),
            ApparelPanel {
                versus_equipment_hidden: true,
                created: true,
                shown: true,
                finished: true,
                current_form: true,
                items: 3,
            }
        );
        // A second visit only shows it again: no items, nothing finished,
        // and it is still bound.
        assert_eq!(
            show_apparel_panel(false, true, Some(5), 3),
            ApparelPanel {
                versus_equipment_hidden: false,
                created: false,
                shown: true,
                finished: false,
                current_form: true,
                items: 0,
            }
        );
        // A named story background stops after the made panel is bound:
        // it is the current equipment form but gets no items or finish.
        assert_eq!(
            show_apparel_panel(false, false, None, 4),
            ApparelPanel {
                versus_equipment_hidden: false,
                created: true,
                shown: true,
                finished: false,
                current_form: true,
                items: 0,
            }
        );
        // An existing panel is bound whatever the mode's table.
        assert!(show_apparel_panel(false, true, None, 0).current_form);
    }

    #[test]
    fn the_weapons_panel_is_made_once_and_filled() {
        assert_eq!(
            show_weapon_panel(true, false, Some(6), 2),
            WeaponPanel {
                second_panel_hidden: true,
                created: true,
                shown: true,
                finished: true,
                flags: 0x2_0000,
                items: 2,
            }
        );
        assert_eq!(
            show_weapon_panel(false, true, Some(6), 2),
            WeaponPanel {
                second_panel_hidden: false,
                created: false,
                shown: true,
                finished: false,
                flags: 0,
                items: 0,
            }
        );
        // Story mode's table is empty: the panel is made and finished with
        // nothing in it.
        assert_eq!(
            show_weapon_panel(false, false, Some(0), 0),
            WeaponPanel {
                second_panel_hidden: false,
                created: true,
                shown: true,
                finished: true,
                flags: 0x2_0000,
                items: 0,
            }
        );
    }

    #[test]
    fn the_creator_panels_place_against_the_form() {
        assert_eq!(
            apparel_panel_place(true),
            PanelPlace {
                x: 4,
                y: 0x18,
                w: 0xac,
                h: 0x26c
            }
        );
        assert_eq!(apparel_panel_place(false).h, 0x238);
        assert_eq!(
            weapon_panel_place(1000, false),
            PanelPlace {
                x: 824,
                y: 0x18,
                w: 0xac,
                h: 0x238
            }
        );
        assert_eq!(
            skill_panel_place(1000, 10),
            PanelPlace {
                x: 772,
                y: 0x18,
                w: 0xd8,
                h: 0x204
            }
        );
        assert_eq!(skill_panel_place(1000, 0).h, 0x294);
    }

    #[test]
    fn the_skill_tab_slides_by_four_units() {
        let placed = skill_tab_place(800, 100, 4, 6, false);
        assert_eq!(placed.width, 676);
        assert_eq!(placed.tab_x, 8);
        assert_eq!(placed.tab_y, 784);

        let continued = skill_tab_place(800, 100, 4, 6, true);
        assert_eq!(continued.width, 676);
        assert_eq!(continued.tab_x, 0);
        assert_eq!(continued.tab_y, 0);

        // The two label fields are 16-bit words: a larger product wraps.
        let wrapped = skill_tab_place(40_000, 0, 1_000, 40, false);
        assert_eq!(wrapped.tab_x, 36_000u16 as i16 as i32);
        assert_eq!(wrapped.tab_y, 2_000);
        assert_eq!(wrapped.tab_x, -29_536);
    }

    #[test]
    fn the_continue_screen_places_its_list_and_name() {
        let place = continue_place(4, true);
        assert_eq!(place.view_x, 236);
        assert_eq!(place.list_step, 2);
        assert_eq!(place.list_flags, 0x80);
        assert_eq!(place.list_word, 0x0010_0010);
        assert_eq!(place.name_flags, 0x200);
        assert_eq!(place.name_word, 0x0600_0200);
        assert_eq!(place.details_word, 0x100);
        assert_eq!(
            place.list,
            PanelPlace {
                x: 4,
                y: 0x74,
                w: 0xe4,
                h: 0xb4
            }
        );
        assert_eq!(
            place.details,
            PanelPlace {
                x: 0,
                y: 0x148,
                w: 0xec,
                h: 0x140
            }
        );
        assert_eq!(
            place.name,
            PanelPlace {
                x: 4,
                y: 0x28,
                w: 0xe4,
                h: 0x118
            }
        );
        assert_eq!(continue_place(4, false).name.h, 0xdc);
    }

    #[test]
    fn a_save_slot_offering_nothing_keeps_the_kind() {
        let none = select_save(4, SaveKind::Save, SaveAvailability(0));
        assert_eq!(none.kind, SaveKind::Save);
        assert!(!none.adjusted);

        let changed = select_save(4, SaveKind::Save, SaveAvailability(1));
        assert_eq!(changed.kind, SaveKind::Checkpoint);
        assert!(changed.adjusted);

        assert_eq!(select_save_from_list(3, 1), Some(2));
        assert_eq!(select_save_from_list(3, 2), None);
        // Position 0 is not filtered: it selects index -1.
        assert_eq!(select_save_from_list(0, 0), Some(-1));
    }

    #[test]
    fn the_creator_opens_on_the_right_continue_slot() {
        let story = open_creator(4, SaveAvailability(2), false);
        assert!(!story.show_continue);
        assert!(!story.reset_to_portal);
        assert!(story.update_input);

        let save = open_creator(12, SaveAvailability(2), true);
        assert_eq!(save.continue_kind, SaveKind::Save);
        assert!(save.show_continue);
        assert!(save.reset_to_portal);

        let checkpoint = open_creator(12, SaveAvailability(1), false);
        assert_eq!(checkpoint.continue_kind, SaveKind::Checkpoint);
        assert!(!checkpoint.reset_to_portal);
    }

    #[test]
    fn the_equipment_tabs_store_and_toggle() {
        assert_eq!(equipment_tab(2), EquipmentTab::Character);
        // Index 0 hides the form; any other index shows it.
        assert_eq!(equipment_tab(0), EquipmentTab::Other { enabled: false });
        assert_eq!(equipment_tab(1), EquipmentTab::Other { enabled: true });
    }

    #[test]
    fn a_voice_selection_needs_its_sample() {
        // A plain voice takes only with its sample: the screen keeps the
        // index, the record's voice word takes it, the seventh control shows
        // `A` plus the index and the sample previews unless muted.
        assert_eq!(
            select_voice(3, true, false),
            VoiceChoice {
                index: Some(3),
                word: VoiceWord::Voice,
                caption: Some(0x44),
                plays: true,
            }
        );
        assert_eq!(
            select_voice(0, true, true),
            VoiceChoice {
                index: Some(0),
                word: VoiceWord::Voice,
                caption: Some(0x41),
                plays: false,
            }
        );
        assert_eq!(
            select_voice(3, false, false),
            VoiceChoice {
                index: None,
                word: VoiceWord::None,
                caption: None,
                plays: false,
            }
        );

        // The variant voice of a race with its band set takes the index into
        // the variant word with no sample, no caption and no preview; a
        // human needs its `DA` sample, and then captions and previews like
        // the plain voice.
        assert_eq!(
            select_variant_voice(2, true, false, false),
            VoiceChoice {
                index: Some(2),
                word: VoiceWord::Variant,
                caption: None,
                plays: false,
            }
        );
        assert_eq!(
            select_variant_voice(2, false, false, false),
            VoiceChoice {
                index: None,
                word: VoiceWord::None,
                caption: None,
                plays: false,
            }
        );
        assert_eq!(
            select_variant_voice(2, false, true, false),
            VoiceChoice {
                index: Some(2),
                word: VoiceWord::Variant,
                caption: Some(0x43),
                plays: true,
            }
        );
        assert!(!select_variant_voice(2, false, true, true).plays);
    }

    #[test]
    fn the_voice_variant_avoids_the_last_draws() {
        let mut rng = Lcg(0);
        let mut history = VoiceHistory::default();

        // The first two draws are 0 and 0 and repeat the last variant; the
        // third is 3.
        assert_eq!(draw_voice_variant(&mut rng, 4, &mut history), Some(3));
        assert_eq!(history.last, 3);
        assert_eq!(history.before_last, 0);

        let mut rng = Lcg(0);
        let mut history = VoiceHistory::default();
        assert_eq!(draw_voice_variant(&mut rng, 2, &mut history), Some(1));
        assert_eq!(history.last, 1);

        let mut rng = Lcg(0);
        let mut history = VoiceHistory {
            last: 5,
            before_last: 5,
        };
        assert_eq!(draw_voice_variant(&mut rng, 0, &mut history), None);
        assert_eq!(history.last, 5);
        assert_eq!(draw_voice_variant(&mut rng, 1, &mut history), Some(0));
        assert_eq!(rng.0, 0);
        assert_eq!(history.last, 0);
        assert_eq!(history.before_last, 5);
    }

    #[test]
    fn a_loaded_preset_clears_the_race_and_powers() {
        let load = apply_loaded_preset(0x1234_0101);
        assert_eq!(load.profession, 0x1234_0001);
        assert_eq!(load.known_powers, 0);
        assert!(load.power_slots_cleared);

        let template = use_template(true);
        assert!(template.replaced);
        assert_eq!(template.known_powers, 0);
        assert!(template.power_slots_cleared);
        assert!(!use_template(false).replaced);
    }

    #[test]
    fn adding_a_preview_selects_it_after_the_last_was_hovered() {
        // A hovered -1 is 2^32 - 1 unsigned and never matches.
        assert_eq!(push_preview(0, Some(-1)).selected, None);
        assert_eq!(
            push_preview(4, None),
            PreviewPush {
                count: 5,
                selected: None,
                refreshed: true,
                finished: true
            }
        );
        assert_eq!(
            push_preview(4, Some(3)),
            PreviewPush {
                count: 5,
                selected: Some(4),
                refreshed: true,
                finished: true
            }
        );
        assert_eq!(
            push_preview(4, Some(2)),
            PreviewPush {
                count: 5,
                selected: None,
                refreshed: true,
                finished: true
            }
        );
    }

    #[test]
    fn the_continue_row_and_delete_text_follow_the_selection() {
        let rows = ["Mira", "Ann", "Bob"];
        assert_eq!(
            continue_name_row(1, &rows),
            ContinueNameRow {
                row: 0,
                text: Some("Mira")
            }
        );
        assert_eq!(
            continue_name_row(3, &rows),
            ContinueNameRow {
                row: 2,
                text: Some("Bob")
            }
        );
        // The pick is not checked: zero stores -1 and writes nothing, and a
        // row past the list has no text either.
        assert_eq!(
            continue_name_row(0, &rows),
            ContinueNameRow {
                row: -1,
                text: None
            }
        );
        assert_eq!(
            continue_name_row(-1, &rows),
            ContinueNameRow {
                row: -2,
                text: None
            }
        );
        assert_eq!(
            continue_name_row(4, &rows),
            ContinueNameRow { row: 3, text: None }
        );

        // The page opens on the list's first entry: the label selects it of
        // the list's three and shows its display string, the page keeps row
        // 0 and the name box takes the first name.
        let labels = ["Mira, day 3", "Ann, day 1", "Bob, day 9"];
        assert_eq!(
            continue_texts(&labels, &rows),
            ContinueTexts {
                row: Some(0),
                label: Some([1, 3]),
                label_text: Some("Mira, day 3"),
                name_text: Some("Mira"),
            }
        );
        // An empty list clears both texts and keeps the stored row.
        assert_eq!(
            continue_texts(&[], &rows),
            ContinueTexts {
                row: None,
                label: None,
                label_text: None,
                name_text: None,
            }
        );
        // The count is the label list's; a name list shorter than it leaves
        // no name to show.
        let short = continue_texts(&labels, &[]);
        assert_eq!(short.label, Some([1, 3]));
        assert_eq!(short.row, Some(0));
        assert_eq!(short.name_text, None);

        assert_eq!(delete_confirm_text("Delete ", "Mira"), "Delete Mira?");
        assert_eq!(copy_creation_name("Mira"), "Mira");

        assert_eq!(select_save_from_list(4, 2), Some(3));
        assert_eq!(select_save_from_list(3, 2), None);
        assert_eq!(select_save_from_list(0, 2), Some(-1));
    }

    #[test]
    fn the_icon_sheet_is_created_once() {
        assert!(ensure_creation_icons(false, false).installed);
        assert!(!ensure_creation_icons(true, false).built);
    }

    #[test]
    fn the_creation_sliders_write_nan_and_round_like_the_original() {
        let epsilon = f32::from_bits(0x33ff_ffd7);
        let nan = settle_appearance(f32::NAN, 0.0, 0, 0);
        assert!(nan.stored.unwrap().is_nan());
        assert!(nan.pending);
        assert!(!nan.callback);

        let nan_due = settle_appearance(f32::NAN, 0.0, 0, 2);
        assert!(nan_due.stored.unwrap().is_nan());
        assert!(nan_due.callback);

        let boundary = settle_appearance(0.5, -epsilon, 10, 12);
        assert_eq!(boundary.stored, Some(0.0));
        assert!(boundary.callback);

        let copied = copy_appearance(f32::NAN, 0.0, 0, 0);
        assert!(copied.stored.unwrap().is_nan());

        let just_below = copy_appearance(f32::from_bits(0x33ff_ffd6), 0.0, 10, 12);
        assert_eq!(just_below.stored, None);

        let above = copy_appearance(epsilon * 2.0, 0.0, 10, 12);
        assert_eq!(above.stored, Some(epsilon * 2.0));
    }

    #[test]
    fn the_creation_deferred_calls_use_the_unsigned_clock() {
        let fired = flush_creation_deferred(true, 10, 12);
        assert!(fired.fired);
        assert_eq!(fired.stamp, 12);
        assert!(!fired.pending);

        let no_wrap = flush_creation_deferred(true, -1, 1);
        assert!(!no_wrap.fired);
        assert!(no_wrap.pending);

        let early = flush_creation_deferred(true, 10, 11);
        assert!(!early.fired);
        assert!(early.pending);

        let due = flush_creation_deferred(true, 10, 12);
        assert!(due.fired);

        assert!(refresh_creation_label(300, 0, false).ran);
        assert!(!refresh_creation_label(299, 0, false).ran);
    }

    /// The preview frame: 180 in, 24 down, the parent less 360 wide and
    /// 28 short, unclamped.
    #[test]
    fn the_preview_frame_is_the_parent_less_its_margins() {
        assert_eq!(
            preview_frame((1024, 768)),
            PreviewFrame {
                x: 180,
                y: 24,
                width: 664,
                height: 740
            }
        );
        assert_eq!(preview_frame((300, 20)).width, -60);
        assert_eq!(preview_frame((300, 20)).height, -8);
    }

    /// The equipment page's panels share the preview's parent and keep the
    /// original's fixed widths and the aspect-dependent heights.
    #[test]
    fn the_creation_form_lays_out_its_panels() {
        let layout = creation_form_layout((1024, 768), 10, true);
        assert_eq!(
            layout.preview,
            PreviewFrame {
                x: 180,
                y: 24,
                width: 664,
                height: 740
            }
        );
        assert_eq!(
            layout.apparel,
            PanelPlace {
                x: 4,
                y: 24,
                w: 172,
                h: 620
            }
        );
        assert_eq!(
            layout.weapons,
            PanelPlace {
                x: 1024 - 176,
                y: 24,
                w: 172,
                h: 620
            }
        );
        assert_eq!(
            layout.skills,
            PanelPlace {
                x: 1024 - 216 - 12,
                y: 24,
                w: 216,
                h: 516
            }
        );

        let short = creation_form_layout((800, 600), 0, false);
        assert_eq!(short.apparel.h, 568);
        assert_eq!(short.weapons.h, 568);
        assert_eq!(short.skills.h, 660);
    }

    /// Mode 12 takes the ten-element table without editors; the others
    /// seven elements and the editors.
    #[test]
    fn the_creation_preview_view_close_and_change() {
        let view = preview_view();
        assert_eq!((view.border, view.form_flags), (2, 0xf_0080));
        assert_eq!(view.style(0xffff_ffff), 0xffff_3fff);
        assert_eq!(view.style(0), 0x90);
        assert!(view.smoothing && !view.bloom && !view.second_switch);
        assert_eq!(PREVIEW_EYE_CM, [-200.0, 210.0, 360.0]);
        assert_eq!(PREVIEW_TARGET_CM, [0.0, 60.0, 0.0]);
        assert_eq!(
            (PREVIEW_FOV_DEG, PREVIEW_FOG_MODE, PREVIEW_RENDER_WORD),
            (50.0, 2, 0x4100)
        );
        let close = creation_form_close(&[8, 0x31], true, true);
        assert!(!close.smoothing && close.close_child && close.edit.detach);
        assert_eq!(close.refreshed, [8, 0x31]);
        assert_eq!(release_drop_record(1), DropRecordRelease::Freed);
        assert_eq!(
            release_drop_record(3),
            DropRecordRelease::Released { count: 2 }
        );
        use PreviewChangeStep::*;
        assert_eq!(
            preview_change(true),
            [
                Busy, Regenerate, Refresh, HairPick, Textures, Materials, Idle
            ]
        );
        assert_eq!(preview_change(false).len(), 6);
        assert!(refresh_preview_body(true) && !refresh_preview_body(false));
        assert_eq!(mode_container_drop(8, true), None);
        assert_eq!(mode_container_drop(9, false), None);
        assert_eq!(
            mode_container_drop(9, true),
            Some(EquipDrop {
                list: DropList::Mode,
                place: true
            })
        );
    }

    #[test]
    fn the_creation_table_words_follow_the_game_mode() {
        assert_eq!(creation_table_words(3, 4), None);
        assert_eq!(
            creation_table_words(0xc, 4),
            Some(CreationTableWords {
                first: 0,
                second: 0x10,
                handler: true
            })
        );
        assert_eq!(
            creation_table_words(0xc, 8),
            Some(CreationTableWords {
                first: 0x110,
                second: 0x20,
                handler: false
            })
        );
        assert_eq!(
            (BODY_EDITOR_AT, HAIR_EDITOR_RIGHT, HAIR_EDITOR_TOP),
            ([4, 60], 176, 32)
        );
        assert_eq!((CREATION_FORM_FIFTY, HAIR_EDITOR_STYLE), (50, 0x20000));
    }

    #[test]
    fn the_creation_layout_has_two_tables() {
        assert_eq!(
            creation_layout(0xc),
            CreationLayout {
                elements: 10,
                editors: false
            }
        );
        assert_eq!(
            creation_layout(3),
            CreationLayout {
                elements: 7,
                editors: true
            }
        );
    }

    /// The drop: the mode list first, then game modes 8 and 0x1010, then
    /// the item's kinds in order; 0x8000 is not placed; item 8 and items
    /// without a record are refused.
    #[test]
    fn drops_route_by_mode_and_kind() {
        let apparel = |k: u32| k == APPAREL_KIND || k == OTHER_KIND;
        let weapon = |k: u32| k == WEAPON_KIND || k == OTHER_KIND;
        let other = |k: u32| k == OTHER_KIND;
        let nothing = |_: u32| false;
        let at = |list, place| Some(EquipDrop { list, place });
        assert_eq!(
            equip_drop(5, 0, false, Some(&apparel)),
            at(DropList::Apparel, true)
        );
        assert_eq!(
            equip_drop(5, 0, false, Some(&weapon)),
            at(DropList::Weapons, true)
        );
        assert_eq!(
            equip_drop(5, 0, false, Some(&other)),
            at(DropList::Weapons, false)
        );
        assert_eq!(equip_drop(5, 0, false, Some(&nothing)), None);
        assert_eq!(equip_drop(5, 0, false, None), None);
        assert_eq!(equip_drop(5, 8, false, None), at(DropList::Apparel, true));
        assert_eq!(
            equip_drop(5, 0x1010, false, None),
            at(DropList::Hook, false)
        );
        assert_eq!(equip_drop(5, 8, true, None), at(DropList::Mode, true));
        assert_eq!(equip_drop(8, 0, true, Some(&apparel)), None);
    }

    #[test]
    fn the_preview_list_grows_and_only_the_last_hover_moves_the_selection() {
        let mut list = PreviewIcons::default();
        assert_eq!(
            add_preview_icon(&mut list, 10, None),
            PreviewPush {
                count: 1,
                selected: None,
                refreshed: true,
                finished: true
            }
        );
        assert_eq!(list.icons, vec![10]);
        assert_eq!(list.selected, None);

        // The pointer was on the old last entry (index 0 of 1).
        assert_eq!(
            add_preview_icon(&mut list, 11, Some(0)),
            PreviewPush {
                count: 2,
                selected: Some(1),
                refreshed: true,
                finished: true
            }
        );
        assert_eq!(list.selected, Some(1));
        assert_eq!(list.icons, vec![10, 11]);

        // Index 0 is no longer the last entry of the two.
        assert_eq!(
            add_preview_icon(&mut list, 12, Some(0)),
            PreviewPush {
                count: 3,
                selected: None,
                refreshed: true,
                finished: true
            }
        );
        assert_eq!(list.selected, Some(1));
        assert_eq!(list.icons, vec![10, 11, 12]);

        // A negative hovered word is zero-extended, so it never matches.
        assert_eq!(add_preview_icon(&mut list, 13, Some(-1)).selected, None);
        assert!(release_preview_icon(true));
        assert!(!release_preview_icon(false));
    }

    #[test]
    fn a_preview_material_covers_every_listed_item() {
        let items = [40, 41, 42, 43];
        // Without a generator nothing is applied; with one, every listed
        // item takes the same material in list order.
        assert!(apply_preview_material(false, &items).is_empty());
        assert_eq!(apply_preview_material(true, &items), items.to_vec());
        assert!(apply_preview_material(true, &[]).is_empty());

        assert_eq!(
            release_preview_material(true),
            PreviewMaterialRelease {
                released: true,
                cleared: true
            }
        );
        assert_eq!(
            release_preview_material(false),
            PreviewMaterialRelease {
                released: false,
                cleared: false
            }
        );
    }

    #[test]
    fn the_dummy_head_preview_sets_its_model_and_hooks() {
        let head = dummy_head_preview(96);
        assert_eq!(head.model, "dummyhead.rfc");
        assert_eq!(head.options, 0x00a0_a040);
        assert_eq!(head.icon_size, 96);
        assert_eq!(head.entry_height, 98);
        assert_eq!(head.clear_flag, 0);
        assert_eq!(head.references, 1);
        assert!(head.reset);
        assert_eq!(
            head.hooks,
            [
                DummyHeadHook::Material,
                DummyHeadHook::Icon,
                DummyHeadHook::Release
            ]
        );

        let tall = dummy_head_preview(120);
        assert_eq!(tall.entry_height, 122);

        let refresh = refresh_character_preview();
        assert!(refresh.rebuilt && refresh.labels && refresh.textures && refresh.materials);
    }

    #[test]
    fn binding_panels_keeps_the_character_and_figure() {
        assert_eq!(
            bind_creation_panels(true, true),
            CreationPanelBinding {
                character: true,
                figure: true,
                record: true,
                item_list: true,
                second_panel: true
            }
        );
        assert_eq!(
            bind_creation_panels(false, false),
            CreationPanelBinding {
                character: true,
                figure: true,
                record: true,
                item_list: false,
                second_panel: false
            }
        );
    }

    #[test]
    fn the_creator_opens_the_power_tree_with_its_two_callbacks() {
        assert_eq!(
            open_creator_power_tree(),
            CreatorPowerTree {
                gui_hidden: true,
                creator_hidden: true,
                tree_opened: true,
                commit: CreatorTreeHook::Commit,
                close: CreatorTreeHook::Close
            }
        );
        assert_eq!(
            close_creator_power_tree(),
            CreatorTreeClosed {
                tree_closed: true,
                creator_shown: true,
                input_cleared: true
            }
        );
        assert_eq!(
            reopen_creator_power_tree(),
            TreeReopen {
                deferred: true,
                closes_tree: true,
                arg: 0
            }
        );
        assert_eq!(
            close_name_warning(false),
            NameWarningClose {
                closed: true,
                via_steam_input: false
            }
        );
        assert!(close_name_warning(true).via_steam_input);
        assert_eq!(reset_to_portal_dialog().centred, [-1, -1]);
    }

    #[test]
    fn the_delete_dialog_appends_the_cut_name() {
        let dialog = delete_confirm_dialog("Delete ", "Mira");
        assert_eq!(dialog.text, "Delete Mira?");
        assert!(dialog.created && dialog.owner_bound && dialog.shown);
        assert_eq!(dialog.centred, [-1, -1]);

        let long = delete_confirm_dialog("Delete ", "Abcdefghijklmnopq");
        assert_eq!(long.text, "Delete Abcdefghijklmno?");
    }

    fn continue_input(name: &str) -> ContinueViewInput<'_> {
        ContinueViewInput {
            screen_mode: 0,
            game_mode: 0,
            name,
            model_available: true,
            loader_hook: false,
            item_count: 0,
            extra_item: false,
            listed_flagged: false,
            panel: false,
            second_panel: false,
            item_list: false,
            record_flags: 0,
        }
    }

    #[test]
    fn the_continue_view_names_and_labels_its_character() {
        let plain = continue_view(continue_input("Abcdefghijklmnopq"));
        assert_eq!(plain.name, "Abcdefghijklmno");
        assert_eq!(plain.title, None);
        assert_eq!(plain.item_passes, 0);
        assert!(plain.model && plain.view_target);
        assert!(plain.cleared_view);
        assert!(!plain.record_applied);

        let mut input = continue_input("Mira");
        input.screen_mode = 12;
        input.item_count = 3;
        input.extra_item = true;
        let continued = continue_view(input);
        assert_eq!(continued.name, "");
        assert_eq!(continued.title, Some(exa_ui::creation::CONTINUE));
        assert_eq!(continued.item_passes, 3);
        assert_eq!(continued.listed_passes, 0);
        assert_eq!(continued.extra_passes, 3);
        input.listed_flagged = true;
        assert_eq!(continue_view(input).listed_passes, 3);
        input.listed_flagged = false;
        assert!(continued.record_applied);

        input.game_mode = 0x1010;
        assert_eq!(continue_view(input).title, Some(exa_ui::creation::VERSUS));

        let mut waiting = continue_input("Mira");
        waiting.loader_hook = true;
        waiting.model_available = false;
        let waited = continue_view(waiting);
        assert!(waited.waited);
        assert!(!waited.model);
    }

    #[test]
    fn the_continue_view_binds_its_panels_and_hair_state() {
        let mut input = continue_input("Mira");
        input.panel = true;
        input.second_panel = true;
        input.item_list = true;
        input.record_flags = 0x0030_0000;
        let both = continue_view(input);
        assert_eq!(both.panels_bound, 2);
        assert!(both.look_refreshed);
        assert_eq!(both.panels_shown, 2);
        assert_eq!(both.hair_button, None);
        assert_eq!(both.panel_targets, 2);
        assert!(both.item_list_attached);

        input.second_panel = false;
        let male = continue_view(input);
        assert_eq!(male.panels_bound, 2);
        assert!(!male.look_refreshed);
        assert_eq!(male.hair_button, Some(HairButton::Checked));
        assert_eq!(male.panels_shown, 1);

        input.record_flags = 1;
        assert_eq!(continue_view(input).hair_button, Some(HairButton::Hidden));

        let mut absent = continue_input("Mira");
        absent.panel = false;
        let none = continue_view(absent);
        assert_eq!(none.panels_bound, 0);
        assert_eq!(none.panels_shown, 0);
        assert_eq!(none.panel_targets, 0);
        assert_eq!(none.hair_button, None);
    }

    #[test]
    fn the_continue_page_lays_out_the_chosen_kind() {
        let saves = ContinueSaveList {
            labels: &["Mira, day 3", "Ann, day 1"],
            names: &["Mira", "Ann"],
        };
        let page = open_continue_page(
            continue_input("Mira"),
            saves,
            SaveKind::Save,
            SaveAvailability(2),
            true,
            4,
            true,
        );
        assert!(page.gui_hidden);
        assert_eq!(page.button.label, exa_ui::creation::CONTINUE_FROM_SAVE);
        assert!(page.button.mirrored);
        assert!(!page.button.enabled);
        assert!(page.reset_dialog);
        assert_eq!(page.texts.row, Some(0));
        assert_eq!(page.texts.name_text, Some("Mira"));
        assert_eq!(page.texts.label_text, Some("Mira, day 3"));
        assert_eq!(page.texts.label, Some([1, 2]));
        assert_eq!(page.place.view_x, 4 * 0x3b);
        assert_eq!(page.place.details.h, 0x140);

        let saves = ContinueSaveList {
            labels: &[],
            names: &["Mira", "Ann"],
        };
        let page = open_continue_page(
            continue_input("Mira"),
            saves,
            SaveKind::Checkpoint,
            SaveAvailability(3),
            false,
            4,
            false,
        );
        assert_eq!(
            page.button.label,
            exa_ui::creation::CONTINUE_FROM_CHECKPOINT
        );
        assert!(page.button.enabled);
        assert!(!page.reset_dialog);
        assert_eq!(page.texts.name_text, None);
        assert_eq!(page.texts.label_text, None);
        assert_eq!(page.place.details.h, 0x104);
    }

    #[test]
    fn the_continue_save_steps_offer_the_checkpoint_first() {
        assert_eq!(reselect_save(-3), -3);
        assert_eq!(previous_save_step(0), None);
        assert_eq!(
            previous_save_step(2),
            Some(SaveStep {
                index: 1,
                kind: SaveKind::Checkpoint
            })
        );
        assert_eq!(
            next_save_step(1, 3),
            Some(SaveStep {
                index: 2,
                kind: SaveKind::Checkpoint
            })
        );
        assert_eq!(next_save_step(2, 3), None);
        assert_eq!(
            pick_save_from_list(4, 2),
            Some(SavePick {
                index: 3,
                kind: SaveKind::Checkpoint
            })
        );
        assert_eq!(pick_save_from_list(3, 2), None);
    }

    #[test]
    fn deleting_the_selected_save_clamps_and_reselects() {
        assert_eq!(
            delete_selected_save(2, 3),
            DeleteSelectedSave {
                index: 2,
                closed: false,
                reselected: true
            }
        );
        assert_eq!(
            delete_selected_save(5, 3),
            DeleteSelectedSave {
                index: 2,
                closed: false,
                reselected: true
            }
        );
        assert_eq!(
            delete_selected_save(0, 0),
            DeleteSelectedSave {
                index: -1,
                closed: true,
                reselected: false
            }
        );
    }

    #[test]
    fn the_icon_sheet_adds_four_cells() {
        assert_eq!(CREATION_ICON_SHEET, "charcreate.rfi");
        let built = build_creation_icons(4, false, true);
        assert!(!built.reused);
        assert_eq!(built.word, 0x203);
        assert_eq!(built.flags, 0x0100_2008);
        assert_eq!(built.icon_size, 32);
        assert_eq!(build_creation_icons(5, false, true).icon_size, 40);
        assert!(built.loaded);
        assert_eq!(built.cells, 4);
        // The cells are cut from a 96 by 384 sheet, top band first, at mip
        // level 0 of a 96 by 96 image made with the manager's own flags.
        assert_eq!(built.bands, [288, 192, 96, 0]);
        assert_eq!(built.cell_size, 96);
        assert_eq!(CREATION_ICON_HEIGHT, 384);
        assert_eq!(built.image_flags, 0x2000_0000);
        assert_eq!(built.mip, 0);
        assert!(!built.reset);

        // A manager that exists is reset instead of made, and a missing
        // sheet still resets it but adds no cells.
        let missing = build_creation_icons(4, true, false);
        assert!(missing.reused);
        assert!(missing.reset);
        assert!(!missing.loaded);
        assert_eq!(missing.cells, 0);
        assert_eq!(missing.bands, [0; 4]);
        assert_eq!(missing.icon_size, 32);
    }

    #[test]
    fn the_icon_sheet_chains_the_reload_hook_once() {
        // The first call builds the sheet and takes the reload hook's slot,
        // keeping the hook that was already there.
        assert_eq!(
            ensure_creation_icons(false, true),
            CreationIconHook {
                built: true,
                previous_hook: true,
                installed: true,
            }
        );
        // With no earlier hook the slot is still taken.
        assert_eq!(
            ensure_creation_icons(false, false),
            CreationIconHook {
                built: true,
                previous_hook: false,
                installed: true,
            }
        );
        // An existing manager is returned untouched: no rebuild, no chain.
        assert_eq!(
            ensure_creation_icons(true, false),
            CreationIconHook {
                built: false,
                previous_hook: false,
                installed: false,
            }
        );
    }

    #[test]
    fn showing_the_creator_makes_the_form_once() {
        let first = show_creator_here(false, true);
        assert_eq!(
            first,
            CreatorShow {
                created: true,
                viewport: true,
                view_word: 0x1002,
                record: true,
                hook: true
            }
        );
        let again = show_creator_here(true, false);
        assert!(!again.created);
        assert!(!again.hook);
        assert!(again.viewport && again.record);
    }

    #[test]
    fn the_creator_form_opens_with_three_input_actions() {
        let saved = open_creator_form(12, SaveAvailability(2), false);
        assert_eq!(saved.continue_kind, Some(SaveKind::Save));
        assert!(saved.cursor_reset && saved.form_shown && saved.centred);
        assert!(saved.viewport_shown);
        assert_eq!(saved.input_actions, 3);
        assert!(!saved.reset_dialog);

        let checkpoint = open_creator_form(12, SaveAvailability(3), true);
        assert_eq!(checkpoint.continue_kind, Some(SaveKind::Checkpoint));
        assert!(checkpoint.reset_dialog);

        let free = open_creator_form(0, SaveAvailability(3), true);
        assert_eq!(free.continue_kind, None);
        assert!(!free.reset_dialog);
    }

    /// The installed sheet is the 96 by 384 image the four cells are cut
    /// from; skipped without `EXANIMA_DIR`.
    #[test]
    fn the_installed_creator_sheet_has_four_cells() {
        let Ok(root) = std::env::var("EXANIMA_DIR") else {
            return;
        };
        let Ok(data) = exa_rpk::GameData::open(root) else {
            return;
        };
        let Some(bytes) = data.texture("charcreate") else {
            return;
        };
        let rfi = exa_formats::rfi::Rfi::parse(bytes).expect("charcreate.rfi parses");
        assert_eq!(rfi.header.width, CREATION_ICON_SIZE as u32);
        assert_eq!(
            rfi.header.height,
            CREATION_ICON_SIZE as u32 * CREATION_ICON_OFFSETS.len() as u32
        );
        assert_eq!(rfi.mip_count, 1);
    }
}
