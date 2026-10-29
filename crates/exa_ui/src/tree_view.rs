//! The tree view (`TGUITreeView`): nodes listed in pre-order, one row
//! each, every level indented by an icon's width. Expansion is by
//! presence: a node with children shows them all and a down chevron; a
//! leaf that has had children shows a right chevron. Expanding a leaf asks
//! the owner to add its children; collapsing a node releases them. The
//! pointer selects the row under it (a press on the selected row expands
//! or collapses it, a press on a chevron always does), the arrows walk the
//! rows (Left goes to the parent or collapses, Right expands or steps
//! down), Enter toggles, F2 renames in place and a drag carries a node.
//! In popup mode the row under the pointer is selected and a press picks
//! it and ends the popup; that mode has no blank row after the last node.
//!
//! Only the dormant editor's open and save dialog creates a tree view; the
//! retail game never does. This type has no live caller in Exarust and is
//! kept, tested, for completeness. Rules: `docs/UI.md` (Tree views).

use crate::form::{ClientRect, set_scroll_y};
use crate::gui_font::{GuiFont, round_i32};
use crate::gui_text::{ellipsis_fit, measure};
use crate::popup_menu::{GlyphInk, TextInk};
use crate::skin::{PaletteInk, SkinInk, SkinPalette};
use crate::text_edit::{self, EditAction, EditView};

/// The border the constructor sets.
pub const BORDER: i32 = 3;
/// The style bits the constructor adds.
pub const STYLE: u32 = 0x88;
/// The attribute bits the constructor adds (snapped scrolling among them).
pub const ATTRIBUTES: u32 = 0x490;
/// The style bit set while a rename is open.
pub const STYLE_RENAMING: u32 = 0x4;
/// The style bit that adds 0x10 to a press's reply.
pub const STYLE_PRESS_FLAG: u32 = 0x8;
/// The style bit that refuses every drop.
pub const STYLE_NO_DROP: u32 = 0x10_0000;
/// The first hit code of a chevron: the code is this plus the row's
/// 0-based index.
pub const EXPANDER_CODE: i32 = 0x7f18;

/// The tree's option bits.
pub mod option {
    /// F2 renames the selected node.
    pub const RENAME: u32 = 0x20;
    /// A press below the last row is the tree's (not the form's).
    pub const PRESS_EMPTY: u32 = 0x100;
    /// No icon column before the text.
    pub const NO_ICON_SPACE: u32 = 0x10_0000;
    /// Popup mode: hover selects, a press picks and closes.
    pub const POPUP: u32 = 0x200_0000;
    /// Compact: half indents, no chevrons; a press on the selected row
    /// does not toggle it.
    pub const COMPACT: u32 = 0x400_0000;
    /// A drag carries only the icon.
    pub const ICON_DRAG: u32 = 0x4000_0000;
    /// A drop reports where it lands (0x201, or 0x301 with
    /// [`ICON_DRAG`]).
    pub const DROP_HINT: u32 = 0x8000_0000;
}

/// Pointer bits a press reads.
pub mod pointer {
    pub const LEFT: u32 = 0x1;
    pub const PRESS: u32 = 0x2000;
    pub const DRAG: u32 = 0x4000;
}

/// A node flag: the text is drawn in the greyed ink.
pub const NODE_GREYED: u32 = 0x4;

/// One node: its text, icon handle, flags, whether it has ever had
/// children (it then keeps a chevron when collapsed) and its children.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TreeNode {
    pub text: Vec<u8>,
    pub icon: i32,
    pub flags: u32,
    pub expandable: bool,
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    pub fn new(text: &[u8]) -> Self {
        TreeNode {
            text: text.to_vec(),
            ..TreeNode::default()
        }
    }
}

/// A node's place: the child index at each level from the roots.
pub type NodePath = Vec<usize>;

/// What the tree asks of its owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeEvent {
    /// The selection changed (the select callback).
    Selected(Option<NodePath>),
    /// A leaf is to be expanded: the owner adds its children.
    Expand(NodePath),
    /// A node is released (the release callback), children before their
    /// parent; its text is given.
    Released(Vec<u8>),
    /// A rename was committed with a changed text.
    Renamed(NodePath),
    /// Popup mode: the selection was picked (the select callback runs
    /// with it, then the popup ends).
    Picked(Option<NodePath>),
    /// Popup mode: a press off the rows closes the popup.
    Close,
    /// A key the tree passes to its parent form (Escape).
    ParentKey(u32),
    /// A key the tree leaves to the form's keyboard navigation.
    FormKey(u32),
    /// The icon renderer the tree owns is to be cleared.
    ClearIcons,
    /// A drag begins: its kind, the node and the grab offset.
    Drag {
        kind: i32,
        node: Option<NodePath>,
        grab: (i32, i32),
    },
}

fn count(nodes: &[TreeNode]) -> i32 {
    nodes.iter().map(|n| 1 + count(&n.children)).sum()
}

/// The node at a 1-based pre-order index.
fn path_at(nodes: &[TreeNode], index: i32) -> Option<NodePath> {
    fn walk(nodes: &[TreeNode], index: i32, seen: &mut i32, path: &mut NodePath) -> bool {
        for (i, n) in nodes.iter().enumerate() {
            *seen += 1;
            path.push(i);
            if *seen == index || walk(&n.children, index, seen, path) {
                return true;
            }
            path.pop();
        }
        false
    }
    let mut path = Vec::new();
    let mut seen = 0;
    walk(nodes, index, &mut seen, &mut path).then_some(path)
}

/// A node's 1-based pre-order index; the node count when it is not in the
/// tree (as the original's walk returns its count).
fn index_of(nodes: &[TreeNode], path: Option<&NodePath>) -> i32 {
    let Some(path) = path else {
        return count(nodes);
    };
    let mut index = 0;
    let mut level = nodes;
    for (depth, &i) in path.iter().enumerate() {
        if i >= level.len() {
            return count(nodes);
        }
        index += count(&level[..i]) + 1;
        if depth + 1 == path.len() {
            return index;
        }
        level = &level[i].children;
    }
    count(nodes)
}

fn release(node: &TreeNode, events: &mut Vec<TreeEvent>) {
    for c in &node.children {
        release(c, events);
    }
    events.push(TreeEvent::Released(node.text.clone()));
}

