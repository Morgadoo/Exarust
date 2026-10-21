//! Apparel layering: the original's eighteen body slots, the layer a body or
//! garment node belongs to, and which garment nodes and body parts a worn
//! set keeps.
//!
//! A node's name begins with its slot's two digits and, for apparel, two
//! more digits whose decimal value is a set of sub-part bits (`0906Chest`:
//! chest, bits `0b110`). The builder runs one pass per worn garment, in
//! layer-level order: [`garment_pass`] compares the garment's own
//! occupancy bytes with the worn set's coverage union and yields the
//! per-slot cover the node layers are scored against and the garment's
//! visible mask; the [`Assembler`] then walks the slot tree, keeps the
//! best-scoring groups of nodes slot by slot and chooses the first group's
//! nodes where [`SELECT_SLOT_MASK`] meets the visible mask. Once every
//! garment is merged, [`visible_body_slots`] of the union decides which
//! body parts the body model still adds. Evidence: the character builder's
//! slot table, coverage pass, slot assembler and merge passes; the slot
//! names are those of the shipped `humanbase.rfc`.
//!
//! The tables below are the original's data, written in Exarust's terms;
//! the sub-part bits themselves are not decoded.

/// A body slot: the eighteen bones a character's parts and garments stack
/// on, named as the body model's nodes are.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    /// The body model's node name without its digits (`Pelvis`).
    pub name: &'static str,
    /// The slot this one hangs under, 0 for the pelvis.
    pub parent: usize,
    /// The slots that hang under this one.
    pub children: &'static [usize],
    /// The original's per-slot flag word: `0x1F` on the head, `0x7` on the
    /// legs, belly, chest and arms, `0x3` on the pelvis, feet, shoulders and
    /// hands. The layer passes below do not read it.
    pub flags: u32,
}

/// The eighteen body slots, indexed by their number less one.
pub const SLOTS: [Slot; 18] = [
    Slot {
        name: "Pelvis",
        parent: 0,
        children: &[2, 5, 8],
        flags: 0x3,
    },
    Slot {
        name: "RULeg",
        parent: 1,
        children: &[3],
        flags: 0x7,
    },
    Slot {
        name: "RLLeg",
        parent: 2,
        children: &[4],
        flags: 0x7,
    },
    Slot {
        name: "RFoot",
        parent: 3,
        children: &[],
        flags: 0x3,
    },
    Slot {
        name: "LULeg",
        parent: 1,
        children: &[6],
        flags: 0x7,
    },
    Slot {
        name: "LLLeg",
        parent: 5,
        children: &[7],
        flags: 0x7,
    },
    Slot {
        name: "LFoot",
        parent: 6,
        children: &[],
        flags: 0x3,
    },
    Slot {
        name: "Belly",
        parent: 1,
        children: &[9],
        flags: 0x7,
    },
    Slot {
        name: "Chest",
        parent: 8,
        children: &[10, 11, 14],
        flags: 0x7,
    },
    Slot {
        name: "Head",
        parent: 9,
        children: &[],
        flags: 0x1F,
    },
    Slot {
        name: "RShld",
        parent: 9,
        children: &[12],
        flags: 0x3,
    },
    Slot {
        name: "RUArm",
        parent: 11,
        children: &[13],
        flags: 0x7,
    },
    Slot {
        name: "RFArm",
        parent: 12,
        children: &[17],
        flags: 0x7,
    },
    Slot {
        name: "LShld",
        parent: 9,
        children: &[15],
        flags: 0x3,
    },
    Slot {
        name: "LUArm",
        parent: 14,
        children: &[16],
        flags: 0x7,
    },
    Slot {
        name: "LFarm",
        parent: 15,
        children: &[18],
        flags: 0x7,
    },
    Slot {
        name: "RHand",
        parent: 13,
        children: &[],
        flags: 0x3,
    },
    Slot {
        name: "LHand",
        parent: 16,
        children: &[],
        flags: 0x3,
    },
];

/// Per-slot partial masks of the coverage pass: a `0x40` byte above a
/// garment that holds none of these sub-part bits (or a garment holding
/// none of them at all) hides the slot's [`CHILD_SLOT_MASK`] bits.
pub const HIDDEN_SLOT_MASK: [u32; 18] = [
    0x2, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x2, 0x1, 0x1C, 0x2, 0x2, 0x2, 0x2, 0x2, 0x2, 0x0, 0x0,
];

/// Per-slot masks of the coverage pass: a `0x80` byte above a garment that
/// holds none of these sub-part bits (or a garment holding none of them at
/// all) hides the slot.
pub const VISIBLE_SLOT_MASK: [u32; 18] = [
    0x1, 0x7, 0x7, 0x7, 0x7, 0x7, 0x7, 0x1, 0x6, 0x3, 0x1, 0x1, 0x1, 0x1, 0x1, 0x1, 0x3, 0x3,
];

