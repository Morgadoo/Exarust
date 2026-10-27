//! The thaumaturgy power tree, its power bar and the character's power
//! record, as the original's tree screen runs them.
//!
//! - **Domains**. A power word carries its domain in bits 8 to 15: mind 1,
//!   force 2, body 3, energy 4, light 5 and displacement 6. Bits 0 to 7 are
//!   the power's place in the domain. Bits 16 and 17 mark a sustained and a
//!   persistent power. Bits 20 to 23 and 28 to 31 are ignored when a known
//!   power is matched to a tree node; the high nibble is the technique a
//!   power requires and the low nibble picks its frame.
//! - **The record** ([`PowerRecord`]) holds the character's remaining focus
//!   (4.0 for a new character), its two specialised domains, its known
//!   power pairs, its ten power-bar slots and the Insight and Concentration
//!   skills whose techniques the learning rules read.
//!   A pair's flags carry [`LEARNED`] and [`PICKED`] beyond its low byte.
//! - **Opening** ([`TreeScreen::load`]): a domain's tree file gives the
//!   nodes; a missing file leaves one node of the domain's first power. A
//!   pair of this domain is merged with the node it matches (the node's high
//!   nibble wins) and dropped when the tree lacks it. The marks then copy
//!   the pairs, show the first node, show every learnt node, and open a node
//!   whose links all end on learnt nodes (a high nibble of 7 never opens
//!   that way). The test screen starts every node learnt and shown. The
//!   creation screen, with no record, starts the original's two default
//!   pairs.
//! - **The view**: the zoom is a charge from −9 to 5 levels, each level's
//!   factor `0.25` to `2.0`; the opening height over 2300 picks the nearest
//!   level. A middle-button drag pans, the wheel moves the charge by ten
//!   times the wheel step, the left and right keys cycle the bar. A pan and
//!   the zoom ease by the original's springs.
//! - **Choosing** ([`TreeScreen::press`]): a click on a shown node that is
//!   not learnt selects it as the new power to learn when the character has
//!   the techniques it asks for ([`can_learn`]: Concentration's second for a
//!   persistent power, Insight's for a power with a high nibble), and leaves it stored on the record with
//!   [`NEW`]. A click on a learnt node picks or drops it: picking needs the
//!   domain's cost within what is left of the record's focus and every link
//!   end picked; dropping is refused while another picked node links to it.
//!   Closing stores the domain's learnt and picked nodes back into the
//!   record, replacing that domain's pairs.
//! - **The power bar** ([`PowerBar`]): ten slots, the chosen one marked with
//!   bit 20. Loading it from a record copies the slots (clearing bits 23 and
//!   24 and the technique nibble), resets each slot's state and marks a slot
//!   picked when its pair is; the slot's stamp and duration are kept. A
//!   slot's state flashes in over 200 ms, stays lit, then fades over its
//!   duration. A press selects a slot; storing the bar back copies the
//!   words, and a cast store instead moves the chosen bit to the cast slot.
//!   A dropped power fills the slot (and, from another slot, leaves a copy
//!   there), and a drop that misses clears the slot it came from.
//! - **The cast entry** ([`TreeScreen::cast_slot`]) reads the record's slot
//!   (the named one, else the first marked chosen, else the default), ends
//!   a body's control instead of casting while it controls another, refuses
//!   a body whose item flags refuse powers, and hands the power to the
//!   domain's front end: mind, force, energy or displacement. A front end
//!   that answers "not now" queues the slot. A cast that starts stores the
//!   bar back, closes it and flashes the slot, and records the last instant
//!   power (a sustained or persistent power clears it); a refused one plays
//!   the failure sound. The sound and the queue are returned in the
//!   [`CastEntry`]; the in-game screen does not play or retry them yet.
//! - **The draw** ([`TreeScreen::view`]): hands `exa_ui::tree_draw` the
//!   placed nodes, their link curves, the marks, the flash and learn
//!   timers and the bar's slots; the tree pass and the bar are laid out
//!   there. A slot whose power matches a picked node is given the
//!   persistent-pick bit in the view. The link ribbon's spline texture is
//!   not modelled (the ribbon reads as white).
//! - **The view's clock** ([`TreeScreen::advance`]): the springs step in the
//!   draw's 17 ms frames and the pan is shown between the step before the
//!   last and the last by the frame's place in a step; a stretched link is
//!   the drag's own cubic (0.508 or 0.29 of the fixed zoom, a 400 cap on
//!   the tangents).
//! - **The form's windows** ([`TreeScreen::present_window`]): showing the
//!   tree raises a 1000 ms fade whose alpha is `min(2·(1 − (1 −
//!   dt·0.001)²), 1)` and whose inner value, `1 − (1 − dt·0.001)²`, fades
//!   the bar child; hiding cuts both over 256 ms from the closing stamp.
//!   The backdrop takes the first alpha and the content the second; a
//!   domain switch cuts the content out over 256 ms, swaps the domain at
//!   the stamp and fades the new tree in over 1000 ms ([`fade_domain`]
//!   starts it; the wheel that calls it is outside the pack).
//! - **The bar's animation** ([`TreeScreen::open_bar`], [`close_bar`] and
//!   [`step_bar`]): opening loads the bar, clears each slot's flash and
//!   picked bits and runs a 900 ms rise whose height and size are an ease of
//!   the elapsed share (the pitch eases twice); closing runs 900 ms, the
//!   marked slots falling as `1 − f⁴` and the rest as `1 − 1.5f`, then
//!   clears the animation and the slots' flash and picked bits.
//! - **The slot budget** ([`limit_running_powers`]): the highest Concentration
//!   technique of 2 to 4 among the skill's levels, one less without the
//!   focus item, is how many running powers a body may keep; the rest are
//!   told to end.
//!
//! - **Saved powers** ([`validate_saved_power`]): loading a character gives
//!   each saved power the flags of the last record pair it matches and adds
//!   the power with `0x9fff` when it is not the first entry and its first
//!   link's prerequisite is learnt. The save chunk's reader is
//!   `exa_formats::power_tree::read_saved_powers`; no character save loader
//!   calls either yet.
//! - **Granting** ([`grant_power`]): the game's grant routine adds a power
//!   to a record that holds Insight's seventh technique when its node's
//!   prerequisite is learnt. Nothing calls it from the running game yet.
//!
//! Evidence: the tree reader, the open, store, mark, focus, click, drag,
//! key and bar routines, the bar's layout and its state timings, and the
//! record's pair and slot arrays. Where the original calls into the
//! character creator for the final commitment of a [`NEW`] power, this
//! module leaves the pair on the record and exposes [`TreeScreen::commit`].

use exa_formats::characters::Skill;
use exa_formats::power_tree::{PowerNode, PowerTree, learn_marks, node_pixels, place_node};

/// What loading the bar clears from a record slot: bits 23 and 24 and the
/// technique nibble.
const BAR_LOAD_MASK: u32 = 0x0E7F_FFFF;

use super::casting::{ActivePower, CastBody};
use crate::nav_camera::fast_acos;

/// The mind domain's code.
pub const DOMAIN_MIND: u8 = 1;
/// The force domain's code.
pub const DOMAIN_FORCE: u8 = 2;
/// The body domain's code.
pub const DOMAIN_BODY: u8 = 3;
/// The energy domain's code.
pub const DOMAIN_ENERGY: u8 = 4;
/// The light domain's code.
pub const DOMAIN_LIGHT: u8 = 5;
/// The displacement domain's code.
pub const DOMAIN_DISPLACEMENT: u8 = 6;

/// The tree files, by domain code (the original's name table).
pub const DOMAIN_FILES: [&str; 7] = [
    "",
    "pwr_mind.pwr",
    "pwr_force.pwr",
    "pwr_body.pwr",
    "pwr_energy.pwr",
    "pwr_light.pwr",
    "pwr_displace.pwr",
];

/// The bits of a power word.
pub mod word {
    /// Bits 0 to 7 index the power inside its domain.
    pub const INDEX: u32 = 0xff;
    /// Bits 8 to 15 are the domain code.
    pub const DOMAIN: u32 = 0xff00;
    /// The power runs while its cast is held.
    pub const SUSTAINED: u32 = 0x1_0000;
    /// The power takes a persistent slot while it runs.
    pub const PERSISTENT: u32 = 0x2_0000;
    /// Bits 16 and 17 together.
    pub const KIND: u32 = 0x3_0000;
    /// The low nibble of the high byte picks the frame picture.
    pub const FRAME: u32 = 0x0f00_0000;
    /// The high nibble of the high byte is the technique a power requires.
    pub const TECHNIQUE: u32 = 0xf000_0000;
    /// A power whose required technique is this one never opens from its
    /// links.
    pub const UNOPENED: u32 = 0x7000_0000;
    /// The mask a known pair and a node are matched by.
    pub const MATCH: u32 = 0x0e0f_ffff;
    /// The mask a bar slot and a pair are matched by.
    pub const BAR_MATCH: u32 = 0x0f0f_ffff;
    /// The bar slot the bar itself selected.
    pub const CHOSEN: u32 = 0x10_0000;
}

/// A node is reachable: its links are learnt, or it is learnt itself.
pub const SHOWN: u32 = 0x1_0000;
/// A node is learnt (a pair of the record).
pub const LEARNED: u32 = 0x1000;
/// A node is picked: it counts against the record's focus.
pub const PICKED: u32 = 0x4000;
/// A node is the new power the player chose to learn.
pub const NEW: u32 = 0x2000;
/// The flags a stored pair keeps.
pub const PAIR_BITS: u32 = 0x7fff;

/// The domain code of a power word.
pub fn domain_of(power: u32) -> u8 {
    ((power & word::DOMAIN) >> 8) as u8
}

/// The index of a power inside its domain.
pub fn power_index(power: u32) -> u8 {
    (power & word::INDEX) as u8
}

/// Whether a power runs while its cast is held.
pub fn is_sustained(power: u32) -> bool {
    power & word::SUSTAINED != 0
}

/// Whether a power takes a persistent slot.
pub fn is_persistent(power: u32) -> bool {
    power & word::PERSISTENT != 0
}

/// The technique a power requires (0: none).
pub fn technique_of(power: u32) -> u8 {
    ((power & word::TECHNIQUE) >> 28) as u8
}

/// One known power pair: its id and its flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KnownPower {
    pub power: u32,
    pub flags: u32,
}

/// The two skills the tree's learning rules read: Insight (skill 4) gates
/// the techniques of a power's high nibble and Concentration (skill 5) gates
/// persistent powers. Both ask [`Skill::knows`]: any of the levels the
/// skill's points fund (one per 5000) may hold the technique. The original
/// reads the bytes after the points for as many levels as the points give;
/// a skill stores six, so more are never looked at.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkillTiers {
    /// Insight: its techniques open the powers whose word carries a
    /// technique nibble.
    pub insight: Skill,
    /// Concentration: its second technique (Continuity) opens persistent
    /// powers.
    pub concentration: Skill,
}

/// The character's power record: what the tree reads and writes.
#[derive(Debug, Clone, PartialEq)]
pub struct PowerRecord {
    /// The focus the picked powers share (4.0 for a new character).
    pub focus: f32,
    /// The specialist domains packed in the record's word: the first at
    /// bits 16 to 18 and the second at bits 19 to 21; bits 22 and 23 give
    /// the pair's discount.
    pub specialisation: u32,
    /// The known powers, in the original's order.
    pub pairs: Vec<KnownPower>,
    /// The ten power-bar slots.
    pub slots: [u32; 10],
    /// The record wears the focus item.
    pub focus_item: bool,
    /// The learnt technique tiers.
    pub skills: SkillTiers,
}

/// The two default pairs of a tree opened with no record (the original's
/// table of two sixteen-byte entries).
pub const DEFAULT_PAIRS: [KnownPower; 2] = [
    KnownPower {
        power: 0x101,
        flags: LEARNED,
    },
    KnownPower {
        power: 0x0102_0101,
        flags: NEW | 0x800,
    },
];

/// The original's default power bar.
pub const DEFAULT_BAR: [u32; 7] = [
    0x101,
    0x0002_0101,
    0x102,
    0x105,
    0x0001_0106,
    0x107,
    0x20_10B,
];

impl PowerRecord {
    /// A new character's record: 4 focus, the mind and force domains, the
    /// two starting powers and their two bar slots.
    pub fn starter() -> Self {
        Self {
            focus: 4.0,
            specialisation: 0,
            pairs: vec![
                KnownPower {
                    power: 0x101,
                    flags: 0x5fff,
                },
                KnownPower {
                    power: 0x201,
                    flags: 0x5fff,
                },
            ],
            slots: [0x100101, 0x201, 0, 0, 0, 0, 0, 0, 0, 0],
            focus_item: true,
            skills: SkillTiers::default(),
        }
    }

    /// An empty record with the given focus.
    pub fn empty(focus: f32) -> Self {
        Self {
            focus,
            specialisation: 0,
            pairs: Vec::new(),
            slots: [0; 10],
            focus_item: false,
            skills: SkillTiers::default(),
        }
    }

    /// The flags of the pair whose id matches `power` under `mask`.
    pub fn pair_flags(&self, power: u32, mask: u32) -> Option<u32> {
        self.pairs
            .iter()
            .find(|pair| pair.power & mask == power & mask)
            .map(|pair| pair.flags)
    }

    /// Stores a pair: clears the previous [`NEW`] mark from the first pair
    /// that has it, then replaces the pair with the same id or appends it.
    pub fn store_pair(&mut self, power: u32, flags: u32) {
        if flags & NEW != 0
            && let Some(first) = self.pairs.iter_mut().find(|p| p.flags & NEW != 0)
        {
            first.flags &= !NEW;
        }
        if let Some(pair) = self.pairs.iter_mut().find(|pair| pair.power == power) {
            pair.flags = flags;
        } else {
            self.pairs.push(KnownPower { power, flags });
        }
    }

    /// The domain of every pair's power.
    pub fn known_domains(&self) -> [bool; 6] {
        let mut domains = [false; 6];
        for pair in &self.pairs {
            let domain = domain_of(pair.power);
            if (1..=6).contains(&domain) {
                domains[usize::from(domain) - 1] = true;
            }
        }
        domains
    }

    /// The per-domain cost of a picked power.
    pub fn costs(&self) -> [f32; 6] {
        let first = usize::from(((self.specialisation & 0x7_0000) >> 16) as u8);
        let second = usize::from(((self.specialisation & 0x38_0000) >> 19) as u8);
        let mut costs = [4.0; 6];
        match self.specialisation & 0xc0_0000 {
            0x40_0000 => {
                if (1..=6).contains(&first) {
                    costs[first - 1] = 2.0;
                }
                if (1..=6).contains(&second) {
                    costs[second - 1] = 2.0;
                }
            }
            0x80_0000 => {
                if (1..=6).contains(&first) {
                    costs[first - 1] = 1.5;
                }
                if (1..=6).contains(&second) {
                    costs[second - 1] = 1.5;
                }
            }
            0xc0_0000 => {
                if (1..=6).contains(&second) {
                    costs[second - 1] = 3.0;
                }
                if (1..=6).contains(&first) {
                    costs[first - 1] = 1.0;
                }
            }
            _ => {}
        }
        costs
    }
}

/// The domain wheel's values: the focus left over and each domain's picked
/// share.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WheelMarks {
    pub total: f32,
    pub domains: [f32; 6],
}

/// What the tree does with a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeMode {
    /// The in-game tree.
    Play,
    /// A screen the tree does not open in.
    Practice,
    /// Every node starts learnt and shown.
    Test,
    /// The creator's "choose which new power" prompt.
    Create,
}

/// One link of a node: both turns as fractions of a circle and the
/// one-based end.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeLink {
    pub out_turn: f32,
    pub in_turn: f32,
    pub end: u32,
}

impl NodeLink {
    fn read(bytes: &[u8; 16]) -> Self {
        let word = |at: usize| {
            let mut cell = [0u8; 4];
            cell.copy_from_slice(&bytes[at..at + 4]);
            f32::from_le_bytes(cell)
        };
        let mut cell = [0u8; 4];
        cell.copy_from_slice(&bytes[8..12]);
        Self {
            out_turn: word(0),
            in_turn: word(4),
            end: u32::from_le_bytes(cell),
        }
    }
}

/// A sound the tree or the bar plays, with its volume and pitch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TreeSound {
    pub name: &'static str,
    pub volume: f32,
    pub pitch: f32,
}

/// The tree opens.
pub const SHOW_SOUND: TreeSound = TreeSound {
    name: "ThaumShow02",
    volume: 0.25,
    pitch: 1.0,
};
/// An unlearnt power is marked new.
pub const LEARN_SOUND: TreeSound = TreeSound {
    name: "ThaumSel01",
    volume: 0.2,
    pitch: 1.0,
};
/// A bar slot is selected.
pub const SLOT_SELECT_SOUND: TreeSound = TreeSound {
    name: "ThaumSel01",
    volume: 0.1,
    pitch: 0.8,
};
/// A power is dropped onto the bar.
pub const SLOT_SOUND: TreeSound = TreeSound {
    name: "ThaumSlot02",
    volume: 0.1,
    pitch: 1.0,
};
/// A power is picked into the record.
pub const PICK_SOUND: TreeSound = TreeSound {
    name: "ThaumChnl01",
    volume: 0.05,
    pitch: 0.8,
};
/// A power is dropped from the record.
pub const DROP_SOUND: TreeSound = TreeSound {
    name: "ThaumChnl02",
    volume: 0.05,
    pitch: 0.8,
};
/// A mark, a pick or a cast was refused.
pub const FAIL_SOUND: TreeSound = TreeSound {
    name: "ThaumFail01",
    volume: 0.05,
    pitch: 0.8,
};

/// The ids a hover can focus.
pub mod focus {
    /// The low 16 bits are a one-based node.
    pub const NODE: i32 = 0xffff;
    /// The low 16 bits are a one-based bar slot.
    pub const SLOT: i32 = 0x40_0000;
    /// The BEGIN button.
    pub const BEGIN: i32 = 0x200_0000;
    /// The BACK button.
    pub const BACK: i32 = 0x100_0000;
    /// The special-id mask.
    pub const SPECIAL: i32 = 0x300_0000;
    /// The link mask: bits 16 to 30 are a one-based link.
    pub const LINK: i32 = 0x7fff_0000;
    /// An incoming link, not an outgoing one.
    pub const INCOMING: i32 = 0x20_0000;
}

/// The press word's button and kind bits.
pub mod press {
    pub const LEFT: u32 = 1;
    pub const RIGHT: u32 = 2;
    pub const MIDDLE: u32 = 4;
    /// A press event.
    pub const PRESS: u32 = 0x1000;
    /// A click: a release that did not drag.
    pub const CLICK: u32 = 0x2000;
    /// A drag starts.
    pub const DRAG: u32 = 0x4000;
    /// The power-drag payload's kind.
    pub const POWER_DRAG: u32 = 0xAE56_CA01;
    /// What a press answers when nothing else happens.
    pub const HANDLED: u32 = 0x10;
}

/// The tree's drag state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragState {
    Idle,
    /// A middle-button pan.
    Pan,
    /// A node moves.
    MoveNode,
    /// A link's end moves.
    MoveLink,
    /// A link is being drawn to the node under the pointer.
    LinkHover,
    /// A link was drawn to the node under the pointer.
    Link,
}

/// A power dragged out of the tree or the bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PowerDrag {
    pub power: u32,
    /// The node's or slot's flags.
    pub flags: u32,
    /// The third word of the payload: a bar slot's first word, or the
    /// domain a tree power was dragged from. The drop does not read it.
    pub extra: u32,
    /// The kind word the bar's drop handler matches.
    pub kind: u32,
    /// The bar slot it came from, one-based, or 0 from the tree.
    pub origin: usize,
}

/// What a press asked the screen to do.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PressResult {
    pub result: u32,
    /// A button's callback ran.
    pub begin: bool,
    pub back: bool,
    /// A power drag started.
    pub drag: Option<PowerDrag>,
    /// A bar slot was selected.
    pub slot: Option<usize>,
    /// The tree wants opening.
    pub open: bool,
    pub sound: Option<TreeSound>,
}