/// The tree view's state.
#[derive(Debug, Clone, PartialEq)]
pub struct TreeView {
    pub unit: i32,
    pub client: ClientRect,
    pub border: i32,
    pub style: u32,
    pub attributes: u32,
    pub options: u32,
    pub roots: Vec<TreeNode>,
    /// The icon size, also the indent.
    pub row: i32,
    /// The row pitch.
    pub pitch: i32,
    /// The row count the scroll covers: every node, plus a blank row
    /// unless in popup mode.
    pub rows: i32,
    pub content_height: i32,
    pub scroll: f32,
    pub scroll_step: f32,
    pub selected: Option<NodePath>,
    /// The form's hover code: a row's 1-based index, a chevron's
    /// [`EXPANDER_CODE`] plus its 0-based index, or below 1 for none.
    pub hover: i32,
    /// The drag kind a drag carries (0: rows cannot be dragged).
    pub drag_kind: i32,
    /// An icon renderer is attached.
    pub icons: bool,
    pub focused: bool,
    pub renaming: bool,
    /// The rename field's state and the text it started from.
    pub edit_view: EditView,
    pub original: Vec<u8>,
}

impl TreeView {
    /// The constructor's tree: border 3, the style and attribute bits
    /// above, icons `(unit + 1) · 4` pixels and rows a unit more apart.
    pub fn new(unit: i32, client: ClientRect, options: u32) -> Self {
        let row = (unit + 1) * 4;
        TreeView {
            unit,
            client,
            border: BORDER,
            style: STYLE,
            attributes: ATTRIBUTES,
            options,
            roots: Vec::new(),
            row,
            pitch: row + unit,
            rows: 0,
            content_height: 0,
            scroll: 0.0,
            scroll_step: 0.0,
            selected: None,
            hover: 0,
            drag_kind: 0,
            icons: false,
            focused: false,
            renaming: false,
            edit_view: EditView::default(),
            original: Vec::new(),
        }
    }

    fn has(&self, bit: u32) -> bool {
        self.options & bit == bit
    }

    /// The indent per level: half an icon when compact, else an icon (or,
    /// for the chevrons and icons without a renderer, four units).
    fn step(&self, draw: bool) -> i32 {
        if self.has(option::COMPACT) {
            ((self.row as u32) >> 1) as i32
        } else if draw && !self.icons {
            self.unit << 2
        } else {
            self.row
        }
    }

    pub fn node(&self, path: &NodePath) -> Option<&TreeNode> {
        let (first, rest) = path.split_first()?;
        let mut node = self.roots.get(*first)?;
        for &i in rest {
            node = node.children.get(i)?;
        }
        Some(node)
    }

    fn node_mut(&mut self, path: &NodePath) -> Option<&mut TreeNode> {
        let (first, rest) = path.split_first()?;
        let mut node = self.roots.get_mut(*first)?;
        for &i in rest {
            node = node.children.get_mut(i)?;
        }
        Some(node)
    }

    /// The node at a 1-based pre-order index.
    pub fn path_at(&self, index: i32) -> Option<NodePath> {
        path_at(&self.roots, index)
    }

    /// A node's 1-based pre-order index (the node count when absent).
    pub fn index_of(&self, path: Option<&NodePath>) -> i32 {
        index_of(&self.roots, path)
    }

    /// Adds a root (the owner fills the root list; Exarust's helper).
    pub fn add_root(&mut self, node: TreeNode) -> NodePath {
        self.roots.push(node);
        vec![self.roots.len() - 1]
    }

    /// Adds an empty child to `parent`, which becomes expandable.
    pub fn add_child(&mut self, parent: &NodePath) -> Option<NodePath> {
        let node = self.node_mut(parent)?;
        node.children.push(TreeNode::default());
        node.expandable = true;
        let mut path = parent.clone();
        path.push(node.children.len() - 1);
        Some(path)
    }

    /// The layout: the row count, the content height (the client's height
    /// plus the hidden rows plus two units) and the scroll step (one over
    /// the hidden rows, 0 when everything fits). The original runs the
    /// height once without the two units before the client metrics; with
    /// the client rectangle given, only the final value is kept.
    pub fn layout(&mut self) {
        self.rows = count(&self.roots) + i32::from(!self.has(option::POPUP));
        let height = i64::from(self.client.bottom) - i64::from(self.client.top);
        let visible = (height / i64::from(self.pitch)) as i32;
        self.content_height = self.unit * 2 + (self.rows - visible) * self.pitch + height as i32;
        self.scroll_step = if visible < self.rows {
            (1.0 / (f64::from(self.rows) - f64::from(visible))) as f32
        } else {
            0.0
        };
    }

    /// Sets the scroll through the form's setter (clamped, snapped).
    pub fn set_scroll(&mut self, f: f32) {
        if let Some((stored, _)) = set_scroll_y(
            f,
            self.scroll,
            self.scroll_step,
            self.attributes,
            self.client,
            self.content_height,
            (0, 0),
            0,
        ) {
            self.scroll = stored;
        }
    }

    fn first_row(&self) -> i32 {
        if self.scroll_step <= 0.0 {
            0
        } else {
            round_i32(f64::from(self.scroll / self.scroll_step))
        }
    }

    /// Lays out again and keeps the scroll on the same whole row.
    pub fn resnap(&mut self) {
        let k = self.first_row();
        self.layout();
        self.set_scroll(k as f32 * self.scroll_step);
    }

    /// A new unit: icons four units, rows five units apart.
    pub fn rescale(&mut self, unit: i32) {
        self.unit = unit;
        self.row = unit * 4;
        self.pitch = unit + self.row;
        self.resnap();
    }

    /// Releases every node: closes a rename, clears the owned icon
    /// renderer, drops the selection (reporting it), releases the nodes
    /// and zeroes the scroll.
    pub fn clear(&mut self) -> Vec<TreeEvent> {
        let mut events = Vec::new();
        self.renaming = false;
        if self.icons {
            events.push(TreeEvent::ClearIcons);
        }
        if self.selected.take().is_some() {
            events.push(TreeEvent::Selected(None));
        }
        for n in &self.roots {
            release(n, &mut events);
        }
        self.roots.clear();
        self.scroll_step = 0.0;
        self.scroll = 0.0;
        events
    }

    /// Expands a leaf (the owner adds its children) or collapses a node
    /// (its descendants are released). A selection inside the collapsed
    /// branch is dropped: the original keeps a pointer to the released
    /// node.
    pub fn toggle(&mut self, path: &NodePath) -> Vec<TreeEvent> {
        let mut events = Vec::new();
        let Some(node) = self.node(path) else {
            return events;
        };
        if node.children.is_empty() {
            events.push(TreeEvent::Expand(path.clone()));
        } else {
            for c in &node.children {
                release(c, &mut events);
            }
            if let Some(node) = self.node_mut(path) {
                node.children.clear();
            }
            if self
                .selected
                .as_ref()
                .is_some_and(|s| s.len() > path.len() && s.starts_with(path))
            {
                self.selected = None;
            }
        }
        self.resnap();
        events
    }

