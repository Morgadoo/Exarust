//! The in-game menu command that begins an arena match.
//!
//! A dismissed child closes unless it is already closed. A pinned child
//! raises a notice and still closes its own children, first to last.
//! A capture is released first. The task's other children that name this
//! one close before its own, a hover clears, and a shown child hides.
//! Releasing a capture publishes the one underneath it. After the two
//! lists, argument 1 copies ten hand words when a focus is set or the
//! mask shares bit 0. The shape table, the virtual call, writing a save
//! package, and the transition are not part of this step.

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
/// Style bit set while this child holds the capture.
pub const CAPTURE_BIT: u32 = 0x20;
/// Bits a redraw ask sets. Each ask ORs them on.
pub const REDRAW_BITS: u32 = 0x3;
/// Bit on a capture code that passes the low byte as the shape.
pub const CURSOR_MARK: u32 = 0x8000;
/// Argument passed after the two window lists.
pub const FOLLOW_ARG: u8 = 1;
/// Ten words copied into a ready hand list.
pub const HAND_WORDS: [u32; 10] = [
    0x101, 0x2_0101, 0x102, 0x105, 0x1_0106, 0x107, 0x2_010b, 0, 0, 0,
];

/// One held capture. The code is what a later release publishes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capture {
    pub name: String,
    pub code: u32,
}

/// One runtime cursor-shape record. Shape 1 is index 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorShape {
    pub image: u64,
    pub hook: bool,
}

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
    /// The child this one names, when it sits on the task list.
    pub owner: Option<String>,
    /// Shown. Hiding a shown child asks for a redraw.
    pub visible: bool,
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
    /// Captures, oldest first. The last one is the top.
    pub captures: Vec<Capture>,
    /// Code published by the latest release. 0 when the list is empty.
    pub cursor_code: u32,
    /// Name published by the latest release.
    pub cursor_holder: Option<String>,
    /// Shape argument. 0 marks the default unless the code has bit 0x8000.
    pub cursor_shape: u8,
    pub cursor_default: bool,
    /// Shape records. Index 0 is shape 1.
    pub shapes: Vec<CursorShape>,
    pub default_image: u64,
    pub current_image: u64,
    pub shape_mark: bool,
    pub shape_stamp: u32,
    pub shape_clock: u32,
    /// Times a different non-zero image was shown.
    pub shown: u32,
    pub shape_hooks: u32,
    /// Times a focus was invoked. The method body is not in.
    pub focus_calls: u32,
    /// Session mark cleared to 0 when it was not already 0.
    pub session_mark: u8,
    /// A focus object is set, so the follow runs even with a mask of 0.
    pub focus: bool,
    /// Mask tested against argument 1.
    pub list_mask: u8,
    pub mode_word: u32,
    pub clock: u32,
    pub stamp: u32,
    pub hands_ready: bool,
    pub hands: [u32; 10],
    pub hands_set: bool,
    pub latched: bool,
    /// Cleared on the focus path. Starts set so the clear is visible.
    pub pending: bool,
    /// The task's active form, when one is set.
    pub active: Option<String>,
    /// The task's hover, when one is set.
    pub hover: Option<String>,
    /// The task's other children.
    pub siblings: Vec<Window>,
    /// The redraw word. Each ask ORs on bits 0 and 1.
    pub redraw: u32,
    /// How many times a redraw was asked.
    pub redraws: u32,
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
            captures: Vec::new(),
            cursor_code: 0,
            cursor_holder: None,
            cursor_shape: 0,
            cursor_default: false,
            shapes: Vec::new(),
            default_image: 0,
            current_image: 0,
            shape_mark: false,
            shape_stamp: 0,
            shape_clock: 0,
            shown: 0,
            shape_hooks: 0,
            focus_calls: 0,
            session_mark: 0,
            focus: false,
            list_mask: 0,
            mode_word: 0,
            clock: 0,
            stamp: 0,
            hands_ready: false,
            hands: [0; 10],
            hands_set: false,
            latched: false,
            pending: true,
            active: None,
            hover: None,
            siblings: Vec::new(),
            redraw: 0,
            redraws: 0,
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
        follow_lists(self);
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
struct CloseState {
    order: Vec<String>,
    log: Vec<String>,
    notice: bool,
    captures: Vec<Capture>,
    cursor_code: u32,
    cursor_holder: Option<String>,
    cursor_shape: u8,
    cursor_default: bool,
    shapes: Vec<CursorShape>,
    default_image: u64,
    current_image: u64,
    shape_mark: bool,
    shape_stamp: u32,
    shape_clock: u32,
    shown: u32,
    shape_hooks: u32,
    active: Option<String>,
    hover: Option<String>,
    siblings: Vec<Window>,
    redraw: u32,
    redraws: u32,
}