/// The partial bits a slot's `0x40` coverage hides: bits 20 to 31 of the
/// visible mask, above the eighteen slot bits, read back through
/// [`SELECT_SLOT_MASK`] and the cull masks.
pub const CHILD_SLOT_MASK: [u32; 18] = [
    0x10_0000,
    0,
    0,
    0,
    0,
    0,
    0,
    0x8000_0000,
    0x20_0000,
    0x1000_0000,
    0x40_0000,
    0x80_0000,
    0x100_0000,
    0x200_0000,
    0x400_0000,
    0x800_0000,
    0,
    0,
];

/// Per-slot masks of the cull pass: when the slot's first
/// mask does not meet the visible body mask the merged triangles take cull
/// bit `0x200000`, and when the second does not, `0x400000`.
pub const CULL_FIRST: [u32; 18] = [
    0x1, 0, 0, 0, 0, 0, 0, 0x80, 0x100, 0x200, 0, 0x800, 0x1000, 0, 0x4000, 0x8000, 0, 0,
];
/// The second cull mask per slot.
pub const CULL_SECOND: [u32; 18] = [
    0x10_0000,
    0,
    0,
    0,
    0,
    0,
    0,
    0x8000_0000,
    0x20_0000,
    0x1000_0000,
    0,
    0x80_0000,
    0x100_0000,
    0,
    0x400_0000,
    0x800_0000,
    0,
    0,
];

/// The cull bits a slot's merged triangles take from the garment's visible
/// mask: `0x400000` when the second mask does not meet it, else `0x200000`
/// when the first does not, else none. The second test overwrites the
/// first, so a slot never takes both bits.
pub fn cull_bits(slot: usize, visible: u32) -> u32 {
    let Some(i) = slot.checked_sub(1).filter(|&i| i < 18) else {
        return 0;
    };
    if CULL_SECOND[i] & visible == 0 {
        0x40_0000
    } else if CULL_FIRST[i] & visible == 0 {
        0x20_0000
    } else {
        0
    }
}

/// The per-slot bit of a slot in a `u32` body mask.
pub const fn slot_bit(slot: usize) -> u32 {
    1 << (slot - 1)
}

/// The slot a node's name begins with: its first two digits, 1 to 18. A name
/// the builder keeps out of the story (`#…`, `_…`) or one without digits has
/// no slot.
pub fn slot_of(name: &str) -> Option<usize> {
    let b = name.as_bytes();
    if b.len() < 2 || !b[0].is_ascii_digit() || !b[1].is_ascii_digit() {
        return None;
    }
    let slot = usize::from(b[0] - b'0') * 10 + usize::from(b[1] - b'0');
    (1..=SLOTS.len()).contains(&slot).then_some(slot)
}

/// The layer a node's name names: its third and fourth digits (`0900Chest`
/// is layer 0). A name without them is layer 0, as the builder's parse
/// returns 0.
pub fn layer_of(name: &str) -> u32 {
    let b = name.as_bytes();
    if b.len() < 4 || !b[2].is_ascii_digit() || !b[3].is_ascii_digit() {
        return 0;
    }
    u32::from(b[2] - b'0') * 10 + u32::from(b[3] - b'0')
}

/// The number of set bits among the low five bits: the original's layer
/// scorer counts five sub-part bits of the layer word.
pub fn five_bit_count(v: u64) -> i32 {
    (0..5).filter(|&i| v & (1 << i) != 0).count() as i32
}

/// How well a node of `layer` matches a slot's coverage `word`:
///
/// `matched = popcount(layer & word)`; when `layer & word` does not hold all
/// of `layer`, the unmatched bits cost eight each from a base of sixteen.
/// A garment whose layer bits the slot's coverage holds scores highest, then
/// the one sharing most of them.
pub fn layer_score(layer: u32, word: u64) -> i32 {
    let layer = u64::from(layer);
    let matched = layer & word;
    let base = if matched < layer {
        16 - 8 * five_bit_count(!word & layer)
    } else {
        16
    };
    five_bit_count(matched) + base
}

/// The outermost layer an eight-byte coverage word occupies: its highest
/// non-zero byte, or `None` for an empty word.
pub fn word_layer(word: u64) -> Option<u8> {
    (0..8)
        .rev()
        .find(|&i| (word >> (8 * i)) & 0xFF != 0)
        .map(|i| i as u8)
}

