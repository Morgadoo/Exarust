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
/// Code stored when a session mark changes to 0.
pub const ENTERED_CODE: u32 = 4;
/// Bit set on the view word when the session mark is raised to 1.
pub const VIEW_BIT: u32 = 0x10;
/// Mask kept on the world flags when a non-zero side is stored.
pub const WORLD_KEEP: u32 = 0xffff_fdbd;
/// High nibble that keeps a hook code. A code without it gains the low bit.
pub const HOOK_HIGH: u32 = 0xf000;
pub const HOOK_LOW: u32 = 0x1000;
/// Codes stored on the primary hook list.
pub const HOOK_GATE: u32 = 0x8001;
pub const HOOK_WIDE: u32 = 0x2001;
pub const HOOK_SMALL: u32 = 2;
/// Sort key of the ordered hook swapped when the mark changes.
pub const HOOK_ORDER: i32 = 4;
/// Bit that keeps a watch stamp. Without it the stamp becomes 0.
pub const WATCH_BIT: u32 = 0x2000;
/// Bit a watch code of 1 clears and a code of 2 sets.
pub const WATCH_MARK: u32 = 0x800000;
/// Words copied from one control bank. The bytes between them are not copied.
pub const BANK_WORDS: usize = 16;

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

/// One install record. Index 1 is the first record. The eight bytes after the
/// hook are not read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallRecord {
    pub value: u64,
    pub hook: bool,
}