fn dismiss_open(gate: &mut MatchGate) {
    let mut state = CloseState {
        order: std::mem::take(&mut gate.dismissed),
        log: std::mem::take(&mut gate.close_log),
        notice: gate.notice,
        captures: std::mem::take(&mut gate.captures),
        cursor_code: gate.cursor_code,
        cursor_holder: gate.cursor_holder.clone(),
        cursor_shape: gate.cursor_shape,
        cursor_default: gate.cursor_default,
        shapes: gate.shapes.clone(),
        default_image: gate.default_image,
        current_image: gate.current_image,
        shape_mark: gate.shape_mark,
        shape_stamp: gate.shape_stamp,
        shape_clock: gate.shape_clock,
        shown: gate.shown,
        shape_hooks: gate.shape_hooks,
        active: gate.active.clone(),
        hover: gate.hover.clone(),
        siblings: std::mem::take(&mut gate.siblings),
        redraw: gate.redraw,
        redraws: gate.redraws,
    };
    dismiss_list(&mut gate.first, gate.first_count, &mut state);
    dismiss_list(&mut gate.second, gate.second_count, &mut state);
    gate.dismissed = state.order;
    gate.close_log = state.log;
    gate.notice = state.notice;
    gate.captures = state.captures;
    gate.cursor_code = state.cursor_code;
    gate.cursor_holder = state.cursor_holder;
    gate.cursor_shape = state.cursor_shape;
    gate.cursor_default = state.cursor_default;
    gate.shapes = state.shapes;
    gate.default_image = state.default_image;
    gate.current_image = state.current_image;
    gate.shape_mark = state.shape_mark;
    gate.shape_stamp = state.shape_stamp;
    gate.shape_clock = state.shape_clock;
    gate.shown = state.shown;
    gate.shape_hooks = state.shape_hooks;
    gate.active = state.active;
    gate.hover = state.hover;
    gate.siblings = state.siblings;
    gate.redraw = state.redraw;
    gate.redraws = state.redraws;
}

fn dismiss_list(windows: &mut [Window], count: i32, state: &mut CloseState) {
    if count <= 0 {
        return;
    }
    let end = (count as usize).min(windows.len());
    for window in windows[..end].iter_mut().rev() {
        if window.flags & DISMISS_MARK == DISMISS_MARK {
            window.dismissed = true;
            state.order.push(window.name.clone());
            close_window(window, 0, state);
        }
    }
}

/// Close one window. A capture is released before the already-closed return.
/// A pinned window is marked closed and raises the notice. The active form
/// stops after its callback. Any other form clears a hover, closes the task's
/// children that name it, then its own children. A shown form hides. The
/// callback runs, and a redraw is asked. Cursor restore is not in.
fn close_window(window: &mut Window, arg: u32, state: &mut CloseState) {
    release_capture(window, state);
    if window.closed {
        return;
    }
    if window.pinned {
        window.closed = true;
        state.notice = true;
    }
    if state.active.as_deref() == Some(window.name.as_str()) {
        run_close(window, state);
        state.active = None;
        window.closed = true;
        window.seen_arg = Some(arg);
        return;
    }
    if state.hover.as_deref() == Some(window.name.as_str()) {
        state.hover = None;
    }
    close_named(window.name.clone(), arg, state);
    for child in &mut window.children {
        if !child.closed {
            close_window(child, arg, state);
        }
    }
    hide_window(window, state);
    run_close(window, state);
    window.closed = true;
    window.seen_arg = Some(arg);
    ask_redraw(state);
}

fn release_capture(window: &mut Window, state: &mut CloseState) {
    if window.flags & CAPTURE_BIT == 0 {
        return;
    }
    let Some(index) = state
        .captures
        .iter()
        .rposition(|capture| capture.name == window.name)
    else {
        return;
    };
    state.captures.remove(index);
    window.flags &= !CAPTURE_BIT;
    let Some(top) = state.captures.last() else {
        state.cursor_code = 0;
        state.cursor_holder = None;
        return;
    };
    state.cursor_code = top.code;
    state.cursor_holder = Some(top.name.clone());
    let shape = if top.code & CURSOR_MARK == 0 {
        0
    } else {
        top.code as u8
    };
    state.cursor_shape = shape;
    if shape == 0 {
        state.cursor_default = true;
    }
    apply_shape(state, shape);
}

