//! Rigid bodies of models: the collision-body chunk (`0x3DD0_B001`) that
//! follows a mesh node's body and makes the node a four-particle rigid body
//! in the original (props, items, doors, levers, pressure plates).
//!
//! Payload: `u32 class`, `f32 mobility`, the rest tetrahedron (four
//! `f32[3]`, cm), four point masses, five words the original skips
//! (friction, 1.0, 0, a group mask and one more value), then sub-chunks
//! (`u32 tag, u32 size`, payload): `0xCD00` collision spheres, `0xDDB0`
//! links and `0xDDB3` scrape and roll sound slots. Layout and evidence:
//! `docs/formats/RFC.md` (Rigid bodies).
//!
//! Lengths are returned in metres; the file stores centimetres.

use crate::{Error, Result};

const CENTIMETRES_TO_METRES: f32 = 0.01;
/// Bytes of the fixed part before the sub-chunks.
const HEADER: usize = 4 + 4 + 48 + 16 + 20;
const SPHERES: u32 = 0xCD00;
const LINKS: u32 = 0xDDB0;
const SOUNDS: u32 = 0xDDB3;
/// Bytes of one sound slot in the file: id, point, name[16], volume, pitch.
const SOUND_LEN: usize = 4 + 12 + 16 + 8;
/// Bytes of a link without the slider axis (a slider adds 12).
const LINK_LEN: usize = 4 + 12 + 12 + 12;

/// Link flags.
pub mod link {
    /// A slider: the link carries an axis and its slack is the travel
    /// along it, stored squared.
    pub const SLIDER: u32 = 0x4;
}

/// A model node's rigid body as stored.
#[derive(Debug, Clone, PartialEq)]
pub struct RigidBody {
    /// Class word: `0x1000_0000` (a rigid body) on every shipped body, with
    /// `0x400`, `0x8000` or `0x8` on some (meaning not established).
    pub class: u32,
    /// How easily the body moves (larger moves more), as stored; the
    /// original divides it by a value of the node's material when it
    /// registers the body.
    pub mobility: f32,
    /// The four particles' rest points in the node's frame (metres).
    pub rest: [[f32; 3]; 4],
    /// The four particles' point masses: 4 over the centre of mass's
    /// barycentric weight on each, so gravity acts at the centre of mass.
    pub point_masses: [f32; 4],
    pub spheres: Vec<Sphere>,
    pub links: Vec<BodyLink>,
    pub sounds: Vec<SoundSlot>,
}

/// A collision sphere in the node's frame (metres).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sphere {
    pub centre: [f32; 3],
    pub radius: f32,
}

/// A link of the body: a pin, a rope (with slack), or a slider. A root
/// body's links hold it to its parent node (world links); a child body's
/// links hold it to its parent's body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodyLink {
    /// [`link`] flags.
    pub flags: u32,
    /// The point on this body (metres, node frame).
    pub on_body: [f32; 3],
    /// The point on the parent (metres, in the parent node's frame before
    /// the node's own translation is added).
    pub on_parent: [f32; 3],
    /// Free distance before the link pulls (metres, 0: a pivot); for a
    /// slider, its travel: the square root of the stored value.
    pub slack: f32,
    pub strength: f32,
    pub smoothing: f32,
    /// A slider's axis (metres, node frame); its length is the travel.
    pub axis: Option<[f32; 3]>,
}

impl BodyLink {
    pub fn is_slider(&self) -> bool {
        self.flags & link::SLIDER != 0
    }
}

/// A scrape or roll loop: a sound played while the point moves.
#[derive(Debug, Clone, PartialEq)]
pub struct SoundSlot {
    pub id: u32,
    /// The point whose motion drives the loop (metres, node frame).
    pub point: [f32; 3],
    /// Sound name; slots whose sound is not shipped are dropped by the
    /// original.
    pub sound: String,
    pub volume: f32,
    pub pitch: f32,
}

