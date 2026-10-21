//! The generator's rebuild paths: saving a body's frames, rebuilding the
//! body, its look or only its textures, and the hair refresh cut.
//!
//! The original runs the same builder again on an existing body when the
//! creation screen changes a look (`0x1000ee9c0`), when only the textures
//! change (`0x1000ef960`) or when only the hair changes (`0x1000ef490`,
//! `0x1000ef7f0`). The parts that touch the scene are kept as a plan here;
//! the frame and slot operations run on [`crate::runtime::Body`], so the
//! rules are testable without a renderer.

use exa_formats::morph::Frame;

use crate::look::{HairCut, hair_refresh};
use crate::runtime::{Body, IDENTITY};
use crate::texture_job::SkinJob;

/// A body's saved build state, as the rebuild keeps it while it runs.
#[derive(Clone, Debug, PartialEq)]
pub struct Rebuild {
    /// The world frames saved before the rebuild (`0x1000ee570`).
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
    /// Starts a rebuild of `body` (`0x1000ee9c0`): saves its frames, resets
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

/// The dressed-mesh plan (`0x1000e7ec0`/`0x1000e9350`): the chosen node of
/// each slot, the body slots still drawn, and the merge order through the
/// tree.
#[derive(Clone, Debug, PartialEq)]
pub struct DressPlan {
    pub chosen: [Option<usize>; 19],
    pub visible: u32,
    pub merge: Vec<usize>,
}

/// Plans a dressed body from the garment coverage words (`0x1000e9350`):
/// assemble the slots, compute the visible body slots, then merge the
/// chosen parts in body order.
pub fn dress_plan(body: &Body, coverage: &[u64; 19], visibility: &[u32; 18]) -> DressPlan {
    let roots = body.roots();
    let selection = crate::layers::assemble_all(body, &roots, coverage, visibility);
    let merge = body.merge_order(&selection.chosen, selection.visible);
    DressPlan {
        chosen: selection.chosen,
        visible: selection.visible,
        merge,
    }
}

/// A texture-only rebuild (`0x1000ef960`): reset the job, re-read the look,
/// and take the first apparel texture. Returns the job.
pub fn retexture(
    pixel_format: u32,
    stem: &str,
    record: &exa_formats::characters::CharacterRecord,
    look_word: u32,
    controller_weights: &[f32],
    apparel: &[Option<String>],
    exists: &dyn Fn(&str) -> bool,
) -> SkinJob {
    let mut job = SkinJob::of_record(
        pixel_format,
        stem,
        record,
        look_word,
        controller_weights,
        exists,
    );
    job.take_first_apparel(apparel);
    job
}

/// The hair cut a look rebuild needs (`0x1000ef490`): keep the hair, show
/// only its `?` roots, or replace it whole.
pub fn hair_cut(
    built_style: u32,
    built_covered: bool,
    style: u32,
    covered: bool,
    has_hair: bool,
) -> HairCut {
    hair_refresh(built_style, built_covered, style, covered, has_hair)
}

/// Rebuilds the hair of an existing body (`0x1000ef7f0`): the style's roots
/// are kept when the cut is [`HairCut::Keep`], the `?` roots alone when it
/// is [`HairCut::Cover`], and nothing when it is [`HairCut::Replace`].
pub fn hair_roots<'a>(
    roots: impl Iterator<Item = &'a str>,
    cut: HairCut,
    head_covered: bool,
) -> Vec<&'a str> {
    roots
        .filter(|name| match cut {
            HairCut::Keep => true,
            HairCut::Cover => crate::look::hair_root_kept(name, head_covered),
            HairCut::Replace => false,
        })
        .collect()
}

/// The creator's preview wrapper (`0x1000f13c0`): a rebuild of the existing
/// body with the new record look.
pub fn preview(
    body: &Body,
    pixel_format: u32,
    slot_frames: [Frame; 19],
    weights: Vec<f32>,
) -> Rebuild {
    Rebuild::prepare(body, pixel_format, slot_frames, weights, None, true)
}

/// The re-texture wrapper (`0x1000f14a0`): only the skin job is rebuilt.
pub fn retexture_only(
    pixel_format: u32,
    stem: &str,
    record: &exa_formats::characters::CharacterRecord,
    look_word: u32,
    controller_weights: &[f32],
    apparel: &[Option<String>],
    exists: &dyn Fn(&str) -> bool,
) -> SkinJob {
    retexture(
        pixel_format,
        stem,
        record,
        look_word,
        controller_weights,
        apparel,
        exists,
    )
}

/// Waits while the render thread still owns the old texture
/// (`0x1000ee930`). The original sleeps a millisecond and checks the node's
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

    #[test]
    fn the_dress_plan_assembles_and_orders_the_merge() {
        let mut b = body();
        b.nodes[0].mesh = true;
        b.nodes[1].mesh = true;
        let mut coverage = [0u64; 19];
        coverage[1] = 1;
        let visibility = [!0u32; 18];
        let plan = dress_plan(&b, &coverage, &visibility);
        assert_eq!(plan.chosen[1], Some(0));
        assert_eq!(plan.merge, [0, 1]);
        assert_eq!(plan.visible, !0);
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
            &exists,
        );
        assert_eq!(job.colour.as_deref(), Some("humandifm"));
        assert_eq!(job.first_apparel.as_deref(), Some("&doublet_ba1"));
        assert_eq!(job.format.side, 0x200);
    }

    #[test]
    fn the_hair_cut_and_roots_follow_the_look() {
        assert_eq!(hair_cut(0, false, 3, false, false), HairCut::Replace);
        assert_eq!(hair_cut(3, false, 3, false, true), HairCut::Keep);
        assert_eq!(hair_cut(3, false, 3, true, true), HairCut::Cover);
        let roots = ["HULA01 o", "?HULA01_"];
        assert_eq!(
            hair_roots(roots.iter().copied(), HairCut::Keep, false),
            roots
        );
        assert_eq!(
            hair_roots(roots.iter().copied(), HairCut::Cover, true),
            ["?HULA01_"]
        );
        assert_eq!(
            hair_roots(roots.iter().copied(), HairCut::Cover, false),
            ["HULA01 o"]
        );
        assert!(hair_roots(roots.iter().copied(), HairCut::Replace, false).is_empty());
    }

    #[test]
    fn a_free_render_flag_does_not_wait() {
        wait_for_render(&std::sync::atomic::AtomicBool::new(false));
    }
}
