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
/// (slot 1 first), the slots it has nodes in (bit `slot - 1`) and the
/// impact material its parts sound with.
#[derive(Clone, Debug, PartialEq)]
pub struct WornGarment {
    pub roots: Vec<usize>,
    pub occupancy: [u64; 18],
    pub coverage: [u64; 18],
    pub slots: u32,
    /// The garment's layer level: the highest non-zero layer byte over the
    /// item's twenty occupancy words (bones 1 to 20), not just the eighteen
    /// slots. The cache inserts each garment by it.
    pub level: u8,
    /// The hit-sound class the garment gives a body part it covers most
    /// (the item's material byte).
    pub material: u8,
}

/// One garment's part of the dressed mesh: its chosen node per slot, its
/// visible mask (for the cull bits) and its nodes in the weight pass's
/// slot-tree order ([`crate::runtime::merge_order`]; the geometry appends in
/// slot order, [`crate::dress::garment_steps`]).
#[derive(Clone, Debug, PartialEq)]
pub struct GarmentMerge {
    pub chosen: [Option<usize>; 19],
    pub visible: u32,
    pub merge: Vec<usize>,
}

/// The dressed-mesh plan: each garment's merge, in the order given (the
/// builder's layer-level order), and the body's visible mask and the body
/// slots it still merges. `slot_material` is the impact material each body
/// part takes from the garment that leaves most of its slot uncovered.
#[derive(Clone, Debug, PartialEq)]
pub struct DressPlan {
    pub garments: Vec<GarmentMerge>,
    pub body_visible: u32,
    pub body_slots: u32,
    pub slot_material: [u8; 18],
}

