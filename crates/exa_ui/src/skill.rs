//! The skill screen's node model and placement.
//!
//! A skill tree has six groups of eight nodes. The character record owns the
//! skill points, the chosen techniques, the prerequisite table and the
//! surplus-choice query ([`exa_character::record`]); this module reads them,
//! lays the ring and its satellites out and models the screen's own rules:
//! the learning row's requirement mask, a node's press state, the screen's
//! unlearning and choosing (which also keep the committed points), the
//! experience pass and ready flag, and the icon, orbit, tooltip, popup and
//! fade geometry. A node code holds a 1-based group in bits 8–15 and a
//! 1-based level in bits 0–2; a non-positive index takes the scaled base or
//! the centre of the bounds.
//!
//! The engine's sine-cosine helper returns the sine first; every ring and
//! pentagon offset puts the sine across and the cosine down (or up).
//!
//! Evidence: the retail skill screen's methods (`docs/UI.md`, Forms).

use exa_character::record::{CharacterData, technique_prerequisites};
use exa_formats::characters::{
    MAX_TOTAL_LEVELS, POINTS_PER_TECHNIQUE, SKILL_NAMES, SKILLS, Skill, TECHNIQUE_NAMES,
    WRITTEN_TECHNIQUES,
};

/// The nodes a skill group can hold (1-based node ids 1 to 8).
pub const NODES_PER_GROUP: usize = 8;
/// A ring node's press state: nothing happens.
pub const NODE_LOCKED: u8 = 0;
/// A ring node's press state: the level is learnt; a press asks to forget it.
pub const NODE_LEARNT: u8 = 1;
/// A ring node's press state: the level is open; a press opens the learning
/// row.
pub const NODE_OPEN: u8 = 2;
/// A learning-row slot that may be chosen.
pub const REQUIREMENT_MET: u8 = 0;
/// A learning-row slot whose technique the record already holds.
pub const REQUIREMENT_CHOSEN: u8 = 1;
/// A learning-row slot whose prerequisites are not funded.
pub const REQUIREMENT_MISSING: u8 = 2;
/// A learning-row slot the tree has no node for.
pub const REQUIREMENT_ABSENT: u8 = 3;

/// The skill screen's bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillBounds {
    pub left: i32,
    pub right: i32,
    pub top: i32,
    pub bottom: i32,
}

/// Places a skill node from its index.
pub fn skill_position(
    index: i32,
    bounds: SkillBounds,
    scaled: bool,
    base: (i32, i32),
    layout: impl FnOnce(i32, i32) -> (f32, f32),
) -> (i32, i32) {
    if index > 0 {
        let (x, y) = layout((index >> 8) & 0xff, index & 0xff);
        (x.round_ties_even() as i32, y.round_ties_even() as i32)
    } else if scaled {
        base
    } else {
        (
            (bounds.right - bounds.left) / 2,
            (bounds.bottom - bounds.top) / 2,
        )
    }
}

/// The skill screen after showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillShow {
    pub relayout: bool,
    pub state: u16,
    pub deadline: i32,
}

/// Shows the skill screen: relayout, set bit 1 and stamp the deadline.
pub fn show_skill_ui(relayout: bool, state: u16, clock: i32, delay: i32) -> SkillShow {
    SkillShow {
        relayout,
        state: state | 2,
        deadline: clock + delay,
    }
}

/// What closing the skill screen did, as its close routine does it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SkillClose {
    /// The screen was open and is now released.
    pub closed: bool,
    /// Player controls were restored (the routine only does this when a
    /// controls object was held).
    pub controls_restored: bool,
    /// The player body's state was reset.
    pub body_released: bool,
    /// The player's character was refit.
    pub character_refit: bool,
}

/// Closes the skill screen: a nil screen returns false and does nothing;
/// otherwise the form is released, and with the player's controls held it
/// restores them, resets the body state and refits the character, then
/// reports that it closed.
pub fn close_skill_screen(open: bool, has_controls: bool) -> SkillClose {
    if !open {
        return SkillClose::default();
    }
    SkillClose {
        closed: true,
        controls_restored: has_controls,
        body_released: has_controls,
        character_refit: has_controls,
    }
}

/// What opening (or toggling) the skill screen did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillOpenPlan {
    /// The screen was already open, so the open routine only closed it.
    pub close_existing: bool,
    /// The unit's atlases were built.
    pub build_resources: bool,
    /// The reveal sound was started.
    pub play_sound: bool,
    /// Player controls were suspended.
    pub restore_controls: bool,
    /// The screen's page: 0 is the ring, a positive value a four-group page.
    pub page: i32,
    /// The groups on the page: six on the ring, four on a page.
    pub groups: i32,
    /// A page's ring radius in units (the ring keeps the constructor's).
    pub radius: i32,
    /// The screen's initial state byte (4 on a page: it opens already shown).
    pub state: u8,
    pub show: bool,
    pub activate: bool,
    /// The Steam action bindings were set up for the screen.
    pub steam_bind: bool,
}

/// Opens or closes the skill screen. With it already open the routine closes
/// it and does nothing else; otherwise it builds the unit's atlases, starts
/// the reveal sound when the handle is ready, suspends the player's
/// controls, builds the screen (a positive page turns it into a four-group
/// page at a 72-unit radius in state 4, the page group and slot tables set),
/// refreshes its ready flag and shows and activates it.
pub fn open_skill_screen(
    open: bool,
    page: i32,
    has_controls: bool,
    sound_ready: bool,
) -> SkillOpenPlan {
    if open {
        return SkillOpenPlan {
            close_existing: true,
            build_resources: false,
            play_sound: false,
            restore_controls: false,
            page: 0,
            groups: 0,
            radius: 0,
            state: 0,
            show: false,
            activate: false,
            steam_bind: false,
        };
    }
    let page = page.max(0);
    let paged = page > 0;
    SkillOpenPlan {
        close_existing: false,
        build_resources: true,
        play_sound: sound_ready,
        restore_controls: has_controls,
        page,
        groups: if paged { 4 } else { 6 },
        radius: if paged { 72 } else { 0 },
        state: if paged { 4 } else { 1 },
        show: true,
        activate: true,
        steam_bind: true,
    }
}

/// What leaving the skill screen did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SkillLeave {
    pub call_base: bool,
    pub call_clear: bool,
}

/// Leaves the skill screen, waiting out the page and the transition's tail.
pub fn leave_skill_ui(
    closing: bool,
    page: i32,
    state: u16,
    deadline: i32,
    clock: i32,
) -> SkillLeave {
    if closing {
        return SkillLeave {
            call_base: true,
            ..SkillLeave::default()
        };
    }
    if page != 0 {
        return SkillLeave::default();
    }
    if state & 1 != 0 && deadline - 400 > clock {
        return SkillLeave::default();
    }
    if state & 2 != 0 {
        return SkillLeave::default();
    }
    SkillLeave {
        call_clear: true,
        ..SkillLeave::default()
    }
}

/// The skill ring's layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkillRing {
    pub count: i32,
    /// The ring radius in units (64, 58 with the inset layout flag, 72 on a
    /// four-group page).
    pub radius: i32,
    /// The horizontal stretch (1.0 as built); it scales only the x offset.
    pub stretch: f32,
    /// The reveal scale the draw stores; it also turns the ring as it opens.
    pub turn: f32,
    pub width: i32,
    pub height: i32,
    pub page: i32,
    pub unit: i32,
    /// The global layout flag that moves a page's ring one unit left.
    pub scrolled: bool,
}

/// The ring's centre: half the width (one unit less on a page with the
/// global layout flag) and half the height, three units higher on the ring
/// page.
fn ring_centre(ring: &SkillRing) -> (f32, f32) {
    let mut x = (ring.width / 2) as f32;
    if ring.scrolled && ring.page > 0 {
        x -= ring.unit as f32;
    }
    let y = ring.height / 2 - if ring.page == 0 { 3 * ring.unit } else { 0 };
    (x, y as f32)
}

/// The angle of a ring slot: `(1 − turn) + (slot − shift)·2π / count`, the
/// sum in the original's order.
fn ring_angle(slot: f32, shift: f32, ring: &SkillRing) -> f32 {
    (1.0 - ring.turn) + ((slot - shift) * std::f32::consts::TAU) / ring.count as f32
}

/// A ring node's offset from the centre: the radius times the unit times the
/// engine's sine across (stretched by `stretch`) and its cosine down, each
/// rounded half to even and scaled by the reveal.
fn ring_offset(angle: f32, ring: &SkillRing) -> (f32, f32) {
    let unit = ring.unit as f32;
    let radius = ring.radius as f32;
    let x = (radius * (unit * angle.sin()) * ring.stretch).round_ties_even() * ring.turn;
    let y = (radius * (unit * angle.cos())).round_ties_even() * ring.turn;
    (x, y)
}

/// Places a skill node on the ring. The engine's sine-cosine helper hands
/// back the sine first, and the x offset is the sine.
pub fn skill_ring_position(slot: i32, ring: &SkillRing) -> (f32, f32) {
    let (cx, cy) = ring_centre(ring);
    let (ox, oy) = ring_offset(ring_angle(slot as f32, 3.5, ring), ring);
    (ox + cx, oy + cy)
}

/// The skill ring's hit test inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkillHit {
    pub mouse: (i32, i32),
    pub unit: i32,
    pub turn: f32,
    pub page: i32,
}

/// Tests the five samples around every node.
pub fn skill_node_hit(hit: SkillHit, nodes: &[(f32, f32)]) -> i32 {
    let unit = f64::from(hit.unit);
    let outer = (unit * 23.0) as f32;
    let inner = (unit * 5.0) as f32;
    let radius = unit * 18.3;
    let mut result = if hit.page == 2 { 0 } else { -1 };
    for &(x, y) in nodes {
        let dx = hit.mouse.0 as f32 - x;
        let dy = hit.mouse.1 as f32 - y;
        if dx * dx + dy * dy > outer * outer {
            continue;
        }
        for j in 0..5 {
            let angle = j as f32 * std::f32::consts::TAU * 0.2;
            let ox = ((radius * f64::from(angle.sin())).round_ties_even() as f32) * hit.turn;
            let oy = ((radius * f64::from(angle.cos())).round_ties_even() as f32) * hit.turn;
            let ddx = hit.mouse.0 as f32 - (x + ox);
            let ddy = hit.mouse.1 as f32 - (y - oy);
            if ddx * ddx + ddy * ddy < inner * inner {
                result = (j | result) + 1;
                break;
            }
        }
    }
    result
}

/// The skill hover's inputs.
#[derive(Debug, Clone, Copy)]
pub struct SkillHoverInput<'a> {
    pub index: i32,
    pub current: i32,
    pub grid: Option<&'a [u8]>,
    pub previous: i32,
    pub last_positive: i32,
    pub stamp: i32,
    pub clock: i32,
    pub base: i32,
}

/// The skill hover's result and state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillHover {
    pub result: i32,
    pub current: i32,
    pub previous: i32,
    pub last_positive: i32,
    pub stamp: i32,
    pub called_base: bool,
}

/// Sets the skill screen's hovered node.
pub fn skill_hover(input: SkillHoverInput<'_>) -> SkillHover {
    let mut out = SkillHover {
        result: input.base,
        current: input.current,
        previous: input.previous,
        last_positive: input.last_positive,
        stamp: input.stamp,
        called_base: false,
    };
    if input.index <= 0 && input.current <= 0 {
        out.called_base = true;
        return out;
    }
    out.result = 3;
    out.current = input.index;
    let Some(grid) = input.grid else {
        return out;
    };
    let candidate = if input.index > 0 && input.index & 0x700 > 0 && input.index & 7 > 0 {
        let at = (((input.index >> 8) & 7) * 8 + (input.index & 7) + 1) as usize;
        if grid.get(at).is_some_and(|&byte| byte != 0) {
            input.index
        } else {
            0
        }
    } else {
        0
    };
    if candidate == input.previous {
        return out;
    }
    if candidate > 0 && input.last_positive > 0 && input.last_positive != candidate {
        out.stamp = input.clock;
    } else {
        let elapsed = input.clock - input.stamp;
        out.stamp = if elapsed >= 200 {
            input.clock
        } else {
            input.clock - (200 - elapsed)
        };
    }
    out.previous = candidate;
    if candidate > 0 {
        out.last_positive = candidate;
    }
    out
}

/// Places the stored slot's satellite at a sample index. `stored` is the
/// group's ring slot from the screen's group-to-slot table (0-based, hence
/// the `2.5` shift); the sample's offset is `18.3` units at a fifth of a
/// turn per sample, the sine across and the cosine up, rounded in double
/// precision.
pub fn skill_satellite(stored: i32, sample: i32, ring: &SkillRing) -> (f32, f32) {
    let (cx, cy) = ring_centre(ring);
    let (ox, oy) = ring_offset(ring_angle(stored as f32, 2.5, ring), ring);
    let (node_x, node_y) = (ox + cx, oy + cy);
    let (sx, sy) = pentagon_offset(sample - 1, ring.unit);
    (sx * ring.turn + node_x, node_y - sy * ring.turn)
}

/// The offset of a group's `slot`-th (0-based) pentagon point: `18.3`
/// units at a fifth of a turn per slot, the sine across and the cosine up,
/// each rounded half to even in double precision. Slot 0 is straight up.
pub fn pentagon_offset(slot: i32, unit: i32) -> (f32, f32) {
    let angle = slot as f32 * std::f32::consts::TAU * 0.2;
    let radius = f64::from(unit) * 18.3;
    (
        (f64::from(angle.sin()) * radius).round_ties_even() as f32,
        (f64::from(angle.cos()) * radius).round_ties_even() as f32,
    )
}

/// The skill screen key handler's inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillKeyInput {
    pub key: i32,
    pub low: i32,
    pub high: i32,
    pub index: i32,
    pub state: u16,
    pub hide: u8,
    pub deadline: i32,
    pub closing: i32,
    pub clock: i32,
    pub grid: [u8; 8],
    pub repage: i32,
    pub hit: i32,
    pub flags: u32,
}

/// What the skill screen key handler did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillKeyOutput {
    pub index: i32,
    pub high: i32,
    pub low: i32,
    pub published: Option<i32>,
    pub base: bool,
    pub hint: bool,
    pub state: u16,
    pub hide: u8,
    pub deadline: i32,
    pub flags: u32,
    pub cursor: bool,
    pub hit_published: Option<i32>,
    pub clear_call: bool,
    pub result: i32,
}

fn skill_key_tail(input: SkillKeyInput, mut out: SkillKeyOutput) -> SkillKeyOutput {
    if input.closing == 1 {
        return out;
    }
    if input.key & 1 != 0 {
        out.cursor = true;
        if input.hit != 0 {
            out.hit_published = Some(input.hit);
        }
    }
    if input.key & 0x200 != 0 && input.state & 2 == 0 {
        out.clear_call = true;
    }
    out
}