/// The pan's spring: the point the view is pulled toward, the point it is
/// at, and the point it was at one step ago.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Spring2 {
    /// The point the view eases toward; a drag and the wheel move it.
    pub goal: [f32; 2],
    /// Where the view is: the nodes are placed from this point.
    pub position: [f32; 2],
    /// Where the view was at the previous step.
    pub previous: [f32; 2],
}

/// The zoom's charge eases its shown value by this spring.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoomSpring {
    pub value: f32,
    pub target: f32,
    pub previous: f32,
}

/// A press word with its place and the screen's width.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PressInput {
    pub x: f32,
    pub y: f32,
    pub word: u32,
    pub width: f32,
    pub now: u32,
    /// The screen is hidden: a right press does not open it.
    pub hidden: bool,
}

/// A key with the form's state: the original keeps two bits in one byte,
/// bit 0 for the opening guard and bit 1 for hidden.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyInput {
    pub key: i32,
    pub flags: u32,
    /// Bit 0 of the form byte: the form has a stamp that guards its keys.
    pub guarded: bool,
    /// Bit 1 of the form byte: the screen is hidden.
    pub hidden: bool,
    /// The form's stamp in milliseconds.
    pub stamp: u32,
    pub now: u32,
}

/// What a key did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KeyResult {
    /// The tree should open.
    pub open: bool,
    /// The bar's chosen slot moved.
    pub cycled: bool,
}

/// The power tree screen's state.
#[derive(Debug, Clone)]
pub struct TreeScreen {
    pub domain: u8,
    pub mode: TreeMode,
    /// The file's nodes.
    pub nodes: Vec<PowerNode>,
    /// Each node's runtime marks.
    pub marks: Vec<u32>,
    /// Each link, parsed.
    pub links: Vec<Vec<NodeLink>>,
    /// Each node's place before the view.
    pub base: Vec<[f32; 2]>,
    /// Each node's place in pixels.
    pub placed: Vec<[f32; 2]>,
    /// The spring the pan eases by.
    pub pan: Spring2,
    /// The point the nodes are placed from this frame.
    pub shown_pan: [f32; 2],
    /// The charge level.
    pub charge: f32,
    /// The zoom's spring.
    pub zoom: ZoomSpring,
    /// The zoom in 8.8 fixed point, rounded to even.
    pub zoom_fixed: f32,
    /// The view is dragged, so places are not rounded.
    pub free_view: bool,
    /// The stamp the free view ends.
    pub free_view_stamp: u32,
    /// The dragged place factor about the centre.
    pub factor: f32,
    /// The per-domain picked-power costs.
    pub costs: [f32; 6],
    /// The focus a record gives its picked powers.
    pub focus_total: f32,
    /// The focus left for picked powers.
    pub available: f32,
    pub wheel: WheelMarks,
    /// Which domains the record knows.
    pub known_domains: [bool; 6],
    /// The focused id.
    pub focus: i32,
    /// The last node a focus selected.
    pub last_choice: i32,
    /// The node whose choice was shown.
    pub shown_choice: i32,
    /// The stamp of the shown choice.
    pub choice_stamp: u32,
    /// The node under the pointer, one-based.
    pub node_under: i32,
    /// The node (0-based) a pick or a drop last flashed, or -1.
    pub flash: i32,
    /// When the flash ends.
    pub flash_until: u32,
    /// When another new power may be marked; the node marked last fades
    /// in until then.
    pub learn_until: u32,
    /// The node (one-based) that lost its new mark when another was
    /// marked, or 0; it fades out until `learn_until`.
    pub retired_new: u32,
    /// A refused mark or pick turns its frame red.
    pub denied: bool,
    /// The drag state.
    pub drag: DragState,
    /// The point a drag started from.
    pub drag_origin: [f32; 2],
    /// The pointer point a link is being drawn to.
    pub pointer: [f32; 2],
    /// The point a pan drag started from.
    pub press_point: [f32; 2],
    /// A link's turn while it is dragged.
    pub link_turn: f32,
    /// The node the screen must look at.
    pub hovered: i32,
    /// The quick bar overlay is open.
    pub bar_open: bool,
    /// Every shown node wears its name (the form's label mode, set while a
    /// power is dragged).
    pub labels: bool,
    /// The form's fade flags: bit 0 opening, bit 1 closing.
    pub fade_flags: u8,
    /// The stamp the opening fade ends.
    pub open_at: u32,
    /// The stamp the closing fade ends.
    pub close_at: u32,
    /// The closing fade has run out: the screen is no longer drawn.
    pub faded_out: bool,
    /// The domain switch's fade flags: bit 0 the new tree fades in, bit 1
    /// the old tree fades out.
    pub domain_flags: u8,
    /// The stamp of the domain switch's fade.
    pub domain_at: u32,
    /// The record and domain files are waiting for the switch's fade to run
    /// out; the caller performs it.
    pub switch_due: bool,
    /// The window's fade this frame (the backdrop's alpha).
    pub window_alpha: f32,
    /// The tree content's fade this frame.
    pub content_alpha: f32,
    /// The bar child's fade this frame.
    pub bar_alpha: f32,
    /// The bar's animation flags: bit 1 closing, bit 2 opening.
    pub bar_flags: u8,
    /// The stamp the bar's opening began.
    pub bar_open_at: u32,
    /// The stamp the bar's closing began.
    pub bar_close_at: u32,
    /// The bar's animation this frame.
    pub bar_phase_now: Option<exa_ui::tree_draw::BarPhase>,
    /// The simulated time the view's springs have reached: the draw
    /// advances it in 17 ms steps.
    pub step_at: u32,
}

/// The original's zoom levels' thresholds and gains.
pub fn zoom_gain(charge: f32) -> f32 {
    if charge < 0.0 {
        (4.0 - (-charge).sqrt()) * 0.25
    } else {
        (charge + 5.0) * 0.2
    }
}

/// The charge level nearest to `value`: levels 1 to 5 above one, levels −8
/// to −1 below, and 0 for one, a not-a-number or more than two.
///
/// The thresholds are worked out in double precision and rounded to single,
/// as the original does.
pub fn zoom_level(value: f32) -> i32 {
    if value.is_nan() {
        return 0;
    }
    if value >= 1.0 {
        if value > 1.0 {
            for step in 1..=5 {
                if value < ((step + 5) as f64 * 0.2) as f32 {
                    return step;
                }
            }
        }
        return 0;
    }
    // The first comparison's previous level is a not-a-number: the distance
    // compare is unordered and answers the level just reached, so a value
    // below the first threshold answers −8, not −9.
    let mut previous = f32::NAN;
    for next in -8..=-1 {
        let level = ((4.0 - f64::from(-next).sqrt()) * 0.25) as f32;
        if value < level {
            // Unordered or not farther from this level: this level.
            let farther = (level - value).abs() > (previous - value).abs();
            return if farther { next - 1 } else { next };
        }
        previous = level;
    }
    0
}

impl TreeScreen {
    /// The empty tree: one node of the domain's first power.
    pub fn empty(domain: u8, mode: TreeMode) -> Self {
        let node = PowerNode {
            power: u32::from(domain) << 8 | 1,
            name_text: 0,
            description_text: 0,
            x: 0.0,
            y: 0.0,
            links: Vec::new(),
        };
        Self::from_nodes(domain, mode, vec![node])
    }

    /// A tree from the parsed nodes of a domain's file.
    pub fn from_parsed(domain: u8, mode: TreeMode, tree: &PowerTree) -> Self {
        Self::from_nodes(domain, mode, tree.nodes.clone())
    }

    fn from_nodes(domain: u8, mode: TreeMode, nodes: Vec<PowerNode>) -> Self {
        let links: Vec<Vec<NodeLink>> = nodes
            .iter()
            .map(|node| node.links.iter().map(NodeLink::read).collect())
            .collect();
        let base: Vec<[f32; 2]> = nodes.iter().map(|node| [node.x, node.y]).collect();
        let count = nodes.len();
        Self {
            domain,
            mode,
            nodes,
            marks: vec![0; count],
            links,
            base,
            placed: vec![[0.0; 2]; count],
            pan: Spring2::default(),
            shown_pan: [0.0; 2],
            charge: 0.0,
            zoom: ZoomSpring {
                value: 1.0,
                target: 1.0,
                previous: 1.0,
            },
            zoom_fixed: 256.0,
            free_view: false,
            free_view_stamp: 0,
            factor: 1.0,
            costs: [4.0; 6],
            focus_total: 4.0,
            available: 0.0,
            wheel: WheelMarks::default(),
            known_domains: [false; 6],
            focus: 0,
            last_choice: 0,
            shown_choice: 0,
            choice_stamp: 0,
            node_under: 0,
            flash: -1,
            flash_until: 0,
            learn_until: 0,
            retired_new: 0,
            denied: false,
            drag: DragState::Idle,
            drag_origin: [0.0; 2],
            pointer: [0.0; 2],
            press_point: [0.0; 2],
            link_turn: 0.0,
            hovered: 0,
            bar_open: false,
            labels: false,
            fade_flags: 0,
            open_at: 0,
            close_at: 0,
            faded_out: false,
            domain_flags: 0,
            domain_at: 0,
            switch_due: false,
            window_alpha: 1.0,
            content_alpha: 1.0,
            bar_alpha: 1.0,
            bar_flags: 0,
            bar_open_at: 0,
            bar_close_at: 0,
            bar_phase_now: Some(exa_ui::tree_draw::BarPhase::Steady),
            step_at: 0,
        }
    }

    /// Loads a domain's tree: marks it from the record (or the defaults),
    /// sets the view from the screen's height, and reads the record's
    /// costs. A missing or malformed file leaves the empty tree.
    pub fn load(
        domain: u8,
        mode: TreeMode,
        record: Option<&PowerRecord>,
        bytes: Option<&[u8]>,
        width: f32,
        height: f32,
    ) -> Self {
        let mut screen = match bytes {
            Some(bytes) => match exa_formats::power_tree::read_power_tree(bytes) {
                Ok(tree) => Self::from_parsed(domain, mode, &tree),
                Err(_) => Self::empty(domain, mode),
            },
            None => Self::empty(domain, mode),
        };
        match (mode, record) {
            (TreeMode::Test, _) => screen.marks.fill(SHOWN | LEARNED),
            (_, Some(record)) => {
                let mut pairs = record.pairs.clone();
                filter_known(&mut pairs, &screen.nodes, domain);
                screen.apply_marks(&pairs);
                screen.read_record(record);
            }
            (_, None) => {
                screen.apply_marks(&DEFAULT_PAIRS);
                screen.costs = [4.0; 6];
                screen.focus_total = 48.0;
                screen.available = 48.0;
                screen.wheel.total = 48.0;
            }
        }
        // The level comes from the whole-pixel height over 2300, worked out
        // in double precision; the width is halved as an integer.
        let level = zoom_level((f64::from(height.trunc()) * (1.0 / 2300.0)) as f32);
        screen.charge = level as f32;
        let gain = zoom_gain(screen.charge);
        screen.zoom = ZoomSpring {
            value: gain,
            target: gain,
            previous: gain,
        };
        let centre = (width as i64 / 2) as f32;
        let seed = [centre, height - 250.0 * gain];
        screen.pan = Spring2 {
            goal: seed,
            position: seed,
            previous: seed,
        };
        screen.shown_pan = seed;
        screen.refresh_placement();
        // The view the first placement shows: the centre, and the height
        // less the rounded zoom and a hundred pixels, whole numbers.
        screen.shown_pan = [
            centre,
            (height as i64 - screen.zoom_fixed as i64 - 100) as f32,
        ];
        screen.refresh_placement();
        screen.focus = 0;
        screen
    }

    /// Loads the record's costs, its known domains and its picked marks.
    pub fn read_record(&mut self, record: &PowerRecord) {
        self.costs = record.costs();
        self.focus_total = record.focus;
        self.known_domains = record.known_domains();
        self.refresh_wheel(&record.pairs);
    }

    /// Applies the known pairs and opens the nodes their links reach.
    pub fn apply_marks(&mut self, pairs: &[KnownPower]) {
        let known: Vec<(u32, u32)> = pairs.iter().map(|pair| (pair.power, pair.flags)).collect();
        self.marks = learn_marks(&self.nodes, &known);
    }

    /// The record's remaining focus recomputed from its picked pairs, and
    /// the wheel's domain shares.
    pub fn refresh_wheel(&mut self, pairs: &[KnownPower]) {
        self.available = self.record_focus(pairs);
        let mut wheel = WheelMarks::default();
        for pair in pairs {
            if pair.flags & PICKED == 0 {
                continue;
            }
            let domain = domain_of(pair.power);
            if (1..=6).contains(&domain) {
                wheel.domains[usize::from(domain) - 1] += 0.0625;
            }
        }
        wheel.total = self.available * 0.020_833_334;
        for share in &mut wheel.domains {
            if 1.0 < *share {
                *share = 1.0;
            }
        }
        if 0.0 < wheel.total || wheel.total == 0.0 {
            if 1.0 < wheel.total {
                wheel.total = 1.0;
            }
        } else {
            wheel.total = 0.0;
        }
        self.wheel = wheel;
    }

    fn record_focus(&self, pairs: &[KnownPower]) -> f32 {
        let mut available = self.focus_total;
        for pair in pairs {
            if pair.flags & PICKED == 0 {
                continue;
            }
            let domain = domain_of(pair.power);
            if (1..=6).contains(&domain) {
                available -= self.costs[usize::from(domain) - 1];
            }
        }
        available
    }

    /// The node's one-based place at `x`,`y`, or 0.
    pub fn node_at(&self, x: f32, y: f32) -> usize {
        let radius = self.zoom.value * 0.5;
        for (index, node) in self.nodes.iter().enumerate() {
            let dx = x - node.x;
            let dy = y - node.y;
            if dy * dy + dx * dx < radius * radius {
                return index + 1;
            }
        }
        0
    }

    /// The focused node's index (0-based).
    pub fn node_index(&self) -> Option<usize> {
        let node = self.node_under;
        if node > 0 && node as usize <= self.nodes.len() {
            Some(node as usize - 1)
        } else {
            None
        }
    }

    /// Shows the pan at `alpha` of the way from its previous point to its
    /// point (the original blends the last two steps by the frame's share
    /// of a step; one is the end of a step) and places the nodes from it.
    pub fn present(&mut self, alpha: f32) {
        self.shown_pan = lerp2(self.pan.previous, self.pan.position, alpha);
        self.refresh_placement();
    }

    /// Places the nodes at ten times their place times the zoom plus the
    /// shown pan, rounded to even unless the view is dragged.
    pub fn refresh_placement(&mut self) {
        self.zoom_fixed = (self.zoom.value * 256.0).round_ties_even();
        for index in 0..self.nodes.len() {
            let point = node_pixels(
                self.base[index][0],
                self.base[index][1],
                self.zoom.value,
                self.shown_pan[0],
                self.shown_pan[1],
                !self.free_view,
            );
            self.placed[index] = place_node(point, 0.0, 0.0, self.factor);
        }
    }

    /// Steps the charge by `elapsed` units of a hundred-and-twentieth of a
    /// second and derives the zoom target. A value above five clamps to
    /// five; a value below minus nine to minus nine; a not-a-number clamps
    /// to five.
    pub fn step_charge(&mut self, elapsed: f32, now: u32) {
        let mut charge = (f64::from(self.charge) + f64::from(elapsed) * (1.0 / 120.0)) as f32;
        if charge <= 5.0 {
            if charge < -9.0 {
                charge = -9.0;
            }
        } else if !charge.is_nan() {
            // A not-a-number charge skips both clamps and stays not a
            // number, as the original's unordered compares do.
            charge = 5.0;
        }
        self.charge = charge;
        self.zoom.target = zoom_gain(charge);
        if !self.free_view {
            self.free_view = true;
            self.free_view_stamp = now;
        }
    }

    /// The charge moves by `steps` wheel steps of a tenth of a level.
    pub fn zoom_by(&mut self, steps: f32, now: u32) {
        self.step_charge(steps * 10.0, now);
    }

    /// The view's step, in milliseconds (the original's 0x11).
    pub const STEP_MS: u32 = 17;

    /// Advances the pan and zoom springs to `now` in 17 ms steps, then
    /// shows the pan blended from the step before the last to the last by
    /// the frame's place inside a step, as the draw does. The first call or
    /// an opening sets the step clock.
    pub fn advance(&mut self, now: u32) {
        if self.step_at == 0 {
            self.step_at = now;
        }
        while self.step_at < now {
            self.step_at = self.step_at.wrapping_add(Self::STEP_MS);
            if !self.settle() {
                self.step_at = now;
                break;
            }
        }
        let overshoot = self.step_at.wrapping_sub(now) as f32;
        self.present(1.0 - overshoot * 0.058_823_529);
    }

    /// Eases the pan and the zoom. Returns whether anything moved.
    pub fn settle(&mut self) -> bool {
        let moved_pan = step_pan(&mut self.pan);
        let moved_zoom = step_zoom(&mut self.zoom);
        self.present(1.0);
        moved_pan || moved_zoom
    }

    /// Reads a focus id: a plain node selects, and a bar slot or a special
    /// id does not. Returns 3 when the selection changed.
    pub fn set_focus(&mut self, id: i32, now: u32) -> i32 {
        if id < 1 && self.focus < 1 {
            return 0;
        }
        if self.mode == TreeMode::Create && (special(id) || special(self.focus)) {
            self.focus = id;
            return 3;
        }
        let candidate = if id & focus::SLOT != 0 {
            0
        } else if id > 0 && id < 0xffff {
            let index = id as usize - 1;
            // Only a node that has a name text can be chosen.
            if self
                .nodes
                .get(index)
                .is_some_and(|node| node.name_text != 0)
            {
                id & focus::NODE
            } else {
                0
            }
        } else {
            if self.focus > 0 && self.focus & focus::SPECIAL != 0 {
                return 3;
            }
            0
        };
        self.focus = id;
        if candidate == self.last_choice {
            return 0;
        }
        if candidate != self.shown_choice {
            self.last_choice = candidate;
            if candidate != 0 {
                self.shown_choice = candidate;
            }
            self.choice_stamp = now;
            return 3;
        }
        0
    }

    /// Handles a press word. `record` is the tree's character record; `bar`
    /// is the power bar. A click on a bar slot selects the slot the pointer
    /// went down on, so a press comes before its click.
    pub fn press(
        &mut self,
        input: PressInput,
        record: Option<&mut PowerRecord>,
        bar: &mut PowerBar,
    ) -> PressResult {
        let PressInput {
            word, now, hidden, ..
        } = input;
        let mut out = PressResult {
            result: press::HANDLED,
            ..PressResult::default()
        };
        self.drag = DragState::Idle;
        let id = self.focus;
        if word & press::CLICK != 0 && word & press::LEFT != 0 {
            if id == focus::BACK {
                out.back = true;
                return out;
            }
            if id == focus::BEGIN {
                out.begin = true;
                return out;
            }
            if id > 0 && id & focus::SLOT != 0 {
                let slot = self.node_under - 1;
                if (0..10).contains(&slot) && bar.select(slot as usize) {
                    out.slot = Some(slot as usize);
                    out.sound = Some(SLOT_SELECT_SOUND);
                }
                return out;
            }
        }
        if word & press::MIDDLE != 0 {
            // The drag starts from the pan as shown, in whole pixels
            // (rounded to even), and from the point the button went down.
            self.drag = DragState::Pan;
            self.drag_origin = [
                self.shown_pan[0].round_ties_even(),
                self.shown_pan[1].round_ties_even(),
            ];
            self.press_point = [input.x, input.y];
            out.result = 0x15;
            return out;
        }
        if word & press::RIGHT != 0 {
            // Only the play screen answers a right press: it opens the
            // tree unless the screen is hidden.
            if self.mode == TreeMode::Play {
                out.open = !hidden;
                out.result = 1;
            }
            return out;
        }
        if word & press::LEFT == 0 {
            return out;
        }
        self.node_under = if id > 0 { id & focus::NODE } else { 0 };
        if self.node_under > 0 && word & press::CLICK != 0 {
            if id & focus::LINK != 0 || now < self.learn_until {
                return out;
            }
            if id & focus::SLOT != 0 {
                return out;
            }
            let node = self.node_under as usize - 1;
            if node < self.nodes.len() {
                self.click_node(node, now, record, &mut out);
            }
            return out;
        }
        if word & press::DRAG != 0 && id > 0 {
            self.node_under = id & focus::NODE;
            if self.node_under > 0 {
                self.start_power_drag(id, bar, &mut out);
            }
        }
        out
    }

