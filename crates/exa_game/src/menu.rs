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
/// Mask and value that mark a picked item.
pub const PICK_CLASS: u32 = 0x7000;
pub const PICK_ITEM: u32 = 0x4000;
/// High byte of an actor, and the kind word of a container.
pub const PICK_ACTOR: u32 = 0x1a00_0000;
pub const PICK_KIND: u32 = 0xa00;
pub const PICK_SKIP: u32 = 0x800;
/// Record kinds that open the parent when the link is the world.
pub const ROW_OPEN: u32 = 0x10010a;
pub const ROW_OPEN_ALT: u32 = 0x10010d;
/// Mark a kind span must hold. The two high bits choose the table.
pub const KIND_MARK: u32 = 0x20000;
pub const KIND_HIGH: u32 = 0x8000_0000;
pub const KIND_WIDE: u32 = 0x4000_0000;
pub const KIND_MASK: u32 = 0x3fff_ffff;
/// Tag of a pane that already shows an item. A held byte of 1 closes.
pub const PANE_TAG: u32 = 0xce1c_2013;
pub const HELD_READY: u8 = 1;
/// A body is visited when these bits are all set, and skipped when this bit is set.
pub const BODY_SOLID: u32 = 0x89;
pub const BODY_SKIP: u32 = 0x0400_0000;
/// The ray starts this long. A closer hit shortens it.
pub const RAY_REACH: i32 = 4000;
/// Child flags used when a pane stores its size.
pub const CHILD_STRETCH_X: u32 = 0x30000;
pub const CHILD_ANCHOR_X: u32 = 0x20000;
pub const CHILD_STRETCH_Y: u32 = 0xc0000;
pub const CHILD_ANCHOR_Y: u32 = 0x80000;
pub const CHILD_CENTER_X: u32 = 0x800;
pub const CHILD_CENTER_Y: u32 = 0x8000;
/// Bits of 0.1 and of 5. The probe chooses one.
pub const PAD_NEAR: u32 = 0x3dcc_cccd;
pub const PAD_FAR: u32 = 0x40a0_0000;
/// A set bit chooses the wide slot.
pub const SPAN_WIDE: u32 = 0x1000;
/// Flags a taken item ORs on, and the flags a hover stores.
pub const PICK_FOLLOW: u32 = 5;
pub const CLICK_MARK: u32 = 0x11;
pub const HOVER_FLAGS: u32 = 0x15;
/// Bits of 70. A faster positive aim copies its place.
pub const AIM_SPEED: u32 = 0x428c_0000;
/// Bits of 80.
pub const AIM_FASTER: u32 = 0x42a0_0000;
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

/// One record on the actor the cursor hit. The bytes between the kind and the
/// link are not read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickRow {
    pub kind: u32,
    pub link: u64,
}

/// One sorted kind row. The key is the first word. The span starts at `offset`
/// in the table bytes and is `size` bytes long. A size of 0 is not read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KindEntry {
    pub key: u32,
    pub offset: u32,
    pub size: u32,
}

/// One kind table. The rows are already in key order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KindTable {
    pub entries: Vec<KindEntry>,
    pub blob: Vec<u8>,
}

/// One open pane. The first match of the tag and the item is shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KindPane {
    pub tag: u32,
    pub item: u32,
    pub id: u64,
}

/// One body in a sector. `hits` is the triangle count the ray test supplies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneBody {
    pub flags: u32,
    pub mask: u32,
    pub id: u64,
    pub hits: i32,
    pub probe: u8,
    /// Centre and radius. The ray runs from the origin along z.
    pub place: [i32; 3],
    pub radius: i32,
}

