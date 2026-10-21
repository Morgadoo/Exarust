//! The character body as the original's generator manipulates it: a node
//! tree with each node's local and world frames, its slot and layer, its
//! mesh and cloth flags and its rigid-body state.
//!
//! The generator's rebuild, re-dress and cover passes all walk this tree and
//! write the same fields, so they are kept as plain data and pure functions
//! here. The running game maps the tree onto its Bevy entities; the rules
//! themselves need no renderer. Node flags and frame fields follow the
//! original's layout: the tree's local transform is a 3 × 4 row frame
//! (metres), `kind` is the node's build kind, `slot` is the body slot its
//! name gives, and a mesh carries its material and held flags.
//!
//! The dynamics object's full layout is not in the pack; [`Dynamics`] keeps
//! the fields the builder touches (its flags, four scales and rest vectors
//! and the body it follows). `restore_frames`'s flagged path and
//! `copy_state`'s node pairing are the parts marked as guesses below.

use exa_formats::morph::{Frame, compose};

/// A node's build kind for a mesh (its chunk kind's low twelve bits).
pub const MESH: u32 = 0x89;
/// Node flag: the node is a mesh and follows the character's skin.
pub const MESH_SKIN: u32 = 0x800;
/// Node flag: the mesh is subdivided when its body is built.
pub const SUBDIVIDE: u32 = 0x20_0000;
/// Node flag: level geometry, never a character part.
pub const STATIC: u32 = 0x8_0000;
/// Node flag: the node is a held item's mesh.
pub const HELD: u32 = 0x1_0000;
/// Node flag: the node's mesh has the character's occlusion mask.
pub const OCCLUDED: u32 = 0x40_0000;
/// Node flag: the node carries cloth.
pub const CLOTH: u32 = 0x6000_0000;

/// A material's state, as the generator's hair/cloth flag pass writes it.
#[derive(Clone, Copy, Debug, PartialEq)]
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

impl Default for MaterialStyle {
    fn default() -> Self {
        Self {
            flags: 0,
            flags2: 0,
            render_flags: 0,
            values: [0.0; 4],
        }
    }
}

/// The hair/cloth material pass: a node whose mesh carries
/// flag `0x400000` raises its material's flags and sets the four shading
/// values. Returns whether it did.
pub fn raise_material_flags(mesh_flags: u32, material: &mut MaterialStyle) -> bool {
    if mesh_flags & 0x40_0000 == 0 {
        return false;
    }
    material.flags |= 0x0800_0000;
    material.flags2 |= 0x4000;
    material.render_flags |= 0x100;
    material.values = [1.0, 0.06, 0.04, 0.3];
    true
}

/// Overlay mesh nodes: mesh nodes with the held/overlay
/// flags `0x400040` past the eighteen body slots. Their flags gain
/// `0x0C0000` when merged.
pub fn overlay_nodes(body: &Body) -> Vec<usize> {
    (0..body.nodes.len())
        .filter(|&i| {
            body.nodes[i].mesh && body.nodes[i].flags & 0x40_0040 != 0 && body.nodes[i].slot > 18
        })
        .collect()
}

/// Marks the overlay nodes for the merge and returns them.
pub fn mark_overlays(body: &mut Body) -> Vec<usize> {
    let nodes = overlay_nodes(body);
    for &i in &nodes {
        body.nodes[i].flags = (body.nodes[i].flags & 0xFFFC_FFFF) | 0x0C_0000;
    }
    nodes
}

/// The identity frame.
pub const IDENTITY: Frame = [
    [1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 0.0, 1.0],
    [0.0, 0.0, 0.0],
];

/// The rigid-body state the generator gives a node. The original's dynamics
/// object is much larger; these are the fields the character builder reads
/// and writes (its build flags, the four geometry scales and the body it
/// follows).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dynamics {
    /// Build flags (`0x12000000` marks a character's own body).
    pub flags: u32,
    /// Four scale factors set at build and split time.
    pub scales: [f32; 4],
    /// The four rest-geometry vectors, each multiplied by 1.25 at build.
    pub rest: [[f32; 3]; 4],
    /// The dynamics this one follows (a character's parent), if any.
    pub follows: Option<usize>,
}

impl Default for Dynamics {
    fn default() -> Self {
        Self {
            flags: 0,
            scales: [1.0; 4],
            rest: [[0.0; 3]; 4],
            follows: None,
        }
    }
}