    /// A click on a node: an unlearnt power that can be learnt is marked
    /// new, a learnt one is picked or dropped.
    fn click_node(
        &mut self,
        node: usize,
        now: u32,
        mut record: Option<&mut PowerRecord>,
        out: &mut PressResult,
    ) {
        if self.marks[node] & LEARNED == 0 {
            let learnable = record
                .as_deref()
                .is_some_and(|record| can_learn(record, self.nodes[node].power));
            if !learnable || self.marks[node] & (NEW | LEARNED) != 0 {
                return;
            }
            // Any other new mark is retired and stored.
            self.retired_new = 0;
            for index in 0..self.nodes.len() {
                if self.marks[index] & NEW != 0 {
                    self.marks[index] &= !NEW;
                    self.retired_new = index as u32 + 1;
                    if let Some(record) = record.as_deref_mut() {
                        record.store_pair(self.nodes[index].power, self.marks[index]);
                    }
                }
            }
            out.sound = Some(LEARN_SOUND);
            self.marks[node] |= NEW;
            if let Some(record) = record {
                record.store_pair(self.nodes[node].power, self.marks[node]);
            }
            self.learn_until = now.wrapping_add(200);
            return;
        }
        self.flash = node as i32;
        self.flash_until = now.wrapping_add(300);
        if self.marks[node] & PICKED != 0 {
            // A picked power another picked power needs stays picked.
            let needed = (0..self.nodes.len()).any(|index| {
                self.marks[index] & PICKED != 0
                    && self.links[index]
                        .iter()
                        .any(|link| link.end as usize == node + 1)
            });
            if needed {
                out.sound = Some(FAIL_SOUND);
                self.denied = true;
                return;
            }
            self.marks[node] &= !PICKED;
            out.sound = Some(DROP_SOUND);
        } else {
            // The cost is the one of the domain the screen shows.
            let cost = match usize::from(self.domain).checked_sub(1) {
                Some(index) if index < self.costs.len() => self.costs[index],
                _ => 0.0,
            };
            // A cost that is not above what is left (or not comparable)
            // passes.
            let short = cost > self.available;
            let unmet = self.links[node].iter().any(|link| {
                let end = link.end as usize;
                end < 1 || end > self.nodes.len() || self.marks[end - 1] & PICKED == 0
            });
            if short || unmet {
                out.sound = Some(FAIL_SOUND);
                self.denied = true;
                return;
            }
            self.marks[node] |= PICKED;
            out.sound = Some(PICK_SOUND);
        }
        if let Some(record) = record {
            record.store_pair(self.nodes[node].power, self.marks[node]);
            self.read_record(record);
        }
    }

    /// A drag that starts on a bar slot or a learnt node carries its
    /// power.
    fn start_power_drag(&mut self, id: i32, bar: &PowerBar, out: &mut PressResult) {
        let at = self.node_under as usize - 1;
        if id & focus::SLOT != 0 {
            if let Some(entry) = bar.slots.get(at).filter(|entry| entry.word != 0) {
                out.result = 0x105;
                out.drag = Some(PowerDrag {
                    power: entry.word,
                    flags: entry.state,
                    extra: entry.extra,
                    kind: press::POWER_DRAG,
                    origin: self.node_under as usize,
                });
            }
        } else if let Some(node) = self.nodes.get(at).filter(|_| self.marks[at] & LEARNED != 0) {
            out.result = 0x105;
            out.drag = Some(PowerDrag {
                power: node.power,
                // The original leaves this word as the heap held it; the
                // preview frame reads its picked bit, so the marks fill it.
                flags: self.marks[at],
                extra: u32::from(self.domain),
                kind: press::POWER_DRAG,
                origin: 0,
            });
        }
    }

    /// Moves a drag to `(x, y)`. A pan moves the goal by the whole pixels
    /// the pointer has gone since the button went down.
    pub fn drag_move(&mut self, x: f32, y: f32, now: u32) -> u32 {
        match self.drag {
            DragState::Pan => {
                self.pan.goal = [
                    (self.drag_origin[0] as i64 + (x as i64 - self.press_point[0] as i64)) as f32,
                    (self.drag_origin[1] as i64 + (y as i64 - self.press_point[1] as i64)) as f32,
                ];
                if !self.free_view {
                    self.free_view = true;
                    self.free_view_stamp = now;
                }
                3
            }
            DragState::MoveNode | DragState::MoveLink => 3,
            DragState::LinkHover | DragState::Link => {
                let Some(node) = self.node_under.checked_sub(1) else {
                    return 3;
                };
                let node = node as usize;
                let centre = [self.nodes[node].x, self.nodes[node].y];
                let dx = x - centre[0];
                let dy = y - centre[1];
                let reach = self.zoom.value * 0.55;
                if reach * reach <= dy * dy + dx * dx {
                    if self.drag != DragState::Link {
                        self.drag = DragState::Link;
                        self.link_turn =
                            link_angle(centre[0], centre[1], x, y) / std::f32::consts::TAU;
                    }
                    self.pointer = [x, y];
                } else {
                    self.drag = DragState::LinkHover;
                }
                3
            }
            DragState::Idle => 0,
        }
    }

    /// The curve of the link being dragged out of the focused node, as the
    /// original draws it while the drag is live: a 30-sample cubic from the
    /// node's edge to the pointer, its start along the drag's turn by
    /// `(0.508 − zoom·0.003)·zoom_fixed` (0.29 when the power is framed),
    /// and both tangents scaled by the distance, held to 400. The curve ends
    /// a thirtieth of the far tangent past the pointer. The tangent
    /// convention (the far one pointing back at the node) is inferred from
    /// the engine's vector calls: a labelled guess.
    pub fn drag_link_points(&self) -> Option<Vec<[f32; 2]>> {
        if !matches!(self.drag, DragState::LinkHover | DragState::Link) {
            return None;
        }
        let index = self.node_under.checked_sub(1)? as usize;
        let node = self.nodes.get(index)?;
        let (sine, cosine) = (self.link_turn * std::f32::consts::TAU).sin_cos();
        let base = if node.power & word::FRAME == 0 {
            0.508
        } else {
            0.29
        };
        let reach =
            ((base - f64::from(self.zoom.value) * 0.003) * f64::from(self.zoom_fixed)) as f32;
        let start = [node.x + sine * reach, node.y + cosine * reach];
        let pointer = self.pointer;
        let delta = [pointer[0] - start[0], pointer[1] - start[1]];
        let length = (delta[0] * delta[0] + delta[1] * delta[1]).sqrt();
        let capped = if length >= 400.0 { 400.0 } else { length };
        let unit = if length == 0.0 {
            [0.0, 0.0]
        } else {
            [delta[0] / length, delta[1] / length]
        };
        let out = [
            sine * capped * self.zoom.value,
            cosine * capped * self.zoom.value,
        ];
        let in_ = [
            -unit[0] * capped * self.zoom.value,
            -unit[1] * capped * self.zoom.value,
        ];
        let mut points = Vec::with_capacity(32);
        points.push(start);
        for step in 1..=30 {
            let t = step as f32 / 30.0;
            points.push(exa_formats::power_tree::cubic_point(
                start, pointer, out, in_, t,
            ));
        }
        points.push([
            pointer[0] + in_[0] * 0.033_333_334,
            pointer[1] + in_[1] * 0.033_333_334,
        ]);
        Some(points)
    }

    /// Ends a drag: a link dropped on a node is added to the focused node.
    /// Returns the released pointer and whether a link was added.
    pub fn release_drag(&mut self, x: f32, y: f32) -> bool {
        let mut added = false;
        if self.drag == DragState::Link {
            let end = self.node_at(x, y);
            if end > 0
                && let Some(node) = self.node_under.checked_sub(1)
            {
                {
                    let node = node as usize;
                    let turn = link_angle(self.nodes[end - 1].x, self.nodes[end - 1].y, x, y)
                        / std::f32::consts::TAU;
                    let link = NodeLink {
                        out_turn: self.link_turn,
                        in_turn: turn,
                        end: end as u32,
                    };
                    self.links[node].push(link);
                    let mut raw = [0u8; 16];
                    raw[0..4].copy_from_slice(&link.out_turn.to_le_bytes());
                    raw[4..8].copy_from_slice(&link.in_turn.to_le_bytes());
                    raw[8..12].copy_from_slice(&link.end.to_le_bytes());
                    self.nodes[node].links.push(raw);
                    added = true;
                }
            }
        }
        self.drag = DragState::Idle;
        self.node_under = 0;
        added
    }

    /// A wheel or a paging event. `flags` bit 0x10 pans by `dx`,`dy`; bit
    /// 0x20 moves the charge by ten times the wheel step. In the creation
    /// prompt 0x200 is BACK and 0x100 is BEGIN; elsewhere either opens the
    /// tree.
    pub fn wheel(&mut self, flags: u32, dx: f32, dy: f32, now: u32) -> PressResult {
        let mut out = PressResult::default();
        if flags & 0x10 != 0 {
            let step = self.zoom.value + 2.0;
            self.pan.goal[0] -= step * dx * 4.0;
            self.pan.goal[1] -= step * dy * 4.0;
            if !self.free_view {
                self.free_view = true;
                self.free_view_stamp = now;
            }
            out.result = 3;
        }
        if flags & 0x20 != 0 {
            let steps = (-10.0 * dy).round_ties_even();
            self.step_charge(steps, now);
            out.result = 3;
        }
        if self.mode == TreeMode::Create {
            if flags == 0x200 {
                out.back = true;
            } else if flags == 0x100 {
                out.begin = true;
            }
        } else if flags & 0x300 != 0 {
            out.open = true;
        }
        out
    }

    /// The bar cycle on the right (0x27) and left (0x25) keys, and the
    /// tree opening on the key that is neither. A guarded form ignores keys
    /// until 950 ms before its stamp has passed (signed, in 64 bits: a stamp
    /// under 950 never guards); a hidden form does not open. The original
    /// hands a screen that is not the play screen to the base form's key
    /// handler, which is not modelled.
    pub fn key(
        &mut self,
        input: KeyInput,
        bar: &mut PowerBar,
        record: Option<&mut PowerRecord>,
    ) -> KeyResult {
        let KeyInput {
            key,
            flags,
            guarded,
            hidden,
            stamp,
            now,
        } = input;
        let mut out = KeyResult::default();
        if self.mode != TreeMode::Play {
            return out;
        }
        if guarded && i64::from(stamp) - 0x3b6 > i64::from(now) {
            return out;
        }
        if flags & 2 == 0 {
            if key == 0x27 {
                out.cycled = bar.cycle(1, record);
                return out;
            }
            if key == 0x25 {
                out.cycled = bar.cycle(-1, record);
                return out;
            }
        }
        if !hidden {
            out.open = true;
        }
        out
    }

    /// Stores this domain's learnt and picked nodes into the record,
    /// replacing the domain's pairs.
    pub fn store_domain(&self, record: &mut PowerRecord) {
        record
            .pairs
            .retain(|pair| domain_of(pair.power) != self.domain);
        for (index, node) in self.nodes.iter().enumerate() {
            if self.marks[index] & 0x3fff != 0 {
                record.pairs.push(KnownPower {
                    power: node.power,
                    flags: self.marks[index] & PAIR_BITS,
                });
            }
        }
        // The original logs a bad pair's data pointer and clears it; an
        // Exarust pair has no data pointer.
    }

    /// The pair of the node the creation prompt marked, ready for the
    /// creator's commitment: the original's caller (the character screen)
    /// turns a NEW pair into a learnt and picked one. A labelled guess.
    pub fn commit(&mut self, record: &mut PowerRecord) -> Option<KnownPower> {
        let index = self.marks.iter().position(|mark| mark & NEW != 0)?;
        self.marks[index] = (self.marks[index] | LEARNED | PICKED) & !NEW;
        let power = self.nodes[index].power;
        record.store_pair(power, self.marks[index] & PAIR_BITS);
        record
            .pairs
            .iter()
            .find(|pair| pair.power == power)
            .copied()
    }

    /// The power bar's slot index of the focus (0-based).
    pub fn focused_slot(&self) -> Option<usize> {
        if self.focus > 0 && self.focus & focus::SLOT != 0 {
            let slot = (self.focus & focus::NODE) as usize;
            if (1..=10).contains(&slot) {
                return Some(slot - 1);
            }
        }
        None
    }
}

impl TreeScreen {
    /// The tooltip's name and description texts of a node.
    pub fn node_text<'a>(&self, index: usize, texts: &'a [Vec<u8>]) -> (&'a [u8], &'a [u8]) {
        let empty: &[u8] = &[];
        let Some(node) = self.nodes.get(index) else {
            return (empty, empty);
        };
        let name = texts
            .get(node.name_text as usize)
            .map(Vec::as_slice)
            .unwrap_or(empty);
        let description = texts
            .get(node.description_text as usize)
            .map(Vec::as_slice)
            .unwrap_or(empty);
        (name, description)
    }

    /// The hover id at `x`,`y`: a bar slot (`0x40_0000 | slot`), the
    /// creation prompt's BACK and BEGIN buttons, a node, or a link of a
    /// node (`0x200_0000` for an incoming link). A hidden screen hovers
    /// nothing.
    pub fn hover(&mut self, x: f32, y: f32, width: f32, height: f32, hidden: bool) -> i32 {
        if hidden {
            return 0;
        }
        // The pointer and the form's size are whole pixels.
        let (x, y) = (x as i32, y as i32);
        let (width, height) = (width as i32, height as i32);
        if self.mode == TreeMode::Create {
            if height - 9 * 4 < y && y < height - 2 * 4 {
                if x < 37 * 4 {
                    if 5 * 4 < x {
                        return focus::BACK;
                    }
                } else if width - 37 * 4 < x && x < width - 5 * 4 {
                    return focus::BEGIN;
                }
            }
        } else if let Some(slot) = bar_slot_at(x as f32, y as f32, width as f32, false, 4.0) {
            return focus::SLOT | (slot as i32 + 1);
        }
        let (x, y) = (x as f32, y as f32);
        for index in 0..self.nodes.len() {
            if self.marks[index] & SHOWN == 0 {
                continue;
            }
            let node = &self.nodes[index];
            let dx = x - node.x;
            let dy = y - node.y;
            let distance = f64::from(dy * dy + dx * dx);
            // A framed node's circle is half the size of the others'.
            let radius = if node.power & word::FRAME == 0 {
                self.zoom_fixed
            } else {
                self.zoom_fixed * 0.5
            };
            let core = f64::from(radius) * 0.45;
            if core * core > distance {
                return index as i32 + 1;
            }
            let reach = f64::from(radius) * 0.55 + 10.0;
            if reach * reach > distance {
                let turn = link_angle(node.x, node.y, x, y) / std::f32::consts::TAU;
                let near = |other: f32| f64::from((turn - other).abs()) < 0.01;
                for (at, link) in self.links[index].iter().enumerate() {
                    if near(link.out_turn) {
                        return ((at as i32 + 1) << 16) | (index as i32 + 1);
                    }
                }
                // An incoming link is named by the node it leaves from.
                for (owner, links) in self.links.iter().enumerate() {
                    for (at, link) in links.iter().enumerate() {
                        if link.end as usize == index + 1 && near(link.in_turn) {
                            return ((at as i32 + 1) << 16) | (owner as i32 + 1) | focus::INCOMING;
                        }
                    }
                }
            }
        }
        0
    }

    /// Reads the record's ten slots into the bar without their high marks,
    /// as the bar's first showing does.
    pub fn fill_bar(&self, bar: &mut PowerBar, record: &PowerRecord) {
        for (slot, word) in bar.slots.iter_mut().zip(record.slots) {
            slot.word = word;
        }
    }

    /// Marks each node from a bar slot with the same power. The original has
    /// no such pass: its callback hands a running power object the state of
    /// the first slot holding exactly its id, and the tree's nodes take
    /// their picked mark from the record's pairs. This keeps the two
    /// views of the picked mark in step when the bar is opened.
    pub fn apply_bar_marks(&mut self, bar: &PowerBar) {
        for (index, node) in self.nodes.iter().enumerate() {
            for slot in &bar.slots {
                if slot.word != 0 && slot.word & word::MATCH == node.power & word::MATCH {
                    // The bar carries only the picked mark; the node keeps
                    // its learnt and shown marks.
                    self.marks[index] = (self.marks[index] & !PICKED) | (slot.state & PICKED);
                    break;
                }
            }
        }
    }

    /// Opens or closes the quick bar: opening loads it from the record and
    /// re-marks the nodes; closing stores it. Returns whether it is open.
    pub fn toggle_bar(&mut self, bar: &mut PowerBar, record: &mut PowerRecord, now: u32) -> bool {
        if self.bar_open {
            bar.store(record, false);
            self.close_bar(now);
        } else {
            self.open_bar(bar, record, now);
            self.apply_bar_marks(bar);
        }
        self.bar_open = !self.bar_open;
        self.bar_open
    }

    /// Closes the screen: stores the domain's learnt and picked nodes into
    /// the record.
    pub fn close(&mut self, record: &mut PowerRecord) {
        self.store_domain(record);
        self.bar_open = false;
    }

    /// Leaves the screen: the open bar is stored back into the record
    /// first, and the overlay closes.
    pub fn leave(&mut self, bar: &mut PowerBar, record: &mut PowerRecord) {
        if self.bar_open {
            bar.store(record, false);
        }
        self.close(record);
    }

    /// The cast entry. It takes the named slot (or the chosen one when
    /// none is named) and hands its word to the domain's front end through
    /// `dispatch` with the domain and the full word; the Body, Light and
    /// unnamed domains answer −1, as does a body whose item flags refuse
    /// powers. The caller picks the body: the controlled one while a
    /// possession runs (status bit 0x40_0000), else the player's.
    ///
    /// - A body in control of another (status bit 0x20_0000) casts nothing:
    ///   the answer asks the caller to end the control.
    /// - "Not now" (−3) leaves the bar alone and queues the slot for the
    ///   player controls.
    /// - Any other answer stores the open bar (the chosen slot marked in
    ///   the record) and closes it; a started power also rewrites the
    ///   slot's flash from where it stood ([`PowerBar::cast_flash`]).
    /// - A started instant power becomes the body's last instant power; a
    ///   refused one plays the fail sound.
    pub fn cast_slot(
        &mut self,
        record: &mut PowerRecord,
        bar: &mut PowerBar,
        slot: Option<usize>,
        body: &mut CastBody,
        now: u32,
        dispatch: impl FnOnce(u8, u32, &mut CastBody) -> i32,
    ) -> CastEntry {
        let mut out = CastEntry::default();
        if body.status_bits & CONTROLLING != 0 {
            out.end_control = true;
            return out;
        }
        let slot = match slot {
            Some(slot) => slot,
            None => match record
                .slots
                .iter()
                .position(|slot| *slot & word::CHOSEN != 0)
            {
                Some(slot) => slot,
                None => return out,
            },
        };
        if slot > 9 {
            return out;
        }
        let word = record.slots[slot];
        if word & word::INDEX == 0 {
            return out;
        }
        let result = if body.item_flags & ITEM_REFUSES_POWERS != 0 {
            super::casting::code::REFUSED
        } else {
            match domain_of(word) {
                DOMAIN_MIND | DOMAIN_FORCE | DOMAIN_ENERGY | DOMAIN_DISPLACEMENT => {
                    dispatch(domain_of(word), word, body)
                }
                _ => super::casting::code::REFUSED,
            }
        };
        if result == super::casting::code::NOT_NOW {
            out.queued_slot = Some(slot);
            return out;
        }
        if self.bar_open {
            bar.store(record, true);
            self.bar_open = false;
            if result > 0 {
                bar.cast_flash(slot, now);
            }
        }
        if result > 0 {
            body.last_instant = if word & word::KIND == 0 {
                word & super::casting::ID_MASK
            } else {
                0
            };
            out.result = 1;
        } else if result < 0 {
            out.sound = Some(CAST_FAIL_SOUND);
        }
        out
    }
}

