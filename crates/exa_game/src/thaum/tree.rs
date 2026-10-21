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
//!   power pairs, its ten power-bar slots and the learnt technique tiers.
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
//!   the techniques it asks for, and leaves it stored on the record with
//!   [`NEW`]. A click on a learnt node picks or drops it: picking needs the
//!   domain's cost within what is left of the record's focus and every link
//!   end picked; dropping is refused while another picked node links to it.
//!   Closing stores the domain's learnt and picked nodes back into the
//!   record, replacing that domain's pairs.
//! - **The power bar** ([`PowerBar`]): ten slots, the chosen one marked with
//!   bit 20. Loading it from a record copies the slots and marks a slot
//!   picked when its pair is. A slot's state flashes in over 200 ms, stays
//!   lit, then fades over its duration. A press selects a slot, a cast
//!   stores the bar back and flashes the slot, a dropped power fills the
//!   slot (and, from another slot, leaves a copy there), and a drop that
//!   misses clears the slot it came from.
//! - **The cast entry** ([`TreeScreen::cast_slot`]) reads the record's slot,
//!   refuses while the body's powers are blocked, and hands the power to
//!   the domain's front end: mind, force, energy or displacement. A cast
//!   that starts records the last instant power (a sustained or persistent
//!   power clears it); a refused one plays the failure sound.
//! - **The slot budget** ([`limit_running_powers`]): the highest thaumaturgy
//!   technique of 2 to 4 among the record's tiers, one less without the
//!   focus item, is how many running powers a body may keep; the rest are
//!   told to end.
//!
//! Evidence: the tree reader, the open, store, mark, focus, click, drag,
//! key and bar routines, the bar's layout and its state timings, and the
//! record's pair and slot arrays. Where the original calls into the
//! character creator for the final commitment of a [`NEW`] power, this
//! module leaves the pair on the record and exposes [`TreeScreen::commit`].

use exa_formats::power_tree::{PowerNode, PowerTree, learn_marks, node_pixels, place_node};

/// What loading the bar clears from a record slot: bits 23 and 24 and the
/// technique nibble.
const BAR_LOAD_MASK: u32 = 0x0E7F_FFFF;

use super::casting::{ActivePower, CastBody};

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

/// The learnt technique tiers the tree's learning rules read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkillTiers {
    /// The casting skill's tiers, one byte a technique.
    pub casting: Vec<u8>,
    /// The thaumaturgy skill's tiers, one byte a technique.
    pub thaumaturgy: Vec<u8>,
}