/// Plans a dressed body: the coverage union of the worn set, then for each
/// garment, in its layer-level order (the builder inserts each into the
/// character's cache by level, outermost last), its coverage pass, slot
/// assembly and merge order, then the body mask from the union and the
/// per-slot impact material.
pub fn dress_plan(body: &Body, garments: &[WornGarment]) -> DressPlan {
    use crate::layers;
    let mut union = [0u64; 18];
    for g in garments {
        for (u, w) in union.iter_mut().zip(&g.coverage) {
            *u |= w;
        }
    }
    // Stable ascending layer level, as the cache's insertion keeps it: each
    // garment goes after every entry whose level is at most its own.
    let mut order: Vec<usize> = (0..garments.len()).collect();
    order.sort_by_key(|&i| garments[i].level);
    let mut slot_material = [0u8; 18];
    let mut slot_uncovered = [0u64; 18];
    let garments: Vec<GarmentMerge> = order
        .into_iter()
        .map(|i| {
            let g = &garments[i];
            let pass = layers::garment_pass(&g.occupancy, &union, g.slots);
            let chosen = layers::assemble_garment(body, &g.roots, &pass);
            for slot in 1..=18 {
                if g.slots & layers::slot_bit(slot) != 0
                    && slot_uncovered[slot - 1] < pass.uncovered[slot]
                {
                    slot_uncovered[slot - 1] = pass.uncovered[slot];
                    slot_material[slot - 1] = g.material;
                }
            }
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
        slot_material,
    }
}

/// A texture-only rebuild: the skin job built again from the record
/// ([`SkinJob::of_record`]) takes the first apparel texture of the body's
/// mesh children. The original also composites a feral character's
/// apparel overlay (its feral share `(2 − f)·f` in two bytes), which is
/// [`feral_underwear_appearance`].
pub fn retexture(mut job: SkinJob, apparel: &[Option<String>]) -> SkinJob {
    job.take_first_apparel(apparel);
    job
}

/// The re-texture pass's first apparel layer: what it paints and the seed it
/// paints with.
#[derive(Clone, Debug, PartialEq)]
pub struct RetextureLayer {
    pub texture: String,
    pub appearance: exa_formats::objdb::Appearance,
    /// The seed the texture compositor takes as its third word.
    pub seed: u32,
}

/// The first apparel layer of a re-texture: the underwear (item 8) of a
/// feral character takes [`feral_underwear_appearance`] and the record's
/// look seed; every other item keeps its stored appearance and its own seed.
pub fn retexture_layer(
    first: Option<(&str, exa_formats::objdb::Appearance, u32)>,
    item_id: u32,
    feral: f32,
    record_seed: u32,
) -> Option<RetextureLayer> {
    let (texture, appearance, item_seed) = first?;
    if item_id == crate::UNDERWEAR_ITEM {
        Some(RetextureLayer {
            texture: texture.to_string(),
            appearance: feral_underwear_appearance(appearance, feral),
            seed: record_seed,
        })
    } else {
        Some(RetextureLayer {
            texture: texture.to_string(),
            appearance,
            seed: item_seed,
        })
    }
}

/// The two bytes the dressing and re-texture passes write on the
/// underwear's appearance (its damage and dirt, the block's second and
/// third bytes) for a feral level `f`: `(2 − f)·f·255`, rounded to even as
/// the original's conversion does and cut to a byte. Quality is kept.
pub fn feral_underwear_appearance(
    mut appearance: exa_formats::objdb::Appearance,
    feral: f32,
) -> exa_formats::objdb::Appearance {
    let k = (2.0 - feral) * feral;
    let byte = (k * 255.0).round_ties_even() as i64 as u8;
    appearance.damage = byte;
    appearance.dirt = byte;
    appearance
}

/// The creation preview's rebuild ([`Rebuild::prepare`]): the editor hands
/// the factory the look it edits, the body is rebuilt, and the caller
/// refreshes the editor's own view of it. The view refresh is the caller's,
/// as the scene and renderer steps between the rebuild's parts are.
pub fn preview(
    body: &Body,
    root: usize,
    bones: &[usize],
    pixel_format: u32,
    apparel: &[Option<String>],
) -> Rebuild {
    Rebuild::prepare(body, root, bones, pixel_format, apparel)
}

/// The apply step of a background rebuild, as the main thread runs it after
/// the build: the first direct mesh child of the body that carries a skin
/// job has its `0x20` render flag cleared; the new node tree is registered
/// with the scene when the build made one; when the hair was rebuilt the
/// old hair roots (the head's direct children the hair builder owns) are
/// removed and the new ones attached under the head; the viewer is told to
/// refresh. Exarust models the decisions; the scene and viewer calls are
/// the caller's.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ApplyPlan {
    /// The direct mesh child whose texture mark the apply clears.
    pub clear_texture: Option<usize>,
    /// The new body tree the caller registers, when a rebuild built one.
    pub register_tree: Option<usize>,
    /// The old hair roots to unregister and free.
    pub remove_hair: Vec<usize>,
    /// The new hair roots to attach under the head.
    pub attach_hair: Vec<usize>,
    pub notify_viewer: bool,
}

/// The background rebuild's apply plan for `body` ([`ApplyPlan`]): `head`
/// is the node the hair hangs under, and `new_hair` the roots the build
/// grew, `None` when it did not rebuild the hair. (The original keeps the
/// hair step behind a count word: non-zero removes, positive attaches; a
/// rebuilt hair of no roots is read here as removing only. A labelled
/// guess at the word's meaning.)
pub fn apply_plan(
    body: &Body,
    root: usize,
    head: usize,
    new_tree: Option<usize>,
    new_hair: Option<&[usize]>,
) -> ApplyPlan {
    let clear_texture = body.nodes[root]
        .children
        .iter()
        .copied()
        .find(|&i| body.nodes[i].is_mesh() && body.nodes[i].texture.is_some());
    let remove_hair = match new_hair {
        Some(_) => body.nodes[head]
            .children
            .iter()
            .copied()
            .filter(|&i| body.nodes[i].owner == crate::runtime::HAIR_OWNER)
            .collect(),
        None => Vec::new(),
    };
    ApplyPlan {
        clear_texture,
        register_tree: new_tree,
        remove_hair,
        attach_hair: new_hair.map_or_else(Vec::new, <[usize]>::to_vec),
        notify_viewer: true,
    }
}