    /// Where a node's text starts for the rename caret: three units plus
    /// the icon column, plus an indent per level; one pitch per row above
    /// it from one unit down.
    pub fn node_position(&self, path: Option<&NodePath>) -> (i32, i32) {
        let mut x = self.unit * 3;
        if !self.has(option::NO_ICON_SPACE) {
            x += self.row;
        }
        let step = self.step(false);
        let index = self.index_of(path);
        let depth = match path {
            Some(p) if self.node(p).is_some() => p.len() as i32 - 1,
            _ => 0,
        };
        (x + depth * step, self.unit + self.pitch * (index - 1))
    }

    /// Keeps a node in view: mode 1 puts it two rows below the top;
    /// otherwise the scroll moves only when it is above the first row
    /// (to two rows above it) or below the last.
    pub fn scroll_to(&mut self, path: Option<&NodePath>, mode: i32) {
        let Some(path) = path else {
            return;
        };
        let index = i64::from(self.index_of(Some(path)));
        if mode == 1 {
            self.set_scroll((index - 2) as f32 * self.scroll_step);
            return;
        }
        let visible =
            (i64::from(self.client.bottom) - i64::from(self.client.top)) / i64::from(self.pitch);
        let first = i64::from(round_i32(f64::from(
            (i64::from(self.rows) - visible) as f32 * self.scroll,
        )));
        if index - 2 < first {
            self.set_scroll((index - 2) as f32 * self.scroll_step);
        } else if first + visible < index + 1 {
            self.set_scroll((index - visible + 1) as f32 * self.scroll_step);
        }
    }

    fn select(&mut self, path: Option<NodePath>, events: &mut Vec<TreeEvent>) {
        self.selected = path;
        events.push(TreeEvent::Selected(self.selected.clone()));
    }

    /// The hit code at a point in content coordinates (the form's scroll
    /// added): a row's 1-based index right of its text start (two units
    /// plus an indent per level), its chevron's code in the four units to
    /// the left, `None` elsewhere (the form's own code stands). Compact
    /// trees have no chevron zone.
    pub fn hit(&self, font: &GuiFont, x: i32, y: i32) -> Option<i32> {
        struct Walk {
            x: i32,
            y: i32,
            counter: i32,
            result: i32,
            zone: i32,
            step: i32,
            pitch: i32,
            limit: i32,
            px: i32,
            py: i32,
        }
        fn visit(w: &mut Walk, node: &TreeNode) -> bool {
            let mut found = false;
            w.counter += 1;
            if w.y < w.py && i64::from(w.py) < i64::from(w.pitch) + i64::from(w.y) {
                found = true;
                if w.px < w.x {
                    if i64::from(w.px) < i64::from(w.zone) + i64::from(w.x) {
                        found = false;
                    } else {
                        w.result = -w.counter;
                    }
                } else {
                    w.result = w.counter;
                }
            }
            w.y += w.pitch;
            w.x += w.step;
            for c in &node.children {
                if w.y < w.limit {
                    break;
                }
                if visit(w, c) {
                    found = true;
                    break;
                }
            }
            w.x -= w.step;
            found
        }
        let compact = self.has(option::COMPACT);
        let mut w = Walk {
            x: self.unit << 1,
            y: self.unit,
            counter: 0,
            result: 0,
            zone: if compact { 1 } else { -(self.unit << 2) },
            step: self.step(false),
            pitch: self.pitch,
            limit: self.border - font.glyph.1,
            px: x,
            py: y,
        };
        for n in &self.roots {
            if visit(&mut w, n) {
                break;
            }
        }
        match w.result {
            r if r > 0 => Some(r),
            r if r < 0 => Some(0x7f17 - r),
            _ => None,
        }
    }

    /// The form's hover setter: a row or chevron code redraws (and in
    /// popup mode selects the row, a chevron code selecting nothing);
    /// anything else takes the form's reply, with a redraw when a row was
    /// hovered before.
    pub fn set_hover(&mut self, code: i32, form_reply: u32) -> u32 {
        if code < 1 {
            let reply = if self.hover < 1 {
                form_reply
            } else {
                form_reply | 3
            };
            self.hover = code;
            return reply;
        }
        if self.has(option::POPUP) {
            self.selected = self.path_at(code);
        }
        self.hover = code;
        3
    }

    /// A pointer move with the form's hit code: over a row the row is
    /// selected (without the select callback) and the tree redraws; while
    /// renaming the caret follows the pointer (content coordinates).
    pub fn pointer_move(&mut self, font: &GuiFont, code: Option<i32>, at: (i32, i32)) -> u32 {
        if self.renaming {
            self.rename_caret_at(font, at.0);
            if let Some(text) = self
                .selected
                .as_ref()
                .and_then(|p| self.node(p))
                .map(|n| n.text.clone())
            {
                text_edit::scroll_view(font, &text, &mut self.edit_view, 100);
            }
            return 3;
        }
        if let Some(code) = code.filter(|&c| 0 < c && c < EXPANDER_CODE) {
            let node = self.path_at(code);
            if node != self.selected {
                self.selected = node;
                return 3;
            }
        }
        0
    }

    /// The rename caret from a pointer x: whole gaps from the text start
    /// (rounded) plus the first visible byte, clamped to the text.
    pub fn rename_caret_at(&mut self, font: &GuiFont, x: i32) {
        let (nx, _) = self.node_position(self.selected.as_ref());
        let cells = round_i32((f64::from(x) - f64::from(nx)) / f64::from(font.gap))
            + i32::from(self.edit_view.first);
        self.edit_view.caret = if cells < 1 { 0 } else { cells as u8 };
        let len = self
            .selected
            .as_ref()
            .and_then(|p| self.node(p))
            .map_or(0, |n| n.text.len().min(255) as u8);
        self.edit_view.caret = self.edit_view.caret.min(len);
    }

