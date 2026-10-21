//! The character body as the original's generator manipulates it: a node
//! tree with each node's local and world frames, its slot and layer, its
//! chunk kind, node and render flag words, the word the item loader writes
//! through it, and its rigid-body state.
//!
//! The generator's rebuild, re-dress and pose passes all walk this tree and
//! write the same fields, so they are kept as plain data and pure functions
//! here; nothing in the running game drives them yet. Each pass follows the
//! original's instructions: which word it tests, which nodes it recurses
//! into, and the operation order of its vector helpers (the dot products
//! sum x and z first where the original does, the normalising uses the
//! processor's reciprocal square root estimate).
//!
//! The dynamics object is much larger in the original; [`Dynamics`] keeps
//! the fields these passes touch. The rest vectors come from the engine's
//! shape builder, which callers supply as a closure.

use exa_formats::morph::{Frame, unplace, unturn};
use exa_formats::rfc::reciprocal_sqrt;

/// A chunk kind's low twelve bits for a mesh node.
pub const MESH: u32 = 0x89;
/// Chunk kind bit: the node's children keep their own frames (the frame
/// passes copy a child's whole frame instead of re-deriving it).
pub const OWN_FRAMES: u32 = 0x10;
/// Chunk kind bit the tree pose restore clears on every node.
pub const POSE_PENDING: u32 = 0x100_0000;
/// Node flag: the mesh is subdivided and gets its own body when the
/// dynamics are built.
pub const SUBDIVIDE: u32 = 0x20_0000;
/// Node flag: the state copy follows this child into the source tree.
pub const FOLLOWED: u32 = 0x8_0000;
/// Node flag: the node is a held item's mesh.
pub const HELD: u32 = 0x1_0000;
/// The word the hair builder writes on its roots.
pub const HAIR_OWNER: u32 = 0x1000_fca0;
/// Dynamics flag: a character's own body.
pub const CHARACTER_BODY: u32 = 0x1200_0000;
/// Dynamics flag the split and build passes follow on a parent.
pub const FOLLOWABLE: u32 = 0x1000_0000;
/// Dynamics flag the detach sets on the old scene's root body.
pub const SCENE_LEFT: u32 = 0x20_0000;

/// A material's state, as the generator's hair/cloth flag pass writes it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MaterialStyle {
    /// The material's first flag word.
    pub flags: u32,
    /// The material's second flag word.
    pub flags2: u32,
    /// The material's render flags.
    pub render_flags: u32,
    /// Four shading values.
    pub values: [f32; 4],
}

/// The hair/cloth material pass: a node whose render flags carry
/// `0x400000` raises its first material's flags and sets the four shading
/// values. Returns whether it did.
pub fn raise_material_flags(render_flags: u32, material: &mut MaterialStyle) -> bool {
    if render_flags & 0x40_0000 == 0 {
        return false;
    }
    material.flags |= 0x0800_0000;
    material.flags2 |= 0x4000;
    material.render_flags |= 0x100;
    material.values = [1.0, 0.06, 0.04, 0.3];
    true
}

/// The identity frame.
pub const IDENTITY: Frame = exa_assets::merge::IDENTITY;

/// The rigid-body state the generator gives a node: the fields its passes
/// read and write.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dynamics {
    /// Build flags ([`CHARACTER_BODY`], [`FOLLOWABLE`], …).
    pub flags: u32,
    /// Four scale factors set at build and split time.
    pub scales: [f32; 4],
    /// The four rest-geometry vectors, the shape builder's times 1.25.
    pub rest: [[f32; 3]; 4],
    /// The four rest points placed in the world by [`Body::restore_world`].
    pub points: [[f32; 3]; 4],
    /// The body this one follows (a character's parent), by node.
    pub follows: Option<usize>,
    /// The body's index in its simulation; negative before it is added.
    pub index: i32,
    /// The local frame the state copy saved before overwriting it.
    pub saved_local: Option<Frame>,
}

impl Default for Dynamics {
    fn default() -> Self {
        Self {
            flags: 0,
            scales: [1.0; 4],
            rest: [[0.0; 3]; 4],
            points: [[0.0; 3]; 4],
            follows: None,
            index: -1,
            saved_local: None,
        }
    }
}

impl Dynamics {
    /// The split copy: the low flag bits kept, [`CHARACTER_BODY`] set, the
    /// four scales moved halfway to 1 (`s·0.5 + 0.5`).
    pub fn split(&self) -> Self {
        Self {
            flags: (self.flags & 0x00FF_FFFF) | CHARACTER_BODY,
            scales: self.scales.map(|s| s * 0.5 + 0.5),
            ..*self
        }
    }
}

/// One node of a character body.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub name: String,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    /// The node's local frame in its parent.
    pub local: Frame,
    /// Its frame in the body's space.
    pub world: Frame,
    /// The chunk kind ([`MESH`] in the low twelve bits for a mesh).
    pub chunk_kind: u32,
    /// The node flags ([`SUBDIVIDE`], [`FOLLOWED`], [`HELD`], …).
    pub flags: u32,
    /// The render flags the slot and overlay passes write.
    pub render_flags: u32,
    /// The word the item loader writes through an item's model (its id;
    /// [`HAIR_OWNER`] on hair roots).
    pub owner: u32,
    /// The body slot its name gives.
    pub slot: usize,
    /// The layer bits of its name ([`crate::layers::layer_of`]).
    pub layer: u64,
    /// The mesh is linked to the morph controller (the frame refresh
    /// after a morph only walks such meshes).
    pub morphed: bool,
    /// Its mesh parts' hidden flag (set while held).
    pub part_hidden: bool,
    /// The node carries cloth instead of a rigid body.
    pub cloth: bool,
    /// The rigid-body state, if built.
    pub dynamics: Option<Dynamics>,
    /// The scene the node is registered with.
    pub scene: Option<usize>,
    /// The velocity of a cloth mesh, zero otherwise.
    pub velocity: [f32; 3],
}