/// A texture-only rebuild from a record: the job [`SkinJob::of_record`]
/// builds for `record`, then [`retexture`]. The original sets the factory's
/// look to the record before this, which the caller does.
#[allow(clippy::too_many_arguments)]
pub fn retexture_only(
    pixel_format: u32,
    stem: &str,
    record: &exa_formats::characters::CharacterRecord,
    look_word: u32,
    controller_weights: &[f32],
    fresh: &mut dyn FnMut() -> u32,
    exists: &dyn Fn(&str) -> bool,
    apparel: &[Option<String>],
) -> SkinJob {
    let job = SkinJob::of_record(
        pixel_format,
        stem,
        record,
        look_word,
        controller_weights,
        fresh,
        exists,
    );
    retexture(job, apparel)
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

/// What the appearance editor's hair refresh does to a built body: it
/// always removes the old hair, and grows the style again only when the body
/// grows hair (the race's look bit `0x10` set and the head word's bit 1
/// clear, [`crate::look::grows_hair`]). The new roots hang under the body's
/// head-cover node and are registered with the scene; those calls are the
/// caller's.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HairRebuild<'a> {
    /// The old hair is removed and nothing new grows.
    pub remove_only: bool,
    /// The roots grown, in the style's order (`?` roots under a covered
    /// head, the rest otherwise). Empty when nothing grows.
    pub roots: Vec<&'a str>,
}