/// Handles a key on the skill screen.
pub fn skill_screen_key(input: SkillKeyInput) -> SkillKeyOutput {
    let mut out = SkillKeyOutput {
        index: input.index,
        high: input.high,
        low: input.low,
        published: None,
        base: false,
        hint: false,
        state: input.state,
        hide: input.hide,
        deadline: input.deadline,
        flags: input.flags,
        cursor: false,
        hit_published: None,
        clear_call: false,
        result: 0,
    };
    if input.low > 0 || input.high > 0 {
        if input.key & 1 == 0 {
            if input.key & 0x100 != 0 {
                out.base = true;
                out.published = Some(if input.high > 0 {
                    input.high
                } else {
                    input.low
                });
            } else if input.key & 0x200 != 0 {
                out.hint = true;
                if input.hide == 0 {
                    out.hide = 2;
                    out.deadline = input.clock + 200;
                }
                out.flags &= !0x40;
                out.published = Some(if input.high > 0 {
                    input.high
                } else {
                    input.low
                });
            }
            return out;
        }
        if input.high > 0 {
            if input.key & 0xf001 == 0x1001 {
                out.index -= 1;
            } else if input.key & 0xf001 == 0x2001 && out.index < 2 {
                out.index += 1;
            }
        } else if input.key & 0xf001 == 0x1001 {
            let mut at = out.index - 2;
            while at >= 0 {
                if input.grid[at as usize] == 0 {
                    out.index = at + 1;
                    break;
                }
                at -= 1;
            }
        } else if input.key & 0xf001 == 0x2001 && out.index <= 7 {
            let mut at = out.index - 1;
            loop {
                at += 1;
                if input.grid[at as usize] == 0 {
                    out.index = at + 1;
                    break;
                }
                if at >= 7 {
                    break;
                }
            }
        }
        if out.index > 0 && out.index != input.index {
            out.published = Some(out.index);
        }
        return out;
    }
    if input.key != 0x100 {
        return skill_key_tail(input, out);
    }
    match input.repage {
        1 => {
            out.high = input.index;
            out.published = Some(1);
        }
        2 => {
            out.low = input.index;
            let mut at = 0;
            while at < 8 {
                if input.grid[at] == 0 {
                    out.index = at as i32 + 1;
                    break;
                }
                at += 1;
            }
            if at == 8 {
                out.index = 0;
            }
            out.published = Some(out.index);
        }
        _ => return out,
    }
    out.hint = true;
    out.hide = 1;
    out.deadline = input.clock + 300;
    out.flags |= 0x40;
    skill_key_tail(input, out)
}

/// The skill screen pointer input's inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillInputInput {
    pub index: i32,
    pub mask: u16,
    pub page: i32,
    pub closing: i32,
    pub state: u16,
    /// The node awaiting the forget confirmation.
    pub high: i32,
    /// The node whose learning row is open.
    pub low: i32,
    pub hide: u8,
    /// The panel only shows the skills (an arena recruit's or hireling's):
    /// a node click does nothing.
    pub read_only: bool,
    pub clock: i32,
    /// The learning row's requirement mask; a row slot (1–8) reads byte
    /// `slot − 1`.
    pub used: [u8; 8],
    pub repage: i32,
    pub selected: i32,
    pub base: i32,
    pub delay: i32,
    /// The open state's deadline, 500 ms after state 4 turns to 1 (the
    /// screen does not close until 400 ms before it).
    pub open_end: i32,
    /// When the close fade ends.
    pub close_end: i32,
    /// When the hint panel's fade ends (300 ms in, 200 ms out).
    pub hint_fade_end: i32,
}

/// What the skill screen pointer input did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillInputOutput {
    pub result: i32,
    pub index: i32,
    pub high: i32,
    pub low: i32,
    pub state: u16,
    pub hide: u8,
    pub open_end: i32,
    pub close_end: i32,
    pub hint_fade_end: i32,
    pub clear: bool,
    pub hint: bool,
    pub select: bool,
    pub fill: bool,
    pub slot: Option<(i32, i32)>,
    pub base: bool,
}

fn skill_input_finish(input: &SkillInputInput, out: &mut SkillInputOutput) {
    if out.index < 0 {
        out.base = true;
        out.result = input.base;
    }
}

/// Handles a pointer input on the skill screen.
pub fn handle_skill_input(input: SkillInputInput) -> SkillInputOutput {
    let mut out = SkillInputOutput {
        result: 0x10,
        index: input.index,
        high: input.high,
        low: input.low,
        state: input.state,
        hide: input.hide,
        open_end: input.open_end,
        close_end: input.close_end,
        hint_fade_end: input.hint_fade_end,
        clear: false,
        hint: false,
        select: false,
        fill: false,
        slot: None,
        base: false,
    };
    if input.index < -1 {
        out.base = true;
        out.result = input.base;
        return out;
    }
    if input.mask & 2 != 0 {
        if input.page > 0 {
            return out;
        }
        if input.closing != 0 {
            if input.closing == 1 {
                return out;
            }
            skill_input_finish(&input, &mut out);
            return out;
        }
        if out.state & 2 == 0 {
            out.clear = true;
        }
        out.result = 1;
        return out;
    }
    if out.index <= 0 {
        if input.low <= 0 && input.high <= 0 {
            skill_input_finish(&input, &mut out);
            return out;
        }
        if out.hide == 0 {
            out.hide = 2;
            out.hint_fade_end = input.clock + 200;
        }
        out.hint = true;
        skill_input_finish(&input, &mut out);
        return out;
    }
    if out.index == 0x10000 {
        if input.mask & 0x2000 == 0 {
            return out;
        }
        if out.state as u8 == 4 {
            out.state = 1;
            out.open_end = input.clock + 500;
            return out;
        }
        if out.state & 2 != 0 {
            return out;
        }
        out.state |= 2;
        out.close_end = input.clock + input.delay;
        return out;
    }
    if input.read_only {
        return out;
    }
    if input.mask & 0x2000 == 0 {
        skill_input_finish(&input, &mut out);
        return out;
    }
    if input.low <= 0 && input.high <= 0 {
        match input.repage {
            1 => out.high = out.index,
            2 => {
                out.low = out.index;
                out.fill = true;
            }
            _ => return out,
        }
        out.hint = true;
        out.hide = 1;
        out.hint_fade_end = input.clock + 300;
        skill_input_finish(&input, &mut out);
        return out;
    }
    if out.high > 0 {
        if out.index == 1 {
            out.select = true;
        }
    } else if input
        .used
        .get((out.index - 1) as usize)
        .copied()
        .unwrap_or(0)
        == 0
    {
        if input.selected > 0 {
            out.select = true;
        }
        let row = ((out.low >> 8) & 7) - 1;
        let column = out.low & 7;
        out.slot = Some((row, column));
    }
    out.hint = true;
    if out.hide == 0 {
        out.hide = 2;
        out.hint_fade_end = input.clock + 200;
    }
    skill_input_finish(&input, &mut out);
    out
}

/// The skill screen point hit test's inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkillPointInput {
    pub state: u16,
    pub page: i32,
    pub base: i32,
    pub owns: bool,
    pub allowed: bool,
    pub point: (i32, i32),
    pub width: i32,
    pub height: i32,
    pub unit: i32,
    /// The node awaiting the forget confirmation; checked first.
    pub high: i32,
    /// The node whose learning row is open.
    pub low: i32,
    /// `high`'s position after the 41-unit tooltip clamp.
    pub high_point: (f32, f32),
    /// `low`'s position after the 41-unit tooltip clamp.
    pub low_point: (f32, f32),
    pub fallback: i32,
}

/// Hit-tests a point against the skill screen's bands and nodes.
pub fn hit_skill_point(input: SkillPointInput) -> i32 {
    if input.state & 2 != 0 && input.page < 2 {
        return 0;
    }
    if !input.owns {
        return input.base;
    }
    if input.base != -1 && !input.allowed {
        return input.base;
    }
    let (x, y) = input.point;
    let unit = input.unit;
    let centre_x = input.width / 2;
    if input.page == 2
        && y > input.height - 16 * unit
        && y < input.height - 8 * unit
        && x > centre_x - 6 * unit
        && x < centre_x + 6 * unit
    {
        return 0x1_0000;
    }
    if input.state as u8 == 4 {
        return input.base;
    }
    // `high` holds the node awaiting the forget confirmation (its point
    // already moved into the window by the 41-unit tooltip clamp): Yes and
    // No sit six to ten units below it, Yes 4 to 13 units left, No 4 to 13
    // units right of the rounded (half to even) offset.
    if input.high > 0 {
        let (px, py) = input.high_point;
        let top = (6 * unit) as f32 + py;
        if top > y as f32 {
            return -1;
        }
        if (4 * unit) as f32 + top < y as f32 {
            return -1;
        }
        let dx = (x as f32 - px).round_ties_even() as i32;
        if -4 * unit > dx && -13 * unit < dx {
            return 1;
        }
        if 4 * unit < dx && 13 * unit > dx {
            return 2;
        }
        return -1;
    }
    // `low` holds the node whose learning row is open (its point clamped
    // the same way): the row's eight slots lie in the eight units above it,
    // ten units apart, the slot rounded half to even.
    if input.low > 0 {
        let (px, py) = input.low_point;
        let top = py - (8 * unit) as f32;
        if top > y as f32 {
            return -1;
        }
        if (8 * unit) as f32 + top < y as f32 {
            return -1;
        }
        let slot = ((x as f32 - px) / (10 * unit) as f32 + 4.5).round_ties_even() as i32;
        if slot > 0 && slot < 9 {
            return slot;
        }
        return -1;
    }
    input.fallback
}

/// The skill link search's inputs.
#[derive(Clone, Copy)]
pub struct SkillLinkInput<'a> {
    pub point: (i32, i32),
    pub direction: (f32, f32),
    pub hit: i32,
    pub node_point: (f32, f32),
    pub nodes: &'a [(i32, (f32, f32))],
    pub reachable: &'a dyn Fn(i32) -> bool,
    pub unit: i32,
    pub turn: f32,
}

/// The chosen skill link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillLink {
    pub code: i32,
    pub point: (i32, i32),
}

fn link_satellite(node: (f32, f32), slot: i32, unit: i32, turn: f32) -> (f32, f32) {
    let (sx, sy) = pentagon_offset(slot, unit);
    (sx * turn + node.0, node.1 - sy * turn)
}

fn link_metric(
    point: (i32, i32),
    satellite: (f32, f32),
    direction: (f32, f32),
    scale_y: bool,
) -> (f32, f32) {
    let dx = satellite.0 - point.0 as f32;
    let mut dy = satellite.1 - point.1 as f32;
    if scale_y {
        dy *= 4.0;
    }
    let length = (dx * dx + dy * dy).sqrt();
    let (nx, ny) = if length > 0.0 {
        (dx / length, dy / length)
    } else {
        (f32::NAN, f32::NAN)
    };
    (length, nx * direction.0 + ny * direction.1)
}

/// Searches the adjacent slots then every node's five slots for a link.
pub fn pick_skill_satellite(input: SkillLinkInput<'_>) -> SkillLink {
    let scale_y = input.direction.1.abs() < input.direction.0.abs();
    let point_of = |satellite: (f32, f32)| {
        (
            satellite.0.round_ties_even() as i32,
            satellite.1.round_ties_even() as i32,
        )
    };
    if input.hit > 0 {
        let slot = (input.hit & 7) - 1;
        let mut best_dot = 0.1f32;
        let mut best: Option<(i32, (f32, f32))> = None;
        for step in 0..2 {
            let mut at = 2 * step - 1 + slot;
            if at < 0 {
                at = 4;
            }
            if at > 4 {
                at = 0;
            }
            let code = (input.hit & 0xff00) | (at + 1);
            if !(input.reachable)(code) {
                continue;
            }
            let satellite = link_satellite(input.node_point, at, input.unit, input.turn);
            let (_, dot) = link_metric(input.point, satellite, input.direction, scale_y);
            if dot > best_dot {
                best_dot = dot;
                best = Some((code, satellite));
            }
        }
        if let Some((code, satellite)) = best {
            return SkillLink {
                code,
                point: point_of(satellite),
            };
        }
    }
    let mut best_metric = 1_000_000.0f32;
    let mut best: Option<(i32, (f32, f32))> = None;
    for &(high, node) in input.nodes {
        for at in 0..5 {
            let code = high | (at + 1);
            if !(input.reachable)(code) {
                continue;
            }
            let satellite = link_satellite(node, at, input.unit, input.turn);
            let (length, dot) = link_metric(input.point, satellite, input.direction, scale_y);
            if length * length < 1.0 {
                continue;
            }
            if f64::from(dot) < 0.3 {
                continue;
            }
            let metric = length / (dot * dot);
            if metric < best_metric {
                best_metric = metric;
                best = Some((code, satellite));
            }
        }
    }
    match best {
        Some((code, satellite)) => SkillLink {
            code,
            point: point_of(satellite),
        },
        None => SkillLink {
            code: 0,
            point: input.point,
        },
    }
}

/// The skill screen's fade.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkillFade {
    pub alpha: f32,
    pub sound: f32,
    pub cleared: bool,
}

/// Fades the screen toward its deadline.
pub fn skill_screen_fade(state: u8, deadline: i32, clock: i32) -> SkillFade {
    if state == 4 {
        return SkillFade {
            alpha: 0.0,
            sound: 0.0,
            cleared: false,
        };
    }
    if state & 1 == 0 {
        return SkillFade {
            alpha: 1.0,
            sound: 1.0,
            cleared: false,
        };
    }
    if deadline <= clock {
        return SkillFade {
            alpha: 1.0,
            sound: 1.0,
            cleared: true,
        };
    }
    let t = f64::from(deadline - clock) * 0.002;
    let x = (1.0 - t) as f32;
    let x = x * (2.0 - x);
    SkillFade {
        alpha: x.min(1.0),
        sound: x * (2.0 - x),
        cleared: false,
    }
}

/// The skill screen's fade-in result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkillFadeIn {
    pub state: u8,
    pub sound: f32,
    pub closed: bool,
    pub alpha: Option<f32>,
}

/// Runs the draw's bit-1 fade-in.
pub fn skill_screen_fade_in(
    state: u8,
    deadline: i32,
    clock: i32,
    sound: f32,
    delay: f32,
    page: i32,
) -> SkillFadeIn {
    if state & 2 == 0 {
        return SkillFadeIn {
            state,
            sound,
            closed: false,
            alpha: None,
        };
    }
    if deadline <= clock {
        if page == 2 {
            return SkillFadeIn {
                state: 4,
                sound: 0.0,
                closed: false,
                alpha: Some(0.0),
            };
        }
        return SkillFadeIn {
            state: 4,
            sound,
            closed: true,
            alpha: None,
        };
    }
    SkillFadeIn {
        state,
        sound: (deadline - clock) as f32 * sound / delay,
        closed: false,
        alpha: None,
    }
}

/// The draw's two fade values, as the draw body stores them: `alpha` is
/// the content alpha (it also turns and spreads the ring: the node plates,
/// spokes, icons and orbit bars multiply their offsets by it), `scale` is
/// the opening scale (it also scales the shader alphas, the dim and the
/// tooltips). The hide clock scales the opening scale, not the alpha.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkillFades {
    pub alpha: f32,
    pub scale: f32,
    /// The state byte after the fade's side effects.
    pub state: u8,
    /// The screen is hidden and the draw returns.
    pub hidden: bool,
}

