//! Keyframe tracks of scene nodes: the sub-chunk `0x3AC1` that may follow
//! any stored node's body and gives the node a curve of positions the
//! original plays on its translation (a Catmull-Rom curve through keys).
//!
//! Payload: a word the original skips, `u32 mode`, `f32 duration` (s),
//! `u32 last` (the last key's index), then `last + 1` keys of `f32 time`
//! (s) and `f32[3]` position (cm, the node's translation in its parent's
//! space). Bytes after the keys are ignored. Layout and evidence:
//! `docs/formats/RFC.md` (Keyframe tracks).
//!
//! Positions are returned in metres; the file stores centimetres.

use crate::{Error, Result};

const CENTIMETRES_TO_METRES: f32 = 0.01;
/// Bytes before the keys: the skipped word, mode, duration, last index.
const HEADER: usize = 16;
/// Bytes of one key: time and position.
const KEY_LEN: usize = 16;

/// How a track's clock turns the time since it started into a time on its
/// curve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlayMode {
    /// The time as it is; the track never ends (any word but 1, 2 and 3).
    Free,
    /// Forward once (word 1): clamped at the duration, where it ends.
    Once,
    /// Backward once (word 2): the duration minus the time, ending at 0.
    Reverse,
    /// Forward again and again (word 3): the time modulo the duration.
    Loop,
}

impl PlayMode {
    /// The mode a stored word names. The original's clock leaves the time
    /// as it is for every word but 1, 2 and 3.
    pub fn from_word(word: u32) -> Self {
        match word {
            1 => Self::Once,
            2 => Self::Reverse,
            3 => Self::Loop,
            _ => Self::Free,
        }
    }
}

/// One key: a time on the curve and the position there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PositionKey {
    /// Seconds.
    pub time: f32,
    /// The node's translation (metres, in its parent's space).
    pub position: [f32; 3],
}

/// A node's position track as stored.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyTrack {
    pub mode: PlayMode,
    /// Seconds: where `Once` stops, `Reverse` starts and `Loop` wraps.
    pub duration: f32,
    /// At least one key, in the file's order (times rising in the data).
    pub keys: Vec<PositionKey>,
}

