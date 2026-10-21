//! The body-side half of the Displacement domain: the shell a constant
//! displacement wears, the image it leaves behind, the parts it hides, the
//! collision bits it turns off, and the hit veto that compares one blow's
//! roll with the phased streak.
//!
//! - **The shell** ([`ShellWear`]): a `_EffectShell01` effect whose fade
//!   starts at zero, whose node starts at flags `0x21000000` and whose end
//!   event sets the fade to zero and asks the node to play `0x10`. The body
//!   hands the shell its fade tick; the effect's own subobjects are marked
//!   with bit 8 when it ends. A timed segment shows
//!   `clamp((elapsed − start) / (end − start), 0, 1)`, inverted unless the
//!   segment is reversed, and draws on its side of one half.
//! - **Constant displacement** ([`ConstantVeil::start`]): every physics node
//!   of the body gains collision bit `0x4000`, the shell is made, the
//!   left-behind image is gathered from the body's mesh (`kind 9`) and skin
//!   (`kind 5`) nodes, and the `thmdsp_cns01` hum loops at a pitch of
//!   `0.8 × its length × a random draw`, its first play at volume zero.
//! - **The hit veto** ([`ConstantVeil::hit`]): a percentage draw either
//!   passes the blow through or not, following [`PhaseStreak`]; a pass
//!   plays `thmdsp_phs` at pitch `0.9 + 0.2·r`, sets collision bits
//!   `0xC000` on every part of the struck node, lists the node with the
//!   stamp, and makes a wisp at the contact. A weapon's collisions come
//!   back 500 ms after the blow ([`ConstantVeil::restore`]), which also
//!   ends the shell.
//! - **Visibility** ([`set_visibility`]): a mesh node (`kind 9`) that is not
//!   dead carries bits `0x10` or `0x20`; setting one clears the node's
//!   hidden flag, and clearing them hides a node that has subobjects by
//!   setting its low hidden bit. Fading out first marks the node `0x4` and
//!   sets `0x10`.
//! - **The left-behind image** ([`gather_parts`], [`prune_meshes`]): the
//!   image takes every mesh and skin node; a mesh is pruned by its
//!   subobjects' `0x40` flags: mixed flags drop it, all-set flags move it
//!   to the front, none leaves it in place. The image keeps eight spare
//!   strands and scales by the body's fade length.
//! - **Restoring shells** ([`restore_shells`], [`clear_shells`]): a child
//!   node of a body is patched when its `0x10000000` mark is set: the
//!   restore sets render bits `0x1000` and a 1000-unit stamp, the clear
//!   takes bit `0x1000` away.
//!
//! Evidence: the shell creator and its end, the constant-displacement start
//! and its hit veto, the restore tick, the visibility setter, the part
//! gathers and the pruning pass, read from `Exanima.exe`.

use super::displacement::{PhaseStreak, PhasedWeapons};
use super::engine::Roll;

/// The material a displacement shell wears.
pub const SHELL_MATERIAL: &str = "_EffectShell01";
/// The material the left-behind tendrils wear.
pub const TENDRIL_MATERIAL: &str = "_TDispMat01";
/// The picture the tendrils draw.
pub const TENDRIL_TEXTURE: &str = "FlameletsA3.rfi";
/// The shell node's starting flags.
pub const SHELL_FLAGS: u32 = 0x2100_0000;
/// The end event the shell node is told.
pub const SHELL_END: u32 = 0x10;
/// The collision bit a constantly displaced body's physics nodes gain.
pub const DISPLACED_BIT: u32 = 0x4000;
/// The collision bits a phased weapon loses.
pub const PHASED_BITS: u32 = 0xC000;
/// The loop sound a constant displacement plays.
pub const HUM_SOUND: &str = "thmdsp_cns01";
/// The sound a passing blow plays.
pub const PHASE_SOUND: &str = "thmdsp_phs";
/// The hum's first play is silent.
pub const HUM_VOLUME: f32 = 0.0;
/// The phase sound's volume.
pub const PHASE_VOLUME: f32 = 0.6;
/// The image keeps eight spare strands beyond its anchors.
pub const SPARE_STRANDS: usize = 8;
/// The tendril geometry's registered type word.
pub const TENDRIL_KIND: u32 = 0x15;