/// One registered hook. The primary list stores a code. The ordered list stores
/// a sort key in the same field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedHook {
    pub hook: u64,
    pub owner: u64,
    pub code: u32,
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
    /// Times a focus was invoked. That virtual method is not the watch hook.
    pub focus_calls: u32,
    /// Session mark cleared to 0 when it was not already 0.
    pub session_mark: u8,
    /// Stored as 0 when that mark changes to 0.
    pub entered_flag: u8,
    /// Stored as 4 when that mark changes to 0.
    pub entered_code: u32,
    /// Side 0 selects the first bank. Any other side selects the second.
    pub bank_side: u8,
    /// Raised when the side is not 0.
    pub side_notice: u8,
    /// Cleared when the session mark is raised to 1.
    pub side_mid: u8,
    /// Cleared when a non-zero side is stored, and when the mark is raised.
    pub side_tail: u8,
    /// When this is not 0, a world byte does not replace the side.
    pub side_lock: u8,
    /// A world is present, so a cleared mark updates the side from its byte.
    pub world_open: bool,
    /// World byte the side is taken from, then written back.
    pub world_side: u8,
    /// When this is not 0, that byte is forced to 0.
    pub world_force: u8,
    /// Cleared when the mark is raised and a world is open.
    pub world_child_quiet: u8,
    pub world_flags: u32,
    pub world_child_code: u32,
    /// Cleared when a non-zero side is stored.
    pub world_child_span: u32,
    /// Cleared when the session mark is raised to 1.
    pub hold_byte: u8,
    pub view_flags: u32,
    pub view_code: u32,
    /// The two words passed with a shown value. Filled at run time.
    pub draw_owner: u64,
    pub draw_extra: u64,
    pub draw_owner_used: u64,
    pub draw_extra_used: u64,
    pub draw_value: u64,
    /// Set when a show calls the resolved action-set target.
    pub activates_action_set: bool,
    /// Empty primary list. The first register stores index 0.
    pub hook_high: i32,
    pub primary: Vec<ListedHook>,
    pub ordered_count: u32,
    pub ordered: Vec<ListedHook>,
    pub gate_hook: u64,
    pub plain_hook: u64,
    pub lab_hook: u64,
    pub wide_hook: u64,
    pub self_owner: u64,
    pub watch_hook: u64,
    pub watch_owner: u64,
    pub play_hook: u64,
    pub play_owner: u64,
    /// Words the gate hook clears, then the flags it sets to 1.
    pub gate_hook_out: u64,
    pub gate_owner_out: u64,
    pub gate_flags: u32,
    /// Counted when the mid byte is set. The pick under the cursor is not in.
    pub gate_alt: u64,
    pub gate_alt_calls: u32,
    pub watch_flags: u32,
    pub watch_stamp: u32,
    pub watch_clock: u32,
    pub watch_target: u64,
    pub watch_ready: u8,
    pub watch_held: u8,
    pub watch_code: i32,
    pub target_flags: u32,
    pub watch_releases: u32,
    pub watch_presses: u32,
    pub watch_pending: i32,
    pub watch_refreshes: u32,
    pub watch_leave: u32,
    pub watch_focus: u64,
    /// Cleared by the first bank and set by the second. A match leaves the words.
    pub bank_flag: u8,
    pub bank_active: [u64; BANK_WORDS],
    pub bank_first: [u64; BANK_WORDS],
    pub bank_second: [u64; BANK_WORDS],
    /// Times the install after a bank select was counted.
    pub bank_installs: u32,
    /// Index passed for side 0. Index 1 is the first record.
    pub install_first: u32,
    /// Index passed for any other side.
    pub install_second: u32,
    /// The install returns at once when this is 0.
    pub install_ready: u8,
    /// Index stored by the last install that changed it.
    pub install_index: u32,
    /// Shown when the index is below 1 and the current index is not 0.
    pub install_fallback: u64,
    /// Last value shown. The same value is not shown again.
    pub install_shown: u64,
    /// A non-zero word refuses the show.
    pub install_blocked: u32,
    /// Times a different non-zero value was handed to the platform call.
    pub install_shows: u32,
    /// Set when an index below 1 clears a current index.
    pub install_armed: u8,
    pub install_clock: u32,
    pub install_stamp: u32,
    pub install_records: Vec<InstallRecord>,
    /// Times a record hook ran. That record hook is not the gate or watch hook.
    pub install_hooks: u32,
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
            entered_flag: 0,
            entered_code: 0,
            bank_side: 0,
            side_notice: 0,
            side_mid: 0,
            side_tail: 0,
            side_lock: 0,
            world_open: false,
            world_side: 0,
            world_force: 0,
            world_child_quiet: 0,
            world_flags: 0,
            world_child_code: 0,
            world_child_span: 0,
            hold_byte: 0,
            view_flags: 0,
            view_code: 0,
            draw_owner: 0,
            draw_extra: 0,
            draw_owner_used: 0,
            draw_extra_used: 0,
            draw_value: 0,
            activates_action_set: false,
            hook_high: -1,
            primary: Vec::new(),
            ordered_count: 0,
            ordered: Vec::new(),
            gate_hook: 0,
            plain_hook: 0,
            lab_hook: 0,
            wide_hook: 0,
            self_owner: 0,
            watch_hook: 0,
            watch_owner: 0,
            play_hook: 0,
            play_owner: 0,
            gate_hook_out: 0,
            gate_owner_out: 0,
            gate_flags: 0,
            gate_alt: 0,
            gate_alt_calls: 0,
            watch_flags: 0,
            watch_stamp: 0,
            watch_clock: 0,
            watch_target: 0,
            watch_ready: 0,
            watch_held: 0,
            watch_code: 0,
            target_flags: 0,
            watch_releases: 0,
            watch_presses: 0,
            watch_pending: 0,
            watch_refreshes: 0,
            watch_leave: 0,
            watch_focus: 0,
            bank_flag: 0,
            bank_active: [0; BANK_WORDS],
            bank_first: [0; BANK_WORDS],
            bank_second: [0; BANK_WORDS],
            bank_installs: 0,
            install_first: 0,
            install_second: 0,
            install_ready: 0,
            install_index: 0,
            install_fallback: 0,
            install_shown: 0,
            install_blocked: 0,
            install_shows: 0,
            install_armed: 0,
            install_clock: 0,
            install_stamp: 0,
            install_records: Vec::new(),
            install_hooks: 0,
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

    /// Raise the session mark to 1. A mark that is already 1 changes nothing.
    pub fn set_session_aside(&mut self) {
        if self.session_mark == 1 {
            return;
        }
        self.session_mark = 1;
        self.entered_flag = 1;
        self.entered_code = 0;
        if self.world_open {
            self.world_child_quiet = 0;
            self.world_flags = 0;
            self.world_child_code = ENTERED_CODE;
        }
        if self.bank_side == 0 {
            drop_primary(self, self.plain_hook);
            drop_primary(self, self.lab_hook);
        } else {
            drop_primary(self, self.wide_hook);
            drop_primary(self, self.wide_hook);
        }
        drop_primary(self, self.gate_hook);
        self.side_notice = 0;
        self.side_mid = 0;
        self.side_tail = 0;
        self.entered_flag = 1;
        self.entered_code = 0;
        drop_ordered(self, self.play_hook, self.play_owner);
        insert_ordered(self, self.watch_hook, self.watch_owner, HOOK_ORDER);
        select_bank(self, 0);
        self.hold_byte = 0;
        self.view_flags |= VIEW_BIT;
        self.view_code = 0;
    }

    /// The gate hook. It clears two words and sets the flags to 1. A set mid
    /// byte counts the alternate hook. The pick under the cursor is not in.
    pub fn run_gate(&mut self) -> u32 {
        self.gate_hook_out = 0;
        self.gate_owner_out = 0;
        self.gate_flags = 1;
        if self.side_mid != 0 && self.gate_alt != 0 {
            self.gate_alt_calls = self.gate_alt_calls.saturating_add(1);
        }
        self.gate_flags
    }

    /// The watch hook. It returns 1. A clear bit 0x2000 stores a stamp of 0.
    pub fn run_watch(&mut self) -> u8 {
        if self.watch_flags & WATCH_BIT == 0 {
            self.watch_stamp = 0;
        } else if self.watch_target != 0 && self.watch_ready != 0 {
            if self.watch_held == 0 {
                self.watch_stamp = self.watch_clock;
            }
            if self.watch_code > 0 {
                if self.watch_code == 1 {
                    if self.target_flags & WATCH_MARK != 0 {
                        self.target_flags &= !WATCH_MARK;
                        self.watch_releases = self.watch_releases.saturating_add(1);
                    }
                } else if self.watch_code == 2 {
                    self.target_flags |= WATCH_MARK;
                    self.watch_presses = self.watch_presses.saturating_add(1);
                }
            }
        }
        if self.watch_pending > 0 {
            self.watch_refreshes = self.watch_refreshes.saturating_add(1);
        }
        if self.watch_leave != 0 && self.watch_focus != 0 {
            self.focus_calls = self.focus_calls.saturating_add(1);
            self.watch_focus = 0;
            if self.session_mark != 0 {
                enter_session(self);
            }
        }
        1
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
/// mask shares bit 0. A focus is counted before the hands. The focus method
/// body is not this watch hook.
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
        enter_session(gate);
        drop_ordered(gate, gate.watch_hook, gate.watch_owner);
        insert_ordered(gate, gate.play_hook, gate.play_owner, HOOK_ORDER);
    }
}