/// The screen's two fades, exactly as the draw body computes them.
///
/// The reveal runs for its deadline at 0.002 per unit: `t = 1 − remaining ×
/// 0.002`, the opening scale `t(2 − t)` clamped to 1 (an unordered compare
/// skips the clamp on a NaN), the alpha that eased again. Once over, bit 1
/// clears. The hide, while it is running, scales the opening scale by its
/// remaining share of 256 units; once over the state becomes hidden, on a
/// page other than two the draw closes the screen, and both fades read
/// zero.
pub fn skill_screen_fades(
    state: u8,
    reveal_deadline: i32,
    hide_deadline: i32,
    clock: i32,
    page: i32,
) -> SkillFades {
    let mut state = state;
    let (mut alpha, mut scale) = if state == 4 {
        (0.0, 0.0)
    } else if state & 1 == 0 {
        (1.0, 1.0)
    } else if reveal_deadline > clock {
        let t = (1.0 - f64::from(reveal_deadline - clock) * 0.002) as f32;
        let eased = t * (2.0 - t);
        // The clamp is ordered: a NaN passes unchanged.
        let opening = if eased > 1.0 { 1.0 } else { eased };
        (eased * (2.0 - eased), opening)
    } else {
        state &= !1;
        (1.0, 1.0)
    };
    let mut hidden = false;
    if state & 2 != 0 {
        if hide_deadline > clock {
            scale = (hide_deadline - clock) as f32 * scale / 256.0;
        } else {
            state = 4;
            hidden = page != 2;
            alpha = 0.0;
            scale = 0.0;
        }
    }
    SkillFades {
        alpha,
        scale,
        state,
        hidden,
    }
}

/// The status line of a learning-row node the record already holds.
pub const STATUS_CHOSEN: &str = "Already chosen.";
/// The status line of a learning-row node whose prerequisites are missing.
pub const STATUS_REQUIREMENTS: &str = "Requirements not met.";
/// The status line of a learning-row slot the tree has no node for.
pub const STATUS_UNAVAILABLE: &str = "Unavailable.";
/// The forget dialog's title.
pub const FORGET_TITLE: &str = "Forget Technique";
/// The forget dialog's question.
pub const FORGET_QUESTION: &str = "Forget all knowledge of this technique?";

/// The six skill groups' names, by 0-based group.
pub const SKILL_GROUP_NAMES: [&str; SKILLS] = SKILL_NAMES;

/// The name of the technique a learning-row slot offers: `slot` is the
/// 1-based technique id (the row's icon slot), and `code` the group's node
/// code. An id outside the group's names reads as the empty string.
pub fn technique_name_for(code: i32, slot: u8) -> String {
    let group = ((code >> 8) & 0xff) as usize;
    if group == 0 {
        return String::new();
    }
    TECHNIQUE_NAMES
        .get(group - 1)
        .and_then(|names| names.get(usize::from(slot).wrapping_sub(1)))
        .copied()
        .unwrap_or("")
        .to_string()
}

/// The record's skill block as the screen addresses it: each skill is its
/// points (two bytes, little-endian) then its six technique bytes, eight
/// bytes a skill, so an index past a skill's techniques reads on into the
/// next skill. The screen's loops do read that far (a level count can pass
/// six); bytes past the sixth skill belong to the record's next field,
/// which this model reads as 0 and never writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SkillBlock([u8; SKILLS * 8]);

impl SkillBlock {
    fn of(skills: &[Skill; SKILLS]) -> Self {
        let mut bytes = [0u8; SKILLS * 8];
        for (at, skill) in skills.iter().enumerate() {
            let entry = &mut bytes[at * 8..at * 8 + 8];
            entry[..2].copy_from_slice(&skill.points.to_le_bytes());
            entry[2..].copy_from_slice(&skill.techniques);
        }
        Self(bytes)
    }

    fn store(&self, skills: &mut [Skill; SKILLS]) {
        for (at, skill) in skills.iter_mut().enumerate() {
            let entry = &self.0[at * 8..at * 8 + 8];
            skill.points = u16::from_le_bytes([entry[0], entry[1]]);
            skill.techniques.copy_from_slice(&entry[2..]);
        }
    }

    fn byte(&self, at: usize) -> u8 {
        self.0.get(at).copied().unwrap_or(0)
    }

    fn set_byte(&mut self, at: usize, value: u8) {
        if let Some(byte) = self.0.get_mut(at) {
            *byte = value;
        }
    }

    /// The `index`-th technique byte of a 0-based skill.
    fn technique(&self, skill: usize, index: usize) -> u8 {
        self.byte(skill * 8 + 2 + index)
    }

    fn points(&self, skill: usize) -> u16 {
        u16::from_le_bytes([self.byte(skill * 8), self.byte(skill * 8 + 1)])
    }

    fn set_points(&mut self, skill: usize, points: u16) {
        let [low, high] = points.to_le_bytes();
        self.set_byte(skill * 8, low);
        self.set_byte(skill * 8 + 1, high);
    }
}

/// Which learning-row nodes the tree has: the screen's own node table marks
/// each of a group's eight slots present or not. Ranged Combat has none;
/// Close Combat's fifth to seventh (Draw, Impel, Brace), Armour's sixth to
/// eighth and Shields' seventh and eighth are absent.
pub const NODE_PRESENT: [[bool; NODES_PER_GROUP]; SKILLS] = [
    [true, true, true, true, false, false, false, true],
    [false; NODES_PER_GROUP],
    [true, true, true, true, true, false, false, false],
    [true, true, true, true, true, true, false, false],
    [true; NODES_PER_GROUP],
    [true; NODES_PER_GROUP],
];

/// The skill the screen draws when it has no record: 13 200 points with
/// techniques 1, 3 and 5 chosen, a preview the group icons and orbit bars
/// read in place of the character's.
pub const PREVIEW_SKILL: Skill = Skill {
    points: 13_200,
    techniques: [1, 3, 5, 0, 0, 0],
};

/// The ring page's slot order: the group (0-based) each of the six ring
/// slots shows, clockwise from the first.
pub const RING_GROUPS: [usize; 6] = [0, 2, 4, 5, 3, 1];
/// The inverse of [`RING_GROUPS`]: the 0-based ring slot of each group.
pub const GROUP_SLOTS: [usize; 6] = [0, 5, 1, 4, 2, 3];
/// A four-group page's slot order (the screen opened on a page).
pub const PAGE_GROUPS: [usize; 4] = [1, 0, 2, 3];
/// The inverse of [`PAGE_GROUPS`].
pub const PAGE_SLOTS: [usize; 4] = [1, 0, 2, 3];

/// The requirement mask of a group's learning row: one byte per node slot.
///
/// The chosen techniques of the five written slots mark their nodes
/// [`REQUIREMENT_CHOSEN`]; a slot the tree has no node for becomes
/// [`REQUIREMENT_ABSENT`] (over a chosen mark); then every still-open slot
/// whose prerequisite (each of the two, in turn) is not among the
/// techniques the points fund becomes [`REQUIREMENT_MISSING`]. Both
/// prerequisites must be held: Dual Wield wants Remise and Riposte. The
/// funded run is read through the record's byte layout, so a level count
/// past six reads the next skill's bytes, as the original does. The
/// prerequisite ids are the character record's table
/// ([`technique_prerequisites`]); the screen keeps an identical copy.
/// `skill` is 0-based.
pub fn skill_requirements(skill: usize, skills: &[Skill; SKILLS]) -> [u8; NODES_PER_GROUP] {
    let mut mask = [REQUIREMENT_MET; NODES_PER_GROUP];
    if skill >= SKILLS {
        return mask;
    }
    let block = SkillBlock::of(skills);
    for index in 0..WRITTEN_TECHNIQUES {
        let technique = block.technique(skill, index);
        // An id past the eighth would write past the mask in the original.
        if let Some(slot) = usize::from(technique)
            .checked_sub(1)
            .and_then(|at| mask.get_mut(at))
        {
            *slot = REQUIREMENT_CHOSEN;
        }
    }
    for (slot, present) in mask.iter_mut().zip(NODE_PRESENT[skill]) {
        if !present {
            *slot = REQUIREMENT_ABSENT;
        }
    }
    let funded = usize::from(block.points(skill) / POINTS_PER_TECHNIQUE);
    for which in 0..2 {
        for (node, slot) in mask.iter_mut().enumerate() {
            let need = technique_prerequisites(skill, node as u8 + 1)[which];
            if need == 0 || *slot != REQUIREMENT_MET {
                continue;
            }
            if !(0..funded).any(|at| block.technique(skill, at) == need) {
                *slot = REQUIREMENT_MISSING;
            }
        }
    }
    mask
}

/// What a press on a ring node does: [`NODE_LOCKED`], [`NODE_LEARNT`] (the
/// forget dialog opens) or [`NODE_OPEN`] (the learning row opens).
///
/// `code` is the screen's node code, the group in bits 8–15 (1-based) and
/// the level in bits 0–2 (1-based); either 0 is locked. A level the points
/// fund is learnt. The next level is open while a level is `ready` and its
/// technique byte is empty; with a technique already chosen there (ahead of
/// the points) it is learnt once the skill sits more than ten points past
/// its funded levels, else still open. Any later level is locked. The
/// technique byte is read through the record's layout (a seventh level
/// reads the next skill's points).
pub fn skill_node_state(skills: &[Skill; SKILLS], code: i32, ready: bool) -> u8 {
    let group = usize::try_from((code >> 8) & 0xff).unwrap_or(0);
    let level = usize::try_from(code & 7).unwrap_or(0);
    // A group past the six reads outside the record in the original.
    if group == 0 || level == 0 || group > SKILLS {
        return NODE_LOCKED;
    }
    let block = SkillBlock::of(skills);
    let skill = group - 1;
    let points = block.points(skill);
    let funded = usize::from(points / POINTS_PER_TECHNIQUE);
    if funded >= level {
        return NODE_LEARNT;
    }
    if funded + 1 != level {
        return NODE_LOCKED;
    }
    if block.technique(skill, level - 1) == 0 {
        return if ready { NODE_OPEN } else { NODE_LOCKED };
    }
    let past = i64::from(points) - funded as i64 * i64::from(POINTS_PER_TECHNIQUE);
    if past > 10 { NODE_LEARNT } else { NODE_OPEN }
}

/// The deepest unlearning chain this model follows; the shipped table's
/// chains are three long, and the original has no bound.
const MAX_UNLEARN_DEPTH: usize = 64;

/// Unlearns a level of a group, as the skill screen does: its own variant of
/// the record's rule, which also keeps the screen's committed points
/// (`base`, the points the group had before this visit's experience).
///
/// `group` and `level` are 1-based (a group of 0 does nothing). The points
/// change first, by where the level sits against the committed and the
/// record's levels:
/// - at or below the committed levels: both lose 5000;
/// - one past them: the committed points round down to their levels, and
///   the record's points lose 5000 while it holds more levels, else take
///   the committed points;
/// - further out: the record's points lose 5000 when it funds the level,
///   else round down to their levels.
///
/// Then the level's technique leaves (the later written slots move down,
/// the fifth clears) and, from the last funded level down, every technique
/// that lists the removed one as a prerequisite is unlearned in turn. All
/// arithmetic is 16-bit and wraps. The prerequisite table is the character
/// record's ([`technique_prerequisites`]).
pub fn unlearn_skill_level(
    skills: &mut [Skill; SKILLS],
    base: &mut [u16; SKILLS],
    group: u16,
    level: u16,
) {
    let mut block = SkillBlock::of(skills);
    unlearn_in_block(&mut block, base, group, level, 0);
    block.store(skills);
}

fn unlearn_in_block(
    block: &mut SkillBlock,
    base: &mut [u16; SKILLS],
    group: u16,
    level: u16,
    depth: usize,
) {
    if group == 0 || depth > MAX_UNLEARN_DEPTH {
        return;
    }
    let skill = usize::from(group - 1);
    // A group past the six writes outside the record in the original.
    if skill >= SKILLS {
        return;
    }
    let step = POINTS_PER_TECHNIQUE;
    let committed = base[skill] / step;
    let points = block.points(skill);
    let funded = points / step;
    if level > committed {
        if u32::from(committed) + 1 < u32::from(level) {
            if level > funded {
                block.set_points(skill, funded.wrapping_mul(step));
            } else {
                block.set_points(skill, points.wrapping_sub(step));
            }
        } else {
            base[skill] = committed.wrapping_mul(step);
            if committed < funded {
                block.set_points(skill, points.wrapping_sub(step));
            } else {
                block.set_points(skill, base[skill]);
            }
        }
    } else {
        base[skill] = base[skill].wrapping_sub(step);
        block.set_points(skill, points.wrapping_sub(step));
    }
    let entry = skill * 8;
    let removed = block.byte(entry + 1 + usize::from(level));
    // The shift runs from the level's slot to the fifth; a level of 0 starts
    // one byte early, on the points' high byte, as the original's does.
    let first = i32::from(level) - 1;
    if first <= 4 {
        for at in first..=4 {
            let to = (entry as i32 + 2 + at) as usize;
            block.set_byte(to, block.byte(to + 1));
        }
    }
    block.set_byte(entry + 6, 0);
    for at in (0..usize::from(funded)).rev() {
        let technique = block.technique(skill, at);
        let needs = technique_prerequisites(skill, technique);
        if needs.contains(&removed) {
            unlearn_in_block(block, base, group, at as u16 + 1, depth + 1);
        }
    }
}

/// The skill screen's experience pass: spends the screen's global experience
/// on the record's skill points, as the shown levels allow.
///
/// `base` holds the committed points; the record's points are rewritten
/// from them. `budget` is the experience's worth in points (the experience
/// system's conversion of the screen's global experience). The weight
/// starts at four plus the committed levels. The last group with a partial
/// committed level is topped up first: that costs the missing points times
/// the weight, quartered toward zero; a smaller budget adds
/// `budget × 4 / weight` (in double precision, truncated) instead and is
/// spent. Then every level between a group's committed (or topped-up) level
/// and its record level costs `weight × 5000 / 4`, the weight rising after
/// each. Whatever is left, even below zero, goes to the record's surplus
/// choice (the first skill whose next written technique is ahead of its
/// points): a full level when it covers `weight × 5000 / 4`, else the share
/// on top of the level below. Returns what is left, which the screen turns
/// back into global experience. The budget is 32-bit and its comparisons
/// with the costs read it unsigned, as the original's do.
pub fn spend_skill_experience(data: &mut CharacterData, base: &[u16; SKILLS], budget: u32) -> u32 {
    let step = i64::from(POINTS_PER_TECHNIQUE);
    let mut weight: i32 = 4;
    let mut partial: Option<(usize, i32)> = None;
    for (group, &committed) in base.iter().enumerate() {
        let levels = i32::from(committed / POINTS_PER_TECHNIQUE);
        weight = weight.wrapping_add(levels);
        if i64::from(levels) * step < i64::from(committed) {
            partial = Some((group, (levels + 1) * 5000 - i32::from(committed)));
        }
    }
    let share = |budget: u32, weight: i32| -> i64 {
        ((f64::from(budget) * 4.0) / f64::from(weight)) as i64
    };
    let mut left = budget;
    if let Some((group, need)) = partial {
        let cost = quarter(i64::from(need) * i64::from(weight)) as i32;
        let skill = &mut data.record.skills[group];
        if i64::from(cost) > i64::from(left) {
            skill.points = (share(left, weight) + i64::from(base[group])) as u16;
            left = 0;
        } else {
            skill.points = (i32::from(base[group]) + need) as u16;
            left = left.wrapping_sub(cost as u32);
            weight = weight.wrapping_add(1);
        }
    }
    for (group, &committed) in base.iter().enumerate() {
        let mut from = i32::from(committed / POINTS_PER_TECHNIQUE);
        if partial.is_some_and(|(at, _)| at == group) {
            from += 1;
        }
        let last = i32::from(data.record.skills[group].points / POINTS_PER_TECHNIQUE) - 1;
        for _ in from..=last {
            left = left.wrapping_sub(quarter(i64::from(weight) * step) as i32 as u32);
            weight = weight.wrapping_add(1);
        }
    }
    if left != 0
        && let Some((skill, level)) = data.surplus_chosen_level()
    {
        let cost = i64::from(quarter(i64::from(weight) * step) as i32);
        let level = i64::from(level);
        let entry = &mut data.record.skills[skill - 1];
        if cost > i64::from(left) {
            entry.points = (share(left, weight) + (level - 1) * step) as u16;
            left = 0;
        } else {
            entry.points = (level * step) as u16;
            left = left.wrapping_sub(cost as i32 as u32);
        }
    }
    left
}