impl Default for Node {
    fn default() -> Self {
        Self {
            name: String::new(),
            parent: None,
            children: Vec::new(),
            local: IDENTITY,
            world: IDENTITY,
            chunk_kind: 0,
            flags: 0,
            render_flags: 0,
            owner: 0,
            slot: 0,
            layer: 0,
            morphed: false,
            part_hidden: false,
            cloth: false,
            dynamics: None,
            scene: None,
            velocity: [0.0; 3],
        }
    }
}

impl Node {
    /// A mesh node: its chunk kind's low twelve bits are [`MESH`].
    pub fn is_mesh(&self) -> bool {
        self.chunk_kind & 0xFFF == MESH
    }
}

/// One record of a saved bone tree: the bone's world frame and how many
/// child records follow it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PoseRecord {
    pub frame: Frame,
    pub children: u32,
}

/// The two-digit number the slot passes parse from a name's first two
/// characters: 0 unless both are digits.
pub fn name_digits(name: &str) -> usize {
    let b = name.as_bytes();
    match (b.first(), b.get(1)) {
        (Some(a), Some(c)) if a.is_ascii_digit() && c.is_ascii_digit() => {
            usize::from(a - b'0') * 10 + usize::from(c - b'0')
        }
        _ => 0,
    }
}

/// The slot the re-framing pass reads: the first two characters taken as
/// digits without a check; a value outside 1 to 18 gives slot 19.
pub fn frame_slot(name: &str) -> usize {
    let b = name.as_bytes();
    let digit = |i: usize| i32::from(b.get(i).copied().unwrap_or(0)) - 0x30;
    let value = digit(0) * 10 + digit(1);
    if (1..=18).contains(&value) {
        value as usize
    } else {
        19
    }
}

/// The engine's vector normalise: the length squared summed `(x² + z²) +
/// y²`, its reciprocal square root estimate, zero for a zero vector.
pub fn normalise(v: [f32; 3]) -> [f32; 3] {
    let length = (v[0] * v[0] + v[2] * v[2]) + v[1] * v[1];
    let s = if length == 0.0 {
        0.0
    } else {
        reciprocal_sqrt(length)
    };
    v.map(|c| c * s)
}

/// The engine's projection removal: `a − (a·b)·b`, the dot summed
/// `(x + z) + y`.
pub fn remove_along(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    let d = (a[0] * b[0] + a[2] * b[2]) + a[1] * b[1];
    [a[0] - d * b[0], a[1] - d * b[1], a[2] - d * b[2]]
}

/// The cross product `a × b`.
pub fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// The rows a saved frame is normalised to: row 0 normalised, row 2 made
/// square to it and normalised, row 1 the normalised `row 2 × row 0`.
pub fn orthonormal(r0: [f32; 3], _r1: [f32; 3], r2: [f32; 3]) -> ([f32; 3], [f32; 3], [f32; 3]) {
    let r0 = normalise(r0);
    let r2 = normalise(remove_along(r2, r0));
    let r1 = normalise(cross(r2, r0));
    (r0, r1, r2)
}

/// The engine's point placement: `(x·r0 + y·r1) + (z·r2 + t)`.
pub fn place_point(frame: &Frame, p: [f32; 3]) -> [f32; 3] {
    let [r0, r1, r2, t] = *frame;
    [0, 1, 2].map(|k| (p[0] * r0[k] + p[1] * r1[k]) + (p[2] * r2[k] + t[k]))
}

/// The engine's squared distance, summed `(x² + y²) + z²`.
pub fn distance_sq(a: [f32; 3], b: [f32; 3]) -> f32 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    (d[0] * d[0] + d[1] * d[1]) + d[2] * d[2]
}

/// A character body's node tree.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Body {
    pub nodes: Vec<Node>,
}

impl Body {
    /// A body of `nodes`, linking children from the nodes' parent indices.
    pub fn new(nodes: Vec<Node>) -> Self {
        let mut body = Self { nodes };
        body.refresh_links();
        body
    }

    /// The roots, in node order.
    pub fn roots(&self) -> Vec<usize> {
        (0..self.nodes.len())
            .filter(|&i| self.nodes[i].parent.is_none())
            .collect()
    }

    /// Every node under `root`, the root first, depth first in child order.
    pub fn descendants(&self, root: usize) -> Vec<usize> {
        let mut out = Vec::new();
        let mut stack = vec![root];
        while let Some(i) = stack.pop() {
            out.push(i);
            stack.extend(self.nodes[i].children.iter().rev());
        }
        out
    }

    /// Rebuilds the children lists from the parents.
    pub fn refresh_links(&mut self) {
        for node in &mut self.nodes {
            node.children.clear();
        }
        let parents: Vec<Option<usize>> = self.nodes.iter().map(|n| n.parent).collect();
        for (i, p) in parents.into_iter().enumerate() {
            if let Some(p) = p
                && p < self.nodes.len()
                && p != i
            {
                self.nodes[p].children.push(i);
            }
        }
    }

    fn parent_world(&self, i: usize) -> Option<Frame> {
        self.nodes[i].parent.map(|p| self.nodes[p].world)
    }

    fn parent_owns_frames(&self, i: usize) -> bool {
        self.nodes[i]
            .parent
            .is_some_and(|p| self.nodes[p].chunk_kind & OWN_FRAMES != 0)
    }

    /// The item loader's word write: the word goes on `root` and every node
    /// under it.
    pub fn set_owner(&mut self, root: usize, word: u32) {
        for i in self.descendants(root) {
            self.nodes[i].owner = word;
        }
    }