/// One child of a pane. The flags choose stretch, anchor, or centre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneChild {
    pub flags: u32,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// One sector. A mask that shares no bit with the camera is skipped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneSector {
    pub mask: u32,
    pub bodies: Vec<SceneBody>,
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
    /// Counted when the mid byte is set. A clear mid byte may pick.
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
    /// A set byte refuses the pick. A clear world also refuses it.
    pub pick_busy: u8,
    /// Child byte that lets a non-zero side still pick.
    pub world_aim: u8,
    pub pick_calls: u32,
    pub pick_hit: u64,
    pub hit_flags: u32,
    pub hit_taken: u8,
    pub hit_kind: u32,
    pub kind_plain: KindTable,
    pub kind_high: KindTable,
    pub kind_wide: KindTable,
    pub kind_calls: u32,
    pub held_count: i32,
    pub held_ptr: u64,
    pub held_byte: u8,
    pub held_drops: u32,
    pub held_closes: u32,
    pub cursor_x: i32,
    pub cursor_y: i32,
    pub view_w: i32,
    pub view_h: i32,
    pub panes: Vec<KindPane>,
    pub pane_shows: u32,
    pub pane_builds: u32,
    pub shown_pane: u64,
    pub sectors: Vec<SceneSector>,
    pub camera_mask: u32,
    pub scene_index: i32,
    pub scene_reach: i32,
    pub children: Vec<PaneChild>,
    pub scene_pads: Vec<u32>,
    pub span_w: u16,
    pub span_h: u16,
    pub span_flags: u32,
    pub slot_plain: u32,
    pub slot_wide: u32,
    pub row_spans: Vec<i32>,
    pub row_cap: i32,
    pub unit: i32,
    pub margin: i32,
    pub pane_pad: i32,
    pub screen_w: i32,
    pub order_base: i32,
    pub built_rows: i32,
    pub built_a: i32,
    pub built_b: i32,
    pub built_c: i32,
    pub built_w: i32,
    pub built_h: i32,
    pub built_x: i32,
    pub built_y: i32,
    pub built_order: i32,
    pub built_item: u32,
    pub hit_parent: u64,
    pub world_id: u64,
    pub hit_class: u32,
    pub hit_kind_word: u32,
    pub pick_extra: u64,
    pub hit_named: u8,
    pub pick_rows: Vec<PickRow>,
    pub open_calls: u32,
    pub chest_calls: u32,
    pub notice_owner: u64,
    pub aim_speed: u32,
    pub aim_place: u64,
    pub aim_source: u64,
    pub aim_copied: bool,
    pub aimed: u64,
    pub click_flags: u32,
    pub hover_byte: u8,
    pub hover_mark: u32,
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
            pick_busy: 0,
            world_aim: 0,
            pick_calls: 0,
            pick_hit: 0,
            hit_flags: 0,
            hit_taken: 0,
            hit_kind: 0,
            kind_plain: KindTable::default(),
            kind_high: KindTable::default(),
            kind_wide: KindTable::default(),
            kind_calls: 0,
            held_count: 0,
            held_ptr: 0,
            held_byte: 0,
            held_drops: 0,
            held_closes: 0,
            cursor_x: 0,
            cursor_y: 0,
            view_w: 1,
            view_h: 1,
            panes: Vec::new(),
            pane_shows: 0,
            pane_builds: 0,
            shown_pane: 0,
            sectors: Vec::new(),
            camera_mask: 0,
            scene_index: 0,
            scene_reach: 0,
            children: Vec::new(),
            scene_pads: Vec::new(),
            span_w: 0,
            span_h: 0,
            span_flags: 0,
            slot_plain: 0,
            slot_wide: 0,
            row_spans: Vec::new(),
            row_cap: 0,
            unit: 0,
            margin: 0,
            pane_pad: 0,
            screen_w: 0,
            order_base: 0,
            built_rows: 0,
            built_a: 0,
            built_b: 0,
            built_c: 0,
            built_w: 0,
            built_h: 0,
            built_x: 0,
            built_y: 0,
            built_order: 0,
            built_item: 0,
            hit_parent: 0,
            world_id: 0,
            hit_class: 0,
            hit_kind_word: 0,
            pick_extra: 0,
            hit_named: 0,
            pick_rows: Vec::new(),
            open_calls: 0,
            chest_calls: 0,
            notice_owner: 0,
            aim_speed: 0,
            aim_place: 0,
            aim_source: 0,
            aim_copied: false,
            aimed: 0,
            click_flags: 0,
            hover_byte: 0,
            hover_mark: 0,
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
    /// byte counts the alternate hook and does not pick.
    pub fn run_gate(&mut self) -> u32 {
        self.gate_hook_out = 0;
        self.gate_owner_out = 0;
        self.gate_flags = 1;
        if self.side_mid != 0 {
            if self.gate_alt != 0 {
                self.gate_alt_calls = self.gate_alt_calls.saturating_add(1);
            }
            return self.gate_flags;
        }
        let side_blocks = self.bank_side != 0 && self.world_aim == 0;
        if self.pick_busy != 0
            || self.gate_alt != 0
            || self.hold_byte != 0
            || !self.world_open
            || side_blocks
        {
            return self.gate_flags;
        }
        self.pick_calls = self.pick_calls.saturating_add(1);
        if !self.point_inside() {
            self.pick_hit = 0;
            return self.gate_flags;
        }
        if !self.sectors.is_empty() {
            self.walk_scene();
        }
        if self.pick_hit == 0 {
            return self.gate_flags;
        }
        if (self.hit_flags & PICK_CLASS) == PICK_ITEM {
            if self.hit_taken != 0 {
                self.gate_hook_out = 1;
                self.gate_owner_out = self.notice_owner;
                self.gate_flags |= PICK_FOLLOW;
                return self.gate_flags;
            }
            if self.kind_marked(self.hit_kind) {
                self.kind_calls = self.kind_calls.saturating_add(1);
                self.release_held();
                self.open_marked(self.hit_kind);
                return self.gate_flags;
            }
        }
        let actor = self.hit_parent != 0 && (self.hit_class & 0xff00_0000) == PICK_ACTOR;
        if actor && self.hit_parent == self.world_id {
            self.open_calls = self.open_calls.saturating_add(1);
        } else if actor && self.hit_kind_word == PICK_KIND {
            self.chest_calls = self.chest_calls.saturating_add(1);
        } else if actor
            && (self.hit_kind_word & PICK_SKIP) == 0
            && self.pick_extra == 0
            && (self.hit_named != 0 || self.row_opens())
        {
            self.open_calls = self.open_calls.saturating_add(1);
        }
        self.gate_flags
    }

    /// A cursor inside the view keeps a supplied hit. Outside, the hit is cleared.
    /// The scene walk is not in.
    fn point_inside(&self) -> bool {
        self.cursor_x >= 0
            && self.cursor_y >= 0
            && self.cursor_x < self.view_w
            && self.cursor_y < self.view_h
    }

    /// A count above 0 drops and becomes 0. A count of 0 or less clears a
    /// pointer, and a ready byte counts a close first.
    fn release_held(&mut self) {
        if self.held_count > 0 {
            self.held_drops = self.held_drops.saturating_add(1);
            self.held_count = 0;
            return;
        }
        if self.held_ptr == 0 {
            return;
        }
        if self.held_byte == HELD_READY {
            self.held_closes = self.held_closes.saturating_add(1);
        }
        self.held_ptr = 0;
    }

    /// The first pane with the tag and this item is shown. An id of 0 does
    /// nothing. With no pane, a marked span counts a build. The builder is not in.
    fn open_marked(&mut self, id: u32) {
        if id == 0 {
            return;
        }
        let shown = self
            .panes
            .iter()
            .find(|pane| pane.tag == PANE_TAG && pane.item == id)
            .map(|pane| pane.id);
        if let Some(shown) = shown {
            self.pane_shows = self.pane_shows.saturating_add(1);
            self.shown_pane = shown;
            return;
        }
        if self.kind_marked(id) {
            self.build_pane(id);
        }
    }

    /// Sectors that share a camera bit, then bodies that are solid, unskipped,
    /// and in that mask. The ray starts at length 4000 along z. A body past
    /// that length is not taken. A closer hit shortens the ray. An empty scene
    /// leaves the supplied hit.
    fn walk_scene(&mut self) {
        self.pick_hit = 0;
        self.scene_index = 0;
        self.scene_reach = RAY_REACH;
        self.scene_pads.clear();
        let sectors = self.sectors.clone();
        for sector in &sectors {
            if self.camera_mask & sector.mask == 0 {
                continue;
            }
            for body in &sector.bodies {
                if (body.flags & BODY_SOLID) != BODY_SOLID {
                    continue;
                }
                if self.camera_mask & body.mask == 0 {
                    continue;
                }
                if (body.flags & BODY_SKIP) == BODY_SKIP {
                    continue;
                }
                let pad = if body.probe == 0 { PAD_NEAR } else { PAD_FAR };
                self.scene_pads.push(pad);
                let reach = self.scene_reach as f32;
                if body.hits > 0 && segment_meets(reach, body.place, body.radius) {
                    self.pick_hit = body.id;
                    self.scene_index = body.hits - 1;
                    self.scene_reach = along_z(reach, body.place) as i32;
                }
            }
        }
    }

    /// The doubled row is capped. The inner sizes use the unit, the margin, and
    /// the two span words. An empty pane stores the outer width and height, and
    /// the place is clamped. Child layout is not in.
    fn build_pane(&mut self, id: u32) {
        self.pane_builds = self.pane_builds.saturating_add(1);
        let slot = if self.span_flags & SPAN_WIDE != 0 {
            self.slot_wide
        } else {
            self.slot_plain
        };
        let span = self.row_span(slot);
        let mut rows = span.wrapping_shl(1);
        if rows > self.row_cap {
            rows = self.row_cap;
        }
        let inner_a = self.unit.wrapping_mul(6).wrapping_add(rows);
        let inner_b = self
            .margin
            .wrapping_shl(1)
            .wrapping_add(i32::from(self.span_w).wrapping_mul(self.unit));
        let inner_c = self
            .margin
            .wrapping_shl(1)
            .wrapping_add(i32::from(self.span_h).wrapping_mul(self.unit));
        let width = self.pane_pad.wrapping_shl(1).wrapping_add(inner_b);
        let height = self
            .unit
            .wrapping_shl(1)
            .wrapping_add(inner_a)
            .wrapping_add(self.pane_pad)
            .wrapping_add(inner_c);
        let half = (width as u32 >> 1) as i32;
        let mut place_x = self.cursor_x.wrapping_sub(half);
        if place_x < 0 {
            place_x = 0;
        } else if i64::from(width) + i64::from(place_x) > i64::from(self.screen_w) {
            place_x = self.screen_w.wrapping_sub(width);
        }
        let mut place_y = self
            .cursor_y
            .wrapping_sub(height)
            .wrapping_add(self.unit.wrapping_shl(3));
        if place_y < self.unit {
            place_y = self.unit;
        }
        self.lay_out(width, height);
        self.built_rows = rows;
        self.built_a = inner_a;
        self.built_b = inner_b;
        self.built_c = inner_c;
        self.built_w = width;
        self.built_h = height;
        self.built_x = place_x;
        self.built_y = place_y;
        self.built_order = self.order_base.wrapping_add(0x100);
        self.built_item = id;
    }

    /// A stretch child grows by the change in the pane. An anchor moves by that
    /// change. A centre child is placed in the spare half. Nested children are
    /// not visited.
    fn lay_out(&mut self, width: i32, height: i32) {
        let old_w = self.built_w;
        let old_h = self.built_h;
        for child in &mut self.children {
            let stretch_x = (child.flags & CHILD_STRETCH_X) == CHILD_STRETCH_X;
            let stretch_y = (child.flags & CHILD_STRETCH_Y) == CHILD_STRETCH_Y;
            if stretch_x {
                child.w = child.w.wrapping_add(width).wrapping_sub(old_w);
            } else if (child.flags & CHILD_ANCHOR_X) == CHILD_ANCHOR_X {
                child.x = child.x.wrapping_add(width).wrapping_sub(old_w);
            }
            if stretch_y {
                child.h = child.h.wrapping_add(height).wrapping_sub(old_h);
            } else if (child.flags & CHILD_ANCHOR_Y) == CHILD_ANCHOR_Y {
                child.y = child.y.wrapping_add(height).wrapping_sub(old_h);
            }
            if !stretch_x && (child.flags & CHILD_CENTER_X) == CHILD_CENTER_X {
                child.x = ((i64::from(width) - i64::from(child.w)) / 2) as i32;
            }
            if !stretch_y && (child.flags & CHILD_CENTER_Y) == CHILD_CENTER_Y {
                child.y = ((i64::from(height) - i64::from(child.h)) / 2) as i32;
            }
        }
    }

    fn row_span(&self, index: u32) -> i32 {
        let Some(place) = index.checked_sub(1) else {
            return 0;
        };
        self.row_spans.get(place as usize).copied().unwrap_or(0)
    }

    /// An item id of 0 misses. Bit 0x80000000 searches the high table, else bit
    /// 0x40000000 searches the wide table, else the plain table. A high or wide
    /// id drops those bits. A row is a hit when its span holds the mark.
    fn kind_marked(&self, id: u32) -> bool {
        if id == 0 {
            return false;
        }
        let (table, key) = if id & KIND_HIGH != 0 {
            (&self.kind_high, id & KIND_MASK)
        } else if id & KIND_WIDE != 0 {
            (&self.kind_wide, id & KIND_MASK)
        } else {
            (&self.kind_plain, id)
        };
        let Some(index) = find_kind(&table.entries, key) else {
            return false;
        };
        let entry = &table.entries[index];
        entry.size != 0 && span_has_mark(&table.blob, entry.offset, entry.size)
    }

    /// The first record whose kind is one of the two open kinds and whose link
    /// is the world. A later record is not visited.
    fn row_opens(&self) -> bool {
        self.pick_rows.iter().any(|row| {
            (row.kind == ROW_OPEN || row.kind == ROW_OPEN_ALT) && row.link == self.world_id
        })
    }

    /// The click hook. A fast world aim copies its place. A miss with the hold
    /// byte set clears the aimed word.
    pub fn run_click(&mut self) -> u32 {
        self.gate_hook_out = 0;
        self.gate_owner_out = 0;
        self.click_flags = 0;
        if self.world_open && self.aim_speed > AIM_SPEED {
            self.aim_place = self.aim_source;
            self.aim_copied = true;
        }
        if self.pick_hit == 0 {
            if self.hold_byte != 0 {
                self.aimed = 0;
            }
            return self.click_flags;
        }
        if (self.hit_flags & PICK_CLASS) == PICK_ITEM && self.hit_taken != 0 {
            self.click_flags |= CLICK_MARK;
        }
        self.click_flags
    }

    /// The hover hook. It publishes a follow hook, clears a byte and a mark,
    /// and stores the flags 0x15.
    pub fn run_hover(&mut self) -> u32 {
        self.gate_hook_out = 1;
        self.gate_owner_out = self.self_owner;
        self.hover_byte = 0;
        self.hover_mark = 0;
        self.gate_flags = HOVER_FLAGS;
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

/// Signed binary search. The rows are 16 bytes apart and the key is the first
/// word. A missing key, or a count that does not fit, is absent.
fn find_kind(entries: &[KindEntry], key: u32) -> Option<usize> {
    let len = i32::try_from(entries.len()).ok()?;
    let mut low = 0i32;
    let mut high = len - 1;
    let wanted = key as i32;
    while low <= high {
        let diff = i64::from(high) - i64::from(low);
        let sign = ((diff as u64) >> 63) as i64;
        let mid = (i64::from(low) + ((diff + sign) >> 1)) as i32;
        let stored = entries[mid as usize].key as i32;
        if wanted < stored {
            high = mid - 1;
        } else if wanted > stored {
            low = mid + 1;
        } else {
            return Some(mid as usize);
        }
    }
    None
}

/// The span holds the mark when a record's first word matches. The next record
/// starts `step` bytes later. A step of 0, or a read past the bytes, stops.
fn span_has_mark(blob: &[u8], offset: u32, size: u32) -> bool {
    let start = offset as usize;
    let end = start.saturating_add(size as usize);
    if end > blob.len() {
        return false;
    }
    let mut cursor = start;
    while end > cursor {
        if cursor.saturating_add(8) > blob.len() {
            return false;
        }
        let tag = read_u32(blob, cursor);
        let step = read_u32(blob, cursor + 4);
        if tag == KIND_MARK {
            return true;
        }
        if step == 0 {
            return false;
        }
        cursor = cursor.saturating_add(step as usize);
    }
    false
}

/// The ray is `(0, 0, reach)`. A centre past that end is rejected unless the
/// end still lies in the radius.
fn segment_meets(reach: f32, place: [i32; 3], radius: i32) -> bool {
    let delta = [place[0] as f32, place[1] as f32, place[2] as f32];
    let dir = [0.0, 0.0, reach];
    let dot = dir[2] * delta[2];
    let radius2 = (radius as f32) * (radius as f32);
    if dot <= 0.0 {
        return delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2] <= radius2;
    }
    let dir2 = dir[2] * dir[2];
    if dir2 <= dot {
        let end = delta[2] - reach;
        return delta[0] * delta[0] + delta[1] * delta[1] + end * end <= radius2;
    }
    let nearest =
        delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2] - (dot * dot) / dir2;
    nearest <= radius2
}