impl Dynamics {
    /// A node's own body with the original's four rest vectors at 1.25.
    pub fn built() -> Self {
        Self {
            rest: [[1.25; 3]; 4],
            ..Default::default()
        }
    }

    /// The split copy: the low flag bits kept, `0x12000000` set, the four
    /// scales moved halfway to 1 (`s·0.5 + 0.5`).
    pub fn split(&self) -> Self {
        Self {
            flags: (self.flags & 0x00FF_FFFF) | 0x1200_0000,
            scales: self.scales.map(|s| s * 0.5 + 0.5),
            rest: self.rest,
            follows: self.follows,
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
    /// Its frame in the body's space, refreshed by [`Body::refresh_world`].
    pub world: Frame,
    /// The file's node flags.
    pub flags: u32,
    /// The build kind.
    pub kind: u32,
    /// The body slot its name gives.
    pub slot: usize,
    /// The layer digits of its name.
    pub layer: u32,
    /// Whether it is a mesh node (`kind & 0xFFF == 0x89`).
    pub mesh: bool,
    /// Whether the mesh is a held item's.
    pub held: bool,
    /// Whether the mesh is cloth (its dynamics carries the cloth kind).
    pub cloth: bool,
    /// Per-part hidden flag of its mesh parts.
    pub part_hidden: bool,
    /// The rigid-body state, if built.
    pub dynamics: Option<Dynamics>,
    /// The mesh's position count (a garment's is stored apart).
    pub position_count: usize,
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
            flags: 0,
            kind: 0,
            slot: 0,
            layer: 0,
            mesh: false,
            held: false,
            cloth: false,
            part_hidden: false,
            dynamics: None,
            position_count: 0,
            velocity: [0.0; 3],
        }
    }
}

/// A character body's node tree.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Body {
    pub nodes: Vec<Node>,
}

