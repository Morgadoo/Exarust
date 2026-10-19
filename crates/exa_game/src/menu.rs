//! The in-game menu command that begins an arena match.
//!
//! A dismissed child closes unless it is already closed. A pinned child
//! raises a notice and still closes its own children, first to last.
//! The capture release, the task's other children, detaching, redraw, the
//! call after the two lists, writing a save package, and the transition
//! are not part of this step.

use crate::career::Company;

/// The in-game menu page.
pub const IN_GAME_PAGE: u32 = 2;
/// The command that begins a match from that page.
pub const BEGIN_COMMAND: u32 = 4;
/// Session phase while a transition is starting.
pub const PHASE_TRANSITIONING: u32 = 0;
/// Session phase while play is underway.
pub const PHASE_PLAYING: u32 = 3;
/// Flag bits a child must all have before a match begin dismisses it.
pub const DISMISS_MARK: u32 = 0xce10_0000;

/// A child window on one of the two lists a match begin walks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Window {
    pub name: String,
    pub flags: u32,
    pub dismissed: bool,
    /// Already closed. Close returns without its callback or its children.
    pub closed: bool,
    /// Marked closed as the close starts, and a notice is raised.
    pub pinned: bool,
    pub on_close: bool,
    pub close_ran: bool,
    /// The argument recorded when close proceeds. Already closed stays unset.
    pub seen_arg: Option<u32>,
    pub children: Vec<Window>,
}

/// Which mode the menu command is running in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayKind {
    Practice = 2,
    Story = 4,
    Arena = 8,
}

/// Flags the arena begin command changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchGate {
    pub hub_busy: bool,
    pub click_hook: bool,
    /// A pointer cleared when the match is armed. Its role was not established.
    pub held: bool,
    pub phase: u32,
    /// A save was asked, and the low-health abort was not.
    pub save_unconditional: bool,
    pub first: Vec<Window>,
    pub first_count: i32,
    pub second: Vec<Window>,
    pub second_count: i32,
    /// Names dismissed by the begin, last child first, first list then second.
    pub dismissed: Vec<String>,
    /// Raised when a pinned child starts to close.
    pub notice: bool,
    /// Close callbacks, children before their parent.
    pub close_log: Vec<String>,
}

impl MatchGate {
    /// Play is underway, the hub is free, and a world click hook is set.
    pub fn playing() -> Self {
        Self {
            hub_busy: false,
            click_hook: true,
            held: true,
            phase: PHASE_PLAYING,
            save_unconditional: false,
            first: Vec::new(),
            first_count: 0,
            second: Vec::new(),
            second_count: 0,
            dismissed: Vec::new(),
            notice: false,
            close_log: Vec::new(),
        }
    }

    /// In-game command 4 in arena dismisses marked children, removes one
    /// unhealthy specialist, then arms the match. Any other page, command,
    /// or mode changes nothing.
    pub fn begin_match(
        &mut self,
        page: u32,
        command: u32,
        kind: PlayKind,
        company: &mut Company,
    ) -> bool {
        if page != IN_GAME_PAGE || command != BEGIN_COMMAND || kind != PlayKind::Arena {
            return false;
        }
        dismiss_open(self);
        let _ = company.leave_before_match();
        self.save_unconditional = true;
        self.hub_busy = true;
        self.click_hook = false;
        self.held = false;
        self.phase = PHASE_TRANSITIONING;
        true
    }
}

/// Visit `count` children from the last back to the first. A count of 0 or
/// less does nothing. A count past the list stops at the list.
fn dismiss_open(gate: &mut MatchGate) {
    let mut order = std::mem::take(&mut gate.dismissed);
    let mut log = std::mem::take(&mut gate.close_log);
    let mut notice = gate.notice;
    dismiss_list(
        &mut gate.first,
        gate.first_count,
        &mut order,
        &mut notice,
        &mut log,
    );
    dismiss_list(
        &mut gate.second,
        gate.second_count,
        &mut order,
        &mut notice,
        &mut log,
    );
    gate.dismissed = order;
    gate.close_log = log;
    gate.notice = notice;
}

