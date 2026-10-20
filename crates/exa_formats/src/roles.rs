//! The role database (`charroles.rdb`) and dialogue graphs (role entries,
//! `narrator.rcd`).
//!
//! A role record is a 0x38-byte header (size, flags, name[16], parent,
//! weight, counts), its init script, its variables (name[16] + type word)
//! and self-sized entries. An entry targets a role, the relation partner
//! (0) or one character, carries an attitude and tolerance, and holds
//! optional sections in a fixed order: an unknown block (kind bit `0x1`),
//! a dialogue graph (`0x2`), an encounter script (`0x4`), two unknown blocks
//! (`0x8`, `0x10`), an interaction condition (`0x40`) and a game-event line
//! (`0x20`). `narrator.rcd` is one dialogue graph on its own.
//!
//! A dialogue graph: target word, block size, actor records (name[16],
//! colour, a word, a sized extra block), then nodes. An empty node is a
//! single zero word. A node: size, flags, title[16], editor position,
//! links, then a group of branches; a branch: size, flags, lines
//! {speaker, text}, condition script, action script and a group of reply
//! branches, nested to any depth.
//!
//! Evidence: the role lookup and inheritance walk, the section finder of an
//! entry, the dialogue actor/node walkers, the node-by-title search, the
//! branch walkers of the dialogue engine and the loader of `narrator.rcd`
//! (REA on `Exanima.exe` 0.9.5). Every record of the shipped
//! `charroles.rdb` and the whole of `narrator.rcd` parse to their ends.

use crate::rdb::Database;
use crate::{Error, Result};

/// `charroles.rdb` magic.
pub const MAGIC: u32 = 0xDBCB_0D03;

/// Role flag: a relation role (Ally, Truce, Leader…), meaningful when one
/// character holds it toward another.
pub const RELATION_ROLE: u32 = 0x1;
/// Role flag: adding the role also adds its parent.
pub const INHERITS_PARENT: u32 = 0x1_0000;

/// Entry kind: the attitude in the value word applies.
pub const ATTITUDE: u32 = 0x1000;
/// Entry kind sections, in the order they are stored.
pub const SECTION_EXTRA: u32 = 0x1;
pub const SECTION_DIALOGUE: u32 = 0x2;
pub const SECTION_ENCOUNTER: u32 = 0x4;
pub const SECTION_8: u32 = 0x8;
pub const SECTION_10: u32 = 0x10;
pub const SECTION_INTERACTION: u32 = 0x40;
pub const SECTION_EVENT: u32 = 0x20;

/// Deepest reply nesting accepted (the shipped graphs nest six deep).
const MAX_DEPTH: usize = 64;

/// The role database.
#[derive(Debug, Clone, Default)]
pub struct RoleDb {
    /// Records in table order.
    pub roles: Vec<Role>,
}

impl RoleDb {
    pub fn parse(b: &[u8]) -> Result<Self> {
        let db = Database::parse(b)?;
        if db.magic != MAGIC {
            return Err(Error::BadMagic {
                found: db.magic,
                expected: MAGIC,
            });
        }
        let roles = db
            .records
            .iter()
            .map(|r| Role::parse(r.id, db.payload(r)))
            .collect::<Result<_>>()?;
        Ok(Self { roles })
    }

    /// The role with this id. The game reports an unknown id ("Invalid role
    /// ID") and uses an empty record.
    pub fn get(&self, id: u32) -> Option<&Role> {
        self.roles.iter().find(|r| r.id == id)
    }
}

/// One role record.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Role {
    pub id: u32,
    /// [`RELATION_ROLE`], [`INHERITS_PARENT`]; bit 8 (Corlian only) is not
    /// decoded.
    pub flags: u32,
    pub name: Name,
    pub parent: u32,
    /// Blend weight in percent (low byte used): 0, 50 or 100.
    pub weight: u32,
    /// Run when the role is applied (`Actor.Activity = 5;`).
    pub init_script: Vec<u8>,
    pub variables: Vec<RoleVariable>,
    pub entries: Vec<RoleEntry>,
}

impl Role {
    /// Bit 0, as the dialogue search and the attitude rule test it.
    pub fn is_relation_role(&self) -> bool {
        self.flags & RELATION_ROLE != 0
    }

    /// The whole low byte equal to 1, as topic activation tests it.
    pub fn is_relation_role_byte(&self) -> bool {
        self.flags & 0xFF == RELATION_ROLE
    }

    pub fn inherits_parent(&self) -> bool {
        self.flags & INHERITS_PARENT != 0
    }