/// The mark changes to 0. The focus method that precedes a hand copy is not this.
fn enter_session(gate: &mut MatchGate) {
    gate.session_mark = 0;
    gate.entered_flag = 0;
    gate.entered_code = ENTERED_CODE;
    if gate.world_open {
        gate.bank_side = u8::from(gate.world_side == 0);
        apply_world_side(gate);
    }
    if gate.hold_byte == 0 {
        register_primary(gate, HOOK_GATE, gate.gate_hook, gate.self_owner);
    }
    if gate.bank_side == 0 {
        select_bank(gate, 1);
    } else {
        gate.side_notice = 1;
        select_bank(gate, 2);
    }
}

/// Store a world byte as the side when it differs, then select that bank.
fn apply_world_side(gate: &mut MatchGate) {
    if gate.side_lock != 0 {
        return;
    }
    let mut next = gate.world_side;
    if gate.world_force != 0 {
        next = 0;
    }
    if next == gate.bank_side {
        return;
    }
    gate.bank_side = next;
    if next == 0 {
        select_bank(gate, 1);
        gate.side_notice = 0;
        register_primary(gate, HOOK_WIDE, gate.plain_hook, gate.self_owner);
        register_primary(gate, HOOK_SMALL, gate.lab_hook, gate.self_owner);
    } else {
        select_bank(gate, 2);
        gate.side_notice = 1;
        gate.side_tail = 0;
        gate.world_child_span = 0;
        gate.world_flags &= WORLD_KEEP;
        register_primary(gate, HOOK_WIDE, gate.wide_hook, gate.self_owner);
        register_primary(gate, HOOK_SMALL, gate.wide_hook, gate.self_owner);
        drop_primary(gate, gate.plain_hook);
        drop_primary(gate, gate.lab_hook);
    }
    gate.world_side = gate.bank_side;
}

/// Copy sixteen words when the bank flag does not already match, then run
/// the install. The platform cursor call is counted.
fn select_bank(gate: &mut MatchGate, mode: u8) {
    if mode == 0 {
        install_index(gate, 0);
        gate.bank_installs = gate.bank_installs.saturating_add(1);
        return;
    }
    if mode == 1 {
        if gate.bank_flag != 0 {
            gate.bank_flag = 0;
            gate.bank_active = gate.bank_first;
        }
    } else if mode == 2 && gate.bank_flag == 0 {
        gate.bank_flag = 1;
        gate.bank_active = gate.bank_second;
    }
    let index = if mode == 2 {
        gate.install_second
    } else {
        gate.install_first
    };
    install_index(gate, index);
    gate.bank_installs = gate.bank_installs.saturating_add(1);
}