/// `value / 4`, rounded toward zero as the original's shift-with-bias.
fn quarter(value: i64) -> i64 {
    value / 4
}

/// The skill screen's model: the record it edits, its ring, its pending
/// learn or forget choice and the fades.
#[derive(Debug, Clone, PartialEq)]
pub struct SkillScreen {
    pub bounds: SkillBounds,
    pub unit: i32,
    /// The groups on the ring: six, or four on a page.
    pub count: i32,
    /// The ring radius in units: 64, 58 with the global inset-layout flag,
    /// 72 on a page.
    pub radius: i32,
    /// The ring's horizontal stretch (1.0 as built).
    pub stretch: f32,
    /// The page the screen opened on; 0 is the ring.
    pub page: i32,
    /// The global inset-layout flag (it also moves a page's ring a unit).
    pub inset: bool,
    /// Bit 1 is the reveal, bit 2 the hide; 4 is hidden.
    pub state: u8,
    /// The reveal's end (500 ms after the screen is built).
    pub reveal_deadline: i32,
    /// The hide's end.
    pub hide_deadline: i32,
    /// The learning row's and forget dialog's fade: bit 1 opening, bit 2
    /// closing.
    pub popup: u8,
    pub popup_deadline: i32,
    /// The node code (or row slot, or arrow) under the pointer.
    pub hover: i32,
    /// The node whose learning row is open; 0 for none.
    pub learning: i32,
    /// The node awaiting the forget confirmation; 0 for none.
    pub forgetting: i32,
    /// The learning row's requirement mask.
    pub requirements: [u8; NODES_PER_GROUP],
    /// The committed points of each group, before this visit's experience.
    pub base: [u16; SKILLS],
    /// The global experience the screen may spend.
    pub experience: u32,
    /// The global experience left after the last pass.
    pub leftover: i32,
    /// Whether a level is ready to spend.
    pub ready: bool,
    /// A locked screen never has a level ready.
    pub locked: bool,
}

/// Converts experience to its worth in skill points and back (the
/// experience system's two conversions, which live with the game).
#[derive(Clone, Copy)]
pub struct ExperienceRates {
    pub worth: fn(u32) -> u32,
    pub experience: fn(i32) -> i32,
}

impl SkillScreen {
    /// A screen as the original builds it: six groups at a 64-unit radius
    /// (58 with the inset-layout flag), stretch 1, the reveal running for
    /// 500 ms. A positive `page` (the skill window's constructor) opens a
    /// four-group page instead, at a 72-unit radius stretched 1.25 across,
    /// or 64 units stretched 0.9 with the inset-layout flag.
    pub fn open(bounds: SkillBounds, unit: i32, inset: bool, page: i32, clock: i32) -> Self {
        let (count, radius, stretch) = match (page > 0, inset) {
            (true, true) => (4, 64, 0.9),
            (true, false) => (4, 72, 1.25),
            (false, true) => (6, 58, 1.0),
            (false, false) => (6, 64, 1.0),
        };
        Self {
            bounds,
            unit,
            count,
            radius,
            stretch,
            page: page.max(0),
            inset,
            state: 1,
            reveal_deadline: clock.wrapping_add(500),
            hide_deadline: 0,
            popup: 0,
            popup_deadline: 0,
            hover: 0,
            learning: 0,
            forgetting: 0,
            requirements: [REQUIREMENT_MET; NODES_PER_GROUP],
            base: [0; SKILLS],
            experience: 0,
            leftover: 0,
            ready: false,
            locked: false,
        }
    }

    /// The group (0-based) a 1-based ring slot shows.
    pub fn slot_group(&self, slot: i32) -> Option<usize> {
        let at = usize::try_from(slot).ok()?.checked_sub(1)?;
        if self.page > 0 {
            PAGE_GROUPS.get(at).copied()
        } else {
            RING_GROUPS.get(at).copied()
        }
    }

    /// The 1-based ring slot of a 0-based group.
    pub fn group_slot(&self, group: usize) -> Option<i32> {
        let slot = if self.page > 0 {
            PAGE_SLOTS.get(group)
        } else {
            GROUP_SLOTS.get(group)
        };
        slot.map(|&at| at as i32 + 1)
    }

    /// The ring's layout at a reveal scale.
    pub fn ring(&self, turn: f32) -> SkillRing {
        SkillRing {
            count: self.count,
            radius: self.radius,
            stretch: self.stretch,
            turn,
            width: self.bounds.right - self.bounds.left,
            height: self.bounds.bottom - self.bounds.top,
            page: self.page,
            unit: self.unit,
            scrolled: self.inset,
        }
    }

    /// The draw's two fades: the content alpha (it also spreads and turns
    /// the ring) and the opening scale, after the reveal and hide clocks.
    /// The hide scales the opening scale by its remaining part of 256 ms and,
    /// once over, hides the screen (closing it on a page other than two).
    /// Updates the state bits as the draw does.
    pub fn fade(&mut self, clock: i32) -> (f32, f32) {
        let fades = skill_screen_fades(
            self.state,
            self.reveal_deadline,
            self.hide_deadline,
            clock,
            self.page,
        );
        self.state = fades.state;
        if fades.hidden {
            return (0.0, 0.0);
        }
        (fades.alpha, fades.scale)
    }

    /// Starts the hide: bit 2 and a 256 ms deadline.
    pub fn begin_hide(&mut self, clock: i32) {
        self.state |= 2;
        self.hide_deadline = clock.wrapping_add(256);
    }

    /// A press on a ring node while nothing is pending: a learnt node asks
    /// to be forgotten, an open one opens its learning row (with the
    /// group's requirement mask), and either starts the popup's 300 ms
    /// opening. Returns the node's state.
    pub fn press_node(&mut self, skills: &[Skill; SKILLS], code: i32, clock: i32) -> u8 {
        let state = skill_node_state(skills, code, self.ready);
        match state {
            NODE_LEARNT => self.forgetting = code,
            NODE_OPEN => {
                self.learning = code;
                let group = usize::try_from((code >> 8) & 7).unwrap_or(0);
                self.requirements = group
                    .checked_sub(1)
                    .map(|skill| skill_requirements(skill, skills))
                    .unwrap_or([REQUIREMENT_MET; NODES_PER_GROUP]);
            }
            _ => return state,
        }
        self.popup = 1;
        self.popup_deadline = clock.wrapping_add(300);
        state
    }

    /// The second press while a choice is pending. With the forget dialog
    /// up, `hover` 1 (Yes) unlearns the node's level. With the learning row
    /// open, a slot (`hover`, 1–8) whose requirement byte is clear first
    /// unlearns the record's surplus choice, then writes that technique into
    /// the pending level's slot. Either refreshes the ready flag; then the
    /// popup starts its 200 ms close unless it is already fading. Returns
    /// whether the record changed.
    pub fn commit(
        &mut self,
        data: &mut CharacterData,
        hover: i32,
        clock: i32,
        rates: ExperienceRates,
    ) -> bool {
        let mut changed = false;
        if self.forgetting != 0 {
            if hover == 1 {
                let group = ((self.forgetting >> 8) & 7) as u16;
                let level = (self.forgetting & 7) as u16;
                unlearn_skill_level(&mut data.record.skills, &mut self.base, group, level);
                self.refresh(data, rates);
                changed = true;
            }
        } else if let Some(&clear) = usize::try_from(hover)
            .ok()
            .and_then(|at| at.checked_sub(1))
            .and_then(|at| self.requirements.get(at))
            && clear == REQUIREMENT_MET
        {
            if let Some((skill, level)) = data.surplus_chosen_level() {
                unlearn_skill_level(
                    &mut data.record.skills,
                    &mut self.base,
                    skill as u16,
                    level as u16,
                );
            }
            let group = usize::try_from((self.learning >> 8) & 7).unwrap_or(0);
            let level = usize::try_from(self.learning & 7).unwrap_or(0);
            if let Some(skill) = group.checked_sub(1) {
                let mut block = SkillBlock::of(&data.record.skills);
                block.set_byte(skill * 8 + 1 + level, hover as u8);
                block.store(&mut data.record.skills);
            }
            self.refresh(data, rates);
            changed = true;
        }
        if self.popup == 0 {
            self.popup = 2;
            self.popup_deadline = clock.wrapping_add(200);
        }
        changed
    }

    /// Attaches a character record (the compact screen's constructor, the
    /// character screen and the arena do): each group's committed points
    /// become the record's points of its skill, or 0 with `clear`, and the
    /// ready flag is refreshed.
    pub fn attach(&mut self, data: &mut CharacterData, clear: bool, rates: ExperienceRates) {
        self.base = if clear {
            [0; SKILLS]
        } else {
            std::array::from_fn(|skill| data.record.skills[skill].points)
        };
        self.refresh(data, rates);
    }

    /// Refreshes the ready flag. A locked screen is never ready. Otherwise
    /// the screen spends its experience first (when it has any); while
    /// experience is left over a level is ready. With none left, the scan
    /// stops at the first skill whose next technique byte is set (read
    /// through the record's layout): that skill must sit no more than ten
    /// points past its funded levels, and the levels counted up to there
    /// must stay below the total cap.
    pub fn refresh(&mut self, data: &mut CharacterData, rates: ExperienceRates) {
        if self.locked {
            self.ready = false;
            return;
        }
        self.ready = true;
        if self.experience != 0 {
            let left = spend_skill_experience(data, &self.base, (rates.worth)(self.experience));
            self.leftover = (rates.experience)(left as i32);
        }
        if self.leftover != 0 {
            return;
        }
        let block = SkillBlock::of(&data.record.skills);
        let mut total = 0u32;
        for skill in 0..SKILLS {
            let points = block.points(skill);
            let funded = points / POINTS_PER_TECHNIQUE;
            total += u32::from(funded);
            if block.technique(skill, usize::from(funded)) != 0 {
                if i64::from(funded) * 5000 + 10 < i64::from(points) {
                    self.ready = false;
                }
                break;
            }
        }
        if total >= MAX_TOTAL_LEVELS {
            self.ready = false;
        }
    }
}

/// A screen-space icon quad from the skill icon atlas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkillIconQuad {
    /// The cell's texture rectangle `[u, v, u + du, v + dv]`; the quad's
    /// top edge samples `v + dv` and its bottom edge `v` (texture rows run
    /// up, as GL's).
    pub uv: [f32; 4],
    /// The quad's top-left corner and its side (the atlas cell size).
    pub x: f32,
    pub y: f32,
    pub size: f32,
}

/// The quad a 1-based icon draws: the atlas cell `icon − 1`, counted along
/// rows of `width / cell` cells (integer division), at `(x, y)` with the
/// cell's size. An icon below 1 draws nothing. The cell's texture size is
/// `cell / width` by `cell / height`, divided in double precision.
pub fn skill_icon_quad(
    icon: i32,
    cell: i32,
    width: u16,
    height: u16,
    x: f32,
    y: f32,
) -> Option<SkillIconQuad> {
    let index = icon - 1;
    if index < 0 || cell <= 0 {
        return None;
    }
    let du = (f64::from(cell) / f64::from(width)) as f32;
    let dv = (f64::from(cell) / f64::from(height)) as f32;
    let per_row = (i64::from(width) / i64::from(cell)) as f32;
    let index = index as f32;
    let row = (index / per_row) as i64 as f32;
    let u = (index - row * per_row) * du;
    let v = row * dv;
    Some(SkillIconQuad {
        uv: [u, v, u + du, v + dv],
        x,
        y,
        size: cell as f32,
    })
}

/// One icon of a group on the ring.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkillGroupIcon {
    /// The 1-based level.
    pub level: i32,
    pub x: f32,
    pub y: f32,
    /// The technique id; 0 draws the screen's "choose" icon.
    pub technique: u8,
    /// The icon under the pointer draws in the highlight colour.
    pub hovered: bool,
    /// The chosen technique ahead of the points draws at half alpha.
    pub half: bool,
}

/// The skill block a group's icons and bars read, and the group's index in
/// it: the record's, or the preview skill (followed by zero bytes, as in the
/// original) when the screen has no record.
fn group_block(skills: Option<&[Skill; SKILLS]>, group: usize) -> (SkillBlock, usize) {
    match skills {
        Some(skills) => (SkillBlock::of(skills), group),
        None => {
            let mut preview = [Skill::default(); SKILLS];
            preview[0] = PREVIEW_SKILL;
            (SkillBlock::of(&preview), 0)
        }
    }
}

/// The icons of one group around its ring position `(x, y)`.
///
/// A group shows one icon per funded level, plus the next level when a
/// technique is chosen there ahead of the points, or when a level is
/// `ready` and fewer than five are funded. The icons stand on the pentagon
/// ([`pentagon_offset`]) scaled by the reveal `turn`, about a point four
/// units left of and above `(x, y)`. The first empty technique byte draws
/// the "choose" icon and ends the row. `group` is 0-based; `hover` is the
/// screen's hovered node code; `skills` is the record's, or `None` for the
/// preview.
#[allow(clippy::too_many_arguments)]
pub fn skill_group_icons(
    skills: Option<&[Skill; SKILLS]>,
    group: usize,
    x: f32,
    y: f32,
    unit: i32,
    turn: f32,
    ready: bool,
    hover: i32,
) -> Vec<SkillGroupIcon> {
    let (block, at_group) = group_block(skills, group);
    let funded = usize::from(block.points(at_group) / POINTS_PER_TECHNIQUE);
    let mut count = funded;
    let ahead = block.technique(at_group, funded) != 0;
    if ahead || (ready && funded < 5) {
        count += 1;
    }
    let left = x - (4 * unit) as f32;
    let top = y - (4 * unit) as f32;
    let hovered_group = usize::try_from((hover >> 8) & 0xff).ok() == Some(group + 1);
    let mut icons = Vec::new();
    for at in 0..count {
        let (sx, sy) = pentagon_offset(at as i32, unit);
        let technique = block.technique(at_group, at);
        icons.push(SkillGroupIcon {
            level: at as i32 + 1,
            x: sx * turn + left,
            y: top - sy * turn,
            technique,
            hovered: hovered_group && hover & 0xff == at as i32 + 1,
            half: technique != 0 && ahead && at + 1 == count,
        });
        if technique == 0 {
            break;
        }
    }
    icons
}