    /// Marks a held item's meshes: each direct mesh child of `root` takes
    /// [`HELD`] and its parts the hidden flag. The root and deeper nodes
    /// are left alone.
    pub fn mark_held(&mut self, root: usize) {
        self.set_held(root, true);
    }

    /// Clears what [`Body::mark_held`] set.
    pub fn unmark_held(&mut self, root: usize) {
        self.set_held(root, false);
    }

    fn set_held(&mut self, root: usize, held: bool) {
        for c in self.nodes[root].children.clone() {
            let n = &mut self.nodes[c];
            if n.is_mesh() {
                if held {
                    n.flags |= HELD;
                } else {
                    n.flags &= !HELD;
                }
                n.part_hidden = held;
            }
        }
    }

    /// A character's whole held mark: its held light's model, then, when it
    /// carries a second object and its first hand's item has the torch
    /// section, that object's model.
    pub fn mark_hands(&mut self, light: Option<usize>, second: Option<usize>, torch_in_hand: bool) {
        if let Some(light) = light {
            self.mark_held(light);
        }
        if let Some(second) = second
            && torch_in_hand
        {
            self.mark_held(second);
        }
    }

    /// The race loader's slot pass: every node under `root` (depth first)
    /// takes its name's two digits as its slot, and each mesh the render
    /// flag `0x100000`. Returns the highest slot seen, which
    /// [`Body::number_unnamed`] continues from.
    pub fn assign_slots(&mut self, root: usize) -> usize {
        let mut max = 0;
        for i in self.descendants(root) {
            let n = &mut self.nodes[i];
            n.slot = name_digits(&n.name);
            max = max.max(n.slot);
            if n.is_mesh() {
                n.render_flags |= 0x10_0000;
            }
        }
        max
    }

    /// Numbers the meshes without a slot (depth first): every mesh outside
    /// the `#`/`_` pieces takes the render flags `0x500040`, and one with
    /// slot 0 the next number after `start`. Those numbers lie past the
    /// body slots, which makes such meshes overlays.
    pub fn number_unnamed(&mut self, root: usize, start: usize) {
        let mut next = start;
        for i in self.descendants(root) {
            let n = &mut self.nodes[i];
            if n.is_mesh() && !n.name.starts_with('#') && !n.name.starts_with('_') {
                n.render_flags |= 0x50_0040;
                if n.slot == 0 {
                    next += 1;
                    n.slot = next;
                }
            }
        }
    }

    /// The generator's slot chain: `root` takes slot 1, its subtrees their
    /// names' digits (a node with no digits keeps its subtree out). With
    /// `fill`, a table of `count` slots is filled from the tree: each
    /// node with a slot is recorded under it and searched below, and a
    /// visited mesh with render flag `0x40` gains `0x400000`.
    pub fn chain_slots(&mut self, root: usize, fill: bool, count: usize) -> Vec<Option<usize>> {
        self.nodes[root].slot = 1;
        for c in self.nodes[root].children.clone() {
            self.parse_slots(c);
        }
        let mut table = if fill { vec![None; count] } else { Vec::new() };
        self.fill_slots(root, fill, &mut table);
        table
    }

    fn parse_slots(&mut self, i: usize) {
        let slot = name_digits(&self.nodes[i].name);
        self.nodes[i].slot = slot;
        if slot > 0 {
            for c in self.nodes[i].children.clone() {
                self.parse_slots(c);
            }
        }
    }

    fn fill_slots(&mut self, i: usize, fill: bool, table: &mut [Option<usize>]) {
        let n = &mut self.nodes[i];
        if n.is_mesh() && n.render_flags & 0x40 != 0 {
            n.render_flags |= 0x40_0000;
        }
        if fill && n.slot > 0 {
            if let Some(entry) = table.get_mut(n.slot - 1) {
                *entry = Some(i);
            }
            for c in self.nodes[i].children.clone() {
                self.fill_slots(c, fill, table);
            }
        }
    }

    /// Re-frames a garment's tree on the base body (depth first): each
    /// node reads its slot from its name ([`frame_slot`]; a mesh in a body
    /// slot gains render flag `0x400000`); a node in slots 1 to 18 takes
    /// the base body's slot node world frame and a local translation that
    /// places it there under its parent (the local rotation is kept).
    /// Returns the slot bits seen, which the apparel cache keeps as the
    /// garment's slot mask.
    pub fn reframe_from_slots(&mut self, root: usize, slot_world: &[Option<Frame>; 19]) -> u32 {
        let mut seen = 0;
        for i in self.descendants(root) {
            let slot = frame_slot(&self.nodes[i].name);
            self.nodes[i].slot = slot;
            if slot <= 18 {
                if self.nodes[i].is_mesh() {
                    self.nodes[i].render_flags |= 0x40_0000;
                }
                seen |= 1 << (slot - 1);
                if let (Some(world), Some(parent)) = (slot_world[slot], self.parent_world(i)) {
                    self.nodes[i].world = world;
                    self.nodes[i].local[3] = unplace(&parent, world[3]);
                }
            }
        }
        seen
    }

    /// The frame refresh after a morph: a mesh linked to the morph
    /// controller takes a local translation from its world one under its
    /// parent, or its whole world frame as local when the parent keeps its
    /// children's frames; only such meshes are searched below.
    pub fn reframe_own(&mut self, i: usize) {
        let n = &self.nodes[i];
        if !(n.is_mesh() && n.morphed) {
            return;
        }
        if let Some(parent) = self.parent_world(i) {
            if self.parent_owns_frames(i) {
                self.nodes[i].local = self.nodes[i].world;
            } else {
                let t = self.nodes[i].world[3];
                self.nodes[i].local[3] = unplace(&parent, t);
            }
        }
        for c in self.nodes[i].children.clone() {
            self.reframe_own(c);
        }
    }

