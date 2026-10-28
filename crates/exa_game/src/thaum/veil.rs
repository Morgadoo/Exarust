//! The body-side half of the Displacement domain: the shell a constant
//! displacement wears, the image it leaves behind, the parts it hides, the
//! collision bits it turns off, and the hit veto that compares one blow's
//! roll with the phased streak.
//!
//! - **Constant displacement** ([`ConstantVeil::start`]): a shell effect
//!   is made and starts dead (visibility word `0x8000_0000`); every physics
//!   node of the body gains collision bit `0x4000`; a tendril image is made
//!   (node flags `0x2100_0000`, also dead at first, eight spare strands, its
//!   scale multiplied by the body's size factor); and the `thmdsp_cns01`
//!   hum starts silent at normal pitch, looping from a random point
//!   `0.8 × its length × a draw` seconds in (the product is taken in double
//!   precision).
//! - **The image's end** ([`TendrilImage::end`]): it marks every strand
//!   record with bit 8, sets the image's fade to zero and tells its node to
//!   play `0x10`. The restore tick ends the image this way (the original
//!   finds it among the body's children by its class). A timed
//!   segment shows `clamp((elapsed − start) / (end − start), 0, 1)`,
//!   inverted unless the segment is reversed, and draws on its side of one
//!   half ([`ShellTimeline`]).
//! - **The hit veto** ([`ConstantVeil::hit`]): a percentage draw either
//!   passes the blow through or not, following [`PhaseStreak`]; a pass
//!   plays `thmdsp_phs` at pitch `0.9 + 0.2·r` (double precision) and volume
//!   0.6, sets collision bits `0xC000` on the struck node and its
//!   descendants, makes a wisp at the contact and lists the node with the
//!   stamp. The restore tick ([`ConstantVeil::restore`]) gives a listed
//!   node its collisions back once more than 500 ms have passed, and ends
//!   the tendril image of the node's owner (the last child that is one).
//! - **Visibility** ([`set_visibility`]): a mesh node (kind 9) that is not
//!   dead carries bit `0x10` or `0x20`; setting one clears the node's
//!   hidden flag, and clearing them hides a node that has subobjects by
//!   setting its low render bit.
//! - **The shell's parts** ([`ShellParts`]): the shell effect gathers every
//!   mesh (kind 9, neither skipped by node flag `0x80000` nor dead) and skin
//!   (kind 5) node of a body by reference, in walk order. A mesh is pruned
//!   by its children's `0x40` part flags: mixed flags move it behind the
//!   rest, all-set flags move it behind the kept ones, none leaves it in
//!   place. Fading out marks each mesh `0x4` and sets `0x10`; restoring sets
//!   each mesh's and skin's fade to one and shows the meshes with `0x20`.
//! - **Restoring shells** ([`restore_shells`], [`clear_shells`]): a child
//!   node whose shell effect carries mark `0x10000000` gets render bit
//!   `0x1000` (clearing `0x900`) and a 1000-unit stamp, any other loses
//!   render bit `0x800`; the clear takes bit `0x1000` away.
//!
//! Not modelled: the tendril image's per-bone anchors beyond
//! [`super::visuals::tendril_parts`], the shell's and the tendrils' draw
//! passes, the audio instance, the delayed wisp event, and the global list
//! being shared by every displaced body (each [`ConstantVeil`] keeps its
//! own). The module is not wired into the running game yet: no power
//! class calls it.
//!
//! Evidence: the constant-displacement start, its hit veto and restore
//! tick, the shell's creator, end and timeline, the visibility setter, the
//! part gather and prune passes, the fades and the shell restore and clear
//! walks, read from `Exanima.exe`.

use super::displacement::{DISPLACED_BIT, PHASED_BITS, PhaseStreak, PhasedWeapons};
use super::engine::Roll;

/// The node flags a tendril image starts with.
pub const TENDRIL_FLAGS: u32 = 0x2100_0000;
/// The end event the tendril image's node is told.
pub const IMAGE_END: u32 = 0x10;
/// The visibility word of a node that starts dead.
pub const DEAD: u32 = 0x8000_0000;
/// The hum's first play is silent.
pub const HUM_VOLUME: f32 = 0.0;
/// The hum plays at normal pitch.
pub const HUM_PITCH: f32 = 1.0;
/// The node-flag bit a physics node carries.
pub const PHYSICS: u32 = 0x2000;
/// The node-flag bit that keeps a mesh out of the shell's parts.
pub const SKIPPED: u32 = 0x0008_0000;
/// The render-flag bit that marks a node with subobjects.
pub const SUBOBJECTS: u32 = 0x0100_0000;
/// The part-flag bit the prune pass reads.
pub const PRUNE_FLAG: u32 = 0x40;
/// The mark the image's end sets on each strand record.
pub const ENDED: u32 = 0x8;