/// The garment's layer level: the highest layer any of its twenty
/// occupancy words holds (the index of the word's highest non-zero byte),
/// 0 for a garment that occupies nothing. The builder keeps its worn
/// garments sorted by it, so the outermost is assembled and merged last.
pub fn layer_level(occupancy: &[u64]) -> u8 {
    occupancy
        .iter()
        .filter_map(|&w| word_layer(w))
        .max()
        .unwrap_or(0)
}

/// The per-slot body masks the builder tests a slot's chosen node against:
/// a node is kept (and a body part merged) when its slot's mask meets the
/// visible mask. Each holds the slot's own bit; the pelvis, belly, chest,
/// head and arms also hold the partial bits [`CHILD_SLOT_MASK`] uses, and
/// the chest the shoulders' bits.
pub const SELECT_SLOT_MASK: [u32; 18] = [
    0x10_0001,
    0x2,
    0x4,
    0x8,
    0x10,
    0x20,
    0x40,
    0x8000_0080,
    0x20_2500,
    0x1000_0200,
    0x40_0400,
    0xC0_0800,
    0x180_1000,
    0x200_2000,
    0x600_4000,
    0xC00_8000,
    0x1_0000,
    0x2_0000,
];

/// Whether slot `slot`'s node is kept under `visible`, as the builder tests
/// [`SELECT_SLOT_MASK`].
pub fn slot_selected(slot: usize, visible: u32) -> bool {
    slot.checked_sub(1)
        .and_then(|i| SELECT_SLOT_MASK.get(i))
        .is_some_and(|m| m & visible != 0)
}

/// What the builder's coverage pass computes for one worn garment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GarmentPass {
    /// Per slot (index 0 unused): the worn set's layer bytes that lie above
    /// the garment's own layers, or-ed into one byte. The layer scorer
    /// matches node layers against it.
    pub cover: [u64; 19],
    /// Per slot: the garment's own layer bytes or-ed together, for the
    /// slots the garment has nodes in; zero elsewhere. A non-zero word is
    /// what makes the assembler look at the slot.
    pub words: [u64; 19],
    /// Per slot: the garment's bytes the cover does not hold.
    pub uncovered: [u64; 19],
    /// The visible mask: the inverse of the slots (and partial bits) the
    /// pass hides for this garment.
    pub visible: u32,
}

/// The builder's coverage pass for one garment.
///
/// `occupancy[slot - 1]` is the garment's eight-byte occupancy word of the
/// slot (layer L is byte L), `union[slot - 1]` the or of every worn
/// garment's coverage word, and `slots` the slots the garment has nodes in
/// (bit `slot - 1`). For each slot the garment's layers are walked from the
/// outermost in; at layer L the union's bytes above L are tested against
/// the garment's own bytes above L: a union byte with `0x80` hides the slot
/// unless the garment already holds part of [`VISIBLE_SLOT_MASK`] there,
/// one with `0x40` hides the [`CHILD_SLOT_MASK`] bits unless it holds part
/// of [`HIDDEN_SLOT_MASK`], and the byte joins the slot's cover where the
/// garment does not hold it. A garment holding none of a slot's
/// [`VISIBLE_SLOT_MASK`] (or [`HIDDEN_SLOT_MASK`]) in any layer hides the
/// slot (or the child bits) too. The union's innermost byte is never
/// tested, as in the original.
pub fn garment_pass(occupancy: &[u64; 18], union: &[u64; 18], slots: u32) -> GarmentPass {
    let mut pass = GarmentPass::default();
    let mut hidden = 0u32;
    for i in 0..18 {
        let slot = i + 1;
        let mut higher = 0u64;
        for layer in (0..8).rev() {
            let own = (occupancy[i] >> (8 * layer)) & 0xFF;
            for above in layer + 1..8 {
                let byte = (union[i] >> (8 * above)) & 0xFF;
                pass.cover[slot] |= !higher & byte;
                if byte & 0x80 != 0 && u64::from(VISIBLE_SLOT_MASK[i]) & higher == 0 {
                    hidden |= slot_bit(slot);
                }
                if byte & 0x40 != 0 && u64::from(HIDDEN_SLOT_MASK[i]) & higher == 0 {
                    hidden |= CHILD_SLOT_MASK[i];
                }
            }
            higher |= own;
        }
        if u64::from(VISIBLE_SLOT_MASK[i]) & higher == 0 {
            hidden |= slot_bit(slot);
        }
        if u64::from(HIDDEN_SLOT_MASK[i]) & higher == 0 {
            hidden |= CHILD_SLOT_MASK[i];
        }
        if slots & slot_bit(slot) != 0 {
            pass.words[slot] = higher;
            pass.uncovered[slot] = !pass.cover[slot] & higher;
        }
    }
    pass.visible = !hidden;
    pass
}

