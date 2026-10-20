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
}