pub use super::effect_material::{AURA_FLAGS, SHARED_SHELL_FLAG as SHARED, SHELL_FLAG};

/// A node of a body the displacement routines walk.
#[derive(Debug, Clone, PartialEq)]
pub struct VeilNode {
    /// The node's class word: `status & 0x5f` names the kind (9 a mesh,
    /// 5 a skin).
    pub status: u32,
    /// The node flags: [`PHYSICS`] takes the body's collision bits and
    /// passes them on, [`SKIPPED`] keeps a mesh out of the shell's parts.
    pub flags: u32,
    /// The collision bits.
    pub collision: u32,
    /// The visibility bits (0x10 shown, 0x20 hidden, 0x4 fading out,
    /// [`DEAD`]).
    pub visibility: u32,
    /// The node's fade value.
    pub fade: f32,
    /// The render flags: bit 1 hides a node with subobjects,
    /// [`SUBOBJECTS`] says it has some.
    pub render_flags: u32,
    /// The part flags the prune pass reads ([`PRUNE_FLAG`]).
    pub part_flags: u32,
    /// The bone kind the tendril geometry reads (0 to 18).
    pub bone: usize,
    /// The child nodes.
    pub children: Vec<VeilNode>,
}

impl VeilNode {
    /// A physics mesh node with the given children.
    pub fn mesh(children: Vec<VeilNode>) -> Self {
        Self {
            status: 9,
            flags: PHYSICS,
            collision: 0,
            visibility: 0,
            fade: 0.0,
            render_flags: if children.is_empty() { 0 } else { SUBOBJECTS },
            part_flags: 0,
            bone: 0,
            children,
        }
    }

    /// A skin node.
    pub fn skin() -> Self {
        Self {
            status: 5,
            ..Self::mesh(Vec::new())
        }
    }

    /// A node of another kind.
    pub fn other(status: u32) -> Self {
        Self {
            status,
            ..Self::mesh(Vec::new())
        }
    }

    /// The node with a bone kind.
    pub fn on_bone(mut self, bone: usize) -> Self {
        self.bone = bone;
        self
    }

    /// Whether the node takes the body's collision bits.
    pub fn physics(&self) -> bool {
        self.flags & PHYSICS != 0
    }

    /// Walks this node and its children.
    pub fn walk(&self, visit: &mut dyn FnMut(&VeilNode)) {
        visit(self);
        for child in &self.children {
            child.walk(visit);
        }
    }

    /// Walks this node and its children, changing them.
    pub fn walk_mut(&mut self, visit: &mut impl FnMut(&mut VeilNode)) {
        visit(self);
        for child in &mut self.children {
            child.walk_mut(visit);
        }
    }
}

/// Adds collision `mask` to a physics node and, through it, to its
/// children; a node that is not a physics node stops the walk.
pub fn add_physics_bits(node: &mut VeilNode, mask: u32) {
    if node.physics() {
        node.collision |= mask;
        for child in &mut node.children {
            add_physics_bits(child, mask);
        }
    }
}

/// Adds collision `mask` to the node and every child, physics or not.
pub fn apply_bits(node: &mut VeilNode, mask: u32) {
    node.collision |= mask;
    for child in &mut node.children {
        apply_bits(child, mask);
    }
}

/// Clears collision `mask` from the node and every child.
pub fn clear_bits(node: &mut VeilNode, mask: u32) {
    node.collision &= !mask;
    for child in &mut node.children {
        clear_bits(child, mask);
    }
}

/// Sets the node's visibility bits: a live mesh part (`kind 9`) takes one
/// of 0x10 and 0x20. Setting one clears its hidden flag; clearing them
/// sets the hidden flag of a part with subobjects.
pub fn set_visibility(part: &mut VeilNode, mask: u32) {
    if part.status & 0x5f != 9 || part.visibility & DEAD != 0 {
        return;
    }
    if part.visibility & 0x30 == 0 {
        if mask != 0 {
            part.visibility |= mask;
            part.render_flags &= !1;
            return;
        }
    } else if mask == 0 {
        if part.render_flags & SUBOBJECTS != 0 {
            part.render_flags |= 1;
        }
        part.visibility = (part.visibility & !0x30) | mask;
        return;
    }
    part.visibility = (part.visibility & !0x30) | mask;
}

/// The shell a constant displacement wears.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShellWear {
    pub fade: f32,
    /// The node's event word.
    pub event: u32,
    /// The shell's state word: it starts dead, with the shell material's
    /// 0x9000 bits.
    pub visibility: u32,
    /// The shell node's flags word: the creator's 0x20 and, for a shell
    /// asked for as shared, 0x40000.
    pub flags: u32,
    /// The shell belongs to a constant displacement and is shared with a
    /// vanish.
    pub shared: bool,
}

