//! The generator's rebuild paths: rebuilding an existing body for a new
//! look, its textures only, or its hair in the creation scene.
//!
//! The original runs the builder again on an existing body when the
//! creation screen changes a look. Its steps that touch frames, dynamics and
//! the skin job run here on [`crate::runtime::Body`] and
//! [`crate::texture_job::SkinJob`]; the scene, renderer and morph steps
//! between them are the caller's.

use exa_formats::morph::Frame;

use crate::runtime::{Body, Node};
use crate::texture_job::SkinJob;

/// A body's rebuild state, as the original keeps it while it runs.
#[derive(Clone, Debug, PartialEq)]
pub struct Rebuild {
    /// The character's bones, in its bone list's order.
    pub bones: Vec<usize>,
    /// The bones' local frames saved before the rebuild, normalised.
    pub frames: Vec<Frame>,
    /// The root's local translation before the rebuild.
    pub root_translation: [f32; 3],
    /// The skin texture job, reset for the new pass.
    pub texture: SkinJob,
}

impl Rebuild {
    /// The first steps of a rebuild of `body`: the skin job is reset at
    /// `pixel_format` and takes the first apparel texture of the root's
    /// mesh children, the root's local translation is kept and the bones'
    /// frames are saved ([`Body::save_frames`]).
    pub fn prepare(
        body: &Body,
        root: usize,
        bones: &[usize],
        pixel_format: u32,
        apparel: &[Option<String>],
    ) -> Self {
        let mut texture = SkinJob::new(pixel_format);
        texture.take_first_apparel(apparel);
        Self {
            bones: bones.to_vec(),
            frames: body.save_frames(bones),
            root_translation: body.nodes[root].local[3],
            texture,
        }
    }

    /// The steps after the body is dressed again: its dynamics are rebuilt
    /// from `root`, the saved frames restored, and, unless the character's
    /// condition word has `0x200` (lying), its standing heights set from
    /// `height` and the root raised by the change of standing height over
    /// its kept height; a lying character's root takes its kept
    /// translation back. Returns the new standing and leg-reach heights
    /// (unchanged when lying). The caller then refreshes the node frames
    /// and places the rest points ([`Body::restore_world`]).
    pub fn finish(
        &self,
        body: &mut Body,
        root: usize,
        shape: &dyn Fn(&Node) -> [[f32; 3]; 4],
        standing: (f32, f32),
        height: f32,
        condition: u32,
    ) -> (f32, f32) {
        body.rebuild_dynamics(root, shape);
        body.restore_frames(&self.bones, &self.frames);
        if condition & 0x200 == 0 {
            let heights = Self::heights(height);
            body.nodes[root].local[3][1] = (heights.0 - standing.0) + self.root_translation[1];
            heights
        } else {
            body.nodes[root].local[3] = self.root_translation;
            standing
        }
    }

    /// The body scale a rebuild applies for the record's height:
    /// `1 − 0.05·height`, worked in double.
    pub fn scale(height: f32) -> f32 {
        (1.0 - f64::from(height) * 0.05) as f32
    }

    /// The standing heights the rebuild writes: the pelvis at 109.5 −
    /// 11.25·height and the legs' reach at 108.5 − 11.25·height cm.
    pub fn heights(height: f32) -> (f32, f32) {
        (109.5 - height * 11.25, 108.5 - height * 11.25)
    }
}

/// One worn garment as the dressed-mesh pass takes it: the roots of its
/// node trees in the body, its occupancy and coverage words per slot
/// (slot 1 first) and the slots it has nodes in (bit `slot - 1`).
#[derive(Clone, Debug, PartialEq)]
pub struct WornGarment {
    pub roots: Vec<usize>,
    pub occupancy: [u64; 18],
    pub coverage: [u64; 18],
    pub slots: u32,
}

/// One garment's part of the dressed mesh: its chosen node per slot, its
/// visible mask (for the cull bits) and its nodes in merge order.
#[derive(Clone, Debug, PartialEq)]
pub struct GarmentMerge {
    pub chosen: [Option<usize>; 19],
    pub visible: u32,
    pub merge: Vec<usize>,
}

