//! Where a striking or struck part finds its weapon stat record.
//!
//! The original keeps a weapon's stat record on one part: a held item's top
//! part (the part the holding pass builds it on), or a hand that the unarmed
//! set-up made a fist. Any other part asks up its parent chain. The fetch
//! takes the part's own record first; while there is none it steps to the
//! parent, and it stops with nothing once it reaches the tree's root (the
//! part that is its own parent), whose record it never reads. So a child
//! body of a held item finds the record of the item's top part, which hangs
//! from the holder's hand and is not a root; a limb that is no fist walks up
//! to the pelvis and finds nothing.
//!
//! Three readers ask it: the damage routine for the striking part (with no
//! record it reports that it cannot retrieve the weapon's values and deals
//! nothing), the clash's struck-item sound for the struck part (whose own
//! frame and sound clock it then uses with the found record's split and
//! sound class), and the grip-link set-up for the second hand's place.
//!
//! Exarust keeps a held item's record on its [`crate::Blade`], the item's
//! top part (part 0 of the item's body), and a fist's on the Blade its hand
//! carries. [`HeldChain`] gives the walk its tree: the item's parts by their
//! parent indices, the top part hanging from the holder's hand, the hand
//! from the holder's root.
//!
//! Evidence: the record fetch, its three callers (the damage routine's
//! "Unable to retrieve weapon values." exit, the struck-item sound and the
//! grip-link set-up), read in the instructions of `Exanima.exe`.

/// One node of a held item's chain as the record fetch walks it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainNode {
    /// A part of the item's own body, by its index.
    Item(usize),
    /// The holder's hand the item's top part hangs from.
    Hand,
    /// The holder's root (its pelvis), its own parent.
    Root,
}

/// The fetch's walk over any part tree: `start`'s own record when it has
/// one, else the first parent's up the chain that has one, stopping with
/// `None` at the root (a node that is its own parent), whose record is
/// never read. `parent` gives each node's parent and `has_record` whether
/// a node carries a record.
pub fn record_holder<N: Copy + PartialEq>(
    start: N,
    parent: impl Fn(N) -> N,
    has_record: impl Fn(N) -> bool,
) -> Option<N> {
    if has_record(start) {
        return Some(start);
    }
    let mut at = start;
    loop {
        at = parent(at);
        if parent(at) == at {
            return None;
        }
        if has_record(at) {
            return Some(at);
        }
    }
}

/// A held item's chain: its parts' parent indices (`None` for the top
/// part), and whether the holder's hand carries a fist record.
pub struct HeldChain<'a> {
    pub parents: &'a [Option<usize>],
    pub hand_record: bool,
}

impl HeldChain<'_> {
    /// The node whose record the item's part `part` reads: the top part's
    /// (Exarust's item Blade) for every part of a held item's body. `None`
    /// for a part index past the body.
    pub fn record_of(&self, part: usize) -> Option<ChainNode> {
        if part >= self.parents.len() {
            return None;
        }
        record_holder(
            ChainNode::Item(part),
            |n| match n {
                ChainNode::Item(i) => match self.parents.get(i).copied().flatten() {
                    Some(p) if p < self.parents.len() => ChainNode::Item(p),
                    _ => ChainNode::Hand,
                },
                ChainNode::Hand | ChainNode::Root => ChainNode::Root,
            },
            |n| match n {
                ChainNode::Item(i) => i == 0,
                ChainNode::Hand => self.hand_record,
                ChainNode::Root => false,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A chain of indices 0 → 1 → 2 → 3, node 0 the root (its own parent),
    /// records on the listed nodes.
    fn walk(start: usize, records: &[usize]) -> Option<usize> {
        record_holder(start, |n| n.saturating_sub(1), |n| records.contains(&n))
    }

    #[test]
    fn a_part_with_a_record_finds_its_own() {
        assert_eq!(walk(3, &[3, 2]), Some(3));
    }

    #[test]
    fn a_part_without_one_finds_the_nearest_parents() {
        assert_eq!(walk(3, &[1, 2]), Some(2));
        assert_eq!(walk(3, &[1]), Some(1));
    }

    #[test]
    fn the_roots_record_is_never_read_from_below() {
        assert_eq!(walk(3, &[0]), None);
        // A root asked directly reads its own.
        assert_eq!(walk(0, &[0]), Some(0));
        assert_eq!(walk(0, &[]), None);
    }

    #[test]
    fn every_part_of_a_held_item_reads_the_top_parts_record() {
        let parents = [None, Some(0), Some(1), Some(0)];
        let chain = HeldChain {
            parents: &parents,
            hand_record: true,
        };
        for part in 0..4 {
            assert_eq!(chain.record_of(part), Some(ChainNode::Item(0)));
        }
        assert_eq!(chain.record_of(4), None);
    }
}
