//! The skill screen's node model and placement.
//!
//! A skill tree has six groups of eight nodes. The character record owns the
//! skill points, the chosen techniques and the prerequisite table
//! ([`exa_character::record`]); this module reads them, lays the ring and its
//! satellites out and models the screen's interaction, tooltip and draw
//! geometry. A positive index names a group and a node; a non-positive index
//! takes the scaled base or the centre of the bounds.
//!
//! Evidence: the retail skill screen's methods (`docs/UI.md`, Forms).

use exa_character::record::slot_prerequisites;
use exa_formats::characters::{POINTS_PER_TECHNIQUE, SKILL_NAMES, SKILLS, Skill, TECHNIQUE_NAMES};

/// The nodes a skill group can hold (1-based node ids 1 to 8).
pub const NODES_PER_GROUP: usize = 8;
/// A node's requirement state: it cannot be chosen yet (no node there).
pub const NODE_UNAVAILABLE: u8 = 0;
/// A node's requirement state: it may be chosen, or is already chosen.
pub const NODE_AVAILABLE: u8 = 1;
/// A node's requirement state: its prerequisites are not met.
pub const NODE_REQUIREMENTS_NOT_MET: u8 = 2;
/// A node's requirement state: there is no node in the slot.
pub const NODE_EMPTY: u8 = 3;

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
    pub offset_a: i32,
    pub offset_b: i32,
    pub turn: f32,
    pub width: i32,
    pub height: i32,
    pub page: i32,
    pub unit: i32,
    pub scrolled: bool,
}

/// Places a skill node on the ring.
pub fn skill_ring_position(slot: i32, ring: &SkillRing) -> (f32, f32) {
    let unit = ring.unit as f32;
    let centre_x = ring.width / 2
        - if ring.scrolled && ring.page > 0 {
            ring.unit
        } else {
            0
        };
    let centre_y = ring.height / 2 - if ring.page == 0 { 3 * ring.unit } else { 0 };
    let angle = (slot as f32 - 3.5) * (std::f32::consts::TAU / ring.count as f32) + 1.0 - ring.turn;
    let x = centre_x as f32
        + (ring.offset_a as f32 * (unit * angle.cos()) * ring.offset_b as f32).round_ties_even()
            * ring.turn;
    let y = centre_y as f32
        + (ring.offset_a as f32 * (unit * angle.sin()) * ring.offset_b as f32).round_ties_even()
            * ring.turn;
    (x, y)
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

/// Places the stored slot's satellite at a sample index.
pub fn skill_satellite(stored: i32, sample: i32, ring: &SkillRing) -> (f32, f32) {
    let unit = ring.unit as f32;
    let centre_x = ring.width / 2
        - if ring.scrolled && ring.page > 0 {
            ring.unit
        } else {
            0
        };
    let centre_y = ring.height / 2 - if ring.page == 0 { 3 * ring.unit } else { 0 };
    let angle =
        (stored as f32 - 2.5) * (std::f32::consts::TAU / ring.count as f32) + 1.0 - ring.turn;
    let node_x = centre_x as f32
        + (ring.offset_a as f32 * (unit * angle.cos()) * ring.offset_b as f32).round_ties_even()
            * ring.turn;
    let node_y = centre_y as f32
        + (ring.offset_a as f32 * (unit * angle.sin())).round_ties_even() * ring.turn;
    let sample_angle = (sample - 1) as f32 * std::f32::consts::TAU * 0.2;
    let radius = f64::from(ring.unit) * 18.3;
    let x =
        node_x + ((radius * f64::from(sample_angle.cos())).round_ties_even() as f32) * ring.turn;
    let y =
        node_y - ((radius * f64::from(sample_angle.sin())).round_ties_even() as f32) * ring.turn;
    (x, y)
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
    pub high: i32,
    pub low: i32,
    pub hide: u8,
    pub flag_1ea: bool,
    pub clock: i32,
    pub used: [u8; 8],
    pub repage: i32,
    pub selected: i32,
    pub base: i32,
    pub delay: i32,
    pub deadline_160: i32,
    pub deadline_164: i32,
    pub deadline_15c: i32,
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
    pub deadline_160: i32,
    pub deadline_164: i32,
    pub deadline_15c: i32,
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
        deadline_160: input.deadline_160,
        deadline_164: input.deadline_164,
        deadline_15c: input.deadline_15c,
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
            out.deadline_15c = input.clock + 200;
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
            out.deadline_160 = input.clock + 500;
            return out;
        }
        if out.state & 2 != 0 {
            return out;
        }
        out.state |= 2;
        out.deadline_164 = input.clock + input.delay;
        return out;
    }
    if input.flag_1ea {
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
        out.deadline_15c = input.clock + 300;
        skill_input_finish(&input, &mut out);
        return out;
    }
    if out.high > 0 {
        if out.index == 1 {
            out.select = true;
        }
    } else if input.used.get(out.index as usize).copied().unwrap_or(0) == 0 {
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
        out.deadline_15c = input.clock + 200;
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
    pub high: i32,
    pub low: i32,
    pub high_point: (f32, f32),
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
    if input.high > 0 {
        let (px, py) = input.high_point;
        if py > y as f32 {
            return -1;
        }
        if py + 4.0 * (unit as f32) < y as f32 {
            return -1;
        }
        let dx = (x as f32 - px).round_ties_even() as i32;
        if dx > -13 * unit && dx < -4 * unit {
            return 1;
        }
        if dx > 4 * unit && dx < 13 * unit {
            return 2;
        }
        return -1;
    }
    if input.low > 0 {
        let (px, py) = input.low_point;
        if py > y as f32 {
            return -1;
        }
        if py + 8.0 * (unit as f32) < y as f32 {
            return -1;
        }
        let slot = ((x as f32 - px) / (10.0 * unit as f32) + 4.5).round_ties_even() as i32;
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
    let angle = slot as f32 * std::f32::consts::TAU * 0.2;
    let radius = f64::from(unit) * 18.3;
    let x = node.0 + ((radius * f64::from(angle.cos())).round_ties_even() as f32) * turn;
    let y = node.1 - ((radius * f64::from(angle.sin())).round_ties_even() as f32) * turn;
    (x, y)
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

/// The status text of a node already among the chosen ones.
pub const STATUS_CHOSEN: &str = "Already chosen.";
/// The status text of a node whose table prerequisite is missing.
pub const STATUS_REQUIREMENTS: &str = "Requirements not met.";
/// The status text of a node with no technique.
pub const STATUS_UNAVAILABLE: &str = "Unavailable.";

/// A node tooltip's text: its group and technique names, the status line
/// and the points the level costs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillNodeTip {
    pub group: &'static str,
    pub title: &'static str,
    pub status: Option<&'static str>,
    /// The points the node's level costs (`level × 5000`).
    pub cost: u32,
}

/// The tooltip text of a node code, or `None` for an empty node. The
/// technique table's empty names are empty nodes. `state` is the
/// requirement state; a chosen node shows "Already chosen.", a node asking
/// for requirements "Requirements not met." and an empty one
/// "Unavailable.".
pub fn skill_node_tip(index: i32) -> Option<SkillNodeTip> {
    let group = usize::try_from((index >> 8) & 0x7).ok()?.checked_sub(1)?;
    let node = usize::try_from(index & 0x7).ok()?.checked_sub(1)?;
    let group_name = *SKILL_NAMES.get(group)?;
    let title = *TECHNIQUE_NAMES.get(group)?.get(node)?;
    if title.is_empty() {
        return None;
    }
    Some(SkillNodeTip {
        group: group_name,
        title,
        status: None,
        cost: skill_level_cost(node as u32 + 1),
    })
}

/// The status line a node's cached requirement state shows.
pub fn skill_node_status(requirements: u8) -> Option<&'static str> {
    match requirements {
        NODE_AVAILABLE => Some(STATUS_CHOSEN),
        NODE_REQUIREMENTS_NOT_MET => Some(STATUS_REQUIREMENTS),
        NODE_EMPTY => Some(STATUS_UNAVAILABLE),
        _ => None,
    }
}