/// The hair refresh's decision ([`HairRebuild`]) over a style's root names.
pub fn hair_rebuild<'a>(
    roots: impl Iterator<Item = &'a str>,
    look: u32,
    head_word: u64,
) -> HairRebuild<'a> {
    let grows = crate::look::grows_hair(look, head_word);
    HairRebuild {
        remove_only: !grows,
        roots: hair_roots(roots, look, head_word),
    }
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
            level: crate::layers::layer_level(&occupancy),
            material: 3,
        };
        let plan = dress_plan(&b, &[garment]);
        assert_eq!(plan.garments[0].chosen[1], Some(0));
        assert_eq!(plan.garments[0].merge, [0]);
        // The pelvis holds none of its partial mask, so its partial bit goes.
        assert_eq!(plan.garments[0].visible & 0x10_0001, 0x1);
        assert_eq!(plan.body_visible, !0);
        assert_eq!(plan.body_slots, 0x3_FFFF);
        // The garment leaves its pelvis slot uncovered: its material wins.
        assert_eq!(plan.slot_material[0], 3);
        assert_eq!(plan.slot_material[1], 0);
    }

    /// The garments are planned in layer-level order whatever order the
    /// worn set gives them.
    #[test]
    fn the_garments_take_their_layer_order() {
        let b = body();
        let chest = crate::layers::slot_bit(9);
        let mut inner = [0u64; 18];
        inner[8] = 0x01;
        let mut outer = [0u64; 18];
        outer[8] = 0x0200;
        let plan = dress_plan(
            &b,
            &[
                WornGarment {
                    roots: vec![0],
                    occupancy: outer,
                    coverage: outer,
                    slots: chest,
                    level: crate::layers::layer_level(&outer),
                    material: 7,
                },
                WornGarment {
                    roots: vec![0],
                    occupancy: inner,
                    coverage: inner,
                    slots: chest,
                    level: crate::layers::layer_level(&inner),
                    material: 1,
                },
            ],
        );
        // The inner garment (layer 0) is merged before the outer (layer 1).
        assert_eq!(plan.garments.len(), 2);
        assert_eq!(plan.garments[0].chosen[9], Some(1));
        assert_eq!(plan.garments[1].chosen[9], Some(1));
    }

    /// The cache sorts by the level over all twenty occupancy words: a
    /// garment whose high layer lies on bone 20 comes after one on the
    /// slots even when it is worn first.
    #[test]
    fn the_layer_order_reads_bone_twenty() {
        let b = body();
        let chest = crate::layers::slot_bit(9);
        let mut inner = [0u64; 18];
        inner[8] = 0x01;
        let mut bone20 = [0u64; 20];
        bone20[8] = 0x01;
        bone20[19] = 0x0300_0000_0000_0000;
        let plan = dress_plan(
            &b,
            &[
                WornGarment {
                    roots: vec![0],
                    occupancy: bone20[..18].try_into().unwrap(),
                    coverage: [0; 18],
                    slots: chest,
                    level: crate::layers::layer_level(&bone20),
                    material: 7,
                },
                WornGarment {
                    roots: vec![0],
                    occupancy: inner,
                    coverage: [0; 18],
                    slots: chest,
                    level: 0,
                    material: 1,
                },
            ],
        );
        // The second garment (level 0) is planned first; both choose the
        // chest node.
        assert_eq!(plan.garments[0].chosen[9], Some(1));
        assert_eq!(plan.garments[1].chosen[9], Some(1));
        assert_eq!(plan.slot_material[8], 1);
    }

    /// The per-slot impact material is the garment that leaves the most of
    /// the slot uncovered, compared as the 64-bit word the original keeps.
    #[test]
    fn the_slot_material_is_the_least_covering_garment() {
        let b = body();
        let mut inner = [0u64; 18];
        inner[8] = 0x01;
        let mut outer = [0u64; 18];
        outer[8] = 0x0200;
        let chest = crate::layers::slot_bit(9);
        let plan = dress_plan(
            &b,
            &[
                WornGarment {
                    roots: vec![0],
                    occupancy: inner,
                    coverage: inner,
                    slots: chest,
                    level: crate::layers::layer_level(&inner),
                    material: 1,
                },
                WornGarment {
                    roots: vec![0],
                    occupancy: outer,
                    coverage: outer,
                    slots: chest,
                    level: crate::layers::layer_level(&outer),
                    material: 7,
                },
            ],
        );
        // The outer garment leaves its own layer byte uncovered (2), the
        // inner's own byte is covered by the outer (1): the outer wins.
        assert_eq!(plan.slot_material[8], 7);
        assert_eq!(plan.slot_material[0], 0);
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

    /// The feral overlay on the underwear: damage and dirt become
    /// `(2 − f)·f·255` rounded to even, quality is kept; every other item
    /// keeps its own appearance and seed.
    #[test]
    fn the_feral_underwear_takes_the_feral_share() {
        let appearance = exa_formats::objdb::Appearance {
            quality: 160,
            damage: 64,
            dirt: 3,
            layers: [Default::default(); 8],
        };
        // 0.5: 1.5·0.5·255 = 191.25 → 191.
        let feral = feral_underwear_appearance(appearance, 0.5);
        assert_eq!((feral.damage, feral.dirt), (191, 191));
        assert_eq!(feral.quality, 160);
        let calm = feral_underwear_appearance(appearance, 0.0);
        assert_eq!((calm.damage, calm.dirt), (0, 0));
        let full = feral_underwear_appearance(appearance, 1.0);
        assert_eq!((full.damage, full.dirt), (255, 255));
        // The underwear (item 8) takes the record's seed; anything else its
        // own.
        let layer = retexture_layer(
            Some(("&braies", appearance, 77)),
            crate::UNDERWEAR_ITEM,
            0.25,
            0x1234,
        )
        .unwrap();
        assert_eq!(layer.seed, 0x1234);
        // 0.25: 1.75·0.25·255 = 111.5625 → 112.
        assert_eq!((layer.appearance.damage, layer.appearance.dirt), (112, 112));
        let other = retexture_layer(Some(("&shirt", appearance, 77)), 144, 0.25, 0x1234).unwrap();
        assert_eq!((other.seed, other.appearance), (77, appearance));
        assert_eq!(retexture_layer(None, 8, 1.0, 0), None);
    }

    /// The creation preview builds the same rebuild plan as the builder, and
    /// the texture-only wrapper builds the job from the record and takes the
    /// first apparel.
    #[test]
    fn the_preview_and_texture_wrappers_build_the_same_plans() {
        let body = body();
        let apparel = [Some("&doublet_ba1".to_string())];
        let preview = preview(&body, 0, &[0, 1], 3, &apparel);
        let prepare = Rebuild::prepare(&body, 0, &[0, 1], 3, &apparel);
        assert_eq!(preview, prepare);
        let record = exa_formats::characters::CharacterRecord::default();
        let exists = |_: &str| true;
        let job = retexture_only(
            2,
            "human",
            &record,
            0x1BF,
            &[0.0; 12],
            &mut || 1,
            &exists,
            &apparel,
        );
        assert_eq!(job.colour.as_deref(), Some("humandifm"));
        assert_eq!(job.first_apparel.as_deref(), Some("&doublet_ba1"));
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
        // The refresh removes and does not grow on a race without hair or a
        // head word with bit 1; otherwise it removes and grows the style.
        let remove = hair_rebuild(roots.iter().copied(), 0x1AF, 0);
        assert!(remove.remove_only && remove.roots.is_empty());
        let remove = hair_rebuild(roots.iter().copied(), 0x1BF, 2);
        assert!(remove.remove_only && remove.roots.is_empty());
        let grow = hair_rebuild(roots.iter().copied(), 0x1BF, 0);
        assert!(!grow.remove_only && grow.roots == ["HULA01 o"]);
        let covered = hair_rebuild(roots.iter().copied(), 0x1BF, 1);
        assert!(!covered.remove_only && covered.roots == ["?HULA01_"]);
    }

    #[test]
    fn a_free_render_flag_does_not_wait() {
        wait_for_render(&std::sync::atomic::AtomicBool::new(false));
    }

    /// The apply step clears the first marked direct mesh child, registers
    /// the new tree, and on a hair rebuild swaps the head's own hair roots;
    /// the viewer is always notified.
    #[test]
    fn the_apply_plan_clears_textures_and_swaps_hair() {
        let mut b = body();
        b.nodes.push(Node {
            name: "strap".into(),
            parent: Some(0),
            ..Node::default()
        });
        for i in [1, 2] {
            b.nodes[i].chunk_kind = crate::runtime::MESH;
            b.nodes[i].texture = Some(4);
        }
        let head = 1;
        let hair = |parent| crate::runtime::Node {
            name: "?HULA01_".into(),
            parent: Some(parent),
            owner: crate::runtime::HAIR_OWNER,
            ..Default::default()
        };
        let under_head = b.nodes.len();
        b.nodes.push(hair(head));
        // A hair-owned node elsewhere is not the head's and stays.
        b.nodes.push(hair(0));
        b.refresh_links();
        let plan = apply_plan(&b, 0, head, Some(7), Some(&[9]));
        assert_eq!(plan.clear_texture, Some(1));
        assert_eq!(plan.register_tree, Some(7));
        assert_eq!(plan.remove_hair, [under_head]);
        assert_eq!(plan.attach_hair, [9]);
        assert!(plan.notify_viewer);
        // No hair rebuild: the old roots stay.
        let plan = apply_plan(&b, 0, head, None, None);
        assert!(plan.remove_hair.is_empty() && plan.attach_hair.is_empty());
        assert_eq!(plan.register_tree, None);
        let mut plain = body();
        plain.nodes[1].chunk_kind = crate::runtime::MESH;
        let plan = apply_plan(&plain, 0, 1, None, None);
        assert_eq!(plan.clear_texture, None);
    }
}