impl ShellWear {
    /// A shell as the constant displacement leaves it at the start.
    pub fn new() -> Self {
        Self {
            fade: 0.0,
            event: 0,
            visibility: DEAD | AURA_FLAGS,
            flags: SHELL_FLAG,
            shared: false,
        }
    }

    /// The shell a constant displacement starts: the same node, shared.
    pub fn shared() -> Self {
        Self {
            flags: SHELL_FLAG | SHARED,
            shared: true,
            ..Self::new()
        }
    }
}

impl Default for ShellWear {
    fn default() -> Self {
        Self::new()
    }
}

/// The tendril image a constant displacement leaves behind.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TendrilImage {
    /// The effect node's flags.
    pub flags: u32,
    /// The image node's visibility word: it starts dead.
    pub visibility: u32,
    /// The first strand record the anchors fill.
    pub first: usize,
    /// The last strand record: the anchors' count plus the eight spare.
    pub last: usize,
    /// Strands kept beyond its anchors.
    pub spare: usize,
    /// The image's scale: the geometry's own times the body's size factor.
    pub scale: f32,
    /// The image's fade.
    pub fade: f32,
    /// The image node's event word.
    pub event: u32,
    /// Every strand record carries [`ENDED`].
    pub ended: bool,
}

impl TendrilImage {
    /// An image on a body of the given size factor, over a geometry whose
    /// own scale is `base`; `anchors` is the strands the body's bone kinds
    /// filled.
    pub fn new(base: f32, size_factor: f32, anchors: usize) -> Self {
        Self {
            flags: TENDRIL_FLAGS,
            visibility: DEAD,
            first: anchors,
            last: anchors + super::displacement::SPARE_STRANDS,
            spare: super::displacement::SPARE_STRANDS,
            scale: base * size_factor,
            fade: 0.0,
            event: 0,
            ended: false,
        }
    }

    /// The image's end: every strand record is marked [`ENDED`], the fade
    /// returns to zero and the node is told [`IMAGE_END`].
    pub fn end(&mut self) {
        self.ended = true;
        self.fade = 0.0;
        self.event = IMAGE_END;
    }
}

/// The node at a path, if it exists.
pub fn find<'a>(root: &'a VeilNode, path: &[usize]) -> Option<&'a VeilNode> {
    let mut node = root;
    for step in path {
        node = node.children.get(*step)?;
    }
    Some(node)
}

/// The shell's parts: paths from a body's root to the mesh and skin nodes
/// the shell hides and shows. The original keeps pointers into the live
/// body; a path stands for one here.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShellParts {
    /// The mesh nodes (kind 9), in the walk's order.
    pub meshes: Vec<Vec<usize>>,
    /// The skin nodes (kind 5).
    pub skins: Vec<Vec<usize>>,
}

impl ShellParts {
    /// Gathers a body's parts: a live mesh without [`SKIPPED`] and every
    /// skin, node before children.
    pub fn gather(root: &VeilNode) -> Self {
        let mut parts = Self::default();
        let mut path = Vec::new();
        gather_parts(root, &mut path, &mut parts);
        prune_meshes(root, &mut parts.meshes);
        parts
    }

    /// Fades the meshes out: each is marked fading and shown with 0x10.
    pub fn fade_out(&self, root: &mut VeilNode) {
        for path in &self.meshes {
            if let Some(part) = find_mut(root, path) {
                part.visibility |= 4;
                set_visibility(part, 0x10);
            }
        }
    }

    /// Restores the parts: each mesh's fade becomes one and it is shown
    /// with 0x20; each skin's fade becomes one.
    pub fn fade_restore(&self, root: &mut VeilNode) {
        for path in &self.meshes {
            if let Some(part) = find_mut(root, path) {
                part.fade = 1.0;
                set_visibility(part, 0x20);
            }
        }
        for path in &self.skins {
            if let Some(part) = find_mut(root, path) {
                part.fade = 1.0;
            }
        }
    }
}

/// Collects a node's mesh and skin parts into `parts`, then those of its
/// children.
pub fn gather_parts(node: &VeilNode, path: &mut Vec<usize>, parts: &mut ShellParts) {
    match node.status & 0x5f {
        9 => {
            if node.flags & SKIPPED == 0 && node.visibility & DEAD == 0 {
                parts.meshes.push(path.clone());
            }
        }
        5 => parts.skins.push(path.clone()),
        _ => {}
    }
    for (index, child) in node.children.iter().enumerate() {
        path.push(index);
        gather_parts(child, path, parts);
        path.pop();
    }
}