/// The status bit of a body that controls another (a possession or the
/// spirit form): it casts nothing from the bar.
pub const CONTROLLING: u32 = 0x20_0000;
/// The item-flag bit that refuses every power the bar casts.
pub const ITEM_REFUSES_POWERS: u32 = 0x10;

/// The cast entry's answer.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CastEntry {
    /// 1 when the power started, else 0.
    pub result: i32,
    /// The slot a "not now" answer queues for the player controls to
    /// retry (they hold it one-based for half a second).
    pub queued_slot: Option<usize>,
    /// The body controls another: the caller ends the control.
    pub end_control: bool,
    pub sound: Option<TreeSound>,
}

/// The sound a refused cast plays.
pub const CAST_FAIL_SOUND: TreeSound = TreeSound {
    name: "ThaumFail01",
    volume: 0.05,
    pitch: 1.0,
};

pub use exa_ui::tree_draw::ease;

/// The angle a point makes about a node's centre, measured from +y and
/// signed by x: the engine's arc cosine of `dy / length`, π at or below −1
/// and 0 above 1, negated when dx is negative. A NaN cosine (a point on the
/// centre) falls through to the arc cosine, as the unordered compares do.
pub fn link_angle(node_x: f32, node_y: f32, x: f32, y: f32) -> f32 {
    let dx = x - node_x;
    let dy = y - node_y;
    let length = (dy * dy + dx * dx).sqrt();
    let cosine = dy / length;
    let angle = if cosine <= -1.0 {
        std::f32::consts::PI
    } else if cosine > 1.0 {
        0.0
    } else {
        fast_acos(cosine)
    };
    if dx < 0.0 { -angle } else { angle }
}

/// Whether the tree accepts a power dragged with `kind`.
pub fn wants_power_drag(kind: u32) -> bool {
    kind == press::POWER_DRAG
}

/// The flags a granted power is added with.
pub const GRANTED_FLAGS: u32 = 0x9fff;

/// The technique (Insight's seventh, Revelation) the grant routine asks of
/// the record before it adds anything. It asks as the record's own
/// technique check does: the first of the five written slots that holds it
/// counts when the points fund that level.
pub const GRANT_TECHNIQUE: u8 = 7;

/// Grants `power` to a record, as the game's grant routine does for the
/// scripts and the game functions that hand a power out.
///
/// The record must hold Insight's technique [`GRANT_TECHNIQUE`]. `nodes`
/// are the power's domain tree. The last node whose word matches `power`
/// under [`word::MATCH`] is the target: it must not be the first node, its
/// first link must name a prerequisite node (one-based, 0 for none), and
/// that prerequisite's flags must carry [`LEARNED`]. A node's flags are the
/// flags of the last known pair whose id, under `0xff0f_ffff`, equals the
/// node's whole word (no pair's flags give 0). The power is then written
/// over the first known pair that matches it under [`word::MATCH`], or
/// appended, with [`GRANTED_FLAGS`]. Returns whether it was written.
///
/// A prerequisite past the node list reads as unlearnt, where the original
/// reads past its array.
pub fn grant_power(
    nodes: &[PowerNode],
    insight: &Skill,
    known: &mut Vec<KnownPower>,
    power: u32,
) -> bool {
    if !insight.has_technique(GRANT_TECHNIQUE) {
        return false;
    }
    let flags_of = |node: &PowerNode| {
        known
            .iter()
            .rev()
            .find(|pair| pair.power & 0xff0f_ffff == node.power)
            .map_or(0, |pair| pair.flags)
    };
    let Some(target) = nodes
        .iter()
        .rposition(|node| node.power & word::MATCH == power)
    else {
        return false;
    };
    if target == 0 {
        return false;
    }
    let prerequisite = nodes[target].links.first().map_or(0, |link| {
        u32::from_le_bytes([link[8], link[9], link[10], link[11]])
    });
    if prerequisite == 0 {
        return false;
    }
    let Some(prerequisite) = nodes.get(prerequisite as usize - 1) else {
        return false;
    };
    if flags_of(prerequisite) & LEARNED == 0 {
        return false;
    }
    let entry = KnownPower {
        power,
        flags: GRANTED_FLAGS,
    };
    match known
        .iter_mut()
        .find(|pair| pair.power & word::MATCH == power)
    {
        Some(pair) => *pair = entry,
        None => known.push(entry),
    }
    true
}

/// The flags a saved power is re-validated with.
pub const SAVED_FLAGS: u32 = 0x9fff;

/// Re-validates one of a saved character's powers against the record, as
/// loading a character does: [`exa_formats::power_tree::read_saved_powers`]
/// gives the list, and the record's known pairs give each entry its flags
/// (the last pair whose word, under `0xff0f_ffff`, equals the entry's power
/// wins). The saved copy of `power` is the last entry whose word, under
/// `0x0e0f_ffff`, equals it; it must not be the first entry, its first
/// link's end must name an entry, and that prerequisite's flags must carry
/// [`LEARNED`]. The power is then added with [`SAVED_FLAGS`] (or the first
/// pair that matches it is replaced). Returns whether it was added.
///
/// A prerequisite past the saved list reads as unlearnt, where the original
/// reads past its array.
pub fn validate_saved_power(
    saved: &[exa_formats::power_tree::SavedPower],
    known: &mut Vec<KnownPower>,
    power: u32,
) -> bool {
    let mut marks = vec![0u32; saved.len()];
    for (index, entry) in saved.iter().enumerate() {
        for pair in known.iter() {
            if pair.power & 0xff0f_ffff == entry.power {
                marks[index] = pair.flags;
            }
        }
    }
    let mut target = None;
    for (index, entry) in saved.iter().enumerate() {
        if entry.power & 0x0e0f_ffff == power {
            target = Some(index);
        }
    }
    let Some(target) = target else {
        return false;
    };
    if target == 0 {
        return false;
    }
    let prerequisite = saved[target].prerequisite;
    if prerequisite == 0 {
        return false;
    }
    let Some(at) = usize::try_from(prerequisite - 1)
        .ok()
        .filter(|at| *at < marks.len())
    else {
        return false;
    };
    if marks[at] & LEARNED == 0 {
        return false;
    }
    let entry = KnownPower {
        power,
        flags: SAVED_FLAGS,
    };
    match known
        .iter_mut()
        .find(|pair| pair.power & 0x0e0f_ffff == power)
    {
        Some(pair) => *pair = entry,
        None => known.push(entry),
    }
    true
}

/// The domain a tree opens in: the last domain when the same character
/// opens it again, else the mind domain.
pub fn open_domain(same_character: bool, last_domain: u8) -> u8 {
    if same_character {
        last_domain
    } else {
        DOMAIN_MIND
    }
}

/// Seeds a record that knows no powers with the two default pairs, as the
/// tree's first opening does. Returns whether it seeded them.
pub fn seed_defaults(record: &mut PowerRecord) -> bool {
    if !record.pairs.is_empty() {
        return false;
    }
    record.pairs.extend_from_slice(&DEFAULT_PAIRS);
    true
}

pub use exa_ui::tree_draw::DOMAIN_COLOURS;

impl TreeScreen {
    /// The draw inputs of the tree at `now`: placed nodes, the curves of
    /// their links, the bar's slot alphas and the hover and mark fades.
    pub fn view(
        &self,
        bar: &PowerBar,
        width: f32,
        height: f32,
        scale: f32,
        now: u32,
    ) -> exa_ui::tree_draw::TreeView {
        use exa_ui::tree_draw as draw;
        let mut nodes = Vec::with_capacity(self.nodes.len());
        for (index, node) in self.nodes.iter().enumerate() {
            nodes.push(draw::NodeView {
                power: node.power,
                x: self.placed[index][0],
                y: self.placed[index][1],
                flags: self.marks[index],
            });
        }
        let mut links = Vec::new();
        for (index, node) in self.nodes.iter().enumerate() {
            for (at, raw) in node.links.iter().enumerate() {
                let link = self.links[index][at];
                let end = link.end as usize;
                if end < 1 || end > self.nodes.len() {
                    continue;
                }
                let Some(curve) = exa_formats::power_tree::tree_link(
                    exa_formats::power_tree::LinkEnd {
                        at: self.placed[index],
                        power: node.power,
                        mark: self.marks[index],
                    },
                    exa_formats::power_tree::LinkEnd {
                        at: self.placed[end - 1],
                        power: self.nodes[end - 1].power,
                        mark: self.marks[end - 1],
                    },
                    raw,
                    self.zoom.value,
                ) else {
                    continue;
                };
                links.push(draw::LinkView {
                    points: curve.points,
                    source_mark: self.marks[index],
                    dest_mark: self.marks[end - 1],
                });
            }
        }
        // The link a drag has out of a node is drawn as it is stretched.
        if let Some(points) = self.drag_link_points() {
            let index = self.node_under as usize - 1;
            links.push(draw::LinkView {
                points,
                source_mark: self.marks.get(index).copied().unwrap_or(0),
                dest_mark: 0,
            });
        }
        let picked: Vec<u32> = self
            .nodes
            .iter()
            .zip(&self.marks)
            .filter(|(_, mark)| **mark & PICKED != 0)
            .map(|(node, _)| node.power & word::MATCH)
            .collect();
        let mut slots = [draw::SlotView::default(); 10];
        for (index, slot) in bar.slots.iter().enumerate() {
            let mut state = slot.state;
            if picked.contains(&(slot.word & word::MATCH)) {
                state |= draw::state::PICKED;
            }
            slots[index] = draw::SlotView {
                word: slot.word,
                state,
                stamp: slot.stamp,
                duration: slot.duration,
            };
        }
        draw::TreeView {
            width,
            height,
            scale,
            window: self.window_alpha,
            content: self.content_alpha,
            bar_alpha: self.bar_alpha,
            zoom: self.zoom.value,
            icon_size: self.zoom_fixed,
            mode: match self.mode {
                TreeMode::Play => draw::ScreenMode::Play,
                TreeMode::Practice => draw::ScreenMode::Practice,
                TreeMode::Test => draw::ScreenMode::Test,
                TreeMode::Create => draw::ScreenMode::Create,
            },
            nodes,
            links,
            bar: slots,
            bar_phase: self.bar_phase_now,
            hovered: self.shown_choice,
            choice_stamp: self.choice_stamp,
            still_hovering: self.last_choice != 0,
            labels: self.labels,
            flash: self.flash,
            flash_until: self.flash_until,
            denied: self.denied,
            learn_until: self.learn_until,
            retired_new: i32::try_from(self.retired_new).unwrap_or(0),
            accent: [255, 250, 230, 100],
            now,
            domain: self.domain,
        }
    }
}

/// What showing or hiding the screen sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShowState {
    /// The form fade flags' new value: bit 0 opening, bit 1 closing.
    pub flags: u8,
    /// The fade's end stamp.
    pub stamp: u32,
    /// The remembered domain (the hide path keeps it; the open path reads
    /// it).
    pub remembered: u8,
}

/// The form's opening fade, in milliseconds.
pub const OPEN_MS: u32 = 1000;
/// The form's closing fade, in milliseconds.
pub const CLOSE_MS: u32 = 256;
/// The bar's opening or closing animation, in milliseconds.
pub const BAR_ANIM_MS: u32 = 900;
/// The form's open-fade flag.
pub const FADING_IN: u8 = 1;
/// The form's close-fade flag.
pub const FADING_OUT: u8 = 2;
/// The domain switch's fade-in flag.
pub const DOMAIN_IN: u8 = 1;
/// The domain switch's fade-out flag.
pub const DOMAIN_OUT: u8 = 2;
/// The bar's closing flag.
pub const BAR_CLOSING: u8 = 1;
/// The bar's opening flag.
pub const BAR_OPENING: u8 = 2;
/// The bar is steady after its opening animation.
pub const BAR_STEADY: u8 = 4;

/// The original's opening ease: `1 − (1 − dt·0.001)²` doubled and held to
/// one, beside the inner `1 − (1 − dt·0.001)` value. The clock subtraction is
/// signed over the two zero-extended 32-bit stamps.
fn open_fade(stamp: u32, now: u32) -> (f32, f32) {
    let left = (u64::from(stamp) as i64 - u64::from(now) as i64) as f64;
    let inner = (1.0 - left * 0.001) as f32;
    let second = 1.0 - inner * inner;
    let mut window = second * 2.0;
    // `COMISS`/`JBE`: an unordered or not-greater value is left alone.
    if window > 1.0 {
        window = 1.0;
    }
    (window, second)
}

impl TreeScreen {
    /// Showing the tree starts its 1000 ms opening fade and remembers the
    /// domain for the next opening.
    pub fn show(&mut self, now: u32) -> ShowState {
        self.fade_flags |= FADING_IN;
        self.open_at = now.wrapping_add(OPEN_MS);
        self.faded_out = false;
        self.step_at = now;
        ShowState {
            flags: self.fade_flags,
            stamp: self.open_at,
            remembered: self.domain,
        }
    }

    /// Whether the closing fade is running: the screen takes no pointer
    /// input any more.
    pub fn closing(&self) -> bool {
        self.fade_flags & FADING_OUT != 0
    }

    /// Hiding the tree starts its 256 ms closing fade; the screen keeps
    /// drawing, thinner each frame, until the fade runs out.
    pub fn hide(&mut self, now: u32) -> ShowState {
        self.fade_flags |= FADING_OUT;
        self.close_at = now.wrapping_add(CLOSE_MS);
        ShowState {
            flags: self.fade_flags,
            stamp: self.close_at,
            remembered: self.domain,
        }
    }

    /// Computes this frame's three fades and the domain switch, as the
    /// draw's head does. The closing fade clears its flag and marks the
    /// screen faded out once its stamp is reached; the opening fade clears
    /// its flag at the same point.
    pub fn present_window(&mut self, now: u32) {
        let mut window = 1.0;
        let mut second = 1.0;
        self.faded_out = false;
        self.switch_due = false;
        if self.fade_flags & FADING_IN != 0 {
            if now.wrapping_sub(self.open_at) >= 0x8000_0000 {
                // now < open_at
                let (raised, inner) = open_fade(self.open_at, now);
                window = raised;
                second = inner;
            } else {
                self.fade_flags &= !FADING_IN;
            }
        }
        if self.fade_flags & FADING_OUT != 0 {
            if now.wrapping_sub(self.close_at) < 0x8000_0000 {
                // now >= close_at
                self.fade_flags &= !FADING_OUT;
                self.window_alpha = 0.0;
                self.content_alpha = 0.0;
                self.bar_alpha = 0.0;
                self.faded_out = true;
                return;
            }
            let left = (u64::from(self.close_at) as i64 - u64::from(now) as i64) as f32;
            window = left * window / 256.0;
        }
        // The bar child's own fade is read before the domain switch reuses
        // the second value.
        let bar_second = second;
        let mut content = window;
        if self.domain_flags & DOMAIN_IN != 0 {
            if now.wrapping_sub(self.domain_at) >= 0x8000_0000 {
                let (raised, _) = open_fade(self.domain_at, now);
                content = raised;
            } else {
                self.domain_flags &= !DOMAIN_IN;
            }
        }
        if self.domain_flags & DOMAIN_OUT != 0 {
            if now.wrapping_sub(self.domain_at) >= 0x8000_0000 {
                let left = (u64::from(self.domain_at) as i64 - u64::from(now) as i64) as f32;
                content = left * content / 256.0;
            } else {
                // The old tree has faded out: switch the domain and start
                // the new tree's fade-in. The caller performs the switch.
                self.domain_flags = DOMAIN_IN;
                self.domain_at = now.wrapping_add(OPEN_MS);
                self.switch_due = true;
                content = 0.0;
            }
        }
        self.window_alpha = window;
        self.content_alpha = content;
        self.bar_alpha = if self.fade_flags & FADING_OUT != 0 {
            window * bar_second
        } else {
            bar_second
        };
    }

    /// Starts the domain switch's 256 ms fade-out. The wheel that turns the
    /// domain is outside this module; its click calls this.
    pub fn fade_domain(&mut self, now: u32) {
        self.domain_flags = DOMAIN_OUT;
        self.domain_at = now.wrapping_add(CLOSE_MS);
    }

    /// Loads the bar from the record and starts its 900 ms opening, as
    /// turning it on does.
    pub fn open_bar(&mut self, bar: &mut PowerBar, record: &PowerRecord, now: u32) {
        bar.load(record);
        self.bar_flags = BAR_OPENING;
        self.bar_open_at = now;
        for slot in &mut bar.slots {
            slot.state &= 0xffff_fff6;
        }
    }

    /// Loads the bar and starts its opening only when no animation runs, as
    /// opening the tree does.
    pub fn open_bar_if_closed(&mut self, bar: &mut PowerBar, record: &PowerRecord, now: u32) {
        if self.bar_flags & (BAR_OPENING | BAR_CLOSING) == 0 {
            self.open_bar(bar, record, now);
        }
    }

    /// Starts the bar's 900 ms closing animation, keeping an opening that
    /// is still running flagged (the closing runs after it).
    pub fn close_bar(&mut self, now: u32) {
        self.bar_flags = (self.bar_flags & BAR_OPENING) | BAR_CLOSING;
        self.bar_close_at = now;
    }

    /// Steps the bar's slots and reads its animation for `now`. The bar
    /// closing to its end clears the slots' flash and picked bits and
    /// answers `None`.
    pub fn step_bar(
        &mut self,
        bar: &mut PowerBar,
        now: u32,
    ) -> Option<exa_ui::tree_draw::BarPhase> {
        use exa_ui::tree_draw::BarPhase;
        for slot in &mut bar.slots {
            slot.step(now);
        }
        let phase = if self.bar_flags & BAR_OPENING != 0 {
            if now.wrapping_sub(self.bar_open_at) < BAR_ANIM_MS {
                Some(BarPhase::Opening {
                    elapsed: now.wrapping_sub(self.bar_open_at),
                })
            } else {
                self.bar_flags = if self.bar_flags & BAR_CLOSING == 0 {
                    BAR_STEADY
                } else {
                    BAR_CLOSING
                };
                None
            }
        } else {
            None
        };
        let phase = phase.or_else(|| {
            if self.bar_flags & BAR_CLOSING != 0 {
                if now.wrapping_sub(self.bar_close_at) >= BAR_ANIM_MS {
                    self.bar_flags = 0;
                    for slot in &mut bar.slots {
                        slot.state &= 0xffff_fff6;
                    }
                    None
                } else {
                    Some(BarPhase::Closing {
                        elapsed: now.wrapping_sub(self.bar_close_at),
                    })
                }
            } else {
                Some(BarPhase::Steady)
            }
        });
        self.bar_phase_now = phase;
        phase
    }

    /// Changes the domain: the current one is stored back into the record
    /// and the new tree is loaded. Returns the old domain.
    pub fn switch_domain(
        &mut self,
        domain: u8,
        record: &mut PowerRecord,
        bytes: Option<&[u8]>,
        width: f32,
        height: f32,
    ) -> u8 {
        let old = self.domain;
        self.store_domain(record);
        *self = TreeScreen::load(domain, self.mode, Some(record), bytes, width, height);
        old
    }
}

/// A focus id is one of the tree's own buttons.
fn special(id: i32) -> bool {
    id == focus::BEGIN || id == focus::BACK
}