/// The requirement state of every node in a skill group.
///
/// The returned bytes are indexed by the node's 0-based slot:
/// [`NODE_AVAILABLE`] for a chosen node, [`NODE_REQUIREMENTS_NOT_MET`] when
/// one of its table prerequisites is not among the first `level` choices,
/// [`NODE_EMPTY`] when the slot has no node, else 0. `skill` is 0-based.
///
/// The table is the character record's; this only orders it as the screen
/// does: the five chosen nodes first, then each slot's two prerequisites.
pub fn skill_requirements(skill: usize, skills: &[Skill; SKILLS]) -> [u8; NODES_PER_GROUP] {
    let mut state = [0u8; NODES_PER_GROUP];
    let Some(entry) = skills.get(skill) else {
        return state;
    };
    for &node in entry.techniques.iter().take(5) {
        if node != 0 && usize::from(node) <= NODES_PER_GROUP {
            state[usize::from(node) - 1] = NODE_AVAILABLE;
        }
    }
    let level = usize::try_from(entry.levels()).unwrap_or(usize::MAX);
    for (node, slot) in state.iter_mut().enumerate() {
        let prerequisites = slot_prerequisites(skill, node);
        if prerequisites[0] == 0 {
            *slot = NODE_EMPTY;
        }
        for prerequisite in prerequisites {
            if prerequisite != 0 && *slot == 0 {
                let chosen = entry
                    .techniques
                    .iter()
                    .take(level)
                    .take(entry.techniques.len())
                    .any(|&chosen| chosen == prerequisite);
                if !chosen {
                    *slot = NODE_REQUIREMENTS_NOT_MET;
                }
            }
        }
    }
    state
}

/// A node's own state for the tooltip and the selection rules.
///
/// `index` is the screen's node code: the group in bits 8-10 (1-based) and
/// the node in bits 0-2 (1-based). The result is [`NODE_UNAVAILABLE`],
/// [`NODE_AVAILABLE`] or [`NODE_REQUIREMENTS_NOT_MET`]. `can_learn` is the
/// screen's own "a level is ready" flag. A node below the group's level is
/// always available; the next one is available while its level's technique
/// is chosen and the level is finished, or when a level is ready and no
/// technique has been chosen there yet.
pub fn skill_node_state(skill: &Skill, index: i32, can_learn: bool) -> u8 {
    let group = (index >> 8) & 0xff;
    let node = index & 7;
    if group == 0 || node == 0 {
        return NODE_UNAVAILABLE;
    }
    let slot = usize::try_from(node).unwrap_or(0);
    let level = usize::try_from(skill.levels()).unwrap_or(usize::MAX);
    if level >= slot {
        return NODE_AVAILABLE;
    }
    if level + 1 != slot {
        return NODE_UNAVAILABLE;
    }
    let chosen = skill.techniques.get(slot - 1).copied().unwrap_or_default();
    if chosen == 0 {
        if can_learn {
            NODE_REQUIREMENTS_NOT_MET
        } else {
            NODE_UNAVAILABLE
        }
    } else {
        let spent = (level as u16).wrapping_mul(POINTS_PER_TECHNIQUE);
        if skill.points.wrapping_sub(spent) <= 10 {
            NODE_REQUIREMENTS_NOT_MET
        } else {
            NODE_AVAILABLE
        }
    }
}

/// Whether the screen shows a level ready to spend. The rule is the
/// character record's [`exa_character::record::CharacterData::skill_ready`].
pub fn skill_can_learn(data: &exa_character::record::CharacterData) -> bool {
    data.skill_ready()
}

/// The experience a skill level costs: one [`POINTS_PER_TECHNIQUE`] a level.
pub fn skill_level_cost(level: u32) -> u32 {
    level.saturating_mul(u32::from(POINTS_PER_TECHNIQUE))
}

/// The points still needed for a skill's next level.
pub fn skill_cost_remaining(skill: &Skill) -> u32 {
    let level = skill.levels();
    skill_level_cost(level + 1).saturating_sub(u32::from(skill.points))
}

/// A skill's progress within its current level and the level's cost.
pub fn skill_level_progress(skill: &Skill) -> (u16, u16) {
    (skill.points % POINTS_PER_TECHNIQUE, POINTS_PER_TECHNIQUE)
}

/// Unlearns the node of a group and level, with the record's dependent and
/// point rules. `skill` and `level` are 1-based, as the screen's node code.
/// Returns the first technique removed, if the record had one.
pub fn unlearn_skill_node(
    data: &mut exa_character::record::CharacterData,
    skill: usize,
    level: u32,
) -> Option<u8> {
    data.unlearn_level(skill, level)
}

/// Chooses a node after its requirements were met. The record removes a
/// technique that its points do not fund first, then writes the node.
pub fn choose_skill_node(
    data: &mut exa_character::record::CharacterData,
    skill: usize,
    level: u32,
) -> bool {
    data.choose_technique(skill, level)
}