    /// A press, with the hover code the form left. Returns the reply and
    /// the events.
    pub fn press(
        &mut self,
        font: &GuiFont,
        buttons: u32,
        at: (i32, i32),
        form_reply: u32,
    ) -> (u32, Vec<TreeEvent>) {
        let mut events = Vec::new();
        let popup = self.has(option::POPUP);
        if (self.hover < 1 && !self.has(option::PRESS_EMPTY)) || self.hover < -1 {
            if self.hover < 1 && popup {
                events.push(TreeEvent::Close);
                return (0x80, events);
            }
            return (form_reply, events);
        }
        if buttons & pointer::LEFT == 0 {
            return (0, events);
        }
        if self.renaming {
            self.rename_caret_at(font, at.0);
            self.edit_view.anchor = self.edit_view.caret;
            if let Some(text) = self
                .selected
                .as_ref()
                .and_then(|p| self.node(p))
                .map(|n| n.text.clone())
            {
                text_edit::scroll_view(font, &text, &mut self.edit_view, 200);
            }
            return (0x17, events);
        }
        if popup {
            events.push(TreeEvent::Picked(self.selected.clone()));
            return (0x80, events);
        }
        if buttons & pointer::DRAG != 0 {
            if self.drag_kind != 0 && self.hover > 0 {
                let grab = if self.has(option::ICON_DRAG) {
                    let half = ((self.row as u32) >> 1) as i32;
                    (half, half)
                } else {
                    (
                        ((self.row as u32) >> 1) as i32 - self.unit,
                        ((self.pitch as u32) >> 1) as i32,
                    )
                };
                events.push(TreeEvent::Drag {
                    kind: self.drag_kind,
                    node: self.path_at(self.hover),
                    grab,
                });
                return (0x105, events);
            }
            return (0, events);
        }
        if buttons & pointer::PRESS == 0 {
            return (0, events);
        }
        let mut reply = if self.style & STYLE_PRESS_FLAG == STYLE_PRESS_FLAG {
            0x10
        } else {
            0
        };
        if self.hover < EXPANDER_CODE {
            let node = if self.hover < 1 {
                None
            } else {
                self.path_at(self.hover)
            };
            if node == self.selected {
                if let Some(sel) = self.selected.clone() {
                    if !self.has(option::COMPACT) {
                        events.extend(self.toggle(&sel));
                    }
                    reply |= 1;
                }
            } else {
                self.select(node, &mut events);
                reply |= 3;
            }
        } else if let Some(node) = self.path_at(self.hover - 0x7f17) {
            events.extend(self.toggle(&node));
            reply |= 1;
        } else {
            reply |= 1;
        }
        (reply, events)
    }

    /// Opens the rename field on the selected node (with the rename
    /// option): the field's window is 20 bytes, the caret at the end, the
    /// view scrolled for 200 pixels.
    pub fn begin_rename(&mut self, font: &GuiFont) {
        if !self.has(option::RENAME) {
            return;
        }
        let Some(text) = self
            .selected
            .as_ref()
            .and_then(|p| self.node(p))
            .map(|n| n.text.clone())
        else {
            return;
        };
        self.renaming = true;
        self.style |= STYLE_RENAMING;
        self.edit_view.length = 0x14;
        self.original = text.clone();
        self.edit_view.anchor = 0;
        self.edit_view.caret = text.len().min(255) as u8;
        text_edit::scroll_view(font, &text, &mut self.edit_view, 200);
        self.focused = true;
    }

    /// Closes the rename: a changed text is reported.
    pub fn commit_rename(&mut self) -> Vec<TreeEvent> {
        let mut events = Vec::new();
        if let Some(path) = self.selected.clone()
            && self.node(&path).is_some_and(|n| n.text != self.original)
        {
            events.push(TreeEvent::Renamed(path));
        }
        self.renaming = false;
        self.style &= !STYLE_RENAMING;
        events
    }

    /// A key. `typed` is a character rather than a key press. While
    /// renaming, Enter commits, Escape restores the text and other keys go
    /// to the edit field. Otherwise (key presses only): Enter toggles,
    /// Escape goes to the parent form, Left goes to the parent of a leaf
    /// or collapses, Right expands a leaf or steps down, Up and Down step,
    /// F2 renames; other keys are the form's. A step is clamped to the
    /// first row and to one before the row count, selects that row and
    /// keeps it in view.
    #[allow(clippy::too_many_arguments)]
    pub fn key(
        &mut self,
        font: &GuiFont,
        key: u32,
        typed: bool,
        ctrl: bool,
        shift: bool,
        clipboard: Option<&[u8]>,
    ) -> (Vec<TreeEvent>, EditAction) {
        let mut events = Vec::new();
        if self.renaming {
            if !typed && key > 0x0c {
                if key == 0x0d {
                    return (self.commit_rename(), EditAction::None);
                }
                if key == 0x1b {
                    let original = self.original.clone();
                    if let Some(path) = self.selected.clone()
                        && let Some(n) = self.node_mut(&path)
                    {
                        n.text = original;
                    }
                    self.renaming = false;
                    self.style &= !STYLE_RENAMING;
                    return (events, EditAction::None);
                }
            }
            let width = self.client.right - self.unit * 2;
            let mut view = self.edit_view;
            let mut action = EditAction::None;
            if let Some(path) = self.selected.clone()
                && let Some(n) = self.node_mut(&path)
            {
                action = text_edit::apply_key(
                    font,
                    &mut n.text,
                    &mut view,
                    key as u8,
                    typed,
                    ctrl,
                    shift,
                    0xff,
                    width,
                    clipboard,
                );
            }
            self.edit_view = view;
            return (events, action);
        }
        if typed {
            return (events, EditAction::None);
        }
        let sel = self.selected.clone();
        let leaf = sel
            .as_ref()
            .and_then(|p| self.node(p))
            .map(|n| n.children.is_empty());
        let step = match key {
            0x0d => {
                if let Some(p) = &sel {
                    events.extend(self.toggle(p));
                }
                None
            }
            0x1b => {
                events.push(TreeEvent::ParentKey(0x1b));
                None
            }
            0x25 => match (leaf, &sel) {
                (Some(true), Some(p)) if p.len() > 1 => {
                    Some(self.index_of(Some(&p[..p.len() - 1].to_vec())))
                }
                (Some(false), Some(p)) => {
                    events.extend(self.toggle(p));
                    None
                }
                _ => None,
            },
            0x26 => Some(self.index_of(sel.as_ref()) - 1),
            0x27 => match (leaf, &sel) {
                (Some(true), Some(p)) => {
                    events.extend(self.toggle(p));
                    None
                }
                (Some(false), _) => Some(self.index_of(sel.as_ref()) + 1),
                _ => None,
            },
            0x28 => Some(self.index_of(sel.as_ref()) + 1),
            0x71 => {
                self.begin_rename(font);
                None
            }
            _ => {
                events.push(TreeEvent::FormKey(key));
                None
            }
        };
        if let Some(mut i) = step {
            if i < 1 {
                i = 1;
            }
            if i64::from(self.rows) - 1 < i64::from(i) {
                i = self.rows - 1;
            }
            let node = self.path_at(i);
            if node != self.selected {
                self.select(node, &mut events);
                let s = self.selected.clone();
                self.scroll_to(s.as_ref(), 0);
            }
        }
        (events, EditAction::None)
    }