/// Keeps only the pairs of `domain` that a tree node matches, and gives
/// each pair the node's high nibble.
pub fn filter_known(pairs: &mut Vec<KnownPower>, nodes: &[PowerNode], domain: u8) {
    pairs.retain_mut(|pair| {
        if domain_of(pair.power) != domain {
            return true;
        }
        match nodes
            .iter()
            .find(|node| node.power & word::MATCH == pair.power & word::MATCH)
        {
            Some(node) => {
                pair.power = (pair.power & 0x0fff_ffff) | (node.power & word::TECHNIQUE);
                true
            }
            None => false,
        }
    });
}

/// Whether the record has the techniques a new power asks for: a persistent
/// power asks for Concentration's technique 2, and a power whose high
/// nibble is set asks for that technique among Insight's.
pub fn can_learn(record: &PowerRecord, power: u32) -> bool {
    if is_persistent(power) && !record.skills.concentration.knows(2) {
        return false;
    }
    let technique = technique_of(power);
    technique == 0 || record.skills.insight.knows(technique)
}

/// Tells the running powers past `slots` to end. Returns false when one
/// was told to.
pub fn limit_running_powers(powers: &mut [ActivePower], slots: i32) -> bool {
    let mut count = 0;
    let mut ok = true;
    for power in powers.iter_mut() {
        if power.id & word::CHOSEN != 0 {
            continue;
        }
        count += 1;
        if slots < count {
            power.state = 2;
            ok = false;
        }
    }
    ok
}

/// One power-bar slot.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BarSlot {
    /// A value the original's drag payload carries and the screen does not
    /// read.
    pub extra: u32,
    pub word: u32,
    /// The display state: bit 0 flashing in, bit 1 lit, bit 2 fading out,
    /// bit 3 picked/in use.
    pub state: u32,
    pub stamp: u32,
    /// How long a fade runs.
    pub duration: u32,
}

/// The bar's display states.
pub use exa_ui::tree_draw::state as bar_state;

impl BarSlot {
    /// The slot's brightness at `now`.
    pub fn alpha(&self, now: u32) -> f32 {
        exa_ui::tree_draw::slot_flash(self.state, self.stamp, self.duration, now)
    }

    /// Moves the state on as the original's draw does: a flash that has run
    /// its 200 ms becomes lit, and a fade that has run its duration ends.
    pub fn step(&mut self, now: u32) {
        if self.state & bar_state::FLASH != 0 {
            if self.stamp.wrapping_add(bar_state::FLASH_MS) < now {
                self.state = (self.state & !bar_state::FLASH) | bar_state::LIT;
            }
        } else if self.state & bar_state::FADE != 0 && self.stamp.wrapping_add(self.duration) < now
        {
            self.state &= !bar_state::FADE;
        }
    }
}

/// The ten-slot power bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PowerBar {
    pub slots: [BarSlot; 10],
    /// The chosen slot, one-based, or 0.
    pub chosen: usize,
}

impl Default for PowerBar {
    fn default() -> Self {
        let mut slots = [BarSlot::default(); 10];
        for (slot, word) in slots.iter_mut().zip(DEFAULT_BAR) {
            slot.word = word;
        }
        Self { slots, chosen: 0 }
    }
}

impl PowerBar {
    /// The bar screen hides (bit 1 of the form's flags) is not modelled;
    /// the bar is open.
    pub fn words(&self) -> [u32; 10] {
        let mut words = [0; 10];
        for (word, slot) in words.iter_mut().zip(&self.slots) {
            *word = slot.word;
        }
        words
    }

    /// Loads the bar from a record: each slot takes the record's word
    /// without bits 23 and 24 and a clear state, keeping the stamp and the
    /// duration it had; the chosen slot is the one with bit 20; each slot
    /// whose power is a picked pair gains the picked mark. Returns the
    /// chosen slot (one-based).
    pub fn load(&mut self, record: &PowerRecord) -> usize {
        self.chosen = 0;
        for index in 0..10 {
            let word = record.slots[index] & BAR_LOAD_MASK;
            let slot = &mut self.slots[index];
            slot.word = word;
            slot.state = 0;
            if word & word::CHOSEN != 0 {
                self.chosen = index + 1;
            }
        }
        // The original also hands each power object the body runs the
        // state of the first slot with its id, through the object's mark
        // method; the active powers here carry no such method, so only the
        // bar's own picked mark is set.
        for index in 0..10 {
            let word = self.slots[index].word;
            if word & word::BAR_MATCH == 0 {
                continue;
            }
            if let Some(flags) = record.pair_flags(word, word::BAR_MATCH) {
                self.slots[index].state |= flags & PICKED;
            }
        }
        self.chosen
    }

    /// The marks the tree's nodes take from the bar: a node whose power is
    /// a slot takes that slot's state.
    pub fn marks(&self) -> Vec<(u32, u32)> {
        self.slots
            .iter()
            .filter(|slot| slot.word != 0)
            .map(|slot| (slot.word, slot.state))
            .collect()
    }

    /// Stores the bar into a record. A cast (`cast`) clears bit 20 on every
    /// slot and sets it on the chosen one; the bar's own close copies the
    /// words as they are, which already carry the chosen bit. The slots'
    /// display states are left alone.
    pub fn store(&self, record: &mut PowerRecord, cast: bool) {
        if !cast {
            record.slots = self.words();
            return;
        }
        for index in 0..10 {
            record.slots[index] = self.slots[index].word & !word::CHOSEN;
        }
        if let Some(slot) = self
            .chosen
            .checked_sub(1)
            .and_then(|chosen| record.slots.get_mut(chosen))
        {
            *slot |= word::CHOSEN;
        }
    }

    /// Selects a slot (0-based) whose word is set: the slot takes the
    /// chosen bit and the previously chosen slot loses it, so selecting
    /// the chosen slot again leaves none chosen. Returns whether the slot
    /// was set.
    pub fn select(&mut self, slot: usize) -> bool {
        if slot > 9 || self.slots[slot].word == 0 {
            return false;
        }
        self.slots[slot].word |= word::CHOSEN;
        let chosen = self.chosen;
        if chosen > 0 {
            self.slots[chosen - 1].word &= !word::CHOSEN;
        }
        self.chosen = if chosen == slot + 1 { 0 } else { slot + 1 };
        true
    }

    /// Cycles the chosen slot by `direction` among the set slots. A chosen
    /// slot's bit 20 moves; the record's matching slot is written too.
    pub fn cycle(&mut self, direction: i32, record: Option<&mut PowerRecord>) -> bool {
        let chosen = self.chosen;
        let mut target = None;
        if direction < 0 {
            if chosen > 1 {
                // From the slot before the chosen one down to the first.
                let mut index = chosen - 1;
                while index > 0 {
                    index -= 1;
                    if self.slots[index].word != 0 {
                        target = Some(index);
                        break;
                    }
                }
            }
        } else {
            // From one past the chosen slot (0 when none is chosen).
            let mut index = chosen as isize - 1;
            loop {
                index += 1;
                if index >= 10 {
                    break;
                }
                if self.slots[index as usize].word != 0 {
                    target = Some(index as usize);
                    break;
                }
            }
        }
        let Some(target) = target else {
            return false;
        };
        self.slots[target].word |= word::CHOSEN;
        if let Some(record) = record {
            record.slots[target] |= word::CHOSEN;
            if chosen > 0 && chosen - 1 != target {
                self.slots[chosen - 1].word &= !word::CHOSEN;
                record.slots[chosen - 1] &= !word::CHOSEN;
            }
        } else if chosen > 0 && chosen - 1 != target {
            self.slots[chosen - 1].word &= !word::CHOSEN;
        }
        self.chosen = target + 1;
        true
    }

    /// Drops a power on the bar at `x`,`y`. A hit fills the slot (keeping
    /// its bits 20 to 22) with the power word and its flags, and the slot's
    /// old entry goes to the slot the power came from. A miss empties the
    /// slot the power came from. Returns whether a slot was hit.
    pub fn drop(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        drag: &PowerDrag,
    ) -> (bool, Option<TreeSound>) {
        if drag.kind != press::POWER_DRAG {
            return (false, None);
        }
        let hit = bar_slot_at(x, y, width, true, 4.0);
        let Some(slot) = hit else {
            if drag.origin > 0
                && let Some(source) = self.slots.get_mut(drag.origin - 1)
            {
                *source = BarSlot::default();
            }
            return (false, None);
        };
        // The slot's old entry moves to the slot the power came from, then
        // the slot takes the power (keeping its bits 20 to 22).
        if drag.origin > 0 && drag.origin <= 10 {
            self.slots[drag.origin - 1] = self.slots[slot];
        }
        let target = &mut self.slots[slot];
        target.word = (target.word & 0x70_0000) | drag.power;
        target.state = drag.flags;
        (true, Some(SLOT_SOUND))
    }

    /// Steps every slot's display state.
    pub fn step(&mut self, now: u32) {
        for slot in &mut self.slots {
            slot.step(now);
        }
    }

    /// Rewrites the flash of a cast slot. A lit slot starts fading out at
    /// once over 200 ms (state fade and mark). Any other flashes in again
    /// from the brightness it showed: that brightness is the flash's
    /// elapsed time times 0.005, or what is left of the fade, worked in
    /// doubles and held in a single, or 0; the flash is back-dated by 200 ms
    /// of it, rounded half to even (a value out of range or not a number
    /// back-dates nothing), and the state gains flash and mark.
    pub fn cast_flash(&mut self, slot: usize, now: u32) {
        let Some(entry) = self.slots.get_mut(slot) else {
            return;
        };
        if entry.state & bar_state::LIT != 0 {
            entry.state = bar_state::FADE | bar_state::MARK;
            entry.stamp = now;
            entry.duration = 200;
            return;
        }
        let elapsed = f64::from(now) - f64::from(entry.stamp);
        let shown = if entry.state & bar_state::FLASH != 0 {
            (elapsed * 0.005) as f32
        } else if entry.state & bar_state::FADE != 0 {
            (1.0 - elapsed / f64::from(entry.duration)) as f32
        } else {
            0.0
        };
        entry.state |= bar_state::FLASH | bar_state::MARK;
        let back = (shown * 200.0).round_ties_even();
        // CVTSS2SI to 64 bits: out of range or not a number is the
        // "integer indefinite" value, whose low half is zero.
        let back = if back.is_finite() && back.abs() < 9.2e18 {
            back as i64
        } else {
            i64::MIN
        };
        entry.stamp = (i64::from(now).wrapping_sub(back)) as u32;
    }
}

/// The bar's layout: ten slots `4 × 17` units apart ending `4 × 9` short of
/// `width`, at `4 × 8` from the top. A slot's hit radius is `4 × 7`, or
/// `4 × 12` on the enlarged row; the enlarged hit takes the nearest slot.
pub fn bar_slot_at(x: f32, y: f32, width: f32, enlarged: bool, scale: f32) -> Option<usize> {
    let mut start = ((width as i64 - (scale as i64) * 17 * 9) >> 1) as f32;
    let top = scale * 8.0;
    let radius = if enlarged { scale * 12.0 } else { scale * 7.0 };
    let mut room = radius * radius;
    let mut found = None;
    for slot in 0..10 {
        let dx = start - x;
        let dy = top - y;
        let distance = dy * dy + dx * dx;
        if distance < room {
            if !enlarged {
                return Some(slot);
            }
            // The enlarged row takes the nearest slot: each later slot
            // must be closer than the best so far.
            room = distance;
            found = Some(slot);
        }
        start += scale * 17.0;
    }
    found
}

/// `a` plus `b − a` times `t`, on both lanes.
fn lerp2(a: [f32; 2], b: [f32; 2], t: f32) -> [f32; 2] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

/// Whether `value` is above `limit`; a not-a-number is not, as an ordered
/// compare answers.
fn exceeds(value: f32, limit: f64) -> bool {
    f64::from(value) > limit
}

/// The squared length of a vector: the x square plus the y square.
fn length_squared(v: [f32; 2]) -> f32 {
    v[0] * v[0] + v[1] * v[1]
}

/// One step of the pan's spring. The view's velocity is the last step's
/// move; the pull is a tenth of the gap to the goal less that velocity,
/// held to a length of four; the velocity takes the pull and, while it is
/// longer than a tenth (squared length above 0.01), the view moves by 0.98
/// of it. A gap of a hundred-billionth (squared) snaps the view to the
/// goal. Returns whether the view moved.
fn step_pan(spring: &mut Spring2) -> bool {
    let gap = [
        spring.goal[0] - spring.position[0],
        spring.goal[1] - spring.position[1],
    ];
    // A not-a-number gap fails the compare and snaps, as the original does.
    if !exceeds(length_squared(gap), 1e-10) {
        spring.position = spring.goal;
        spring.previous = spring.goal;
        return false;
    }
    let mut velocity = [
        spring.position[0] - spring.previous[0],
        spring.position[1] - spring.previous[1],
    ];
    spring.previous = spring.position;
    let pull = [gap[0] * 0.1 - velocity[0], gap[1] * 0.1 - velocity[1]];
    let squared = length_squared(pull);
    if squared < 16.0 {
        velocity = [velocity[0] + pull[0], velocity[1] + pull[1]];
    } else {
        // Longer than four, or not a number: scaled to four.
        let scale = 4.0 / squared.sqrt();
        velocity = [velocity[0] + pull[0] * scale, velocity[1] + pull[1] * scale];
    }
    if exceeds(length_squared(velocity), 0.01) {
        spring.position = [
            spring.position[0] + velocity[0] * 0.98,
            spring.position[1] + velocity[1] * 0.98,
        ];
        return true;
    }
    false
}

/// The zoom's spring, in doubles for the velocity clamp. The pull is
/// held to a tenth; a velocity of 5e-5 or less snaps the zoom to its goal.
fn step_zoom(spring: &mut ZoomSpring) -> bool {
    let gap = spring.target - spring.value;
    // A not-a-number gap fails the compare and snaps.
    if !exceeds(gap.abs(), 1e-10) {
        spring.value = spring.target;
        spring.previous = spring.target;
        return false;
    }
    let velocity = spring.value - spring.previous;
    spring.previous = spring.value;
    let pull = gap * 0.1 - velocity;
    let velocity = if f64::from(pull.abs()) < 0.1 {
        velocity + pull
    } else {
        // The clamped pull is added in double precision and rounded once.
        (0.1 / f64::from(pull.abs()) * f64::from(pull) + f64::from(velocity)) as f32
    };
    if !exceeds(velocity.abs(), 5e-05) {
        spring.value = spring.target;
        spring.previous = spring.target;
        return false;
    }
    spring.value = ((f64::from(velocity) * 0.998) + f64::from(spring.value)) as f32;
    true
}