/// The leftover experience after a share was distributed into the shown
/// skill points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillExperience {
    /// The shown points of the six groups after the pass.
    pub shown: [u16; SKILLS],
    /// What did not fit and stays for the next pass.
    pub leftover: u32,
}

/// Distributes experience into the six groups' shown points.
///
/// `data.record.skills[*].points` are the target points. A partial group
/// takes the next share first (`remainder × weight / 4`), then complete
/// levels fill in at `weight × 5000 / 4` each, and what is left goes to the
/// first unfunded choice or the first group below the cap. The weight is
/// four plus the levels already shown. This follows the screen's arithmetic;
/// the choice of the final group is a guess, because the experience system's
/// selector is outside the evidence. Points are stored in 16-bit fields and
/// wrap as the original's do.
pub fn distribute_skill_experience(
    data: &exa_character::record::CharacterData,
    shown: &mut [u16; SKILLS],
    gained: u32,
) -> SkillExperience {
    let skills = &data.record.skills;
    let mut weight = 4i64;
    let mut partial: Option<(usize, i64)> = None;
    for (group, skill) in skills.iter().enumerate() {
        let levels = i64::from(skill.points / POINTS_PER_TECHNIQUE);
        weight += levels;
        let remainder = i64::from(skill.points % POINTS_PER_TECHNIQUE);
        if remainder != 0 {
            partial = Some((group, i64::from(POINTS_PER_TECHNIQUE) - remainder));
        }
    }
    let mut left = i64::from(gained);
    if let Some((group, remainder)) = partial {
        let cost = remainder * weight / 4;
        if left < cost {
            let share = left * 4 / weight;
            shown[group] = (i64::from(skills[group].points) + share) as u16;
            left = 0;
        } else {
            shown[group] = (i64::from(skills[group].points) + remainder) as u16;
            left -= cost;
            weight += 1;
        }
    }
    for (group, skill) in skills.iter().enumerate() {
        let mut levels = i64::from(skill.points / POINTS_PER_TECHNIQUE);
        if partial.is_some_and(|(at, _)| at == group) {
            levels += 1;
        }
        let target = i64::from(shown[group] / POINTS_PER_TECHNIQUE) - 1;
        let mut at = levels;
        while at <= target {
            left -= weight * i64::from(POINTS_PER_TECHNIQUE) / 4;
            weight += 1;
            at += 1;
        }
    }
    if left > 0 {
        // Guess: the record's surplus choice is the group the leftover
        // belongs to; with none, the first group below the cap.
        let target = data.surplus_chosen_level().or_else(|| {
            skills
                .iter()
                .position(|skill| skill.levels() < 8)
                .map(|group| {
                    (
                        group + 1,
                        u32::from(shown[group] / POINTS_PER_TECHNIQUE) + 1,
                    )
                })
        });
        if let Some((group, level)) = target {
            let group = group.saturating_sub(1);
            if group < SKILLS {
                let cost = weight * i64::from(POINTS_PER_TECHNIQUE) / 4;
                if left < cost {
                    let share = left * 4 / weight;
                    let base = (level.saturating_sub(1) as i64) * i64::from(POINTS_PER_TECHNIQUE);
                    shown[group] = (base + share) as u16;
                    left = 0;
                } else {
                    shown[group] = ((level as i64) * i64::from(POINTS_PER_TECHNIQUE)) as u16;
                    left -= cost;
                }
            }
        }
    }
    SkillExperience {
        shown: *shown,
        leftover: u32::try_from(left.max(0)).unwrap_or(u32::MAX),
    }
}

/// The skill screen's view state: the ring's bounds and zoom, the selected
/// node and the ready-level flag.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkillScreen {
    pub bounds: SkillBounds,
    pub unit: i32,
    /// The number of groups on the ring (six in the retail tree).
    pub count: i32,
    /// The compact character-creation layout (a smaller ring radius).
    pub compact: bool,
    pub zoom: f32,
    /// The ring radius in units (64, or 58 compact).
    pub radius: i32,
    /// The 1-based selected node code; 0 for none.
    pub selected: i32,
    /// The 1-based group whose detail is open; 0 for the ring.
    pub page: i32,
    /// A node waiting for its choice to be confirmed.
    pub pending: i32,
    pub requirements: [u8; NODES_PER_GROUP],
    /// Bit 1 is drawn, bit 2 is fading in.
    pub state: u8,
    /// The reveal deadline (bit 1).
    pub show_deadline: i32,
    /// The hide deadline (bit 2).
    pub hide_deadline: i32,
    /// The forget dialog's close clock.
    pub forget_deadline: i32,
}

impl SkillScreen {
    /// A screen at the original's constructor defaults: six groups, full
    /// zoom, a 64-unit ring (58 compact), bit 1 set.
    pub fn open(bounds: SkillBounds, unit: i32, compact: bool) -> Self {
        Self {
            bounds,
            unit,
            count: 6,
            compact,
            zoom: 1.0,
            radius: if compact { 58 } else { 64 },
            selected: 0,
            page: 0,
            pending: 0,
            requirements: [0; NODES_PER_GROUP],
            state: 1,
            show_deadline: 0,
            hide_deadline: 0,
            forget_deadline: 0,
        }
    }

    /// Starts the reveal: bit 1 and the deadline `delay` ms on.
    pub fn reveal(&mut self, clock: i32, delay: i32) {
        self.state |= 1;
        self.show_deadline = clock.saturating_add(delay);
    }

    /// Starts the hide: bit 2 set and the 256 ms tail stamped.
    pub fn begin_hide(&mut self, clock: i32) {
        self.state |= 2;
        self.hide_deadline = clock.saturating_add(256);
    }

    /// The fade the draw body computes: 0 when closed, 1 when not fading,
    /// the smoothstepped `1 − 0.002 × remaining` while fading in, then the
    /// bit-2 fraction over 256 ms.
    pub fn alpha(&mut self, clock: i32) -> f32 {
        if self.state == 4 {
            return 0.0;
        }
        let mut alpha = if self.state & 1 == 0 {
            1.0
        } else if clock < self.show_deadline {
            let t = (1.0 - f64::from(self.show_deadline - clock) * 0.002) as f32;
            t * (2.0 - t)
        } else {
            self.state &= !1;
            1.0
        };
        if self.state & 2 != 0 {
            if clock < self.hide_deadline {
                alpha *= (self.hide_deadline - clock) as f32 / 256.0;
            } else {
                self.state = 4;
                alpha = 0.0;
            }
        }
        alpha
    }

