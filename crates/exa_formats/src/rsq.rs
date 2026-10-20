//! Skeleton and motion sequences (`.rsq`, only `humanoid.rsq` ships).
//!
//! The file is a bare chunk stream (no sanity word): a scene-node list
//! (`0x3D00_0000`) holding the bones, and a motion set (`0x3DAB`).
//!
//! Motion set: `u32 bones, u32 actions, u32 movesets`; per action `name[16]`
//! and two words; per moveset `name[16]`, `parent[16]` (the moveset it
//! inherits missing actions from), then for every action `i32 keys` and, when
//! positive, `i32 mode`, `f32 time[keys]` (frames), `f32 root[keys][3]` (cm),
//! and `keys × bones` poses of `f32 rotation[4]` + `u32 flags`.
//! Layout and evidence: `docs/formats/RSQ.md`.
//!
//! The movesets form a tree: each one's parent is the nearest moveset
//! before it with its parent's name. A character's clip table is assembled
//! from that tree as the original assembles it ([`MotionSet::assemble`]):
//! the base moveset's chain overlaid from the root, the moveset its held
//! items ask for ([`MovesetConfig`]) found under each ancestor, then the
//! overlays ("Torch").

use crate::rfc::Scene;
use crate::{Error, Result};

pub const MOTION_SET: u32 = 0x3DAB;
const CENTIMETRES_TO_METRES: f32 = 0.01;

#[derive(Debug, Clone)]
pub struct Sequences {
    /// Bones as scene nodes; a bone's index is the two-digit prefix of its name.
    pub skeleton: Scene,
    pub motions: MotionSet,
    /// Physical body data per skeleton node (same order as the nodes).
    pub bodies: Vec<BoneBody>,
}

/// How a bone's body part joins its parent and touches the ground: links
/// (point on this part, point on the parent, slack), foot contact points
/// and shoulder pins, in the bone's space (metres).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BoneBody {
    pub links: Vec<BoneLink>,
    pub contacts: Vec<[f32; 3]>,
    pub pins: Vec<[f32; 3]>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoneLink {
    pub flags: u32,
    /// On this bone's part.
    pub on_part: [f32; 3],
    /// On the parent's part, in the parent's space.
    pub on_parent: [f32; 3],
    /// Free distance before the link pulls (0: a pivot).
    pub slack: f32,
    pub stiffness: f32,
    pub smoothing: f32,
}

const BODY_DATA: u32 = 0x3DD0_B100;
const LINKS: u32 = 0xDDB0;
const CONTACTS: u32 = 0xDDBC;
const PINS: u32 = 0xDDBB;
/// Bytes of a scene node's header before its sub-chunks.
const NODE_HEADER: usize = 112;
/// Offset of the nested chunks inside the body data.
const BODY_CHUNKS: usize = 0x5C;

fn read_bodies(list: &[u8]) -> Result<Vec<BoneBody>> {
    let mut r = Reader { b: list, pos: 0 };
    let count = r.u32()? as usize;
    let mut out = Vec::with_capacity(count.min(256));
    for _ in 0..count {
        r.u32()?;
        let size = r.u32()? as usize;
        let node = r.bytes(size)?;
        let mut body = BoneBody::default();
        let mut sub = Reader {
            b: node,
            pos: NODE_HEADER.min(node.len()),
        };
        while sub.b.len() - sub.pos >= 8 {
            let tag = sub.u32()?;
            let len = sub.u32()? as usize;
            let data = sub.bytes(len)?;
            if tag == BODY_DATA {
                read_body_chunks(data.get(BODY_CHUNKS..).unwrap_or(&[]), &mut body)?;
            }
        }
        out.push(body);
    }
    Ok(out)
}