    /// Saves the character's bones' local frames before a rebuild, each
    /// normalised by [`orthonormal`] with its translation kept.
    pub fn save_frames(&self, bones: &[usize]) -> Vec<Frame> {
        bones
            .iter()
            .map(|&i| {
                let l = self.nodes[i].local;
                let (r0, r1, r2) = orthonormal(l[0], l[1], l[2]);
                [r0, r1, r2, l[3]]
            })
            .collect()
    }

    /// Restores the saved bone frames: a bone whose parent keeps its
    /// children's frames takes the whole saved frame, any other only its
    /// three rows (its translation stays).
    pub fn restore_frames(&mut self, bones: &[usize], saved: &[Frame]) {
        for (&i, frame) in bones.iter().zip(saved) {
            if self.parent_owns_frames(i) {
                self.nodes[i].local = *frame;
            } else {
                self.nodes[i].local[..3].copy_from_slice(&frame[..3]);
            }
        }
    }

    /// Places each character body's four rest points in the world after a
    /// rebuild: a body with [`CHARACTER_BODY`] that is in its simulation
    /// places its rest vectors by its node's world frame. Every mesh child
    /// is visited.
    pub fn restore_world(&mut self, i: usize) {
        let world = self.nodes[i].world;
        if let Some(d) = self.nodes[i].dynamics.as_mut()
            && d.flags & CHARACTER_BODY == CHARACTER_BODY
            && d.index >= 0
        {
            d.points = d.rest.map(|r| place_point(&world, r));
        }
        for c in self.nodes[i].children.clone() {
            if self.nodes[c].is_mesh() {
                self.restore_world(c);
            }
        }
    }

    /// Registers a subtree with `scene`: the node and, below it, every
    /// child that has dynamics.
    pub fn set_scene(&mut self, i: usize, scene: Option<usize>) {
        self.nodes[i].scene = scene;
        for c in self.nodes[i].children.clone() {
            if self.nodes[c].dynamics.is_some() {
                self.set_scene(c, scene);
            }
        }
    }

    /// Detaches a body from its scene. Returns the scene it left, whose
    /// root body the caller flags with [`SCENE_LEFT`].
    pub fn detach(&mut self, root: usize) -> Option<usize> {
        let old = self.nodes[root].scene;
        self.set_scene(root, None);
        old
    }

    /// Sets a cloth velocity through a subtree: every node with cloth
    /// takes the velocity.
    pub fn set_cloth_velocity(&mut self, root: usize, velocity: [f32; 3]) {
        for i in self.descendants(root) {
            if self.nodes[i].cloth {
                self.nodes[i].velocity = velocity;
            }
        }
    }

    /// The rebuild's state copy from `src_node` of `src` onto `dst_node`: a
    /// character body first saves the destination's local frame, then the
    /// local and world frames come over; each destination child flagged
    /// [`FOLLOWED`] is paired with the first source child of the same slot
    /// and copied in turn.
    pub fn copy_state(&mut self, dst_node: usize, src: &Body, src_node: usize) {
        let local = self.nodes[dst_node].local;
        if let Some(d) = self.nodes[dst_node].dynamics.as_mut()
            && d.flags & CHARACTER_BODY == CHARACTER_BODY
        {
            d.saved_local = Some(local);
        }
        self.nodes[dst_node].local = src.nodes[src_node].local;
        self.nodes[dst_node].world = src.nodes[src_node].world;
        for c in self.nodes[dst_node].children.clone() {
            if self.nodes[c].flags & FOLLOWED == 0 {
                continue;
            }
            let slot = self.nodes[c].slot;
            if let Some(&s) = src.nodes[src_node]
                .children
                .iter()
                .find(|&&s| src.nodes[s].slot == slot)
            {
                self.copy_state(c, src, s);
            }
        }
    }

    fn follows(&self, i: usize) -> Option<usize> {
        self.nodes[i].parent.filter(|&p| {
            self.nodes[p]
                .dynamics
                .is_some_and(|d| d.flags & FOLLOWABLE == FOLLOWABLE)
        })
    }

    /// Builds the dynamics of `i`: a node without them gets a body that
    /// follows a followable parent, scales 1 and the shape builder's rest
    /// vectors times 1.25; a mesh gains render flag `0x400`; its subdivided
    /// mesh children are built too. A node that already has followable
    /// dynamics is split instead ([`Body::split_dynamics`]).
    pub fn ensure_dynamics(&mut self, i: usize, shape: &dyn Fn(&Node) -> [[f32; 3]; 4]) {
        if let Some(d) = self.nodes[i].dynamics {
            if d.flags & FOLLOWABLE == FOLLOWABLE {
                self.split_dynamics(i, shape);
            }
            return;
        }
        let rest = shape(&self.nodes[i]).map(|v| v.map(|c| c * 1.25));
        self.nodes[i].dynamics = Some(Dynamics {
            follows: self.follows(i),
            rest,
            ..Dynamics::default()
        });
        if self.nodes[i].is_mesh() {
            self.nodes[i].render_flags |= 0x400;
        }
        self.for_subdivided(i, |b, c| b.ensure_dynamics(c, shape));
    }

    /// Splits the dynamics of `i` into a character body ([`Dynamics::split`]),
    /// following a followable parent; a mesh gains render flag `0x400`, and
    /// its subdivided mesh children are split too. A node without dynamics
    /// is built instead.
    pub fn split_dynamics(&mut self, i: usize, shape: &dyn Fn(&Node) -> [[f32; 3]; 4]) {
        let Some(old) = self.nodes[i].dynamics else {
            self.ensure_dynamics(i, shape);
            return;
        };
        if self.nodes[i].is_mesh() {
            self.nodes[i].render_flags |= 0x400;
        }
        let mut new = old.split();
        if let Some(p) = self.follows(i) {
            new.follows = Some(p);
        }
        self.nodes[i].dynamics = Some(new);
        self.for_subdivided(i, |b, c| b.split_dynamics(c, shape));
    }