/// A radial bar about a point: corners in drawing order.
pub type SkillBar = [[f32; 2]; 4];

/// The four corners of a bar at pentagon slot `slot`, `half` either side of
/// the spoke, from `inner` to `outer` units out (both given negative, as
/// the original's), its sine and cosine scaled by `turn`. The sums keep the
/// original's order.
fn pentagon_bar(
    slot: i32,
    x: f32,
    y: f32,
    half: f32,
    inner: f32,
    outer: f32,
    turn: f32,
) -> SkillBar {
    let angle = slot as f32 * std::f32::consts::TAU * 0.2;
    let sin = angle.sin() * turn;
    let cos = angle.cos() * turn;
    [
        [
            (x - half * cos) - inner * sin,
            inner * cos + (y - half * sin),
        ],
        [
            (x - half * cos) - outer * sin,
            outer * cos + (y - half * sin),
        ],
        [
            (half * cos + x) - outer * sin,
            (outer * cos + half * sin) + y,
        ],
        [
            (half * cos + x) - inner * sin,
            (inner * cos + half * sin) + y,
        ],
    ]
}

/// One orbit bar of a group: its corners and how full its shader draws it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkillOrbitBar {
    pub corners: SkillBar,
    pub fill: f32,
}

/// The orbit bars of a group around its ring position: one per funded level
/// plus one for a technique chosen ahead of the points, which fills by the
/// points past the funded levels (`× 0.0002`, in double precision). Each
/// bar spans 20.3 to 26.3 units out and 14 units either side, on the icons'
/// spokes (up first). A bar fills completely unless it is the last and
/// below 0.99. `group` is 0-based; `skills` is `None` for the preview.
pub fn skill_orbit_bars(
    skills: Option<&[Skill; SKILLS]>,
    group: usize,
    x: f32,
    y: f32,
    unit: i32,
    turn: f32,
) -> Vec<SkillOrbitBar> {
    let (block, at_group) = group_block(skills, group);
    let points = block.points(at_group);
    let funded = usize::from(points / POINTS_PER_TECHNIQUE);
    let mut count = funded;
    let mut progress = 1.0f32;
    if block.technique(at_group, funded) != 0 {
        let past = i64::from(points) - funded as i64 * 5000;
        progress = (past as f64 * 0.0002) as f32;
        count += 1;
    }
    let half = (14 * unit) as f32;
    let inner = (-f64::from(unit) * 20.3) as f32;
    let outer = inner - (6 * unit) as f32;
    (0..count)
        .map(|at| SkillOrbitBar {
            corners: pentagon_bar(at as i32, x, y, half, inner, outer, turn),
            fill: if at + 1 == count && f64::from(progress) < 0.99 {
                progress
            } else {
                1.0
            },
        })
        .collect()
}

/// The five background spokes the draw puts behind every ring node: 12.3 to
/// 26.3 units out, 18 units either side, scaled by the reveal.
pub fn skill_node_spokes(x: f32, y: f32, unit: i32, turn: f32) -> [SkillBar; 5] {
    let half = (18 * unit) as f32;
    let inner = (-f64::from(unit) * 12.3) as f32;
    let outer = inner - (14 * unit) as f32;
    std::array::from_fn(|slot| pentagon_bar(slot as i32, x, y, half, inner, outer, turn))
}

/// The hover tooltip's opacity: it slides in over 200 ms after the hovered
/// node changed and out again once none is hovered (`hovered` below 1),
/// then goes. `since` is the time since the change stamp.
pub fn selection_tip_slide(hovered: i32, since: i32) -> Option<f32> {
    if since < 200 {
        let t = since as f32 * 0.005;
        Some(if hovered < 1 { 1.0 - t } else { t })
    } else if hovered < 1 {
        None
    } else {
        Some(1.0)
    }
}

/// The learning row's or forget dialog's fade.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkillPopupFade {
    pub flags: u8,
    pub alpha: f32,
    /// The width scale of the box.
    pub scale: f32,
    /// The closing fade ended: the pending choice is cleared and nothing
    /// draws.
    pub closed: bool,
}

/// The popup fade both the learning row and the forget dialog run. Opening
/// (bit 1) eases in over 300 ms: `t = 1 − remaining / 300`, the alpha
/// `t(2 − t)` and the width scale that eased again; once over, bit 1
/// clears. Closing (bit 2) fades the alpha as `(remaining × 0.005)²` at
/// full width; once over, the flags clear and the popup closes.
pub fn skill_popup_fade(flags: u8, deadline: i32, clock: i32) -> SkillPopupFade {
    let remaining = f64::from(deadline.wrapping_sub(clock));
    if flags & 1 == 0 {
        if flags & 2 == 0 {
            return SkillPopupFade {
                flags,
                alpha: 1.0,
                scale: 1.0,
                closed: false,
            };
        }
        if deadline <= clock {
            return SkillPopupFade {
                flags: 0,
                alpha: 0.0,
                scale: 0.0,
                closed: true,
            };
        }
        let t = (remaining * 0.005) as f32;
        return SkillPopupFade {
            flags,
            alpha: t * t,
            scale: 1.0,
            closed: false,
        };
    }
    if clock < deadline {
        let t = (1.0 - remaining * 0.003_333_333_333_333_333_5) as f32;
        let alpha = t * (2.0 - t);
        return SkillPopupFade {
            flags,
            alpha,
            scale: alpha * (2.0 - alpha),
            closed: false,
        };
    }
    SkillPopupFade {
        flags: flags & !1,
        alpha: 1.0,
        scale: 1.0,
        closed: false,
    }
}

/// Moves a popup left by however far its right edge (`x + half_width`,
/// with the form's and its parent's origins added) passes the window's
/// width. `offsets` are the form's two origin terms then its parent's, in
/// the original's order of addition.
pub fn clamp_tooltip(x: f32, offsets: &[i32], half_width: f32, window: i32) -> f32 {
    let mut left = x;
    for offset in offsets {
        left += *offset as f32;
    }
    let overflow = (left + half_width) - window as f32;
    if overflow > 0.0 { x - overflow } else { x }
}

/// A popup box: its rectangle and the glow's alpha.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkillTipBox {
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
    /// The box's alpha (0.6 of the text's).
    pub alpha: f32,
}

/// The hover tooltip of a ring group: the group's name in a box sized to
/// it. `(x, y)` is the group's ring position; the text width is halved by
/// an integer shift and padded by a unit. The box spans the four units
/// under the node and the name sits at its bottom, centred. No clamp.
/// Returns the box and the name's origin.
pub fn ring_group_tooltip(
    x: f32,
    y: f32,
    text_width: u32,
    unit: i32,
    alpha: f32,
) -> (SkillTipBox, (f32, f32)) {
    let baseline = (4 * unit) as f32 + y;
    let half = (text_width >> 1) as f32 + unit as f32;
    let tip = SkillTipBox {
        left: x - half,
        top: baseline - (4 * unit) as f32,
        width: half * 2.0,
        height: (4 * unit) as f32,
        alpha: alpha * 0.6,
    };
    (tip, (x - (half - unit as f32), baseline))
}

/// The hover tooltip of a ring node: `82 × 14` units centred 13 units under
/// the node, clamped into the window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkillNodeTip {
    pub tip: SkillTipBox,
    /// The clamped centre.
    pub x: f32,
    /// The title's baseline (3 units above the box's centre).
    pub title_y: f32,
    /// The description block's corner, 76 by 12 units.
    pub text: (f32, f32),
}

/// Places a ring node's hover tooltip. The title is the technique chosen
/// at that level ([`ring_node_title`]), centred on `x`; with none chosen
/// the group's name is drawn from the centre instead.
pub fn ring_node_tooltip(
    x: f32,
    y: f32,
    unit: i32,
    window: i32,
    offsets: &[i32],
    alpha: f32,
) -> SkillNodeTip {
    let centre_y = (13 * unit) as f32 + y;
    let half = (41 * unit) as f32 * 1.0;
    let x = clamp_tooltip(x, offsets, half, window);
    let title_y = centre_y - (3 * unit) as f32;
    SkillNodeTip {
        tip: SkillTipBox {
            left: x - half,
            top: centre_y - (7 * unit) as f32,
            width: half * 2.0,
            height: (7 * unit) as f32 * 2.0,
            alpha: alpha * 0.6,
        },
        x,
        title_y,
        text: (x - (38 * unit) as f32, title_y + (2 * unit) as f32),
    }
}

/// The ring node tooltip's title: the technique the record holds at the
/// node's level, or the group's name (and `false`) when that slot is empty.
/// The level's byte is read through the record's layout.
pub fn ring_node_title(skills: &[Skill; SKILLS], code: i32) -> Option<(&'static str, bool)> {
    let group = usize::try_from((code >> 8) & 0xff).ok()?.checked_sub(1)?;
    let level = usize::try_from(code & 0xff).ok()?.checked_sub(1)?;
    let name = *SKILL_NAMES.get(group)?;
    let technique = SkillBlock::of(skills).technique(group, level);
    if technique == 0 {
        return Some((name, false));
    }
    let title = TECHNIQUE_NAMES
        .get(group)?
        .get(usize::from(technique) - 1)
        .copied()
        .unwrap_or("");
    Some((title, true))
}

/// One icon of the learning row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkillRowIcon {
    /// The 1-based slot (the technique id it offers).
    pub slot: i32,
    pub x: f32,
    pub y: f32,
    /// The slot under the pointer, with a clear requirement byte, draws in
    /// the highlight colour.
    pub hovered: bool,
    /// The icon's alpha factor: a slot with a requirement byte set draws at
    /// half.
    pub alpha: f32,
}

/// The learning row: its box, its eight icons and the hovered slot's text.
#[derive(Debug, Clone, PartialEq)]
pub struct SkillLearningRow {
    pub tip: SkillTipBox,
    pub x: f32,
    pub icons: [SkillRowIcon; NODES_PER_GROUP],
    /// The hovered slot's title baseline and status or description corner.
    pub title_y: f32,
    pub text: (f32, f32),
    /// The status line for the hovered slot, `None` for its description.
    pub status: Option<&'static str>,
}

/// Places the learning row of the node at `(x, y)`: an `82 × 28`-unit box
/// whose width eases with the popup's `scale`, centred four units under the
/// node and clamped into the window; eight icons ten units apart (also
/// scaled), centred four units left of the box's centre, two units under
/// its top. `hover` is the slot under the pointer (1–8, else none);
/// `alpha` the popup's alpha times the screen's.
#[allow(clippy::too_many_arguments)]
pub fn learning_row(
    x: f32,
    y: f32,
    unit: i32,
    scale: f32,
    window: i32,
    offsets: &[i32],
    alpha: f32,
    requirements: &[u8; NODES_PER_GROUP],
    hover: i32,
) -> SkillLearningRow {
    let centre_y = (4 * unit) as f32 + y;
    let half = (41 * unit) as f32 * scale;
    let half_height = (14 * unit) as f32;
    let x = clamp_tooltip(x, offsets, half, window);
    let top = centre_y - half_height;
    let row_y = (2 * unit) as f32 + top;
    let icons = std::array::from_fn(|slot| {
        let left = (((slot as f32 - 3.5) * (10 * unit) as f32) * scale + x) - (4 * unit) as f32;
        let clear = requirements[slot] == REQUIREMENT_MET;
        SkillRowIcon {
            slot: slot as i32 + 1,
            x: left,
            y: row_y,
            hovered: clear && hover - 1 == slot as i32,
            alpha: if clear { 1.0 } else { 0.5 },
        }
    });
    let title_y = (14 * unit) as f32 + row_y;
    let status = match usize::try_from(hover)
        .ok()
        .filter(|&slot| (1..=NODES_PER_GROUP).contains(&slot))
        .map(|slot| requirements[slot - 1])
    {
        Some(REQUIREMENT_CHOSEN) => Some(STATUS_CHOSEN),
        Some(REQUIREMENT_MISSING) => Some(STATUS_REQUIREMENTS),
        Some(REQUIREMENT_ABSENT) => Some(STATUS_UNAVAILABLE),
        _ => None,
    };
    SkillLearningRow {
        tip: SkillTipBox {
            left: x - half,
            top,
            width: half * 2.0,
            height: half_height * 2.0,
            alpha: alpha * 0.6,
        },
        x,
        icons,
        title_y,
        text: (x - (38 * unit) as f32, (2 * unit) as f32 + title_y),
        status,
    }
}

/// The forget dialog's geometry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ForgetDialogPlace {
    pub tip: SkillTipBox,
    /// The clamped centre.
    pub x: f32,
    /// The text lines' left edge and width (82 units, scaled).
    pub text_left: f32,
    pub text_width: f32,
    pub title_y: f32,
    /// The Yes and No captions, drawn only once the alpha passes 0.5.
    pub yes_x: f32,
    pub no_x: f32,
    pub choice_y: f32,
    pub choices: bool,
    /// The captions' alpha factor, `(alpha − 0.5) × 2`.
    pub choice_alpha: f32,
    pub question_y: f32,
}