fn read_body_chunks(b: &[u8], body: &mut BoneBody) -> Result<()> {
    let mut r = Reader { b, pos: 0 };
    let cm = |v: f32| v * CENTIMETRES_TO_METRES;
    while r.b.len() - r.pos >= 8 {
        let tag = r.u32()?;
        let len = r.u32()? as usize;
        let data = r.bytes(len)?;
        let mut d = Reader { b: data, pos: 0 };
        match tag {
            LINKS => {
                let n = d.u32()? as usize;
                for _ in 0..n.min(16) {
                    let flags = d.u32()?;
                    let on_part = [d.f32()?, d.f32()?, d.f32()?].map(cm);
                    let on_parent = [d.f32()?, d.f32()?, d.f32()?].map(cm);
                    let slack = cm(d.f32()?);
                    let stiffness = d.f32()?;
                    let smoothing = d.f32()?;
                    if flags & 4 != 0 {
                        d.bytes(12)?; // slider axis
                    }
                    body.links.push(BoneLink {
                        flags,
                        on_part,
                        on_parent,
                        slack,
                        stiffness,
                        smoothing,
                    });
                }
            }
            CONTACTS | PINS => {
                let n = d.u32()? as usize;
                for _ in 0..n.min(16) {
                    d.u32()?;
                    let p = [d.f32()?, d.f32()?, d.f32()?].map(cm);
                    if tag == CONTACTS {
                        body.contacts.push(p);
                    } else {
                        body.pins.push(p);
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Default)]
pub struct MotionSet {
    pub bone_count: usize,
    pub actions: Vec<Action>,
    pub movesets: Vec<Moveset>,
}

#[derive(Debug, Clone)]
pub struct Action {
    pub name: String,
    pub params: [u32; 2],
}

#[derive(Debug, Clone)]
pub struct Moveset {
    pub name: String,
    /// Name of the moveset this one inherits from: the nearest one before
    /// it of that name ([`MotionSet::parent_of`]).
    pub parent: String,
    /// One entry per action; `None` when not defined here.
    pub clips: Vec<Option<Clip>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Clip {
    pub mode: i32,
    pub keys: Vec<Key>,
}

impl Clip {
    /// The clip loops: its mode's bit 0, which the original's clip start
    /// and samplers read (a looping clip wraps by its last key and blends
    /// from it back to its first).
    pub fn loops(&self) -> bool {
        self.mode & 1 != 0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Key {
    /// Frames.
    pub time: f32,
    /// Root (pelvis) position in metres.
    pub root: [f32; 3],
    /// Per bone `[x, y, z, w]`.
    pub rotations: Vec<[f32; 4]>,
    pub flags: Vec<u32>,
}

/// What a character's held items ask of its clip table, as the original
/// encodes it: the weapon moveset in the low 16 bits (one of
/// [`MovesetConfig::WEAPON_MOVESETS`], searched under the base moveset's
/// chain) and overlay bits above ([`MovesetConfig::OVERLAY_MOVESETS`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct MovesetConfig(pub u32);

impl MovesetConfig {
    /// No weapon moveset: the base moveset's chain alone (the fists).
    pub const NONE: Self = Self(0);
    pub const ONE_HANDED: Self = Self(0x1);
    /// A weapon with a light in the off hand.
    pub const ONE_HANDED_TORCH: Self = Self(0x3);
    pub const TWO_WEAPONS: Self = Self(0x4);
    pub const SHIELD: Self = Self(0x100);
    pub const SHIELD_C1: Self = Self(0x200);
    pub const SHIELD_C2: Self = Self(0x300);
    pub const TWO_HANDED: Self = Self(0x10);
    pub const TWO_HANDED_LIGHT: Self = Self(0x20);
    pub const GREATSWORD: Self = Self(0x30);
    pub const POLEARM: Self = Self(0x40);
    /// The "Torch" overlay: a light in the main hand.
    pub const TORCH: Self = Self(0x1001_0000);
    /// The "Flaming" overlay: a weapon set on fire.
    pub const FLAMING: Self = Self(0x1002_0000);

    /// The weapon movesets by the low 16 bits of the config.
    pub const WEAPON_MOVESETS: [(u32, &'static str); 10] = [
        (0x1, "One Handed"),
        (0x3, "1H & Torch"),
        (0x4, "2W Light"),
        (0x100, "1H & Shield"),
        (0x200, "Shield C1"),
        (0x300, "Shield C2"),
        (0x10, "Two Handed"),
        (0x20, "2H Light"),
        (0x30, "Greatsword"),
        (0x40, "Polearm"),
    ];

    /// The overlay movesets, applied in this order when the config shares
    /// any bit with the mask. The two masks share their top bit, so either
    /// overlay bit asks for both.
    pub const OVERLAY_MOVESETS: [(u32, &'static str); 2] =
        [(0x1001_0000, "Torch"), (0x1002_0000, "Flaming")];

    /// Both configs' bits.
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// The name of the weapon moveset the low 16 bits ask for, if any.
    pub fn weapon_moveset(self) -> Option<&'static str> {
        let low = self.0 & 0xFFFF;
        if low == 0 {
            return None;
        }
        Self::WEAPON_MOVESETS
            .iter()
            .find(|(code, _)| *code == low)
            .map(|(_, name)| *name)
    }

    /// The names of the overlay movesets the config asks for, in order.
    pub fn overlays(self) -> impl Iterator<Item = &'static str> {
        Self::OVERLAY_MOVESETS
            .into_iter()
            .filter(move |(mask, _)| self.0 & mask != 0)
            .map(|(_, name)| name)
    }
}

impl std::fmt::Display for MovesetConfig {
    /// The weapon moveset and the overlays (`One Handed + Torch`), or
    /// `no weapon`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut names: Vec<&str> = self.weapon_moveset().into_iter().collect();
        names.extend(self.overlays());
        if names.is_empty() {
            write!(f, "no weapon")
        } else {
            write!(f, "{}", names.join(" + "))
        }
    }
}

/// A character's clip table: for each action (by index), the index of the
/// moveset whose clip it plays, or `None` when no moveset of the table
/// defines the action.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClipTable(pub Vec<Option<usize>>);

impl ClipTable {
    /// Every clip `moveset` defines replaces the table's.
    fn overlay(&mut self, set: &MotionSet, moveset: usize) {
        let Some(m) = set.movesets.get(moveset) else {
            return;
        };
        for (slot, clip) in self.0.iter_mut().zip(&m.clips) {
            if clip.is_some() {
                *slot = Some(moveset);
            }
        }
    }
}

/// The moveset tree, resolved once per assembly.
struct Tree {
    parents: Vec<Option<usize>>,
    children: Vec<Vec<usize>>,
}

impl Tree {
    /// Each moveset's parent ([`MotionSet::parent_of`]) and children, in
    /// file order, in one pass: the latest moveset seen of a name is the
    /// nearest before the next one asking for it.
    fn of(set: &MotionSet) -> Self {
        let mut latest: std::collections::HashMap<&str, usize> = Default::default();
        let mut parents = Vec::with_capacity(set.movesets.len());
        let mut children = vec![Vec::new(); set.movesets.len()];
        for (i, m) in set.movesets.iter().enumerate() {
            let parent = latest.get(m.parent.as_str()).copied();
            let parent = parent.filter(|_| !m.parent.is_empty());
            if let Some(p) = parent {
                children[p].push(i);
            }
            parents.push(parent);
            latest.insert(m.name.as_str(), i);
        }
        Self { parents, children }
    }

    /// The first child of `node` named `name`.
    fn named_child(&self, set: &MotionSet, node: usize, name: &str) -> Option<usize> {
        self.children[node]
            .iter()
            .copied()
            .find(|&c| set.movesets[c].name == name)
    }

    /// `from` and its ancestors, up to (without) `stop`, or to the root.
    fn up(&self, from: usize, stop: Option<usize>) -> Vec<usize> {
        let mut path = Vec::new();
        let mut node = Some(from);
        while let Some(n) = node {
            if Some(n) == stop || path.len() > self.parents.len() {
                break;
            }
            path.push(n);
            node = self.parents[n];
        }
        path
    }

    /// The first descendant of `node` named `name`: its children first, in
    /// order, then each child's descendants the same way, child by child
    /// (depth first, with a stack of its own rather than recursion).
    fn find_below(&self, set: &MotionSet, node: usize, name: &str) -> Option<usize> {
        if let Some(hit) = self.named_child(set, node, name) {
            return Some(hit);
        }
        // Each frame: a node whose children were checked, and the next of
        // them to descend into.
        let mut stack = vec![(node, 0)];
        while let Some((n, next)) = stack.last_mut() {
            let Some(&child) = self.children[*n].get(*next) else {
                stack.pop();
                continue;
            };
            *next += 1;
            if let Some(hit) = self.named_child(set, child, name) {
                return Some(hit);
            }
            stack.push((child, 0));
        }
        None
    }
}

impl MotionSet {
    pub fn action_index(&self, name: &str) -> Option<usize> {
        self.actions
            .iter()
            .position(|a| a.name.eq_ignore_ascii_case(name))
    }

    /// The index of the first moveset named `name`, ignoring ASCII case, as
    /// the original finds a character's base moveset.
    pub fn moveset_index(&self, name: &str) -> Option<usize> {
        self.movesets
            .iter()
            .position(|m| m.name.eq_ignore_ascii_case(name))
    }

    /// The first moveset named `name` ([`MotionSet::moveset_index`]).
    pub fn moveset(&self, name: &str) -> Option<&Moveset> {
        self.movesets.get(self.moveset_index(name)?)
    }

    /// The moveset `index` inherits from: the nearest moveset before it
    /// whose name is exactly its parent's name. `None` for a root: no parent
    /// name, or none before it of that name.
    pub fn parent_of(&self, index: usize) -> Option<usize> {
        let m = self.movesets.get(index)?;
        if m.parent.is_empty() {
            return None;
        }
        self.movesets[..index]
            .iter()
            .rposition(|p| p.name == m.parent)
    }

    /// The clip table of a character whose base moveset (its race's own) is
    /// `base` and whose held items ask for `config`, as the original
    /// assembles it:
    /// 1. the first moveset named `base` and its ancestors, overlaid from
    ///    the root down;
    /// 2. the weapon moveset, searched under each ancestor from the root
    ///    down to the base; each one found is overlaid with its own
    ///    ancestors below that ancestor;
    /// 3. each overlay, searched among the children of the last weapon
    ///    moveset found and of its ancestors below the ancestor it was found
    ///    under, then of the earlier ones; the first found is overlaid.
    ///
    /// `None` when no moveset is named `base`.
    pub fn assemble(&self, base: &str, config: MovesetConfig) -> Option<ClipTable> {
        let base = self.moveset_index(base)?;
        let tree = Tree::of(self);
        let chain = tree.up(base, None);
        let mut table = ClipTable(vec![None; self.actions.len()]);
        for &m in chain.iter().rev() {
            table.overlay(self, m);
        }
        // Each weapon moveset found, with the ancestor it was found under.
        let mut found = Vec::new();
        if let Some(weapon) = config.weapon_moveset() {
            for &ancestor in chain.iter().rev() {
                if let Some(w) = tree.find_below(self, ancestor, weapon) {
                    for &m in tree.up(w, Some(ancestor)).iter().rev() {
                        table.overlay(self, m);
                    }
                    found.push((w, ancestor));
                }
            }
        }
        for overlay in config.overlays() {
            let hit = found.iter().rev().find_map(|&(w, ancestor)| {
                tree.up(w, Some(ancestor))
                    .into_iter()
                    .find_map(|n| tree.named_child(self, n, overlay))
            });
            if let Some(m) = hit {
                table.overlay(self, m);
            }
        }
        Some(table)
    }

    /// The clip `table` plays for `action`.
    pub fn table_clip(&self, table: &ClipTable, action: &str) -> Option<&Clip> {
        let a = self.action_index(action)?;
        let m = (*table.0.get(a)?)?;
        self.movesets.get(m)?.clips.get(a)?.as_ref()
    }

    /// The clip a character with base moveset `base` and held-item config
    /// `config` plays for `action` ([`MotionSet::assemble`], not cached).
    pub fn clip(&self, base: &str, config: MovesetConfig, action: &str) -> Option<&Clip> {
        self.table_clip(&self.assemble(base, config)?, action)
    }
}

impl Sequences {
    pub fn parse(b: &[u8]) -> Result<Self> {
        let mut skeleton = None;
        let mut motions = None;
        let mut bodies = Vec::new();
        let mut pos = 0usize;
        while pos + 8 <= b.len() {
            let tag = u32::from_le_bytes(b[pos..pos + 4].try_into().expect("4 bytes"));
            let size =
                u32::from_le_bytes(b[pos + 4..pos + 8].try_into().expect("4 bytes")) as usize;
            let data = b.get(pos + 8..pos + 8 + size).ok_or(Error::Truncated {
                offset: pos + 8,
                needed: size,
                len: b.len(),
            })?;
            match tag {
                crate::rfc::NODE_LIST => {
                    // Reuse the chunk-tree reader by framing the node list as a file.
                    let mut framed = vec![0xCF, 0xAF, 0x23, 0x3D];
                    framed.extend_from_slice(&b[pos..pos + 8 + size]);
                    skeleton = Some(Scene::parse(&framed)?);
                    bodies = read_bodies(data)?;
                }
                MOTION_SET => motions = Some(read_motions(data)?),
                _ => {}
            }
            pos += 8 + size;
        }
        Ok(Self {
            skeleton: skeleton.ok_or_else(|| Error::Malformed("no skeleton".into()))?,
            motions: motions.ok_or_else(|| Error::Malformed("no motion set".into()))?,
            bodies,
        })
    }
}

/// Bone index encoded in the first two characters of a bone or body-part name.
pub fn bone_index(name: &str) -> Option<usize> {
    let b = name.as_bytes();
    if b.len() >= 2 && b[0].is_ascii_digit() && b[1].is_ascii_digit() {
        Some(((b[0] - b'0') * 10 + (b[1] - b'0')) as usize)
    } else {
        None
    }
}

fn read_motions(b: &[u8]) -> Result<MotionSet> {
    let mut r = Reader { b, pos: 0 };
    let bone_count = r.u32()? as usize;
    let action_count = r.u32()? as usize;
    let moveset_count = r.u32()? as usize;
    r.fits(action_count, 24)?;
    let mut actions = Vec::with_capacity(action_count);
    for _ in 0..action_count {
        let name = r.name()?;
        actions.push(Action {
            name,
            params: [r.u32()?, r.u32()?],
        });
    }
    r.fits(moveset_count, 32)?;
    let mut movesets = Vec::with_capacity(moveset_count);
    for _ in 0..moveset_count {
        let name = r.name()?;
        let parent = r.name()?;
        let mut clips = Vec::with_capacity(action_count);
        for _ in 0..action_count {
            let n = r.i32()?;
            if n < 1 {
                clips.push(None);
                continue;
            }
            let n = n as usize;
            let mode = r.i32()?;
            r.fits(n, 16 + bone_count * 20)?;
            let times: Vec<f32> = (0..n).map(|_| r.f32()).collect::<Result<_>>()?;
            let roots: Vec<[f32; 3]> = (0..n)
                .map(|_| Ok([r.f32()?, r.f32()?, r.f32()?].map(|v| v * CENTIMETRES_TO_METRES)))
                .collect::<Result<_>>()?;
            let mut keys = Vec::with_capacity(n);
            for (time, root) in times.into_iter().zip(roots) {
                let mut rotations = Vec::with_capacity(bone_count);
                let mut flags = Vec::with_capacity(bone_count);
                for _ in 0..bone_count {
                    rotations.push([r.f32()?, r.f32()?, r.f32()?, r.f32()?]);
                    flags.push(r.u32()?);
                }
                keys.push(Key {
                    time,
                    root,
                    rotations,
                    flags,
                });
            }
            clips.push(Some(Clip { mode, keys }));
        }
        movesets.push(Moveset {
            name,
            parent,
            clips,
        });
    }
    Ok(MotionSet {
        bone_count,
        actions,
        movesets,
    })
}

struct Reader<'a> {
    b: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn bytes(&mut self, n: usize) -> Result<&[u8]> {
        let s = self.b.get(self.pos..self.pos + n).ok_or(Error::Truncated {
            offset: self.pos,
            needed: n,
            len: self.b.len(),
        })?;
        self.pos += n;
        Ok(s)
    }

    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(
            self.bytes(4)?.try_into().expect("4 bytes"),
        ))
    }

    fn i32(&mut self) -> Result<i32> {
        Ok(self.u32()? as i32)
    }

    fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_bits(self.u32()?))
    }

    fn name(&mut self) -> Result<String> {
        let raw = self.bytes(16)?;
        let end = raw.iter().position(|&c| c == 0).unwrap_or(16);
        Ok(raw[..end].iter().map(|&c| c as char).collect())
    }

    fn fits(&self, n: usize, item: usize) -> Result<()> {
        match n.checked_mul(item) {
            Some(bytes) if bytes <= self.b.len() - self.pos => Ok(()),
            _ => Err(Error::Malformed(format!("{n} records do not fit"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(s: &str) -> Vec<u8> {
        let mut n = [0u8; 16];
        n[..s.len()].copy_from_slice(s.as_bytes());
        n.to_vec()
    }

    fn le<T: Copy>(v: &[T], f: impl Fn(T) -> [u8; 4]) -> Vec<u8> {
        v.iter().flat_map(|&x| f(x)).collect()
    }

    #[test]
    fn motion_set_with_inheritance() {
        let mut m = le(&[1u32, 2, 2], u32::to_le_bytes); // 1 bone, 2 actions, 2 movesets
        for a in ["Walk", "Idle"] {
            m.extend(name(a));
            m.extend(le(&[0u32, 0], u32::to_le_bytes));
        }
        // Default: Walk with 1 key, Idle none.
        m.extend(name("Default"));
        m.extend(name(""));
        m.extend(le(&[1i32, 3], i32::to_le_bytes));
        m.extend(le(
            &[10.0f32, 0.0, 100.0, 0.0, 0.0, 0.0, 0.0, 1.0],
            f32::to_le_bytes,
        ));
        m.extend(le(&[7u32], u32::to_le_bytes));
        m.extend(le(&[0i32], i32::to_le_bytes));
        // Child: nothing of its own.
        m.extend(name("Child"));
        m.extend(name("Default"));
        m.extend(le(&[0i32, 0], i32::to_le_bytes));
        let set = read_motions(&m).unwrap();
        let none = MovesetConfig::NONE;
        let clip = set.clip("Child", none, "walk").unwrap();
        assert_eq!(clip.keys[0].root, [0.0, 1.0, 0.0]);
        assert_eq!(clip.keys[0].rotations[0], [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(clip.keys[0].flags[0], 7);
        assert!(set.clip("Child", none, "Idle").is_none());
    }

    /// A motion set of `actions` and `(name, parent, defined actions)`
    /// movesets, each clip one key long.
    fn tree(actions: &[&str], movesets: &[(&str, &str, &[usize])]) -> MotionSet {
        let key = Key {
            time: 0.0,
            root: [0.0; 3],
            rotations: vec![[0.0, 0.0, 0.0, 1.0]],
            flags: vec![0],
        };
        MotionSet {
            bone_count: 1,
            actions: actions
                .iter()
                .map(|a| Action {
                    name: (*a).into(),
                    params: [0, 0],
                })
                .collect(),
            movesets: movesets
                .iter()
                .map(|(name, parent, defined)| Moveset {
                    name: (*name).into(),
                    parent: (*parent).into(),
                    clips: (0..actions.len())
                        .map(|a| {
                            defined.contains(&a).then(|| Clip {
                                mode: 3,
                                keys: vec![key.clone()],
                            })
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    fn table(set: &MotionSet, base: &str, config: MovesetConfig) -> Vec<Option<usize>> {
        set.assemble(base, config).expect("base moveset").0
    }

    #[test]
    fn parents_are_the_nearest_earlier_movesets_of_their_name() {
        let set = tree(
            &["Walk"],
            &[
                ("Default", "", &[0]),
                ("One Handed", "Default", &[]),
                ("Golem", "Default", &[]),
                ("One Handed", "Golem", &[]),
                ("1H & Shield", "One Handed", &[]),
                // Its own name: nothing before it is named so.
                ("Loop", "Loop", &[]),
                // Names match exactly, case included.
                ("Lower", "default", &[]),
                // Its own name again: the earlier one.
                ("Loop", "Loop", &[]),
            ],
        );
        let parents: Vec<_> = (0..8).map(|i| set.parent_of(i)).collect();
        assert_eq!(
            parents,
            [
                None,
                Some(0),
                Some(0),
                Some(2),
                Some(3),
                None,
                None,
                Some(5)
            ]
        );
        assert_eq!(set.parent_of(8), None);
        // The assembly's one-pass tree links them the same way.
        let tree = Tree::of(&set);
        assert_eq!(tree.parents, parents);
        assert_eq!(tree.children[0], [1, 2]);
        assert_eq!(tree.children[2], [3]);
    }

    #[test]
    fn the_base_chain_is_overlaid_from_the_root() {
        let set = tree(
            &["A", "B", "C"],
            &[
                ("Default", "", &[0, 1, 2]),
                ("Mid", "Default", &[1]),
                ("Leaf", "Mid", &[2]),
            ],
        );
        let none = MovesetConfig::NONE;
        assert_eq!(table(&set, "Leaf", none), [Some(0), Some(1), Some(2)]);
        // The base is found ignoring case.
        assert_eq!(table(&set, "MID", none), [Some(0), Some(1), Some(0)]);
        assert_eq!(set.assemble("Nope", none), None);
        // A moveset with no clip for an action leaves it unplayed.
        let bare = tree(&["A", "B"], &[("Default", "", &[0])]);
        assert_eq!(table(&bare, "Default", none), [Some(0), None]);
        assert!(bare.clip("Default", none, "B").is_none());
    }

    #[test]
    fn the_weapon_moveset_is_found_under_every_ancestor() {
        let set = tree(
            &["A", "B", "C"],
            &[
                ("Default", "", &[0, 1, 2]),
                ("One Handed", "Default", &[0]),
                ("Golem", "Default", &[1]),
                ("One Handed", "Golem", &[2]),
            ],
        );
        let one = MovesetConfig::ONE_HANDED;
        // The root's weapon moveset, then the golem's over it.
        assert_eq!(table(&set, "Golem", one), [Some(1), Some(2), Some(3)]);
        // A human never reaches the golem's.
        assert_eq!(table(&set, "Default", one), [Some(1), Some(0), Some(0)]);
        // A weapon moveset nobody has, or low bits naming none, add nothing.
        assert_eq!(
            table(&set, "Golem", MovesetConfig::POLEARM),
            [Some(0), Some(2), Some(0)]
        );
        assert_eq!(
            table(&set, "Golem", MovesetConfig(0x2)),
            [Some(0), Some(2), Some(0)]
        );
    }

    #[test]
    fn children_are_searched_before_deeper_descendants() {
        let movesets: &[(&str, &str, &[usize])] = &[
            ("Default", "", &[0, 1]),
            ("Two Handed", "Default", &[]),
            ("One Handed", "Two Handed", &[0]),
            ("One Handed", "Default", &[1]),
        ];
        let one = MovesetConfig::ONE_HANDED;
        // The child of the root wins over the grandchild before it.
        assert_eq!(
            table(&tree(&["A", "B"], movesets), "Default", one),
            [Some(0), Some(3)]
        );
        // Without it, the grandchild comes with the path down to it.
        assert_eq!(
            table(&tree(&["A", "B"], &movesets[..3]), "Default", one),
            [Some(2), Some(0)]
        );
    }

    #[test]
    fn overlays_are_found_from_the_weapon_moveset_up() {
        let set = tree(
            &["A", "B", "C", "D"],
            &[
                ("Default", "", &[0, 1, 2, 3]),
                ("One Handed", "Default", &[0]),
                ("1H & Shield", "One Handed", &[1]),
                ("Torch", "One Handed", &[2]),
                // Under the ancestor the shield was found under: not seen.
                ("Torch", "Default", &[3]),
            ],
        );
        let torch = MovesetConfig::TORCH;
        assert_eq!(
            table(&set, "Default", MovesetConfig::SHIELD.with(torch)),
            [Some(1), Some(2), Some(3), Some(0)]
        );
        assert_eq!(
            table(&set, "Default", MovesetConfig::ONE_HANDED.with(torch)),
            [Some(1), Some(0), Some(3), Some(0)]
        );
        // The two overlay masks share their top bit: a flaming weapon gets
        // the torch overlay too.
        assert_eq!(
            table(
                &set,
                "Default",
                MovesetConfig::ONE_HANDED.with(MovesetConfig::FLAMING)
            ),
            [Some(1), Some(0), Some(3), Some(0)]
        );
        // No weapon moveset: no overlay.
        assert_eq!(table(&set, "Default", torch), [Some(0); 4]);
        // Only the root's own "Torch": not found.
        let root_torch = tree(
            &["A", "B"],
            &[
                ("Default", "", &[0, 1]),
                ("One Handed", "Default", &[0]),
                ("Torch", "Default", &[1]),
            ],
        );
        assert_eq!(
            table(
                &root_torch,
                "Default",
                MovesetConfig::ONE_HANDED.with(torch)
            ),
            [Some(1), Some(0)]
        );
    }

    #[test]
    fn configs_name_their_movesets() {
        use MovesetConfig as C;
        assert_eq!(C::ONE_HANDED.weapon_moveset(), Some("One Handed"));
        assert_eq!(C::ONE_HANDED_TORCH.weapon_moveset(), Some("1H & Torch"));
        assert_eq!(C::GREATSWORD.weapon_moveset(), Some("Greatsword"));
        assert_eq!(C(0x1001_0001).weapon_moveset(), Some("One Handed"));
        assert_eq!(C(0x2).weapon_moveset(), None);
        assert_eq!(C::NONE.weapon_moveset(), None);
        let overlays = |config: MovesetConfig| config.overlays().collect::<Vec<_>>();
        assert_eq!(overlays(C::TORCH), ["Torch", "Flaming"]);
        assert_eq!(overlays(C::FLAMING), ["Torch", "Flaming"]);
        assert_eq!(overlays(C(0x1_0000)), ["Torch"]);
        assert_eq!(overlays(C(0x2_0000)), ["Flaming"]);
        assert!(overlays(C::POLEARM).is_empty());
        assert_eq!(C::NONE.to_string(), "no weapon");
        assert_eq!(C::POLEARM.to_string(), "Polearm");
        assert_eq!(
            C::ONE_HANDED.with(C::TORCH).to_string(),
            "One Handed + Torch + Flaming"
        );
    }

    /// Exarust's lookup before the tree: the first moveset of a name, its
    /// parents by first name match.
    fn name_chain(set: &MotionSet, moveset: &str) -> Vec<Option<usize>> {
        let Some(start) = set.moveset_index(moveset) else {
            return vec![None; set.actions.len()];
        };
        (0..set.actions.len())
            .map(|a| {
                let mut m = start;
                for _ in 0..set.movesets.len() {
                    if set.movesets[m].clips[a].is_some() {
                        return Some(m);
                    }
                    let parent = &set.movesets[m].parent;
                    m = set.moveset_index(parent).filter(|_| !parent.is_empty())?;
                }
                None
            })
            .collect()
    }

    /// Every race's clip table for every held-item config, assembled from
    /// the installed `humanoid.rsq`, against the name chain Exarust looked
    /// clips up in (skipped without the game).
    #[test]
    fn shipped_movesets_assemble_by_the_tree() {
        let Some(dir) = exa_rpk::locate_install(None) else {
            return;
        };
        let game = exa_rpk::GameData::open(&dir).unwrap();
        let pack = game.pack(exa_rpk::PackKind::Resource).unwrap();
        let set = Sequences::parse(pack.get("humanoid.rsq").unwrap())
            .unwrap()
            .motions;
        let parents: Vec<_> = (0..set.movesets.len()).map(|i| set.parent_of(i)).collect();
        let p = Some;
        assert_eq!(
            parents,
            [
                None,
                p(0),
                p(1),
                p(2),
                p(2),
                p(1),
                p(1),
                p(1),
                p(1),
                p(0),
                p(9),
                p(10),
                p(10),
                p(0),
                p(0),
                p(14), // the golem's One Handed
                p(15), // its 1H & Shield, under it and not the human's
                p(14),
                p(14),
                p(18),
                p(0),
                p(20),
                p(20),
                p(22),
                p(23), // the demon's One Handed
                p(20),
                p(0),
                p(0),
                p(27), // the ogre's One Handed
                p(28),
                p(27),
                p(0)
            ]
        );
        assert_eq!(Tree::of(&set).parents, parents);
        // The races' own movesets, as the original passes them.
        let races = crate::races::parse(pack.get("races.rdb").unwrap()).unwrap();
        let mut bases: Vec<&str> = races.iter().map(|r| r.moveset()).collect();
        bases.sort_unstable();
        bases.dedup();
        assert_eq!(
            bases,
            [
                "Default",
                "Demon A01",
                "Fael",
                "Gobbler",
                "Golem",
                "Harvester",
                "Herath",
                "Herath Ogre",
                "Ogre"
            ]
        );
        use MovesetConfig as C;
        let held = [
            C::NONE,
            C::ONE_HANDED,
            C::ONE_HANDED_TORCH,
            C::TWO_WEAPONS,
            C::SHIELD,
            C::SHIELD_C1,
            C::SHIELD_C2,
            C::TWO_HANDED,
            C::TWO_HANDED_LIGHT,
            C::GREATSWORD,
            C::POLEARM,
            C::ONE_HANDED.with(C::TORCH),
            C::SHIELD.with(C::TORCH),
            C::SHIELD_C1.with(C::TORCH),
            C::SHIELD_C2.with(C::TORCH),
        ];
        let flaming = held
            .iter()
            .filter(|h| h.0 & C::TORCH.0 == 0)
            .map(|h| h.with(C::FLAMING));
        let configs: Vec<_> = held.iter().copied().chain(flaming).collect();
        assert_eq!(configs.len(), 26);
        // The moveset Exarust named for each config: the weapon's, the
        // "Torch" moveset for a main-hand light, else the race's own.
        fn named(base: &str, config: MovesetConfig) -> &str {
            if config.0 & 0xFFFF == 1 && config.0 & C::TORCH.0 == C::TORCH.0 {
                "Torch"
            } else {
                config.weapon_moveset().unwrap_or(base)
            }
        }
        let mut differing = 0;
        let mut human = Vec::new();
        for &base in &bases {
            for &config in &configs {
                let t = table(&set, base, config);
                // "Default" defines every action: every table plays them all.
                assert!(t.iter().all(Option::is_some), "{base} {config}");
                let old = name_chain(&set, named(base, config));
                let changed = (0..t.len()).filter(|&a| t[a] != old[a]).count();
                if changed > 0 {
                    differing += 1;
                    if base == "Default" {
                        human.push((config.0, changed));
                    }
                }
            }
        }
        assert_eq!(differing, 162);
        // A human differs only where the torch overlay now lies over a
        // shield or a flaming weapon: the four "Torch" clips.
        assert_eq!(
            human,
            [
                (0x1001_0100, 4),
                (0x1001_0200, 4),
                (0x1001_0300, 4),
                (0x1002_0001, 4),
                (0x1002_0003, 4),
                (0x1002_0004, 4),
                (0x1002_0100, 4),
                (0x1002_0200, 4),
                (0x1002_0300, 4)
            ]
        );
        let torch = set.moveset_index("Torch").unwrap();
        let shield_torch = table(&set, "Default", C::SHIELD.with(C::TORCH));
        for action in ["Walk", "Cmbt - Idle", "Attack Mid", "Idle"] {
            assert_eq!(shield_torch[set.action_index(action).unwrap()], Some(torch));
        }
        // The movesets each creature's table draws from.
        let used = |base: &str, config: MovesetConfig| {
            let mut m: Vec<usize> = table(&set, base, config).into_iter().flatten().collect();
            m.sort_unstable();
            m.dedup();
            m
        };
        assert_eq!(used("Default", C::NONE), [0]);
        assert_eq!(used("Default", C::GREATSWORD), [0, 9, 10, 12]);
        assert_eq!(used("Golem", C::ONE_HANDED), [0, 1, 14, 15]);
        assert_eq!(used("Golem", C::SHIELD), [0, 2, 15, 16]);
        assert_eq!(used("Golem", C::POLEARM), [0, 13, 17]);
        assert_eq!(used("Ogre", C::ONE_HANDED), [0, 1, 27, 28]);
        // Any Herath creature's one-handed weapon finds the demon's One
        // Handed below the Fael, which comes with it.
        assert_eq!(used("Herath", C::ONE_HANDED), [0, 1, 22, 23, 24]);
        assert_eq!(used("Gobbler", C::ONE_HANDED), [0, 1, 22, 23, 24]);
        assert_eq!(used("Harvester", C::ONE_HANDED), [0, 1, 22, 23, 24, 25]);
        // The original keeps at most four movesets in a base's chain (the
        // base and three ancestors).
        let tree = Tree::of(&set);
        let deepest = (0..set.movesets.len())
            .filter(|&i| set.moveset_index(&set.movesets[i].name) == Some(i))
            .map(|i| tree.up(i, None).len())
            .max();
        assert_eq!(deepest, Some(4));
    }

    #[test]
    fn bone_indices_from_names() {
        assert_eq!(bone_index("01Pelvis"), Some(1));
        assert_eq!(bone_index("17RHand"), Some(17));
        assert_eq!(bone_index("Omni"), None);
    }
}