    fn for_subdivided(&mut self, i: usize, mut f: impl FnMut(&mut Self, usize)) {
        for c in self.nodes[i].children.clone() {
            let n = &self.nodes[c];
            if n.is_mesh() && n.flags & SUBDIVIDE != 0 {
                f(self, c);
            }
        }
    }

    /// Rebuilds the dynamics of a rebuilt body: a character body in its
    /// simulation has its scales reset to 1 and its rest vectors rebuilt
    /// from the shape builder times 1.25; every mesh child is visited.
    pub fn rebuild_dynamics(&mut self, i: usize, shape: &dyn Fn(&Node) -> [[f32; 3]; 4]) {
        let rest = shape(&self.nodes[i]).map(|v| v.map(|c| c * 1.25));
        if let Some(d) = self.nodes[i].dynamics.as_mut()
            && d.flags & CHARACTER_BODY == CHARACTER_BODY
            && d.index >= 0
        {
            d.scales = [1.0; 4];
            d.rest = rest;
        }
        for c in self.nodes[i].children.clone() {
            if self.nodes[c].is_mesh() {
                self.rebuild_dynamics(c, shape);
            }
        }
    }

    /// The node-tree clone: a copy of the tree, where a mesh named `#…`
    /// gets fresh cloth instead of a copy of its dynamics.
    pub fn clone_tree(&self) -> Body {
        let mut out = self.clone();
        for n in &mut out.nodes {
            if n.chunk_kind & MESH == MESH && n.name.starts_with('#') {
                n.cloth = true;
                n.dynamics = None;
            }
        }
        out
    }

    /// Restores a saved bone tree (`records` in depth-first order, each with
    /// its child count). The root takes the first record as its local and
    /// world frame; each later record is matched to the child of its
    /// parent bone nearest its position (under 20 cm, the first nearest
    /// winning), takes the record as its world frame and the rows that
    /// place it there under the parent as its local rows. A record with no
    /// match skips its whole subtree. Every node's [`POSE_PENDING`] bit is
    /// cleared first.
    pub fn restore_tree_pose(&mut self, root: usize, records: &[PoseRecord]) {
        for i in self.descendants(root) {
            self.nodes[i].chunk_kind &= !POSE_PENDING;
        }
        let Some(first) = records.first() else {
            return;
        };
        self.nodes[root].local = first.frame;
        self.nodes[root].world = first.frame;
        let mut next = 1;
        self.pose_children(root, first.children, records, &mut next);
    }

    fn pose_children(&mut self, node: usize, count: u32, records: &[PoseRecord], next: &mut usize) {
        for _ in 0..count {
            let Some(r) = records.get(*next).copied() else {
                return;
            };
            *next += 1;
            let p = unplace(&self.nodes[node].world, r.frame[3]);
            let mut best = 400.0;
            let mut pick = None;
            for &c in &self.nodes[node].children {
                let d = distance_sq(p, self.nodes[c].local[3]);
                if d < best {
                    best = d;
                    pick = Some(c);
                }
            }
            match pick {
                Some(c) => {
                    self.nodes[c].world = r.frame;
                    let parent = self.nodes[node].world;
                    for k in 0..3 {
                        self.nodes[c].local[k] = unturn(&parent, r.frame[k]);
                    }
                    self.pose_children(c, r.children, records, next);
                }
                None => skip_records(r.children, records, next),
            }
        }
    }

    /// Restores a flat saved pose, one world frame per bone: every bone
    /// takes its frame as its world frame, the root (bone 0) also as its
    /// local, and every other bone the rows that place it under its
    /// parent's new world frame as its local rows.
    pub fn restore_flat_pose(&mut self, bones: &[usize], frames: &[Frame]) {
        for (&i, frame) in bones.iter().zip(frames) {
            self.nodes[i].world = *frame;
        }
        if let Some(&root) = bones.first() {
            self.nodes[root].local = self.nodes[root].world;
        }
        for &i in bones.iter().skip(1) {
            if let Some(parent) = self.parent_world(i) {
                let world = self.nodes[i].world;
                for k in 0..3 {
                    self.nodes[i].local[k] = unturn(&parent, world[k]);
                }
            }
        }
    }
}

/// The merge order of a garment's chosen nodes: the slot tree walked from
/// the pelvis, each slot's chosen node before its child slots' (1 to 13,
/// 17, 14 to 16, 18). Only the chosen node itself is merged; the overlays
/// ([`mark_overlays`]) follow once every garment is merged.
pub fn merge_order(chosen: &[Option<usize>; 19]) -> Vec<usize> {
    fn visit(slot: usize, chosen: &[Option<usize>; 19], out: &mut Vec<usize>) {
        if let Some(n) = chosen[slot] {
            out.push(n);
        }
        for &c in crate::layers::SLOTS[slot - 1].children {
            visit(c, chosen, out);
        }
    }
    let mut out = Vec::new();
    visit(1, chosen, &mut out);
    out
}

fn skip_records(count: u32, records: &[PoseRecord], next: &mut usize) {
    for _ in 0..count {
        let Some(r) = records.get(*next).copied() else {
            return;
        };
        *next += 1;
        skip_records(r.children, records, next);
    }
}

/// The overlay meshes of `root`'s tree (depth first): meshes whose render
/// flags have `0x400040` and whose slot lies past the eighteen body slots.
pub fn overlay_nodes(body: &Body, root: usize) -> Vec<usize> {
    body.descendants(root)
        .into_iter()
        .filter(|&i| {
            let n = &body.nodes[i];
            n.is_mesh() && n.render_flags & 0x40_0040 != 0 && n.slot > 18
        })
        .collect()
}