/// Orders the meshes by their children's [`PRUNE_FLAG`]: a mesh with none
/// set stays at the front, one with all set moves behind the kept ones, and
/// one with both moves behind both. The list keeps every mesh; the
/// original's scan and tail cursors walk the tail as they swap.
pub fn prune_meshes(root: &VeilNode, meshes: &mut [Vec<usize>]) {
    let mut scan = meshes.len();
    let mut tail = meshes.len();
    let mut index = 0;
    while index < scan {
        let mut seen = 0u32;
        if let Some(mesh) = find(root, &meshes[index]) {
            for child in &mesh.children {
                seen |= if child.part_flags & PRUNE_FLAG == 0 {
                    1
                } else {
                    2
                };
            }
        }
        if seen >= 3 {
            scan -= 1;
            tail -= 1;
            meshes.swap(index, tail);
        } else if seen == 2 {
            scan -= 1;
            meshes.swap(index, scan);
        } else {
            index += 1;
        }
    }
}

/// A body's constant displacement.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstantVeil {
    /// The body's root node.
    pub body: VeilNode,
    pub shell: ShellWear,
    /// The meshes and skins the shell fades.
    pub parts: ShellParts,
    pub image: TendrilImage,
    /// The phased nodes, each with the stamp its collisions return.
    pub phased: PhasedWeapons,
    /// The streak deciding each blow.
    pub streak: PhaseStreak,
    /// Where in the hum's loop it starts, in seconds.
    pub hum_start: f32,
    /// The hum's first volume (silent) and pitch (normal).
    pub hum_volume: f32,
    pub hum_pitch: f32,
    /// The hum loops from its first play (bit 0x10 in its flags).
    pub hum_loop: bool,
    /// The body's size factor, which the shell and the image keep.
    pub size_factor: f32,
    /// The shell's fade may run (its hold byte starts zero).
    pub shell_can_fade: bool,
    /// The shell's fade callback was appended to the body's 100-room list.
    pub fade_hook: bool,
    /// The body's hit callback is the veil's.
    pub hit_callback: bool,
    /// The body's physics tick restores the phased nodes.
    pub restore_callback: bool,
    /// The wisp the last passing blow made, if any.
    pub last_wisp: Option<[f32; 3]>,
    /// The passing blow's sound pitch.
    pub last_pitch: Option<f32>,
}

impl ConstantVeil {
    /// Starts a constant displacement on `body`. `hum_length` is the hum
    /// sound's length in seconds, `size_factor` the body's size factor and
    /// `roll` draws the hum's start point. The start always asks for an
    /// unshared shell.
    pub fn start(body: VeilNode, hum_length: f32, size_factor: f32, roll: &mut Roll) -> Self {
        let mut body = body;
        add_physics_bits(&mut body, DISPLACED_BIT);
        let parts = ShellParts::gather(&body);
        let anchors = body_anchors(&body);
        let hum_start = (f64::from(hum_length) * 0.8 * f64::from(roll.unit())) as f32;
        Self {
            body,
            shell: ShellWear::new(),
            parts,
            image: TendrilImage::new(1.0, size_factor, anchors),
            phased: PhasedWeapons::default(),
            streak: PhaseStreak::default(),
            hum_start,
            hum_volume: HUM_VOLUME,
            hum_pitch: HUM_PITCH,
            hum_loop: true,
            size_factor,
            // The shell's fade word starts zero and may run.
            shell_can_fade: true,
            fade_hook: true,
            hit_callback: true,
            restore_callback: true,
            last_wisp: None,
            last_pitch: None,
        }
    }

    /// One blow on a node: true when it passes through. On a pass the
    /// node and its children gain [`PHASED_BITS`], the node is listed with
    /// `now`, and `contact` is where the wisp sits.
    pub fn hit(
        &mut self,
        node_path: &[usize],
        now: u64,
        contact: [f32; 3],
        roll: &mut Roll,
    ) -> bool {
        let Some(pitch) = self.streak.pass(roll) else {
            return false;
        };
        self.last_pitch = Some(pitch);
        if let Some(node) = find_mut(&mut self.body, node_path) {
            apply_bits(node, PHASED_BITS);
        }
        self.phased.add(path_key(node_path), now);
        self.last_wisp = Some(contact);
        true
    }

    /// Returns the collisions of the nodes whose 500 ms are over at `now`.
    /// A return ends the body's tendril image.
    pub fn restore(&mut self, now: u64) -> Vec<u64> {
        let freed = self.phased.restore(now);
        if !freed.is_empty() {
            for key in &freed {
                if let Some(node) = find_key_mut(&mut self.body, *key) {
                    clear_bits(node, PHASED_BITS);
                }
            }
            self.image.end();
        }
        freed
    }

