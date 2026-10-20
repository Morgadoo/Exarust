//! The skill screen's node placement.
//!
//! A positive index names a row and a column in the screen's layout; a
//! non-positive index takes the scaled base or the centre of the bounds.
//! Evidence: the 186-byte `TSkillUI` method at `0x100224570`.

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
}