fn apply_shape(state: &mut CloseState, shape: u8) {
    if shape == 0 {
        let image = state.default_image;
        show_image(state, image);
        state.shape_mark = true;
        state.shape_stamp = state.shape_clock;
        return;
    }
    let index = (shape as usize) - 1;
    if index >= state.shapes.len() {
        return;
    }
    let slot = state.shapes[index].clone();
    show_image(state, slot.image);
    if slot.hook {
        state.shape_hooks = state.shape_hooks.saturating_add(1);
    }
}

fn show_image(state: &mut CloseState, image: u64) {
    if image == state.current_image {
        return;
    }
    if image != 0 {
        state.shown = state.shown.saturating_add(1);
    }
    state.current_image = image;
}

/// The call after the two lists. Argument 1 runs for a focus, or when the
/// mask shares bit 0. A focus is counted before the hands. The method body
/// is not in.
fn follow_lists(gate: &mut MatchGate) {
    let run = gate.focus || gate.list_mask & FOLLOW_ARG != 0;
    if !run {
        return;
    }
    if gate.focus {
        gate.focus_calls = gate.focus_calls.saturating_add(1);
        gate.latched = true;
        gate.pending = false;
    }
    gate.mode_word = (gate.mode_word & 2) | 1;
    gate.stamp = gate.clock;
    if gate.hands_ready {
        gate.hands = HAND_WORDS;
        gate.hands_set = true;
    }
    if gate.focus && gate.session_mark != 0 {
        gate.session_mark = 0;
    }
}

fn close_named(owner: String, arg: u32, state: &mut CloseState) {
    let count = state.siblings.len();
    for index in 0..count {
        let named = !state.siblings[index].closed
            && state.siblings[index].owner.as_deref() == Some(owner.as_str());
        if !named {
            continue;
        }
        let mut sibling = std::mem::replace(&mut state.siblings[index], blank_window());
        close_window(&mut sibling, arg, state);
        state.siblings[index] = sibling;
    }
}

fn hide_window(window: &mut Window, state: &mut CloseState) {
    if !window.visible {
        return;
    }
    release_capture(window, state);
    window.visible = false;
    ask_redraw(state);
}

fn run_close(window: &mut Window, state: &mut CloseState) {
    if window.on_close {
        window.close_ran = true;
        state.log.push(window.name.clone());
    }
}

fn ask_redraw(state: &mut CloseState) {
    state.redraw |= REDRAW_BITS;
    state.redraws = state.redraws.saturating_add(1);
}