/// The dressed-mesh plan: each garment's merge, in the order given (the
/// builder's layer-level order), and the body's visible mask and the body
/// slots it still merges.
#[derive(Clone, Debug, PartialEq)]
pub struct DressPlan {
    pub garments: Vec<GarmentMerge>,
    pub body_visible: u32,
    pub body_slots: u32,
}

/// Plans a dressed body: the coverage union of the worn set, then for each
/// garment its coverage pass, slot assembly and merge order, then the body
/// mask from the union.
pub fn dress_plan(body: &Body, garments: &[WornGarment]) -> DressPlan {
    use crate::layers;
    let mut union = [0u64; 18];
    for g in garments {
        for (u, w) in union.iter_mut().zip(&g.coverage) {
            *u |= w;
        }
    }
    let garments = garments
        .iter()
        .map(|g| {
            let pass = layers::garment_pass(&g.occupancy, &union, g.slots);
            let chosen = layers::assemble_garment(body, &g.roots, &pass);
            GarmentMerge {
                chosen,
                visible: pass.visible,
                merge: crate::runtime::merge_order(&chosen),
            }
        })
        .collect();
    let body_visible = layers::visible_body_slots(&union);
    DressPlan {
        garments,
        body_visible,
        body_slots: layers::drawn_body_slots(body_visible),
    }
}

/// A texture-only rebuild: the skin job built again from the record
/// ([`SkinJob::of_record`]) takes the first apparel texture of the body's
/// mesh children. The original also composites a feral character's
/// apparel overlay (its feral share `(2 − f)·f` in two bytes), which is not
/// reproduced.
pub fn retexture(mut job: SkinJob, apparel: &[Option<String>]) -> SkinJob {
    job.take_first_apparel(apparel);
    job
}

/// The creation scene's hair rebuild: every hair root is removed, then,
/// when the body grows hair ([`crate::look::grows_hair`]), the style's
/// roots are grown again under the head's cover rule. Returns the roots
/// grown.
pub fn hair_roots<'a>(
    roots: impl Iterator<Item = &'a str>,
    look: u32,
    head_word: u64,
) -> Vec<&'a str> {
    if !crate::look::grows_hair(look, head_word) {
        return Vec::new();
    }
    let covered = crate::look::head_covered(head_word);
    roots
        .filter(|name| crate::look::hair_root_kept(name, covered))
        .collect()
}