fn dismiss_list(
    windows: &mut [Window],
    count: i32,
    order: &mut Vec<String>,
    notice: &mut bool,
    log: &mut Vec<String>,
) {
    if count <= 0 {
        return;
    }
    let end = (count as usize).min(windows.len());
    for window in windows[..end].iter_mut().rev() {
        if window.flags & DISMISS_MARK == DISMISS_MARK {
            window.dismissed = true;
            order.push(window.name.clone());
            close_window(window, 0, notice, log);
        }
    }
}

/// Close one window. An already closed window returns. A pinned window is
/// marked closed and raises the notice, then its children close from the
/// first to the last. The callback runs after those children. The capture
/// release, the task's other children, detaching, and redraw are not in.
fn close_window(window: &mut Window, arg: u32, notice: &mut bool, log: &mut Vec<String>) {
    if window.closed {
        return;
    }
    if window.pinned {
        window.closed = true;
        *notice = true;
    }
    for child in &mut window.children {
        if !child.closed {
            close_window(child, arg, notice, log);
        }
    }
    if window.on_close {
        window.close_ran = true;
        log.push(window.name.clone());
    }
    window.closed = true;
    window.seen_arg = Some(arg);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::career::{Company, Profession};

    fn hired_physician() -> Company {
        let mut company = Company::new("Ash");
        company.hire("Med", Profession::Physician).unwrap();
        company.roster[0].health = 699;
        company
    }

    #[test]
    fn an_arena_menu_begins_after_the_specialist_leaves() {
        let mut gate = MatchGate::playing();
        let mut empty = Company::new("Ash");
        assert!(gate.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut empty));
        assert!(empty.roster.is_empty());
        assert_eq!(empty.deaths, 0);
        assert!(gate.hub_busy);
        assert!(!gate.click_hook);
        assert!(!gate.held);
        assert!(gate.save_unconditional);
        assert_eq!(gate.phase, PHASE_TRANSITIONING);

        let mut gate = MatchGate::playing();
        let mut dying = hired_physician();
        assert!(gate.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut dying));
        assert!(dying.roster.is_empty());
        assert_eq!(dying.deaths, 1);
        assert!(gate.hub_busy);
        assert_eq!(gate.phase, PHASE_TRANSITIONING);

        let mut gate = MatchGate::playing();
        let mut story = hired_physician();
        assert!(!gate.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Story, &mut story));
        assert_eq!(story.roster.len(), 1);
        assert_eq!(story.roster[0].name, "Med");
        assert_eq!(story.deaths, 0);
        assert!(!gate.hub_busy);
        assert!(gate.click_hook);
        assert!(gate.held);
        assert!(!gate.save_unconditional);
        assert_eq!(gate.phase, PHASE_PLAYING);

        let mut practice = hired_physician();
        assert!(!gate.begin_match(
            IN_GAME_PAGE,
            BEGIN_COMMAND,
            PlayKind::Practice,
            &mut practice
        ));
        assert_eq!(practice.roster.len(), 1);
        assert_eq!(practice.deaths, 0);

        let mut other = hired_physician();
        assert!(!gate.begin_match(IN_GAME_PAGE, 1, PlayKind::Arena, &mut other));
        assert_eq!(other.roster.len(), 1);
        assert!(!gate.hub_busy);

        let mut elsewhere = hired_physician();
        assert!(!gate.begin_match(1, BEGIN_COMMAND, PlayKind::Arena, &mut elsewhere));
        assert_eq!(elsewhere.roster.len(), 1);
        assert_eq!(gate.phase, PHASE_PLAYING);
    }

    fn window(name: &str, flags: u32) -> Window {
        Window {
            name: name.to_string(),
            flags,
            dismissed: false,
            closed: false,
            pinned: false,
            on_close: false,
            close_ran: false,
            seen_arg: None,
            children: Vec::new(),
        }
    }

    #[test]
    fn a_match_begin_dismisses_marked_children_before_the_leave() {
        let mut gate = MatchGate::playing();
        gate.first = vec![
            window("Plain", 0),
            window("Shop", DISMISS_MARK),
            window("Board", DISMISS_MARK | 1),
        ];
        gate.first_count = 2;
        gate.second = vec![window("Tree", 0xffff_ffff), window("Bar", 0xce00_0000)];
        gate.second_count = 2;
        let mut dying = hired_physician();
        assert!(gate.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut dying));
        assert_eq!(gate.dismissed, vec!["Shop".to_string(), "Tree".to_string()]);
        assert!(!gate.first[0].dismissed);
        assert!(gate.first[1].dismissed);
        assert!(!gate.first[2].dismissed);
        assert!(gate.second[0].dismissed);
        assert!(!gate.second[1].dismissed);
        assert!(dying.roster.is_empty());
        assert_eq!(dying.deaths, 1);
        assert!(gate.hub_busy);

        let mut story = MatchGate::playing();
        story.first = vec![window("Shop", DISMISS_MARK)];
        story.first_count = 1;
        let mut physician = hired_physician();
        assert!(!story.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Story, &mut physician));
        assert!(story.dismissed.is_empty());
        assert!(!story.first[0].dismissed);
        assert_eq!(physician.roster.len(), 1);
        assert_eq!(physician.deaths, 0);

        let mut none = MatchGate::playing();
        none.first = vec![window("Shop", DISMISS_MARK)];
        none.first_count = 0;
        none.second = vec![window("Tree", DISMISS_MARK)];
        none.second_count = -1;
        let mut company = Company::new("Ash");
        assert!(none.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut company));
        assert!(none.dismissed.is_empty());
        assert!(!none.first[0].dismissed);
        assert!(!none.second[0].dismissed);
        assert!(none.hub_busy);
    }

    #[test]
    fn a_dismissed_child_closes_its_own_children() {
        let mut gate = MatchGate::playing();
        let mut shop = window("Shop", DISMISS_MARK);
        shop.pinned = true;
        shop.on_close = true;
        let mut first = window("A", 0);
        first.on_close = true;
        let mut second = window("B", 0);
        second.closed = true;
        second.on_close = true;
        shop.children = vec![first, second];
        gate.first = vec![shop];
        gate.first_count = 1;
        let mut dying = hired_physician();
        assert!(gate.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut dying));
        assert_eq!(gate.dismissed, vec!["Shop".to_string()]);
        assert!(gate.notice);
        assert_eq!(gate.close_log, vec!["A".to_string(), "Shop".to_string()]);
        assert!(gate.first[0].closed);
        assert_eq!(gate.first[0].seen_arg, Some(0));
        assert!(gate.first[0].children[0].close_ran);
        assert_eq!(gate.first[0].children[0].seen_arg, Some(0));
        assert!(gate.first[0].children[1].closed);
        assert!(!gate.first[0].children[1].close_ran);
        assert!(gate.first[0].children[1].seen_arg.is_none());
        assert!(dying.roster.is_empty());
        assert_eq!(dying.deaths, 1);

        let mut gate = MatchGate::playing();
        let mut board = window("Board", DISMISS_MARK);
        board.closed = true;
        board.on_close = true;
        let mut child = window("C", 0);
        child.on_close = true;
        board.children = vec![child];
        gate.first = vec![board];
        gate.first_count = 1;
        let mut company = Company::new("Ash");
        assert!(gate.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut company));
        assert_eq!(gate.dismissed, vec!["Board".to_string()]);
        assert!(gate.close_log.is_empty());
        assert!(!gate.notice);
        assert!(!gate.first[0].close_ran);
        assert!(!gate.first[0].children[0].closed);
        assert!(company.roster.is_empty());
    }
}
