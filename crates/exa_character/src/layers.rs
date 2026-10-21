//! Apparel layering: the original's eighteen body slots, the layer a body or
//! garment node belongs to, and which body parts a set of worn garments
//! covers.
//!
//! The character builder keeps one stack of candidate nodes per body slot.
//! A node's name begins with its slot's two digits and, for apparel, the two
//! digits of its layer (`0900Chest`: chest, layer 0). The builder ranks the
//! candidates of a garment against the garment's own per-slot coverage word
//! and keeps the outermost node of each slot; a slot whose body part a
//! garment hides is no longer merged from the body model. Evidence: the
//! character builder's slot table and selection/coverage passes; the slot
//! names are those of the shipped `humanbase.rfc`.
//!
//! The tables below are the original's data, written in Exarust's terms. The
//! per-slot masks of [`HIDDEN_SLOT_MASK`] and [`CHILD_SLOT_MASK`] are the
//! sub-part bits a garment's coverage must touch before the slot (or the
//! slots the mask names) is hidden; the bits themselves are not decoded.

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

/// Per-slot child coverage masks: when none of a garment's sub-part bits at
/// the slot meets this mask, the slots of [`CHILD_SLOT_MASK`] are hidden too.
pub const HIDDEN_SLOT_MASK: [u32; 18] = [
    0x2, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x2, 0x1, 0x1C, 0x2, 0x2, 0x2, 0x2, 0x2, 0x2, 0x0, 0x0,
];

/// Per-slot masks that make a garment hide the body part of its own slot:
/// when none of its sub-part bits meets this mask, the slot is hidden.
pub const VISIBLE_SLOT_MASK: [u32; 18] = [
    0x1, 0x7, 0x7, 0x7, 0x7, 0x7, 0x7, 0x1, 0x6, 0x3, 0x1, 0x1, 0x1, 0x1, 0x1, 0x1, 0x3, 0x3,
];

/// The extra slots [`HIDDEN_SLOT_MASK`] hides when it matches: a mask in the
/// same sub-part space as the per-slot coverage words.
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

/// The cull bits a slot's merged triangles take from the body's visible
/// mask: none when both masks meet it, `0x200000`/`0x400000` for each that
/// does not.
pub fn cull_bits(slot: usize, visible: u32) -> u32 {
    let Some(i) = slot.checked_sub(1).filter(|&i| i < 18) else {
        return 0;
    };
    let mut bits = 0;
    if CULL_FIRST[i] & visible == 0 {
        bits |= 0x20_0000;
    }
    if CULL_SECOND[i] & visible == 0 {
        bits |= 0x40_0000;
    }
    bits
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

/// The slots a worn set covers, as the builder's coverage pass sets its
/// hidden mask.
///
/// `words[slot - 1]` is the union over the worn items of the slot's eight
/// coverage bytes (`0x80` hides the whole body part, `0x40` its partial
/// variant; the low bits name sub-parts). Walking the layers from the
/// outermost in, a layer that hides the part hides the slot unless a higher
/// layer already touched the slot's mask; a partial layer hides the slots of
/// [`CHILD_SLOT_MASK`] under the same test. The pass then repeats both tests
/// with the union of all layers. The caller inverts the result for the
/// visible mask.
pub fn hidden_body_slots(words: &[u64; 18]) -> u32 {
    let mut hidden = 0u32;
    for (i, &word) in words.iter().enumerate() {
        let slot = i + 1;
        let mut higher = 0u64;
        for layer in (0..8).rev() {
            let byte = (word >> (8 * layer)) & 0xFF;
            if byte & 0x80 != 0 && VISIBLE_SLOT_MASK[i] as u64 & higher == 0 {
                hidden |= slot_bit(slot);
            }
            if byte & 0x40 != 0 && HIDDEN_SLOT_MASK[i] as u64 & higher == 0 {
                hidden |= CHILD_SLOT_MASK[i];
            }
            higher |= byte;
        }
        let union = union_bytes(word);
        if VISIBLE_SLOT_MASK[i] as u64 & union == 0 {
            hidden |= slot_bit(slot);
        }
        if HIDDEN_SLOT_MASK[i] as u64 & union == 0 {
            hidden |= CHILD_SLOT_MASK[i];
        }
    }
    hidden
}

/// The slots the builder keeps when it merges the body model: the inverse
/// of the union pass over the worn items' coverage words. Only the `0x80`
/// and `0x40` bits of the union matter here.
pub fn visible_body_slots(words: &[u64; 18]) -> u32 {
    let mut hidden = 0u32;
    for (i, &word) in words.iter().enumerate() {
        let union = union_bytes(word);
        if union & 0x80 != 0 {
            hidden |= slot_bit(i + 1);
        }
        if union & 0x40 != 0 {
            hidden |= CHILD_SLOT_MASK[i];
        }
    }
    !hidden
}

/// All eight layer bytes of `word` or-ed together.
fn union_bytes(word: u64) -> u64 {
    (0..8).fold(0u64, |all, i| all | ((word >> (8 * i)) & 0xFF))
}

/// One node offered to the layer selection: its name's slot and layer and
/// the garment (or body) it belongs to. `index` is the caller's handle on
/// the node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub index: usize,
    pub slot: usize,
    pub layer: u32,
    /// The node belongs to the body model (source 0) or to a garment.
    pub source: usize,
}