    /// Whether a drop of `kind` is taken, onto the row at `target`
    /// (1-based). Refused with another kind or the no-drop style bit; from
    /// this tree, refused onto no row, onto the dragged node or into its
    /// branch, and (without [`option::DROP_HINT`]) refused anyway.
    pub fn accepts_drop(
        &self,
        kind: i32,
        from_self: bool,
        dragged: Option<&NodePath>,
        target: i32,
    ) -> u32 {
        if kind != self.drag_kind || self.style & STYLE_NO_DROP != 0 {
            return 0;
        }
        if from_self {
            let Some(node) = self.path_at(target) else {
                return 0;
            };
            if let Some(d) = dragged
                && node.starts_with(d)
            {
                return 0;
            }
        }
        if self.has(option::DROP_HINT) {
            if self.has(option::ICON_DRAG) {
                0x301
            } else {
                0x201
            }
        } else if from_self {
            0
        } else {
            1
        }
    }
}

/// The selected row's highlight: a bevelled band (form inks 0, 1, 4, 5),
/// a frame (highlight inks 4 and 0, strength 0.25) and a shade box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeSelection {
    pub band: [i32; 4],
    pub frame: [i32; 4],
    pub shade: [i32; 4],
}

/// Which chevron.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chevron {
    /// Right: a collapsed node.
    Collapsed,
    /// Down: an expanded node.
    Expanded,
}

/// A chevron draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeChevron {
    pub kind: Chevron,
    pub at: (i32, i32),
    pub ink: GlyphInk,
}

/// A row's text: plain within `width` (the original's clipped draw) or,
/// for the hovered row, already cut with an ellipsis to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeText {
    pub row: i32,
    pub text: Vec<u8>,
    pub pen: (i32, i32),
    pub width: i32,
    pub ink: TextInk,
}

/// Where a tree's parts are drawn, in content coordinates.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TreeDraw {
    pub first: i32,
    pub selection: Option<TreeSelection>,
    /// Icons: handle and position, all tinted with [`tree_icon_ink`].
    pub icons: Vec<(i32, (i32, i32))>,
    pub chevrons: Vec<TreeChevron>,
    pub texts: Vec<TreeText>,
    /// The open rename field's pen (text ink 8), and whether its caret is
    /// drawn (the form is focused).
    pub edit: Option<((i32, i32), bool)>,
}

/// The icons' tint: text ink 7 with its alpha raised by a quarter, at
/// most to 255.
pub fn tree_icon_ink(palette: &SkinPalette) -> SkinInk {
    let mut ink = PaletteInk::Text(7).ink(palette);
    let a = ink[3];
    ink[3] = a.wrapping_add((a >> 2).min(255 - a));
    ink
}

struct DrawWalk {
    x: i32,
    y: i32,
    counter: i32,
    step: i32,
    hot: i32,
    limit: i32,
}

impl TreeView {
    /// The draw in four passes, as the original's: the selected row's
    /// highlight, the icons, the chevrons (not when compact), the texts.
    /// Rows above the first drawn one (by the scroll) and rows starting
    /// within four pixels of the client's right are skipped, but every
    /// row advances the pen.
    pub fn draw(&self, font: &GuiFont) -> TreeDraw {
        let u = self.unit;
        let compact = self.has(option::COMPACT);
        let right = self.client.right;
        let mut out = TreeDraw {
            first: self.first_row(),
            ..TreeDraw::default()
        };
        let limit = self.border - font.glyph.1;
        let step = self.step(true);
        // The selection: the walk stops advancing at the selected row.
        if let Some(sel) = &self.selected
            && self.node(sel).is_some()
        {
            let y = u + self.pitch * (self.index_of(Some(sel)) - 1);
            let end = right - u - 1;
            out.selection = Some(TreeSelection {
                band: [u - 1, y - 1, end, y + self.pitch + 1],
                frame: [u, y, end - 1, y + self.pitch],
                shade: [u, y, end - u - 1, self.pitch],
            });
        }
        if self.icons {
            let mut w = DrawWalk {
                x: if compact { u << 1 } else { u << 2 },
                y: (((i64::from(self.pitch) - i64::from(self.row)) + 1) as u64 >> 1) as i32 + u + 1,
                counter: -1,
                step,
                hot: -1,
                limit,
            };
            fn icons(t: &TreeView, w: &mut DrawWalk, n: &TreeNode, first: i32, out: &mut TreeDraw) {
                w.counter += 1;
                if first <= w.counter && i64::from(w.x) < i64::from(t.client.right) - 4 {
                    out.icons.push((n.icon, (w.x, w.y)));
                }
                w.y += t.pitch;
                w.x += w.step;
                for c in &n.children {
                    if w.y < w.limit {
                        break;
                    }
                    icons(t, w, c, first, out);
                }
                w.x -= w.step;
            }
            for n in &self.roots {
                icons(self, &mut w, n, out.first, &mut out);
            }
        }
        let mut text_x = 0;
        if !compact {
            let mut w = DrawWalk {
                x: u,
                y: u * 2 - self.pitch,
                counter: -1,
                step,
                hot: if self.hover < EXPANDER_CODE {
                    -1
                } else {
                    self.hover - EXPANDER_CODE
                },
                limit,
            };
            fn chevrons(
                t: &TreeView,
                w: &mut DrawWalk,
                n: &TreeNode,
                first: i32,
                out: &mut TreeDraw,
            ) {
                w.counter += 1;
                w.y += t.pitch;
                let ink = if w.counter == w.hot {
                    GlyphInk::Outlined
                } else {
                    GlyphInk::Tinted(PaletteInk::Text(7))
                };
                let in_view = first <= w.counter && i64::from(w.x) < i64::from(t.client.right) - 4;
                if n.children.is_empty() {
                    if n.expandable && in_view && 0 < w.x {
                        out.chevrons.push(TreeChevron {
                            kind: Chevron::Collapsed,
                            at: (w.x, w.y),
                            ink,
                        });
                    }
                    return;
                }
                if in_view && -i64::from(t.unit) < i64::from(w.x) {
                    out.chevrons.push(TreeChevron {
                        kind: Chevron::Expanded,
                        at: (w.x, w.y),
                        ink,
                    });
                }
                w.x += w.step;
                for c in &n.children {
                    if w.y < w.limit {
                        break;
                    }
                    chevrons(t, w, c, first, out);
                }
                w.x -= w.step;
            }
            for n in &self.roots {
                chevrons(self, &mut w, n, out.first, &mut out);
            }
            text_x = (u >> 1) + w.x;
        }
        text_x += u * 3;
        if !self.has(option::NO_ICON_SPACE) {
            text_x += self.row;
        }
        let mut w = DrawWalk {
            x: text_x,
            y: ((i64::from(font.ascent) + i64::from(self.pitch) + 1) as u64 >> 1) as i32 + u + 1,
            counter: -1,
            step,
            hot: if self.hover < EXPANDER_CODE {
                self.hover - 1
            } else {
                -1
            },
            limit,
        };
        fn texts(
            t: &TreeView,
            font: &GuiFont,
            w: &mut DrawWalk,
            n: &TreeNode,
            path: &mut NodePath,
            first: i32,
            out: &mut TreeDraw,
        ) {
            w.counter += 1;
            let right = t.client.right;
            if first <= w.counter && i64::from(w.x) < i64::from(right) - 4 {
                let selected = t.selected.as_ref() == Some(path);
                let width = right - w.x - (t.unit << 1);
                if t.renaming && selected {
                    out.edit = Some(((w.x, w.y), t.focused));
                } else if w.counter == w.hot {
                    out.texts.push(TreeText {
                        row: w.counter,
                        text: ellipsis_fit(font, &n.text, width),
                        pen: (w.x, w.y),
                        width,
                        ink: TextInk::Hot,
                    });
                } else {
                    let ink = if selected {
                        PaletteInk::Text(9)
                    } else if n.flags & NODE_GREYED != 0 {
                        PaletteInk::Text(4)
                    } else {
                        PaletteInk::Text(6)
                    };
                    out.texts.push(TreeText {
                        row: w.counter,
                        text: n.text.clone(),
                        pen: (w.x, w.y),
                        width,
                        ink: TextInk::Tinted(ink),
                    });
                }
            }
            w.y += t.pitch;
            w.x += w.step;
            for (i, c) in n.children.iter().enumerate() {
                if w.y < w.limit {
                    break;
                }
                path.push(i);
                texts(t, font, w, c, path, first, out);
                path.pop();
            }
            w.x -= w.step;
        }
        let mut path = Vec::new();
        for (i, n) in self.roots.iter().enumerate() {
            path.push(i);
            texts(self, font, &mut w, n, &mut path, out.first, &mut out);
            path.pop();
        }
        out
    }