/// Waits while the render thread still owns the old texture
///. The original sleeps a millisecond and checks the node's
/// busy flag; a single-threaded caller waits cooperatively.
pub fn wait_for_render(busy: &std::sync::atomic::AtomicBool) {
    while busy.load(std::sync::atomic::Ordering::Acquire) {
        std::thread::yield_now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::Node;

    fn body() -> Body {
        let mut nodes = vec![
            Node {
                name: "01Pelvis".into(),
                ..Node::default()
            },
            Node {
                name: "09Chest".into(),
                parent: Some(0),
                ..Node::default()
            },
        ];
        nodes[0].dynamics = Some(crate::runtime::Dynamics {
            flags: 0x1200_0000,
            scales: [0.25; 4],
            index: 0,
            ..Default::default()
        });
        let mut b = Body::new(nodes);
        b.chain_slots(0, false, 18);
        b
    }

    /// The rebuild restores the bones' rows (their translations stay),
    /// resets the body's scales and raises the root by the change of
    /// standing height; a lying body keeps its root where it was.
    #[test]
    fn a_rebuild_saves_frames_and_restores_them() {
        let shape = |_: &Node| [[0.0; 3]; 4];
        let mut b = body();
        b.nodes[0].local[3] = [0.0, 2.0, 0.0];
        b.nodes[1].local[0] = [2.0, 0.0, 0.0];
        let apparel = [None, Some("&doublet_ba1".to_string())];
        let rebuild = Rebuild::prepare(&b, 0, &[0, 1], 2, &apparel);
        assert_eq!(
            rebuild.texture.first_apparel.as_deref(),
            Some("&doublet_ba1")
        );
        b.nodes[1].local[0] = [0.0, 3.0, 0.0];
        b.nodes[1].local[3] = [9.0; 3];
        let heights = rebuild.finish(&mut b, 0, &shape, (100.0, 99.0), 0.0, 0);
        assert_eq!(heights, (109.5, 108.5));
        assert!((b.nodes[1].local[0][0] - 1.0).abs() < 1e-3);
        assert_eq!(b.nodes[1].local[3], [9.0; 3]);
        assert_eq!(b.nodes[0].dynamics.unwrap().scales, [1.0; 4]);
        assert_eq!(b.nodes[0].local[3][1], 9.5 + 2.0);
        b.nodes[0].local[3] = [5.0; 3];
        let lying = rebuild.finish(&mut b, 0, &shape, (100.0, 99.0), 0.0, 0x200);
        assert_eq!(lying, (100.0, 99.0));
        assert_eq!(b.nodes[0].local[3], [0.0, 2.0, 0.0]);
        // The scale is worked in double: 1 − 0.05·0.3.
        assert_eq!(Rebuild::scale(0.3), (1.0 - f64::from(0.3f32) * 0.05) as f32);
    }

    #[test]
    fn heights_follow_the_record() {
        assert_eq!(Rebuild::heights(-1.0), (120.75, 119.75));
        assert_eq!(Rebuild::heights(1.0), (98.25, 97.25));
    }

    /// A garment of a pelvis (layer bits `00`, occupancy `0x01`) with a
    /// chest under it: the pelvis scores 16 against an empty cover and is
    /// chosen; the chest's slot holds no word of the garment, so only the
    /// pelvis merges. Nothing hides a body part.
    #[test]
    fn the_dress_plan_assembles_and_orders_the_merge() {
        let mut b = body();
        b.nodes[0].chunk_kind = crate::runtime::MESH;
        b.nodes[1].chunk_kind = crate::runtime::MESH;
        let mut occupancy = [0u64; 18];
        occupancy[0] = 0x01;
        let garment = WornGarment {
            roots: vec![0],
            occupancy,
            coverage: occupancy,
            slots: crate::layers::slot_bit(1),
        };
        let plan = dress_plan(&b, &[garment]);
        assert_eq!(plan.garments[0].chosen[1], Some(0));
        assert_eq!(plan.garments[0].merge, [0]);
        // The pelvis holds none of its partial mask, so its partial bit goes.
        assert_eq!(plan.garments[0].visible & 0x10_0001, 0x1);
        assert_eq!(plan.body_visible, !0);
        assert_eq!(plan.body_slots, 0x3_FFFF);
    }

    #[test]
    fn a_retexture_takes_the_look_and_the_first_apparel() {
        let record = exa_formats::characters::CharacterRecord::default();
        let exists = |_: &str| true;
        let job = SkinJob::of_record(2, "human", &record, 0x1BF, &[0.0; 12], &mut || 1, &exists);
        let job = retexture(job, &[None, Some("&doublet_ba1".into())]);
        assert_eq!(job.colour.as_deref(), Some("humandifm"));
        assert_eq!(job.first_apparel.as_deref(), Some("&doublet_ba1"));
        assert_eq!(job.format.side, 0x200);
    }

    /// The creation scene grows the `?` roots under a covered head, the
    /// others under a bare one, and none for a race without hair or a head
    /// word with bit 1.
    #[test]
    fn the_hair_roots_follow_the_head() {
        let roots = ["HULA01 o", "?HULA01_"];
        assert_eq!(hair_roots(roots.iter().copied(), 0x1BF, 0), ["HULA01 o"]);
        assert_eq!(hair_roots(roots.iter().copied(), 0x1BF, 1), ["?HULA01_"]);
        assert!(hair_roots(roots.iter().copied(), 0x1BF, 2).is_empty());
        assert!(hair_roots(roots.iter().copied(), 0x1AF, 0).is_empty());
    }

    #[test]
    fn a_free_render_flag_does_not_wait() {
        wait_for_render(&std::sync::atomic::AtomicBool::new(false));
    }
}