/// The node kept in each slot by [`select`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    /// The chosen node's `index`, by slot 1..18 (index 0 unused).
    pub chosen: [Option<usize>; 19],
    /// The slots the body model still draws: [`visible_body_slots`] of the
    /// garments' union words.
    pub visible: u32,
}

/// Picks the node each slot shows: for every slot, the candidate of the
/// garment that covers most of the slot's coverage word, ties going to the
/// later candidate (the builder appends equal-scoring candidates and keeps
/// the last). `coverage[source][slot - 1]` is that garment's coverage word;
/// the body is source 0 and its words are zero.
///
/// This is the observable result of the builder's slot stack: candidates
/// that do not belong to a garment with a node in the slot are left to the
/// slot's related part, as the builder's fallback pass does. Exarust does
/// not keep the builder's group structs, so the pass is written over the
/// candidates directly.
pub fn select(candidates: &[Candidate], coverage: &[[u64; 18]]) -> Selection {
    let mut chosen: [Option<(i32, usize)>; 19] = [None; 19];
    for c in candidates {
        if !(1..=18).contains(&c.slot) {
            continue;
        }
        let word = coverage.get(c.source).map_or(0, |w| w[c.slot - 1]);
        let score = layer_score(c.layer, word);
        match chosen[c.slot] {
            Some((best, _)) if score < best => {}
            _ => chosen[c.slot] = Some((score, c.index)),
        }
    }
    let mut out = Selection::default();
    for (slot, entry) in chosen.iter().enumerate().skip(1) {
        out.chosen[slot] = entry.map(|(_, index)| index);
    }
    // The body's own slots stay visible unless a garment hides them.
    let mut words = [0u64; 18];
    for w in coverage.iter().skip(1) {
        for (slot, word) in words.iter_mut().zip(w) {
            *slot |= word;
        }
    }
    out.visible = visible_body_slots(&words);
    out
}

/// The outermost of a garment's layers: the largest layer number any of its
/// coverage words holds. The builder sorts worn garments by it when it
/// inserts them, so the outermost is merged last.
pub fn outer_layer(coverage: &[u64; 18]) -> Option<u8> {
    coverage.iter().filter_map(|&w| word_layer(w)).max()
}

/// A candidate group kept by the assembler: one node per slot and the
/// group's total score.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SlotGroup {
    pub nodes: [Option<usize>; 19],
    pub score: i32,
}

/// The fallback search: when a garment has no node of its
/// own in `slot`, look under the related slot for a child in `slot` whose
/// layer differs from `wanted`, scoring it against the slot's coverage.
pub fn related_search(
    body: &crate::runtime::Body,
    slot: usize,
    wanted: u32,
    coverage: &[u64; 19],
) -> Option<usize> {
    if !(1..=SLOTS.len()).contains(&slot) {
        return None;
    }
    let related = SLOTS[slot - 1].parent;
    let mut best = (-64, None);
    for root in body.roots() {
        let mut stack = vec![root];
        while let Some(i) = stack.pop() {
            let n = &body.nodes[i];
            if n.slot == related {
                if n.layer != wanted {
                    for &c in &n.children {
                        if body.nodes[c].slot == slot {
                            let score = layer_score(body.nodes[c].layer, coverage[slot]);
                            if score > best.0 {
                                best = (score, Some(c));
                            }
                        }
                    }
                }
            } else {
                stack.extend(n.children.iter().rev());
            }
        }
    }
    best.1
}