    /// The dialogue entry aimed at `target` (a role id, 0 for the relation
    /// partner) with condition word `condition`, if it has a graph.
    pub fn dialogue_for(&self, target: u32, condition: u32) -> Option<&Dialogue> {
        let e = self
            .entries
            .iter()
            .find(|e| e.target == target && e.condition == condition)?;
        e.dialogue.as_ref()
    }

    fn parse(id: u32, b: &[u8]) -> Result<Self> {
        let mut r = Reader { b, pos: 0 };
        let _size = r.u32()?;
        let flags = r.u32()?;
        let name = r.name()?;
        let parent = r.u32()?;
        let _unused = r.u32()?;
        let weight = r.u32()?;
        let nvars = r.u32()? as usize;
        let n_second = r.u32()? as usize;
        let _zero = r.u32()?;
        let nentries = r.u32()? as usize;
        let script_len = r.u32()? as usize;
        let init_script = r.bytes(script_len)?.to_vec();
        let mut variables = Vec::new();
        for _ in 0..nvars {
            variables.push(RoleVariable {
                name: r.name()?,
                kind: r.u32()?,
            });
        }
        // A second 20-byte table, empty in every shipped record.
        r.table(n_second, 20)?;
        let mut entries = Vec::new();
        for _ in 0..nentries {
            let size = r.peek_u32()? as usize;
            entries.push(RoleEntry::parse(r.bytes(size)?)?);
        }
        Ok(Self {
            id,
            flags,
            name,
            parent,
            weight,
            init_script,
            variables,
            entries,
        })
    }
}

/// A role variable: its name and a type word (0, 1, 2 or 4 in the data;
/// meaning not determined).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RoleVariable {
    pub name: Name,
    pub kind: u32,
}

/// A 16-byte, NUL-padded name as stored (a 16-character name has no
/// terminator). Script lookups compare it byte for byte.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Name(pub [u8; 16]);

impl Name {
    /// The stored bytes up to the first NUL.
    pub fn bytes(&self) -> &[u8] {
        let n = self.0.iter().position(|&c| c == 0).unwrap_or(16);
        &self.0[..n]
    }

    /// The name as text (Windows-1252 bytes read as Latin-1).
    pub fn text(&self) -> String {
        latin1(self.bytes())
    }

    /// A name from the first 16 bytes of `s`, NUL-padded.
    pub fn from_bytes(s: &[u8]) -> Self {
        let mut n = [0u8; 16];
        let len = s.len().min(16);
        n[..len].copy_from_slice(&s[..len]);
        Self(n)
    }

    /// Case-blind comparison of all 16 bytes (ASCII letters only), the way
    /// topics are found by name.
    pub fn eq_ignore_case(&self, other: &Name) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl std::fmt::Debug for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.text())
    }
}

/// Windows-1252 text read as Latin-1, enough for the shipped strings.
pub fn latin1(b: &[u8]) -> String {
    b.iter().map(|&c| char::from(c)).collect()
}

/// One entry of a role.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RoleEntry {
    pub kind: u32,
    /// A role id, 0 for the relation partner, or a character id when the
    /// condition word marks the entry as character-specific.
    pub target: u32,
    /// 0 normally; bits 31 and 30 both set: the entry is for one character
    /// in the locale given by the low 30 bits.
    pub condition: u32,
    /// Byte 0: attitude (−100..100); byte 1: tolerance (0..100).
    pub value: u32,
    pub extra: Option<Vec<u8>>,
    pub dialogue: Option<Dialogue>,
    /// Run when the owner meets a matching character.
    pub encounter: Option<Vec<u8>>,
    pub section_8: Option<Vec<u8>>,
    pub section_10: Option<Vec<u8>>,
    /// Gates interaction with the player (`result = …`).
    pub interaction: Option<Vec<u8>>,
    /// A `GameEvent(…)` line (kind `0x20`, one shipped entry; the code that
    /// runs it was not found). Placed after the other sections: a guess, as
    /// no shipped entry combines it with another.
    pub event: Option<Vec<u8>>,
}

impl RoleEntry {
    pub fn attitude(&self) -> i8 {
        self.value as u8 as i8
    }

    pub fn tolerance(&self) -> u8 {
        (self.value >> 8) as u8
    }

    pub fn has_attitude(&self) -> bool {
        self.kind & ATTITUDE != 0
    }

    /// Whether the entry is for one character in one locale.
    pub fn for_character(&self) -> bool {
        self.condition & 0xC000_0000 == 0xC000_0000
    }