    /// The dragged node's label at a pointer (screen) less the grab: a
    /// frame (form inks 4 and 0, strength 0.25) and shade box as wide as
    /// the text, the icon (when a renderer is attached) and the text in
    /// text ink 8 one unit past an icon's width; with [`option::ICON_DRAG`]
    /// only the icon.
    pub fn drag_label(
        &self,
        font: &GuiFont,
        node: &TreeNode,
        pointer: (i32, i32),
        grab: (i32, i32),
    ) -> TreeDragLabel {
        let x = pointer.0 - grab.0;
        let y = pointer.1 - grab.1;
        if self.has(option::ICON_DRAG) {
            return TreeDragLabel {
                frame: None,
                icon: self.icons.then_some((node.icon, (x, y))),
                text: None,
            };
        }
        let w = measure(font, &node.text);
        TreeDragLabel {
            frame: Some((
                [x, y, x + w - 1, y + self.pitch - 2],
                [x, y, w - 1, self.pitch - 2],
            )),
            icon: self.icons.then_some((node.icon, (x, y))),
            text: Some((
                node.text.clone(),
                (
                    self.unit + x + self.row,
                    ((i64::from(self.pitch) + i64::from(font.ascent) + 1) as u64 >> 1) as i32 + y,
                ),
            )),
        }
    }
}