/// Assembles one slot's candidate groups: every node whose
/// slot is `target` is scored against the slot's coverage; the best score
/// clears the groups and equal scores add one. When there is a group, the
/// child slots' groups are pruned and the group's nodes fill the caller's
/// chosen slots where the visibility table lets them.
pub fn assemble_slot(
    body: &crate::runtime::Body,
    roots: &[usize],
    target: usize,
    coverage: &[u64; 19],
    visibility: &[u32; 18],
    groups: &mut Vec<SlotGroup>,
    chosen: &mut [Option<usize>; 19],
) -> bool {
    if !(1..=SLOTS.len()).contains(&target) {
        return false;
    }
    groups.clear();
    let mut best = i32::MIN;
    for &root in roots {
        let mut stack = vec![root];
        while let Some(i) = stack.pop() {
            let n = &body.nodes[i];
            if n.slot == target {
                let score = layer_score(n.layer, coverage[target]);
                if score >= best {
                    if score > best {
                        groups.clear();
                        best = score;
                    }
                    let mut group = SlotGroup::default();
                    group.nodes[target] = Some(i);
                    group.score = score;
                    groups.push(group);
                }
            } else if n.mesh || !n.children.is_empty() {
                for &c in n.children.iter().rev() {
                    stack.push(c);
                }
                if stack.len() > 32 {
                    // The original logs "Body part stack overflow" and keeps
                    // going; stop this branch instead.
                    stack.clear();
                    break;
                }
            }
        }
    }
    if groups.is_empty() {
        return false;
    }
    for &child in SLOTS[target - 1].children {
        if groups.iter().any(|g| g.nodes[child].is_some()) {
            prune_slot(body, child, coverage, groups);
        }
    }
    let bit = slot_bit(target);
    for slot in 1..=18 {
        if groups.iter().any(|g| g.nodes[slot].is_some()) && visibility[slot - 1] & bit != 0 {
            chosen[slot] = groups.last().and_then(|g| g.nodes[slot]);
        }
    }
    true
}

/// Prunes a slot's groups: each group's node in `slot` is
/// looked under for a better child, else the related-slot fallback supplies
/// one; the group's score grows by it. Groups below the best total are
/// dropped, groups with no node in the slot go once one has a node, and the
/// child slots are pruned in turn.
pub fn prune_slot(
    body: &crate::runtime::Body,
    slot: usize,
    coverage: &[u64; 19],
    groups: &mut Vec<SlotGroup>,
) {
    if !(1..=SLOTS.len()).contains(&slot) || groups.is_empty() {
        return;
    }
    let mut best = i32::MIN;
    for group in groups.iter_mut() {
        let Some(node) = group.nodes[slot] else {
            continue;
        };
        let mut found = (i32::MIN, None);
        for &c in &body.nodes[node].children {
            if body.nodes[c].slot == slot {
                let score = layer_score(body.nodes[c].layer, coverage[slot]);
                if score > found.0 {
                    found = (score, Some(c));
                }
            }
        }
        let (score, node) = match found.1 {
            Some(c) => (found.0, Some(c)),
            None => {
                let wanted = body.nodes[node].layer;
                match related_search(body, slot, wanted, coverage) {
                    Some(c) => (layer_score(body.nodes[c].layer, coverage[slot]), Some(c)),
                    None => (0, None),
                }
            }
        };
        if let Some(c) = node {
            group.nodes[slot] = Some(c);
            group.score += score;
            best = best.max(group.score);
        }
    }
    groups.retain(|g| g.score >= best);
    if groups.iter().any(|g| g.nodes[slot].is_some()) {
        groups.retain(|g| g.nodes[slot].is_some());
    }
    for &child in SLOTS[slot - 1].children {
        if groups.iter().any(|g| g.nodes[child].is_some()) {
            prune_slot(body, child, coverage, groups);
        }
    }
}