/// The body's visible mask once every garment is assembled: a slot whose
/// union coverage word has `0x80` in its innermost byte is hidden, one with
/// `0x40` there hides its [`CHILD_SLOT_MASK`] bits. Only the innermost byte
/// is read: the builder tests the or-ed 64-bit words against `0x80` and
/// `0x40` directly.
pub fn visible_body_slots(union: &[u64; 18]) -> u32 {
    let mut hidden = 0u32;
    for (i, &word) in union.iter().enumerate() {
        if word & 0x80 != 0 {
            hidden |= slot_bit(i + 1);
        }
        if word & 0x40 != 0 {
            hidden |= CHILD_SLOT_MASK[i];
        }
    }
    !hidden
}

/// The body slots the builder merges from the body model under the
/// visible mask of [`visible_body_slots`], as a slot mask (bit
/// `slot - 1`): those whose [`SELECT_SLOT_MASK`] meets it.
pub fn drawn_body_slots(visible: u32) -> u32 {
    (1..=18)
        .filter(|&s| slot_selected(s, visible))
        .fold(0, |m, s| m | slot_bit(s))
}

/// A candidate group kept by the assembler: one node per slot and the
/// group's total score.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SlotGroup {
    pub nodes: [Option<usize>; 19],
    pub score: i32,
}

/// The builder's slot assembler for one garment: its node trees, the
/// garment's coverage pass, the groups of equal-scoring candidates and the
/// chosen node per slot.
pub struct Assembler<'a> {
    body: &'a crate::runtime::Body,
    roots: &'a [usize],
    pass: &'a GarmentPass,
    /// The candidate groups of the slot being assembled.
    pub groups: Vec<SlotGroup>,
    /// The best total score the groups reached.
    pub best: i32,
    /// The chosen node per slot (index 0 unused).
    pub chosen: [Option<usize>; 19],
}

/// Pushes a node's children so that the last child is visited first, as
/// the builder's explicit stack walks them.
fn push_children(stack: &mut Vec<usize>, node: &crate::runtime::Node) {
    stack.extend(node.children.iter().copied());
}

impl<'a> Assembler<'a> {
    pub fn new(body: &'a crate::runtime::Body, roots: &'a [usize], pass: &'a GarmentPass) -> Self {
        Self {
            body,
            roots,
            pass,
            groups: Vec::new(),
            best: 0,
            chosen: [None; 19],
        }
    }

    fn score(&self, node: usize, slot: usize) -> i32 {
        layer_score(self.body.nodes[node].layer, self.pass.cover[slot])
    }

    /// Assembles one slot: every node of the garment's trees in `target`
    /// is scored; the best score clears the groups and equal scores add
    /// one. Nodes in the slot are not searched below. With a group, the
    /// child slots the garment has words in are pruned, and the first
    /// group's nodes fill the chosen slots [`slot_selected`] keeps.
    pub fn assemble_slot(&mut self, target: usize) -> bool {
        if !(1..=SLOTS.len()).contains(&target) {
            return false;
        }
        self.groups.clear();
        self.best = -0x1000;
        for &root in self.roots {
            let mut stack = vec![root];
            while let Some(i) = stack.pop() {
                let n = &self.body.nodes[i];
                if n.slot == target {
                    let score = self.score(i, target);
                    if score >= self.best {
                        if score > self.best {
                            self.groups.clear();
                            self.best = score;
                        }
                        let mut group = SlotGroup {
                            score,
                            ..SlotGroup::default()
                        };
                        group.nodes[target] = Some(i);
                        self.groups.push(group);
                    }
                } else {
                    // The original logs "Body part stack overflow" past 32
                    // entries; the stack here grows instead.
                    push_children(&mut stack, n);
                }
            }
        }
        if self.groups.is_empty() {
            return false;
        }
        for &child in SLOTS[target - 1].children {
            if self.pass.words[child] != 0 {
                self.prune_slot(child);
            }
        }
        if let Some(first) = self.groups.first().copied() {
            for slot in 1..=18 {
                if first.nodes[slot].is_some() && slot_selected(slot, self.pass.visible) {
                    self.chosen[slot] = first.nodes[slot];
                }
            }
        }
        true
    }