impl RigidBody {
    /// Decodes a collision-body chunk's payload.
    pub fn parse(b: &[u8]) -> Result<Self> {
        let mut r = Reader::new(b);
        let class = r.u32()?;
        let mobility = r.f32()?;
        let rest = [r.point()?, r.point()?, r.point()?, r.point()?];
        let point_masses = [r.f32()?, r.f32()?, r.f32()?, r.f32()?];
        r.skip(HEADER - r.pos)?;
        let mut body = RigidBody {
            class,
            mobility,
            rest,
            point_masses,
            spheres: Vec::new(),
            links: Vec::new(),
            sounds: Vec::new(),
        };
        while r.remaining() > 0 {
            let tag = r.u32()?;
            let size = r.u32()? as usize;
            let mut d = Reader::new(r.bytes(size)?);
            match tag {
                SPHERES => {
                    d.skip(4)?;
                    let n = d.count(16)?;
                    body.spheres = (0..n)
                        .map(|_| {
                            Ok(Sphere {
                                centre: d.point()?,
                                radius: d.f32()? * CENTIMETRES_TO_METRES,
                            })
                        })
                        .collect::<Result<_>>()?;
                }
                LINKS => {
                    let n = d.count(LINK_LEN)?;
                    body.links = (0..n).map(|_| read_link(&mut d)).collect::<Result<_>>()?;
                }
                SOUNDS => {
                    let n = d.count(SOUND_LEN)?;
                    body.sounds = (0..n)
                        .map(|_| {
                            Ok(SoundSlot {
                                id: d.u32()?,
                                point: d.point()?,
                                sound: d.name()?,
                                volume: d.f32()?,
                                pitch: d.f32()?,
                            })
                        })
                        .collect::<Result<_>>()?;
                }
                _ => {}
            }
        }
        Ok(body)
    }
}

fn read_link(d: &mut Reader<'_>) -> Result<BodyLink> {
    let flags = d.u32()?;
    let on_body = d.point()?;
    let on_parent = d.point()?;
    let raw_slack = d.f32()?;
    let strength = d.f32()?;
    let smoothing = d.f32()?;
    let slider = flags & link::SLIDER != 0;
    let axis = if slider { Some(d.point()?) } else { None };
    let slack = if slider {
        raw_slack.max(0.0).sqrt() * CENTIMETRES_TO_METRES
    } else {
        raw_slack * CENTIMETRES_TO_METRES
    };
    Ok(BodyLink {
        flags,
        on_body,
        on_parent,
        slack,
        strength,
        smoothing,
        axis,
    })
}