/// A node of a body the displacement routines walk.
#[derive(Debug, Clone, PartialEq)]
pub struct VeilNode {
    /// The node's condition/class word: its low seven bits name the kind
    /// (9 a mesh, 5 a skin).
    pub status: u32,
    /// The physics flag: only a node with bit 0x2000 takes the body's
    /// collision bits and passes them on.
    pub physics: bool,
    /// The collision bits.
    pub collision: u32,
    /// The fade bits (0x10 shown, 0x20 hidden, 0x4 fading out,
    /// 0x8000_0000 dead).
    pub visibility: u32,
    /// The node's fade value.
    pub fade: f32,
    /// The node's low render flags: bit 1 is hidden with subobjects.
    pub render_flags: u32,
    /// The node carries subobjects.
    pub subobjects: bool,
    /// The child nodes.
    pub children: Vec<VeilNode>,
}

impl VeilNode {
    /// A mesh node with the given children.
    pub fn mesh(children: Vec<VeilNode>) -> Self {
        Self {
            status: 9,
            physics: true,
            collision: 0,
            visibility: 0,
            fade: 0.0,
            render_flags: 0,
            subobjects: !children.is_empty(),
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

/// Adds collision `mask` to the node and its physics children.
pub fn add_physics_bits(node: &mut VeilNode, mask: u32) {
    if node.physics {
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
    if part.status & 0x5f != 9 || part.visibility & 0x8000_0000 != 0 {
        return;
    }
    if part.visibility & 0x30 == 0 {
        if mask != 0 {
            part.visibility |= mask;
            part.render_flags &= !1;
            return;
        }
    } else if mask == 0 {
        if part.subobjects {
            part.render_flags |= 1;
        }
        part.visibility &= !0x30;
        return;
    }
    part.visibility = (part.visibility & !0x30) | mask;
}

/// Fades the gathered nodes out: a mesh sets its fade to one and hides,
/// a skin only sets its fade.
pub fn fade_out(parts: &mut [VeilNode]) {
    for part in parts {
        part.render_flags |= 4;
        set_visibility(part, 0x10);
    }
}

/// Restores the gathered nodes: marks them fading and shows the meshes.
pub fn fade_restore(parts: &mut [VeilNode]) {
    for part in parts {
        part.fade = 1.0;
        set_visibility(part, 0x20);
    }
}

/// The shell a constant displacement wears.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShellWear {
    pub fade: f32,
    /// The node's event word.
    pub event: u32,
    /// The shell's subobjects were told to drop.
    pub hidden: bool,
}

impl ShellWear {
    /// A shell starting at zero.
    pub fn new() -> Self {
        Self {
            fade: 0.0,
            event: 0,
            hidden: false,
        }
    }

    /// The shell's end: its fade returns to zero and the node is told
    /// [`SHELL_END`].
    pub fn end(&mut self) {
        self.fade = 0.0;
        self.event = SHELL_END;
        self.hidden = true;
    }
}

impl Default for ShellWear {
    fn default() -> Self {
        Self::new()
    }
}

/// The left-behind image's parts.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LeftBehind {
    /// The mesh nodes (kind 9), in the body's order.
    pub meshes: Vec<VeilNode>,
    /// The skin nodes (kind 5).
    pub skins: Vec<VeilNode>,
    /// The image's scale, from the body's fade length.
    pub scale: f32,
}

impl LeftBehind {
    /// Gathers a body's parts: a live mesh without the 0x80000 flag and a
    /// skin, in the walk's order.
    pub fn gather(root: &VeilNode) -> Self {
        let mut image = Self::default();
        gather_parts(root, &mut image.meshes, &mut image.skins);
        prune_meshes(&mut image.meshes);
        image
    }
}

/// Collects a node's mesh and skin parts into the image's lists.
pub fn gather_parts(node: &VeilNode, meshes: &mut Vec<VeilNode>, skins: &mut Vec<VeilNode>) {
    let kind = node.status & 0x5f;
    if kind == 9 {
        if node.render_flags & 0x0008_0000 == 0 && node.visibility & 0x8000_0000 == 0 {
            meshes.push(node.clone());
        }
    } else if kind == 5 {
        skins.push(node.clone());
    }
    for child in &node.children {
        gather_parts(child, meshes, skins);
    }
}

/// Orders the meshes by their subobjects' 0x40 flags: a mesh with none
/// stays at the front, one with all set moves behind the kept ones, and
/// one with both moves behind both. The list keeps every mesh; the
/// original's scan and drop cursors walk the tail as they swap.
pub fn prune_meshes(meshes: &mut [VeilNode]) {
    let mut scan = meshes.len();
    let mut drop = meshes.len();
    let mut index = 0;
    while index < scan {
        let mut seen = 0u32;
        for child in &meshes[index].children {
            seen |= if child.render_flags & 0x40 == 0 { 1 } else { 2 };
        }
        if seen >= 3 {
            scan -= 1;
            drop -= 1;
            meshes.swap(index, drop);
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
    pub left_behind: LeftBehind,
    /// The phased nodes, each with the stamp its collisions return.
    pub phased: PhasedWeapons,
    /// The streak deciding each blow.
    pub streak: PhaseStreak,
    /// The hum's pitch.
    pub hum_pitch: f32,
    /// The wisp the last passing blow made, if any.
    pub last_wisp: Option<[f32; 3]>,
    /// The passing blow's sound pitch.
    pub last_pitch: Option<f32>,
}

impl ConstantVeil {
    /// Starts a constant displacement on `body`. `hum_length` is the hum
    /// sound's length in seconds; `roll` draws the hum's pitch and the
    /// first frame leaves the loop's volume at zero.
    pub fn start(body: VeilNode, hum_length: f32, roll: &mut Roll) -> Self {
        let mut body = body;
        add_physics_bits(&mut body, DISPLACED_BIT);
        let left_behind = LeftBehind {
            scale: 1.0,
            ..LeftBehind::gather(&body)
        };
        Self {
            body,
            shell: ShellWear::new(),
            left_behind,
            phased: PhasedWeapons::default(),
            streak: PhaseStreak::default(),
            hum_pitch: hum_length * 0.8 * roll.unit(),
            last_wisp: None,
            last_pitch: None,
        }
    }

    /// One blow on a node: true when it passes through. On a pass the
    /// node and its children lose [`PHASED_BITS`], the node is listed with
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
    /// Any return ends the shell, as each restored weapon does.
    pub fn restore(&mut self, now: u64) -> Vec<u64> {
        let freed = self.phased.restore(now);
        if !freed.is_empty() {
            for key in &freed {
                if let Some(node) = find_key_mut(&mut self.body, *key) {
                    clear_bits(node, PHASED_BITS);
                }
            }
            self.shell.end();
        }
        freed
    }

    /// Every physics node's collision bits gained the displaced bit.
    pub fn has_displaced_bit(&self) -> bool {
        let mut all = true;
        self.body.walk(&mut |node| {
            if node.physics && node.collision & DISPLACED_BIT == 0 {
                all = false;
            }
        });
        all
    }
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

/// How long a tendril strand lives after its node's restore, in the
/// original's milliseconds.
pub const TENDRIL_MS: u32 = 50;

#[cfg(test)]
mod tests {
    use super::*;

    fn wear() -> ShellWear {
        ShellWear::new()
    }

    #[test]
    fn a_shell_starts_at_zero_and_ends_with_its_event() {
        let mut shell = wear();
        assert_eq!(shell.fade, 0.0);
        assert_eq!(shell.event, 0);
        shell.end();
        assert_eq!((shell.fade, shell.event), (0.0, SHELL_END));
        assert!(shell.hidden);
    }

    #[test]
    fn a_displaced_body_gains_the_physics_bit_only_on_physics_nodes() {
        let mut body = VeilNode::mesh(vec![
            VeilNode {
                physics: false,
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
        let mut parts = vec![
            VeilNode::mesh(vec![VeilNode::mesh(vec![])]),
            VeilNode::skin(),
        ];
        fade_out(&mut parts);
        assert_eq!(parts[0].render_flags & 4, 4);
        assert_eq!(parts[0].visibility, 0x10);
        assert_eq!(parts[1].visibility, 0);
        fade_restore(&mut parts);
        assert_eq!(parts[0].visibility, 0x20);
        assert_eq!(parts[0].fade, 1.0);
        assert_eq!(parts[1].fade, 1.0);
    }

    #[test]
    fn the_left_behind_takes_meshes_and_skins() {
        let body = VeilNode::mesh(vec![
            VeilNode::mesh(vec![]),
            VeilNode::other(3),
            VeilNode::skin(),
            VeilNode {
                render_flags: 0x0008_0000,
                ..VeilNode::mesh(vec![])
            },
            VeilNode {
                visibility: 0x8000_0000,
                ..VeilNode::mesh(vec![])
            },
        ]);
        let image = LeftBehind::gather(&body);
        assert_eq!(image.meshes.len(), 2);
        assert_eq!(image.skins.len(), 1);
    }

    #[test]
    fn the_prune_orders_the_meshes_by_their_subobject_flags() {
        let full = |flags: u32| VeilNode {
            children: vec![
                VeilNode {
                    render_flags: flags,
                    ..VeilNode::mesh(vec![])
                },
                VeilNode {
                    render_flags: flags,
                    ..VeilNode::mesh(vec![])
                },
            ],
            ..VeilNode::mesh(vec![])
        };
        let mixed = VeilNode::mesh(vec![
            VeilNode {
                render_flags: 0,
                ..VeilNode::mesh(vec![])
            },
            VeilNode {
                render_flags: 0x40,
                ..VeilNode::mesh(vec![])
            },
        ]);
        let mut meshes = vec![
            VeilNode::mesh(vec![]),
            full(0x40),
            mixed.clone(),
            VeilNode::mesh(vec![]),
        ];
        prune_meshes(&mut meshes);
        // The meshes with nothing set stay first in order, the all-set
        // follow, and the mixed last; the list keeps all four.
        assert_eq!(meshes.len(), 4);
        assert!(meshes[0].children.is_empty());
        assert!(meshes[1].children.is_empty());
        assert_eq!(meshes[2].children.len(), 2);
        assert!(
            meshes[2]
                .children
                .iter()
                .all(|c| c.render_flags & 0x40 != 0)
        );
        assert_eq!(meshes[3].children.len(), 2);
        assert!(
            meshes[3]
                .children
                .iter()
                .any(|c| c.render_flags & 0x40 == 0)
        );

        let mut one = vec![VeilNode::mesh(vec![]), mixed, full(0x40)];
        prune_meshes(&mut one);
        assert_eq!(one.len(), 3);
        assert!(one[0].children.is_empty());
        assert!(one[1].children.iter().all(|c| c.render_flags & 0x40 != 0));
        assert!(one[2].children.iter().any(|c| c.render_flags & 0x40 == 0));
    }

    #[test]
    fn a_constant_displacement_hides_its_weapon_for_half_a_second() {
        let mut draw = Roll(0x26b0_5733);
        let expected_pitch = 2.0f32 * 0.8 * draw.unit();
        let mut roll = Roll(0x26b0_5733);
        let body = VeilNode::mesh(vec![VeilNode::mesh(vec![])]);
        let mut veil = ConstantVeil::start(body, 2.0, &mut roll);
        assert!(veil.has_displaced_bit());
        assert_eq!(veil.hum_pitch, expected_pitch);
        // The first draw passes (the streak starts at zero): the node and
        // its child lose 0xC000.
        let through = veil.hit(&[0], 1000, [1.0, 2.0, 3.0], &mut Roll(0x26b0_5733));
        assert!(through);
        assert_eq!(veil.body.children[0].collision & PHASED_BITS, PHASED_BITS);
        assert_eq!(veil.last_wisp, Some([1.0, 2.0, 3.0]));
        assert_eq!(veil.streak.0, 1);
        assert!(veil.restore(1500).is_empty());
        assert_eq!(veil.shell.event, 0);
        assert_eq!(veil.restore(1501), vec![path_key(&[0])]);
        assert_eq!(veil.body.children[0].collision & PHASED_BITS, 0);
        assert_eq!(veil.shell.event, SHELL_END);
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
}