/// A drag label: the frame and shade box, the icon (tinted with text ink
/// 10 at alpha 200) and the text with its pen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeDragLabel {
    pub frame: Option<([i32; 4], [i32; 4])>,
    pub icon: Option<(i32, (i32, i32))>,
    pub text: Option<(Vec<u8>, (i32, i32))>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_font::RAYFORM_08;

    const CLIENT: ClientRect = ClientRect {
        left: 3,
        top: 3,
        right: 197,
        bottom: 97,
    };

    /// a (b (c), d), e — unit 4: icon 20, pitch 24.
    fn tree(options: u32) -> TreeView {
        let mut t = TreeView::new(4, CLIENT, options);
        let a = t.add_root(TreeNode::new(b"a"));
        let b = t.add_child(&a).unwrap();
        t.node_mut(&b).unwrap().text = b"b".to_vec();
        let c = t.add_child(&b).unwrap();
        t.node_mut(&c).unwrap().text = b"c".to_vec();
        let d = t.add_child(&a).unwrap();
        t.node_mut(&d).unwrap().text = b"d".to_vec();
        t.add_root(TreeNode::new(b"e"));
        t.layout();
        t
    }

    #[test]
    fn the_constructor_sizes_rows_from_the_unit() {
        let t = TreeView::new(4, CLIENT, 0);
        assert_eq!((t.row, t.pitch, t.border), (20, 24, 3));
        let mut t = t;
        t.rescale(5);
        assert_eq!((t.row, t.pitch), (20, 25));
    }

    #[test]
    fn indices_run_in_pre_order() {
        let t = tree(0);
        assert_eq!(t.path_at(1), Some(vec![0]));
        assert_eq!(t.path_at(3), Some(vec![0, 0, 0]));
        assert_eq!(t.path_at(4), Some(vec![0, 1]));
        assert_eq!(t.path_at(5), Some(vec![1]));
        assert_eq!(t.path_at(6), None);
        assert_eq!(t.index_of(Some(&vec![0, 1])), 4);
        assert_eq!(t.index_of(None), 5);
        assert!(t.node(&vec![0]).unwrap().expandable);
    }

    #[test]
    fn the_layout_counts_a_blank_row_unless_in_popup_mode() {
        let mut t = tree(0);
        t.client.bottom = 51; // two rows visible
        t.layout();
        assert_eq!(t.rows, 6);
        assert_eq!(t.content_height, 8 + 4 * 24 + 48);
        assert_eq!(t.scroll_step, 0.25);
        t.options = option::POPUP;
        t.layout();
        assert_eq!(t.rows, 5);
        assert!((t.scroll_step - 1.0 / 3.0).abs() < 1e-7);
        t.client.bottom = 200;
        t.layout();
        assert_eq!(t.scroll_step, 0.0);
    }

    #[test]
    fn hits_find_rows_right_of_the_text_and_chevrons_left_of_it() {
        let t = tree(0);
        // Row 1 spans y 4..28 (exclusive), text from x 8.
        assert_eq!(t.hit(&RAYFORM_08, 8, 10), Some(1));
        assert_eq!(t.hit(&RAYFORM_08, 7, 10), Some(EXPANDER_CODE));
        assert_eq!(t.hit(&RAYFORM_08, -9, 10), None);
        // Row 2 (depth 1) starts at x 28; its chevron zone is 12..28.
        assert_eq!(t.hit(&RAYFORM_08, 20, 40), Some(EXPANDER_CODE + 1));
        assert_eq!(t.hit(&RAYFORM_08, 11, 40), None);
        assert_eq!(t.hit(&RAYFORM_08, 30, 40), Some(2));
        assert_eq!(t.hit(&RAYFORM_08, 50, 28), None);
        assert_eq!(t.hit(&RAYFORM_08, 50, 4), None);
        assert_eq!(t.hit(&RAYFORM_08, 50, 110), Some(5));
        let c = tree(option::COMPACT);
        assert_eq!(c.hit(&RAYFORM_08, 7, 10), None);
        assert_eq!(c.hit(&RAYFORM_08, 18, 40), Some(2));
    }

    #[test]
    fn hovering_selects_without_the_callback() {
        let mut t = tree(0);
        assert_eq!(t.pointer_move(&RAYFORM_08, Some(4), (0, 0)), 3);
        assert_eq!(t.selected, Some(vec![0, 1]));
        assert_eq!(t.pointer_move(&RAYFORM_08, Some(4), (0, 0)), 0);
        assert_eq!(t.pointer_move(&RAYFORM_08, Some(EXPANDER_CODE), (0, 0)), 0);
    }

    #[test]
    fn a_press_selects_then_toggles_the_selected_row() {
        let mut t = tree(0);
        t.hover = 2;
        let (reply, events) = t.press(&RAYFORM_08, 0x2001, (0, 0), 0);
        // The constructor's style carries the 0x10 reply bit.
        assert_eq!(reply, 0x13);
        assert_eq!(events, vec![TreeEvent::Selected(Some(vec![0, 0]))]);
        let (reply, events) = t.press(&RAYFORM_08, 0x2001, (0, 0), 0);
        assert_eq!(reply, 0x11);
        assert_eq!(events, vec![TreeEvent::Released(b"c".to_vec())]);
        assert!(t.node(&vec![0, 0]).unwrap().children.is_empty());
        // Now a collapsed leaf: the next press asks the owner to expand it.
        let (_, events) = t.press(&RAYFORM_08, 0x2001, (0, 0), 0);
        assert_eq!(events, vec![TreeEvent::Expand(vec![0, 0])]);
    }

    #[test]
    fn a_chevron_press_toggles_its_row() {
        let mut t = tree(0);
        t.hover = EXPANDER_CODE;
        let (reply, events) = t.press(&RAYFORM_08, 0x2001, (0, 0), 0);
        assert_eq!(reply, 0x11);
        assert_eq!(
            events,
            vec![
                TreeEvent::Released(b"c".to_vec()),
                TreeEvent::Released(b"b".to_vec()),
                TreeEvent::Released(b"d".to_vec()),
            ]
        );
        assert_eq!(t.rows, 3);
    }

    #[test]
    fn popup_mode_picks_and_closes() {
        let mut t = tree(option::POPUP);
        assert_eq!(t.set_hover(3, 0), 3);
        assert_eq!(t.selected, Some(vec![0, 0, 0]));
        assert_eq!(
            t.press(&RAYFORM_08, 1, (0, 0), 0),
            (0x80, vec![TreeEvent::Picked(Some(vec![0, 0, 0]))])
        );
        t.hover = 0;
        assert_eq!(
            t.press(&RAYFORM_08, 1, (0, 0), 0),
            (0x80, vec![TreeEvent::Close])
        );
    }

    #[test]
    fn a_drag_carries_the_row_with_its_grab() {
        let mut t = tree(0);
        t.drag_kind = 7;
        t.hover = 4;
        let (reply, events) = t.press(&RAYFORM_08, 0x4001, (0, 0), 0);
        assert_eq!(reply, 0x105);
        assert_eq!(
            events,
            vec![TreeEvent::Drag {
                kind: 7,
                node: Some(vec![0, 1]),
                grab: (6, 12)
            }]
        );
    }

    #[test]
    fn arrows_walk_rows_and_left_right_fold() {
        let mut t = tree(0);
        let (events, _) = t.key(&RAYFORM_08, 0x28, false, false, false, None);
        // No selection: Down from the count goes to row 5 (clamped to rows - 1).
        assert_eq!(events, vec![TreeEvent::Selected(Some(vec![1]))]);
        t.selected = Some(vec![0, 0, 0]);
        let (events, _) = t.key(&RAYFORM_08, 0x25, false, false, false, None);
        assert_eq!(events, vec![TreeEvent::Selected(Some(vec![0, 0]))]);
        let (events, _) = t.key(&RAYFORM_08, 0x25, false, false, false, None);
        assert_eq!(events, vec![TreeEvent::Released(b"c".to_vec())]);
        let (events, _) = t.key(&RAYFORM_08, 0x27, false, false, false, None);
        assert_eq!(events, vec![TreeEvent::Expand(vec![0, 0])]);
        t.selected = Some(vec![0]);
        let (events, _) = t.key(&RAYFORM_08, 0x26, false, false, false, None);
        assert_eq!(events, vec![]);
        assert_eq!(
            t.key(&RAYFORM_08, 0x1b, false, false, false, None).0,
            vec![TreeEvent::ParentKey(0x1b)]
        );
        assert_eq!(
            t.key(&RAYFORM_08, 0x09, false, false, false, None).0,
            vec![TreeEvent::FormKey(9)]
        );
    }

    #[test]
    fn in_popup_mode_the_last_row_is_out_of_the_arrows_reach() {
        let mut t = tree(option::POPUP);
        t.selected = Some(vec![0, 1]);
        let (events, _) = t.key(&RAYFORM_08, 0x28, false, false, false, None);
        assert_eq!(events, vec![]);
        assert_eq!(t.selected, Some(vec![0, 1]));
    }

    #[test]
    fn renaming_edits_and_escape_restores() {
        let mut t = tree(option::RENAME);
        t.selected = Some(vec![1]);
        t.key(&RAYFORM_08, 0x71, false, false, false, None);
        assert!(t.renaming);
        assert_eq!(t.style & STYLE_RENAMING, STYLE_RENAMING);
        assert_eq!((t.edit_view.anchor, t.edit_view.caret), (0, 1));
        t.key(&RAYFORM_08, u32::from(b'x'), true, false, false, None);
        // The field opens with the whole text selected: typing replaces it.
        assert_eq!(t.node(&vec![1]).unwrap().text, b"x");
        assert_eq!(
            t.key(&RAYFORM_08, 0x0d, false, false, false, None).0,
            vec![TreeEvent::Renamed(vec![1])]
        );
        assert!(!t.renaming);
        t.key(&RAYFORM_08, 0x71, false, false, false, None);
        t.key(&RAYFORM_08, u32::from(b'y'), true, false, false, None);
        t.key(&RAYFORM_08, 0x1b, false, false, false, None);
        assert_eq!(t.node(&vec![1]).unwrap().text, b"x");
    }

    #[test]
    fn the_rename_caret_counts_whole_gaps_from_the_node_position() {
        let mut t = tree(option::RENAME);
        t.selected = Some(vec![0, 1]);
        t.roots[0].children[1].text = b"abcdef".to_vec();
        // Depth 1: 12 + 20 + 20 = 52.
        assert_eq!(t.node_position(Some(&vec![0, 1])), (52, 4 + 3 * 24));
        t.rename_caret_at(&RAYFORM_08, 52 + 20);
        assert_eq!(t.edit_view.caret, 2);
        t.rename_caret_at(&RAYFORM_08, 10);
        assert_eq!(t.edit_view.caret, 0);
        t.rename_caret_at(&RAYFORM_08, 400);
        assert_eq!(t.edit_view.caret, 6);
    }

    #[test]
    fn drops_into_the_own_branch_are_refused() {
        let mut t = tree(option::DROP_HINT);
        t.drag_kind = 2;
        assert_eq!(t.accepts_drop(3, false, None, 1), 0);
        assert_eq!(t.accepts_drop(2, false, None, 1), 0x201);
        assert_eq!(t.accepts_drop(2, true, Some(&vec![0, 0]), 3), 0);
        assert_eq!(t.accepts_drop(2, true, Some(&vec![0, 0]), 4), 0x201);
        assert_eq!(t.accepts_drop(2, true, Some(&vec![0, 0]), 9), 0);
        t.options = 0;
        assert_eq!(t.accepts_drop(2, true, Some(&vec![0, 0]), 4), 0);
        assert_eq!(t.accepts_drop(2, false, None, 4), 1);
        t.style |= STYLE_NO_DROP;
        assert_eq!(t.accepts_drop(2, false, None, 4), 0);
    }

    #[test]
    fn scrolling_keeps_the_selection_in_view() {
        let mut t = tree(0);
        t.client.bottom = 51;
        t.layout();
        t.scroll_to(Some(&vec![1]), 0);
        // Row 5 below two visible rows: (5 - 2 + 1) · 0.25.
        assert_eq!(t.scroll, 1.0);
        t.scroll_to(Some(&vec![0]), 0);
        assert_eq!(t.scroll, 0.0);
        t.scroll_to(Some(&vec![0, 1]), 1);
        assert_eq!(t.scroll, 0.5);
    }

    #[test]
    fn the_draw_places_selection_chevrons_icons_and_texts() {
        let mut t = tree(0);
        t.icons = true;
        t.selected = Some(vec![0, 1]);
        t.hover = 2;
        let d = t.draw(&RAYFORM_08);
        assert_eq!(
            d.selection,
            Some(TreeSelection {
                band: [3, 75, 192, 101],
                frame: [4, 76, 191, 100],
                shade: [4, 76, 187, 24]
            })
        );
        assert_eq!(d.icons[0], (0, (16, 7)));
        assert_eq!(d.icons[1], (0, (36, 31)));
        assert_eq!(d.icons[4], (0, (16, 103)));
        assert_eq!(
            d.chevrons,
            vec![
                TreeChevron {
                    kind: Chevron::Expanded,
                    at: (4, 8),
                    ink: GlyphInk::Tinted(PaletteInk::Text(7))
                },
                TreeChevron {
                    kind: Chevron::Expanded,
                    at: (24, 32),
                    ink: GlyphInk::Tinted(PaletteInk::Text(7))
                },
            ]
        );
        // Text from 1.5 units past the chevrons' start, three units and an icon on.
        assert_eq!(d.texts[0].pen, (38, 24));
        assert_eq!(d.texts[1].ink, TextInk::Hot);
        assert_eq!(d.texts[1].pen, (58, 48));
        assert_eq!(d.texts[3].ink, TextInk::Tinted(PaletteInk::Text(9)));
        assert_eq!(d.texts[3].width, 197 - 58 - 8);
    }

    #[test]
    fn a_collapsed_node_keeps_a_right_chevron() {
        let mut t = tree(0);
        t.toggle(&vec![0, 0]);
        t.hover = EXPANDER_CODE + 1;
        let d = t.draw(&RAYFORM_08);
        // Without an icon renderer the indent is four units.
        assert_eq!(
            d.chevrons[1],
            TreeChevron {
                kind: Chevron::Collapsed,
                at: (20, 32),
                ink: GlyphInk::Outlined
            }
        );
        assert_eq!(d.chevrons.len(), 2);
    }

    #[test]
    fn the_drag_label_follows_the_pointer() {
        let mut t = tree(0);
        t.icons = true;
        let label = t.drag_label(&RAYFORM_08, &TreeNode::new(b"ab"), (100, 50), (6, 12));
        assert_eq!(
            label.frame,
            Some(([94, 38, 94 + 15 - 1, 60], [94, 38, 14, 22]))
        );
        assert_eq!(label.icon, Some((0, (94, 38))));
        assert_eq!(label.text, Some((b"ab".to_vec(), (118, 57))));
    }

    #[test]
    fn clearing_releases_every_node_and_the_selection() {
        let mut t = tree(0);
        t.selected = Some(vec![1]);
        let events = t.clear();
        assert_eq!(events[0], TreeEvent::Selected(None));
        assert_eq!(events.len(), 6);
        assert_eq!(events[5], TreeEvent::Released(b"e".to_vec()));
        assert!(t.roots.is_empty());
    }
}
