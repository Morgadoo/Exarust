//! Topic memory: one word per dialogue node, kept in a character's relation
//! record about the partner, and the rules that read and update it.
//!
//! A word is 0 (never used), a world time in seconds (last used), bit 31 +
//! a time (activated at that time), or bit 30 + a count or witness mask
//! (activation in progress). A node's flags pick the rules: bits 0–3 the
//! re-use cooldown, bits 8–12 the hits that activate it, `0x2000` hits as a
//! witness mask, `0x4000` no re-activation while active, bits 16–19 how long
//! an activation lasts, bits 20–22 the delay before it opens.
//!
//! Memory is filed under (relation role entry, key): a role-to-role
//! dialogue uses (the aimed-at role, the owner's role), a relation role's
//! dialogue (the relation role, the aimed-at role). A conversation's words
//! are a count word (the dialogue's node count) and one word per node, node
//! `n` at slot `n`; a stored block keeps the node words only. Loading pads
//! a shorter block with zeros; when the stored count differs from the
//! dialogue's, the stored count word itself lands in slot 1 and the stored
//! words follow it, as in the original.
//!
//! Evidence: the topic gate, the activation updater, the memory load and
//! store of the run-time dialogue, the cooldown table (16 words), the
//! activation-delay table (8 words) and the world clock (REA on
//! `Exanima.exe` 0.9.5).

use crate::cast::{Character, Memory, RelationRole};

/// Re-use cooldown by class (node flag bits 0–3; bits 16–19 pick the
/// activation window from the same table), in seconds: never, a year, 30
/// days, a week, 3 days, a day, 3 h, 1 h, 30 min, 10 min, 5 min, 1 min,
/// 30 s, 10 s, 0, 0.
pub const COOLDOWN: [u32; 16] = [
    0xFFFF_FFFF,
    31_536_000,
    2_592_000,
    604_800,
    259_200,
    86_400,
    10_800,
    3_600,
    1_800,
    600,
    300,
    60,
    30,
    10,
    0,
    0,
];

/// Delay before an activated topic opens (node flag bits 20–22), seconds.
pub const ACTIVATION_DELAY: [u32; 8] = [0, 2, 5, 10, 30, 300, 1800, 0];

pub const ACTIVATED: u32 = 0x8000_0000;
pub const COUNTING: u32 = 0x4000_0000;
const TIME: u32 = 0x3FFF_FFFF;

/// World time in seconds: the saved base plus the seconds the clock ran
/// since it was set.
pub fn world_time(base: u32, now_ms: u32, set_ms: u32) -> u32 {
    let elapsed = (i64::from(now_ms) - i64::from(set_ms)) / 1000;
    (i64::from(base) + elapsed) as u32
}

/// Whether a node may be offered given its memory word.
pub fn available(word: u32, flags: u32, now: u32) -> bool {
    if flags & 0x1F00 != 0 {
        // An activated topic: open only inside its window.
        if word & ACTIVATED == 0 {
            return false;
        }
        let at = word & TIME;
        if now < at {
            return false;
        }
        return u64::from(now)
            < u64::from(at) + u64::from(COOLDOWN[((flags >> 16) & 0xF) as usize]);
    }
    if word != 0 {
        let class = flags & 0xF;
        if class == 0 {
            return false;
        }
        if class < 15 {
            if now < word {
                return false;
            }
            if now.wrapping_sub(word) < COOLDOWN[class as usize] {
                return false;
            }
        }
    }
    true
}

/// One activation hit on a node's word (`mode` is the witness bit for
/// mask-counted topics); whether the word changed.
pub fn activate(word: &mut u32, flags: u32, mode: u32, now: u32) -> bool {
    let delay = ACTIVATION_DELAY[((flags >> 20) & 7) as usize];
    if *word & ACTIVATED != 0 {
        if flags & 0x4000 != 0 {
            return false;
        }
        let at = *word & TIME;
        if now < at {
            return true;
        }
        if u64::from(at) + u64::from(COOLDOWN[((flags >> 16) & 0xF) as usize]) <= u64::from(now) {
            *word = now.wrapping_add(delay) | ACTIVATED;
        } else {
            *word = now | ACTIVATED;
        }
        return true;
    }
    if *word != 0 && *word & (ACTIVATED | COUNTING) == 0 {
        // A used topic: it can count again once its cooldown is over.
        let class = flags & 0xF;
        if class == 0 {
            return false;
        }
        if now < *word {
            return false;
        }
        if i64::from(now) - i64::from(*word) < i64::from(COOLDOWN[class as usize]) {
            return false;
        }
        *word = 0;
    }
    let needed = (flags & 0x1F00) >> 8;
    if flags & 0x2000 == 0 {
        *word = word.wrapping_add(1);
        if *word & TIME < needed {
            *word |= COUNTING;
        } else {
            *word = now.wrapping_add(delay) | ACTIVATED;
        }
    } else {
        *word |= mode;
        if witnesses(*word) < needed as i32 {
            *word |= COUNTING;
        } else {
            *word = now.wrapping_add(delay) | ACTIVATED;
        }
    }
    true
}