    /// Every physics node's collision bits gained the displaced bit.
    pub fn has_displaced_bit(&self) -> bool {
        let mut all = true;
        self.body.walk(&mut |node| {
            if node.physics() && node.collision & DISPLACED_BIT == 0 {
                all = false;
            }
        });
        all
    }
}

/// The strand records a body's bones fill: every bone kind contributes its
/// anchor table's strands, walking the nodes in order.
pub fn body_anchors(root: &VeilNode) -> usize {
    let mut count = 0;
    root.walk(&mut |node| {
        count += super::displacement::wisp_anchors(node.bone).len();
    });
    count
}

/// A stable key for a node's path: a one-byte marker above its steps, one
/// byte a step. A step of 256 or more does not fit.
fn path_key(path: &[usize]) -> u64 {
    let mut key = 1u64 << (8 * path.len());
    for (depth, step) in path.iter().enumerate().take(7) {
        key |= (*step as u64 & 0xff) << (8 * depth);
    }
    key
}

/// The steps of a path key.
fn path_steps(key: u64) -> Vec<usize> {
    let mut marker = 0;
    while marker < 8 && key >> (8 * marker) != 1 {
        marker += 1;
    }
    (0..marker)
        .map(|depth| ((key >> (8 * depth)) & 0xff) as usize)
        .collect()
}

/// The node at a path, if it exists.
pub fn find_mut<'a>(root: &'a mut VeilNode, path: &[usize]) -> Option<&'a mut VeilNode> {
    let mut node = root;
    for step in path {
        node = node.children.get_mut(*step)?;
    }
    Some(node)
}

fn find_key_mut(root: &mut VeilNode, key: u64) -> Option<&mut VeilNode> {
    find_mut(root, &path_steps(key))
}

/// A shell node of a body: its render words and what its restore stamps.
#[derive(Debug, Clone, PartialEq)]
pub struct ShellNode {
    /// The node's render word.
    pub render: u32,
    /// The node's mark word; bit 28 is the shell mark.
    pub mark: u32,
    /// The restore's stamp.
    pub stamp: u32,
    pub children: Vec<ShellNode>,
}

impl ShellNode {
    /// Whether this node carries the shell mark.
    pub fn carries_shell(&self) -> bool {
        self.mark & 0x1000_0000 != 0
    }
}

/// Restores a body's shell nodes: one without the mark clears render bit
/// 0x800; one with it gains render bits 0x1000 and the stamp 1000.
pub fn restore_shells(nodes: &mut [ShellNode]) {
    for node in nodes {
        if node.carries_shell() {
            node.render = (node.render & !0x900) | 0x1000;
            node.stamp = 1000;
        } else {
            node.render &= !0x800;
        }
        restore_shells(&mut node.children);
    }
}

/// Clears render bit 0x1000 from every carrying node.
pub fn clear_shells(nodes: &mut [ShellNode]) {
    for node in nodes {
        if node.carries_shell() {
            node.render &= !0x1000;
        }
        clear_shells(&mut node.children);
    }
}

/// A shell timeline segment: a time window and its flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellSegment {
    pub flags: u32,
    /// The segment's start and end, in the body's elapsed milliseconds.
    pub start: u32,
    pub end: u32,
}

/// The segment's lower draw-side bit.
pub const SEGMENT_DRAW_ON: u32 = 0x200;
/// The segment's shown value follows the elapsed time.
pub const SEGMENT_TIMED: u32 = 0x400;
/// The timed value is inverted.
pub const SEGMENT_REVERSE: u32 = 0x800;
/// The segment draws while its shown value is on its side.
pub const SEGMENT_DRAW: u32 = 0x600;
/// An untimed segment is fully shown.
pub const SEGMENT_FULL: u32 = 0x1000;

/// The timed shown value: the elapsed fraction clamped to 0 to 1, then
/// inverted unless the segment is reversed.
pub fn segment_shown(segment: &ShellSegment, elapsed: u32) -> f32 {
    if segment.flags & SEGMENT_TIMED == 0 {
        return if segment.flags & SEGMENT_FULL != 0 {
            1.0
        } else {
            0.0
        };
    }
    let span = f64::from(segment.end) - f64::from(segment.start);
    let at = f64::from(elapsed) - f64::from(segment.start);
    let t = (at / span) as f32;
    // The original's clamp is a max then a min, so a not-a-number becomes
    // zero rather than staying not a number as `f32::clamp` would.
    #[allow(clippy::manual_clamp)]
    let t = t.max(0.0).min(1.0);
    if segment.flags & SEGMENT_REVERSE != 0 {
        t
    } else {
        1.0 - t
    }
}