    /// Places a group's node on the ring: the first node on the top.
    pub fn group_position(&self, group: i32) -> (f32, f32) {
        let ring = SkillRing {
            count: self.count,
            offset_a: self.radius,
            offset_b: 1,
            turn: self.zoom,
            width: self.bounds.right - self.bounds.left,
            height: self.bounds.bottom - self.bounds.top,
            page: if self.page == 0 { 0 } else { 1 },
            unit: self.unit,
            scrolled: false,
        };
        skill_ring_position(group, &ring)
    }

    /// Selects a node code and returns its node state: an available node
    /// opens its group's detail; one asking for requirements stores the
    /// pending choice.
    pub fn select(&mut self, data: &exa_character::record::CharacterData, index: i32) -> u8 {
        self.selected = index.max(0);
        let group = usize::try_from((index >> 8) & 0x7).unwrap_or(0);
        let state = data
            .record
            .skills
            .get(group.saturating_sub(1))
            .map(|skill| skill_node_state(skill, index, data.skill_ready()))
            .unwrap_or(NODE_UNAVAILABLE);
        if state == NODE_AVAILABLE {
            self.page = ((index >> 8) & 0x7).max(1);
            self.requirements = skill_requirements(group.saturating_sub(1), &data.record.skills);
            self.pending = 0;
        } else if state == NODE_REQUIREMENTS_NOT_MET {
            self.pending = index;
            self.requirements = skill_requirements(group.saturating_sub(1), &data.record.skills);
        } else {
            self.pending = 0;
        }
        state
    }

    /// Confirms the pending node, spending its level. Returns whether a
    /// technique was written.
    pub fn confirm(&mut self, data: &mut exa_character::record::CharacterData) -> bool {
        if self.pending <= 0 {
            return false;
        }
        let group = usize::try_from((self.pending >> 8) & 0x7).unwrap_or(0);
        let level = usize::try_from(self.pending & 0x7).unwrap_or(0);
        self.pending = 0;
        if group == 0 || level == 0 {
            return false;
        }
        choose_skill_node(data, group, level as u32)
    }

    /// Forgets the selected node's technique, with its dependents.
    pub fn forget(&mut self, data: &mut exa_character::record::CharacterData) -> Option<u8> {
        let group = usize::try_from((self.selected >> 8) & 0x7).unwrap_or(0);
        let level = usize::try_from(self.selected & 0x7).unwrap_or(0);
        if group == 0 || level == 0 {
            return None;
        }
        unlearn_skill_node(data, group, level as u32)
    }
}

/// The screen's icons are one atlas cell each. This is the normalised atlas
/// point a 1-based icon draws from: the cell's column and row in the grid.
///
/// `cell` is the cell size in pixels, `columns` and `rows` the atlas size in
/// pixels. The grid step is `columns / cell`, truncated, as the original's.
pub fn skill_icon_cell(icon: i32, cell: i32, columns: i32, rows: i32) -> (f32, f32) {
    if icon < 1 || cell == 0 || columns <= 0 || rows <= 0 {
        return (0.0, 0.0);
    }
    let step = (columns / cell) as f32;
    if step == 0.0 {
        return (0.0, 0.0);
    }
    let index = icon - 1;
    let column =
        (index as f32 - (index as f32 / step).trunc() * step) * (cell as f32 / columns as f32);
    let row =
        (index as f32 / step).trunc() * (cell as f32 / rows as f32) + cell as f32 / rows as f32;
    (column, row)
}

/// One icon of a group's ring: its centre and the 1-based node it shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkillGroupIcon {
    pub node: i32,
    pub x: f32,
    pub y: f32,
    pub chosen: bool,
    pub selected: bool,
}

/// Places the icons of one group's ring.
///
/// `level` is the group's shown points over 5000, `chosen` its eight node
/// bytes, `selected` the hovered/current node code and `can_learn` the
/// screen's ready flag. The icons sit on a pentagon of radius
/// `18.3 × unit × zoom` around the group's centre, four units left and up;
/// a node below the level draws whether or not the record holds one, the
/// next draws only while a level is ready.
#[allow(clippy::too_many_arguments)]
pub fn skill_group_icons(
    group: i32,
    x: f32,
    y: f32,
    unit: i32,
    zoom: f32,
    level: i32,
    chosen: &[u8; NODES_PER_GROUP],
    selected: i32,
    can_learn: bool,
) -> Vec<SkillGroupIcon> {
    let mut count = level.max(0);
    let partial = chosen
        .get(count as usize)
        .copied()
        .unwrap_or_default()
        .ne(&0);
    if partial || (can_learn && count < 5) {
        count += 1;
    }
    let mut icons = Vec::new();
    for node in 0..count {
        let angle = node as f32 * std::f32::consts::TAU * 0.2;
        let radius = f64::from(unit) * 18.3;
        let cx = ((radius * f64::from(angle.cos())).round_ties_even() as f32) * zoom
            + (x - 4.0 * unit as f32);
        let cy = (y - 4.0 * unit as f32)
            - ((radius * f64::from(angle.sin())).round_ties_even() as f32) * zoom;
        let code = (group << 8) | (node + 1);
        icons.push(SkillGroupIcon {
            node: node + 1,
            x: cx,
            y: cy,
            chosen: chosen.get(node as usize).copied().unwrap_or_default() != 0,
            selected: selected == code,
        });
    }
    icons
}

/// The four corners of one orbit mark around a group.
///
/// The marks are the five spokes from `14 × unit` to `20.3 × unit` around
/// the group's ring, at the same angles as its icons.
pub fn skill_orbit_mark(slot: usize, x: f32, y: f32, unit: i32, zoom: f32) -> [[f32; 2]; 4] {
    let angle = slot as f32 * std::f32::consts::TAU * 0.2;
    let (sin, cos) = (angle.sin() * zoom, angle.cos() * zoom);
    let inner = 14.0 * unit as f32;
    let outer = 20.3 * unit as f32;
    // The original's spoke is a quad whose axis is the radius; this is the
    // guessed half width, from the 12 × 14 box of its four corners.
    let half = 6.0 * unit as f32;
    let point =
        |radius: f32, side: f32| [x + radius * cos - side * sin, y - radius * sin - side * cos];
    [
        point(inner, half),
        point(outer, half),
        point(outer, -half),
        point(inner, -half),
    ]
}

/// The selected-node tooltip's slide.
///
/// The original slides the tooltip in over 200 ms after the selection
/// changed, and removes it when the selection is cleared. `since` is the
/// time since the selection stamp; returns the opacity, or `None` when the
/// tooltip is gone.
pub fn selection_tip_slide(selected: i32, since: i32) -> Option<f32> {
    if since < 200 {
        let t = since as f32 * 0.005;
        Some(if selected < 1 { 1.0 - t } else { t })
    } else if selected < 1 {
        None
    } else {
        Some(1.0)
    }
}