    /// Prunes the groups at `slot`: each group's node in the parent slot is
    /// searched for a child in `slot` (the best scoring, ties to the
    /// first); when it has none, [`Self::related_search`] supplies one.
    /// The found node joins the group and its score adds to the group's.
    /// Groups below the best total go, and, when the first group has a node
    /// in the slot, so do the groups without one; then the child slots the
    /// garment has words in are pruned in turn. Removed groups are replaced
    /// by the last one, as the original's arrays are.
    pub fn prune_slot(&mut self, slot: usize) {
        if !(1..=SLOTS.len()).contains(&slot) {
            return;
        }
        let parent = SLOTS[slot - 1].parent;
        let mut best = self.best;
        for g in 0..self.groups.len() {
            let Some(owner) = self.groups[g].nodes.get(parent).copied().flatten() else {
                continue;
            };
            let mut found = false;
            let mut pick = None;
            let mut score = -64;
            for &c in &self.body.nodes[owner].children {
                if self.body.nodes[c].slot == slot {
                    found = true;
                    let s = self.score(c, slot);
                    if score < s {
                        pick = Some(c);
                        score = s;
                    }
                }
            }
            if !found {
                pick = self.related_search(slot, self.body.nodes[owner].layer);
                if let Some(c) = pick {
                    score = self.score(c, slot);
                }
            }
            if let Some(c) = pick {
                let group = &mut self.groups[g];
                group.nodes[slot] = Some(c);
                group.score += score;
                best = best.max(group.score);
            }
        }
        self.best = best;
        let mut i = 0;
        while i < self.groups.len() {
            if self.groups[i].score < best {
                self.groups.swap_remove(i);
            } else {
                i += 1;
            }
        }
        if self.groups.first().is_some_and(|g| g.nodes[slot].is_some()) {
            let mut i = 1;
            while i < self.groups.len() {
                if self.groups[i].nodes[slot].is_none() {
                    self.groups.swap_remove(i);
                } else {
                    i += 1;
                }
            }
            for &child in SLOTS[slot - 1].children {
                if self.pass.words[child] != 0 {
                    self.prune_slot(child);
                }
            }
        }
    }

    /// The fallback search when a group's parent node has no child in
    /// `slot`: walk the garment's trees for nodes in the parent slot; the
    /// first one whose layer is not `wanted` ends that tree's walk, and one
    /// whose layer is offers its children in `slot`, the best scoring one
    /// winning (ties to the first).
    pub fn related_search(&self, slot: usize, wanted: u32) -> Option<usize> {
        if !(1..=SLOTS.len()).contains(&slot) {
            return None;
        }
        let parent = SLOTS[slot - 1].parent;
        let mut best = (-64, None);
        for &root in self.roots {
            let mut stack = vec![root];
            while let Some(i) = stack.pop() {
                let n = &self.body.nodes[i];
                if n.slot == parent {
                    if n.layer != wanted {
                        break;
                    }
                    for &c in &n.children {
                        if self.body.nodes[c].slot == slot {
                            let score = self.score(c, slot);
                            if best.0 < score {
                                best = (score, Some(c));
                            }
                        }
                    }
                } else {
                    push_children(&mut stack, n);
                }
            }
        }
        best.1
    }

    /// Walks the slot tree from `slot`: under a slot the garment has no
    /// word in, each child with a word is assembled and every child walked;
    /// under one it has a word in, the children are only walked.
    pub fn walk(&mut self, slot: usize) {
        if !(1..=SLOTS.len()).contains(&slot) {
            return;
        }
        let own = self.pass.words[slot] != 0;
        for &child in SLOTS[slot - 1].children {
            if !own && self.pass.words[child] != 0 {
                self.assemble_slot(child);
            }
            self.walk(child);
        }
    }
}

