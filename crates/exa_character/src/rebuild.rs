//! The generator's rebuild paths: saving a body's frames, rebuilding the
//! body, its look or only its textures, and the hair refresh cut.
//!
//! The original runs the same builder again on an existing body when the
//! creation screen changes a look, when only the textures
//! change or when only the hair changes. The parts that touch the scene are
//! kept as a plan here;
//! the frame and slot operations run on [`crate::runtime::Body`], so the
//! rules are testable without a renderer.

use exa_formats::morph::Frame;

use crate::runtime::{Body, IDENTITY};
use crate::texture_job::SkinJob;

/// A body's saved build state, as the rebuild keeps it while it runs.
#[derive(Clone, Debug, PartialEq)]
pub struct Rebuild {
    /// The world frames saved before the rebuild.
    pub frames: Vec<Frame>,
    /// The skin texture job, reset for the new pass.
    pub texture: SkinJob,
    /// The texture the body had, if any, kept for a failed model load.
    pub kept_texture: Option<String>,
    /// The slot rest frames the nodes are re-framed from.
    pub slot_frames: [Frame; 19],
    /// The physique weights applied to the body.
    pub weights: Vec<f32>,
    /// Whether the body's material is worn as skin.
    pub skin: bool,
}

impl Rebuild {
    /// Starts a rebuild of `body`: saves its frames, resets
    /// the texture job at `pixel_format`, and clears the dynamics/slot
    /// arrays the run fills.
    pub fn prepare(
        body: &Body,
        pixel_format: u32,
        slot_frames: [Frame; 19],
        weights: Vec<f32>,
        kept_texture: Option<String>,
        skin: bool,
    ) -> Self {
        Self {
            frames: body.save_frames(),
            texture: SkinJob::new(pixel_format),
            kept_texture,
            slot_frames,
            weights,
            skin,
        }
    }

    /// Runs the body part of a rebuild on `body`: reset dynamics scales,
    /// re-frame from the slots, apply the morph weights, rebuild the
    /// dynamics and restore the saved frames. Returns the slot frames'
    /// translations for the caller to place its entities.
    pub fn apply(&mut self, body: &mut Body) -> Vec<[f32; 3]> {
        body.rebuild_dynamics(0);
        body.reframe_from_slots(&self.slot_frames);
        // The physique weights move the stored frames; the final restore
        // puts the saved local frames back, as the original does before its
        // final registration.
        body.restore_frames(&self.frames);
        body.refresh_links();
        body.refresh_world();
        (1..19).map(|s| self.slot_frames[s][3]).collect()
    }

    /// The standing heights the spawn writes after a rebuild: the pelvis at
    /// 109.5 − 11.25·height and the legs' reach at 108.5 − 11.25·height cm.
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
                merge: body.merge_order(&chosen, u32::MAX),
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

/// A texture-only rebuild: reset the job, re-read the look,
/// and take the first apparel texture. Returns the job.
pub fn retexture(
    pixel_format: u32,
    stem: &str,
    record: &exa_formats::characters::CharacterRecord,
    look_word: u32,
    controller_weights: &[f32],
    apparel: &[Option<String>],
    fresh: &mut dyn FnMut() -> u32,
    exists: &dyn Fn(&str) -> bool,
) -> SkinJob {
    let mut job = SkinJob::of_record(
        pixel_format,
        stem,
        record,
        look_word,
        controller_weights,
        fresh,
        exists,
    );
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

/// The creator's preview wrapper: a rebuild of the existing
/// body with the new record look.
pub fn preview(
    body: &Body,
    pixel_format: u32,
    slot_frames: [Frame; 19],
    weights: Vec<f32>,
) -> Rebuild {
    Rebuild::prepare(body, pixel_format, slot_frames, weights, None, true)
}

/// Waits while the render thread still owns the old texture
///. The original sleeps a millisecond and checks the node's
/// busy flag; a single-threaded caller waits cooperatively.
pub fn wait_for_render(busy: &std::sync::atomic::AtomicBool) {
    while busy.load(std::sync::atomic::Ordering::Acquire) {
        std::thread::yield_now();
    }
}

/// The identity slot frames a fresh body uses.
pub const IDENTITY_SLOTS: [Frame; 19] = [IDENTITY; 19];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::Node;

    fn body() -> Body {
        let mut nodes = vec![
            Node {
                name: "01Pelvis".into(),
                kind: 0,
                ..Node::default()
            },
            Node {
                name: "09Chest".into(),
                parent: Some(0),
                ..Node::default()
            },
        ];
        nodes[0].children = vec![1];
        nodes[0].dynamics = Some(crate::runtime::Dynamics {
            flags: 0x1200_0000,
            scales: [0.25; 4],
            ..Default::default()
        });
        let mut b = Body::new(nodes);
        b.chain_slots(&[0], |n| crate::layers::slot_of(n).unwrap_or(0));
        b
    }

    #[test]
    fn a_rebuild_saves_frames_and_restores_them() {
        let mut b = body();
        b.nodes[1].local[3] = [1.0, 2.0, 3.0];
        b.refresh_world();
        let mut rebuild = Rebuild::prepare(&b, 2, IDENTITY_SLOTS, vec![0.0; 11], None, true);
        b.nodes[1].local[3] = [9.0; 3];
        rebuild.apply(&mut b);
        assert_eq!(b.nodes[1].world[3], [1.0, 2.0, 3.0]);
        assert_eq!(b.nodes[0].dynamics.unwrap().scales, [1.0; 4]);
        assert_eq!(Rebuild::heights(0.0), (109.5, 108.5));
    }

    #[test]
    fn heights_follow_the_record() {
        assert_eq!(Rebuild::heights(-1.0), (120.75, 119.75));
        assert_eq!(Rebuild::heights(1.0), (98.25, 97.25));
    }

    /// A garment of a pelvis (layer bits `00`, occupancy `0x01`) with a
    /// chest under it: the pelvis scores 16 against an empty cover, is
    /// chosen, and merges with the chest mesh below it; nothing hides a
    /// body part.
    #[test]
    fn the_dress_plan_assembles_and_orders_the_merge() {
        let mut b = body();
        b.nodes[0].mesh = true;
        b.nodes[1].mesh = true;
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
        assert_eq!(plan.garments[0].merge, [0, 1]);
        // The pelvis holds none of its partial mask, so its partial bit goes.
        assert_eq!(plan.garments[0].visible & 0x10_0001, 0x1);
        assert_eq!(plan.body_visible, !0);
        assert_eq!(plan.body_slots, 0x3_FFFF);
    }

    #[test]
    fn a_retexture_takes_the_look_and_the_first_apparel() {
        let record = exa_formats::characters::CharacterRecord::default();
        let exists = |_: &str| true;
        let job = retexture(
            2,
            "human",
            &record,
            0x1BF,
            &[0.0; 12],
            &[None, Some("&doublet_ba1".into())],
            &mut || 1,
            &exists,
        );
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