/// A displacement shell's timed segments.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ShellTimeline {
    pub segments: Vec<ShellSegment>,
    /// The segment being shown.
    pub index: usize,
    /// When the timeline started.
    pub start: u32,
    /// The current shown value.
    pub shown: f32,
    /// The segment whose callback ran this step.
    pub fired: Option<usize>,
    /// Whether the fired segment was on the closing side of its draw
    /// window when it ended (read from the segment that ended, not from the
    /// one that follows).
    pub fired_closing: bool,
}

impl ShellTimeline {
    /// Steps the shown value: the current segment's value and whether its
    /// draw side is above one half.
    pub fn step(&mut self, now: u32) -> (f32, bool) {
        let Some(segment) = self.segments.get(self.index).copied() else {
            return (0.0, false);
        };
        let elapsed = now.wrapping_sub(self.start);
        self.shown = segment_shown(&segment, elapsed);
        let draw = segment.flags & SEGMENT_DRAW != 0 && self.shown > 0.5;
        (self.shown, draw)
    }

    /// Advances past a segment whose end has passed: its callback runs and
    /// the next segment shows. Returns the fired segment.
    pub fn advance(&mut self, now: u32) -> Option<usize> {
        let segment = *self.segments.get(self.index)?;
        let elapsed = now.wrapping_sub(self.start);
        if segment.end < elapsed {
            let fired = self.index;
            self.fired_closing = self.closing_draw();
            self.index += 1;
            self.fired = Some(fired);
            Some(fired)
        } else {
            None
        }
    }

    /// Whether the shell's subobject draws at half the shown value: the
    /// closing side of the draw window.
    pub fn closing_draw(&self) -> bool {
        let Some(segment) = self.segments.get(self.index) else {
            return false;
        };
        segment.flags & SEGMENT_DRAW != 0 && self.shown <= 0.5
    }