fn along_z(reach: f32, place: [i32; 3]) -> f32 {
    let z = place[2] as f32;
    if z <= 0.0 {
        0.0
    } else if z >= reach {
        reach
    } else {
        z
    }
}

fn read_u32(blob: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([blob[at], blob[at + 1], blob[at + 2], blob[at + 3]])
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

    fn le_pair(tag: u32, step: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&tag.to_le_bytes());
        bytes.extend_from_slice(&step.to_le_bytes());
        bytes
    }

    fn marked_kind(key: u32) -> KindTable {
        let blob = le_pair(KIND_MARK, 8);
        KindTable {
            entries: vec![KindEntry {
                key,
                offset: 0,
                size: blob.len() as u32,
            }],
            blob,
        }
    }

    fn plain_item(id: u32) -> MatchGate {
        let mut gate = MatchGate::playing();
        gate.world_open = true;
        gate.pick_hit = 1;
        gate.hit_flags = PICK_ITEM;
        gate.hit_kind = id;
        gate
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

    #[test]
    fn a_clear_gate_picks_and_a_click_and_hover_publish() {
        let mut taken = MatchGate::playing();
        taken.world_open = true;
        taken.pick_hit = 1;
        taken.hit_flags = PICK_ITEM;
        taken.hit_taken = 1;
        taken.notice_owner = 6;
        assert_eq!(taken.run_gate(), 1 | PICK_FOLLOW);
        assert_eq!(taken.pick_calls, 1);
        assert_eq!(taken.gate_hook_out, 1);
        assert_eq!(taken.gate_owner_out, 6);

        let mut held = MatchGate::playing();
        held.world_open = true;
        held.hold_byte = 1;
        held.pick_hit = 1;
        assert_eq!(held.run_gate(), 1);
        assert_eq!(held.pick_calls, 0);

        let mut actor = MatchGate::playing();
        actor.world_open = true;
        actor.world_id = 4;
        actor.pick_hit = 1;
        actor.hit_parent = 4;
        actor.hit_class = PICK_ACTOR;
        assert_eq!(actor.run_gate(), 1);
        assert_eq!(actor.open_calls, 1);

        let mut busy = MatchGate::playing();
        busy.world_open = true;
        busy.pick_busy = 1;
        busy.pick_hit = 1;
        assert_eq!(busy.run_gate(), 1);
        assert_eq!(busy.pick_calls, 0);

        let mut kind = MatchGate::playing();
        kind.world_open = true;
        kind.pick_hit = 1;
        kind.hit_flags = PICK_ITEM;
        kind.hit_kind = 1;
        kind.kind_plain = marked_kind(1);
        assert_eq!(kind.run_gate(), 1);
        assert_eq!(kind.kind_calls, 1);
        assert_eq!(kind.pick_calls, 1);

        let mut chest = MatchGate::playing();
        chest.world_open = true;
        chest.pick_hit = 1;
        chest.hit_parent = 2;
        chest.world_id = 4;
        chest.hit_class = PICK_ACTOR;
        chest.hit_kind_word = PICK_KIND;
        assert_eq!(chest.run_gate(), 1);
        assert_eq!(chest.chest_calls, 1);

        let mut named = MatchGate::playing();
        named.world_open = true;
        named.pick_hit = 1;
        named.hit_parent = 2;
        named.hit_class = PICK_ACTOR;
        named.hit_named = 1;
        assert_eq!(named.run_gate(), 1);
        assert_eq!(named.open_calls, 1);

        let mut skipped = MatchGate::playing();
        skipped.world_open = true;
        skipped.pick_hit = 1;
        skipped.hit_parent = 2;
        skipped.hit_class = PICK_ACTOR;
        skipped.hit_kind_word = PICK_SKIP;
        skipped.hit_named = 1;
        assert_eq!(skipped.run_gate(), 1);
        assert_eq!(skipped.open_calls, 0);

        let mut sided = MatchGate::playing();
        sided.world_open = true;
        sided.bank_side = 1;
        sided.pick_hit = 1;
        assert_eq!(sided.run_gate(), 1);
        assert_eq!(sided.pick_calls, 0);
        sided.world_aim = 1;
        assert_eq!(sided.run_gate(), 1);
        assert_eq!(sided.pick_calls, 1);

        let mut click = MatchGate::playing();
        click.world_open = true;
        click.aim_speed = AIM_FASTER;
        click.aim_source = 9;
        click.hold_byte = 1;
        click.aimed = 3;
        assert_eq!(click.run_click(), 0);
        assert!(click.aim_copied);
        assert_eq!(click.aim_place, 9);
        assert_eq!(click.aimed, 0);

        click.aim_copied = false;
        click.aim_speed = AIM_SPEED;
        click.pick_hit = 1;
        click.hit_flags = PICK_ITEM;
        click.hit_taken = 1;
        assert_eq!(click.run_click(), CLICK_MARK);
        assert!(!click.aim_copied);

        let mut hover = MatchGate::playing();
        hover.self_owner = 3;
        hover.hover_byte = 4;
        hover.hover_mark = 6;
        assert_eq!(hover.run_hover(), HOVER_FLAGS);
        assert_eq!(hover.gate_hook_out, 1);
        assert_eq!(hover.gate_owner_out, 3);
        assert_eq!(hover.hover_byte, 0);
        assert_eq!(hover.hover_mark, 0);

        let mut focus = MatchGate::playing();
        focus.focus = true;
        focus.hands_ready = true;
        focus.clock = 5;
        let mut company = Company::new("Ash");
        assert!(focus.begin_match(IN_GAME_PAGE, BEGIN_COMMAND, PlayKind::Arena, &mut company));
        assert_eq!(focus.focus_calls, 1);
        assert!(focus.hands_set);
        assert_eq!(focus.stamp, 5);
    }

    #[test]
    fn a_row_opens_when_its_link_is_the_world() {
        let mut first = MatchGate::playing();
        first.world_open = true;
        first.pick_hit = 1;
        first.hit_parent = 2;
        first.world_id = 4;
        first.hit_class = PICK_ACTOR;
        first.pick_rows = vec![
            PickRow { kind: 1, link: 4 },
            PickRow {
                kind: ROW_OPEN,
                link: 4,
            },
            PickRow {
                kind: ROW_OPEN,
                link: 4,
            },
        ];
        assert_eq!(first.run_gate(), 1);
        assert_eq!(first.open_calls, 1);

        let mut other = MatchGate::playing();
        other.world_open = true;
        other.pick_hit = 1;
        other.hit_parent = 2;
        other.world_id = 4;
        other.hit_class = PICK_ACTOR;
        other.pick_rows = vec![PickRow {
            kind: ROW_OPEN_ALT,
            link: 9,
        }];
        assert_eq!(other.run_gate(), 1);
        assert_eq!(other.open_calls, 0);
        other.pick_rows[0].link = 4;
        assert_eq!(other.run_gate(), 1);
        assert_eq!(other.open_calls, 1);

        let mut named = MatchGate::playing();
        named.world_open = true;
        named.pick_hit = 1;
        named.hit_parent = 2;
        named.hit_class = PICK_ACTOR;
        named.hit_named = 1;
        named.pick_rows = vec![PickRow { kind: 1, link: 0 }];
        assert_eq!(named.run_gate(), 1);
        assert_eq!(named.open_calls, 1);
    }

    #[test]
    fn a_kind_mark_is_found_in_the_chosen_table() {
        let mut zero = plain_item(0);
        zero.kind_plain = marked_kind(0);
        assert_eq!(zero.run_gate(), 1);
        assert_eq!(zero.kind_calls, 0);

        let mut plain = plain_item(7);
        plain.kind_plain = marked_kind(7);
        assert_eq!(plain.run_gate(), 1);
        assert_eq!(plain.kind_calls, 1);
        assert_eq!(plain.gate_flags, 1);

        let mut bare = plain_item(7);
        bare.kind_plain = KindTable {
            entries: vec![KindEntry {
                key: 7,
                offset: 0,
                size: 8,
            }],
            blob: le_pair(1, 8),
        };
        assert_eq!(bare.run_gate(), 1);
        assert_eq!(bare.kind_calls, 0);

        let mut high = plain_item(KIND_HIGH | 5);
        high.kind_plain = KindTable {
            entries: vec![KindEntry {
                key: 5,
                offset: 0,
                size: 8,
            }],
            blob: le_pair(1, 8),
        };
        high.kind_high = marked_kind(5);
        assert_eq!(high.run_gate(), 1);
        assert_eq!(high.kind_calls, 1);

        let mut wide = plain_item(KIND_WIDE | 9);
        wide.kind_plain = marked_kind(KIND_WIDE | 9);
        wide.kind_wide = KindTable {
            entries: vec![KindEntry {
                key: 9,
                offset: 0,
                size: 8,
            }],
            blob: le_pair(1, 8),
        };
        assert_eq!(wide.run_gate(), 1);
        assert_eq!(wide.kind_calls, 0);
        wide.kind_wide = marked_kind(9);
        wide.kind_calls = 0;
        assert_eq!(wide.run_gate(), 1);
        assert_eq!(wide.kind_calls, 1);

        let mut both = plain_item(KIND_HIGH | KIND_WIDE | 5);
        both.kind_high = KindTable {
            entries: vec![KindEntry {
                key: 5,
                offset: 0,
                size: 8,
            }],
            blob: le_pair(1, 8),
        };
        both.kind_wide = marked_kind(5);
        assert_eq!(both.run_gate(), 1);
        assert_eq!(both.kind_calls, 0);

        let mut middle = plain_item(7);
        let blob = le_pair(KIND_MARK, 8);
        middle.kind_plain = KindTable {
            entries: vec![
                KindEntry {
                    key: 2,
                    offset: 0,
                    size: 0,
                },
                KindEntry {
                    key: 7,
                    offset: 0,
                    size: blob.len() as u32,
                },
                KindEntry {
                    key: 9,
                    offset: 0,
                    size: 0,
                },
            ],
            blob,
        };
        assert_eq!(middle.run_gate(), 1);
        assert_eq!(middle.kind_calls, 1);
        middle.hit_kind = 8;
        middle.kind_calls = 0;
        assert_eq!(middle.run_gate(), 1);
        assert_eq!(middle.kind_calls, 0);

        let mut later = plain_item(4);
        let mut span = le_pair(3, 8);
        span.extend_from_slice(&le_pair(KIND_MARK, 8));
        later.kind_plain = KindTable {
            entries: vec![KindEntry {
                key: 4,
                offset: 0,
                size: span.len() as u32,
            }],
            blob: span,
        };
        assert_eq!(later.run_gate(), 1);
        assert_eq!(later.kind_calls, 1);

        let mut empty_size = plain_item(4);
        empty_size.kind_plain = KindTable {
            entries: vec![KindEntry {
                key: 4,
                offset: 0,
                size: 0,
            }],
            blob: le_pair(KIND_MARK, 8),
        };
        assert_eq!(empty_size.run_gate(), 1);
        assert_eq!(empty_size.kind_calls, 0);

        let mut short = plain_item(4);
        short.kind_plain = KindTable {
            entries: vec![KindEntry {
                key: 4,
                offset: 0,
                size: 8,
            }],
            blob: Vec::new(),
        };
        assert_eq!(short.run_gate(), 1);
        assert_eq!(short.kind_calls, 0);

        let mut stuck = plain_item(4);
        stuck.kind_plain = KindTable {
            entries: vec![KindEntry {
                key: 4,
                offset: 0,
                size: 8,
            }],
            blob: le_pair(1, 0),
        };
        assert_eq!(stuck.run_gate(), 1);
        assert_eq!(stuck.kind_calls, 0);

        let mut taken = plain_item(7);
        taken.hit_taken = 1;
        taken.notice_owner = 6;
        taken.kind_plain = marked_kind(7);
        assert_eq!(taken.run_gate(), 1 | PICK_FOLLOW);
        assert_eq!(taken.kind_calls, 0);
    }

    #[test]
    fn a_point_outside_misses_and_a_hit_releases_then_opens() {
        let mut miss = plain_item(7);
        miss.view_w = 8;
        miss.view_h = 8;
        miss.cursor_x = 8;
        miss.kind_plain = marked_kind(7);
        miss.held_count = 2;
        assert_eq!(miss.run_gate(), 1);
        assert_eq!(miss.pick_calls, 1);
        assert_eq!(miss.pick_hit, 0);
        assert_eq!(miss.kind_calls, 0);
        assert_eq!(miss.held_count, 2);
        assert_eq!(miss.held_drops, 0);

        let mut above = plain_item(7);
        above.view_w = 8;
        above.view_h = 8;
        above.cursor_y = -1;
        above.kind_plain = marked_kind(7);
        assert_eq!(above.run_gate(), 1);
        assert_eq!(above.kind_calls, 0);

        let mut drop = plain_item(7);
        drop.kind_plain = marked_kind(7);
        drop.held_count = 3;
        drop.held_ptr = 9;
        assert_eq!(drop.run_gate(), 1);
        assert_eq!(drop.kind_calls, 1);
        assert_eq!(drop.held_count, 0);
        assert_eq!(drop.held_drops, 1);
        assert_eq!(drop.held_ptr, 9);
        assert_eq!(drop.pane_builds, 1);
        assert_eq!(drop.pane_shows, 0);

        let mut close = plain_item(7);
        close.kind_plain = marked_kind(7);
        close.held_count = 0;
        close.held_ptr = 4;
        close.held_byte = HELD_READY;
        assert_eq!(close.run_gate(), 1);
        assert_eq!(close.held_closes, 1);
        assert_eq!(close.held_ptr, 0);
        assert_eq!(close.held_drops, 0);

        let mut quiet = plain_item(7);
        quiet.kind_plain = marked_kind(7);
        quiet.held_count = -1;
        quiet.held_ptr = 4;
        quiet.held_byte = 2;
        assert_eq!(quiet.run_gate(), 1);
        assert_eq!(quiet.held_closes, 0);
        assert_eq!(quiet.held_ptr, 0);

        let mut shown = plain_item(7);
        shown.view_w = 8;
        shown.view_h = 8;
        shown.cursor_x = 7;
        shown.cursor_y = 7;
        shown.kind_plain = marked_kind(7);
        shown.panes = vec![
            KindPane {
                tag: 1,
                item: 7,
                id: 2,
            },
            KindPane {
                tag: PANE_TAG,
                item: 7,
                id: 11,
            },
            KindPane {
                tag: PANE_TAG,
                item: 7,
                id: 12,
            },
        ];
        assert_eq!(shown.run_gate(), 1);
        assert_eq!(shown.kind_calls, 1);
        assert_eq!(shown.pane_shows, 1);
        assert_eq!(shown.shown_pane, 11);
        assert_eq!(shown.pane_builds, 0);
    }

    #[test]
    fn a_scene_walk_picks_the_last_body_and_a_pane_is_sized() {
        let mut scene = plain_item(7);
        scene.view_w = 8;
        scene.view_h = 8;
        scene.camera_mask = 1;
        scene.pick_hit = 99;
        scene.sectors = vec![
            SceneSector {
                mask: 2,
                bodies: vec![SceneBody {
                    flags: BODY_SOLID,
                    mask: 2,
                    id: 4,
                    hits: 5,
                    probe: 0,
                    place: [0, 0, 0],
                    radius: 0,
                }],
            },
            SceneSector {
                mask: 1,
                bodies: vec![
                    SceneBody {
                        flags: BODY_SOLID | BODY_SKIP,
                        mask: 1,
                        id: 5,
                        hits: 4,
                        probe: 0,
                        place: [0, 0, 0],
                        radius: 0,
                    },
                    SceneBody {
                        flags: BODY_SOLID,
                        mask: 1,
                        id: 8,
                        hits: 3,
                        probe: 1,
                        place: [0, 0, 0],
                        radius: 0,
                    },
                    SceneBody {
                        flags: 0x01,
                        mask: 1,
                        id: 9,
                        hits: 9,
                        probe: 0,
                        place: [0, 0, 0],
                        radius: 0,
                    },
                    SceneBody {
                        flags: BODY_SOLID,
                        mask: 1,
                        id: 15,
                        hits: 2,
                        probe: 0,
                        place: [0, 0, 0],
                        radius: 0,
                    },
                ],
            },
        ];
        assert_eq!(scene.run_gate(), 1);
        assert_eq!(scene.pick_hit, 15);
        assert_eq!(scene.scene_index, 1);
        assert_eq!(scene.scene_pads, vec![PAD_FAR, PAD_NEAR]);

        let mut built = plain_item(7);
        built.kind_plain = marked_kind(7);
        built.view_w = 400;
        built.view_h = 400;
        built.cursor_x = 100;
        built.cursor_y = 80;
        built.span_w = 3;
        built.span_h = 4;
        built.unit = 2;
        built.margin = 5;
        built.pane_pad = 6;
        built.row_spans = vec![9];
        built.row_cap = 100;
        built.slot_plain = 1;
        built.screen_w = 200;
        built.order_base = 10;
        assert_eq!(built.run_gate(), 1);
        assert_eq!(built.pane_builds, 1);
        assert_eq!(built.built_rows, 18);
        assert_eq!(built.built_a, 30);
        assert_eq!(built.built_b, 16);
        assert_eq!(built.built_c, 18);
        assert_eq!(built.built_w, 28);
        assert_eq!(built.built_h, 58);
        assert_eq!(built.built_x, 86);
        assert_eq!(built.built_y, 38);
        assert_eq!(built.built_order, 0x10a);
        assert_eq!(built.built_item, 7);

        let mut wide = plain_item(7);
        wide.kind_plain = marked_kind(7);
        wide.view_w = 400;
        wide.view_h = 400;
        wide.cursor_x = 0;
        wide.span_flags = SPAN_WIDE;
        wide.slot_plain = 1;
        wide.slot_wide = 2;
        wide.row_spans = vec![9, 4];
        wide.row_cap = 10;
        wide.unit = 2;
        wide.screen_w = 20;
        assert_eq!(wide.run_gate(), 1);
        assert_eq!(wide.built_rows, 8);
        assert_eq!(wide.built_x, 0);
        assert_eq!(wide.built_y, 2);

        let mut edge = plain_item(7);
        edge.kind_plain = marked_kind(7);
        edge.view_w = 400;
        edge.view_h = 400;
        edge.cursor_x = 30;
        edge.span_w = 3;
        edge.span_h = 4;
        edge.unit = 2;
        edge.margin = 5;
        edge.pane_pad = 6;
        edge.row_spans = vec![9];
        edge.row_cap = 100;
        edge.slot_plain = 1;
        edge.screen_w = 20;
        assert_eq!(edge.run_gate(), 1);
        assert_eq!(edge.built_w, 28);
        assert_eq!(edge.built_x, -8);
    }

    #[test]
    fn a_farther_body_is_rejected_and_a_child_is_laid_out() {
        let mut near = plain_item(7);
        near.view_w = 8;
        near.view_h = 8;
        near.camera_mask = 1;
        near.sectors = vec![SceneSector {
            mask: 1,
            bodies: vec![
                SceneBody {
                    flags: BODY_SOLID,
                    mask: 1,
                    id: 3,
                    hits: 2,
                    probe: 0,
                    place: [0, 0, 100],
                    radius: 1,
                },
                SceneBody {
                    flags: BODY_SOLID,
                    mask: 1,
                    id: 4,
                    hits: 2,
                    probe: 0,
                    place: [0, 0, 500],
                    radius: 1,
                },
            ],
        }];
        assert_eq!(near.run_gate(), 1);
        assert_eq!(near.pick_hit, 3);
        assert_eq!(near.scene_index, 1);
        assert_eq!(near.scene_reach, 100);

        let mut beyond = plain_item(7);
        beyond.view_w = 8;
        beyond.view_h = 8;
        beyond.camera_mask = 1;
        beyond.pick_hit = 9;
        beyond.sectors = vec![SceneSector {
            mask: 1,
            bodies: vec![SceneBody {
                flags: BODY_SOLID,
                mask: 1,
                id: 5,
                hits: 3,
                probe: 0,
                place: [0, 0, 5000],
                radius: 1,
            }],
        }];
        assert_eq!(beyond.run_gate(), 1);
        assert_eq!(beyond.pick_hit, 0);
        assert_eq!(beyond.scene_reach, RAY_REACH);

        let mut laid = plain_item(7);
        laid.kind_plain = marked_kind(7);
        laid.view_w = 400;
        laid.view_h = 400;
        laid.cursor_x = 100;
        laid.cursor_y = 80;
        laid.span_w = 3;
        laid.span_h = 4;
        laid.unit = 2;
        laid.margin = 5;
        laid.pane_pad = 6;
        laid.row_spans = vec![9];
        laid.row_cap = 100;
        laid.slot_plain = 1;
        laid.screen_w = 200;
        laid.children = vec![
            PaneChild {
                flags: CHILD_CENTER_X | CHILD_CENTER_Y,
                x: 1,
                y: 1,
                w: 10,
                h: 4,
            },
            PaneChild {
                flags: CHILD_ANCHOR_X,
                x: 4,
                y: 2,
                w: 6,
                h: 6,
            },
            PaneChild {
                flags: CHILD_STRETCH_X,
                x: 0,
                y: 0,
                w: 10,
                h: 8,
            },
        ];
        assert_eq!(laid.run_gate(), 1);
        assert_eq!(laid.children[0].x, 9);
        assert_eq!(laid.children[0].y, 27);
        assert_eq!(laid.children[1].x, 32);
        assert_eq!(laid.children[2].w, 38);
    }
}