/// Store a new index and show its record. Index 0 clears the current index.
/// A matching sign-extended index returns. The platform call is counted.
fn install_index(gate: &mut MatchGate, index: u32) {
    if gate.install_ready == 0 {
        return;
    }
    if index == 0 {
        if gate.install_index == 0 {
            return;
        }
        gate.install_index = 0;
        let fallback = gate.install_fallback;
        show_installed(gate, fallback);
        gate.install_armed = 1;
        gate.install_stamp = gate.install_clock;
        return;
    }
    let wide = u64::from(index);
    let current = i64::from(gate.install_index.cast_signed()) as u64;
    if wide == current {
        return;
    }
    gate.install_index = index;
    let slot = (index as usize) - 1;
    if slot >= gate.install_records.len() {
        return;
    }
    let record = gate.install_records[slot].clone();
    show_installed(gate, record.value);
    if record.hook {
        gate.install_hooks = gate.install_hooks.saturating_add(1);
    }
}

fn show_installed(gate: &mut MatchGate, value: u64) {
    if value == gate.install_shown || value == 0 || gate.install_blocked != 0 {
        return;
    }
    let owner = gate.draw_owner;
    let extra = gate.draw_extra;
    gate.draw_owner_used = owner;
    gate.draw_extra_used = extra;
    gate.draw_value = value;
    gate.activates_action_set = true;
    gate.install_shows = gate.install_shows.saturating_add(1);
    gate.install_shown = value;
}

fn register_primary(gate: &mut MatchGate, code: u32, hook: u64, owner: u64) {
    gate.hook_high = gate.hook_high.wrapping_add(1);
    let stored = if code & HOOK_HIGH == 0 {
        code | HOOK_LOW
    } else {
        code
    };
    let index = gate.hook_high as usize;
    if gate.primary.len() <= index {
        gate.primary.resize(
            index + 1,
            ListedHook {
                hook: 0,
                owner: 0,
                code: 0,
            },
        );
    }
    gate.primary[index] = ListedHook {
        hook,
        owner,
        code: stored,
    };
}

fn drop_primary(gate: &mut MatchGate, hook: u64) {
    let mut index = 0i32;
    while index <= gate.hook_high {
        let slot = index as usize;
        let matched = gate.primary.get(slot).is_some_and(|row| row.hook == hook);
        if !matched {
            index += 1;
            continue;
        }
        gate.hook_high -= 1;
        if index <= gate.hook_high {
            let mut cursor = index;
            while cursor <= gate.hook_high {
                let from = (cursor as usize) + 1;
                gate.primary[cursor as usize] = gate.primary[from].clone();
                cursor += 1;
            }
        }
    }
    let keep = (gate.hook_high + 1).max(0) as usize;
    gate.primary.truncate(keep);
}

fn insert_ordered(gate: &mut MatchGate, hook: u64, owner: u64, key: i32) {
    let count = gate.ordered_count;
    let mut place = count;
    gate.ordered.resize(
        (count as usize) + 1,
        ListedHook {
            hook: 0,
            owner: 0,
            code: 0,
        },
    );
    if count > 0 {
        let mut index: i32 = -1;
        let last = count as i32 - 1;
        loop {
            index += 1;
            let existing = gate.ordered[index as usize].code.cast_signed();
            if key < existing {
                let stop = index + 1;
                if stop <= count as i32 {
                    let mut cursor = count as i32 + 1;
                    loop {
                        cursor -= 1;
                        let from = (cursor as usize) - 1;
                        gate.ordered[cursor as usize] = gate.ordered[from].clone();
                        if stop >= cursor {
                            break;
                        }
                    }
                }
                place = index as u32;
                break;
            }
            if last <= index {
                break;
            }
        }
    }
    gate.ordered[place as usize] = ListedHook {
        hook,
        owner,
        code: key as u32,
    };
    gate.ordered_count = count + 1;
}