/// The forget dialog's or a tooltip's fade.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkillPopupFade {
    pub flags: u8,
    pub alpha: f32,
    /// The alpha for the second layer (the dialog's arrow or the tooltip's
    /// icons).
    pub secondary: f32,
    pub closed: bool,
}

/// The forget dialog's fade (0.5% a millisecond in, 0.333% out, squared).
pub fn forget_dialog_fade(flags: u8, deadline: i32, clock: i32) -> SkillPopupFade {
    skill_popup_fade(flags, deadline, clock, 0.005, 0.003_333_333_5, true)
}

/// A node tooltip's fade (the same curve, its second layer squared again).
pub fn node_tooltip_fade(flags: u8, deadline: i32, clock: i32) -> SkillPopupFade {
    skill_popup_fade(flags, deadline, clock, 0.005, 0.003_333_333_5, false)
}

fn skill_popup_fade(
    flags: u8,
    deadline: i32,
    clock: i32,
    opening: f64,
    closing: f64,
    dialog: bool,
) -> SkillPopupFade {
    if flags & 1 == 0 {
        if flags & 2 == 0 {
            return SkillPopupFade {
                flags,
                alpha: 1.0,
                secondary: 1.0,
                closed: false,
            };
        }
        if deadline <= clock {
            return SkillPopupFade {
                flags: flags & !3,
                alpha: 0.0,
                secondary: 0.0,
                closed: true,
            };
        }
        let t = f64::from(deadline - clock) * opening;
        let alpha = (t * t) as f32;
        return SkillPopupFade {
            flags,
            alpha,
            // Guess: the dialog's second layer is fully in while it opens;
            // the node tooltip's icon strip starts hidden.
            secondary: if dialog { 1.0 } else { 0.0 },
            closed: false,
        };
    }
    if clock < deadline {
        let t = 1.0 - f64::from(deadline - clock) * closing;
        let a = (t * (2.0 - t)) as f32;
        return SkillPopupFade {
            flags,
            alpha: a,
            secondary: a * (2.0 - a),
            closed: false,
        };
    }
    SkillPopupFade {
        flags: flags & !1,
        alpha: 1.0,
        secondary: 1.0,
        closed: false,
    }
}

/// Clamps a tooltip's left edge into the window.
///
/// `x` is the untransformed centre, `window` the window width, `offsets`
/// the form and parent origins and `radius` the tooltip's half width. The
/// original moves the tip left by the amount it overflows the right edge.
pub fn clamp_tooltip(x: f32, offsets: &[i32], radius: f32, window: i32) -> f32 {
    let mut left = x;
    for offset in offsets {
        left += *offset as f32;
    }
    let overflow = (left + radius) - window as f32;
    if overflow > 0.0 { x - overflow } else { x }
}

/// A tooltip's box: centre and half extent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkillNodeTipPlace {
    pub x: f32,
    pub y: f32,
    pub half_width: f32,
    pub half_height: f32,
}

/// Places the selected node's tooltip.
///
/// The tip is `82 × unit` wide and `14 × unit` tall, its centre one unit
/// above the node, clamped to the window. The node icon is `5 × unit` above
/// the centre and the title two units below the icon.
pub fn selected_tooltip(
    x: f32,
    y: f32,
    unit: i32,
    window: i32,
    offsets: &[i32],
) -> SkillNodeTipPlace {
    let half_width = 41.0 * unit as f32;
    let half_height = 14.0 * unit as f32;
    SkillNodeTipPlace {
        x: clamp_tooltip(x, offsets, half_width, window),
        y: y + unit as f32,
        half_width,
        half_height,
    }
}

/// One icon of the node tooltip's seven-slot strip.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkillTipIcon {
    pub node: i32,
    pub x: f32,
    pub y: f32,
    pub dimmed: bool,
}

/// Places the group tooltip and its seven node icons.
///
/// The box is `82 × unit` wide and `28 × unit` tall; the icons stand in a
/// row `10 × unit` apart, centred, four units left of the box's centre, and
/// a node before the group's level is drawn at full opacity while the rest
/// are dimmed. `chosen` holds the eight chosen bytes and `current` the
/// hovered node code.
#[allow(clippy::too_many_arguments)]
pub fn group_tooltip(
    group: i32,
    x: f32,
    y: f32,
    unit: i32,
    window: i32,
    offsets: &[i32],
    chosen: &[u8; NODES_PER_GROUP],
    current: i32,
) -> (SkillNodeTipPlace, Vec<SkillTipIcon>) {
    let place = selected_tooltip(x, y, unit, window, offsets);
    let bottom = place.y - 14.0 * unit as f32;
    let mut icons = Vec::new();
    for slot in 0..7usize {
        let ix = (slot as f32 - 3.5) * 10.0 * unit as f32 + place.x - 4.0 * unit as f32;
        let iy = bottom + 2.0 * unit as f32;
        let code = (group << 8) | (slot as i32 + 1);
        let dimmed = chosen.get(slot).copied().unwrap_or_default() == 0 && current != code;
        icons.push(SkillTipIcon {
            node: slot as i32 + 1,
            x: ix,
            y: iy,
            dimmed,
        });
    }
    (place, icons)
}

/// A forget dialog's geometry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ForgetDialogPlace {
    pub x: f32,
    pub y: f32,
    pub half_width: f32,
    pub title_y: f32,
    pub yes_x: f32,
    pub no_x: f32,
    pub choice_y: f32,
    pub text_y: f32,
}