/// The zoom's gain at a charge: `(4 − √−charge)/4` below zero, otherwise
/// `(charge + 5)/5`.
pub fn charge_gain(charge: f32) -> f32 {
    zoom_gain(charge)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(
        screen: &mut TreeScreen,
        word: u32,
        now: u32,
        record: Option<&mut PowerRecord>,
        bar: &mut PowerBar,
    ) -> PressResult {
        screen.press(
            PressInput {
                x: 0.0,
                y: 0.0,
                word,
                width: 1000.0,
                now,
                hidden: false,
            },
            record,
            bar,
        )
    }

    fn node(power: u32, links: Vec<[u8; 16]>) -> PowerNode {
        PowerNode {
            power,
            name_text: 0,
            description_text: 0,
            x: 0.0,
            y: 0.0,
            links,
        }
    }

    fn skill(points: u16, techniques: [u8; 6]) -> Skill {
        Skill { points, techniques }
    }

    fn link_to(end: u32) -> [u8; 16] {
        let mut link = [0u8; 16];
        link[8..12].copy_from_slice(&end.to_le_bytes());
        link
    }

    #[test]
    fn a_power_word_carries_its_domain_and_kind() {
        assert_eq!(domain_of(0x10203), DOMAIN_FORCE);
        assert_eq!(domain_of(0x10205), DOMAIN_FORCE);
        assert_eq!(domain_of(0x101), DOMAIN_MIND);
        assert_eq!(domain_of(0x20206), DOMAIN_FORCE);
        assert_eq!(domain_of(0x401), DOMAIN_ENERGY);
        assert_eq!(domain_of(0x10601), DOMAIN_DISPLACEMENT);
        assert_eq!(power_index(0x2020b), 0x0b);
        assert!(is_sustained(0x10203));
        assert!(is_persistent(0x20206));
        assert!(!is_sustained(0x201));
        assert_eq!(technique_of(0x7102_0106), 7);
        assert_eq!(technique_of(0x201), 0);
        assert_eq!(DOMAIN_FILES[1], "pwr_mind.pwr");
        assert_eq!(DOMAIN_FILES[6], "pwr_displace.pwr");
        assert_eq!(
            DEFAULT_BAR,
            [0x101, 0x2_0101, 0x102, 0x105, 0x1_0106, 0x107, 0x2_010B]
        );
    }

    #[test]
    fn the_zoom_levels_pick_the_nearest() {
        assert_eq!(zoom_level(0.304_347_83), -8);
        assert_eq!(zoom_level(0.25), -8);
        assert_eq!(zoom_level(0.26), -8);
        assert_eq!(zoom_level(0.30), -8);
        assert_eq!(zoom_level(0.65), -2);
        assert_eq!(zoom_level(0.74), -1);
        assert_eq!(zoom_level(0.74), -1);
        assert_eq!(zoom_level(0.75), 0);
        assert_eq!(zoom_level(1.0), 0);
        assert_eq!(zoom_level(1.01), 1);
        assert_eq!(zoom_level(1.19), 1);
        assert_eq!(zoom_level(1.21), 2);
        assert_eq!(zoom_level(1.99), 5);
        assert_eq!(zoom_level(2.0), 0);
        assert_eq!(zoom_level(3.0), 0);
        assert_eq!(zoom_level(f32::NAN), 0);
        assert_eq!(zoom_gain(-8.0), f32::from_bits(0x3e95_f61a));
        assert_eq!(zoom_gain(0.0), 1.0);
        assert_eq!(zoom_gain(5.0), 2.0);
    }

    #[test]
    fn the_charge_clamps_at_its_ends() {
        let mut tree = TreeScreen::empty(DOMAIN_MIND, TreeMode::Play);
        tree.zoom = ZoomSpring {
            value: 0.5,
            target: 1.0,
            previous: 1.0,
        };
        tree.step_charge(1000.0, 10);
        assert_eq!(tree.charge, 5.0);
        assert_eq!(tree.zoom.target, 2.0);
        tree.step_charge(-100_000.0, 20);
        assert_eq!(tree.charge, -9.0);
        assert_eq!(tree.zoom.target, 0.25);
        // A not-a-number charge skips both clamps and stays not a number.
        tree.step_charge(f32::NAN, 30);
        assert!(tree.charge.is_nan());
        tree.step_charge(0.0, 40);
        assert!(tree.charge.is_nan());
    }

    #[test]
    fn the_springs_ease_toward_their_targets() {
        let mut zoom = ZoomSpring {
            value: 0.5,
            target: 1.0,
            previous: 0.5,
        };
        // The pull is a tenth of the gap, 0.05, under the clamp of a tenth:
        // the velocity takes it whole and the zoom moves by 0.998 of it.
        assert!(step_zoom(&mut zoom));
        assert_eq!(zoom.previous, 0.5);
        assert!((zoom.value - 0.549_9).abs() < 1e-6);
        assert!(step_zoom(&mut zoom));
        // The pan, at rest and ten units from its goal: the pull is one
        // unit, the velocity one, and the view moves 0.98.
        let mut pan = Spring2 {
            goal: [10.0, 0.0],
            position: [0.0, 0.0],
            previous: [0.0, 0.0],
        };
        assert!(step_pan(&mut pan));
        assert!((pan.position[0] - 0.98).abs() < 1e-6);
        assert_eq!(pan.previous, [0.0, 0.0]);
        // Next step: the gap is 9.02, a tenth 0.902, less the 0.98 velocity
        // leaves -0.078; the velocity is 0.902 and the view moves 0.88396.
        assert!(step_pan(&mut pan));
        assert!((pan.position[0] - 1.863_96).abs() < 1e-5);
        assert_eq!(pan.previous[0], 0.98);
        // A far goal pulls by four at most: the pull of 1000 is held to 4.
        let mut far = Spring2 {
            goal: [10_000.0, 0.0],
            position: [0.0, 0.0],
            previous: [0.0, 0.0],
        };
        assert!(step_pan(&mut far));
        assert!((far.position[0] - 3.92).abs() < 1e-5);
        // Next to the goal the view snaps onto it.
        let mut near = Spring2 {
            goal: [1.0, 1.0],
            position: [1.0, 1.0],
            previous: [0.0, 0.0],
        };
        assert!(!step_pan(&mut near));
        assert_eq!(near.previous, [1.0, 1.0]);
        // A not-a-number goal snaps too.
        let mut lost = Spring2 {
            goal: [f32::NAN, 0.0],
            position: [1.0, 1.0],
            previous: [0.0, 0.0],
        };
        assert!(!step_pan(&mut lost));
        assert!(lost.position[0].is_nan());
    }

    #[test]
    fn the_view_advances_in_seventeen_millisecond_steps() {
        let mut screen = TreeScreen::empty(DOMAIN_MIND, TreeMode::Play);
        screen.show(1000);
        screen.pan.position = [0.0, 0.0];
        screen.pan.previous = [0.0, 0.0];
        screen.pan.goal = [10.0, 0.0];
        // A step lands the pan at 0.98 and the frame at the step's end
        // shows it whole: the overshoot is exactly one step.
        screen.advance(1000 + 17);
        assert_eq!(screen.step_at, 1017);
        assert!((screen.pan.position[0] - 0.98).abs() < 1e-6);
        assert!((screen.shown_pan[0] - 0.98).abs() < 1e-6);
        // Twelve milliseconds into the next step the frame shows 5/17 of
        // the way from the previous point to the new one.
        screen.advance(1000 + 17 + 5);
        assert_eq!(screen.step_at, 1000 + 34);
        let alpha = 1.0 - 12.0 * 0.058_823_529;
        let expected =
            screen.pan.previous[0] + (screen.pan.position[0] - screen.pan.previous[0]) * alpha;
        assert_eq!(screen.pan.previous[0], 0.98);
        assert!((screen.shown_pan[0] - expected).abs() < 1e-5);
        // When the springs come to rest the clock snaps to the frame.
        screen.pan.position = screen.pan.goal;
        screen.pan.previous = screen.pan.goal;
        screen.advance(10_000);
        assert_eq!(screen.step_at, 10_000);
        assert_eq!(screen.shown_pan[0], 10.0);
    }

    #[test]
    fn the_bar_lies_ten_slots_across() {
        assert_eq!(bar_slot_at(194.0, 32.0, 1000.0, false, 4.0), Some(0));
        assert_eq!(
            bar_slot_at(194.0 + 68.0 * 9.0, 32.0, 1000.0, false, 4.0),
            Some(9)
        );
        assert_eq!(bar_slot_at(194.0 - 29.0, 32.0, 1000.0, false, 4.0), None);
        assert_eq!(bar_slot_at(194.0 + 20.0, 32.0, 1000.0, false, 4.0), Some(0));
        // The enlarged row reaches 48 px and takes the nearest.
        let far = bar_slot_at(194.0 + 68.0 - 10.0, 32.0, 1000.0, true, 4.0);
        assert_eq!(far, Some(1));
    }

    #[test]
    fn a_new_power_asks_for_its_technique() {
        let mut record = PowerRecord::empty(4.0);
        record.skills.insight = skill(20_000, [0, 0, 0, 1, 0, 0]);
        record.skills.concentration = skill(10_000, [0, 2, 0, 0, 0, 0]);
        assert!(can_learn(&record, 0x1000_0102));
        assert!(!can_learn(&record, 0x2000_0103));
        assert!(can_learn(&record, 0x201));
        record.skills.insight.techniques[3] = 0;
        assert!(!can_learn(&record, 0x1000_0102));
        assert!(can_learn(&record, 0x201));
        record.skills.concentration.techniques[1] = 0;
        // Blast is neither persistent nor tiered: it needs no technique.
        assert!(can_learn(&record, 0x201));
    }

    #[test]
    fn a_store_pair_keeps_one_new_mark() {
        let mut record = PowerRecord::empty(4.0);
        record.store_pair(0x101, LEARNED | NEW);
        record.store_pair(0x102, LEARNED | NEW);
        assert_eq!(
            record.pairs,
            vec![
                KnownPower {
                    power: 0x101,
                    flags: LEARNED
                },
                KnownPower {
                    power: 0x102,
                    flags: LEARNED | NEW
                }
            ]
        );
        record.store_pair(0x101, LEARNED | PICKED);
        assert_eq!(record.pairs[0].flags, LEARNED | PICKED);
    }

    #[test]
    fn a_specialist_domain_costs_less() {
        let mut record = PowerRecord::starter();
        assert_eq!(record.costs(), [4.0; 6]);
        // Mind and force, both discounted to 2.
        record.specialisation = 0x40_0000 | 1 << 16 | 2 << 19;
        let costs = record.costs();
        assert_eq!(costs[0], 2.0);
        assert_eq!(costs[1], 2.0);
        assert_eq!(costs[2], 4.0);
        // Mind and energy both at three halves.
        record.specialisation = 0x80_0000 | 1 << 16 | 4 << 19;
        let costs = record.costs();
        assert_eq!(costs[0], 1.5);
        assert_eq!(costs[3], 1.5);
        // Mind at one, force at three.
        record.specialisation = 0xc0_0000 | 1 << 16 | 2 << 19;
        let costs = record.costs();
        assert_eq!(costs[0], 1.0);
        assert_eq!(costs[1], 3.0);
        assert_eq!(costs[5], 4.0);
    }

    #[test]
    fn known_pairs_keep_only_their_domain_and_take_the_node_nibble() {
        let nodes = [
            node(0x2000_0103, vec![]),
            node(0x102, vec![]),
            node(0x401, vec![]),
        ];
        let mut pairs = vec![
            KnownPower {
                power: 0x1000_0103,
                flags: 0x1000,
            },
            KnownPower {
                power: 0x102,
                flags: 0x4000,
            },
            KnownPower {
                power: 0x401,
                flags: 0x1000,
            },
            KnownPower {
                power: 0x10_0109,
                flags: 0x1000,
            },
        ];
        filter_known(&mut pairs, &nodes, DOMAIN_MIND);
        assert_eq!(
            pairs,
            vec![
                KnownPower {
                    power: 0x2000_0103,
                    flags: 0x1000
                },
                KnownPower {
                    power: 0x102,
                    flags: 0x4000
                },
                KnownPower {
                    power: 0x401,
                    flags: 0x1000
                },
            ]
        );
    }

    #[test]
    fn a_tree_opens_its_learnt_powers() {
        let mut screen = TreeScreen::from_nodes(
            DOMAIN_MIND,
            TreeMode::Play,
            vec![
                node(0x101, vec![]),
                node(0x102, vec![link_to(1)]),
                node(0x103, vec![link_to(2)]),
            ],
        );
        let pairs = [
            KnownPower {
                power: 0x101,
                flags: LEARNED,
            },
            KnownPower {
                power: 0x102,
                flags: LEARNED,
            },
        ];
        screen.apply_marks(&pairs);
        assert_eq!(screen.marks, vec![SHOWN | LEARNED, SHOWN | LEARNED, SHOWN]);
        screen.apply_marks(&[]);
        assert_eq!(screen.marks, vec![SHOWN, 0, 0]);
        screen.mode = TreeMode::Test;
        screen.marks.fill(SHOWN | LEARNED);
        assert_eq!(screen.marks, vec![0x11000; 3]);
    }

    #[test]
    fn the_wheel_reads_the_picked_pairs() {
        let mut record = PowerRecord::empty(4.0);
        record.specialisation = 0xc0_0000 | 1 << 16 | 2 << 19;
        record.pairs = vec![KnownPower {
            power: 0x101,
            flags: LEARNED | PICKED,
        }];
        let mut screen = TreeScreen::empty(DOMAIN_MIND, TreeMode::Play);
        screen.read_record(&record);
        assert_eq!(screen.available, 3.0);
        assert_eq!(screen.wheel.domains[0], 0.0625);
        assert_eq!(screen.wheel.total, 3.0 * 0.020_833_334);
        assert_eq!(
            screen.known_domains,
            [true, false, false, false, false, false]
        );
        record.pairs[0].flags = LEARNED;
        screen.read_record(&record);
        assert_eq!(screen.available, 4.0);
        assert_eq!(screen.wheel.domains[0], 0.0);
    }

    #[test]
    fn a_click_picks_a_learnt_power_when_its_links_are_picked() {
        let mut screen = TreeScreen::from_nodes(
            DOMAIN_MIND,
            TreeMode::Play,
            vec![node(0x101, vec![link_to(2)]), node(0x102, vec![])],
        );
        let mut record = PowerRecord::empty(8.0);
        record.pairs = vec![
            KnownPower {
                power: 0x101,
                flags: LEARNED,
            },
            KnownPower {
                power: 0x102,
                flags: LEARNED,
            },
        ];
        screen.apply_marks(&record.pairs);
        screen.read_record(&record);
        let mut bar = PowerBar::default();
        // The first node's link does not end on a picked node yet.
        screen.set_focus(1, 0);
        let out = press(
            &mut screen,
            press::LEFT | press::CLICK,
            100,
            Some(&mut record),
            &mut bar,
        );
        assert_eq!(out.sound, Some(FAIL_SOUND));
        assert!(screen.denied);
        assert_eq!(screen.marks[0] & PICKED, 0);
        // The second has no links, so it may be picked.
        screen.set_focus(2, 0);
        let out = press(
            &mut screen,
            press::LEFT | press::CLICK,
            200,
            Some(&mut record),
            &mut bar,
        );
        assert_eq!(out.sound, Some(PICK_SOUND));
        assert_eq!(screen.marks[1] & PICKED, PICKED);
        assert_eq!(screen.available, 4.0);
        // Now the first may be picked too.
        screen.set_focus(1, 0);
        let out = press(
            &mut screen,
            press::LEFT | press::CLICK,
            300,
            Some(&mut record),
            &mut bar,
        );
        assert_eq!(out.sound, Some(PICK_SOUND));
        assert_eq!(screen.available, 0.0);
        // The second is the first's prerequisite: dropping it is refused
        // while the first is still picked.
        screen.set_focus(2, 0);
        let out = press(
            &mut screen,
            press::LEFT | press::CLICK,
            400,
            Some(&mut record),
            &mut bar,
        );
        assert_eq!(out.sound, Some(FAIL_SOUND));
        assert_eq!(screen.marks[1] & PICKED, PICKED);
        // Nothing links to the first, so it may be dropped.
        screen.set_focus(1, 0);
        let out = press(
            &mut screen,
            press::LEFT | press::CLICK,
            500,
            Some(&mut record),
            &mut bar,
        );
        assert_eq!(out.sound, Some(DROP_SOUND));
        assert_eq!(screen.marks[0] & PICKED, 0);
        assert_eq!(screen.available, 4.0);
    }

    #[test]
    fn a_press_needs_the_domains_affordable_cost() {
        let mut screen =
            TreeScreen::from_nodes(DOMAIN_MIND, TreeMode::Play, vec![node(0x101, vec![])]);
        let mut record = PowerRecord::empty(3.9);
        record.pairs = vec![KnownPower {
            power: 0x101,
            flags: LEARNED,
        }];
        screen.apply_marks(&record.pairs);
        screen.read_record(&record);
        assert_eq!(screen.available, 3.9);
        screen.set_focus(1, 0);
        let mut bar = PowerBar::default();
        let out = press(
            &mut screen,
            press::LEFT | press::CLICK,
            10,
            Some(&mut record),
            &mut bar,
        );
        assert_eq!(out.sound, Some(FAIL_SOUND));
        assert_eq!(screen.marks[0] & PICKED, 0);
        // A budget exactly equal to the cost passes.
        record.focus = 4.0;
        screen.read_record(&record);
        assert_eq!(screen.available, 4.0);
        let out = press(
            &mut screen,
            press::LEFT | press::CLICK,
            20,
            Some(&mut record),
            &mut bar,
        );
        assert_eq!(out.sound, Some(PICK_SOUND));
        assert_eq!(screen.available, 0.0);
    }

    #[test]
    fn a_new_power_is_marked_for_the_creator() {
        let mut screen = TreeScreen::from_nodes(
            DOMAIN_MIND,
            TreeMode::Create,
            vec![node(0x101, vec![]), node(0x102, vec![link_to(1)])],
        );
        screen.marks = vec![SHOWN | LEARNED, SHOWN];
        let mut record = PowerRecord::empty(4.0);
        record.skills.concentration = skill(10_000, [0, 2, 0, 0, 0, 0]);
        let mut bar = PowerBar::default();
        screen.set_focus(2, 0);
        let out = press(
            &mut screen,
            press::LEFT | press::CLICK,
            100,
            Some(&mut record),
            &mut bar,
        );
        assert_eq!(out.sound, Some(LEARN_SOUND));
        assert_eq!(screen.learn_until, 300);
        // A mark does not flash the node, and nothing was retired.
        assert_eq!(screen.flash, -1);
        assert_eq!(screen.retired_new, 0);
        assert_eq!(screen.marks[1] & NEW, NEW);
        assert_eq!(
            record.pairs,
            vec![KnownPower {
                power: 0x102,
                flags: SHOWN | NEW
            }]
        );
        let committed = screen.commit(&mut record);
        assert_eq!(committed.map(|pair| pair.flags), Some(LEARNED | PICKED));
        assert_eq!(screen.marks[1] & (LEARNED | PICKED), LEARNED | PICKED);
    }

    #[test]
    fn a_second_new_mark_retires_the_first() {
        let mut screen = TreeScreen::from_nodes(
            DOMAIN_MIND,
            TreeMode::Create,
            vec![node(0x101, vec![]), node(0x102, vec![])],
        );
        screen.marks = vec![SHOWN, SHOWN];
        let mut record = PowerRecord::empty(4.0);
        record.skills.concentration = skill(10_000, [0, 2, 0, 0, 0, 0]);
        let mut bar = PowerBar::default();
        screen.set_focus(1, 0);
        press(
            &mut screen,
            press::LEFT | press::CLICK,
            100,
            Some(&mut record),
            &mut bar,
        );
        assert_eq!(screen.marks[0] & NEW, NEW);
        // The mark's fade runs for 200 ms; a click inside it does nothing.
        screen.set_focus(2, 0);
        press(
            &mut screen,
            press::LEFT | press::CLICK,
            250,
            Some(&mut record),
            &mut bar,
        );
        assert_eq!(screen.marks[1] & NEW, 0);
        let out = press(
            &mut screen,
            press::LEFT | press::CLICK,
            300,
            Some(&mut record),
            &mut bar,
        );
        assert_eq!(out.sound, Some(LEARN_SOUND));
        assert_eq!(screen.marks[0] & NEW, 0);
        assert_eq!(screen.marks[1] & NEW, NEW);
        // The first is kept for its fade out, one-based.
        assert_eq!(screen.retired_new, 1);
        assert_eq!(screen.learn_until, 500);
    }

    #[test]
    fn a_pick_flashes_its_node_and_a_refusal_turns_it_red() {
        let mut screen = TreeScreen::from_nodes(
            DOMAIN_MIND,
            TreeMode::Play,
            vec![node(0x101, vec![]), node(0x102, vec![link_to(1)])],
        );
        let mut record = PowerRecord::empty(8.0);
        record.pairs = vec![
            KnownPower {
                power: 0x101,
                flags: LEARNED,
            },
            KnownPower {
                power: 0x102,
                flags: LEARNED,
            },
        ];
        screen.apply_marks(&record.pairs);
        screen.read_record(&record);
        assert_eq!(screen.flash, -1);
        let mut bar = PowerBar::default();
        // The second power needs the first one picked.
        screen.set_focus(2, 0);
        press(
            &mut screen,
            press::LEFT | press::CLICK,
            1000,
            Some(&mut record),
            &mut bar,
        );
        assert!(screen.denied);
        assert_eq!(screen.flash, 1);
        assert_eq!(screen.flash_until, 1300);
    }

    #[test]
    fn a_cost_is_the_one_of_the_domain_shown() {
        // A power of another domain than the screen's is charged the
        // screen's domain cost.
        let mut screen =
            TreeScreen::from_nodes(DOMAIN_FORCE, TreeMode::Play, vec![node(0x101, vec![])]);
        let mut record = PowerRecord::empty(1.0);
        record.pairs = vec![KnownPower {
            power: 0x101,
            flags: LEARNED,
        }];
        screen.apply_marks(&record.pairs);
        screen.read_record(&record);
        screen.costs = [4.0, 0.5, 4.0, 4.0, 4.0, 4.0];
        screen.set_focus(1, 0);
        let mut bar = PowerBar::default();
        let out = press(
            &mut screen,
            press::LEFT | press::CLICK,
            10,
            Some(&mut record),
            &mut bar,
        );
        assert_eq!(out.sound, Some(PICK_SOUND));
    }

    #[test]
    fn a_right_press_opens_only_the_visible_play_screen() {
        let mut bar = PowerBar::default();
        let mut play = TreeScreen::empty(DOMAIN_MIND, TreeMode::Play);
        let out = press(&mut play, press::RIGHT, 10, None, &mut bar);
        assert!(out.open);
        assert_eq!(out.result, 1);
        let hidden = play.press(
            PressInput {
                x: 0.0,
                y: 0.0,
                word: press::RIGHT,
                width: 1000.0,
                now: 10,
                hidden: true,
            },
            None,
            &mut bar,
        );
        assert!(!hidden.open);
        assert_eq!(hidden.result, 1);
        let mut practice = TreeScreen::empty(DOMAIN_MIND, TreeMode::Practice);
        let out = press(&mut practice, press::RIGHT, 10, None, &mut bar);
        assert!(!out.open);
        assert_eq!(out.result, press::HANDLED);
    }

    #[test]
    fn a_slot_click_selects_the_slot_the_press_went_down_on() {
        let mut screen = TreeScreen::empty(DOMAIN_MIND, TreeMode::Play);
        let mut bar = PowerBar::default();
        screen.set_focus(focus::SLOT | 3, 0);
        // The press names the slot; the click reads it back.
        press(&mut screen, press::LEFT | press::PRESS, 10, None, &mut bar);
        assert_eq!(screen.node_under, 3);
        let out = press(&mut screen, press::LEFT | press::CLICK, 20, None, &mut bar);
        assert_eq!(out.slot, Some(2));
        assert_eq!(out.sound, Some(SLOT_SELECT_SOUND));
        assert_eq!(bar.chosen, 3);
        // An empty slot cannot be selected and plays nothing.
        bar.slots[4].word = 0;
        screen.set_focus(focus::SLOT | 5, 0);
        press(&mut screen, press::LEFT | press::PRESS, 30, None, &mut bar);
        let out = press(&mut screen, press::LEFT | press::CLICK, 40, None, &mut bar);
        assert_eq!(out.slot, None);
        assert_eq!(out.sound, None);
    }

    #[test]
    fn a_drag_from_the_tree_names_the_power() {
        let mut screen =
            TreeScreen::from_nodes(DOMAIN_FORCE, TreeMode::Play, vec![node(0x201, vec![])]);
        screen.marks = vec![SHOWN | LEARNED | PICKED];
        let mut bar = PowerBar::default();
        screen.set_focus(1, 0);
        let out = press(&mut screen, press::LEFT | press::DRAG, 10, None, &mut bar);
        assert_eq!(out.result, 0x105);
        assert_eq!(
            out.drag,
            Some(PowerDrag {
                power: 0x201,
                flags: SHOWN | LEARNED | PICKED,
                extra: u32::from(DOMAIN_FORCE),
                kind: press::POWER_DRAG,
                origin: 0
            })
        );
        // Only a learnt node can be dragged.
        screen.marks[0] = SHOWN;
        let out = press(&mut screen, press::LEFT | press::DRAG, 20, None, &mut bar);
        assert!(out.drag.is_none());
    }

    #[test]
    fn a_slot_drop_swaps_and_a_miss_clears() {
        let mut bar = PowerBar::default();
        bar.slots[0] = BarSlot {
            word: 0x101,
            state: 0x4000,
            ..BarSlot::default()
        };
        bar.slots[9] = BarSlot {
            word: 0x103 | 0x20_0000,
            ..BarSlot::default()
        };
        let drag = PowerDrag {
            power: 0x102,
            flags: 0x4000,
            extra: 0,
            kind: press::POWER_DRAG,
            origin: 1,
        };
        let (hit, sound) = bar.drop(194.0 + 68.0 * 9.0, 32.0, 1000.0, &drag);
        assert!(hit);
        assert_eq!(sound, Some(SLOT_SOUND));
        // The slot keeps its bits 20 to 22 and takes the power; its old
        // entry goes to the slot the power came from.
        assert_eq!(bar.slots[9].word, 0x102 | 0x20_0000);
        assert_eq!(bar.slots[9].state, 0x4000);
        assert_eq!(bar.slots[0].word, 0x103 | 0x20_0000);
        let (hit, _) = bar.drop(0.0, 0.0, 1000.0, &drag);
        assert!(!hit);
        assert_eq!(bar.slots[0].word, 0);
    }

    #[test]
    fn the_bar_cycles_and_selects() {
        let mut bar = PowerBar::default();
        assert_eq!(bar.chosen, 0);
        assert!(bar.select(2));
        assert_eq!(bar.chosen, 3);
        assert_eq!(bar.slots[2].word & word::CHOSEN, word::CHOSEN);
        let mut record = PowerRecord::empty(4.0);
        assert!(bar.cycle(1, Some(&mut record)));
        assert_eq!(bar.chosen, 4);
        assert_eq!(record.slots[3] & word::CHOSEN, word::CHOSEN);
        assert_eq!(bar.slots[2].word & word::CHOSEN, 0);
        assert_eq!(record.slots[2] & word::CHOSEN, 0);
        assert!(bar.cycle(-1, None));
        assert_eq!(bar.chosen, 3);
        assert!(!bar.select(9));
        // Selecting the chosen slot again takes the chosen bit off it.
        assert!(bar.select(2));
        assert_eq!(bar.chosen, 0);
        assert_eq!(bar.slots[2].word & word::CHOSEN, 0);
    }

    #[test]
    fn a_slot_flashes_in_then_fades() {
        let mut slot = BarSlot {
            word: 0x101,
            state: 1,
            stamp: 1000,
            duration: 0,
            ..BarSlot::default()
        };
        assert_eq!(slot.alpha(1100), 0.5);
        slot.step(1199);
        assert_eq!(slot.state, 1);
        slot.step(1201);
        assert_eq!(slot.state, 2);
        assert_eq!(slot.alpha(1300), 1.0);
        slot.state = bar_state::FADE;
        slot.stamp = 2000;
        slot.duration = 200;
        assert_eq!(slot.alpha(2100), 0.5);
        slot.step(2201);
        assert_eq!(slot.state, 0);
        assert_eq!(slot.alpha(2300), 0.0);
    }

    #[test]
    fn loading_the_bar_marks_the_picked_slots() {
        let mut record = PowerRecord::empty(4.0);
        record.slots[0] = 0x100101;
        record.slots[1] = 0x201;
        record.pairs = vec![KnownPower {
            power: 0x101,
            flags: PICKED,
        }];
        let mut bar = PowerBar::default();
        let chosen = bar.load(&record);
        assert_eq!(chosen, 1);
        assert_eq!(bar.slots[0].word, 0x100101);
        assert_eq!(bar.slots[0].state, PICKED);
        assert_eq!(bar.slots[1].state, 0);
        let mut target = PowerRecord::empty(4.0);
        bar.store(&mut target, false);
        assert_eq!(target.slots[0], 0x100101);
        assert_eq!(target.slots[1], 0x201);
        // Storing leaves the display states alone.
        assert_eq!(bar.slots[0].state, PICKED);
        // Loading keeps a slot's stamp and duration and clears its state.
        bar.slots[1] = BarSlot {
            word: 0,
            state: bar_state::FLASH,
            stamp: 5,
            duration: 7,
            ..BarSlot::default()
        };
        bar.load(&record);
        assert_eq!((bar.slots[1].state, bar.slots[1].stamp), (0, 5));
        assert_eq!(bar.slots[1].duration, 7);
    }

    #[test]
    fn a_cast_store_moves_the_chosen_bit() {
        let mut bar = PowerBar::default();
        bar.slots[0].word = 0x100101;
        bar.slots[1].word = 0x201;
        bar.chosen = 2;
        let mut record = PowerRecord::empty(4.0);
        bar.store(&mut record, true);
        assert_eq!(record.slots[0], 0x101);
        assert_eq!(record.slots[1], 0x100201);
        // The bar's own close copies the words as they are.
        bar.store(&mut record, false);
        assert_eq!(record.slots[0], 0x100101);
        assert_eq!(record.slots[1], 0x201);
        // A cast with nothing chosen clears every bit.
        bar.chosen = 0;
        bar.store(&mut record, true);
        assert_eq!(record.slots[0], 0x101);
    }

    #[test]
    fn an_unpicked_mark_beyond_the_slots_ends() {
        let mut powers = vec![
            ActivePower {
                id: 0x20207,
                state: 1,
            },
            ActivePower {
                id: 0x20208,
                state: 1,
            },
            ActivePower {
                id: 0x10_2020b,
                state: 1,
            },
        ];
        assert!(!limit_running_powers(&mut powers, 1));
        assert_eq!(powers[0].state, 1);
        assert_eq!(powers[1].state, 2);
        assert_eq!(powers[2].state, 2);
        let mut powers = vec![ActivePower {
            id: 0x20207,
            state: 1,
        }];
        assert!(limit_running_powers(&mut powers, 1));
    }

    #[test]
    fn a_tree_store_replaces_only_its_domain() {
        let mut screen = TreeScreen::from_nodes(
            DOMAIN_MIND,
            TreeMode::Play,
            vec![node(0x101, vec![]), node(0x102, vec![])],
        );
        screen.marks = vec![SHOWN | LEARNED | PICKED, SHOWN | LEARNED];
        let mut record = PowerRecord::empty(4.0);
        record.pairs = vec![
            KnownPower {
                power: 0x109,
                flags: LEARNED,
            },
            KnownPower {
                power: 0x201,
                flags: LEARNED,
            },
        ];
        screen.store_domain(&mut record);
        assert_eq!(
            record.pairs,
            vec![
                KnownPower {
                    power: 0x201,
                    flags: LEARNED
                },
                KnownPower {
                    power: 0x101,
                    flags: LEARNED | PICKED
                },
                KnownPower {
                    power: 0x102,
                    flags: LEARNED
                },
            ]
        );
    }

    #[test]
    fn a_focus_follows_the_original_branch_order() {
        let mut screen = TreeScreen::from_nodes(
            DOMAIN_MIND,
            TreeMode::Play,
            vec![node(0x101, vec![]), node(0, vec![])],
        );
        screen.nodes[0].name_text = 1;
        assert_eq!(screen.set_focus(1, 10), 3);
        assert_eq!(screen.last_choice, 1);
        assert_eq!(screen.set_focus(1, 20), 0);
        // A special id of a non-creation screen is read: its candidate is 0.
        assert_eq!(screen.set_focus(0x200_0000, 30), 3);
        assert_eq!(screen.last_choice, 0);
        // A node with no name text is no candidate, and the candidate stays 0.
        assert_eq!(screen.set_focus(2, 40), 0);
        assert_eq!(screen.last_choice, 0);
        // The second node has no name, so a bar slot keeps the candidate 0.
        assert_eq!(screen.set_focus(focus::SLOT | 1, 50), 0);
        // Returning to the node whose choice was shown gives 0.
        assert_eq!(screen.set_focus(1, 60), 0);
        assert_eq!(screen.shown_choice, 1);
    }

    #[test]
    fn opening_seeds_the_view_from_the_window() {
        // 1080 over 2300 is 0.4696: between the levels -5 (0.4410) and -4
        // (0.5), and nearer -5, so the charge opens at -5.
        let screen = TreeScreen::load(DOMAIN_MIND, TreeMode::Play, None, None, 1001.0, 1080.0);
        assert_eq!(screen.charge, -5.0);
        let gain = (4.0 - 5.0_f32.sqrt()) * 0.25;
        assert_eq!(screen.zoom.value, gain);
        // The width is halved as an integer: 500, not 500.5.
        assert_eq!(screen.pan.goal, [500.0, 1080.0 - 250.0 * gain]);
        assert_eq!(screen.pan.position, screen.pan.goal);
        // The first placement shows whole numbers: 0.441 * 256 is 113.
        assert_eq!(screen.zoom_fixed, 113.0);
        assert_eq!(screen.shown_pan, [500.0, 1080.0 - 113.0 - 100.0]);
    }

    #[test]
    fn a_middle_drag_moves_the_goal_and_the_view_follows() {
        let mut screen = TreeScreen::empty(DOMAIN_MIND, TreeMode::Play);
        let mut bar = PowerBar::default();
        screen.shown_pan = [500.5, 300.6];
        let input = PressInput {
            x: 100.0,
            y: 100.0,
            word: press::MIDDLE,
            width: 1000.0,
            now: 10,
            hidden: false,
        };
        let out = screen.press(input, None, &mut bar);
        assert_eq!(out.result, 0x15);
        assert_eq!(screen.drag, DragState::Pan);
        // The origin is the shown pan rounded to even.
        assert_eq!(screen.drag_origin, [500.0, 301.0]);
        assert_eq!(screen.drag_move(130.0, 90.0, 20), 3);
        assert_eq!(screen.pan.goal, [530.0, 291.0]);
        assert!(screen.free_view);
        assert_eq!(screen.free_view_stamp, 20);
        // A later move does not restamp the free view.
        screen.drag_move(131.0, 90.0, 30);
        assert_eq!(screen.free_view_stamp, 20);
        assert_eq!(screen.pan.goal, [531.0, 291.0]);
        // The view eases toward the goal: nothing has moved yet.
        assert_eq!(screen.shown_pan, [500.5, 300.6]);
        screen.pan.position = [500.0, 301.0];
        screen.pan.previous = [500.0, 301.0];
        assert!(screen.settle());
        assert!(screen.shown_pan[0] > 500.0 && screen.shown_pan[0] < 531.0);
    }

    #[test]
    fn a_key_cycles_the_bar_or_opens_the_tree() {
        let mut screen = TreeScreen::empty(DOMAIN_MIND, TreeMode::Play);
        let mut bar = PowerBar::default();
        let input = |key, flags, guarded, hidden, stamp, now| KeyInput {
            key,
            flags,
            guarded,
            hidden,
            stamp,
            now,
        };
        // Any other key opens the tree, unless the form is hidden.
        let out = screen.key(input(0x50, 0, false, false, 0, 0), &mut bar, None);
        assert!(out.open && !out.cycled);
        let out = screen.key(input(0x50, 0, false, true, 0, 0), &mut bar, None);
        assert!(!out.open);
        // A key with bit 1 set in its flags never cycles.
        let out = screen.key(input(0x27, 2, false, false, 0, 0), &mut bar, None);
        assert!(out.open && !out.cycled);
        // The guard holds keys until 950 ms before the stamp is past.
        let out = screen.key(input(0x50, 0, true, false, 2000, 1049), &mut bar, None);
        assert!(!out.open);
        let out = screen.key(input(0x50, 0, true, false, 2000, 1050), &mut bar, None);
        assert!(out.open);
        // A stamp under 950 reads negative and never guards.
        let out = screen.key(input(0x50, 0, true, false, 100, 0), &mut bar, None);
        assert!(out.open);
        // A screen that is not the play screen takes no key.
        let mut create = TreeScreen::empty(DOMAIN_MIND, TreeMode::Create);
        let out = create.key(input(0x50, 0, false, false, 0, 0), &mut bar, None);
        assert!(!out.open && !out.cycled);
    }

    #[test]
    fn the_creation_screen_takes_the_buttons() {
        let mut screen = TreeScreen::empty(DOMAIN_MIND, TreeMode::Create);
        let mut bar = PowerBar::default();
        assert_eq!(screen.set_focus(focus::BEGIN, 10), 3);
        let out = press(&mut screen, press::LEFT | press::CLICK, 10, None, &mut bar);
        assert!(out.begin);
        let out = screen.wheel(0x200, 0.0, 0.0, 10);
        assert!(out.back);
        // CREATE mode does not open the tree from a wheel.
        assert!(!screen.wheel(0x300, 0.0, 0.0, 10).open);
    }

    #[test]
    fn a_stretched_link_runs_from_the_node_to_the_pointer() {
        let mut screen =
            TreeScreen::from_nodes(DOMAIN_MIND, TreeMode::Play, vec![node(0x101, vec![])]);
        screen.zoom.value = 1.0;
        screen.zoom_fixed = 256.0;
        assert!(screen.drag_link_points().is_none());
        screen.drag = DragState::LinkHover;
        screen.node_under = 1;
        screen.link_turn = 0.0;
        screen.pointer = [0.0, 100.0];
        let points = screen.drag_link_points().expect("the drag draws");
        assert_eq!(points.len(), 32);
        // The start is 0.508 − 0.003 of the zoom, times 256, straight up.
        assert!((points[0][1] - 129.28).abs() < 1e-4);
        assert_eq!(points[0][0], 0.0);
        // The cubic reaches the pointer at its thirtieth sample and runs a
        // thirtieth of the far tangent past it.
        assert!((points[30][1] - 100.0).abs() < 1e-3);
        assert!((points[31][1] - 100.976).abs() < 1e-3);
        // The far tangent's reach is held to 400: a pointer ten thousand
        // away still ends 13.3 past it.
        screen.pointer = [0.0, 10_000.0];
        let far = screen.drag_link_points().expect("the drag draws");
        assert!((far[31][1] - (10_000.0 - 400.0 / 30.0)).abs() < 1e-2);
        // A framed power starts nearer its centre: 0.29 − 0.003 over 256.
        screen.nodes[0].power = 0x0100_0101;
        screen.pointer = [0.0, 100.0];
        let framed = screen.drag_link_points().expect("the drag draws");
        assert!((framed[0][1] - 73.472).abs() < 1e-4);
        screen.drag = DragState::Idle;
        assert!(screen.drag_link_points().is_none());
    }

    #[test]
    fn a_dragged_link_is_added_to_the_node_and_its_curve() {
        let mut screen = TreeScreen::from_nodes(
            DOMAIN_MIND,
            TreeMode::Play,
            vec![node(0x101, vec![]), node(0x102, vec![])],
        );
        screen.node_under = 1;
        screen.drag = DragState::Link;
        screen.link_turn = 0.0;
        screen.zoom.value = 1.0;
        screen.nodes[1].x = 100.0;
        screen.nodes[1].y = 0.0;
        let added = screen.release_drag(100.0, 0.0);
        assert!(added);
        assert_eq!(screen.nodes[0].links.len(), 1);
        assert_eq!(screen.links[0].len(), 1);
        assert_eq!(screen.links[0][0].end, 2);
    }

    #[test]
    fn a_malformed_tree_leaves_the_domain_power() {
        let screen = TreeScreen::load(
            DOMAIN_FORCE,
            TreeMode::Play,
            None,
            Some(b"not a tree"),
            1000.0,
            700.0,
        );
        assert_eq!(screen.nodes.len(), 1);
        assert_eq!(screen.nodes[0].power, 0x201);
        let empty = TreeScreen::load(DOMAIN_ENERGY, TreeMode::Play, None, None, 1000.0, 700.0);
        assert_eq!(empty.nodes[0].power, 0x401);
        assert_eq!(empty.charge, -8.0);
        assert_eq!(empty.zoom.value, zoom_gain(-8.0));
    }

    #[test]
    fn the_hover_reads_buttons_slots_nodes_and_links() {
        let mut screen = TreeScreen::from_nodes(
            DOMAIN_MIND,
            TreeMode::Play,
            vec![node(0x101, vec![link_to(2)]), node(0x102, vec![])],
        );
        screen.marks = vec![SHOWN, SHOWN];
        screen.zoom_fixed = 4.0;
        assert_eq!(screen.hover(194.0, 32.0, 1000.0, 700.0, false), 0x40_0001);
        assert_eq!(screen.hover(0.0, 0.0, 1000.0, 700.0, false), 1);
        assert_eq!(screen.hover(0.0, 0.0, 1000.0, 700.0, true), 0);
        // A node's link: at 10 px straight above the second node with an
        // incoming turn of zero.
        screen.nodes[0].x = 0.0;
        screen.nodes[0].y = 0.0;
        screen.nodes[1].x = 0.0;
        screen.nodes[1].y = 100.0;
        let mut link = [0u8; 16];
        link[4..8].copy_from_slice(&(-0.5_f32).to_le_bytes());
        link[8..12].copy_from_slice(&1_u32.to_le_bytes());
        screen.nodes[1].links = vec![link];
        screen.links[1] = vec![NodeLink {
            out_turn: 0.0,
            in_turn: -0.5,
            end: 1,
        }];
        // 0.01 of a circle above the first node's centre is straight up:
        // the first node's link leaves at turn zero.
        let id = screen.hover(0.0, 10.0, 1000.0, 700.0, false);
        assert_eq!(id, 1 << 16 | 1);
        // The creation prompt's buttons.
        let mut create = TreeScreen::empty(DOMAIN_MIND, TreeMode::Create);
        assert_eq!(
            create.hover(900.0, 675.0, 1000.0, 700.0, false),
            focus::BEGIN
        );
        assert_eq!(create.hover(60.0, 675.0, 1000.0, 700.0, false), focus::BACK);
    }

    #[test]
    fn the_hover_sizes_the_circles_by_the_pixel_zoom_and_names_incoming_links_by_their_owner() {
        let mut screen = TreeScreen::from_nodes(
            DOMAIN_MIND,
            TreeMode::Play,
            vec![node(0x101, vec![]), node(0x102, vec![])],
        );
        screen.marks = vec![SHOWN, SHOWN];
        screen.nodes[0].x = 500.0;
        screen.nodes[0].y = 500.0;
        screen.nodes[1].x = 100.0;
        screen.nodes[1].y = 100.0;
        screen.zoom_fixed = 100.0;
        // Inside 0.45 of the circle's size: the node; outside it, and with no
        // link there, nothing.
        assert_eq!(screen.hover(500.0, 544.0, 1000.0, 700.0, false), 1);
        assert_eq!(screen.hover(500.0, 546.0, 1000.0, 700.0, false), 0);
        // A framed node's circle is half the size.
        screen.nodes[0].power |= 0x100_0000;
        assert_eq!(screen.hover(500.0, 522.0, 1000.0, 700.0, false), 1);
        assert_eq!(screen.hover(500.0, 524.0, 1000.0, 700.0, false), 0);
        // A link that leaves the second node and ends at the first is named
        // by the second node when the first is hovered where it arrives.
        screen.nodes[0].power = 0x101;
        screen.zoom_fixed = 60.0;
        screen.links[1] = vec![NodeLink {
            out_turn: 0.9,
            in_turn: 0.0,
            end: 1,
        }];
        assert_eq!(
            screen.hover(500.0, 540.0, 1000.0, 700.0, false),
            (1 << 16) | 2 | focus::INCOMING
        );
        // The creation prompt does not read the quick bar.
        let mut create = TreeScreen::empty(DOMAIN_MIND, TreeMode::Create);
        assert_eq!(create.hover(194.0, 32.0, 1000.0, 700.0, false), 0);
    }

    #[test]
    fn the_cast_entry_reads_the_record_slot_and_flashes_it() {
        let mut screen = TreeScreen::load(DOMAIN_FORCE, TreeMode::Play, None, None, 1000.0, 700.0);
        let mut record = PowerRecord::starter();
        record.slots[0] = 0x100201;
        record.slots[1] = 0x10203;
        let mut bar = PowerBar::default();
        bar.load(&record);
        let mut body = CastBody::default();
        screen.bar_open = true;
        let out = screen.cast_slot(
            &mut record,
            &mut bar,
            None,
            &mut body,
            500,
            |domain, word, _body| {
                assert_eq!(domain, DOMAIN_FORCE);
                assert_eq!(word, 0x100201);
                1
            },
        );
        assert_eq!(out.result, 1);
        assert_eq!(body.last_instant, 0x201);
        assert!(!screen.bar_open);
        assert_eq!(record.slots[0], 0x100201);
        assert_eq!(bar.slots[0].state & bar_state::MARK, bar_state::MARK);
        // A sustained power clears the last instant power.
        let out = screen.cast_slot(&mut record, &mut bar, Some(1), &mut body, 600, |_, _, _| 1);
        assert_eq!(out.result, 1);
        assert_eq!(body.last_instant, 0);
        // A body in control of another casts nothing: the caller ends the
        // control. The front ends' own block bit is theirs to answer.
        body.status_bits = CONTROLLING;
        let out = screen.cast_slot(&mut record, &mut bar, Some(0), &mut body, 700, |_, _, _| {
            panic!("no dispatch")
        });
        assert_eq!(out.result, 0);
        assert!(out.end_control);
        body.status_bits = 0;
        // A body whose item flags refuse powers answers −1 without
        // dispatching: the fail sound plays at 0.05.
        body.item_flags = ITEM_REFUSES_POWERS;
        let out = screen.cast_slot(&mut record, &mut bar, Some(0), &mut body, 750, |_, _, _| {
            panic!("no dispatch")
        });
        assert_eq!(out.result, 0);
        assert_eq!(out.sound, Some(CAST_FAIL_SOUND));
        assert_eq!(CAST_FAIL_SOUND.volume, 0.05);
        body.item_flags = 0;
        // Shadow/body domains are refused.
        record.slots[2] = 0x301;
        let out = screen.cast_slot(&mut record, &mut bar, Some(2), &mut body, 800, |_, _, _| {
            panic!("no dispatch")
        });
        assert_eq!(out.result, 0);
        assert_eq!(out.sound, Some(CAST_FAIL_SOUND));
        // "Not now" queues the slot and leaves the bar alone.
        screen.bar_open = true;
        let before = bar.clone();
        let out = screen.cast_slot(&mut record, &mut bar, Some(0), &mut body, 900, |_, _, _| -3);
        assert_eq!(out.queued_slot, Some(0));
        assert_eq!(out.result, 0);
        assert!(screen.bar_open);
        assert_eq!(bar, before);
    }

    #[test]
    fn a_cast_slot_flashes_from_where_it_stood() {
        let slot = |state, stamp, duration| BarSlot {
            word: 0x101,
            state,
            stamp,
            duration,
            ..BarSlot::default()
        };
        let mut bar = PowerBar::default();
        // A lit slot fades out over 200 ms from now.
        bar.slots[0] = slot(bar_state::LIT, 0, 0);
        bar.cast_flash(0, 1000);
        assert_eq!(
            bar.slots[0],
            slot(bar_state::FADE | bar_state::MARK, 1000, 200)
        );
        // A fading slot (a quarter gone of 400 ms) flashes in from the
        // three quarters it still shows: back-dated by 0.75 × 200 = 150 ms.
        bar.slots[1] = slot(bar_state::FADE, 2000, 400);
        bar.cast_flash(1, 2100);
        assert_eq!(
            bar.slots[1].state,
            bar_state::FADE | bar_state::FLASH | bar_state::MARK
        );
        assert_eq!(bar.slots[1].stamp, 1950);
        // A flash 100 ms in shows 0.5 and keeps its start.
        bar.slots[2] = slot(bar_state::FLASH, 3000, 0);
        bar.cast_flash(2, 3100);
        assert_eq!(bar.slots[2].stamp, 3000);
        // A dark slot starts at once.
        bar.slots[3] = slot(0, 7, 9);
        bar.cast_flash(3, 4000);
        assert_eq!(
            bar.slots[3],
            slot(bar_state::FLASH | bar_state::MARK, 4000, 9)
        );
        // A fade with no duration is not a number and back-dates nothing.
        bar.slots[4] = slot(bar_state::FADE, 5000, 0);
        bar.cast_flash(4, 5100);
        assert_eq!(bar.slots[4].stamp, 5100);
    }

    #[test]
    fn the_ease_and_the_link_angle_match_the_original() {
        assert_eq!(ease(0.0), 0.0);
        // 1 + 0.111/2 - 1/5, 1 + 0.111 - 1/9: it rises fast, then flattens.
        assert!((ease(0.5) - 0.855_5).abs() < 1e-6);
        assert!((ease(1.0) - 0.999_888_9).abs() < 1e-6);
        assert!(ease(0.1) > 0.4 && ease(0.1) < 0.5);
        assert!((link_angle(0.0, 0.0, 0.0, 10.0) / std::f32::consts::TAU).abs() < 1e-6);
        assert!((link_angle(0.0, 0.0, 10.0, 0.0) / std::f32::consts::TAU - 0.25).abs() < 1e-6);
        assert!((link_angle(0.0, 0.0, -10.0, 0.0) / std::f32::consts::TAU + 0.25).abs() < 1e-6);
        assert!(link_angle(1.0, 2.0, 1.0, 2.0).is_nan());
        assert_eq!(link_angle(0.0, 0.0, 0.0, -10.0), std::f32::consts::PI);
        assert!(wants_power_drag(press::POWER_DRAG));
        assert!(!wants_power_drag(0));
    }

    #[test]
    fn the_bar_loads_marks_and_toggles() {
        let mut screen = TreeScreen::load(DOMAIN_MIND, TreeMode::Play, None, None, 1000.0, 700.0);
        screen.nodes[0].power = 0x101;
        screen.marks = vec![SHOWN | LEARNED];
        let mut record = PowerRecord::empty(4.0);
        record.slots[0] = 0x100101;
        record.pairs = vec![KnownPower {
            power: 0x101,
            flags: LEARNED | PICKED,
        }];
        let mut bar = PowerBar::default();
        assert!(screen.toggle_bar(&mut bar, &mut record, 10));
        assert_eq!(bar.slots[0].word, 0x100101);
        assert_eq!(screen.marks[0], SHOWN | LEARNED | PICKED);
        assert!(!screen.toggle_bar(&mut bar, &mut record, 20));
        assert_eq!(record.slots[0], 0x100101);
        screen.leave(&mut bar, &mut record);
        assert!(!screen.bar_open);
        assert_eq!(
            record.pairs,
            vec![KnownPower {
                power: 0x101,
                flags: LEARNED | PICKED
            }]
        );
    }

    #[test]
    fn a_saved_power_needs_a_learnt_prerequisite() {
        use exa_formats::power_tree::SavedPower;
        let saved = [
            SavedPower {
                power: 0x101,
                prerequisite: 0,
            },
            SavedPower {
                power: 0x102,
                prerequisite: 1,
            },
            SavedPower {
                power: 0x103,
                prerequisite: 3,
            },
        ];
        let mut known = vec![KnownPower {
            power: 0x101,
            flags: LEARNED,
        }];
        // The second entry names the first, which is learnt.
        assert!(validate_saved_power(&saved, &mut known, 0x102));
        assert_eq!(
            known[1],
            KnownPower {
                power: 0x102,
                flags: SAVED_FLAGS
            }
        );
        // The first entry is never re-validated.
        assert!(!validate_saved_power(&saved, &mut known, 0x101));
        // The third names a prerequisite past the list: unlearnt.
        assert!(!validate_saved_power(&saved, &mut known, 0x103));
        // An entry without a prerequisite is refused.
        let flat = [SavedPower {
            power: 0x101,
            prerequisite: 0,
        }];
        assert!(!validate_saved_power(&flat, &mut known, 0x101));
        // The same power again replaces the first matching pair.
        let twice = [
            SavedPower {
                power: 0x101,
                prerequisite: 0,
            },
            SavedPower {
                power: 0x10_0102,
                prerequisite: 1,
            },
        ];
        known.push(KnownPower {
            power: 0x102,
            flags: 0,
        });
        assert!(validate_saved_power(&twice, &mut known, 0x102));
        assert_eq!(known[1].flags, SAVED_FLAGS);
        assert_eq!(known[2].flags, 0);
    }

    #[test]
    fn a_granted_power_needs_its_prerequisite_learnt() {
        let nodes = [
            node(0x101, vec![]),
            node(0x102, vec![link_to(1)]),
            node(0x103, vec![link_to(2)]),
            node(0x104, vec![]),
        ];
        let mut insight = skill(35_000, [0; 6]);
        insight.techniques[1] = GRANT_TECHNIQUE;
        let mut known = vec![KnownPower {
            power: 0x101,
            flags: LEARNED,
        }];
        assert!(grant_power(&nodes, &insight, &mut known, 0x102));
        assert_eq!(
            known[1],
            KnownPower {
                power: 0x102,
                flags: GRANTED_FLAGS
            }
        );
        // The first node never has a prerequisite to ask.
        assert!(!grant_power(&nodes, &insight, &mut known, 0x101));
        // A granted power's flags carry the learnt bit, so it opens the
        // next node.
        assert!(grant_power(&nodes, &insight, &mut known, 0x103));
        // A node without a link has no prerequisite.
        assert!(!grant_power(&nodes, &insight, &mut known, 0x104));
        // A power the tree lacks is refused.
        assert!(!grant_power(&nodes, &insight, &mut known, 0x105));
        assert_eq!(known.len(), 3);
    }

    #[test]
    fn a_grant_overwrites_the_first_matching_pair() {
        let nodes = [node(0x101, vec![]), node(0x102, vec![link_to(1)])];
        let mut insight = skill(35_000, [0; 6]);
        insight.techniques[1] = GRANT_TECHNIQUE;
        // A pair that carries the chosen bit still stands for its node: the
        // match drops bits 20 to 23.
        let mut known = vec![
            KnownPower {
                power: 0x10_0101,
                flags: LEARNED,
            },
            KnownPower {
                power: 0x102,
                flags: 0x1234,
            },
            KnownPower {
                power: 0x102,
                flags: 0x5678,
            },
        ];
        let mut unlearnt = known.clone();
        assert!(grant_power(&nodes, &insight, &mut known, 0x102));
        assert_eq!(known.len(), 3);
        assert_eq!(known[1].flags, GRANTED_FLAGS);
        assert_eq!(known[2].flags, 0x5678);
        // The last pair for the prerequisite gives its flags.
        unlearnt.push(KnownPower {
            power: 0x101,
            flags: 0,
        });
        assert!(!grant_power(&nodes, &insight, &mut unlearnt, 0x102));
        assert_eq!(unlearnt[1].flags, 0x1234);
    }

    #[test]
    fn a_grant_needs_the_seventh_insight_technique() {
        let nodes = [node(0x101, vec![]), node(0x102, vec![link_to(1)])];
        let mut known = vec![KnownPower {
            power: 0x101,
            flags: LEARNED,
        }];
        let mut insight = skill(35_000, [0; 6]);
        insight.techniques[1] = GRANT_TECHNIQUE;
        // The second slot needs 10 000 points; the points fund it.
        assert!(grant_power(&nodes, &insight, &mut known.clone(), 0x102));
        insight.points = 9_999;
        assert!(!grant_power(&nodes, &insight, &mut known, 0x102));
        assert!(!grant_power(
            &nodes,
            &skill(35_000, [0; 6]),
            &mut known,
            0x102
        ));
    }

    #[test]
    fn a_prerequisite_past_the_nodes_is_unlearnt() {
        let nodes = [node(0x101, vec![]), node(0x102, vec![link_to(9)])];
        let mut insight = skill(35_000, [0; 6]);
        insight.techniques[0] = GRANT_TECHNIQUE;
        let mut known = vec![];
        assert!(!grant_power(&nodes, &insight, &mut known, 0x102));
    }

    #[test]
    fn the_first_opening_picks_the_mind_domain_and_seeds_nothing() {
        assert_eq!(open_domain(false, DOMAIN_DISPLACEMENT), DOMAIN_MIND);
        assert_eq!(open_domain(true, DOMAIN_DISPLACEMENT), DOMAIN_DISPLACEMENT);
        assert_eq!(open_domain(true, DOMAIN_FORCE), DOMAIN_FORCE);
        let mut record = PowerRecord::empty(4.0);
        assert!(seed_defaults(&mut record));
        assert_eq!(record.pairs, DEFAULT_PAIRS);
        // The table the open copies: the first mind power learnt, and the
        // persistent second one new, each with its stored flags.
        assert_eq!(DEFAULT_PAIRS[0].power, 0x101);
        assert_eq!(DEFAULT_PAIRS[0].flags, 0x1000);
        assert_eq!(DEFAULT_PAIRS[1].power, 0x0102_0101);
        assert_eq!(DEFAULT_PAIRS[1].flags, 0x2800);
        assert!(!seed_defaults(&mut record));
        let mut known = PowerRecord::starter();
        assert!(!seed_defaults(&mut known));
        assert_eq!(known.pairs.len(), 2);
    }

    #[test]
    fn the_window_fades_in_and_out_by_its_stamps() {
        let mut tree = TreeScreen::empty(DOMAIN_MIND, TreeMode::Play);
        tree.show(1000);
        // Halfway through the opening fade: the inner value is 0.5, so the
        // window is 1 − 0.25 doubled and held at one, and its inner value
        // is 0.75.
        tree.present_window(1500);
        assert_eq!(tree.window_alpha, 1.0);
        assert_eq!(tree.content_alpha, 1.0);
        assert_eq!(tree.bar_alpha, 0.75);
        // A millisecond before the fade ends it is nearly shut: the inner
        // 0.999 squares to 0.99800104, so the window is its complement
        // doubled, 0.003997922, and the bar's fade is 0.001998961.
        tree.present_window(1999);
        assert!((tree.window_alpha - 0.003_997_922).abs() < 1e-9);
        assert!((tree.content_alpha - 0.003_997_922).abs() < 1e-9);
        assert!((tree.bar_alpha - 0.001_998_961).abs() < 1e-9);
        // At the stamp the flag clears and the window is whole.
        tree.present_window(2000);
        assert_eq!(tree.fade_flags, 0);
        assert_eq!(tree.window_alpha, 1.0);
        assert!(!tree.faded_out);
        // The closing fade halves the content a 128 ms from its end.
        tree.hide(3000);
        tree.present_window(3128);
        assert_eq!(tree.window_alpha, 0.5);
        assert_eq!(tree.content_alpha, 0.5);
        assert_eq!(tree.bar_alpha, 0.5);
        tree.present_window(3256);
        assert_eq!(tree.fade_flags, 0);
        assert!(tree.faded_out);
        assert_eq!(tree.window_alpha, 0.0);
    }

    #[test]
    fn a_domain_switch_fades_out_then_in() {
        let mut tree = TreeScreen::empty(DOMAIN_MIND, TreeMode::Play);
        tree.fade_domain(1000);
        assert_eq!((tree.domain_flags, tree.domain_at), (DOMAIN_OUT, 1256));
        tree.present_window(1128);
        assert_eq!(tree.content_alpha, 0.5);
        // The stamp switches the domain: the caller's flag rises and the
        // new tree fades in over the next second.
        tree.present_window(1256);
        assert!(tree.switch_due);
        assert_eq!((tree.domain_flags, tree.domain_at), (DOMAIN_IN, 2256));
        assert_eq!(tree.content_alpha, 0.0);
        tree.present_window(1756);
        assert!(!tree.switch_due);
        assert_eq!(tree.content_alpha, 1.0);
        // The window itself never faded.
        assert_eq!(tree.window_alpha, 1.0);
    }

    #[test]
    fn the_bar_opens_and_closes_over_nine_hundred_milliseconds() {
        use exa_ui::tree_draw::BarPhase;
        let mut tree = TreeScreen::empty(DOMAIN_MIND, TreeMode::Play);
        let mut bar = PowerBar::default();
        bar.slots[0].state = bar_state::FLASH | bar_state::MARK;
        let record = PowerRecord::starter();
        tree.open_bar(&mut bar, &record, 1000);
        // Opening clears the slot's flash and picked bits; the loaded
        // record's picked pair puts the picked bit back.
        assert_eq!(bar.slots[0].state, bar_state::PICKED);
        assert_eq!(
            tree.step_bar(&mut bar, 1049),
            Some(BarPhase::Opening { elapsed: 49 })
        );
        assert_eq!(
            tree.step_bar(&mut bar, 1899),
            Some(BarPhase::Opening { elapsed: 899 })
        );
        assert_eq!(tree.step_bar(&mut bar, 1900), Some(BarPhase::Steady));
        assert_eq!(tree.bar_flags, BAR_STEADY);
        // A closing bar runs its own 900 ms and leaves the slots' flash,
        // lit, fade and picked bits alone.
        bar.slots[1].state = bar_state::LIT;
        tree.close_bar(2000);
        assert_eq!(tree.bar_flags, BAR_CLOSING);
        assert_eq!(
            tree.step_bar(&mut bar, 2100),
            Some(BarPhase::Closing { elapsed: 100 })
        );
        assert_eq!(
            tree.step_bar(&mut bar, 2899),
            Some(BarPhase::Closing { elapsed: 899 })
        );
        assert_eq!(tree.step_bar(&mut bar, 2900), None);
        assert_eq!(tree.bar_flags, 0);
        assert_eq!(bar.slots[1].state, bar_state::LIT);
        // Closing while opening: the closing starts when the opening ends.
        tree.open_bar(&mut bar, &record, 3000);
        tree.close_bar(3200);
        assert_eq!(
            tree.step_bar(&mut bar, 3900),
            Some(BarPhase::Closing { elapsed: 700 })
        );
    }

    #[test]
    fn showing_stamps_the_open_fade_and_the_domain_switches() {
        let mut tree = TreeScreen::empty(DOMAIN_FORCE, TreeMode::Play);
        assert_eq!(
            tree.show(1000),
            ShowState {
                flags: FADING_IN,
                stamp: 2000,
                remembered: DOMAIN_FORCE
            }
        );
        assert_eq!(tree.hide(3000).stamp, 3256);
        assert_eq!(tree.fade_flags, FADING_IN | FADING_OUT);
        let mut record = PowerRecord::starter();
        let old = tree.switch_domain(DOMAIN_MIND, &mut record, None, 1000.0, 700.0);
        assert_eq!(old, DOMAIN_FORCE);
        assert_eq!(tree.domain, DOMAIN_MIND);
        assert_eq!(tree.nodes[0].power, 0x101);
    }

    #[test]
    fn a_real_tree_parses_when_it_is_installed() {
        let Ok(root) = std::env::var("EXANIMA_DIR") else {
            return;
        };
        let path = std::path::Path::new(&root).join("Resource.rpk");
        let Ok(bytes) = std::fs::read(path) else {
            return;
        };
        let magic = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        assert_eq!(magic, 0xAFBF_0C01);
        let table = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
        for record in 0..table / 32 {
            let at = 8 + record * 32;
            let name = &bytes[at..at + 16];
            let end = name.iter().position(|b| *b == 0).unwrap_or(16);
            let name = std::str::from_utf8(&name[..end]).unwrap_or("");
            if !name.ends_with(".pwr") {
                continue;
            }
            let offset = u32::from_le_bytes(bytes[at + 16..at + 20].try_into().unwrap()) as usize;
            let size = u32::from_le_bytes(bytes[at + 20..at + 24].try_into().unwrap()) as usize;
            let start = 8 + table + offset;
            let file = &bytes[start..start + size];
            let tree = exa_formats::power_tree::read_power_tree(file);
            assert!(tree.is_ok(), "{name}");
            assert!(!tree.unwrap().nodes.is_empty());
        }
    }
}