fn blank_window() -> Window {
    Window {
        name: String::new(),
        flags: 0,
        dismissed: false,
        closed: true,
        pinned: false,
        on_close: false,
        close_ran: false,
        seen_arg: None,
        children: Vec::new(),
        owner: None,
        visible: false,
    }
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
            owner: None,
            visible: false,
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

    #[test]
    fn a_close_releases_a_capture_and_redraws() {
        let mut gate = MatchGate::playing();
        let mut shop = window("Shop", DISMISS_MARK | CAPTURE_BIT);
        shop.visible = true;
        shop.on_close = true;
        let mut child = window("A", 0);
        child.on_close = true;
        shop.children = vec![child];
        gate.first = vec![shop];
        gate.first_count = 1;
        gate.captures = vec![Capture {
            name: "Shop".to_string(),
            code: 0,
        }];
        gate.hover = Some("Shop".to_string());
        let mut side = window("Side", 0);
        side.owner = Some("Shop".to_string());
        side.on_close = true;
        let mut other = window("Other", 0);
        other.owner = Some("Else".to_string());
        other.on_close = true;
        let mut done = window("Done", 0);
        done.owner = Some("Shop".to_string());
        done.closed = true;
        done.on_close = true;
        gate.siblings = vec![side, other, done];
        let mut dying = hired_physician();
        assert!(gate.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut dying));
        assert!(gate.captures.is_empty());
        assert_eq!(gate.first[0].flags & CAPTURE_BIT, 0);
        assert!(gate.hover.is_none());
        assert_eq!(
            gate.close_log,
            vec!["Side".to_string(), "A".to_string(), "Shop".to_string()]
        );
        assert!(gate.siblings[0].close_ran);
        assert_eq!(gate.siblings[0].seen_arg, Some(0));
        assert!(!gate.siblings[1].closed);
        assert!(!gate.siblings[2].close_ran);
        assert!(!gate.first[0].visible);
        assert_eq!(gate.redraw, REDRAW_BITS);
        assert_eq!(gate.redraws, 4);
        assert!(dying.roster.is_empty());
        assert_eq!(dying.deaths, 1);

        let mut gate = MatchGate::playing();
        let mut board = window("Board", DISMISS_MARK | CAPTURE_BIT);
        board.closed = true;
        board.on_close = true;
        board.visible = true;
        gate.first = vec![board];
        gate.first_count = 1;
        gate.captures = vec![Capture {
            name: "Board".to_string(),
            code: 0,
        }];
        gate.hover = Some("Board".to_string());
        let mut company = Company::new("Ash");
        assert!(gate.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut company));
        assert!(gate.captures.is_empty());
        assert_eq!(gate.first[0].flags & CAPTURE_BIT, 0);
        assert!(gate.close_log.is_empty());
        assert_eq!(gate.redraws, 0);
        assert_eq!(gate.hover.as_deref(), Some("Board"));
        assert!(gate.first[0].visible);

        let mut gate = MatchGate::playing();
        let mut live = window("Live", DISMISS_MARK);
        live.on_close = true;
        gate.first = vec![live];
        gate.first_count = 1;
        gate.active = Some("Live".to_string());
        gate.hover = Some("Live".to_string());
        let mut side = window("Side", 0);
        side.owner = Some("Live".to_string());
        side.on_close = true;
        gate.siblings = vec![side];
        let mut company = Company::new("Ash");
        assert!(gate.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut company));
        assert!(gate.active.is_none());
        assert_eq!(gate.hover.as_deref(), Some("Live"));
        assert!(!gate.siblings[0].closed);
        assert_eq!(gate.close_log, vec!["Live".to_string()]);
        assert_eq!(gate.redraws, 0);

        let mut gate = MatchGate::playing();
        let held = window("Held", DISMISS_MARK | CAPTURE_BIT);
        gate.first = vec![held];
        gate.first_count = 1;
        gate.captures = vec![Capture {
            name: "Someone".to_string(),
            code: 0,
        }];
        let mut company = Company::new("Ash");
        assert!(gate.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut company));
        assert_eq!(
            gate.captures,
            vec![Capture {
                name: "Someone".to_string(),
                code: 0,
            }]
        );
        assert_eq!(gate.first[0].flags & CAPTURE_BIT, CAPTURE_BIT);
        assert!(gate.first[0].closed);
    }

    fn capture(name: &str, code: u32) -> Capture {
        Capture {
            name: name.to_string(),
            code,
        }
    }

    fn marked(name: &str) -> Window {
        let mut window = window(name, DISMISS_MARK | CAPTURE_BIT);
        window.on_close = true;
        window
    }

    #[test]
    fn a_release_restores_the_lower_capture_and_the_lists_are_followed() {
        let mut gate = MatchGate::playing();
        gate.first = vec![marked("Shop")];
        gate.first_count = 1;
        gate.captures = vec![capture("Lower", 0x15), capture("Shop", 0x15)];
        let mut dying = hired_physician();
        assert!(gate.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut dying));
        assert_eq!(gate.cursor_code, 0x15);
        assert_eq!(gate.cursor_holder.as_deref(), Some("Lower"));
        assert_eq!(gate.cursor_shape, 0);
        assert!(gate.cursor_default);
        assert_eq!(dying.deaths, 1);

        let mut gate = MatchGate::playing();
        gate.first = vec![marked("Shop")];
        gate.first_count = 1;
        gate.captures = vec![capture("Lower", 0x8007), capture("Shop", 1)];
        let mut company = Company::new("Ash");
        assert!(gate.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut company));
        assert_eq!(gate.cursor_code, 0x8007);
        assert_eq!(gate.cursor_holder.as_deref(), Some("Lower"));
        assert_eq!(gate.cursor_shape, 7);
        assert!(!gate.cursor_default);

        let mut gate = MatchGate::playing();
        gate.first = vec![marked("Shop")];
        gate.first_count = 1;
        gate.captures = vec![capture("Shop", 0x15)];
        gate.cursor_default = false;
        let mut company = Company::new("Ash");
        assert!(gate.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut company));
        assert!(gate.captures.is_empty());
        assert_eq!(gate.cursor_code, 0);
        assert!(gate.cursor_holder.is_none());
        assert!(!gate.cursor_default);

        let mut gate = MatchGate::playing();
        gate.list_mask = 1;
        gate.clock = 99;
        gate.mode_word = 6;
        gate.hands_ready = true;
        let mut company = Company::new("Ash");
        company.hire("Med", Profession::Physician).unwrap();
        company.roster[0].health = 699;
        assert!(gate.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut company));
        assert!(gate.hands_set);
        assert_eq!(gate.hands, HAND_WORDS);
        assert_eq!(gate.mode_word, 3);
        assert_eq!(gate.stamp, 99);
        assert!(!gate.latched);
        assert!(gate.pending);
        assert!(company.roster.is_empty());

        let mut gate = MatchGate::playing();
        gate.list_mask = 0;
        gate.focus = false;
        gate.hands_ready = true;
        gate.mode_word = 6;
        let mut company = Company::new("Ash");
        assert!(gate.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut company));
        assert!(!gate.hands_set);
        assert_eq!(gate.mode_word, 6);
        assert_eq!(gate.stamp, 0);

        let mut gate = MatchGate::playing();
        gate.focus = true;
        gate.list_mask = 0;
        gate.hands_ready = true;
        gate.clock = 4;
        let mut company = Company::new("Ash");
        assert!(gate.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut company));
        assert!(gate.latched);
        assert!(!gate.pending);
        assert!(gate.hands_set);
        assert_eq!(gate.stamp, 4);
        assert_eq!(gate.mode_word, 1);
    }

    fn shape(image: u64, hook: bool) -> CursorShape {
        CursorShape { image, hook }
    }

    #[test]
    fn a_shape_is_shown_and_a_focus_is_invoked_before_the_hands() {
        let mut gate = MatchGate::playing();
        gate.first = vec![marked("Shop")];
        gate.first_count = 1;
        gate.captures = vec![capture("Lower", 0x8007), capture("Shop", 1)];
        gate.shapes = vec![shape(0, false); 6];
        gate.shapes.push(shape(42, true));
        let mut company = Company::new("Ash");
        assert!(gate.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut company));
        assert_eq!(gate.current_image, 42);
        assert_eq!(gate.shown, 1);
        assert_eq!(gate.shape_hooks, 1);
        assert!(!gate.shape_mark);

        let mut again = MatchGate::playing();
        again.first = vec![marked("Shop")];
        again.first_count = 1;
        again.captures = vec![capture("Lower", 0x8007), capture("Shop", 1)];
        again.shapes = vec![shape(0, false); 6];
        again.shapes.push(shape(42, true));
        again.current_image = 42;
        let mut company = Company::new("Ash");
        assert!(again.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut company));
        assert_eq!(again.shown, 0);
        assert_eq!(again.shape_hooks, 1);
        assert_eq!(again.current_image, 42);

        let mut plain = MatchGate::playing();
        plain.first = vec![marked("Shop")];
        plain.first_count = 1;
        plain.captures = vec![capture("Lower", 0x15), capture("Shop", 1)];
        plain.default_image = 9;
        plain.shape_clock = 12;
        let mut company = Company::new("Ash");
        assert!(plain.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut company));
        assert_eq!(plain.current_image, 9);
        assert_eq!(plain.shown, 1);
        assert!(plain.shape_mark);
        assert_eq!(plain.shape_stamp, 12);
        assert!(plain.cursor_default);

        let mut empty = MatchGate::playing();
        empty.first = vec![marked("Shop")];
        empty.first_count = 1;
        empty.captures = vec![capture("Lower", 0x8007), capture("Shop", 1)];
        empty.current_image = 3;
        let mut company = Company::new("Ash");
        assert!(empty.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut company));
        assert_eq!(empty.current_image, 3);
        assert_eq!(empty.shown, 0);
        assert_eq!(empty.cursor_shape, 7);

        let mut focus = MatchGate::playing();
        focus.focus = true;
        focus.session_mark = 1;
        focus.hands_ready = true;
        focus.clock = 5;
        let mut dying = hired_physician();
        assert!(focus.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut dying));
        assert_eq!(focus.focus_calls, 1);
        assert!(focus.hands_set);
        assert_eq!(focus.session_mark, 0);
        assert_eq!(dying.deaths, 1);

        let mut mask = MatchGate::playing();
        mask.list_mask = 1;
        mask.session_mark = 1;
        mask.hands_ready = true;
        let mut company = Company::new("Ash");
        assert!(mask.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut company));
        assert_eq!(mask.focus_calls, 0);
        assert_eq!(mask.session_mark, 1);
        assert!(mask.hands_set);
    }
}