impl KeyTrack {
    /// Decodes a `0x3AC1` payload.
    pub fn parse(b: &[u8]) -> Result<Self> {
        let word = |o: usize| -> Result<u32> { crate::read_u32(b, o) };
        let mode = PlayMode::from_word(word(4)?);
        let duration = f32::from_bits(word(8)?);
        let count = (word(12)? as usize)
            .checked_add(1)
            .ok_or_else(|| Error::Malformed("key count overflow".into()))?;
        let needed = count
            .checked_mul(KEY_LEN)
            .and_then(|n| n.checked_add(HEADER))
            .ok_or_else(|| Error::Malformed("key count overflow".into()))?;
        if b.len() < needed {
            return Err(Error::Truncated {
                offset: HEADER,
                needed: needed - HEADER,
                len: b.len(),
            });
        }
        let f = |o: usize| f32::from_le_bytes(b[o..o + 4].try_into().expect("4 bytes"));
        let keys = (0..count)
            .map(|i| {
                let at = HEADER + i * KEY_LEN;
                PositionKey {
                    time: f(at),
                    position: [f(at + 4), f(at + 8), f(at + 12)].map(|v| v * CENTIMETRES_TO_METRES),
                }
            })
            .collect();
        Ok(Self {
            mode,
            duration,
            keys,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload(mode: u32, duration: f32, keys: &[(f32, [f32; 3])]) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend(0x3AE2u32.to_le_bytes());
        b.extend(mode.to_le_bytes());
        b.extend(duration.to_le_bytes());
        b.extend((keys.len() as u32 - 1).to_le_bytes());
        for (t, p) in keys {
            b.extend(t.to_le_bytes());
            for v in p {
                b.extend(v.to_le_bytes());
            }
        }
        b
    }

    /// The shipped track's shape: once, 3 s, four keys pushing 5 cm along
    /// −z and back, in metres.
    #[test]
    fn decodes_keys_in_metres() {
        let b = payload(
            1,
            3.0,
            &[
                (0.0, [0.0, 0.0, -0.0]),
                (1.0, [0.0, 0.0, -5.0]),
                (2.0, [0.0, 0.0, -5.0]),
                (3.0, [0.0, 0.0, -0.0]),
            ],
        );
        let t = KeyTrack::parse(&b).unwrap();
        assert_eq!(t.mode, PlayMode::Once);
        assert_eq!(t.duration, 3.0);
        assert_eq!(t.keys.len(), 4);
        assert_eq!(t.keys[1].time, 1.0);
        assert_eq!(t.keys[1].position, [0.0, 0.0, -5.0 * CENTIMETRES_TO_METRES]);
        assert_eq!(t.keys[3].time, 3.0);
    }

    #[test]
    fn modes_from_words() {
        assert_eq!(PlayMode::from_word(0), PlayMode::Free);
        assert_eq!(PlayMode::from_word(1), PlayMode::Once);
        assert_eq!(PlayMode::from_word(2), PlayMode::Reverse);
        assert_eq!(PlayMode::from_word(3), PlayMode::Loop);
        assert_eq!(PlayMode::from_word(7), PlayMode::Free);
    }

    #[test]
    fn short_or_huge_payloads_are_errors() {
        let mut b = payload(3, 1.0, &[(0.0, [1.0; 3]), (1.0, [2.0; 3])]);
        b.truncate(b.len() - 1);
        assert!(KeyTrack::parse(&b).is_err());
        assert!(KeyTrack::parse(&b[..10]).is_err());
        let mut huge = payload(1, 1.0, &[(0.0, [0.0; 3])]);
        huge[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(KeyTrack::parse(&huge).is_err());
        // Trailing bytes after the keys are ignored.
        let mut long = payload(1, 1.0, &[(0.0, [0.0; 3])]);
        long.extend([0xAA; 5]);
        assert_eq!(KeyTrack::parse(&long).unwrap().keys.len(), 1);
    }

    /// Every chunk tree of the installed packs, nested packs included
    /// (skipped without the game): one keyframe track ships, on the brick
    /// of the secret switch (`uwa01 brkswtch`, node `Box001`, under the
    /// root `Plane001`): once, 3 s, pushed 5 cm along −z in the first
    /// second, held, and back by 3 s. Every track decodes, and the
    /// sub-chunks read after the bodies of other node kinds hold no
    /// collision body or cloth marker.
    #[test]
    fn shipped_tracks() {
        use crate::FileKind;
        use crate::rfc::{NODE_MESH, Scene};
        let Some(dir) = exa_rpk::locate_install(None) else {
            return;
        };
        let game = exa_rpk::GameData::open(&dir).unwrap();
        let mut trees = Vec::new();
        for (kind, pack) in game.packs() {
            for e in pack.entries() {
                let b = pack.bytes(e);
                match FileKind::detect(b) {
                    FileKind::ChunkTree | FileKind::ChunkTreeAlt => {
                        trees.push((format!("{}/{}", kind.file_stem(), e.name), b));
                    }
                    FileKind::Pack => {
                        let inner = exa_rpk::Archive::parse(b).unwrap();
                        for f in inner.entries() {
                            let b = inner.bytes(f);
                            if matches!(
                                FileKind::detect(b),
                                FileKind::ChunkTree | FileKind::ChunkTreeAlt
                            ) {
                                trees.push((format!("{}/{}", e.name, f.name), b));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        let (mut scenes, mut found, mut bad, mut stray) = (0, Vec::new(), 0, 0);
        for (name, b) in &trees {
            let Ok(scene) = Scene::parse(b) else {
                continue;
            };
            scenes += 1;
            for node in &scene.nodes {
                bad += node.key_tracks.iter().filter(|t| t.is_err()).count();
                if node.kind != NODE_MESH && (node.has_rigid_body() || node.cloth_marker) {
                    stray += 1;
                }
                for t in node.tracks() {
                    let parent = node.parent.map(|p| scene.nodes[p - 1].name.clone());
                    found.push((name.clone(), node.name.clone(), parent, t.clone()));
                }
            }
        }
        assert!(scenes > 7000, "{scenes} chunk trees");
        assert_eq!((bad, stray), (0, 0));
        assert_eq!(found.len(), 1, "{found:?}");
        let (file, node, parent, track) = &found[0];
        assert_eq!(file, "Objects/uwa01 brkswtch");
        assert_eq!(
            (node.as_str(), parent.as_deref()),
            ("Box001", Some("Plane001"))
        );
        assert_eq!((track.mode, track.duration), (PlayMode::Once, 3.0));
        let keys: Vec<_> = track.keys.iter().map(|k| (k.time, k.position)).collect();
        let pushed = -5.0 * CENTIMETRES_TO_METRES;
        assert_eq!(
            keys,
            [
                (0.0, [0.0, 0.0, -0.0]),
                (1.0, [0.0, 0.0, pushed]),
                (2.0, [0.0, 0.0, pushed]),
                (3.0, [0.0, 0.0, -0.0]),
            ]
        );
    }
}