    /// The subobject's fade: one less the shown value.
    pub fn fade(&self) -> f32 {
        1.0 - self.shown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wear() -> ShellWear {
        ShellWear::new()
    }

    #[test]
    fn a_shell_starts_at_zero() {
        let shell = wear();
        assert_eq!(shell.fade, 0.0);
        assert_eq!(shell.event, 0);
    }

    #[test]
    fn an_image_ends_with_its_event_and_marked_strands() {
        let mut image = TendrilImage::new(1.0, 1.0, 3);
        assert!(!image.ended);
        image.fade = 0.4;
        image.end();
        assert_eq!((image.fade, image.event), (0.0, IMAGE_END));
        assert!(image.ended);
    }

    #[test]
    fn a_displaced_body_gains_the_physics_bit_only_on_physics_nodes() {
        let mut body = VeilNode::mesh(vec![
            VeilNode {
                flags: 0,
                ..VeilNode::mesh(vec![])
            },
            VeilNode::mesh(vec![VeilNode::skin()]),
        ]);
        add_physics_bits(&mut body, DISPLACED_BIT);
        assert_eq!(body.collision, DISPLACED_BIT);
        assert_eq!(body.children[0].collision, 0);
        assert_eq!(body.children[1].collision, DISPLACED_BIT);
        assert_eq!(body.children[1].children[0].collision, DISPLACED_BIT);
        clear_bits(&mut body, DISPLACED_BIT);
        body.walk(&mut |node| assert_eq!(node.collision & DISPLACED_BIT, 0));
        apply_bits(&mut body.children[0], PHASED_BITS);
        assert_eq!(body.children[0].collision, PHASED_BITS);
    }

    #[test]
    fn visibility_clears_and_sets_the_hidden_flag() {
        let mut part = VeilNode::mesh(vec![VeilNode::mesh(vec![])]);
        set_visibility(&mut part, 0x10);
        assert_eq!(part.visibility, 0x10);
        assert_eq!(part.render_flags & 1, 0);
        set_visibility(&mut part, 0x20);
        assert_eq!(part.visibility, 0x20);
        set_visibility(&mut part, 0);
        assert_eq!(part.visibility, 0);
        assert_eq!(part.render_flags & 1, 1);
        let mut skin = VeilNode::skin();
        set_visibility(&mut skin, 0x10);
        assert_eq!(skin.visibility, 0);
        let mut dead = VeilNode::mesh(vec![]);
        dead.visibility = 0x8000_0000;
        set_visibility(&mut dead, 0x10);
        assert_eq!(dead.visibility, 0x8000_0000);
    }

    #[test]
    fn fading_marks_and_shows_the_parts() {
        let mut body = VeilNode::mesh(vec![
            VeilNode::mesh(vec![VeilNode::mesh(vec![])]),
            VeilNode::skin(),
        ]);
        let parts = ShellParts::gather(&body);
        assert_eq!(parts.meshes.len(), 3);
        assert_eq!(parts.skins, vec![vec![1]]);
        parts.fade_out(&mut body);
        // The fading bit is on the visibility word, with the shown bit.
        assert_eq!(body.children[0].visibility, 0x14);
        assert_eq!(body.children[0].render_flags & 4, 0);
        assert_eq!(body.children[1].visibility, 0);
        parts.fade_restore(&mut body);
        assert_eq!(body.children[0].visibility, 0x24);
        assert_eq!(body.children[0].fade, 1.0);
        // The skin's own fade is restored too.
        assert_eq!(body.children[1].fade, 1.0);
    }

    #[test]
    fn the_shell_parts_take_meshes_and_skins() {
        let body = VeilNode::mesh(vec![
            VeilNode::mesh(vec![]),
            VeilNode::other(3),
            VeilNode::skin(),
            VeilNode {
                flags: PHYSICS | SKIPPED,
                ..VeilNode::mesh(vec![])
            },
            VeilNode {
                visibility: DEAD,
                ..VeilNode::mesh(vec![])
            },
        ]);
        let parts = ShellParts::gather(&body);
        // The body's own root counts: it is a live mesh.
        assert_eq!(parts.meshes, vec![vec![], vec![0]]);
        assert_eq!(parts.skins, vec![vec![2]]);
        assert_eq!(body_anchors(&body), 0);
    }

    #[test]
    fn the_prune_orders_the_meshes_by_their_part_flags() {
        let kid = |flags: u32| VeilNode {
            part_flags: flags,
            ..VeilNode::mesh(vec![])
        };
        let full = VeilNode::mesh(vec![kid(0x40), kid(0x40)]);
        let mixed = VeilNode::mesh(vec![kid(0), kid(0x40)]);
        // Mesh 0 and 3 have no children, 1 has all set, 2 is mixed.
        let body = VeilNode::mesh(vec![
            VeilNode::mesh(vec![]),
            full,
            mixed,
            VeilNode::mesh(vec![]),
        ]);
        let mut meshes = vec![vec![0], vec![1], vec![2], vec![3]];
        prune_meshes(&body, &mut meshes);
        // Nothing set stays in front in order, all-set follows, mixed is
        // last; the list keeps all four.
        assert_eq!(meshes, vec![vec![0], vec![3], vec![1], vec![2]]);
        // A render flag is not a part flag.
        let mut other = VeilNode::mesh(vec![VeilNode {
            render_flags: 0x40,
            ..VeilNode::mesh(vec![])
        }]);
        other.children.push(other.children[0].clone());
        let mut one = vec![vec![]];
        prune_meshes(&other, &mut one);
        assert_eq!(one, vec![Vec::<usize>::new()]);
    }

    #[test]
    fn the_tendril_image_starts_dead_and_scales_with_the_body() {
        let image = TendrilImage::new(0.5, 1.5, 3);
        assert_eq!(image.flags, 0x2100_0000);
        assert_eq!(image.visibility, 0x8000_0000);
        assert_eq!(image.scale, 0.75);
        assert_eq!((image.first, image.last, image.spare), (3, 11, 8));
        assert_eq!(ShellWear::new().visibility, 0x8000_9000);
        assert_eq!(ShellWear::new().flags, 0x20);
        let shared = ShellWear::shared();
        assert_eq!(shared.flags, 0x4_0020);
        assert!(shared.shared);
    }

    #[test]
    fn a_constant_displacement_hides_its_weapon_for_half_a_second() {
        let mut draw = Roll(0x26b0_5733);
        // The product is taken in double precision.
        let expected = (2.0f64 * 0.8 * f64::from(draw.unit())) as f32;
        let mut roll = Roll(0x26b0_5733);
        let body = VeilNode::mesh(vec![VeilNode::mesh(vec![])]);
        let mut veil = ConstantVeil::start(body, 2.0, 1.0, &mut roll);
        assert!(veil.has_displaced_bit());
        assert_eq!(veil.hum_start, expected);
        assert_eq!((veil.hum_volume, veil.hum_pitch), (0.0, 1.0));
        assert!(veil.hum_loop && veil.shell_can_fade);
        assert!(veil.fade_hook && veil.hit_callback && veil.restore_callback);
        assert!(!veil.shell.shared);
        assert_eq!(veil.parts.meshes.len(), 2);
        assert_eq!(veil.image.visibility, DEAD);
        // The first draw passes (the streak starts at zero): the node and
        // its child lose 0xC000.
        let through = veil.hit(&[0], 1000, [1.0, 2.0, 3.0], &mut Roll(0x26b0_5733));
        assert!(through);
        assert_eq!(veil.body.children[0].collision & PHASED_BITS, PHASED_BITS);
        assert_eq!(veil.last_wisp, Some([1.0, 2.0, 3.0]));
        assert_eq!(veil.streak.0, 1);
        assert!(veil.restore(1500).is_empty());
        assert_eq!(veil.image.event, 0);
        assert_eq!(veil.restore(1501), vec![path_key(&[0])]);
        assert_eq!(veil.body.children[0].collision & PHASED_BITS, 0);
        assert_eq!(veil.image.event, IMAGE_END);
        assert!(veil.image.ended);
        assert_eq!(veil.phased.list.len(), 0);
    }

    #[test]
    fn a_shell_segment_follows_its_timed_window() {
        let segment = ShellSegment {
            flags: SEGMENT_TIMED | SEGMENT_REVERSE | SEGMENT_DRAW,
            start: 100,
            end: 300,
        };
        assert_eq!(segment_shown(&segment, 0), 0.0);
        assert_eq!(segment_shown(&segment, 100), 0.0);
        assert_eq!(segment_shown(&segment, 200), 0.5);
        assert_eq!(segment_shown(&segment, 300), 1.0);
        assert_eq!(segment_shown(&segment, 400), 1.0);
        let reverse = ShellSegment {
            flags: SEGMENT_TIMED,
            start: 0,
            end: 100,
        };
        assert_eq!(segment_shown(&reverse, 25), 0.75);
        let full = ShellSegment {
            flags: SEGMENT_FULL,
            start: 0,
            end: 1,
        };
        assert_eq!(segment_shown(&full, 0), 1.0);
        assert_eq!(segment_shown(&ShellSegment { flags: 0, ..full }, 0), 0.0);
    }

    #[test]
    fn the_shell_timeline_advances_and_draws_on_its_side() {
        let mut timeline = ShellTimeline {
            segments: vec![
                ShellSegment {
                    flags: SEGMENT_TIMED | SEGMENT_REVERSE | SEGMENT_DRAW,
                    start: 0,
                    end: 100,
                },
                ShellSegment {
                    flags: SEGMENT_FULL | SEGMENT_DRAW_ON,
                    start: 0,
                    end: 200,
                },
            ],
            start: 1000,
            ..ShellTimeline::default()
        };
        assert_eq!(timeline.step(1050), (0.5, false));
        assert!(timeline.closing_draw());
        assert_eq!(timeline.step(1101), (1.0, true));
        assert!(!timeline.closing_draw());
        assert!(timeline.advance(1100).is_none());
        assert_eq!(timeline.advance(1101), Some(0));
        // The first segment ended past its half: the draw is not closing.
        assert!(!timeline.fired_closing);
        assert_eq!(timeline.index, 1);
        assert_eq!(timeline.fired, Some(0));
        assert_eq!(timeline.step(1201), (1.0, true));
        assert_eq!(timeline.fade(), 0.0);
    }

    #[test]
    fn the_restore_tick_marks_the_shell_nodes() {
        let mut nodes = vec![
            ShellNode {
                render: 0x800,
                mark: 0,
                stamp: 0,
                children: vec![ShellNode {
                    render: 0,
                    mark: 0x1000_0000,
                    stamp: 0,
                    children: vec![],
                }],
            },
            ShellNode {
                render: 0xffff,
                mark: 0x1000_0000,
                stamp: 0,
                children: vec![],
            },
        ];
        restore_shells(&mut nodes);
        assert_eq!(nodes[0].render, 0);
        assert_eq!(nodes[0].children[0].render, 0x1000);
        assert_eq!(nodes[0].children[0].stamp, 1000);
        assert_eq!(nodes[1].render & 0x1000, 0x1000);
        clear_shells(&mut nodes);
        assert_eq!(nodes[1].render & 0x1000, 0);
        assert_eq!(nodes[0].children[0].render & 0x1000, 0);
    }
    #[test]
    fn the_tendril_anchors_follow_the_bones() {
        let body = VeilNode::mesh(vec![
            VeilNode::mesh(vec![]).on_bone(1),
            VeilNode::mesh(vec![VeilNode::mesh(vec![]).on_bone(9)]).on_bone(2),
            VeilNode::mesh(vec![]).on_bone(19),
            VeilNode::mesh(vec![]).on_bone(8),
        ]);
        // Bone 1 has four anchors, 2 one, 9 four, 19 none and 8 none.
        assert_eq!(body_anchors(&body), 9);
        let image = TendrilImage::new(2.0, 0.5, body_anchors(&body));
        assert_eq!((image.first, image.last), (9, 17));
        assert_eq!(image.scale, 1.0);
    }
}