/// Places the forget dialog under the selected node.
///
/// The title sits five units under the node, the Yes and No choices one
/// title line lower, twelve units either side of the centre, and the
/// question seven units under the choices. The glow ellipse behind them is
/// `82 × 20` units.
pub fn forget_dialog_place(
    x: f32,
    y: f32,
    unit: i32,
    window: i32,
    offsets: &[i32],
    alpha: f32,
) -> ForgetDialogPlace {
    let half_width = 41.0 * unit as f32 * alpha;
    let cx = clamp_tooltip(x, offsets, half_width, window);
    let title_y = y + 2.0 * unit as f32 + 5.0 * unit as f32;
    let choice_y = title_y + 13.0 * unit as f32;
    ForgetDialogPlace {
        x: cx,
        y: y + 2.0 * unit as f32,
        half_width,
        title_y,
        yes_x: cx - 12.0 * unit as f32,
        no_x: cx + 6.0 * unit as f32,
        choice_y,
        text_y: choice_y - 7.0 * unit as f32,
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
    fn a_skill_ring_places_its_nodes() {
        let ring = SkillRing {
            count: 8,
            offset_a: 10,
            offset_b: 5,
            turn: 1.0,
            width: 200,
            height: 100,
            page: 0,
            unit: 4,
            scrolled: false,
        };
        assert_eq!(skill_ring_position(3, &ring), (285.0, -39.0));
        let scrolled = SkillRing {
            page: 1,
            scrolled: true,
            ..ring
        };
        assert_eq!(skill_ring_position(3, &scrolled), (281.0, -27.0));
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
            offset_a: 10,
            offset_b: 5,
            turn: 1.0,
            width: 200,
            height: 100,
            page: 0,
            unit: 4,
            scrolled: false,
        };
        assert_eq!(skill_satellite(3, 1, &ring), (358.0, 53.0));
        assert_eq!(skill_satellite(3, 3, &ring), (226.0, 10.0));
        let scrolled = SkillRing {
            page: 1,
            scrolled: true,
            ..ring
        };
        assert_eq!(skill_satellite(3, 1, &scrolled), (354.0, 65.0));
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
            flag_1ea: false,
            clock: 100,
            used: [1; 8],
            repage: 0,
            selected: 0,
            base: 7,
            delay: 250,
            deadline_160: 0,
            deadline_164: 0,
            deadline_15c: 0,
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
        assert_eq!(hidden.deadline_15c, 300);
        assert!(hidden.hint);
        assert_eq!(hidden.result, 0x10);

        let shown = handle_skill_input(SkillInputInput {
            index: 0x10000,
            mask: 0x2000,
            state: 4,
            ..input()
        });
        assert_eq!(shown.state, 1);
        assert_eq!(shown.deadline_160, 600);
        assert_eq!(shown.result, 0x10);

        let second = handle_skill_input(SkillInputInput {
            index: 0x10000,
            mask: 0x2000,
            state: 0,
            delay: 250,
            ..input()
        });
        assert_eq!(second.state, 2);
        assert_eq!(second.deadline_164, 350);

        let high = handle_skill_input(SkillInputInput {
            index: 5,
            mask: 0x2000,
            repage: 1,
            ..input()
        });
        assert_eq!(high.high, 5);
        assert!(high.hint);
        assert_eq!(high.hide, 1);
        assert_eq!(high.deadline_15c, 400);

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

        let mut free = input();
        free.used[5] = 0;
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
            flag_1ea: true,
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
        assert_eq!(adjacent.code, 0x0201);
        assert_eq!(adjacent.point, (28, 10));

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
        assert_eq!(scanned.code, 0x105);
        assert_eq!(scanned.point, (6, 17));

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

        let only = |code: i32| code == 0x105;
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
        assert_eq!(fallback.code, 0x105);
        assert_eq!(fallback.point, (6, 17));
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

        assert_eq!(
            hit_skill_point(SkillPointInput {
                point: (80, 50),
                ..point_input()
            }),
            1
        );
        assert_eq!(
            hit_skill_point(SkillPointInput {
                point: (120, 50),
                ..point_input()
            }),
            2
        );
        assert_eq!(
            hit_skill_point(SkillPointInput {
                point: (100, 50),
                ..point_input()
            }),
            -1
        );
        assert_eq!(
            hit_skill_point(SkillPointInput {
                point: (80, 70),
                ..point_input()
            }),
            -1
        );

        let low = SkillPointInput {
            high: 0,
            low: 1,
            low_point: (100.0, 40.0),
            ..point_input()
        };
        assert_eq!(
            hit_skill_point(SkillPointInput {
                point: (120, 40),
                ..low
            }),
            5
        );
        assert_eq!(
            hit_skill_point(SkillPointInput {
                point: (-70, 40),
                ..low
            }),
            -1
        );
        assert_eq!(
            hit_skill_point(SkillPointInput {
                point: (120, 80),
                ..low
            }),
            -1
        );
        assert_eq!(
            hit_skill_point(SkillPointInput {
                point: (-30, 40),
                ..low
            }),
            1
        );
        assert_eq!(
            hit_skill_point(SkillPointInput {
                point: (240, 40),
                ..low
            }),
            8
        );

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

    #[test]
    fn requirements_follow_the_chosen_nodes_and_the_table() {
        let mut tree = skills();
        tree[0] = Skill {
            points: 35_000,
            techniques: [1, 2, 3, 4, 5, 0],
        };
        let state = skill_requirements(0, &tree);
        // Nodes 1 to 5 are chosen, but only nodes with a table entry keep a
        // selectable state; the empty Close Combat slots read unavailable.
        assert_eq!(state, [3, 3, 3, 3, 3, 3, 3, 0]);
        // Dual Wield (slot 8) needs Remise or Riposte (slots 1 or 4); both
        // are chosen here, so no requirement is missing.
        assert_eq!(state[7], 0);

        let mut missing = skills();
        missing[0] = Skill {
            points: 35_000,
            techniques: [2, 3, 4, 5, 0, 0],
        };
        let state = skill_requirements(0, &missing);
        assert_eq!(state[7], NODE_REQUIREMENTS_NOT_MET);

        // Armour's second tier needs the first.
        let mut armour = skills();
        armour[2] = Skill {
            points: 10_000,
            techniques: [1, 0, 0, 0, 0, 0],
        };
        assert_eq!(skill_requirements(2, &armour)[1], 0);
        let mut armour_missing = skills();
        armour_missing[2] = Skill {
            points: 10_000,
            techniques: [0, 0, 0, 0, 0, 0],
        };
        assert_eq!(
            skill_requirements(2, &armour_missing)[1],
            NODE_REQUIREMENTS_NOT_MET
        );
    }

    #[test]
    fn a_nodes_state_reads_the_level_and_the_ready_flag() {
        let skill = Skill {
            points: 5_000,
            techniques: [1, 0, 0, 0, 0, 0],
        };
        assert_eq!(skill_node_state(&skill, 0x0101, false), NODE_AVAILABLE);
        assert_eq!(skill_node_state(&skill, 0x0102, false), NODE_UNAVAILABLE);
        assert_eq!(skill_node_state(&skill, 0x0103, true), NODE_UNAVAILABLE);
        // A ready level with no choice at the next level shows the
        // requirements-not-met state in the original (its "can learn"
        // branch).
        assert_eq!(
            skill_node_state(&skill, 0x0102, true),
            NODE_REQUIREMENTS_NOT_MET
        );
        assert_eq!(skill_node_state(&skill, 0x0001, true), NODE_UNAVAILABLE);
        assert_eq!(skill_node_state(&skill, 0x0100, true), NODE_UNAVAILABLE);
        // A chosen next technique with ten points left is not yet
        // available; one more point makes it so.
        let done = Skill {
            points: 5_010,
            techniques: [1, 2, 0, 0, 0, 0],
        };
        assert_eq!(
            skill_node_state(&done, 0x0102, false),
            NODE_REQUIREMENTS_NOT_MET
        );
        let next = Skill {
            points: 5_011,
            techniques: [1, 2, 0, 0, 0, 0],
        };
        assert_eq!(skill_node_state(&next, 0x0102, false), NODE_AVAILABLE);
    }

    #[test]
    fn the_ready_flag_stops_at_the_first_chosen_next_node_and_the_cap() {
        use exa_character::record::CharacterData;

        let mut tree = CharacterData::new();
        assert!(skill_can_learn(&tree));
        tree.record.skills[0] = Skill {
            points: 10_000,
            techniques: [1, 0, 0, 0, 0, 0],
        };
        assert!(skill_can_learn(&tree));
        tree.record.skills[0].techniques[1] = 2;
        assert!(skill_can_learn(&tree));
        tree.record.skills[0].techniques[2] = 3;
        tree.record.skills[0].points = 10_011;
        assert!(!skill_can_learn(&tree));

        let mut full = CharacterData::new();
        for (slot, skill) in full.record.skills.iter_mut().enumerate() {
            *skill = Skill {
                points: (slot as u16 + 2) * POINTS_PER_TECHNIQUE,
                techniques: [1, 2, 0, 0, 0, 0],
            };
        }
        assert!(!skill_can_learn(&full));
    }

    #[test]
    fn learning_and_forgetting_use_the_records_rules() {
        use exa_character::record::CharacterData;

        let mut data = CharacterData::new();
        data.record.skills[0] = Skill {
            points: 10_000,
            techniques: [1, 0, 0, 0, 0, 0],
        };
        assert!(choose_skill_node(&mut data, 1, 2));
        assert_eq!(data.record.skills[0].techniques, [1, 2, 0, 0, 0, 0]);
        assert_eq!(unlearn_skill_node(&mut data, 1, 1), Some(1));
        assert_eq!(data.record.skills[0].points, 5_000);
    }

    #[test]
    fn experience_fills_a_partial_group_then_a_level() {
        use exa_character::record::CharacterData;

        let mut data = CharacterData::new();
        data.record.skills[0] = Skill {
            points: 2_500,
            techniques: [0, 0, 0, 0, 0, 0],
        };
        let mut shown = [0u16; SKILLS];
        // Weight 4: the partial 2500 points cost 2500 experience, then the
        // leftover 2500 experience (a weight of 5) tops the next level up
        // by 2000 points.
        let first = distribute_skill_experience(&data, &mut shown, 5_000);
        assert_eq!(first.shown[0], 2_500 + 2_500 + 2_000);
        assert_eq!(first.leftover, 0);

        let mut shown = [0u16; SKILLS];
        let second = distribute_skill_experience(&data, &mut shown, 20_000);
        assert_eq!(second.shown[0], 10_000);
        assert_eq!(second.leftover, 11_250);

        let empty = CharacterData::new();
        let mut shown = [0u16; SKILLS];
        let third = distribute_skill_experience(&empty, &mut shown, 20_000);
        assert_eq!(third.shown[0], 5_000);
        assert_eq!(third.leftover, 15_000);
    }

    #[test]
    fn a_group_ring_places_its_icons_on_a_pentagon() {
        let chosen = [1, 2, 0, 0, 0, 0, 0, 0];
        let icons = skill_group_icons(1, 100.0, 100.0, 4, 1.0, 2, &chosen, 0x0102, false);
        assert_eq!(icons.len(), 2);
        assert_eq!(icons[0].node, 1);
        assert_eq!(icons[1].node, 2);
        assert!(icons[0].chosen && icons[1].chosen);
        // The first icon sits on the +x axis, the second 72° on.
        assert_eq!(icons[0].x, 84.0 + 73.0);
        assert_eq!(icons[0].y, 84.0);
        assert!(icons[1].x > 84.0 && icons[1].x < 157.0);
        assert!(icons[1].y < 84.0);

        // With a ready level and one chosen, the next icon appears for the
        // choice while an empty node is not shown.
        let ready = skill_group_icons(
            1,
            0.0,
            0.0,
            4,
            1.0,
            1,
            &[1, 0, 0, 0, 0, 0, 0, 0],
            0x0102,
            true,
        );
        assert_eq!(ready.len(), 2);
        assert!(ready[0].chosen);
        assert!(!ready[1].chosen);
        assert!(ready[1].selected);
    }

    #[test]
    fn an_orbit_mark_spans_its_two_radii() {
        let mark = skill_orbit_mark(0, 100.0, 100.0, 4, 1.0);
        // Slot 0 points down the +x axis: inner 56, outer 81.2, half 24.
        assert_eq!(mark[0], [156.0, 76.0]);
        assert!((mark[1][0] - 181.2).abs() < 1e-4);
        assert_eq!(mark[1][1], 76.0);
        assert!((mark[2][0] - 181.2).abs() < 1e-4);
        assert_eq!(mark[2][1], 124.0);
        assert_eq!(mark[3], [156.0, 124.0]);
    }

    #[test]
    fn a_skill_icon_cell_is_a_normalised_atlas_grid_point() {
        // A 4 by 4 grid of 64-pixel cells: icon 1 is the top-left cell's
        // lower-left normalised point, icon 20 the next row's first.
        let (u, v) = skill_icon_cell(1, 64, 256, 256);
        assert_eq!((u, v), (0.0, 0.25));
        let (u, v) = skill_icon_cell(5, 64, 256, 256);
        assert_eq!((u, v), (0.0, 0.5));
        let (u, v) = skill_icon_cell(16, 64, 256, 256);
        assert_eq!((u, v), (0.75, 1.0));
        assert_eq!(skill_icon_cell(0, 64, 256, 256), (0.0, 0.0));
    }

    #[test]
    fn the_popup_fades_hold_their_curve() {
        let plain = forget_dialog_fade(0, 1000, 500);
        assert_eq!(plain.alpha, 1.0);
        assert_eq!(plain.secondary, 1.0);
        assert!(!plain.closed);

        // Opening over the last 200 ms: 0.5% a millisecond, squared.
        let opening = forget_dialog_fade(2, 700, 600);
        assert!((opening.alpha - 0.25).abs() < 1e-6);
        assert_eq!(opening.secondary, 1.0);

        let closed = forget_dialog_fade(2, 500, 600);
        assert!(closed.closed);
        assert_eq!(closed.flags & 3, 0);

        // Closing: 0.333% a millisecond, smoothstepped, the secondary
        // smoothstepped again.
        let closing = forget_dialog_fade(1, 950, 900);
        let a = (1.0 - 50.0f64 * 0.003_333_333_5) as f32;
        assert!((closing.alpha - (a * (2.0 - a))).abs() < 1e-6);
        let b = closing.alpha;
        assert!((closing.secondary - (b * (2.0 - b))).abs() < 1e-6);

        let tooltip = node_tooltip_fade(2, 700, 600);
        assert!((tooltip.alpha - 0.25).abs() < 1e-6);
        assert_eq!(tooltip.secondary, 0.0);
    }

    #[test]
    fn a_selection_tip_slides_in_and_disappears() {
        assert_eq!(selection_tip_slide(5, 100), Some(0.5));
        assert_eq!(selection_tip_slide(0, 100), Some(0.5));
        assert_eq!(selection_tip_slide(5, 200), Some(1.0));
        assert_eq!(selection_tip_slide(0, 200), None);
    }

    #[test]
    fn tooltips_clamp_to_the_window_and_place_their_icons() {
        let place = selected_tooltip(300.0, 200.0, 4, 500, &[10, 20]);
        assert_eq!(place.half_width, 164.0);
        assert_eq!(place.half_height, 56.0);
        assert_eq!(place.y, 204.0);
        assert!(place.x + 164.0 <= 500.0);

        let (place, icons) = group_tooltip(1, 100.0, 100.0, 4, 1000, &[], &[1; 8], 0x0101);
        assert_eq!(place.y, 104.0);
        assert_eq!(icons.len(), 7);
        assert_eq!(icons[0].x, place.x - 156.0);
        assert_eq!(icons[6].x, place.x + 84.0);
        assert!(icons.iter().all(|icon| !icon.dimmed));
    }

    #[test]
    fn a_node_tooltip_names_its_group_technique_status_and_cost() {
        let tip = skill_node_tip(0x0104).unwrap();
        assert_eq!(tip.group, "Close Combat");
        assert_eq!(tip.title, "Riposte");
        assert_eq!(tip.cost, 20_000);
        assert_eq!(tip.status, None);
        assert_eq!(skill_node_status(NODE_AVAILABLE), Some(STATUS_CHOSEN));
        assert_eq!(
            skill_node_status(NODE_REQUIREMENTS_NOT_MET),
            Some(STATUS_REQUIREMENTS)
        );
        assert_eq!(skill_node_status(NODE_EMPTY), Some(STATUS_UNAVAILABLE));
        assert_eq!(skill_node_status(0), None);
        // Ranged Combat has no techniques and the seventh Armour slot none.
        assert_eq!(skill_node_tip(0x0201), None);
        assert_eq!(skill_node_tip(0x0307), None);
        assert_eq!(skill_node_tip(0x0100), None);
        assert_eq!(skill_node_tip(0x0801), None);
    }

    #[test]
    fn a_screen_opens_fades_and_selects_a_node() {
        use exa_character::record::CharacterData;

        let mut screen = SkillScreen::open(
            SkillBounds {
                left: 0,
                right: 400,
                top: 0,
                bottom: 300,
            },
            4,
            false,
        );
        assert_eq!((screen.count, screen.radius), (6, 64));
        screen.reveal(100, 256);
        assert_eq!(screen.show_deadline, 356);
        let opening = screen.alpha(100);
        assert!(opening > 0.73 && opening < 0.74);
        assert!((screen.alpha(356) - 1.0).abs() < 1e-6);
        // The reveal bit clears once its deadline passes.
        assert_eq!(screen.state, 0);
        screen.begin_hide(400);
        assert_eq!(screen.hide_deadline, 656);
        assert!((screen.alpha(400) - 1.0).abs() < 1e-6);
        assert_eq!(screen.alpha(656), 0.0);
        assert_eq!(screen.state, 4);

        let mut data = CharacterData::new();
        data.record.skills[0] = Skill {
            points: 10_000,
            techniques: [1, 2, 0, 0, 0, 0],
        };
        assert_eq!(screen.select(&data, 0x0102), NODE_AVAILABLE);
        assert_eq!(screen.page, 1);
        let mut data = CharacterData::new();
        data.record.skills[0] = Skill {
            points: 5_000,
            techniques: [1, 0, 0, 0, 0, 0],
        };
        data.record.skills[0].points = 5_011;
        assert_eq!(screen.select(&data, 0x0102), NODE_REQUIREMENTS_NOT_MET);
        assert_eq!(screen.pending, 0x0102);
        assert!(screen.confirm(&mut data));
        assert_eq!(data.record.skills[0].techniques[1], 2);
        screen.selected = 0x0101;
        assert_eq!(screen.forget(&mut data), Some(1));
        assert_eq!(data.record.skills[0].techniques, [2, 0, 0, 0, 0, 0]);
        assert_eq!(data.record.skills[0].points, 11);

        // Six groups put the first at −150° on the 64-unit circle.
        let position = screen.group_position(1);
        assert_eq!(position.0, -22.0);
        assert_eq!(position.1, 22.0);
    }

    #[test]
    fn a_level_costs_five_thousand_points() {
        let mut skill = Skill::default();
        assert_eq!(skill_level_cost(1), 5_000);
        assert_eq!(skill_cost_remaining(&skill), 5_000);
        skill.points = 2_500;
        assert_eq!(skill_cost_remaining(&skill), 2_500);
        assert_eq!(skill_level_progress(&skill), (2_500, 5_000));
        skill.points = 15_000;
        assert_eq!(skill_level_cost(3), 15_000);
        assert_eq!(skill_cost_remaining(&skill), 5_000);
        assert_eq!(skill_level_progress(&skill), (0, 5_000));
    }

    #[test]
    fn the_forget_dialog_places_its_choices() {
        let place = forget_dialog_place(100.0, 200.0, 4, 800, &[], 1.0);
        assert_eq!(place.x, 100.0);
        assert_eq!(place.y, 208.0);
        assert_eq!(place.half_width, 164.0);
        assert_eq!(place.title_y, 228.0);
        assert_eq!(place.choice_y, 280.0);
        assert_eq!(place.yes_x, 52.0);
        assert_eq!(place.no_x, 124.0);
        assert_eq!(place.text_y, 252.0);
    }
}