/// Assembles one garment: the pelvis when the garment has a word there,
/// then the walk from the pelvis. Returns the chosen node per slot.
pub fn assemble_garment(
    body: &crate::runtime::Body,
    roots: &[usize],
    pass: &GarmentPass,
) -> [Option<usize>; 19] {
    let mut assembler = Assembler::new(body, roots, pass);
    if pass.words[1] != 0 {
        assembler.assemble_slot(1);
    }
    assembler.walk(1);
    assembler.chosen
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shipped body model's slot names, in order. `humanbase.rfc` has
    /// these eighteen parts; the table's names are theirs.
    #[test]
    fn the_slots_are_the_body_models_parts() {
        assert_eq!(SLOTS.len(), 18);
        assert_eq!(SLOTS[0].name, "Pelvis");
        assert_eq!(SLOTS[8].name, "Chest");
        assert_eq!(SLOTS[9].name, "Head");
        assert_eq!(SLOTS[16].name, "RHand");
        assert_eq!(SLOTS[17].name, "LHand");
        // Each slot hangs under its parent and is one of the parent's
        // children, as the table's parent and child words say.
        for (i, slot) in SLOTS.iter().enumerate().skip(1) {
            assert!(SLOTS[slot.parent - 1].children.contains(&(i + 1)), "{i}");
        }
        assert_eq!(SLOTS[9].flags, 0x1F);
        assert_eq!(SLOTS[0].flags, 0x3);
        assert_eq!(SLOTS[0].children, [2, 5, 8]);
        assert_eq!(SLOTS[8].children, [10, 11, 14]);
        assert_eq!(SLOTS[15].children, [18]);
    }

    #[test]
    fn a_nodes_name_names_its_slot_and_layer() {
        assert_eq!(slot_of("01Pelvis"), Some(1));
        assert_eq!(slot_of("0900Chest"), Some(9));
        assert_eq!(slot_of("1000Head"), Some(10));
        assert_eq!(slot_of("17RHand"), Some(17));
        assert_eq!(slot_of("1900Bogus"), None);
        assert_eq!(slot_of("00Bogus"), None);
        assert_eq!(slot_of("#hidden"), None);
        assert_eq!(slot_of("_hidden"), None);
        assert_eq!(layer_of("0900Chest"), 0);
        assert_eq!(layer_of("0904Chest"), 4);
        assert_eq!(layer_of("09Chest"), 0);
        assert_eq!(layer_of("01Pelvis"), 0);
    }

    /// A node's layer matches the garment's own coverage: breastplate chest
    /// coverage is 0x06 on layer 6: both layer 6 and layer 2 are held by
    /// it, so layer 6 scores 18 (two matched bits plus the sixteen base)
    /// and layer 2 scores 17 (one plus sixteen).
    #[test]
    fn a_layer_scores_by_its_match_with_the_slot_coverage() {
        assert_eq!(layer_score(6, 0x06), 18);
        assert_eq!(layer_score(2, 0x06), 17);
        assert_eq!(layer_score(0, 0x06), 16);
        assert_eq!(layer_score(6, 0), 0);
        assert_eq!(layer_score(1, 0), 8);
        assert_eq!(layer_score(3, 0), 0);
        // Only the low five bits count, as the original's counter does.
        assert_eq!(layer_score(32, u64::MAX), 16);
    }

    #[test]
    fn the_outermost_layer_is_the_highest_nonzero_byte() {
        assert_eq!(word_layer(0), None);
        assert_eq!(word_layer(0x03), Some(0));
        assert_eq!(word_layer(0x0000_0000_0600_0000), Some(3));
        assert_eq!(word_layer(0x0083_8383_8383_8383), Some(6));
    }

    /// A plate helm's coverage hides the head: its head word is
    /// `0x0083_8383_8383_8383` (the 0x80 hide bit on every layer).
    #[test]
    fn a_helm_hides_the_head() {
        let mut words = [0u64; 18];
        words[9] = 0x0083_8383_8383_8383;
        let visible = visible_body_slots(&words);
        assert_eq!(visible & slot_bit(10), 0);
        assert_ne!(visible & slot_bit(9), 0);
        // No partial bit: 0x83 has no 0x40.
        assert_eq!(!visible & !0x3_FFFF, 0);
    }

    /// Only the innermost byte of the union hides a body part: a hide bit on
    /// layer 3 alone leaves the slot visible.
    #[test]
    fn only_the_innermost_byte_hides_the_body() {
        let mut words = [0u64; 18];
        words[9] = 0x8000_0000;
        assert_ne!(visible_body_slots(&words) & slot_bit(10), 0);
        // A partial byte hides the slot's partial bit.
        words[8] = 0x41;
        let visible = visible_body_slots(&words);
        assert_eq!(visible & CHILD_SLOT_MASK[8], 0);
        assert_ne!(visible & slot_bit(9), 0);
    }

    /// Boots hide the feet with `0x83` innermost bytes; left and right are
    /// independent.
    #[test]
    fn boots_hide_the_feet() {
        let mut words = [0u64; 18];
        words[2] = 0x0006_0606;
        words[3] = 0x0083_8383;
        words[5] = 0x0006_0606;
        words[6] = 0x0083_8383;
        let visible = visible_body_slots(&words);
        assert_eq!(visible & slot_bit(4), 0);
        assert_eq!(visible & slot_bit(7), 0);
        assert_ne!(visible & slot_bit(3), 0);
        assert_ne!(visible & slot_bit(1), 0);
    }

    /// The breastplate's union: chest `0x0101…`, head/arms `0x0101…` and
    /// `0x0606…`. Its slots stay visible; only a helm's 0x80 hides a part.
    #[test]
    fn a_breastplate_covers_without_hiding_the_body() {
        let mut words = [0u64; 18];
        words[7] = 0x0606_0606_0606_0606;
        words[8] = 0x0101_0101_0101_0101;
        words[10] = 0x0101_0101_0101_0101;
        words[13] = 0x0101_0101_0101_0101;
        let visible = visible_body_slots(&words);
        for slot in [1, 8, 9, 10, 11, 14] {
            assert_ne!(visible & slot_bit(slot), 0, "{slot}");
        }
    }

    /// A body part stays merged while any bit of its select mask is
    /// visible: the chest goes only with both shoulders and its partial bit.
    #[test]
    fn the_chest_stays_while_a_shoulder_shows() {
        let drawn = drawn_body_slots(!slot_bit(9));
        assert_ne!(drawn & slot_bit(9), 0);
        let drawn = drawn_body_slots(!(slot_bit(9) | slot_bit(11) | slot_bit(14) | 0x20_0000));
        assert_eq!(drawn & slot_bit(9), 0);
        // The shoulder stays: its own partial bit is still visible.
        assert_ne!(drawn & slot_bit(11), 0);
        assert_ne!(drawn & slot_bit(10), 0);
        assert_eq!(drawn_body_slots(!0), 0x3_FFFF);
    }

    /// A hood (head byte `0x03` on layer 1) under a helm whose head byte is
    /// `0x83` on layer 2. At layer 1 the helm's byte lies above a garment
    /// that holds nothing higher, so it joins the cover and hides the
    /// hood's head; at layer 0 only its `0x80` bit is new (`!0x03 & 0x83`).
    /// The hood holds none of the head's partial mask `0x1C`, so the head's
    /// partial bit goes too, and neither head bit of the select mask stays.
    #[test]
    fn a_helm_over_a_hood_hides_the_hoods_head() {
        let mut occupancy = [0u64; 18];
        occupancy[9] = 0x0300;
        let mut union = [0u64; 18];
        union[9] = 0x0300 | 0x83_0000;
        let pass = garment_pass(&occupancy, &union, slot_bit(10));
        assert_eq!(pass.cover[10], 0x83);
        assert_eq!(pass.words[10], 0x03);
        assert_eq!(pass.uncovered[10], 0);
        // The head and its partial bit go (as does every slot the hood
        // holds nothing of).
        assert_eq!(pass.visible & (slot_bit(10) | 0x1000_0000), 0);
        assert!(!slot_selected(10, pass.visible));
        // Slots the garment has no nodes in keep no word.
        assert_eq!(pass.words[9], 0);
        // A hood node's "03" bits score 16 plus its two matched bits.
        assert_eq!(layer_score(3, pass.cover[10]), 18);
        // Alone, the hood keeps its head.
        let alone = garment_pass(&occupancy, &occupancy, slot_bit(10));
        assert_eq!(alone.cover[10], 0);
        assert!(slot_selected(10, alone.visible));
    }

    /// The shipped object database's armour: a plate helm hides the head,
    /// boots the feet, and the braies and breastplate hide nothing.
    #[test]
    fn the_shipped_armour_hides_what_it_covers() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let resource = game.pack(exa_assets::PackKind::Resource).unwrap();
        let db = exa_formats::rdb::Database::parse(resource.get("objdb.rdb").unwrap()).unwrap();
        let items = exa_formats::objdb::items(&db);
        let by_id = |id: u32| items.iter().find(|i| i.id == id).unwrap();
        let visible = |item: &exa_formats::objdb::Item| {
            let words: [u64; 18] = std::array::from_fn(|i| item.slot_word(i + 1));
            visible_body_slots(&words)
        };
        // 3: Plate Helm; 4: Boots; 8: Braies; 1: Breastplate.
        assert_eq!(visible(by_id(3)) & slot_bit(10), 0);
        assert_eq!(visible(by_id(4)) & slot_bit(4), 0);
        assert_eq!(visible(by_id(4)) & slot_bit(7), 0);
        assert_ne!(visible(by_id(8)) & slot_bit(1), 0);
        assert_ne!(visible(by_id(1)) & slot_bit(9), 0);
        // The helm's coverage word has the hide bit on every layer.
        assert_eq!(by_id(3).slot_word(10), 0x0083_8383_8383_8383);
        // The body pass agrees with the fit's first-byte hidden slots.
        for item in [3, 4, 8, 1] {
            let hidden = !visible(by_id(item)) & 0x3_FFFF;
            assert_eq!(hidden, by_id(item).hidden_slots() >> 1, "{item}");
        }
    }

    fn node(name: &str, parent: Option<usize>, children: Vec<usize>) -> crate::runtime::Node {
        crate::runtime::Node {
            name: name.into(),
            slot: slot_of(name).unwrap_or(0),
            layer: layer_of(name),
            parent,
            children,
            ..crate::runtime::Node::default()
        }
    }

    /// The fallback looks under nodes of the parent slot whose layer is the
    /// wanted one; the first parent-slot node of another layer ends that
    /// tree's walk.
    #[test]
    fn the_related_slot_search_follows_the_parent_slot() {
        use crate::runtime::Body;
        let mut pass = GarmentPass::default();
        pass.cover[17] = 0x03;
        // The root's last child is visited first.
        let body = Body::new(vec![
            node("00Root", None, vec![1, 2]),
            node("1301RFArm", Some(0), vec![]),
            node("1301RFArm", Some(0), vec![3]),
            node("1700RHand", Some(2), vec![]),
        ]);
        let roots = [0];
        let a = Assembler::new(&body, &roots, &pass);
        assert_eq!(a.related_search(17, 1), Some(3));
        assert_eq!(a.related_search(17, 4), None);
        // A mismatching forearm met first stops the search.
        let body = Body::new(vec![
            node("00Root", None, vec![1, 2]),
            node("1301RFArm", Some(0), vec![3]),
            node("1304RFArm", Some(0), vec![]),
            node("1700RHand", Some(1), vec![]),
        ]);
        let a = Assembler::new(&body, &roots, &pass);
        assert_eq!(a.related_search(17, 1), None);
    }

    /// The assembler keeps the chest node whose bits match the cover best
    /// (`06` scores 18 against `0x06`, `02` 17) and prunes its shoulder
    /// into the same group (18 more, 36 in all).
    #[test]
    fn the_assembler_keeps_the_best_group() {
        use crate::runtime::Body;
        let body = Body::new(vec![
            node("00Root", None, vec![1, 2]),
            node("0902Chest", Some(0), vec![]),
            node("0906Chest", Some(0), vec![3]),
            node("1106RShld", Some(2), vec![]),
        ]);
        let mut pass = GarmentPass {
            visible: !0,
            ..GarmentPass::default()
        };
        pass.cover[9] = 0x06;
        pass.cover[11] = 0x06;
        pass.words[9] = 0x06;
        pass.words[11] = 0x06;
        let roots = [0];
        let mut a = Assembler::new(&body, &roots, &pass);
        assert!(a.assemble_slot(9));
        assert_eq!(a.groups.len(), 1);
        assert_eq!(a.groups[0].score, 36);
        assert_eq!(a.best, 36);
        assert_eq!(a.chosen[9], Some(2));
        assert_eq!(a.chosen[11], Some(3));
        // The whole garment: the walk reaches the chest through the belly.
        let chosen = assemble_garment(&body, &roots, &pass);
        assert_eq!(chosen[9], Some(2));
        assert_eq!(chosen[11], Some(3));
        assert_eq!(chosen[1], None);
        // A hidden shoulder is not chosen.
        pass.visible = !(slot_bit(11) | 0x40_0000);
        let chosen = assemble_garment(&body, &roots, &pass);
        assert_eq!(chosen[9], Some(2));
        assert_eq!(chosen[11], None);
    }

    /// Equal scores keep every group; the first group, the first node the
    /// walk meets (the root's last child), is the one chosen.
    #[test]
    fn a_tie_goes_to_the_first_node_met() {
        use crate::runtime::Body;
        let body = Body::new(vec![
            node("00Root", None, vec![1, 2]),
            node("0906Chest", Some(0), vec![]),
            node("0906Chest", Some(0), vec![]),
        ]);
        let mut pass = GarmentPass {
            visible: !0,
            ..GarmentPass::default()
        };
        pass.cover[9] = 0x06;
        let roots = [0];
        let mut a = Assembler::new(&body, &roots, &pass);
        assert!(a.assemble_slot(9));
        assert_eq!(a.groups.len(), 2);
        assert_eq!(a.chosen[9], Some(2));
    }

    /// The cull pass: the second mask's bit overwrites the first's, so a
    /// slot takes `0x400000`, `0x200000` or nothing.
    #[test]
    fn the_cull_bits_follow_the_visible_mask() {
        assert_eq!(cull_bits(1, !0), 0);
        assert_eq!(cull_bits(1, 0), 0x40_0000);
        assert_eq!(cull_bits(2, !0), 0x40_0000);
        assert_eq!(cull_bits(9, 0x100), 0x40_0000);
        assert_eq!(cull_bits(9, 0x20_0000), 0x20_0000);
        assert_eq!(cull_bits(9, 0x20_0100), 0);
        assert_eq!(cull_bits(0, 0), 0);
        assert_eq!(cull_bits(19, 0), 0);
    }

    /// The layer level is the highest layer any occupancy word holds:
    /// braies sit on layer 0, plate on layer 6.
    #[test]
    fn the_layer_level_is_the_garments_highest() {
        let mut braies = [0u64; 20];
        braies[0] = 0x03;
        assert_eq!(layer_level(&braies), 0);
        let mut plate = [0u64; 20];
        plate[8] = 0x06_0000_0000_0000;
        plate[19] = 0x0300;
        assert_eq!(layer_level(&plate), 6);
        assert_eq!(layer_level(&[0; 20]), 0);
    }
}