    fn parse(b: &[u8]) -> Result<Self> {
        let mut r = Reader { b, pos: 0 };
        let _size = r.u32()?;
        let kind = r.u32()?;
        let target = r.u32()?;
        let condition = r.u32()?;
        let value = r.u32()?;
        let mut e = Self {
            kind,
            target,
            condition,
            value,
            ..Self::default()
        };
        if kind & SECTION_EXTRA != 0 {
            e.extra = Some(r.sized()?.to_vec());
        }
        if kind & SECTION_DIALOGUE != 0 {
            let size = r.b.get(r.pos + 4..r.pos + 8).map_or(0, |s| {
                u32::from_le_bytes(s.try_into().expect("slice of 4")) as usize
            });
            e.dialogue = Some(Dialogue::parse(r.bytes(size)?)?);
        }
        if kind & SECTION_ENCOUNTER != 0 {
            e.encounter = Some(r.sized()?.to_vec());
        }
        if kind & SECTION_8 != 0 {
            e.section_8 = Some(r.sized()?.to_vec());
        }
        if kind & SECTION_10 != 0 {
            e.section_10 = Some(r.sized()?.to_vec());
        }
        if kind & SECTION_INTERACTION != 0 {
            e.interaction = Some(r.sized()?.to_vec());
        }
        if kind & SECTION_EVENT != 0 {
            e.event = Some(r.sized()?.to_vec());
        }
        Ok(e)
    }
}

/// A dialogue graph.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Dialogue {
    /// The target word of the entry the graph belongs to (0 for the
    /// relation partner and in `narrator.rcd`); topic memory is filed under it.
    pub target: u32,
    /// Actor 1 is the owner, actor 2 the partner (`Actor[0]`, `Actor[1]`).
    pub actors: Vec<DialogueActor>,
    /// Nodes in order; `None` is an empty node. Node numbers start at 1.
    pub nodes: Vec<Option<Node>>,
}

impl Dialogue {
    /// A whole graph block (`narrator.rcd`, or the dialogue section of an
    /// entry).
    pub fn parse(b: &[u8]) -> Result<Self> {
        let mut r = Reader { b, pos: 0 };
        let target = r.u32()?;
        let size = r.u32()? as usize;
        if size > b.len() {
            return Err(Error::Malformed(format!(
                "dialogue block of {size} bytes in {}",
                b.len()
            )));
        }
        r.b = &b[..size];
        let nactors = r.u32()? as usize;
        let mut actors = Vec::new();
        for _ in 0..nactors {
            let name = r.name()?;
            let colour = r.u32()?;
            let unknown = r.u32()?;
            let extra = r.sized()?.to_vec();
            actors.push(DialogueActor {
                name,
                colour,
                unknown,
                extra,
            });
        }
        let nnodes = r.u32()? as usize;
        let mut nodes = Vec::new();
        for _ in 0..nnodes {
            let size = r.peek_u32()? as usize;
            if size < 5 {
                r.u32()?;
                nodes.push(None);
            } else {
                nodes.push(Some(Node::parse(r.bytes(size)?)?));
            }
        }
        Ok(Self {
            target,
            actors,
            nodes,
        })
    }

    /// The node numbered `n` (from 1).
    pub fn node(&self, n: usize) -> Option<&Node> {
        n.checked_sub(1)
            .and_then(|i| self.nodes.get(i))
            .and_then(Option::as_ref)
    }

    /// The number (from 1) of the first non-empty node titled `title`,
    /// compared without case.
    pub fn find(&self, title: &Name) -> Option<usize> {
        self.nodes
            .iter()
            .position(|n| n.as_ref().is_some_and(|n| n.title.eq_ignore_case(title)))
            .map(|i| i + 1)
    }
}

/// An actor slot of a dialogue graph.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DialogueActor {
    pub name: Name,
    /// Editor colour (ARGB).
    pub colour: u32,
    /// Zero in every shipped graph.
    pub unknown: u32,
    /// Sized block after the colour, empty in every shipped graph.
    pub extra: Vec<u8>,
}

/// Node flag: an entry node (a greeting the start search may choose).
pub const NODE_ENTRY: u32 = 0x100_0000;
/// Node flag set in the data; the routine that ends a dialogue tests this
/// bit on its cursor word instead of the node, so it never takes effect
/// (its intent, not recording the node as used, is a guess).
pub const NODE_UNRECORDED: u32 = 0x200_0000;
/// Node flag: never chosen by the start search.
pub const NODE_NOT_ENTRY: u32 = 0x800_0000;

