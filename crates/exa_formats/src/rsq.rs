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

use crate::rfc::Scene;
use crate::{Error, Result};

pub const MOTION_SET: u32 = 0x3DAB;
const CENTIMETRES_TO_METRES: f32 = 0.01;

#[derive(Debug, Clone)]
pub struct Sequences {
    /// Bones as scene nodes; a bone's index is the two-digit prefix of its name.
    pub skeleton: Scene,
    pub motions: MotionSet,
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
    /// Moveset to fall back to for actions this one does not define.
    pub parent: String,
    /// One entry per action; `None` when not defined here.
    pub clips: Vec<Option<Clip>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Clip {
    pub mode: i32,
    pub keys: Vec<Key>,
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

impl MotionSet {
    pub fn action_index(&self, name: &str) -> Option<usize> {
        self.actions
            .iter()
            .position(|a| a.name.eq_ignore_ascii_case(name))
    }

    pub fn moveset(&self, name: &str) -> Option<&Moveset> {
        self.movesets
            .iter()
            .find(|m| m.name.eq_ignore_ascii_case(name))
    }

    /// The clip for `action` in `moveset`, following parents for missing
    /// actions.
    pub fn clip(&self, moveset: &str, action: &str) -> Option<&Clip> {
        let a = self.action_index(action)?;
        let mut current = self.moveset(moveset)?;
        for _ in 0..self.movesets.len() {
            if let Some(Some(c)) = current.clips.get(a) {
                return Some(c);
            }
            if current.parent.is_empty() || current.parent.eq_ignore_ascii_case(&current.name) {
                return None;
            }
            current = self.moveset(&current.parent)?;
        }
        None
    }
}

impl Sequences {
    pub fn parse(b: &[u8]) -> Result<Self> {
        let mut skeleton = None;
        let mut motions = None;
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
                }
                MOTION_SET => motions = Some(read_motions(data)?),
                _ => {}
            }
            pos += 8 + size;
        }
        Ok(Self {
            skeleton: skeleton.ok_or_else(|| Error::Malformed("no skeleton".into()))?,
            motions: motions.ok_or_else(|| Error::Malformed("no motion set".into()))?,
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
        let clip = set.clip("Child", "walk").unwrap();
        assert_eq!(clip.keys[0].root, [0.0, 1.0, 0.0]);
        assert_eq!(clip.keys[0].rotations[0], [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(clip.keys[0].flags[0], 7);
        assert!(set.clip("Child", "Idle").is_none());
    }

    #[test]
    fn bone_indices_from_names() {
        assert_eq!(bone_index("01Pelvis"), Some(1));
        assert_eq!(bone_index("17RHand"), Some(17));
        assert_eq!(bone_index("Omni"), None);
    }
}