struct Reader<'a> {
    b: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(b: &'a [u8]) -> Self {
        Self { b, pos: 0 }
    }

    fn remaining(&self) -> usize {
        self.b.len() - self.pos
    }

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

    fn skip(&mut self, n: usize) -> Result<()> {
        self.bytes(n).map(drop)
    }

    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(
            self.bytes(4)?.try_into().expect("4 bytes"),
        ))
    }

    fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_bits(self.u32()?))
    }

    fn point(&mut self) -> Result<[f32; 3]> {
        Ok([self.f32()?, self.f32()?, self.f32()?].map(|v| v * CENTIMETRES_TO_METRES))
    }

    fn name(&mut self) -> Result<String> {
        let raw = self.bytes(16)?;
        let end = raw.iter().position(|&c| c == 0).unwrap_or(16);
        Ok(raw[..end].iter().map(|&c| c as char).collect())
    }

    /// A stored count, rejected when `count * min_item_size` cannot fit.
    fn count(&mut self, min_item_size: usize) -> Result<usize> {
        let n = self.u32()? as usize;
        match n.checked_mul(min_item_size) {
            Some(bytes) if bytes <= self.remaining() => Ok(n),
            _ => Err(Error::Malformed(format!(
                "count {n} does not fit in {} remaining bytes",
                self.remaining()
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u32s(v: &[u32]) -> Vec<u8> {
        v.iter().flat_map(|x| x.to_le_bytes()).collect()
    }

    fn f32s(v: &[f32]) -> Vec<u8> {
        v.iter().flat_map(|x| x.to_le_bytes()).collect()
    }

    fn chunk(tag: u32, payload: &[u8]) -> Vec<u8> {
        let mut v = u32s(&[tag, payload.len() as u32]);
        v.extend_from_slice(payload);
        v
    }

    /// The fixed part of a body chunk: class, mobility, a unit
    /// tetrahedron (cm), point masses 4, then the five skipped words.
    fn header(class: u32, mobility: f32) -> Vec<u8> {
        let mut b = u32s(&[class]);
        b.extend(f32s(&[mobility]));
        b.extend(f32s(&[
            0.0, 0.0, 0.0, 100.0, 0.0, 0.0, 0.0, 100.0, 0.0, 0.0, 0.0, 100.0,
        ]));
        b.extend(f32s(&[4.0, 4.0, 4.0, 4.0]));
        b.extend(f32s(&[0.75, 1.0]));
        b.extend(u32s(&[0, 0, 0]));
        b
    }

    /// A link record: flags, the two points (cm), slack, strength,
    /// smoothing and, for a slider, its axis.
    fn link(
        flags: u32,
        on_body: [f32; 3],
        on_parent: [f32; 3],
        slack: f32,
        axis: Option<[f32; 3]>,
    ) -> Vec<u8> {
        let mut b = u32s(&[flags]);
        b.extend(f32s(&on_body));
        b.extend(f32s(&on_parent));
        b.extend(f32s(&[slack, 1.0, 0.2]));
        if let Some(a) = axis {
            b.extend(f32s(&a));
        }
        b
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    /// The door of `door uw01a`: a hinge pin and the swing stop at
    /// 134.04 cm of slack, two of its spheres.
    #[test]
    fn decodes_a_door() {
        let mut b = header(0x1000_8000, 0.016);
        let mut spheres = u32s(&[0, 2]);
        spheres.extend(f32s(&[10.0, 20.0, -30.0, 3.5, 0.0, 242.9, 0.0, 0.96]));
        b.extend(chunk(SPHERES, &spheres));
        let mut links = u32s(&[2]);
        links.extend(link(
            0,
            [0.7, 242.9, -0.7],
            [-0.07, 242.9, -1.04],
            0.0,
            None,
        ));
        links.extend(link(
            0,
            [-164.8, 0.0, -0.7],
            [-110.8, 0.0, 121.9],
            134.04,
            None,
        ));
        b.extend(chunk(LINKS, &links));
        let body = RigidBody::parse(&b).unwrap();
        assert_eq!(body.class, 0x1000_8000);
        assert_eq!(body.mobility, 0.016);
        assert_eq!(body.rest[1], [1.0, 0.0, 0.0]);
        assert_eq!(body.point_masses, [4.0; 4]);
        assert_eq!(body.spheres.len(), 2);
        let s = body.spheres[0];
        assert!(close(s.centre[0], 0.1) && close(s.centre[2], -0.3) && close(s.radius, 0.035));
        assert!(close(body.spheres[1].radius, 0.0096));
        let [pin, stop] = [body.links[0], body.links[1]];
        assert!(!pin.is_slider() && pin.axis.is_none());
        assert_eq!(pin.slack, 0.0);
        assert!(close(pin.on_body[1], 2.429));
        assert!(close(stop.slack, 1.3404));
        assert!(close(stop.on_parent[2], 1.219));
        assert_eq!((stop.strength, stop.smoothing), (1.0, 0.2));
        assert!(body.sounds.is_empty());
    }

    /// Sliders store their travel squared: a pressure plate's 16 is 4 cm
    /// along (0, −4, 0), a portcullis's 28 900 is 170 cm.
    #[test]
    fn slider_travel_is_stored_squared() {
        let mut b = header(0x1000_0400, 0.31);
        let mut links = u32s(&[3]);
        links.extend(link(
            link::SLIDER,
            [-25.3, 5.0, 14.7],
            [-25.3, 5.27, 14.7],
            16.0,
            Some([0.0, -4.0, 0.0]),
        ));
        links.extend(link(
            link::SLIDER,
            [70.0, 27.0, 0.0],
            [70.0, 27.0, 0.0],
            28_900.0,
            Some([-170.0, 0.0, 0.0]),
        ));
        links.extend(link(
            link::SLIDER,
            [0.0; 3],
            [0.0; 3],
            -1.0,
            Some([0.0, 0.0, -7.0]),
        ));
        b.extend(chunk(LINKS, &links));
        let body = RigidBody::parse(&b).unwrap();
        let [plate, gate, bad] = [body.links[0], body.links[1], body.links[2]];
        assert!(plate.is_slider());
        assert!(close(plate.slack, 0.04));
        assert_eq!(plate.axis, Some([0.0, -0.04, 0.0]));
        assert!(close(gate.slack, 1.7));
        assert!(close(gate.axis.unwrap()[0], -1.7));
        // A negative stored travel is no travel.
        assert_eq!(bad.slack, 0.0);
    }

    #[test]
    fn decodes_sound_slots_and_skips_unknown_chunks() {
        let mut b = header(0x1000_0000, 34.4);
        b.extend(chunk(0xDDBB, &[1, 2, 3]));
        let mut sounds = u32s(&[1, 0]);
        sounds.extend(f32s(&[0.0, -10.0, 0.0]));
        let mut name = b"ChnAMov".to_vec();
        name.resize(16, 0);
        sounds.extend(name);
        sounds.extend(f32s(&[0.5, 1.0]));
        b.extend(chunk(SOUNDS, &sounds));
        let body = RigidBody::parse(&b).unwrap();
        assert!(body.spheres.is_empty() && body.links.is_empty());
        let slot = &body.sounds[0];
        assert_eq!((slot.id, slot.sound.as_str()), (0, "ChnAMov"));
        assert!(close(slot.point[1], -0.1) && slot.point[0] == 0.0);
        assert_eq!((slot.volume, slot.pitch), (0.5, 1.0));
    }

    #[test]
    fn malformed_bodies_are_errors() {
        // Shorter than the fixed part.
        assert!(RigidBody::parse(&header(0x1000_0000, 1.0)[..40]).is_err());
        // A sphere count the chunk cannot hold.
        let mut b = header(0x1000_0000, 1.0);
        b.extend(chunk(SPHERES, &u32s(&[0, 1000, 0, 0, 0, 0])));
        assert!(RigidBody::parse(&b).is_err());
        // A sub-chunk running past the end.
        let mut b = header(0x1000_0000, 1.0);
        b.extend(u32s(&[LINKS, 400, 1]));
        assert!(RigidBody::parse(&b).is_err());
        // A slider without its axis.
        let mut b = header(0x1000_0000, 1.0);
        let mut links = u32s(&[1]);
        links.extend(link(link::SLIDER, [0.0; 3], [0.0; 3], 16.0, None));
        b.extend(chunk(LINKS, &links));
        assert!(RigidBody::parse(&b).is_err());
        // Every prefix of a valid body parses or fails without panicking.
        let mut b = header(0x1000_0000, 1.0);
        b.extend(chunk(SPHERES, &u32s(&[0, 1, 0, 0, 0, 0])));
        let mut links = u32s(&[1]);
        links.extend(link(
            link::SLIDER,
            [0.0; 3],
            [0.0; 3],
            16.0,
            Some([0.0, -4.0, 0.0]),
        ));
        b.extend(chunk(LINKS, &links));
        for n in 0..b.len() {
            let _ = RigidBody::parse(&b[..n]);
        }
        assert!(RigidBody::parse(&b).is_ok());
    }

    /// Every model of the installed `Objects.rpk` (skipped without the
    /// game): which have bodies and links, the sliders, and the bodies the
    /// notes pin.
    #[test]
    fn shipped_bodies() {
        use crate::FileKind;
        use crate::rfc::Scene;
        use std::collections::BTreeMap;
        let Some(dir) = exa_rpk::locate_install(None) else {
            return;
        };
        let game = exa_rpk::GameData::open(&dir).unwrap();
        let objects = game.pack(exa_rpk::PackKind::Objects).unwrap();
        let (mut models, mut with_body, mut with_links, mut hung, mut bodies) = (0, 0, 0, 0, 0);
        let (mut slider_bodies, mut slider_links, mut sounds) = (0, 0, 0);
        let mut classes: BTreeMap<u32, usize> = BTreeMap::new();
        let mut scenes = BTreeMap::new();
        for e in objects.entries() {
            let b = objects.bytes(e);
            if FileKind::detect(b) != FileKind::ChunkTree {
                continue;
            }
            models += 1;
            let scene = Scene::parse(b).unwrap();
            let decoded: Vec<_> = scene.nodes.iter().filter_map(|n| n.rigid()).collect();
            let present = scene.nodes.iter().filter(|n| n.has_rigid_body()).count();
            assert_eq!(decoded.len(), present, "{}: a body does not decode", e.name);
            bodies += decoded.len();
            with_body += usize::from(!decoded.is_empty());
            with_links += usize::from(decoded.iter().any(|b| !b.links.is_empty()));
            // Hung from the world: a body whose parent node has none carries
            // links.
            let root_links = scene.nodes.iter().any(|n| {
                let parent_body = n
                    .parent
                    .and_then(|p| scene.nodes.get(p - 1))
                    .is_some_and(|p| p.has_rigid_body());
                !parent_body && n.rigid().is_some_and(|b| !b.links.is_empty())
            });
            hung += usize::from(root_links);
            for body in &decoded {
                *classes.entry(body.class).or_default() += 1;
                sounds += body.sounds.len();
                let sliders: Vec<_> = body.links.iter().filter(|l| l.is_slider()).collect();
                slider_bodies += usize::from(!sliders.is_empty());
                slider_links += sliders.len();
                for l in sliders {
                    let a = l.axis.unwrap();
                    let len = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
                    assert!(
                        close(len, l.slack),
                        "{}: travel {} vs axis {len}",
                        e.name,
                        l.slack
                    );
                }
            }
            scenes.insert(e.name.clone(), scene);
        }
        assert_eq!((models, with_body, bodies), (1253, 1163, 1230));
        // 69 models carry links; 65 hang from the world, the flails and the
        // scales only link child bodies to their parents.
        assert_eq!((with_links, hung), (69, 65));
        assert_eq!((slider_bodies, slider_links), (11, 33));
        let classes: Vec<_> = classes.into_iter().collect();
        assert_eq!(
            classes,
            [
                (0x1000_0000, 564),
                (0x1000_0400, 600),
                (0x1000_8000, 39),
                (0x1000_8400, 19),
                (0x1000_8408, 8)
            ]
        );
        // The two flails carry the only sound slots (a chain rattle).
        assert_eq!(sounds, 2);
        let body = |model: &str, node: &str| -> RigidBody {
            let scene = &scenes[model];
            scene
                .nodes
                .iter()
                .find(|n| n.name == node)
                .and_then(|n| n.rigid())
                .unwrap()
                .clone()
        };
        // The door swings on a top hinge pin and a stop 134.04 cm slack.
        let door = body("door uw01a", "UW01_Door");
        assert!(close(door.mobility, 0.016_08));
        assert_eq!(
            (door.class, door.spheres.len(), door.links.len()),
            (0x1000_8000, 1899, 2)
        );
        assert_eq!(door.links[0].slack, 0.0);
        assert!(close(door.links[0].on_body[1], 2.429_12));
        assert!(close(door.links[1].slack, 1.340_372));
        let knocker = body("door uw01a", "Knocker A");
        assert!(close(knocker.links[1].slack, 0.319_244));
        // The lever's arm: slack 33.81 cm at 40 cm.
        let lever = body("lever a01", "LeverHand");
        assert!(close(lever.links[1].slack, 0.338_095));
        assert!(close(lever.links[1].on_body[1], 0.4));
        // Plates sink 4 cm, buttons 7, the portcullis rises 170.
        let plate = body("pressplateu01", "Box001");
        assert!(
            plate
                .links
                .iter()
                .all(|l| l.is_slider() && close(l.slack, 0.04))
        );
        assert!(close(body("button a01", "ButtonA01").links[0].slack, 0.07));
        let gate = body("portcullis uwa", "UWPortCul");
        assert!(
            gate.links
                .iter()
                .all(|l| close(l.slack, 1.7) && l.strength == 0.0)
        );
        assert!(gate.links.iter().all(|l| l.smoothing == 0.01));
        // A chain of six links holds the hanging brazier.
        let brazier = &scenes["brazierhang01"];
        let chain: Vec<_> = brazier.nodes.iter().filter_map(|n| n.rigid()).collect();
        assert_eq!(chain.len(), 7);
        assert!(
            chain
                .iter()
                .all(|b| b.links.len() == 1 && close(b.links[0].slack, 0.0684))
        );
        assert_eq!(chain[0].links[0].flags, 0);
        assert!(chain[1..].iter().all(|b| b.links[0].flags == 0x1_0000));
        assert_eq!(chain[6].spheres.len(), 319);
        // The largest sphere set: an ornate arch door's left wing.
        let most = scenes
            .values()
            .flat_map(|s| s.nodes.iter().filter_map(|n| n.rigid()))
            .map(|b| b.spheres.len())
            .max();
        assert_eq!(most, Some(7839));
        // Every body of the other model packs decodes too.
        for (kind, expected) in [
            (exa_rpk::PackKind::Objlib, 1503),
            (exa_rpk::PackKind::Characters, 12),
            (exa_rpk::PackKind::Resource, 37),
        ] {
            let pack = game.pack(kind).unwrap();
            let mut decoded = 0;
            for e in pack.entries() {
                let b = pack.bytes(e);
                if FileKind::detect(b) != FileKind::ChunkTree {
                    continue;
                }
                let Ok(scene) = Scene::parse(b) else { continue };
                for n in scene.nodes.iter().filter(|n| n.has_rigid_body()) {
                    assert!(
                        n.rigid().is_some(),
                        "{kind:?} {}: {:?}",
                        e.name,
                        n.rigid_body
                    );
                    decoded += 1;
                }
            }
            assert_eq!(decoded, expected, "{kind:?}");
        }
    }
}