/// Places the forget dialog under the node at `(x, y)`: a glow box 82 units
/// wide (eased by the popup's `scale`) and 20 tall, centred two units under
/// the node and clamped into the window; the title five units into it, Yes
/// and No one 13-unit line lower at 12 units left and 6 units right of the
/// centre, and the question seven units above the choices.
pub fn forget_dialog_place(
    x: f32,
    y: f32,
    unit: i32,
    scale: f32,
    window: i32,
    offsets: &[i32],
    alpha: f32,
) -> ForgetDialogPlace {
    let centre_y = (2 * unit) as f32 + y;
    let half = (41 * unit) as f32 * scale;
    let half_height = (10 * unit) as f32;
    let x = clamp_tooltip(x, offsets, half, window);
    let title_y = (5 * unit) as f32 + (centre_y - half_height);
    let text_width = (82 * unit) as f32 * scale;
    let choice_y = (13 * unit) as f32 + title_y;
    ForgetDialogPlace {
        tip: SkillTipBox {
            left: x - half,
            top: centre_y - half_height,
            width: half * 2.0,
            height: half_height * 2.0,
            alpha: alpha * 0.6,
        },
        x,
        text_left: x - text_width * 0.5,
        text_width,
        title_y,
        yes_x: x - (12 * unit) as f32,
        no_x: (6 * unit) as f32 + x,
        choice_y,
        choices: 0.5 < alpha,
        choice_alpha: (alpha - 0.5) * 2.0,
        question_y: choice_y - (7 * unit) as f32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_skill_node_is_placed_centred_or_forwarded() {
        let bounds = SkillBounds {
            left: 10,
            right: 110,
            top: 20,
            bottom: 80,
        };
        let placed = skill_position(0x0205, bounds, false, (0, 0), |row, col| {
            assert_eq!((row, col), (2, 5));
            (12.5, 34.5)
        });
        assert_eq!(placed, (12, 34));
        assert_eq!(
            skill_position(0, bounds, false, (0, 0), |_, _| unreachable!()),
            (50, 30)
        );
        assert_eq!(
            skill_position(-1, bounds, true, (7, 9), |_, _| unreachable!()),
            (7, 9)
        );
    }

    #[test]
    fn the_skill_screen_shows_and_leaves_on_its_guards() {
        let shown = show_skill_ui(true, 0, 100, 250);
        assert_eq!(
            shown,
            SkillShow {
                relayout: true,
                state: 2,
                deadline: 350,
            }
        );

        assert_eq!(
            leave_skill_ui(true, 0, 0, 0, 0),
            SkillLeave {
                call_base: true,
                call_clear: false,
            }
        );
        assert_eq!(leave_skill_ui(false, 2, 0, 0, 0), SkillLeave::default());
        assert!(leave_skill_ui(false, 0, 1, 900, 500).call_clear);
        assert_eq!(
            leave_skill_ui(false, 0, 1, 1000, 500),
            SkillLeave::default()
        );
        assert_eq!(leave_skill_ui(false, 0, 2, 0, 100), SkillLeave::default());
        assert!(leave_skill_ui(false, 0, 0, 0, 0).call_clear);
    }

    #[test]
    fn the_screen_opens_or_closes_and_restores_controls() {
        let opened = open_skill_screen(false, 2, true, true);
        assert!(opened.build_resources && opened.play_sound && opened.restore_controls);
        assert_eq!(
            (opened.page, opened.groups, opened.radius, opened.state),
            (2, 4, 72, 4)
        );
        assert!(opened.show && opened.activate && opened.steam_bind);
        let ring = open_skill_screen(false, 0, false, false);
        assert_eq!((ring.groups, ring.radius, ring.state), (6, 0, 1));
        assert!(!ring.play_sound && !ring.restore_controls);
        let toggle = open_skill_screen(true, 2, true, true);
        assert!(toggle.close_existing);
        assert!(!toggle.build_resources && !toggle.show);

        let nil = close_skill_screen(false, true);
        assert_eq!(nil, SkillClose::default());
        let closed = close_skill_screen(true, true);
        assert!(closed.closed && closed.controls_restored && closed.body_released);
        let bare = close_skill_screen(true, false);
        assert!(bare.closed && !bare.controls_restored);
    }

    #[test]
    fn a_skill_ring_places_its_nodes() {
        let ring = SkillRing {
            count: 8,
            radius: 10,
            stretch: 5.0,
            turn: 1.0,
            width: 200,
            height: 100,
            page: 0,
            unit: 4,
            scrolled: false,
        };
        assert_eq!(skill_ring_position(3, &ring), (23.0, 75.0));
        let scrolled = SkillRing {
            page: 1,
            scrolled: true,
            ..ring
        };
        assert_eq!(skill_ring_position(3, &scrolled), (19.0, 87.0));
    }

    #[test]
    fn the_skill_ring_hits_its_satellites() {
        let nodes = [(100.0, 100.0)];
        let hit = SkillHit {
            mouse: (100, 27),
            unit: 4,
            turn: 1.0,
            page: 2,
        };
        assert_eq!(skill_node_hit(hit, &nodes), 1);
        assert_eq!(
            skill_node_hit(
                SkillHit {
                    mouse: (170, 77),
                    ..hit
                },
                &nodes
            ),
            2
        );
        assert_eq!(
            skill_node_hit(
                SkillHit {
                    mouse: (0, 0),
                    page: 0,
                    ..hit
                },
                &nodes
            ),
            -1
        );
        assert_eq!(
            skill_node_hit(
                SkillHit {
                    mouse: (100, 27),
                    page: 0,
                    ..hit
                },
                &nodes
            ),
            0
        );
    }

    #[test]
    fn the_skill_hover_stores_validates_and_stamps() {
        let base = skill_hover(SkillHoverInput {
            index: 0,
            current: 0,
            grid: None,
            previous: 0,
            last_positive: 0,
            stamp: 0,
            clock: 0,
            base: 7,
        });
        assert_eq!(base.result, 7);
        assert!(base.called_base);

        let mut grid = [0u8; 64];
        grid[20] = 1;
        let hit = skill_hover(SkillHoverInput {
            index: 0x0203,
            current: 0,
            grid: Some(&grid),
            previous: 0,
            last_positive: 0,
            stamp: 900,
            clock: 1000,
            base: 0,
        });
        assert_eq!(hit.result, 3);
        assert_eq!(hit.current, 0x0203);
        assert_eq!(hit.previous, 0x0203);
        assert_eq!(hit.last_positive, 0x0203);
        assert_eq!(hit.stamp, 900);
        assert!(!hit.called_base);

        let moved = skill_hover(SkillHoverInput {
            index: 0x0203,
            current: 0,
            grid: Some(&grid),
            previous: 0,
            last_positive: 0x0101,
            stamp: 900,
            clock: 1000,
            base: 0,
        });
        assert_eq!(moved.stamp, 1000);

        let same = skill_hover(SkillHoverInput {
            index: 0x0203,
            current: 0,
            grid: Some(&grid),
            previous: 0x0203,
            last_positive: 0x0203,
            stamp: 900,
            clock: 1000,
            base: 0,
        });
        assert_eq!(same.stamp, 900);
        assert_eq!(same.previous, 0x0203);

        let miss = skill_hover(SkillHoverInput {
            index: 0x0203,
            current: 0,
            grid: Some(&[0u8; 64]),
            previous: 0,
            last_positive: 0,
            stamp: 900,
            clock: 1000,
            base: 0,
        });
        assert_eq!(miss.previous, 0);
        assert_eq!(miss.stamp, 900);

        let plain = skill_hover(SkillHoverInput {
            index: 5,
            current: 0,
            grid: None,
            previous: 0,
            last_positive: 0,
            stamp: 0,
            clock: 0,
            base: 0,
        });
        assert_eq!(plain.result, 3);
        assert_eq!(plain.current, 5);
        assert_eq!(plain.previous, 0);
    }

    #[test]
    fn a_stored_slot_places_its_satellites() {
        let ring = SkillRing {
            count: 8,
            radius: 10,
            stretch: 5.0,
            turn: 1.0,
            width: 200,
            height: 100,
            page: 0,
            unit: 4,
            scrolled: false,
        };
        // The node at (177, 75); the first sample straight up, the third
        // 144° on.
        assert_eq!(skill_satellite(3, 1, &ring), (177.0, 2.0));
        assert_eq!(skill_satellite(3, 3, &ring), (220.0, 134.0));
        let scrolled = SkillRing {
            page: 1,
            scrolled: true,
            ..ring
        };
        assert_eq!(skill_satellite(3, 1, &scrolled), (173.0, 14.0));
    }

    fn key_input() -> SkillKeyInput {
        SkillKeyInput {
            key: 0,
            low: 0,
            high: 1,
            index: 3,
            state: 0,
            hide: 0,
            deadline: 0,
            closing: 0,
            clock: 100,
            grid: [1; 8],
            repage: 0,
            hit: 0,
            flags: 0,
        }
    }

    #[test]
    fn the_skill_screen_key_moves_and_dispatches() {
        let back = skill_screen_key(SkillKeyInput {
            key: 0x1001,
            ..key_input()
        });
        assert_eq!(back.index, 2);
        assert_eq!(back.published, Some(2));

        let forward = skill_screen_key(SkillKeyInput {
            key: 0x2001,
            index: 1,
            ..key_input()
        });
        assert_eq!(forward.index, 2);
        assert_eq!(forward.published, Some(2));

        let stopped = skill_screen_key(SkillKeyInput {
            key: 0x2001,
            index: 2,
            ..key_input()
        });
        assert_eq!(stopped.index, 2);
        assert_eq!(stopped.published, None);

        let mut grid = [1; 8];
        grid[2] = 0;
        let backward = skill_screen_key(SkillKeyInput {
            key: 0x1001,
            high: 0,
            low: 1,
            index: 4,
            grid,
            ..key_input()
        });
        assert_eq!(backward.index, 3);
        assert_eq!(backward.published, Some(3));

        let mut grid = [1; 8];
        grid[4] = 0;
        let forward = skill_screen_key(SkillKeyInput {
            key: 0x2001,
            high: 0,
            low: 1,
            index: 4,
            grid,
            ..key_input()
        });
        assert_eq!(forward.index, 5);
        assert_eq!(forward.published, Some(5));

        let base = skill_screen_key(SkillKeyInput {
            key: 0x100,
            high: 4,
            low: 1,
            index: 2,
            ..key_input()
        });
        assert!(base.base);
        assert_eq!(base.published, Some(4));

        let hint = skill_screen_key(SkillKeyInput {
            key: 0x200,
            high: 0,
            low: 3,
            flags: 0x41,
            ..key_input()
        });
        assert!(hint.hint);
        assert_eq!(hint.published, Some(3));
        assert_eq!(hint.hide, 2);
        assert_eq!(hint.deadline, 300);
        assert_eq!(hint.flags, 1);

        let repage = skill_screen_key(SkillKeyInput {
            key: 0x100,
            high: 0,
            low: 0,
            index: 5,
            repage: 1,
            ..key_input()
        });
        assert_eq!(repage.high, 5);
        assert_eq!(repage.published, Some(1));
        assert!(repage.hint);
        assert_eq!(repage.hide, 1);
        assert_eq!(repage.deadline, 400);
        assert_eq!(repage.flags, 0x40);
        assert!(!repage.cursor);
        assert!(!repage.clear_call);

        let nothing = skill_screen_key(SkillKeyInput {
            key: 0x100,
            high: 0,
            low: 0,
            index: 5,
            repage: 0,
            ..key_input()
        });
        assert_eq!(nothing.published, None);
        assert_eq!(nothing.high, 0);
        assert!(!nothing.hint);

        let mut grid = [1; 8];
        grid[1] = 0;
        let refill = skill_screen_key(SkillKeyInput {
            key: 0x100,
            high: 0,
            low: 0,
            index: 5,
            repage: 2,
            grid,
            ..key_input()
        });
        assert_eq!(refill.low, 5);
        assert_eq!(refill.index, 2);
        assert_eq!(refill.published, Some(2));

        let closing = skill_screen_key(SkillKeyInput {
            key: 0x100,
            high: 0,
            low: 0,
            index: 5,
            repage: 1,
            closing: 1,
            hit: 7,
            ..key_input()
        });
        assert_eq!(closing.high, 5);
        assert!(!closing.cursor);
        assert_eq!(closing.hit_published, None);

        let cursor = skill_screen_key(SkillKeyInput {
            key: 1,
            high: 0,
            low: 0,
            hit: 7,
            ..key_input()
        });
        assert!(cursor.cursor);
        assert_eq!(cursor.hit_published, Some(7));
        assert!(!cursor.clear_call);

        let clear = skill_screen_key(SkillKeyInput {
            key: 0x201,
            high: 0,
            low: 0,
            ..key_input()
        });
        assert!(clear.cursor);
        assert!(clear.clear_call);

        let held = skill_screen_key(SkillKeyInput {
            key: 0x201,
            high: 0,
            low: 0,
            state: 2,
            ..key_input()
        });
        assert!(!held.clear_call);
    }

    fn input() -> SkillInputInput {
        SkillInputInput {
            index: 0,
            mask: 0,
            page: 0,
            closing: 0,
            state: 0,
            high: 0,
            low: 0,
            hide: 0,
            read_only: false,
            clock: 100,
            used: [1; 8],
            repage: 0,
            selected: 0,
            base: 7,
            delay: 250,
            open_end: 0,
            close_end: 0,
            hint_fade_end: 0,
        }
    }

    #[test]
    fn the_skill_pointer_input_dispatches_its_branches() {
        let base = handle_skill_input(SkillInputInput {
            index: -2,
            ..input()
        });
        assert!(base.base);
        assert_eq!(base.result, 7);

        let paged = handle_skill_input(SkillInputInput {
            mask: 2,
            page: 1,
            index: 5,
            ..input()
        });
        assert_eq!(paged.result, 0x10);
        assert!(!paged.clear);

        let cleared = handle_skill_input(SkillInputInput {
            mask: 2,
            index: 5,
            ..input()
        });
        assert!(cleared.clear);
        assert_eq!(cleared.result, 1);

        let closing = handle_skill_input(SkillInputInput {
            mask: 2,
            closing: 1,
            index: 5,
            ..input()
        });
        assert_eq!(closing.result, 0x10);
        assert!(!closing.clear);

        let late = handle_skill_input(SkillInputInput {
            mask: 2,
            closing: 2,
            index: -1,
            base: 9,
            ..input()
        });
        assert!(late.base);
        assert_eq!(late.result, 9);

        let hidden = handle_skill_input(SkillInputInput {
            index: 0,
            high: 1,
            ..input()
        });
        assert_eq!(hidden.hide, 2);
        assert_eq!(hidden.hint_fade_end, 300);
        assert!(hidden.hint);
        assert_eq!(hidden.result, 0x10);

        let shown = handle_skill_input(SkillInputInput {
            index: 0x10000,
            mask: 0x2000,
            state: 4,
            ..input()
        });
        assert_eq!(shown.state, 1);
        assert_eq!(shown.open_end, 600);
        assert_eq!(shown.result, 0x10);

        let second = handle_skill_input(SkillInputInput {
            index: 0x10000,
            mask: 0x2000,
            state: 0,
            delay: 250,
            ..input()
        });
        assert_eq!(second.state, 2);
        assert_eq!(second.close_end, 350);

        let high = handle_skill_input(SkillInputInput {
            index: 5,
            mask: 0x2000,
            repage: 1,
            ..input()
        });
        assert_eq!(high.high, 5);
        assert!(high.hint);
        assert_eq!(high.hide, 1);
        assert_eq!(high.hint_fade_end, 400);

        let low = handle_skill_input(SkillInputInput {
            index: 5,
            mask: 0x2000,
            repage: 2,
            ..input()
        });
        assert_eq!(low.low, 5);
        assert!(low.fill);

        let used = handle_skill_input(SkillInputInput {
            index: 5,
            mask: 0x2000,
            low: 3,
            selected: 2,
            ..input()
        });
        assert!(!used.select);
        assert_eq!(used.slot, None);
        assert!(used.hint);

        // Row slot 5 reads mask byte 4.
        let mut free = input();
        free.used[4] = 0;
        let written = handle_skill_input(SkillInputInput {
            index: 5,
            mask: 0x2000,
            low: 3,
            selected: 2,
            used: free.used,
            ..input()
        });
        assert!(written.select);
        assert_eq!(written.slot, Some((-1, 3)));

        let first = handle_skill_input(SkillInputInput {
            index: 1,
            mask: 0x2000,
            high: 2,
            ..input()
        });
        assert!(first.select);

        let flagged = handle_skill_input(SkillInputInput {
            index: 5,
            mask: 0x2000,
            low: 1,
            read_only: true,
            ..input()
        });
        assert_eq!(flagged.result, 0x10);
        assert!(!flagged.hint);

        let no_mask = handle_skill_input(SkillInputInput {
            index: 5,
            low: 1,
            ..input()
        });
        assert_eq!(no_mask.result, 0x10);
        assert!(!no_mask.hint);
    }

    fn yes(_: i32) -> bool {
        true
    }

    fn no(_: i32) -> bool {
        false
    }

    #[test]
    fn the_skill_link_search_takes_the_adjacent_or_scanned_slot() {
        let adjacent = pick_skill_satellite(SkillLinkInput {
            point: (0, 0),
            direction: (1.0, 0.0),
            hit: 0x0202,
            node_point: (10.0, 10.0),
            nodes: &[],
            reachable: &yes,
            unit: 1,
            turn: 1.0,
        });
        // Slot 1 is straight up (10, −8); slot 3 is at (21, 25).
        assert_eq!(adjacent.code, 0x0201);
        assert_eq!(adjacent.point, (10, -8));

        let nodes = [(0x100, (0.0, 0.0))];
        let scanned = pick_skill_satellite(SkillLinkInput {
            point: (0, 0),
            direction: (0.0, 1.0),
            hit: 0,
            node_point: (0.0, 0.0),
            nodes: &nodes,
            reachable: &yes,
            unit: 1,
            turn: 1.0,
        });
        assert_eq!(scanned.code, 0x103);
        assert_eq!(scanned.point, (11, 15));

        let unreachable = pick_skill_satellite(SkillLinkInput {
            point: (3, 4),
            direction: (1.0, 0.0),
            hit: 0,
            node_point: (0.0, 0.0),
            nodes: &nodes,
            reachable: &no,
            unit: 1,
            turn: 1.0,
        });
        assert_eq!(unreachable.code, 0);
        assert_eq!(unreachable.point, (3, 4));

        let only = |code: i32| code == 0x103;
        let fallback = pick_skill_satellite(SkillLinkInput {
            point: (0, 0),
            direction: (0.0, 1.0),
            hit: 0x0202,
            node_point: (10.0, 10.0),
            nodes: &nodes,
            reachable: &only,
            unit: 1,
            turn: 1.0,
        });
        assert_eq!(fallback.code, 0x103);
        assert_eq!(fallback.point, (11, 15));
    }

    fn point_input() -> SkillPointInput {
        SkillPointInput {
            state: 0,
            page: 0,
            base: 0x10,
            owns: true,
            allowed: true,
            point: (100, 50),
            width: 200,
            height: 100,
            unit: 4,
            high: 1,
            low: 0,
            high_point: (100.0, 50.0),
            low_point: (0.0, 0.0),
            fallback: 99,
        }
    }

    #[test]
    fn the_skill_point_guards_bands_and_nodes() {
        assert_eq!(
            hit_skill_point(SkillPointInput {
                state: 2,
                page: 1,
                ..point_input()
            }),
            0
        );

        assert_eq!(
            hit_skill_point(SkillPointInput {
                owns: false,
                base: 7,
                ..point_input()
            }),
            7
        );

        assert_eq!(
            hit_skill_point(SkillPointInput {
                base: 5,
                allowed: false,
                ..point_input()
            }),
            5
        );

        assert_eq!(
            hit_skill_point(SkillPointInput {
                page: 2,
                ..point_input()
            }),
            0x1_0000
        );

        assert_eq!(
            hit_skill_point(SkillPointInput {
                state: 4,
                ..point_input()
            }),
            0x10
        );

        // The forget confirmation (high, at (100, 50)): Yes and No six to
        // ten units below, beyond four units either side.
        let at = |point| {
            hit_skill_point(SkillPointInput {
                point,
                ..point_input()
            })
        };
        assert_eq!(at((80, 80)), 1);
        assert_eq!(at((120, 80)), 2);
        assert_eq!(at((100, 80)), -1);
        assert_eq!(at((84, 80)), -1);
        assert_eq!(at((80, 74)), 1);
        assert_eq!(at((80, 70)), -1);
        assert_eq!(at((80, 95)), -1);
        // It is checked before the learning row.
        assert_eq!(
            hit_skill_point(SkillPointInput {
                point: (100, 20),
                low: 1,
                low_point: (100.0, 40.0),
                ..point_input()
            }),
            -1
        );

        // The learning row (low, at (100, 40)): eight slots in the eight
        // units above, ten units apart, rounded half to even.
        let low = SkillPointInput {
            high: 0,
            low: 1,
            low_point: (100.0, 40.0),
            ..point_input()
        };
        let row = |point| hit_skill_point(SkillPointInput { point, ..low });
        assert_eq!(row((80, 20)), 4);
        assert_eq!(row((120, 20)), 5);
        assert_eq!(row((0, 20)), 2);
        assert_eq!(row((100, 20)), 4);
        assert_eq!(row((260, 20)), 8);
        assert_eq!(row((-80, 20)), -1);
        assert_eq!(row((100, 5)), -1);
        assert_eq!(row((100, 45)), -1);

        assert_eq!(
            hit_skill_point(SkillPointInput {
                high: 0,
                low: 0,
                ..point_input()
            }),
            99
        );
    }

    #[test]
    fn the_skill_screen_fades_to_its_deadline() {
        let closed = skill_screen_fade(4, 0, 0);
        assert_eq!(closed.alpha, 0.0);
        assert_eq!(closed.sound, 0.0);

        let plain = skill_screen_fade(0, 0, 0);
        assert_eq!(plain.alpha, 1.0);
        assert_eq!(plain.sound, 1.0);

        let passed = skill_screen_fade(1, 500, 600);
        assert!(passed.cleared);
        assert_eq!(passed.alpha, 1.0);

        let fading = skill_screen_fade(1, 1000, 800);
        assert!(!fading.cleared);
        assert!((fading.alpha - 0.84).abs() < 1e-5);
        assert!((fading.sound - 0.9744).abs() < 1e-5);
    }

    #[test]
    fn the_skill_screen_fades_in_or_closes() {
        let quiet = skill_screen_fade_in(0, 500, 600, 0.5, 250.0, 2);
        assert_eq!(quiet.state, 0);
        assert_eq!(quiet.sound, 0.5);
        assert!(!quiet.closed);

        let page_two = skill_screen_fade_in(2, 500, 600, 0.5, 250.0, 2);
        assert_eq!(page_two.state, 4);
        assert_eq!(page_two.sound, 0.0);
        assert_eq!(page_two.alpha, Some(0.0));

        let closing = skill_screen_fade_in(2, 500, 600, 0.5, 250.0, 1);
        assert!(closing.closed);
        assert_eq!(closing.state, 4);

        let fading = skill_screen_fade_in(2, 1000, 800, 0.5, 250.0, 2);
        assert!((fading.sound - 0.4).abs() < 1e-6);
        assert!(!fading.closed);
    }

    fn skills() -> [Skill; SKILLS] {
        [Skill::default(); SKILLS]
    }

    fn skill(points: u16, techniques: [u8; 6]) -> Skill {
        Skill { points, techniques }
    }

    /// Identity stand-ins for the experience system's conversions: the
    /// screen's rules only pass the values through them.
    const SAME: ExperienceRates = ExperienceRates {
        worth: |xp| xp,
        experience: |worth| worth,
    };

    #[test]
    fn the_requirement_mask_marks_chosen_absent_and_missing_nodes() {
        // Close Combat at seven funded levels holding Remise to Riposte:
        // Draw, Impel and Brace are absent; Dual Wield has both of its
        // prerequisites.
        let mut tree = skills();
        tree[0] = skill(35_000, [1, 2, 3, 4, 0, 0]);
        assert_eq!(skill_requirements(0, &tree), [1, 1, 1, 1, 3, 3, 3, 0]);
        // Without Riposte, Dual Wield misses one: both are needed.
        tree[0] = skill(35_000, [1, 2, 3, 0, 0, 0]);
        assert_eq!(skill_requirements(0, &tree), [1, 1, 1, 0, 3, 3, 3, 2]);
        // Riposte chosen but not funded (one level) does not count.
        tree[0] = skill(5_000, [1, 4, 0, 0, 0, 0]);
        assert_eq!(skill_requirements(0, &tree), [1, 0, 0, 1, 3, 3, 3, 2]);
        // Seven funded levels read the next skill's points as the seventh
        // technique byte: Ranged Combat's 4 points count as Riposte.
        tree[0] = skill(35_000, [1, 0, 0, 0, 0, 0]);
        tree[1].points = 4;
        assert_eq!(skill_requirements(0, &tree), [1, 0, 0, 0, 3, 3, 3, 0]);
        // Ranged Combat has no nodes.
        assert_eq!(skill_requirements(1, &tree), [3; 8]);
        // Armour's second tier needs the first, the third the second.
        let mut armour = skills();
        armour[2] = skill(10_000, [1, 0, 0, 0, 0, 0]);
        assert_eq!(skill_requirements(2, &armour), [1, 0, 2, 0, 0, 3, 3, 3]);
        assert_eq!(skill_requirements(6, &armour), [0; 8]);
    }

    #[test]
    fn a_nodes_press_state_reads_the_funded_levels_and_the_ready_flag() {
        let mut tree = skills();
        tree[0] = skill(5_000, [1, 0, 0, 0, 0, 0]);
        assert_eq!(skill_node_state(&tree, 0x101, false), NODE_LEARNT);
        assert_eq!(skill_node_state(&tree, 0x102, true), NODE_OPEN);
        assert_eq!(skill_node_state(&tree, 0x102, false), NODE_LOCKED);
        assert_eq!(skill_node_state(&tree, 0x103, true), NODE_LOCKED);
        assert_eq!(skill_node_state(&tree, 0x001, true), NODE_LOCKED);
        assert_eq!(skill_node_state(&tree, 0x100, true), NODE_LOCKED);
        assert_eq!(skill_node_state(&tree, 0x701, true), NODE_LOCKED);
        // A technique chosen ahead of the points is learnt once the skill
        // is more than ten points past its funded level.
        tree[0] = skill(5_010, [1, 2, 0, 0, 0, 0]);
        assert_eq!(skill_node_state(&tree, 0x102, false), NODE_OPEN);
        tree[0].points = 5_011;
        assert_eq!(skill_node_state(&tree, 0x102, false), NODE_LEARNT);
        // The seventh level's byte is the next skill's low points byte.
        tree[0] = skill(30_000, [1, 2, 3, 4, 5, 6]);
        assert_eq!(skill_node_state(&tree, 0x107, false), NODE_LOCKED);
        tree[1].points = 1;
        assert_eq!(skill_node_state(&tree, 0x107, false), NODE_OPEN);
    }

    #[test]
    fn unlearning_moves_the_committed_and_the_record_points() {
        // At or below the committed levels both lose 5000; the later
        // technique moves down.
        let mut tree = skills();
        let mut base = [0u16; SKILLS];
        tree[0] = skill(10_000, [1, 2, 0, 0, 0, 0]);
        base[0] = 10_000;
        unlearn_skill_level(&mut tree, &mut base, 1, 1);
        assert_eq!(tree[0], skill(5_000, [2, 0, 0, 0, 0, 0]));
        assert_eq!(base[0], 5_000);

        // One past the committed levels: the record falls back.
        let mut base = [0u16; SKILLS];
        tree[0] = skill(5_000, [1, 0, 0, 0, 0, 0]);
        unlearn_skill_level(&mut tree, &mut base, 1, 1);
        assert_eq!(tree[0], skill(0, [0; 6]));
        assert_eq!(base[0], 0);

        // Further out: a choice ahead of the points keeps the funded points.
        tree[0] = skill(5_000, [1, 2, 0, 0, 0, 0]);
        unlearn_skill_level(&mut tree, &mut base, 1, 2);
        assert_eq!(tree[0], skill(5_000, [1, 0, 0, 0, 0, 0]));

        // 16-bit arithmetic wraps.
        tree[0] = skill(0, [3, 0, 0, 0, 0, 0]);
        base[0] = 5_000;
        unlearn_skill_level(&mut tree, &mut base, 1, 1);
        assert_eq!(tree[0], skill(60_536, [0; 6]));
        assert_eq!(base[0], 0);

        // Armour I takes II (which needs it) and II takes III, each from the
        // last funded level down.
        let mut tree = skills();
        let mut base = [0u16; SKILLS];
        tree[2] = skill(15_000, [1, 2, 3, 0, 0, 0]);
        base[2] = 15_000;
        unlearn_skill_level(&mut tree, &mut base, 3, 1);
        assert_eq!(tree[2], skill(0, [0; 6]));
        assert_eq!(base[2], 0);

        // A group of 0 or past the six does nothing.
        let before = tree;
        unlearn_skill_level(&mut tree, &mut base, 0, 1);
        unlearn_skill_level(&mut tree, &mut base, 7, 1);
        assert_eq!(tree, before);
    }

    #[test]
    fn the_experience_pass_tops_up_charges_levels_and_feeds_the_surplus() {
        // The partial committed level costs its missing 2500 at weight 4;
        // the rest is left over.
        let mut data = CharacterData::new();
        data.record.skills[0] = skill(2_500, [0; 6]);
        let base = [2_500, 0, 0, 0, 0, 0];
        assert_eq!(spend_skill_experience(&mut data, &base, 5_000), 2_500);
        assert_eq!(data.record.skills[0].points, 5_000);

        // A surplus choice takes the rest: 2500 short of a 6250 level at
        // weight 5 adds 2500 × 4 / 5 on top of the level below.
        data.record.skills[0] = skill(2_500, [1, 2, 0, 0, 0, 0]);
        assert_eq!(spend_skill_experience(&mut data, &base, 5_000), 0);
        assert_eq!(data.record.skills[0].points, 7_000);

        // A budget short of the top-up adds its share: 1000 × 4 / 4.
        data.record.skills[0] = skill(2_500, [0; 6]);
        assert_eq!(spend_skill_experience(&mut data, &base, 1_000), 0);
        assert_eq!(data.record.skills[0].points, 3_500);

        // Two record levels above none committed cost 5000 then 6250.
        let mut data = CharacterData::new();
        data.record.skills[0] = skill(10_000, [1, 2, 0, 0, 0, 0]);
        assert_eq!(spend_skill_experience(&mut data, &[0; 6], 20_000), 8_750);

        // Overspent, the budget wraps; its unsigned compare still buys the
        // surplus level (7500 at weight 6) and wraps further.
        data.record.skills[0] = skill(10_000, [1, 2, 3, 0, 0, 0]);
        let left = spend_skill_experience(&mut data, &[0; 6], 6_000);
        assert_eq!(left as i32, -12_750);
        assert_eq!(data.record.skills[0].points, 15_000);
    }

    #[test]
    fn the_ready_flag_stops_at_the_first_chosen_next_technique() {
        let mut screen = SkillScreen::open(bounds(), 4, false, 0, 0);
        let mut data = CharacterData::new();
        screen.refresh(&mut data, SAME);
        assert!(screen.ready);
        data.record.skills[0] = skill(5_011, [1, 2, 0, 0, 0, 0]);
        screen.refresh(&mut data, SAME);
        assert!(!screen.ready);
        data.record.skills[0].points = 5_010;
        screen.refresh(&mut data, SAME);
        assert!(screen.ready);

        // The scan stops at the first set next byte: later levels are not
        // counted against the cap.
        let mut data = CharacterData::new();
        for entry in data.record.skills.iter_mut() {
            *entry = skill(15_000, [1, 2, 0, 0, 0, 0]);
        }
        screen.refresh(&mut data, SAME);
        assert!(!screen.ready);
        data.record.skills[0] = skill(10_000, [1, 2, 3, 0, 0, 0]);
        screen.refresh(&mut data, SAME);
        assert!(screen.ready);

        // Experience left over keeps a level ready; a locked screen never is.
        screen.experience = 1_000;
        for entry in data.record.skills.iter_mut() {
            *entry = skill(15_000, [1, 2, 0, 0, 0, 0]);
        }
        screen.base = [15_000; SKILLS];
        screen.refresh(&mut data, SAME);
        assert_eq!(screen.leftover, 1_000);
        assert!(screen.ready);
        screen.locked = true;
        screen.refresh(&mut data, SAME);
        assert!(!screen.ready);
    }

    #[test]
    fn attaching_a_record_commits_its_points_or_clears_them() {
        let mut screen = SkillScreen::open(bounds(), 4, false, 0, 0);
        let mut data = CharacterData::new();
        for (k, entry) in data.record.skills.iter_mut().enumerate() {
            *entry = skill(1_000 * (k as u16 + 1), [0; 6]);
        }
        screen.attach(&mut data, false, SAME);
        assert_eq!(screen.base, [1_000, 2_000, 3_000, 4_000, 5_000, 6_000]);
        assert!(screen.ready);
        screen.attach(&mut data, true, SAME);
        assert_eq!(screen.base, [0; SKILLS]);
    }

    fn bounds() -> SkillBounds {
        SkillBounds {
            left: 0,
            right: 400,
            top: 0,
            bottom: 300,
        }
    }

    #[test]
    fn a_press_opens_the_row_and_a_second_press_writes_the_choice() {
        let mut screen = SkillScreen::open(bounds(), 4, false, 0, 0);
        let mut data = CharacterData::new();
        data.record.skills[0] = skill(5_000, [1, 0, 0, 0, 0, 0]);
        screen.refresh(&mut data, SAME);
        assert_eq!(
            screen.press_node(&data.record.skills, 0x102, 1_000),
            NODE_OPEN
        );
        assert_eq!(screen.learning, 0x102);
        assert_eq!(screen.requirements, [1, 0, 0, 0, 3, 3, 3, 2]);
        assert_eq!((screen.popup, screen.popup_deadline), (1, 1_300));
        // Dual Wield's prerequisites are missing: nothing is written.
        assert!(!screen.commit(&mut data, 8, 1_100, SAME));
        assert!(!screen.commit(&mut data, 0, 1_100, SAME));
        // Riposte goes into the second level's slot; the opening popup
        // keeps fading in.
        assert!(screen.commit(&mut data, 4, 1_100, SAME));
        assert_eq!(data.record.skills[0].techniques, [1, 4, 0, 0, 0, 0]);
        assert_eq!(screen.popup, 1);
        screen.popup = 0;
        screen.commit(&mut data, 8, 1_200, SAME);
        assert_eq!((screen.popup, screen.popup_deadline), (2, 1_400));

        // A surplus choice elsewhere is unlearned first.
        let mut screen = SkillScreen::open(bounds(), 4, false, 0, 0);
        let mut data = CharacterData::new();
        data.record.skills[0] = skill(5_000, [1, 0, 0, 0, 0, 0]);
        data.record.skills[2] = skill(0, [1, 0, 0, 0, 0, 0]);
        screen.ready = true;
        screen.press_node(&data.record.skills, 0x102, 0);
        assert!(screen.commit(&mut data, 2, 0, SAME));
        assert_eq!(data.record.skills[0].techniques, [1, 2, 0, 0, 0, 0]);
        assert_eq!(data.record.skills[2], skill(0, [0; 6]));

        // A learnt node asks to be forgotten; Yes unlearns it.
        let mut screen = SkillScreen::open(bounds(), 4, false, 0, 0);
        let mut data = CharacterData::new();
        data.record.skills[0] = skill(5_000, [1, 0, 0, 0, 0, 0]);
        screen.base[0] = 5_000;
        assert_eq!(
            screen.press_node(&data.record.skills, 0x101, 0),
            NODE_LEARNT
        );
        assert_eq!(screen.forgetting, 0x101);
        assert!(!screen.commit(&mut data, 2, 0, SAME));
        assert!(screen.commit(&mut data, 1, 0, SAME));
        assert_eq!(data.record.skills[0], skill(0, [0; 6]));
        assert_eq!(screen.base[0], 0);
        // A locked node does nothing.
        assert_eq!(
            screen.press_node(&data.record.skills, 0x103, 0),
            NODE_LOCKED
        );
    }

    #[test]
    fn a_screen_opens_on_the_ring_or_a_page_and_fades() {
        let mut screen = SkillScreen::open(bounds(), 4, false, 0, 100);
        assert_eq!((screen.count, screen.radius, screen.state), (6, 64, 1));
        assert_eq!(screen.reveal_deadline, 600);
        assert_eq!(SkillScreen::open(bounds(), 4, true, 0, 0).radius, 58);
        let page = SkillScreen::open(bounds(), 4, false, 2, 0);
        assert_eq!((page.count, page.radius, page.page), (4, 72, 2));
        assert_eq!(page.stretch, 1.25);
        let inset_page = SkillScreen::open(bounds(), 4, true, 2, 0);
        assert_eq!((inset_page.radius, inset_page.stretch), (64, 0.9));
        assert_eq!(screen.stretch, 1.0);
        assert_eq!(page.slot_group(1), Some(1));
        assert_eq!(screen.slot_group(2), Some(2));
        assert_eq!(screen.group_slot(1), Some(6));
        assert_eq!(screen.slot_group(7), None);
        // The first slot sits at −150°: the sine across, the cosine down.
        assert_eq!(skill_ring_position(1, &screen.ring(1.0)), (72.0, -84.0));

        // Half way through the reveal: alpha 0.75, the scale eased again.
        assert_eq!(screen.fade(100), (0.0, 0.0));
        assert_eq!(screen.fade(350), (0.9375, 0.75));
        assert_eq!(screen.fade(600), (1.0, 1.0));
        assert_eq!(screen.state, 0);
        screen.begin_hide(700);
        assert_eq!(screen.hide_deadline, 956);
        assert_eq!(screen.fade(828), (1.0, 0.5));
        assert_eq!(screen.fade(956), (0.0, 0.0));
        assert_eq!(screen.state, 4);
    }

    #[test]
    fn an_icon_quad_is_an_atlas_cell() {
        let first = skill_icon_quad(1, 64, 256, 128, 10.0, 20.0).unwrap();
        assert_eq!(first.uv, [0.0, 0.0, 0.25, 0.5]);
        assert_eq!((first.x, first.y, first.size), (10.0, 20.0, 64.0));
        let sixth = skill_icon_quad(6, 64, 256, 128, 0.0, 0.0).unwrap();
        assert_eq!(sixth.uv, [0.25, 0.5, 0.5, 1.0]);
        assert_eq!(skill_icon_quad(0, 64, 256, 128, 0.0, 0.0), None);
        assert_eq!(skill_icon_quad(1, 0, 256, 128, 0.0, 0.0), None);
    }

    #[test]
    fn a_groups_icons_stand_on_its_pentagon() {
        let mut tree = skills();
        tree[0] = skill(10_000, [1, 2, 0, 0, 0, 0]);
        let icons = skill_group_icons(Some(&tree), 0, 100.0, 100.0, 4, 1.0, false, 0x102);
        assert_eq!(icons.len(), 2);
        // Up first, then 72° on: (0, 73) and (70, 23) from (84, 84).
        assert_eq!(
            (icons[0].x, icons[0].y, icons[0].technique),
            (84.0, 11.0, 1)
        );
        assert_eq!(
            (icons[1].x, icons[1].y, icons[1].technique),
            (154.0, 61.0, 2)
        );
        assert!(!icons[0].hovered && icons[1].hovered);

        // A ready level adds the "choose" icon.
        tree[0] = skill(5_000, [1, 0, 0, 0, 0, 0]);
        let ready = skill_group_icons(Some(&tree), 0, 0.0, 0.0, 4, 1.0, true, 0);
        assert_eq!(ready.len(), 2);
        assert_eq!(ready[1].technique, 0);
        // A choice ahead of the points draws at half alpha.
        tree[0] = skill(5_000, [1, 3, 0, 0, 0, 0]);
        let ahead = skill_group_icons(Some(&tree), 0, 0.0, 0.0, 4, 1.0, false, 0);
        assert!(ahead[1].half && !ahead[0].half);
        // An empty funded level draws the "choose" icon and ends the row.
        tree[0] = skill(10_000, [0, 2, 0, 0, 0, 0]);
        assert_eq!(
            skill_group_icons(Some(&tree), 0, 0.0, 0.0, 4, 1.0, false, 0).len(),
            1
        );
        // With no record the preview shows its three techniques.
        let preview = skill_group_icons(None, 3, 0.0, 0.0, 4, 1.0, false, 0);
        let ids: Vec<u8> = preview.iter().map(|icon| icon.technique).collect();
        assert_eq!(ids, [1, 3, 5]);
        assert!(preview[2].half);
    }

    #[test]
    fn orbit_bars_and_spokes_span_their_radii() {
        let mut tree = skills();
        tree[0] = skill(7_500, [1, 3, 0, 0, 0, 0]);
        let bars = skill_orbit_bars(Some(&tree), 0, 100.0, 100.0, 4, 1.0);
        assert_eq!(bars.len(), 2);
        // Slot 0 points up: 81.2 to 105.2 units above, 56 either side.
        let up = bars[0].corners;
        assert_eq!(
            (up[0][0], up[1][0], up[2][0], up[3][0]),
            (44.0, 44.0, 156.0, 156.0)
        );
        assert!((up[0][1] - 18.8).abs() < 1e-4 && (up[1][1] + 5.2).abs() < 1e-4);
        assert!((up[2][1] + 5.2).abs() < 1e-4 && (up[3][1] - 18.8).abs() < 1e-4);
        assert_eq!(bars[0].fill, 1.0);
        // The choice ahead fills by 2500 × 0.0002.
        assert!((bars[1].fill - 0.5).abs() < 1e-6);
        // 0.99 in single precision is not below the double 0.99.
        tree[0] = skill(9_950, [1, 2, 0, 0, 0, 0]);
        let full = skill_orbit_bars(Some(&tree), 0, 0.0, 0.0, 4, 1.0);
        assert_eq!(full[1].fill, 1.0);
        tree[0] = skill(5_000, [1, 0, 0, 0, 0, 0]);
        assert_eq!(skill_orbit_bars(Some(&tree), 0, 0.0, 0.0, 4, 1.0).len(), 1);

        let spokes = skill_node_spokes(0.0, 0.0, 4, 1.0);
        assert_eq!((spokes[0][0][0], spokes[0][2][0]), (-72.0, 72.0));
        assert!((spokes[0][0][1] + 49.2).abs() < 1e-4);
        assert!((spokes[0][1][1] + 105.2).abs() < 1e-4);
    }

    #[test]
    fn the_popup_fade_opens_and_closes() {
        let plain = skill_popup_fade(0, 1_000, 500);
        assert_eq!((plain.alpha, plain.scale, plain.closed), (1.0, 1.0, false));
        // Closing: (100 × 0.005)².
        let closing = skill_popup_fade(2, 700, 600);
        assert_eq!((closing.alpha, closing.scale), (0.25, 1.0));
        let closed = skill_popup_fade(2, 500, 600);
        assert!(closed.closed);
        assert_eq!(closed.flags, 0);
        // Opening half way through 300 ms: 0.75, the width 0.9375.
        let opening = skill_popup_fade(1, 900, 750);
        assert!((opening.alpha - 0.75).abs() < 1e-6);
        assert!((opening.scale - 0.9375).abs() < 1e-6);
        let opened = skill_popup_fade(1, 500, 600);
        assert_eq!((opened.flags, opened.alpha), (0, 1.0));
    }

    #[test]
    fn the_hover_tip_slides_in_and_out() {
        assert_eq!(selection_tip_slide(5, 100), Some(0.5));
        assert_eq!(selection_tip_slide(0, 50), Some(0.75));
        assert_eq!(selection_tip_slide(5, 200), Some(1.0));
        assert_eq!(selection_tip_slide(0, 200), None);
    }

    #[test]
    fn the_ring_tooltips_place_their_boxes_and_titles() {
        assert_eq!(clamp_tooltip(300.0, &[10, 20], 164.0, 500), 300.0);
        assert_eq!(clamp_tooltip(400.0, &[10, 20], 164.0, 500), 306.0);

        let (tip, name) = ring_group_tooltip(300.0, 200.0, 61, 4, 1.0);
        assert_eq!(
            (tip.left, tip.top, tip.width, tip.height),
            (266.0, 200.0, 68.0, 16.0)
        );
        assert!((tip.alpha - 0.6).abs() < 1e-6);
        assert_eq!(name, (270.0, 216.0));

        let node = ring_node_tooltip(300.0, 200.0, 4, 1_000, &[], 0.5);
        assert_eq!(
            (node.tip.left, node.tip.top, node.tip.width, node.tip.height),
            (136.0, 224.0, 328.0, 56.0)
        );
        assert_eq!((node.title_y, node.text), (240.0, (148.0, 248.0)));

        let mut tree = skills();
        tree[0] = skill(5_000, [4, 0, 0, 0, 0, 0]);
        assert_eq!(ring_node_title(&tree, 0x101), Some(("Riposte", true)));
        assert_eq!(ring_node_title(&tree, 0x102), Some(("Close Combat", false)));
        assert_eq!(
            ring_node_title(&tree, 0x201),
            Some(("Ranged Combat", false))
        );
        assert_eq!(ring_node_title(&tree, 0x100), None);
    }

    #[test]
    fn the_learning_row_places_eight_icons_and_the_hovered_status() {
        let mask = [1, 0, 0, 0, 3, 3, 3, 2];
        let row = learning_row(100.0, 100.0, 4, 1.0, 1_000, &[], 1.0, &mask, 2);
        assert_eq!(
            (row.tip.left, row.tip.top, row.tip.width, row.tip.height),
            (-64.0, 60.0, 328.0, 112.0)
        );
        assert_eq!((row.icons[0].x, row.icons[0].y), (-56.0, 68.0));
        assert_eq!(row.icons[7].x, 224.0);
        assert!(row.icons[1].hovered && !row.icons[0].hovered);
        assert_eq!((row.icons[0].alpha, row.icons[1].alpha), (0.5, 1.0));
        assert_eq!((row.title_y, row.text), (124.0, (-52.0, 132.0)));
        assert_eq!(row.status, None);
        let status = |hover| learning_row(0.0, 0.0, 4, 1.0, 1_000, &[], 1.0, &mask, hover).status;
        assert_eq!(status(1), Some(STATUS_CHOSEN));
        assert_eq!(status(8), Some(STATUS_REQUIREMENTS));
        assert_eq!(status(5), Some(STATUS_UNAVAILABLE));
        assert_eq!(status(9), None);
    }

    #[test]
    fn the_forget_dialog_places_its_lines() {
        let dialog = forget_dialog_place(100.0, 200.0, 4, 1.0, 800, &[], 1.0);
        assert_eq!(
            (
                dialog.tip.left,
                dialog.tip.top,
                dialog.tip.width,
                dialog.tip.height
            ),
            (-64.0, 168.0, 328.0, 80.0)
        );
        assert_eq!((dialog.text_left, dialog.text_width), (-64.0, 328.0));
        assert_eq!(
            (dialog.title_y, dialog.choice_y, dialog.question_y),
            (188.0, 240.0, 212.0)
        );
        assert_eq!((dialog.yes_x, dialog.no_x), (52.0, 124.0));
        assert!(dialog.choices);
        assert!(!forget_dialog_place(100.0, 200.0, 4, 1.0, 800, &[], 0.5).choices);
    }
}