fn drop_ordered(gate: &mut MatchGate, hook: u64, owner: u64) {
    let count = gate.ordered_count as i32;
    if count < 1 {
        return;
    }
    let mut index: i32 = -1;
    loop {
        index += 1;
        let row = &gate.ordered[index as usize];
        if row.hook == hook && row.owner == owner {
            gate.ordered_count -= 1;
            let new_count = gate.ordered_count as i32;
            if index < new_count {
                let mut cursor = index;
                while cursor < new_count {
                    let from = (cursor as usize) + 1;
                    gate.ordered[cursor as usize] = gate.ordered[from].clone();
                    cursor += 1;
                }
            }
            gate.ordered.truncate(gate.ordered_count as usize);
            return;
        }
        if index >= count - 1 {
            return;
        }
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

    fn bank(first: u64) -> [u64; BANK_WORDS] {
        let mut words = [0; BANK_WORDS];
        words[0] = first;
        words[15] = first + 1;
        words
    }

    #[test]
    fn a_cleared_mark_copies_one_control_bank() {
        let mut first = MatchGate::playing();
        first.focus = true;
        first.session_mark = 1;
        first.entered_flag = 3;
        first.bank_flag = 1;
        first.bank_first = bank(11);
        let mut dying = hired_physician();
        assert!(first.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut dying));
        assert_eq!(first.session_mark, 0);
        assert_eq!(first.entered_flag, 0);
        assert_eq!(first.entered_code, ENTERED_CODE);
        assert_eq!(first.bank_flag, 0);
        assert_eq!(first.bank_active, bank(11));
        assert_eq!(first.bank_installs, 1);
        assert_eq!(first.side_notice, 0);
        assert_eq!(dying.deaths, 1);

        let mut second = MatchGate::playing();
        second.focus = true;
        second.session_mark = 1;
        second.bank_side = 1;
        second.bank_second = bank(22);
        let mut company = Company::new("Ash");
        assert!(second.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut company));
        assert_eq!(second.side_notice, 1);
        assert_eq!(second.bank_flag, 1);
        assert_eq!(second.bank_active, bank(22));
        assert_eq!(second.entered_code, ENTERED_CODE);
        assert_eq!(second.bank_installs, 1);

        let mut stayed = MatchGate::playing();
        stayed.focus = true;
        stayed.entered_code = 9;
        stayed.bank_flag = 1;
        stayed.bank_active = bank(5);
        stayed.bank_first = bank(11);
        let mut company = Company::new("Ash");
        assert!(stayed.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut company));
        assert_eq!(stayed.entered_code, 9);
        assert_eq!(stayed.bank_flag, 1);
        assert_eq!(stayed.bank_active, bank(5));
        assert_eq!(stayed.bank_installs, 0);

        let mut same = MatchGate::playing();
        same.focus = true;
        same.session_mark = 1;
        same.bank_active = bank(7);
        same.bank_first = bank(11);
        let mut company = Company::new("Ash");
        assert!(same.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut company));
        assert_eq!(same.bank_active, bank(7));
        assert_eq!(same.bank_flag, 0);
        assert_eq!(same.bank_installs, 1);
        assert_eq!(same.entered_code, ENTERED_CODE);

        let mut mask = MatchGate::playing();
        mask.list_mask = 1;
        mask.session_mark = 1;
        mask.bank_flag = 1;
        mask.bank_active = bank(5);
        mask.bank_first = bank(11);
        let mut company = Company::new("Ash");
        assert!(mask.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut company));
        assert_eq!(mask.bank_installs, 0);
        assert_eq!(mask.entered_code, 0);
        assert_eq!(mask.session_mark, 1);
        assert_eq!(mask.bank_active, bank(5));
        assert_eq!(mask.bank_flag, 1);
    }

    fn installed(value: u64, hook: bool) -> InstallRecord {
        InstallRecord { value, hook }
    }

    #[test]
    fn an_install_shows_a_record_and_runs_its_hook() {
        let mut gate = MatchGate::playing();
        gate.focus = true;
        gate.session_mark = 1;
        gate.install_ready = 1;
        gate.install_first = 1;
        gate.install_records = vec![installed(42, true)];
        let mut dying = hired_physician();
        assert!(gate.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut dying));
        assert_eq!(gate.install_index, 1);
        assert_eq!(gate.install_shown, 42);
        assert_eq!(gate.install_shows, 1);
        assert_eq!(gate.install_hooks, 1);
        assert_eq!(gate.bank_installs, 1);
        assert_eq!(dying.deaths, 1);

        gate.session_mark = 1;
        let mut company = Company::new("Ash");
        assert!(gate.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut company));
        assert_eq!(gate.install_shows, 1);
        assert_eq!(gate.install_hooks, 1);
        assert_eq!(gate.install_index, 1);

        let mut zero = MatchGate::playing();
        zero.focus = true;
        zero.session_mark = 1;
        zero.install_ready = 1;
        zero.install_first = 2;
        zero.install_records = vec![installed(1, false), installed(0, true)];
        let mut company = Company::new("Ash");
        assert!(zero.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut company));
        assert_eq!(zero.install_index, 2);
        assert_eq!(zero.install_shows, 0);
        assert_eq!(zero.install_shown, 0);
        assert_eq!(zero.install_hooks, 1);

        let mut blocked = MatchGate::playing();
        blocked.focus = true;
        blocked.session_mark = 1;
        blocked.install_ready = 1;
        blocked.install_first = 1;
        blocked.install_blocked = 1;
        blocked.install_shown = 3;
        blocked.install_records = vec![installed(42, true)];
        let mut company = Company::new("Ash");
        assert!(blocked.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut company));
        assert_eq!(blocked.install_shows, 0);
        assert_eq!(blocked.install_shown, 3);
        assert_eq!(blocked.install_hooks, 1);
        assert_eq!(blocked.install_index, 1);

        let mut same = MatchGate::playing();
        same.focus = true;
        same.session_mark = 1;
        same.install_ready = 1;
        same.install_first = 1;
        same.install_shown = 42;
        same.install_records = vec![installed(42, true)];
        let mut company = Company::new("Ash");
        assert!(same.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut company));
        assert_eq!(same.install_shows, 0);
        assert_eq!(same.install_shown, 42);
        assert_eq!(same.install_hooks, 1);

        let mut past = MatchGate::playing();
        past.focus = true;
        past.session_mark = 1;
        past.install_ready = 1;
        past.install_first = 4;
        past.install_records = vec![installed(42, true)];
        let mut company = Company::new("Ash");
        assert!(past.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut company));
        assert_eq!(past.install_index, 4);
        assert_eq!(past.install_shows, 0);
        assert_eq!(past.install_hooks, 0);

        let mut clear = MatchGate::playing();
        clear.focus = true;
        clear.session_mark = 1;
        clear.install_ready = 1;
        clear.install_index = 5;
        clear.install_fallback = 9;
        clear.install_clock = 12;
        let mut company = Company::new("Ash");
        assert!(clear.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut company));
        assert_eq!(clear.install_index, 0);
        assert_eq!(clear.install_shown, 9);
        assert_eq!(clear.install_shows, 1);
        assert_eq!(clear.install_armed, 1);
        assert_eq!(clear.install_stamp, 12);
        assert_eq!(clear.install_hooks, 0);

        let mut idle = MatchGate::playing();
        idle.focus = true;
        idle.session_mark = 1;
        idle.install_ready = 1;
        idle.install_fallback = 9;
        let mut company = Company::new("Ash");
        assert!(idle.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut company));
        assert_eq!(idle.install_index, 0);
        assert_eq!(idle.install_armed, 0);
        assert_eq!(idle.install_shows, 0);
        assert_eq!(idle.bank_installs, 1);

        let mut closed = MatchGate::playing();
        closed.focus = true;
        closed.session_mark = 1;
        closed.install_first = 1;
        closed.install_records = vec![installed(42, true)];
        let mut company = Company::new("Ash");
        assert!(closed.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut company));
        assert_eq!(closed.install_index, 0);
        assert_eq!(closed.install_shows, 0);
        assert_eq!(closed.install_hooks, 0);
        assert_eq!(closed.bank_installs, 1);
    }

    #[test]
    fn a_shown_value_is_drawn_and_a_world_byte_sets_the_side() {
        let mut drawn = MatchGate::playing();
        drawn.focus = true;
        drawn.session_mark = 1;
        drawn.install_ready = 1;
        drawn.install_first = 1;
        drawn.install_records = vec![installed(42, true)];
        drawn.draw_owner = 7;
        drawn.draw_extra = 8;
        let mut dying = hired_physician();
        assert!(drawn.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut dying));
        assert_eq!(drawn.draw_owner_used, 7);
        assert_eq!(drawn.draw_extra_used, 8);
        assert_eq!(drawn.draw_value, 42);
        assert_eq!(drawn.install_shows, 1);
        assert_eq!(dying.deaths, 1);

        let mut clear = MatchGate::playing();
        clear.focus = true;
        clear.session_mark = 1;
        clear.world_open = true;
        clear.bank_flag = 1;
        clear.bank_first = bank(11);
        let mut company = Company::new("Ash");
        assert!(clear.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut company));
        assert_eq!(clear.bank_side, 0);
        assert_eq!(clear.world_side, 0);
        assert_eq!(clear.bank_active, bank(11));
        assert_eq!(clear.bank_flag, 0);
        assert_eq!(clear.bank_installs, 2);
        assert_eq!(clear.side_notice, 0);

        let mut other = MatchGate::playing();
        other.focus = true;
        other.session_mark = 1;
        other.world_open = true;
        other.world_side = 5;
        other.world_flags = u32::MAX;
        other.world_child_span = 9;
        other.side_tail = 3;
        other.bank_second = bank(22);
        let mut company = Company::new("Ash");
        assert!(other.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut company));
        assert_eq!(other.bank_side, 5);
        assert_eq!(other.world_side, 5);
        assert_eq!(other.bank_active, bank(22));
        assert_eq!(other.bank_flag, 1);
        assert_eq!(other.side_notice, 1);
        assert_eq!(other.side_tail, 0);
        assert_eq!(other.world_child_span, 0);
        assert_eq!(other.world_flags, WORLD_KEEP);
        assert_eq!(other.bank_installs, 2);

        let mut forced = MatchGate::playing();
        forced.focus = true;
        forced.session_mark = 1;
        forced.world_open = true;
        forced.world_side = 5;
        forced.world_force = 1;
        forced.world_flags = 0x11;
        forced.bank_active = bank(7);
        let mut company = Company::new("Ash");
        assert!(forced.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut company));
        assert_eq!(forced.bank_side, 0);
        assert_eq!(forced.world_side, 5);
        assert_eq!(forced.world_flags, 0x11);
        assert_eq!(forced.bank_active, bank(7));
        assert_eq!(forced.bank_flag, 0);
        assert_eq!(forced.bank_installs, 1);

        let mut locked = MatchGate::playing();
        locked.focus = true;
        locked.session_mark = 1;
        locked.world_open = true;
        locked.side_lock = 1;
        locked.bank_flag = 1;
        locked.side_tail = 3;
        locked.bank_active = bank(7);
        let mut company = Company::new("Ash");
        assert!(locked.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut company));
        assert_eq!(locked.bank_side, 1);
        assert_eq!(locked.world_side, 0);
        assert_eq!(locked.side_notice, 1);
        assert_eq!(locked.side_tail, 3);
        assert_eq!(locked.bank_flag, 1);
        assert_eq!(locked.bank_active, bank(7));
        assert_eq!(locked.bank_installs, 1);

        let mut aside = MatchGate::playing();
        aside.world_open = true;
        aside.world_child_quiet = 3;
        aside.world_flags = 0xff;
        aside.world_child_code = 9;
        aside.side_notice = 1;
        aside.side_mid = 1;
        aside.side_tail = 1;
        aside.hold_byte = 1;
        aside.view_code = 6;
        aside.install_ready = 1;
        aside.install_index = 5;
        aside.install_fallback = 9;
        aside.bank_flag = 1;
        aside.bank_active = bank(7);
        aside.set_session_aside();
        assert_eq!(aside.session_mark, 1);
        assert_eq!(aside.entered_flag, 1);
        assert_eq!(aside.entered_code, 0);
        assert_eq!(aside.world_child_quiet, 0);
        assert_eq!(aside.world_flags, 0);
        assert_eq!(aside.world_child_code, ENTERED_CODE);
        assert_eq!(aside.side_notice, 0);
        assert_eq!(aside.side_mid, 0);
        assert_eq!(aside.side_tail, 0);
        assert_eq!(aside.install_index, 0);
        assert_eq!(aside.install_shown, 9);
        assert_eq!(aside.install_armed, 1);
        assert_eq!(aside.draw_value, 9);
        assert_eq!(aside.bank_active, bank(7));
        assert_eq!(aside.bank_flag, 1);
        assert_eq!(aside.hold_byte, 0);
        assert_eq!(aside.view_flags, VIEW_BIT);
        assert_eq!(aside.view_code, 0);
        assert_eq!(aside.bank_installs, 1);

        aside.entered_code = 7;
        aside.set_session_aside();
        assert_eq!(aside.entered_code, 7);
        assert_eq!(aside.bank_installs, 1);
    }

    fn listed(hook: u64, owner: u64, code: u32) -> ListedHook {
        ListedHook { hook, owner, code }
    }

    #[test]
    fn a_cleared_mark_registers_hooks_and_activates_the_action_set() {
        let mut gate = MatchGate::playing();
        gate.focus = true;
        gate.session_mark = 1;
        gate.install_ready = 1;
        gate.install_first = 1;
        gate.install_records = vec![installed(42, false)];
        gate.draw_owner = 7;
        gate.draw_extra = 8;
        gate.gate_hook = 11;
        gate.self_owner = 3;
        gate.play_hook = 21;
        gate.play_owner = 4;
        gate.watch_hook = 22;
        gate.watch_owner = 5;
        gate.ordered_count = 1;
        gate.ordered = vec![listed(22, 5, HOOK_ORDER as u32)];
        let mut dying = hired_physician();
        assert!(gate.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut dying));
        assert_eq!(gate.primary, vec![listed(11, 3, HOOK_GATE)]);
        assert_eq!(gate.hook_high, 0);
        assert_eq!(gate.ordered, vec![listed(21, 4, HOOK_ORDER as u32)]);
        assert!(gate.activates_action_set);
        assert_eq!(gate.draw_value, 42);
        assert_eq!(dying.deaths, 1);

        let mut held = MatchGate::playing();
        held.focus = true;
        held.session_mark = 1;
        held.hold_byte = 1;
        held.gate_hook = 11;
        let mut company = Company::new("Ash");
        assert!(held.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut company));
        assert!(held.primary.is_empty());
        assert_eq!(held.hook_high, -1);

        let mut side = MatchGate::playing();
        side.focus = true;
        side.session_mark = 1;
        side.world_open = true;
        side.world_side = 5;
        side.wide_hook = 30;
        side.plain_hook = 31;
        side.lab_hook = 32;
        side.self_owner = 3;
        side.gate_hook = 11;
        let mut company = Company::new("Ash");
        assert!(side.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut company));
        assert_eq!(
            side.primary,
            vec![
                listed(30, 3, HOOK_WIDE),
                listed(30, 3, HOOK_SMALL | HOOK_LOW),
                listed(11, 3, HOOK_GATE),
            ]
        );

        let mut back = MatchGate::playing();
        back.bank_side = 0;
        back.plain_hook = 31;
        back.lab_hook = 32;
        back.gate_hook = 11;
        back.self_owner = 3;
        back.play_hook = 21;
        back.play_owner = 4;
        back.watch_hook = 22;
        back.watch_owner = 5;
        back.hook_high = 2;
        back.primary = vec![
            listed(31, 3, HOOK_WIDE),
            listed(32, 3, HOOK_SMALL | HOOK_LOW),
            listed(11, 3, HOOK_GATE),
        ];
        back.ordered_count = 1;
        back.ordered = vec![listed(21, 4, HOOK_ORDER as u32)];
        back.set_session_aside();
        assert!(back.primary.is_empty());
        assert_eq!(back.hook_high, -1);
        assert_eq!(back.ordered, vec![listed(22, 5, HOOK_ORDER as u32)]);
        assert!(!back.activates_action_set);
    }

    #[test]
    fn a_gate_hook_sets_its_flags_and_a_watch_hook_stamps() {
        let mut gate = MatchGate::playing();
        gate.gate_hook_out = 9;
        gate.gate_owner_out = 8;
        gate.gate_flags = 0;
        assert_eq!(gate.run_gate(), 1);
        assert_eq!(gate.gate_hook_out, 0);
        assert_eq!(gate.gate_owner_out, 0);
        assert_eq!(gate.gate_flags, 1);
        assert_eq!(gate.gate_alt_calls, 0);

        gate.side_mid = 1;
        gate.gate_alt = 4;
        assert_eq!(gate.run_gate(), 1);
        assert_eq!(gate.gate_alt_calls, 1);
        assert_eq!(gate.gate_flags, 1);

        let mut quiet = MatchGate::playing();
        quiet.watch_stamp = 9;
        assert_eq!(quiet.run_watch(), 1);
        assert_eq!(quiet.watch_stamp, 0);

        let mut held = MatchGate::playing();
        held.watch_flags = WATCH_BIT;
        held.watch_target = 1;
        held.watch_ready = 1;
        held.watch_clock = 12;
        held.watch_code = 1;
        held.target_flags = WATCH_MARK;
        held.watch_pending = 2;
        assert_eq!(held.run_watch(), 1);
        assert_eq!(held.watch_stamp, 12);
        assert_eq!(held.target_flags, 0);
        assert_eq!(held.watch_releases, 1);
        assert_eq!(held.watch_refreshes, 1);

        let mut pressed = MatchGate::playing();
        pressed.watch_flags = WATCH_BIT;
        pressed.watch_target = 1;
        pressed.watch_ready = 1;
        pressed.watch_held = 1;
        pressed.watch_stamp = 3;
        pressed.watch_code = 2;
        assert_eq!(pressed.run_watch(), 1);
        assert_eq!(pressed.watch_stamp, 3);
        assert_eq!(pressed.target_flags, WATCH_MARK);
        assert_eq!(pressed.watch_presses, 1);

        let mut leave = MatchGate::playing();
        leave.watch_leave = 1;
        leave.watch_focus = 1;
        leave.session_mark = 2;
        leave.gate_hook = 11;
        leave.self_owner = 3;
        assert_eq!(leave.run_watch(), 1);
        assert_eq!(leave.watch_focus, 0);
        assert_eq!(leave.focus_calls, 1);
        assert_eq!(leave.session_mark, 0);
        assert_eq!(leave.entered_code, ENTERED_CODE);
        assert_eq!(leave.primary, vec![listed(11, 3, HOOK_GATE)]);
    }
}