/// Set bits among the low 30.
pub fn witnesses(word: u32) -> i32 {
    (word & TIME).count_ones() as i32
}

/// `TopicUsed`: the stored word, 0 while activated or counting.
pub fn used_time(word: u32) -> u32 {
    if word & (ACTIVATED | COUNTING) != 0 {
        0
    } else {
        word
    }
}

/// Where a dialogue's memory is filed in the owner's relation record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryKey {
    /// The other character's id.
    pub other: u32,
    /// The relation-role entry that holds the block.
    pub role: u32,
    /// The block's key.
    pub key: u32,
}

/// A new conversation's words for `count` nodes: the count word and one
/// zero per node.
pub fn fresh(count: usize) -> Vec<u32> {
    let mut words = vec![0; count + 1];
    if let Some(first) = words.first_mut() {
        *first = count as u32;
    }
    words
}

/// The words of a dialogue with `count` nodes, as the run-time dialogue
/// loads them: slot 1 is node 1. `Some` when the block exists (otherwise
/// the caller keeps the words it has, zeros for a new one), and whether
/// the holding relation-role entry exists.
pub fn load(owner: &Character, at: MemoryKey, count: usize) -> (Option<Vec<u32>>, bool) {
    let Some(entry) = owner.relation(at.other).and_then(|r| r.role(at.role)) else {
        return (None, false);
    };
    let Some(m) = entry.memories.iter().find(|m| m.key == at.key) else {
        return (None, true);
    };
    let mut words = fresh(count);
    let stored = m.words.as_deref().unwrap_or(&[]);
    let saved = stored.first().copied().unwrap_or(0) as usize;
    if saved == count {
        let n = (count + 1).min(stored.len());
        words[..n].copy_from_slice(&stored[..n]);
    } else if saved < count {
        // The stored count lands in slot 1, the stored node words follow
        // (one short: the last is dropped) and the rest stays zero.
        let n = saved.min(stored.len());
        words[1..1 + n].copy_from_slice(&stored[..n]);
    } else {
        // More stored nodes than the dialogue has: the stored count and
        // its first `count − 1` node words are copied.
        let n = count.min(stored.len());
        words[1..1 + n].copy_from_slice(&stored[..n]);
    }
    (Some(words), true)
}

/// Store the node words, creating the relation record, the entry and the
/// block as needed.
pub fn store(owner: &mut Character, at: MemoryKey, words: &[u32]) {
    let rel = owner.relation_entry(at.other);
    let entry = match rel.roles.iter().position(|r| r.role == at.role) {
        Some(i) => &mut rel.roles[i],
        None => {
            rel.roles.push(RelationRole {
                role: at.role,
                memories: Vec::new(),
            });
            rel.roles.last_mut().expect("just pushed")
        }
    };
    match entry.memories.iter_mut().find(|m| m.key == at.key) {
        Some(m) => m.words = Some(words.to_vec()),
        None => entry.memories.push(Memory {
            key: at.key,
            words: Some(words.to_vec()),
        }),
    }
}

/// The memory block stored for (`role`, `key`), as the dialogue search
/// passes it to a new conversation.
pub fn block(owner: &Character, other: u32, role: u32, key: u32) -> Option<&Memory> {
    owner
        .relation(other)?
        .role(role)?
        .memories
        .iter()
        .find(|m| m.key == key)
}

/// A new conversation's words from a stored block: the node words are
/// copied into slots 1.., padded with zeros or cut to `count` (no shift on
/// this path).
pub fn from_block(block: Option<&Memory>, count: usize) -> Vec<u32> {
    let mut words = fresh(count);
    if let Some(stored) = block.and_then(|b| b.words.as_deref()) {
        // Slot 0 of the stored block is its count; the nodes follow.
        for (w, &s) in words[1..].iter_mut().zip(stored.get(1..).unwrap_or(&[])) {
            *w = s;
        }
    }
    words
}