/// Assembles the whole body from slot 1, the pelvis, and returns the chosen
/// node per slot.
pub fn assemble_all(
    body: &crate::runtime::Body,
    roots: &[usize],
    coverage: &[u64; 19],
    visibility: &[u32; 18],
) -> Selection {
    let mut groups = Vec::new();
    let mut chosen = [None; 19];
    assemble_slot(
        body,
        roots,
        1,
        coverage,
        visibility,
        &mut groups,
        &mut chosen,
    );
    let mut words = [0u64; 18];
    words.copy_from_slice(&coverage[1..19]);
    let mut out = Selection { chosen, visible: 0 };
    out.visible = visible_body_slots(&words);
    out
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
        let hidden = hidden_body_slots(&words);
        assert_ne!(hidden & slot_bit(10), 0);
        let visible = visible_body_slots(&words);
        assert_eq!(visible & slot_bit(10), 0);
        assert_ne!(visible & slot_bit(9), 0);
    }

    /// Boots cover the shins' and feet's sub-parts on layers 0 to 2; their
    /// hide bits (`0x80`) on the foot words hide the feet and the propagated
    /// child slots. Left and right are independent.
    #[test]
    fn boots_hide_the_feet() {
        let mut words = [0u64; 18];
        words[2] = 0x0006_0606;
        words[3] = 0x0083_8383;
        words[5] = 0x0006_0606;
        words[6] = 0x0083_8383;
        let hidden = hidden_body_slots(&words);
        assert_ne!(hidden & slot_bit(4), 0);
        assert_ne!(hidden & slot_bit(7), 0);
        let visible = visible_body_slots(&words);
        assert_eq!(visible & slot_bit(4), 0);
        assert_eq!(visible & slot_bit(7), 0);
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

    /// The selection keeps the node whose garment layer matches its slot's
    /// coverage best; an equal score keeps the later node.
    #[test]
    fn the_selection_keeps_the_best_matching_layer() {
        let candidates = [
            Candidate {
                index: 1,
                slot: 9,
                layer: 0,
                source: 1,
            },
            Candidate {
                index: 2,
                slot: 9,
                layer: 6,
                source: 2,
            },
            Candidate {
                index: 3,
                slot: 9,
                layer: 2,
                source: 3,
            },
        ];
        let mut coverage = [[0u64; 18]; 4];
        coverage[1][8] = 0x06;
        coverage[2][8] = 0x06;
        coverage[3][8] = 0x06;
        let selection = select(&candidates, &coverage);
        assert_eq!(selection.chosen[9], Some(2));
        assert_eq!(selection.chosen[1], None);
        // A tie goes to the later candidate.
        let tie = [
            Candidate {
                index: 4,
                slot: 9,
                layer: 6,
                source: 1,
            },
            Candidate {
                index: 5,
                slot: 9,
                layer: 6,
                source: 2,
            },
        ];
        assert_eq!(select(&tie, &coverage).chosen[9], Some(5));
    }

    /// The body's own slot nodes are not hidden by a garment that covers
    /// them with the low (non-hide) bits only.
    #[test]
    fn covering_is_not_hiding() {
        let mut words = [0u64; 18];
        words[8] = 0x0000_0000_0000_0003;
        let hidden = hidden_body_slots(&words);
        assert_eq!(hidden & slot_bit(9), 0);
        assert_ne!(hidden, !0);
    }

    /// The installed object database's armour: a plate helm hides the head,
    /// boots the feet, and the braies hide nothing.
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
    }

    /// A child under its related slot supplies a garment node when the
    /// garment has none for the slot itself.
    #[test]
    fn the_related_slot_search_finds_a_child() {
        use crate::runtime::{Body, Node};
        let mut nodes = vec![
            Node {
                name: "13RFArm".into(),
                slot: 13,
                layer: 1,
                ..Node::default()
            },
            Node {
                name: "1700RHand".into(),
                slot: 17,
                layer: 0,
                parent: Some(0),
                ..Node::default()
            },
        ];
        nodes[0].children = vec![1];
        let body = Body::new(nodes);
        let mut coverage = [0u64; 19];
        coverage[17] = 0x03;
        assert_eq!(related_search(&body, 17, 4, &coverage), Some(1));
        // The search wants a layer other than the parent's.
        assert_eq!(related_search(&body, 17, 1, &coverage), None);
    }

    /// The assembler picks the higher-scoring of two chest nodes and fills
    /// the chosen slot from it.
    #[test]
    fn the_assembler_keeps_the_best_group() {
        use crate::runtime::{Body, Node};
        let mut nodes = vec![
            Node {
                name: "00Root".into(),
                ..Node::default()
            },
            Node {
                name: "0902Chest".into(),
                slot: 9,
                layer: 2,
                parent: Some(0),
                ..Node::default()
            },
            Node {
                name: "0906Chest".into(),
                slot: 9,
                layer: 6,
                parent: Some(0),
                ..Node::default()
            },
        ];
        nodes[0].children = vec![1, 2];
        let body = Body::new(nodes);
        let mut coverage = [0u64; 19];
        coverage[9] = 0x06;
        let mut groups = Vec::new();
        let mut chosen = [None; 19];
        let visibility = [!0u32; 18];
        assert!(assemble_slot(
            &body,
            &[0],
            9,
            &coverage,
            &visibility,
            &mut groups,
            &mut chosen
        ));
        assert_eq!(groups.len(), 1);
        assert_eq!(chosen[9], Some(2));
    }

    /// The cull pass gives a slot no bits when its masks meet the visible
    /// body mask, and one bit for each mask that does not.
    #[test]
    fn the_cull_bits_follow_the_visible_mask() {
        assert_eq!(cull_bits(1, !0), 0);
        assert_eq!(cull_bits(1, 0), 0x60_0000);
        assert_eq!(cull_bits(2, 0), 0x60_0000);
        assert_eq!(cull_bits(9, 0x100), 0x40_0000);
        assert_eq!(cull_bits(0, 0), 0);
        assert_eq!(cull_bits(19, 0), 0);
    }

    /// The outermost garment layer is the largest any slot holds: braies
    /// are layer 1, a mail shirt 4, plate 7.
    #[test]
    fn the_outer_layer_is_the_garments_highest() {
        let mut braies = [0u64; 18];
        braies[0] = 0x03;
        assert_eq!(outer_layer(&braies), Some(0));
        let mut plate = [0u64; 18];
        plate[8] = 0x06_0000_0000_0000;
        assert_eq!(outer_layer(&plate), Some(6));
        assert_eq!(outer_layer(&[0; 18]), None);
    }
}