impl SkillTiers {
    /// Whether a tier holds `technique` among its first `count` bytes.
    pub fn holds(tiers: &[u8], count: u16, technique: u8) -> bool {
        let count = usize::from(count / 5000);
        tiers.iter().take(count).any(|&tier| tier == technique)
    }
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

/// The two default pairs of a tree opened with no record.
pub const DEFAULT_PAIRS: [KnownPower; 2] = [
    KnownPower {
        power: 0x101,
        flags: LEARNED,
    },
    KnownPower {
        power: 0x1002_0101,
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
/// A power is marked or the selected bar slot moves.
pub const SELECT_SOUND: TreeSound = TreeSound {
    name: "ThaumSel01",
    volume: 0.1,
    pitch: 1.0,
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

/// The spring the pan and the zoom ease by.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Spring2 {
    pub value: [f32; 2],
    pub target: [f32; 2],
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
}

/// A key with the screen's hide state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyInput {
    pub key: i32,
    pub flags: u32,
    pub hidden: bool,
    pub hide_until: u32,
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
    /// The node a mark or pick last flashed.
    pub flash: i32,
    /// When the flash ends.
    pub flash_until: u32,
    /// When another new power may be marked.
    pub learn_until: u32,
    /// A refused mark or pick turns its frame red.
    pub denied: bool,
    /// The drag state.
    pub drag: DragState,
    /// The point a drag started from.
    pub drag_origin: [f32; 2],
    /// The last pointer point.
    pub pointer: [f32; 2],
    /// A link's turn while it is dragged.
    pub link_turn: f32,
    /// The node the screen must look at.
    pub hovered: i32,
    /// The quick bar overlay is open.
    pub bar_open: bool,
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
/// to −1 below, and 0 for one or more than two.
pub fn zoom_level(value: f32) -> i32 {
    if value >= 1.0 {
        if value > 1.0 {
            for step in 1..=5 {
                if value < (step as f32 + 5.0) * 0.2 {
                    return step;
                }
            }
        }
        return 0;
    }
    // The first comparison's previous level is a not-a-number, so a value
    // below the first threshold always answers the loop's starting level.
    let mut previous = f32::NAN;
    let mut at = -9;
    loop {
        let next = at + 1;
        let level = (4.0 - (-f64::from(next)).sqrt()) as f32 * 0.25;
        if value < level {
            return if (level - value).abs() <= (previous - value).abs() {
                next
            } else {
                at
            };
        }
        previous = level;
        at = next;
        if at >= -1 {
            return 0;
        }
    }
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
            flash: 0,
            flash_until: 0,
            learn_until: 0,
            denied: false,
            drag: DragState::Idle,
            drag_origin: [0.0; 2],
            pointer: [0.0; 2],
            link_turn: 0.0,
            hovered: 0,
            bar_open: false,
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
        let level = zoom_level(height * 0.000_434_782_608_695_652);
        screen.charge = level as f32;
        let gain = zoom_gain(screen.charge);
        screen.zoom = ZoomSpring {
            value: gain,
            target: gain,
            previous: gain,
        };
        screen.pan = Spring2 {
            value: [width / 2.0, height - gain * 250.0],
            target: [width / 2.0, height - gain * 250.0],
            previous: [width / 2.0, height - gain * 250.0],
        };
        screen.refresh_placement();
        screen.pan.target = [width / 2.0, height - screen.zoom_fixed - 100.0];
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

    /// Places the nodes at ten times their place times the zoom plus the
    /// pan, rounded to even unless the view is dragged.
    pub fn refresh_placement(&mut self) {
        self.zoom_fixed = (self.zoom.value * 256.0).round_ties_even();
        for index in 0..self.nodes.len() {
            let point = node_pixels(
                self.base[index][0],
                self.base[index][1],
                self.zoom.value,
                self.pan.value[0],
                self.pan.value[1],
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

    /// Eases the pan and the zoom. Returns whether anything moved.
    pub fn settle(&mut self) -> bool {
        let moved_pan = step_pan(&mut self.pan);
        let moved_zoom = step_zoom(&mut self.zoom);
        if moved_pan {
            self.refresh_placement();
        }
        if moved_zoom {
            self.zoom_fixed = (self.zoom.value * 256.0).round_ties_even();
        }
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
        } else if id > 0 && id < 0xfffe {
            let index = id as usize - 1;
            if self.nodes.get(index).is_some_and(|node| node.power != 0) {
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

    /// Handles a press word at `x`,`y`. `width` lays the bar out; `record`
    /// is the tree's character record; `bar` is the power bar.
    pub fn press(
        &mut self,
        input: PressInput,
        mut record: Option<&mut PowerRecord>,
        bar: &mut PowerBar,
    ) -> PressResult {
        let PressInput { word, now, .. } = input;
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
                let slot = (id & focus::NODE) as usize;
                if slot > 0 {
                    bar.select(slot - 1);
                    out.slot = Some(slot - 1);
                    out.sound = Some(SELECT_SOUND);
                }
                return out;
            }
        }
        if word & press::MIDDLE != 0 {
            self.drag = DragState::Pan;
            self.drag_origin = self.pan.value;
            out.result = 0x15;
            return out;
        }
        if word & press::RIGHT != 0 {
            if self.mode == TreeMode::Play {
                out.open = true;
            }
            return out;
        }
        if word & press::LEFT != 0 {
            self.node_under = if id > 0 { id & focus::NODE } else { 0 };
            let plain = self.node_under > 0
                && word & press::CLICK != 0
                && id & focus::LINK == 0
                && now >= self.learn_until
                && id & focus::SLOT == 0;
            if plain {
                let node = self.node_under as usize - 1;
                if self.marks[node] & LEARNED == 0 {
                    let learnable = record
                        .as_deref()
                        .is_some_and(|record| can_learn(record, self.nodes[node].power));
                    if learnable && self.marks[node] & (NEW | LEARNED) == 0 {
                        self.flash = 0;
                        for index in 0..self.nodes.len() {
                            if self.marks[index] & NEW != 0 {
                                self.marks[index] &= !NEW;
                                self.flash = index as i32 + 1;
                                if let Some(record) = record.as_deref_mut() {
                                    record.store_pair(self.nodes[index].power, self.marks[index]);
                                }
                            }
                        }
                        out.sound = Some(SELECT_SOUND);
                        self.flash = node as i32 + 1;
                        self.marks[node] |= NEW;
                        if let Some(record) = record {
                            record.store_pair(self.nodes[node].power, self.marks[node]);
                        }
                        self.learn_until = now.wrapping_add(200);
                    }
                    return out;
                }
                self.flash = node as i32;
                self.flash_until = now.wrapping_add(300);
                let picked = self.marks[node] & PICKED != 0;
                if picked {
                    for index in 0..self.nodes.len() {
                        if self.marks[index] & PICKED == 0 {
                            continue;
                        }
                        if self.links[index]
                            .iter()
                            .any(|link| link.end as usize == node + 1)
                        {
                            out.sound = Some(FAIL_SOUND);
                            self.denied = true;
                            return out;
                        }
                    }
                    self.marks[node] &= !PICKED;
                    out.sound = Some(DROP_SOUND);
                } else {
                    let domain = domain_of(self.nodes[node].power);
                    let cost = if (1..=6).contains(&domain) {
                        self.costs[usize::from(domain) - 1]
                    } else {
                        4.0
                    };
                    if self.available < cost {
                        out.sound = Some(FAIL_SOUND);
                        self.denied = true;
                        return out;
                    }
                    for link in &self.links[node] {
                        let end = link.end as usize;
                        if end < 1 || end > self.nodes.len() || self.marks[end - 1] & PICKED == 0 {
                            out.sound = Some(FAIL_SOUND);
                            self.denied = true;
                            return out;
                        }
                    }
                    self.marks[node] |= PICKED;
                    out.sound = Some(PICK_SOUND);
                }
                if let Some(record) = record {
                    record.store_pair(self.nodes[node].power, self.marks[node]);
                    self.read_record(record);
                }
                return out;
            }
        }
        if word & press::DRAG != 0 && id > 0 {
            let node = id & focus::NODE;
            if node > 0 {
                if id & focus::SLOT != 0 {
                    let slot = node as usize - 1;
                    if bar.slots.get(slot).is_some_and(|entry| entry.word != 0) {
                        let entry = bar.slots[slot];
                        out.result = 0x105;
                        out.drag = Some(PowerDrag {
                            power: entry.word,
                            flags: entry.state,
                            kind: press::POWER_DRAG,
                            origin: node as usize,
                        });
                    }
                } else if self
                    .marks
                    .get(node as usize - 1)
                    .is_some_and(|mark| mark & LEARNED != 0)
                {
                    let node = &self.nodes[node as usize - 1];
                    out.result = 0x105;
                    out.drag = Some(PowerDrag {
                        power: node.power,
                        // The original leaves this word to the caller; its
                        // preview frame reads the picked bit.
                        flags: self.marks[(id & focus::NODE) as usize - 1],
                        kind: press::POWER_DRAG,
                        origin: 0,
                    });
                }
            }
            return out;
        }
        out
    }

    /// Starts a pan at `(x, y)`: the middle button was pressed.
    pub fn drag_move(&mut self, x: f32, y: f32, now: u32) -> u32 {
        self.pointer = [x, y];
        match self.drag {
            DragState::Pan => {
                self.pan.value = [
                    self.drag_origin[0] + (x - self.pointer[0]),
                    self.drag_origin[1] + (y - self.pointer[1]),
                ];
                self.free_view = true;
                self.free_view_stamp = now;
                self.refresh_placement();
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
            self.pan.value[0] -= step * dx * 4.0;
            self.pan.value[1] -= step * dy * 4.0;
            if !self.free_view {
                self.free_view = true;
                self.free_view_stamp = now;
            }
            self.refresh_placement();
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
    /// tree opening on the key that is neither. A hidden screen with a
    /// future hide stamp ignores keys until its 950 ms guard has passed.
    pub fn key(
        &mut self,
        input: KeyInput,
        bar: &mut PowerBar,
        record: Option<&mut PowerRecord>,
    ) -> KeyResult {
        let KeyInput {
            key,
            flags,
            hidden,
            hide_until,
            now,
        } = input;
        let mut out = KeyResult::default();
        if self.mode != TreeMode::Play {
            return out;
        }
        if hidden && now < hide_until.wrapping_sub(0x3b6) {
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
        if self.mode == TreeMode::Create && height - 36.0 < y && y < height - 8.0 && 20.0 < x {
            if width - 148.0 < x && x < width - 20.0 {
                return focus::BEGIN;
            }
            if x < 148.0 {
                return focus::BACK;
            }
        } else if let Some(slot) = bar_slot_at(x, y, width, false, 4.0) {
            return focus::SLOT | (slot as i32 + 1);
        }
        for index in 0..self.nodes.len() {
            if self.marks[index] & SHOWN == 0 {
                continue;
            }
            let node = &self.nodes[index];
            let dx = x - node.x;
            let dy = y - node.y;
            let distance = dy * dy + dx * dx;
            let radius = if node.power & word::FRAME != 0 {
                self.zoom.value
            } else {
                self.zoom.value * 0.5
            };
            if distance < radius * 0.45 * radius * 0.45 {
                return index as i32 + 1;
            }
            let reach = radius * 0.55 + 10.0;
            if distance < reach * reach {
                let turn = link_angle(node.x, node.y, x, y) / std::f32::consts::TAU;
                for (at, link) in self.links[index].iter().enumerate() {
                    if (turn - link.out_turn).abs() < 0.01 {
                        return ((at as i32 + 1) << 16) | (index as i32 + 1);
                    }
                }
                for other in 0..self.nodes.len() {
                    for (at, link) in self.links[other].iter().enumerate() {
                        if link.end as usize == index + 1 && (turn - link.in_turn).abs() < 0.01 {
                            return ((at as i32 + 1) << 16) | (index as i32 + 1) | focus::INCOMING;
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

    /// Marks each node from a bar slot with the same power.
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
            bar.store(record, now);
        } else {
            bar.load(record);
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
    pub fn leave(&mut self, bar: &mut PowerBar, record: &mut PowerRecord, now: u32) {
        if self.bar_open {
            bar.store(record, now);
        }
        self.close(record);
    }

    /// The cast entry: reads the record's slot (or the chosen one when no
    /// slot is named), refuses while the body's powers are blocked, and
    /// hands the power to the domain's front end by calling `dispatch`
    /// with the domain and the asked word. The bar is stored and the slot
    /// flashed when the cast starts.
    pub fn cast_slot(
        &mut self,
        record: &mut PowerRecord,
        bar: &mut PowerBar,
        slot: Option<usize>,
        body: &mut CastBody,
        now: u32,
        dispatch: impl FnOnce(u8, u32) -> i32,
    ) -> CastEntry {
        let mut out = CastEntry::default();
        if body.status_bits & super::casting::POWERS_BLOCKED != 0 {
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
        let result = if body.status_bits & 0x10 != 0 {
            super::casting::code::REFUSED
        } else {
            match domain_of(word) {
                DOMAIN_MIND | DOMAIN_FORCE | DOMAIN_ENERGY | DOMAIN_DISPLACEMENT => {
                    dispatch(domain_of(word), word & word::MATCH)
                }
                _ => super::casting::code::REFUSED,
            }
        };
        if result == super::casting::code::NOT_NOW {
            out.result = -1;
            out.stop_cast = true;
            return out;
        }
        if self.bar_open {
            bar.store(record, now);
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

/// The cast entry's answer.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CastEntry {
    /// 1 started, 0 released or refused, −1 "not now".
    pub result: i32,
    /// The running cast must be stopped.
    pub stop_cast: bool,
    pub sound: Option<TreeSound>,
}

/// The sound a refused cast plays.
pub const CAST_FAIL_SOUND: TreeSound = TreeSound {
    name: "ThaumFail01",
    volume: 0.1,
    pitch: 1.0,
};

/// The smoothstep the pan's ease uses: `(3 − 2x)·x²`.
pub fn ease(x: f32) -> f32 {
    (3.0 - 2.0 * x) * (x * x)
}

/// The angle a point makes about a node's centre, measured from +y and
/// signed by x: `acos(dy / length)`, negated when dx is negative. A
/// zero-length vector answers π (its x is not negative).
pub fn link_angle(node_x: f32, node_y: f32, x: f32, y: f32) -> f32 {
    let dx = x - node_x;
    let dy = y - node_y;
    let length = (dy * dy + dx * dx).sqrt();
    let cosine = dy / length;
    let angle = if -1.0 < cosine {
        if cosine <= 1.0 { cosine.acos() } else { 0.0 }
    } else {
        std::f32::consts::PI
    };
    if dx < 0.0 { -angle } else { angle }
}

/// Whether the tree accepts a power dragged with `kind`.
pub fn wants_power_drag(kind: u32) -> bool {
    kind == press::POWER_DRAG
}

/// A power read from a saved record: its id, its prerequisite (one-based
/// into the saved list, 0 for none) and its flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SavedPower {
    pub power: u32,
    pub prerequisite: usize,
    pub flags: u32,
}

/// The flags a revalidated saved power is added with.
pub const SAVED_RESTORED: u32 = 0x9fff;

/// Revalidates a saved power against the tree: the node must be in the
/// tree, the saved list must hold it after its first entry, and its
/// prerequisite's flags must carry [`LEARNED`]. A missing power is added
/// to the known list with [`SAVED_RESTORED`]. Returns whether it was
/// added.
pub fn validate_saved_power(
    nodes: &[PowerNode],
    saved: &[SavedPower],
    known: &mut Vec<KnownPower>,
    power: u32,
) -> bool {
    if !nodes
        .iter()
        .any(|node| node.power & 0xff0f_ffff == power & 0xff0f_ffff)
    {
        return false;
    }
    let index = saved
        .iter()
        .position(|entry| entry.power & word::MATCH == power & word::MATCH);
    let Some(index) = index else {
        return false;
    };
    // The original reads the saved list's second entry onward.
    if index == 0 {
        return false;
    }
    let prerequisite = saved[index].prerequisite;
    if prerequisite == 0
        || prerequisite > saved.len()
        || saved[prerequisite - 1].flags & LEARNED == 0
    {
        return false;
    }
    if known
        .iter()
        .any(|pair| pair.power & word::MATCH == power & word::MATCH)
    {
        return false;
    }
    known.push(KnownPower {
        power,
        flags: SAVED_RESTORED,
    });
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

/// The six domains' colours from the tree's palette (RGBA); index 0 is
/// unused.
pub const DOMAIN_COLOURS: [[u8; 4]; 7] = [
    [0, 0, 0, 0],
    [90, 50, 20, 255],
    [40, 0, 180, 255],
    [100, 0, 0, 255],
    [0, 40, 160, 255],
    [110, 90, 30, 255],
    [60, 65, 90, 255],
];

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
        let mut slots = [draw::SlotView {
            word: 0,
            state: 0,
            alpha: 0.0,
            extra: 0,
        }; 10];
        for (index, slot) in bar.slots.iter().enumerate() {
            slots[index] = draw::SlotView {
                word: slot.word,
                state: slot.state,
                alpha: slot.alpha(now),
                extra: slot.extra,
            };
        }
        draw::TreeView {
            width,
            height,
            scale,
            window: 1.0,
            shown: 1.0,
            splash: 1.0,
            dim: 1.0,
            zoom: self.zoom.value,
            icon_size: self.zoom_fixed,
            prompt: self.mode == TreeMode::Create,
            nodes,
            links,
            bar: slots,
            hovered: self.node_under,
            marked: self.flash,
            hover_until: self.flash_until,
            hover_flip: self.denied,
            marked_until: self.flash_until,
            now,
            domain: self.domain,
            domain_colour: DOMAIN_COLOURS
                .get(usize::from(self.domain))
                .copied()
                .unwrap_or([255; 4]),
            slot_pitch: scale * 17.0,
            slot_size: scale * 13.0,
        }
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
/// power asks for the thaumaturgy skill's technique 2, and a power whose
/// high nibble is set asks for that technique among the casting skill's
/// tiers.
pub fn can_learn(record: &PowerRecord, power: u32) -> bool {
    if is_persistent(power) && !SkillTiers::holds(&record.skills.thaumaturgy, 50_000, 2) {
        return false;
    }
    let technique = technique_of(power);
    technique == 0 || SkillTiers::holds(&record.skills.casting, 50_000, technique)
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
pub mod bar_state {
    /// The slot flashes in.
    pub const FLASH: u32 = 1;
    /// The slot is lit.
    pub const LIT: u32 = 2;
    /// The slot fades out.
    pub const FADE: u32 = 4;
    /// The slot's power is picked or being cast.
    pub const MARK: u32 = 8;
    /// How long the flash in takes.
    pub const FLASH_MS: u32 = 200;
}

impl BarSlot {
    /// The slot's brightness at `now`.
    pub fn alpha(&self, now: u32) -> f32 {
        if self.state & bar_state::FLASH != 0 {
            if self.stamp.wrapping_add(bar_state::FLASH_MS) < now {
                1.0
            } else {
                now.wrapping_sub(self.stamp) as f32 * 0.005
            }
        } else if self.state & bar_state::FADE != 0 {
            if self.stamp.wrapping_add(self.duration) < now {
                0.0
            } else {
                1.0 - now.wrapping_sub(self.stamp) as f32 / self.duration as f32
            }
        } else if self.state & bar_state::LIT != 0 {
            1.0
        } else {
            0.0
        }
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

    /// Loads the bar from a record: the slots lose bits 20 to 23; the
    /// chosen slot is the one with bit 20; each slot whose power is a picked
    /// pair gains the picked mark. Returns the chosen slot (one-based).
    pub fn load(&mut self, record: &PowerRecord) -> usize {
        self.chosen = 0;
        for index in 0..10 {
            let word = record.slots[index] & BAR_LOAD_MASK;
            self.slots[index] = BarSlot {
                word,
                ..BarSlot::default()
            };
            if word & word::CHOSEN != 0 {
                self.chosen = index + 1;
            }
        }
        // The original hands each running power object the matching slot's
        // state through its mark method; an Exarust active power has no
        // mark, so only the bar's own state is kept.
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

    /// Stores the bar into a record, with the chosen slot's bit 20, and
    /// starts the bar's flourish.
    pub fn store(&mut self, record: &mut PowerRecord, now: u32) {
        for slot in &mut self.slots {
            slot.state = (slot.state & bar_state::LIT) | bar_state::FLASH;
            slot.stamp = now;
        }
        if self.chosen == 0 {
            record.slots = self.words();
        } else {
            for index in 0..10 {
                record.slots[index] = self.slots[index].word & !word::CHOSEN;
            }
            if let Some(slot) = record.slots.get_mut(self.chosen - 1) {
                *slot |= word::CHOSEN;
            }
        }
    }

    /// Selects a slot (0-based) whose word is set. Returns whether it
    /// moved.
    pub fn select(&mut self, slot: usize) -> bool {
        if slot > 9 || self.slots[slot].word == 0 {
            return false;
        }
        self.slots[slot].word |= word::CHOSEN;
        let chosen = self.chosen;
        if chosen > 0 && chosen - 1 != slot {
            self.slots[chosen - 1].word &= !word::CHOSEN;
        }
        self.chosen = slot + 1;
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
    /// its bits 20 to 22) with the power word and its flags; a power from
    /// another slot is copied back to it. A miss empties the slot the
    /// power came from. Returns whether a slot was hit.
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
        let target = &mut self.slots[slot];
        target.word = (target.word & 0x70_0000) | drag.power;
        target.state = drag.flags;
        if drag.origin > 0 && drag.origin <= 10 && drag.origin - 1 != slot {
            let copy = *target;
            let source = drag.origin - 1;
            self.slots[source] = copy;
        }
        (true, Some(SLOT_SOUND))
    }

    /// Steps every slot's display state.
    pub fn step(&mut self, now: u32) {
        for slot in &mut self.slots {
            slot.step(now);
        }
    }

    /// Flashes the slot of a cast: a lit or unlit slot flashes in from its
    /// current brightness, a lit one fades out over 200 ms.
    pub fn cast_flash(&mut self, slot: usize, now: u32) {
        let Some(entry) = self.slots.get_mut(slot) else {
            return;
        };
        if entry.state & bar_state::LIT == 0 {
            let shown = if entry.state & bar_state::FLASH == 0 {
                if entry.state & bar_state::FADE == 0 {
                    0.0
                } else {
                    1.0 - now.wrapping_sub(entry.stamp) as f32 / entry.duration as f32
                }
            } else {
                now.wrapping_sub(entry.stamp) as f32 * 0.005
            };
            entry.state |= bar_state::FLASH | bar_state::MARK;
            entry.stamp = now.wrapping_sub((shown * 200.0).round_ties_even() as u32);
        } else {
            entry.state = bar_state::FADE | bar_state::MARK;
            entry.stamp = now;
            entry.duration = 200;
        }
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

/// The pan's spring: the target is nudged by the force made from the gap
/// and the target's velocity, clamped to four units.
fn step_pan(spring: &mut Spring2) -> bool {
    let gap = [
        spring.value[0] - spring.target[0],
        spring.value[1] - spring.target[1],
    ];
    let length = (gap[0] * gap[0] + gap[1] * gap[1]).sqrt();
    if f64::from(length) <= 1e-10 {
        spring.target = spring.value;
        spring.previous = spring.value;
        return false;
    }
    let mut force = [
        spring.target[0] - spring.previous[0],
        spring.target[1] - spring.previous[1],
    ];
    spring.previous = spring.target;
    force = [gap[0] * 0.1 - force[0], gap[1] * 0.1 - force[1]];
    let length = (force[0] * force[0] + force[1] * force[1]).sqrt();
    if length < 16.0 {
        let scale = if length == 0.0 { 0.0 } else { 1.0 / length };
        force = [force[0] * scale, force[1] * scale];
    } else {
        let scale = 4.0 / length.sqrt();
        force = [force[0] * scale, force[1] * scale];
    }
    let length = (force[0] * force[0] + force[1] * force[1]).sqrt();
    if 0.01 < f64::from(length) {
        spring.target = [
            spring.target[0] + force[0] * 0.98,
            spring.target[1] + force[1] * 0.98,
        ];
        return true;
    }
    false
}

/// The zoom's spring, in doubles for the velocity clamp.
fn step_zoom(spring: &mut ZoomSpring) -> bool {
    if f64::from((spring.target - spring.value).abs()) <= 1e-10 {
        spring.value = spring.target;
        spring.previous = spring.target;
        return false;
    }
    let gap = spring.target - spring.value;
    let mut velocity = spring.value - spring.previous;
    spring.previous = spring.value;
    let mut force = gap * 0.1 - velocity;
    if 0.1 <= f64::from(force.abs()) {
        force = (0.1 / f64::from(force.abs()) * f64::from(force)) as f32;
    }
    velocity += force;
    if f64::from(velocity.abs()) <= 5e-05 {
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
        assert_eq!(zoom_level(0.25), -9);
        assert_eq!(zoom_level(0.26), -9);
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
        assert!(step_zoom(&mut zoom));
        assert_eq!(zoom.previous, 0.5);
        assert!((zoom.value - 0.5499).abs() < 1e-6);
        assert!(step_zoom(&mut zoom));
        let mut pan = Spring2 {
            value: [0.0, 0.0],
            target: [10.0, 0.0],
            previous: [10.0, 0.0],
        };
        assert!(step_pan(&mut pan));
        // The force is the gap's tenth, so the target backs off by 0.98.
        assert!((pan.target[0] - 9.02).abs() < 1e-5);
        pan.value = pan.target;
        assert!(!step_pan(&mut pan));
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
        record.skills.casting = vec![0, 0, 0, 1, 0];
        record.skills.thaumaturgy = vec![0, 2, 0, 0];
        assert!(can_learn(&record, 0x1000_0102));
        assert!(!can_learn(&record, 0x2000_0103));
        assert!(can_learn(&record, 0x201));
        record.skills.casting[3] = 0;
        assert!(!can_learn(&record, 0x1000_0102));
        assert!(can_learn(&record, 0x201));
        record.skills.thaumaturgy[1] = 0;
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
        record.skills.thaumaturgy = vec![0, 2, 0, 0];
        let mut bar = PowerBar::default();
        screen.set_focus(2, 0);
        let out = press(
            &mut screen,
            press::LEFT | press::CLICK,
            100,
            Some(&mut record),
            &mut bar,
        );
        assert_eq!(out.sound, Some(SELECT_SOUND));
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
    fn a_slot_drop_copies_and_a_miss_clears() {
        let mut bar = PowerBar::default();
        bar.slots[0] = BarSlot {
            word: 0x101,
            state: 0x4000,
            ..BarSlot::default()
        };
        bar.slots[9] = BarSlot::default();
        let drag = PowerDrag {
            power: 0x102,
            flags: 0x4000,
            kind: press::POWER_DRAG,
            origin: 1,
        };
        let (hit, sound) = bar.drop(194.0 + 68.0 * 9.0, 32.0, 1000.0, &drag);
        assert!(hit);
        assert_eq!(sound, Some(SLOT_SOUND));
        assert_eq!(bar.slots[9].word, 0x102);
        assert_eq!(bar.slots[0].word, 0x102);
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
        bar.store(&mut target, 500);
        assert_eq!(target.slots[0], 0x100101);
        assert_eq!(target.slots[1], 0x201);
        assert_eq!(bar.slots[0].state & bar_state::FLASH, bar_state::FLASH);
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
        assert_eq!(screen.set_focus(1, 10), 3);
        assert_eq!(screen.last_choice, 1);
        assert_eq!(screen.set_focus(1, 20), 0);
        // A special id of a non-creation screen is read: its candidate is 0.
        assert_eq!(screen.set_focus(0x200_0000, 30), 3);
        assert_eq!(screen.last_choice, 0);
        // A node with no power is no candidate, and the candidate stays 0.
        assert_eq!(screen.set_focus(2, 40), 0);
        assert_eq!(screen.last_choice, 0);
        // The second node has no power, so a bar slot keeps the candidate 0.
        assert_eq!(screen.set_focus(focus::SLOT | 1, 50), 0);
        // Returning to the node whose choice was shown gives 0.
        assert_eq!(screen.set_focus(1, 60), 0);
        assert_eq!(screen.shown_choice, 1);
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
        screen.zoom.value = 0.2929;
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
            |domain, word| {
                assert_eq!(domain, DOMAIN_FORCE);
                assert_eq!(word, 0x201);
                1
            },
        );
        assert_eq!(out.result, 1);
        assert_eq!(body.last_instant, 0x201);
        assert!(!screen.bar_open);
        assert_eq!(record.slots[0], 0x100201);
        assert_eq!(bar.slots[0].state & bar_state::MARK, bar_state::MARK);
        // A sustained power clears the last instant power.
        let out = screen.cast_slot(&mut record, &mut bar, Some(1), &mut body, 600, |_, _| 1);
        assert_eq!(out.result, 1);
        assert_eq!(body.last_instant, 0);
        // Blocked powers refuse without dispatching.
        body.status_bits = super::super::casting::POWERS_BLOCKED;
        let out = screen.cast_slot(&mut record, &mut bar, Some(0), &mut body, 700, |_, _| {
            panic!("no dispatch")
        });
        assert_eq!(out.result, 0);
        body.status_bits = 0;
        // Shadow/body domains are refused.
        record.slots[2] = 0x301;
        let out = screen.cast_slot(&mut record, &mut bar, Some(2), &mut body, 800, |_, _| {
            panic!("no dispatch")
        });
        assert_eq!(out.result, 0);
        assert_eq!(out.sound, Some(CAST_FAIL_SOUND));
        // "Not now" stops the running cast.
        let out = screen.cast_slot(&mut record, &mut bar, Some(0), &mut body, 900, |_, _| -3);
        assert!(out.stop_cast);
        assert_eq!(out.result, -1);
    }

    #[test]
    fn the_ease_and_the_link_angle_match_the_original() {
        assert_eq!(ease(0.0), 0.0);
        assert_eq!(ease(1.0), 1.0);
        assert_eq!(ease(0.5), 0.5);
        assert!(ease(0.25) < 0.25);
        assert!(ease(0.75) > 0.75);
        assert!((link_angle(0.0, 0.0, 0.0, 10.0) / std::f32::consts::TAU).abs() < 1e-6);
        assert!((link_angle(0.0, 0.0, 10.0, 0.0) / std::f32::consts::TAU - 0.25).abs() < 1e-6);
        assert!((link_angle(0.0, 0.0, -10.0, 0.0) / std::f32::consts::TAU + 0.25).abs() < 1e-6);
        assert_eq!(link_angle(1.0, 2.0, 1.0, 2.0), std::f32::consts::PI);
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
        assert!(screen.toggle_bar(&mut bar, &mut record, 100));
        assert_eq!(bar.slots[0].word, 0x100101);
        assert_eq!(screen.marks[0], SHOWN | LEARNED | PICKED);
        assert!(!screen.toggle_bar(&mut bar, &mut record, 200));
        assert_eq!(record.slots[0], 0x100101);
        screen.leave(&mut bar, &mut record, 300);
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
    fn a_saved_power_needs_its_prerequisite_learnt() {
        let nodes = [node(0x101, vec![]), node(0x102, vec![])];
        let saved = [
            SavedPower {
                power: 0x100101,
                prerequisite: 0,
                flags: LEARNED,
            },
            SavedPower {
                power: 0x102,
                prerequisite: 1,
                flags: 0,
            },
        ];
        let mut known = Vec::new();
        assert!(validate_saved_power(&nodes, &saved, &mut known, 0x102));
        assert_eq!(
            known,
            vec![KnownPower {
                power: 0x102,
                flags: SAVED_RESTORED
            }]
        );
        // The first saved entry is never revalidated.
        assert!(!validate_saved_power(&nodes, &saved, &mut known, 0x101));
        // A learnt prerequisite is needed.
        let unlearnt = [SavedPower {
            power: 0x102,
            prerequisite: 1,
            flags: 0,
        }];
        let mut known = Vec::new();
        assert!(!validate_saved_power(&nodes, &unlearnt, &mut known, 0x102));
        // A node the tree lacks is refused.
        assert!(!validate_saved_power(&nodes, &saved, &mut known, 0x103));
    }

    #[test]
    fn the_first_opening_picks_the_mind_domain_and_seeds_nothing() {
        assert_eq!(open_domain(false, DOMAIN_DISPLACEMENT), DOMAIN_MIND);
        assert_eq!(open_domain(true, DOMAIN_DISPLACEMENT), DOMAIN_DISPLACEMENT);
        assert_eq!(open_domain(true, DOMAIN_FORCE), DOMAIN_FORCE);
        let mut record = PowerRecord::empty(4.0);
        assert!(seed_defaults(&mut record));
        assert_eq!(record.pairs, DEFAULT_PAIRS);
        assert!(!seed_defaults(&mut record));
        let mut known = PowerRecord::starter();
        assert!(!seed_defaults(&mut known));
        assert_eq!(known.pairs.len(), 2);
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