/// Marks the overlay meshes for the merge (their node flags become
/// `flags & !0x30000 | 0xC0000`) and returns them.
pub fn mark_overlays(body: &mut Body, root: usize) -> Vec<usize> {
    let nodes = overlay_nodes(body, root);
    for &i in &nodes {
        body.nodes[i].flags = (body.nodes[i].flags & 0xFFFC_FFFF) | 0x0C_0000;
    }
    nodes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(name: &str, parent: Option<usize>) -> Node {
        Node {
            name: name.into(),
            parent,
            chunk_kind: MESH,
            ..Node::default()
        }
    }

    /// Pelvis (0) with a leg chain and a chest/head chain; names are the
    /// body model's.
    fn body() -> Body {
        let mut nodes = vec![
            node("01Pelvis", None),
            node("02RULeg", Some(0)),
            node("03RLLeg", Some(1)),
            node("09Chest", Some(0)),
            node("10Head", Some(3)),
        ];
        nodes[0].chunk_kind = 0;
        Body::new(nodes)
    }

    fn translated(t: [f32; 3]) -> Frame {
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], t]
    }

    #[test]
    fn a_word_writes_the_root_and_every_child() {
        let mut b = body();
        b.set_owner(3, 0x1234);
        assert_eq!(b.nodes[3].owner, 0x1234);
        assert_eq!(b.nodes[4].owner, 0x1234);
        assert_eq!(b.nodes[0].owner, 0);
    }

    /// Only the root's direct mesh children take the held mark.
    #[test]
    fn held_marks_take_the_direct_mesh_children() {
        let mut b = body();
        b.mark_held(1);
        assert_eq!(b.nodes[2].flags & HELD, HELD);
        assert!(b.nodes[2].part_hidden);
        assert_eq!(b.nodes[1].flags & HELD, 0);
        b.mark_held(0);
        assert_eq!(b.nodes[1].flags & HELD, HELD);
        assert_eq!(b.nodes[4].flags & HELD, 0);
        b.unmark_held(1);
        assert_eq!(b.nodes[2].flags & HELD, 0);
        assert!(!b.nodes[2].part_hidden);
        // The second object is marked only with a torch in the first hand.
        let mut b = body();
        b.mark_hands(None, Some(3), false);
        assert_eq!(b.nodes[4].flags & HELD, 0);
        b.mark_hands(Some(1), Some(3), true);
        assert_eq!(b.nodes[2].flags & HELD, HELD);
        assert_eq!(b.nodes[4].flags & HELD, HELD);
    }

    /// The root is slot 1; a child without digits keeps its subtree out of
    /// the chain; the table records each slot's node.
    #[test]
    fn the_slot_chain_reads_the_names() {
        let mut b = body();
        b.nodes[0].name = "Root".into();
        b.nodes[1].name = "Leg".into();
        b.nodes[3].render_flags = 0x40;
        let table = b.chain_slots(0, true, 18);
        assert_eq!(b.nodes[0].slot, 1);
        assert_eq!(table[0], Some(0));
        assert_eq!(b.nodes[1].slot, 0);
        // The leg's child was never parsed.
        assert_eq!(b.nodes[2].slot, 0);
        assert_eq!((table[8], table[9]), (Some(3), Some(4)));
        assert_eq!(b.nodes[3].render_flags, 0x40_0040);
        assert!(b.chain_slots(0, false, 18).is_empty());
    }

    /// The race loader's numbering continues after the highest slot.
    #[test]
    fn unnamed_meshes_are_numbered_after_the_slots() {
        let mut b = body();
        b.nodes.push(node("strap", Some(3)));
        b.nodes.push(node("#cape", Some(3)));
        b.refresh_links();
        let max = b.assign_slots(0);
        assert_eq!(max, 10);
        assert_eq!(b.nodes[5].slot, 0);
        assert_eq!(b.nodes[3].render_flags & 0x10_0000, 0x10_0000);
        b.number_unnamed(0, max);
        assert_eq!(b.nodes[5].slot, 11);
        assert_eq!(b.nodes[5].render_flags & 0x50_0040, 0x50_0040);
        assert_eq!(b.nodes[6].slot, 0);
        assert_eq!(b.nodes[3].slot, 9);
        // A whole body's highest slot is 18, so its unnamed meshes number
        // from 19 and are overlays.
        b.nodes[5].slot = 0;
        b.number_unnamed(0, 18);
        assert_eq!(b.nodes[5].slot, 19);
        assert_eq!(overlay_nodes(&b, 0), [5]);
    }

    /// A garment node in a body slot takes the base body's world frame and
    /// the local translation that puts it there; a node without a slot
    /// keeps its frames.
    #[test]
    fn reframing_takes_the_base_slot_frames() {
        let mut b = body();
        b.nodes[0].world = translated([0.0, 1.0, 0.0]);
        b.nodes[3].name = "0900Chest".into();
        b.nodes[4].name = "strap".into();
        let mut slots = [None; 19];
        slots[9] = Some(translated([0.5, 2.0, 0.0]));
        let seen = b.reframe_from_slots(3, &slots);
        assert_eq!(seen, 1 << 8);
        assert_eq!(b.nodes[3].world[3], [0.5, 2.0, 0.0]);
        assert_eq!(b.nodes[3].local[3], [0.5, 1.0, 0.0]);
        assert_eq!(b.nodes[3].render_flags, 0x40_0000);
        assert_eq!(b.nodes[4].slot, 19);
        assert_eq!(b.nodes[4].local, IDENTITY);
        assert_eq!(frame_slot("1"), 19);
        assert_eq!(frame_slot("18x"), 18);
    }

    /// The morph refresh derives a local translation from the world one,
    /// or copies the world frame under a parent that keeps its children's
    /// frames, and walks only morphed meshes.
    #[test]
    fn the_morph_refresh_derives_local_frames() {
        let mut b = body();
        b.nodes[0].world = translated([0.0, 1.0, 0.0]);
        b.nodes[3].morphed = true;
        b.nodes[3].world = translated([0.0, 3.0, 0.0]);
        b.nodes[4].world = translated([0.0, 5.0, 0.0]);
        b.reframe_own(3);
        assert_eq!(b.nodes[3].local[3], [0.0, 2.0, 0.0]);
        // The head is not morphed: the walk stops there.
        assert_eq!(b.nodes[4].local, IDENTITY);
        // A parent of another chunk kind that keeps its children's frames:
        // the head takes its world frame whole.
        b.nodes[4].morphed = true;
        b.nodes[3].chunk_kind = OWN_FRAMES;
        b.reframe_own(4);
        assert_eq!(b.nodes[4].local, b.nodes[4].world);
    }

    #[test]
    fn the_split_moves_the_scales_halfway_to_one() {
        let shape = |_: &Node| [[2.0; 3]; 4];
        let mut b = body();
        b.ensure_dynamics(0, &shape);
        let d = b.nodes[0].dynamics.unwrap();
        assert_eq!(d.rest, [[2.5; 3]; 4]);
        assert_eq!(d.follows, None);
        b.nodes[0].dynamics.as_mut().unwrap().scales = [0.0, 0.5, 1.0, 2.0];
        b.split_dynamics(0, &shape);
        let d = b.nodes[0].dynamics.unwrap();
        assert_eq!(d.flags, CHARACTER_BODY);
        assert_eq!(d.scales, [0.5, 0.75, 1.0, 1.5]);
        // A followable body is split again, not rebuilt.
        b.nodes[0].dynamics.as_mut().unwrap().scales = [0.0; 4];
        b.ensure_dynamics(0, &shape);
        assert_eq!(b.nodes[0].dynamics.unwrap().scales, [0.5; 4]);
        // A subdivided mesh child is built and follows the parent.
        b.nodes[1].flags |= SUBDIVIDE;
        b.split_dynamics(0, &shape);
        assert_eq!(b.nodes[1].dynamics.unwrap().follows, Some(0));
        assert_eq!(b.nodes[1].render_flags & 0x400, 0x400);
        assert!(b.nodes[3].dynamics.is_none());
    }

    /// The rebuild resets a character body in its simulation and visits the
    /// mesh children.
    #[test]
    fn the_rebuild_resets_scales_and_rest() {
        let shape = |_: &Node| [[4.0; 3]; 4];
        let mut b = body();
        let d = Dynamics {
            flags: CHARACTER_BODY,
            scales: [0.2; 4],
            rest: [[9.0; 3]; 4],
            index: 0,
            ..Dynamics::default()
        };
        b.nodes[0].dynamics = Some(d);
        b.nodes[1].dynamics = Some(d);
        b.nodes[3].dynamics = Some(Dynamics { index: -1, ..d });
        b.rebuild_dynamics(0, &shape);
        let d0 = b.nodes[0].dynamics.unwrap();
        assert_eq!((d0.scales, d0.rest), ([1.0; 4], [[5.0; 3]; 4]));
        assert_eq!(b.nodes[1].dynamics.unwrap().scales, [1.0; 4]);
        // Not yet in its simulation: left alone.
        assert_eq!(b.nodes[3].dynamics.unwrap().scales, [0.2; 4]);
        assert_eq!(b.nodes[0].render_flags & 0x400, 0);
    }

    /// The saved frame is normalised (row 2 square to row 0, row 1 their
    /// cross) and comes back as rows only, unless the parent keeps its
    /// children's frames.
    #[test]
    fn frames_save_and_restore() {
        let mut b = body();
        b.nodes[3].local = [
            [2.0, 0.0, 0.0],
            [0.0, 5.0, 0.0],
            [1.0, 0.0, 4.0],
            [1.0, 2.0, 3.0],
        ];
        let bones = [0, 3];
        let saved = b.save_frames(&bones);
        let r = saved[1];
        assert!((r[0][0] - 1.0).abs() < 1e-3 && r[0][1] == 0.0);
        assert!((r[2][2] - 1.0).abs() < 1e-3 && r[2][0].abs() < 1e-3);
        // Row 1 is row 2 × row 0: +y for z × x.
        assert!((r[1][1] - 1.0).abs() < 1e-3);
        assert_eq!(r[3], [1.0, 2.0, 3.0]);
        b.nodes[3].local = translated([7.0, 7.0, 7.0]);
        b.restore_frames(&bones, &saved);
        assert_eq!(b.nodes[3].local[..3], saved[1][..3]);
        assert_eq!(b.nodes[3].local[3], [7.0, 7.0, 7.0]);
        b.nodes[0].chunk_kind |= OWN_FRAMES;
        b.restore_frames(&bones, &saved);
        assert_eq!(b.nodes[3].local, saved[1]);
        assert_eq!(normalise([0.0; 3]), [0.0; 3]);
    }

    /// A tree pose matches each record to the nearest child within 20 cm
    /// and skips the subtree of a record without one.
    #[test]
    fn a_tree_pose_matches_children_by_position() {
        let mut b = body();
        b.nodes[1].local[3] = [-8.0, -8.0, 0.0];
        b.nodes[3].local[3] = [0.0, 9.0, 0.0];
        b.nodes[4].local[3] = [0.0, 35.0, 0.0];
        let records = [
            PoseRecord {
                frame: translated([0.0, 100.0, 0.0]),
                children: 2,
            },
            // Near the chest's place: the chest.
            PoseRecord {
                frame: translated([1.0, 110.0, 0.0]),
                children: 1,
            },
            PoseRecord {
                frame: translated([0.0, 200.0, 0.0]),
                children: 0,
            },
            // Nowhere near a child of the pelvis: skipped with its child.
            PoseRecord {
                frame: translated([90.0, 100.0, 0.0]),
                children: 1,
            },
            PoseRecord {
                frame: translated([0.0, 0.0, 0.0]),
                children: 0,
            },
        ];
        b.restore_tree_pose(0, &records);
        assert_eq!(b.nodes[0].local[3], [0.0, 100.0, 0.0]);
        assert_eq!(b.nodes[3].world[3], [1.0, 110.0, 0.0]);
        // Only the rows are re-derived; the head's record is 90 cm up, too
        // far from the head.
        assert_eq!(b.nodes[3].local[3], [0.0, 9.0, 0.0]);
        assert_eq!(b.nodes[4].world, IDENTITY);
        assert_eq!(b.nodes[1].world, IDENTITY);
    }

    #[test]
    fn a_flat_pose_writes_worlds_and_local_rows() {
        let mut b = body();
        let turned: Frame = [
            [0.0, 1.0, 0.0],
            [-1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0],
        ];
        b.restore_flat_pose(&[0, 3], &[turned, turned]);
        assert_eq!(b.nodes[0].local, turned);
        assert_eq!(b.nodes[3].world, turned);
        assert_eq!(b.nodes[3].local[..3], IDENTITY[..3]);
    }

    /// The state copy pairs flagged children by slot.
    #[test]
    fn the_state_copy_pairs_children_by_slot() {
        let mut src = body();
        for (i, n) in src.nodes.iter_mut().enumerate() {
            n.slot = i + 1;
            n.local = translated([i as f32; 3]);
        }
        let mut dst = body();
        for (i, n) in dst.nodes.iter_mut().enumerate() {
            n.slot = i + 1;
        }
        dst.nodes[3].flags |= FOLLOWED;
        dst.nodes[0].dynamics = Some(Dynamics {
            flags: CHARACTER_BODY,
            ..Dynamics::default()
        });
        dst.copy_state(0, &src, 0);
        assert_eq!(dst.nodes[0].dynamics.unwrap().saved_local, Some(IDENTITY));
        assert_eq!(dst.nodes[3].local[3], [3.0; 3]);
        // The leg is not flagged; the head is not followed past the chest.
        assert_eq!(dst.nodes[1].local, IDENTITY);
        assert_eq!(dst.nodes[4].local, IDENTITY);
    }

    #[test]
    fn detaching_clears_the_scene_of_the_bodies() {
        let mut b = body();
        b.nodes[1].dynamics = Some(Dynamics::default());
        b.set_scene(0, Some(7));
        assert_eq!(b.nodes[1].scene, Some(7));
        assert_eq!(b.nodes[3].scene, None);
        assert_eq!(b.detach(0), Some(7));
        assert_eq!((b.nodes[0].scene, b.nodes[1].scene), (None, None));
        assert_eq!(b.nodes[1].parent, Some(0));
    }

    /// The merge walks the slot tree: the right hand (17) after the right
    /// forearm (13), before the left shoulder (14).
    #[test]
    fn the_merge_follows_the_slot_tree() {
        let mut chosen = [None; 19];
        for (slot, entry) in chosen.iter_mut().enumerate().skip(1) {
            *entry = Some(slot);
        }
        assert_eq!(
            merge_order(&chosen),
            [
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 17, 14, 15, 16, 18
            ]
        );
        let mut b = body();
        b.nodes[4].slot = 20;
        b.nodes[4].render_flags = 0x40_0040;
        b.nodes[4].flags = 0x3_0000;
        assert_eq!(mark_overlays(&mut b, 0), [4]);
        assert_eq!(b.nodes[4].flags, 0x0C_0000);
    }

    #[test]
    fn a_clone_gives_hash_meshes_fresh_cloth() {
        let mut b = body();
        b.nodes[4].name = "#cape".into();
        b.nodes[4].dynamics = Some(Dynamics::default());
        let c = b.clone_tree();
        assert!(c.nodes[4].cloth && c.nodes[4].dynamics.is_none());
        assert_eq!(c.nodes[3], b.nodes[3]);
    }

    #[test]
    fn the_rest_points_follow_the_world_frame() {
        let mut b = body();
        b.nodes[0].world = translated([0.0, 10.0, 0.0]);
        b.nodes[0].dynamics = Some(Dynamics {
            flags: CHARACTER_BODY,
            rest: [[1.0, 0.0, 0.0]; 4],
            index: 0,
            ..Dynamics::default()
        });
        b.restore_world(0);
        assert_eq!(b.nodes[0].dynamics.unwrap().points, [[1.0, 10.0, 0.0]; 4]);
    }

    #[test]
    fn the_hair_material_pass_sets_the_original_numbers() {
        let mut m = MaterialStyle::default();
        assert!(!raise_material_flags(0x0, &mut m));
        assert_eq!(m, MaterialStyle::default());
        assert!(raise_material_flags(0x400000, &mut m));
        assert_eq!(m.flags, 0x0800_0000);
        assert_eq!(m.flags2, 0x4000);
        assert_eq!(m.render_flags, 0x100);
        assert_eq!(m.values, [1.0, 0.06, 0.04, 0.3]);
    }

    #[test]
    fn cloth_takes_its_velocity() {
        let mut b = body();
        b.nodes[4].cloth = true;
        b.set_cloth_velocity(0, [1.0, 2.0, 3.0]);
        assert_eq!(b.nodes[4].velocity, [1.0, 2.0, 3.0]);
        assert_eq!(b.nodes[3].velocity, [0.0; 3]);
    }

    #[test]
    fn the_name_digits_need_two_digits() {
        assert_eq!(name_digits("0906Chest"), 9);
        assert_eq!(name_digits("99x"), 99);
        assert_eq!(name_digits("9Chest"), 0);
        assert_eq!(name_digits(""), 0);
    }
}