impl Body {
    /// A body of `nodes`, linking parents from the nodes' parent indices.
    pub fn new(nodes: Vec<Node>) -> Self {
        let mut body = Self { nodes };
        body.refresh_links();
        body.refresh_world();
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
            for &c in self.nodes[i].children.iter().rev() {
                stack.push(c);
            }
        }
        out
    }

    /// Rebuilds the children lists from the parents.
    pub fn refresh_links(&mut self) {
        for i in 0..self.nodes.len() {
            self.nodes[i].children.clear();
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

    /// Recomputes every node's world frame from its local frame and parent.
    pub fn refresh_world(&mut self) {
        let mut order = Vec::with_capacity(self.nodes.len());
        for root in self.roots() {
            order.extend(self.descendants(root));
        }
        for i in order {
            let world = match self.nodes[i].parent {
                Some(p) => compose(&self.nodes[p].world, &self.nodes[i].local),
                None => self.nodes[i].local,
            };
            self.nodes[i].world = world;
        }
    }

    /// The original's node-word write: the word goes on `root` and every
    /// node under it, the root first.
    pub fn set_kind(&mut self, root: usize, word: u32) {
        for i in self.descendants(root) {
            self.nodes[i].kind = word;
        }
    }

    /// Marks a character's held meshes: every mesh node under `root` takes
    /// the held flag and every part's hidden flag is set.
    pub fn mark_held(&mut self, root: usize) {
        for i in self.descendants(root) {
            if self.nodes[i].mesh {
                self.nodes[i].flags |= HELD;
                self.nodes[i].held = true;
                self.nodes[i].part_hidden = true;
            }
        }
    }

    /// Clears the held mark from every mesh node under `root`.
    pub fn unmark_held(&mut self, root: usize) {
        for i in self.descendants(root) {
            if self.nodes[i].mesh {
                self.nodes[i].flags &= !HELD;
                self.nodes[i].held = false;
                self.nodes[i].part_hidden = false;
            }
        }
    }

    /// The character's whole held mark: both hand subtrees, then the held
    /// weapon if the character is armed (its record flag `0x100`).
    pub fn mark_hands(&mut self, hands: [Option<usize>; 2], armed: bool) {
        for hand in hands.into_iter().flatten() {
            self.mark_held(hand);
        }
        if armed {
            for root in self.roots() {
                if self.nodes[root].kind == MESH && self.nodes[root].held {
                    self.mark_held(root);
                }
            }
        }
    }

    /// Numbers the mesh nodes that have no slot yet: each node outside the
    /// `#`/`_` story pieces gets the next number, starting at 1, and the
    /// render flags `0x500040`.
    pub fn number_unnamed(&mut self) {
        let mut next = 0;
        for i in 0..self.nodes.len() {
            let n = &mut self.nodes[i];
            if n.mesh && !n.name.starts_with('#') && !n.name.starts_with('_') {
                n.flags |= 0x50_0040;
                if n.slot == 0 {
                    next += 1;
                    n.slot = next;
                }
            }
        }
    }

    /// Writes each node's slot from its name's first two digits, through
    /// every subtree, and marks its mesh.
    pub fn assign_slots(&mut self, root: usize, digits: impl Fn(&str) -> usize) {
        for i in self.descendants(root) {
            let slot = digits(&self.nodes[i].name);
            if slot > 0 {
                self.nodes[i].slot = slot;
                if self.nodes[i].mesh {
                    self.nodes[i].flags |= 0x10_0000;
                }
            }
        }
    }

    /// The generator's slot table: each slot's selected node, cleared and
    /// then filled from names through the tree.
    pub fn chain_slots(
        &mut self,
        roots: &[usize],
        digits: impl Fn(&str) -> usize,
    ) -> [Option<usize>; 19] {
        let mut slots = [None; 19];
        for &root in roots {
            for i in self.descendants(root) {
                let slot = digits(&self.nodes[i].name);
                if (1..=18).contains(&slot) {
                    self.nodes[i].slot = slot;
                    slots[slot] = Some(i);
                }
            }
        }
        slots
    }

    /// Re-frames every node with a slot from the slot's stored rest frame
    ///: the local frame becomes the slot frame, the world
    /// frame is composed again and the children follow.
    pub fn reframe_from_slots(&mut self, frames: &[Frame; 19]) {
        let order: Vec<usize> = self
            .roots()
            .into_iter()
            .flat_map(|r| self.descendants(r))
            .collect();
        for i in order {
            let slot = self.nodes[i].slot;
            if (1..19).contains(&slot) {
                self.nodes[i].local = frames[slot];
            }
        }
        self.refresh_world();
    }

    /// The original's other frame mode: when the parent's flag `0x10` is
    /// set, the node's local frame is its stored frame rather than the
    /// slot's.
    pub fn reframe_own(&mut self) {
        self.refresh_world();
    }

    /// Saves each node's world frame: its three basis rows
    /// are normalised and its translation kept, before a rebuild.
    pub fn save_frames(&self) -> Vec<Frame> {
        self.nodes
            .iter()
            .map(|n| {
                let (r0, r1, r2) = orthonormal(n.world[0], n.world[1], n.world[2]);
                [r0, r1, r2, n.world[3]]
            })
            .collect()
    }

    /// Restores the saved local frames: a node whose parent
    /// is not flagged `0x10` copies the whole frame; a child of a flagged
    /// parent takes only its stored translation. **Guess:** the original
    /// copies four qwords and a dword from its saved block in that branch;
    /// only the translation is used here.
    pub fn restore_frames(&mut self, saved: &[Frame]) {
        for (i, frame) in saved.iter().enumerate() {
            let flagged = self.nodes.get(i).is_some_and(|node| {
                node.parent
                    .and_then(|p| self.nodes.get(p))
                    .is_some_and(|p| p.flags & 0x10 != 0)
            });
            if let Some(node) = self.nodes.get_mut(i) {
                if flagged {
                    node.local[3] = frame[3];
                } else {
                    node.local = *frame;
                }
            }
        }
        self.refresh_world();
    }

    /// Restores each node's local frame from a saved bone tree in walk order
    ///: the root first, then its children in the save's
    /// order, each frame copied and its world composed.
    pub fn restore_pose(&mut self, frames: &[Frame]) {
        let order: Vec<usize> = self
            .roots()
            .into_iter()
            .flat_map(|r| self.descendants(r))
            .collect();
        for (node, frame) in order.into_iter().zip(frames) {
            self.nodes[node].local = *frame;
        }
        self.refresh_world();
    }

    /// Recomputes every node's world frame from its parent after a rebuild
    ///.
    pub fn restore_world(&mut self) {
        self.refresh_world();
    }

    /// Clears every node's scene parent.
    pub fn clear_parents(&mut self, root: usize) {
        for i in self.descendants(root) {
            self.nodes[i].parent = None;
        }
        self.refresh_links();
    }

    /// Detaches a body from its scene: every node is
    /// unparented and the root's static flag is cleared.
    pub fn detach(&mut self, root: usize) {
        self.clear_parents(root);
        self.nodes[root].flags &= !STATIC;
        self.roots();
    }

    /// Sets a cloth velocity through a subtree: every node
    /// with cloth takes the velocity (the original calls the cloth body's
    /// velocity setter, which the tree keeps here).
    pub fn set_cloth_velocity(&mut self, root: usize, velocity: [f32; 3]) {
        for i in self.descendants(root) {
            if self.nodes[i].cloth {
                self.nodes[i].velocity = velocity;
            }
        }
    }

    /// Copies one body's node state onto another by matching slot, as the
    /// rebuild's state copy does: transforms, flags, kind,
    /// held and cloth state and position count. **Guess:** the saved body's
    /// descendants are paired in walk order with the rebuilt body's.
    pub fn copy_state(&mut self, dst_root: usize, src: &Body, src_root: usize) {
        let src_nodes: Vec<usize> = src.descendants(src_root);
        for (d, &s) in self.descendants(dst_root).iter().zip(&src_nodes) {
            let (flags, kind, held, cloth, count, local) = {
                let n = &src.nodes[s];
                (n.flags, n.kind, n.held, n.cloth, n.position_count, n.local)
            };
            let n = &mut self.nodes[*d];
            n.flags = flags;
            n.kind = kind;
            n.held = held;
            n.cloth = cloth;
            n.position_count = count;
            n.local = local;
        }
        self.refresh_world();
    }

    /// Builds a rigid body on `root` and every mesh child flagged for it, as
    /// the generator's first dynamics pass does.
    pub fn ensure_dynamics(&mut self, root: usize) {
        let mut stack = vec![root];
        while let Some(i) = stack.pop() {
            let follows = self.nodes[i].parent.filter(|&p| {
                p < self.nodes.len()
                    && self.nodes[p]
                        .dynamics
                        .is_some_and(|d| d.flags & 0x1000_0000 != 0)
            });
            if self.nodes[i].dynamics.is_none() {
                self.nodes[i].dynamics = Some(Dynamics {
                    follows,
                    ..Dynamics::built()
                });
                if self.nodes[i].mesh {
                    self.nodes[i].flags |= 0x400;
                }
            }
            for &c in &self.nodes[i].children {
                if self.nodes[c].mesh && self.nodes[c].flags & SUBDIVIDE != 0 {
                    stack.push(c);
                }
            }
        }
    }

    /// Splits a built body into a second one: the existing
    /// dynamics becomes a new one with `0x12000000`, its four scales moved
    /// halfway to 1, and the split is repeated on every flagged mesh child.
    pub fn split_dynamics(&mut self, root: usize) {
        let mut stack = vec![root];
        while let Some(i) = stack.pop() {
            let Some(old) = self.nodes[i].dynamics else {
                self.ensure_dynamics(i);
                continue;
            };
            if self.nodes[i].mesh {
                self.nodes[i].flags |= 0x400;
            }
            let mut new = old.split();
            new.follows = old.follows;
            if let Some(p) = self.nodes[i]
                .parent
                .and_then(|p| self.nodes.get(p))
                .and_then(|p| p.dynamics)
                && old.flags & 0x1000_0000 == 0
                && p.flags & 0x1000_0000 != 0
            {
                new.follows = Some(self.nodes[i].parent.unwrap_or(i));
            }
            self.nodes[i].dynamics = Some(new);
            for &c in &self.nodes[i].children {
                if self.nodes[c].mesh && self.nodes[c].flags & SUBDIVIDE != 0 {
                    stack.push(c);
                }
            }
        }
    }

    /// Rebuilds an existing node's dynamics: a character
    /// body has its four scales reset to 1 and its rest vectors to 1.25,
    /// then its mesh children are rebuilt too.
    pub fn rebuild_dynamics(&mut self, root: usize) {
        for i in self.descendants(root) {
            if let Some(mut d) = self.nodes[i].dynamics
                && d.flags & 0x1200_0000 == 0x1200_0000
            {
                d.scales = [1.0; 4];
                d.rest = [[1.25; 3]; 4];
                self.nodes[i].dynamics = Some(d);
                self.nodes[i].flags |= 0x400;
            }
        }
    }

    /// The generator's node-tree clone: a deep copy of the
    /// body's nodes, frames and dynamics, linked again.
    pub fn clone_tree(&self) -> Body {
        let mut out = self.clone();
        out.refresh_links();
        out.refresh_world();
        out
    }

    /// The selected merge nodes in body order:
    /// each chosen slot's node and every mesh child under it, once, roots
    /// before leaves.
    pub fn merge_order(&self, chosen: &[Option<usize>; 19], visible: u32) -> Vec<usize> {
        let mut out = Vec::new();
        let mut seen = vec![false; self.nodes.len()];
        for (i, chosen) in chosen.iter().enumerate().skip(1) {
            let Some(node) = *chosen else {
                continue;
            };
            if visible & crate::layers::slot_bit(i) == 0 {
                continue;
            }
            for i in self.descendants(node) {
                if !seen[i] && self.nodes[i].mesh {
                    seen[i] = true;
                    out.push(i);
                }
            }
        }
        // Overlay meshes past the slots come last.
        for i in overlay_nodes(self) {
            if !seen[i] {
                out.push(i);
            }
        }
        out
    }

    /// The number of positions a saved pose's node takes, and its frame.
    pub fn pose(&self) -> Vec<Frame> {
        self.nodes.iter().map(|n| n.local).collect()
    }
}

/// Gram-Schmidt rows as the original normalises a saved frame.
pub fn orthonormal(r0: [f32; 3], r1: [f32; 3], _r2: [f32; 3]) -> ([f32; 3], [f32; 3], [f32; 3]) {
    let norm = |v: [f32; 3]| {
        let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        if l > 0.0 {
            [v[0] / l, v[1] / l, v[2] / l]
        } else {
            v
        }
    };
    let r0 = norm(r0);
    let d = r1[0] * r0[0] + r1[1] * r0[1] + r1[2] * r0[2];
    let r1 = norm([r1[0] - r0[0] * d, r1[1] - r0[1] * d, r1[2] - r0[2] * d]);
    let r2 = norm([
        r0[1] * r1[2] - r0[2] * r1[1],
        r0[2] * r1[0] - r0[0] * r1[2],
        r0[0] * r1[1] - r0[1] * r1[0],
    ]);
    (r0, r1, r2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(name: &str, parent: Option<usize>) -> Node {
        Node {
            name: name.into(),
            parent,
            mesh: true,
            kind: MESH,
            ..Node::default()
        }
    }

    /// Pelvis (0) with two leg chains and a chest/head chain; names are the
    /// body model's.
    fn body() -> Body {
        let mut nodes = vec![
            node("01Pelvis", None),
            node("02RULeg", Some(0)),
            node("03RLLeg", Some(1)),
            node("09Chest", Some(0)),
            node("10Head", Some(3)),
        ];
        nodes[0].kind = 0;
        Body::new(nodes)
    }

    #[test]
    fn a_word_writes_the_root_and_every_child() {
        let mut b = body();
        b.set_kind(3, 0x1234);
        assert_eq!(b.nodes[3].kind, 0x1234);
        assert_eq!(b.nodes[4].kind, 0x1234);
        assert_eq!(b.nodes[0].kind, 0);
        assert_eq!(b.nodes[1].kind, MESH);
    }

    #[test]
    fn held_marks_follow_the_hand_subtrees() {
        let mut b = body();
        b.mark_held(1);
        assert!(b.nodes[1].held && b.nodes[2].held);
        assert!(!b.nodes[0].held);
        assert_eq!(b.nodes[1].flags & HELD, HELD);
        b.unmark_held(1);
        assert!(!b.nodes[1].held && !b.nodes[2].held);
    }

    #[test]
    fn the_slot_chain_reads_the_names() {
        let mut b = body();
        let slots = b.chain_slots(&[0], |n| crate::layers::slot_of(n).unwrap_or(0));
        assert_eq!(slots[1], Some(0));
        assert_eq!(slots[2], Some(1));
        assert_eq!(slots[3], Some(2));
        assert_eq!(slots[9], Some(3));
        assert_eq!(slots[10], Some(4));
        assert_eq!(b.nodes[4].slot, 10);
    }

    #[test]
    fn reframing_takes_the_slot_frames_and_moves_children() {
        let mut b = body();
        b.chain_slots(&[0], |n| crate::layers::slot_of(n).unwrap_or(0));
        let mut frames = [IDENTITY; 19];
        frames[9][3] = [0.1, 1.0, 0.0];
        frames[10][3] = [0.0, 0.2, 0.0];
        b.reframe_from_slots(&frames);
        assert_eq!(b.nodes[3].local[3], [0.1, 1.0, 0.0]);
        assert_eq!(b.nodes[4].world[3], [0.1, 1.2, 0.0]);
    }

    #[test]
    fn the_split_moves_the_scales_halfway_to_one() {
        let mut b = body();
        b.ensure_dynamics(0);
        b.nodes[0].dynamics.as_mut().unwrap().scales = [0.0, 0.5, 1.0, 2.0];
        b.split_dynamics(0);
        let d = b.nodes[0].dynamics.unwrap();
        assert_eq!(d.flags, 0x1200_0000);
        assert_eq!(d.scales, [0.5, 0.75, 1.0, 1.5]);
        assert_eq!(b.nodes[0].flags & 0x400, 0x400);
    }

    #[test]
    fn the_rebuild_resets_scales_and_rest() {
        let mut b = body();
        b.nodes[0].dynamics = Some(Dynamics {
            flags: 0x1200_0000,
            scales: [0.2; 4],
            rest: [[9.0; 3]; 4],
            follows: None,
        });
        b.rebuild_dynamics(0);
        let d = b.nodes[0].dynamics.unwrap();
        assert_eq!(d.scales, [1.0; 4]);
        assert_eq!(d.rest, [[1.25; 3]; 4]);
        assert_eq!(b.nodes[0].flags & 0x400, 0x400);
    }

    #[test]
    fn frames_save_and_restore() {
        let mut b = body();
        b.nodes[3].local[3] = [1.0, 2.0, 3.0];
        b.refresh_world();
        let saved = b.save_frames();
        b.nodes[3].local[3] = [0.0; 3];
        b.restore_frames(&saved);
        assert_eq!(b.nodes[3].world[3], [1.0, 2.0, 3.0]);
        // A parent flagged 0x10 keeps only the translation.
        b.nodes[3].flags |= 0x10;
        b.nodes[4].local[0] = [7.0, 0.0, 0.0];
        b.nodes[4].local[3] = [9.0; 3];
        b.restore_frames(&saved);
        // The basis is left alone; only the saved translation comes back.
        assert_eq!(b.nodes[4].local[0], [7.0, 0.0, 0.0]);
        assert_eq!(b.nodes[4].local[3], saved[4][3]);
    }

    #[test]
    fn a_pose_restores_each_node_in_walk_order() {
        let mut b = body();
        let mut frames = b.pose();
        frames[1][3] = [0.0, 0.5, 0.0];
        b.restore_pose(&frames);
        assert_eq!(b.nodes[1].local[3], [0.0, 0.5, 0.0]);
    }

    #[test]
    fn detaching_clears_parents_and_the_static_flag() {
        let mut b = body();
        b.nodes[0].flags |= STATIC;
        b.detach(0);
        assert!(b.nodes[0].parent.is_none());
        assert_eq!(b.nodes[0].flags & STATIC, 0);
    }

    #[test]
    fn cloning_and_merging_keep_the_tree_and_order() {
        let b = body();
        let clone = b.clone_tree();
        assert_eq!(clone, b);
        let mut chosen = [None; 19];
        chosen[1] = Some(0);
        let order = b.merge_order(&chosen, 0xFFFF);
        assert_eq!(order, [0, 1, 2, 3, 4]);
        assert!(b.merge_order(&chosen, 0).is_empty());
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
    fn overlays_are_the_held_meshes_past_the_slots() {
        let mut b = body();
        b.nodes[4].slot = 20;
        b.nodes[4].flags = 0x40_0040;
        let overlays = mark_overlays(&mut b);
        assert_eq!(overlays, [4]);
        assert_eq!(b.nodes[4].flags & 0x0C_0000, 0x0C_0000);
        assert_eq!(overlay_nodes(&b), [4]);
    }

    #[test]
    fn cloth_takes_its_velocity() {
        let mut b = body();
        b.nodes[4].cloth = true;
        b.set_cloth_velocity(0, [1.0, 2.0, 3.0]);
        assert_eq!(b.nodes[4].velocity, [1.0, 2.0, 3.0]);
        assert_eq!(b.nodes[3].velocity, [0.0; 3]);
    }

    /// Normalising a frame keeps a right-handed row triple.
    #[test]
    fn frames_are_normalised_for_saving() {
        let (r0, r1, r2) = orthonormal([2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 4.0]);
        assert_eq!(r0, [1.0, 0.0, 0.0]);
        assert_eq!(r1, [0.0, 1.0, 0.0]);
        assert_eq!(r2, [0.0, 0.0, 1.0]);
    }
}