/// A dialogue node (a topic or an event).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Node {
    /// Bits 0–3 re-use cooldown class; 4–7 calm class (how low the owner's
    /// arousal must be); 8–12 hits that activate the topic; `0x2000` hits
    /// are a mask of distinct witnesses; `0x4000` an active topic is not
    /// re-activated; 16–19 how long an activation lasts; 20–22 activation
    /// delay; [`NODE_ENTRY`], [`NODE_NOT_ENTRY`]; [`NODE_UNRECORDED`] and
    /// bits 28–31 are set in the data but read by no routine found.
    pub flags: u32,
    pub title: Name,
    /// Editor position.
    pub position: [f32; 2],
    /// Low 16 bits: the node number a branch can continue to; bit 16 set in
    /// the data.
    pub links: Vec<u32>,
    /// The opening group: alternatives tried in order.
    pub branches: Vec<Branch>,
}

impl Node {
    fn parse(b: &[u8]) -> Result<Self> {
        let mut r = Reader { b, pos: 0 };
        let _size = r.u32()?;
        let flags = r.u32()?;
        let title = r.name()?;
        let position = [r.f32()?, r.f32()?];
        let nlinks = r.u32()? as usize;
        let mut links = Vec::new();
        for _ in 0..nlinks {
            links.push(r.u32()?);
        }
        let branches = Branch::group(&mut r, 0)?;
        Ok(Self {
            flags,
            title,
            position,
            links,
            branches,
        })
    }
}

/// One branch: who says what, when, and what follows.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Branch {
    /// Bits 0–2: the link (from 1) followed when the branch ends; bits
    /// 9–10: the highest hostility it is meant for (`0x200` wary, `0x600`
    /// engaged); bits 4–8: attitude bands, stored but not tested; 0 = any
    /// mood.
    pub flags: u32,
    /// Alternative lines; a speaker says one of its own at random.
    pub lines: Vec<Line>,
    /// Must set `result` above 0 for the branch to be taken (empty: always).
    pub condition: Vec<u8>,
    /// Run when the branch is taken.
    pub action: Vec<u8>,
    /// The group that answers this branch.
    pub replies: Vec<Branch>,
}

impl Branch {
    fn group(r: &mut Reader<'_>, depth: usize) -> Result<Vec<Self>> {
        if depth > MAX_DEPTH {
            return Err(Error::Malformed("dialogue branches nest too deep".into()));
        }
        let n = r.u32()? as usize;
        let mut out = Vec::new();
        for _ in 0..n {
            let size = r.peek_u32()? as usize;
            let mut b = Reader {
                b: r.bytes(size)?,
                pos: 0,
            };
            out.push(Self::parse(&mut b, depth)?);
        }
        Ok(out)
    }

    fn parse(r: &mut Reader<'_>, depth: usize) -> Result<Self> {
        let _size = r.u32()?;
        let flags = r.u32()?;
        let nlines = r.u32()? as usize;
        let mut lines = Vec::new();
        for _ in 0..nlines {
            let speaker = r.u32()?;
            let text = r.sized()?.to_vec();
            lines.push(Line { speaker, text });
        }
        let condition = r.sized()?.to_vec();
        let action = r.sized()?.to_vec();
        let replies = Self::group(r, depth + 1)?;
        Ok(Self {
            flags,
            lines,
            condition,
            action,
            replies,
        })
    }

    /// The link (from 1) to follow when this branch ends, 0 for none.
    pub fn link(&self) -> u32 {
        self.flags & 7
    }
}

/// A line of a branch.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Line {
    /// The actor slot (from 1) that says it.
    pub speaker: u32,
    /// Windows-1252 text; `{…}` groups are script expressions.
    pub text: Vec<u8>,
}

struct Reader<'a> {
    b: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        let s = self
            .pos
            .checked_add(n)
            .and_then(|end| self.b.get(self.pos..end))
            .ok_or(Error::Truncated {
                offset: self.pos,
                needed: n,
                len: self.b.len(),
            })?;
        self.pos += n;
        Ok(s)
    }

    fn table(&mut self, n: usize, size: usize) -> Result<&'a [u8]> {
        let len = n
            .checked_mul(size)
            .ok_or_else(|| Error::Malformed(format!("{n} entries of {size} bytes")))?;
        self.bytes(len)
    }

    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(
            self.bytes(4)?.try_into().expect("slice of 4"),
        ))
    }

    fn peek_u32(&self) -> Result<u32> {
        crate::read_u32(self.b, self.pos)
    }

    fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_bits(self.u32()?))
    }

    fn name(&mut self) -> Result<Name> {
        Ok(Name(self.bytes(16)?.try_into().expect("slice of 16")))
    }

    /// A u32 length and that many bytes.
    fn sized(&mut self) -> Result<&'a [u8]> {
        let n = self.u32()? as usize;
        self.bytes(n)
    }
}

#[cfg(test)]
#[path = "roles_test.rs"]
mod tests;
