//! The renderer's pass order and render-state lists.
//!
//! The original walks per-material render-state lists in one fixed order a
//! frame: a clear to the environment's background colour, the alpha-tested
//! and opaque lists with depth writes, hook lists, the depth-biased decal
//! list with depth writes off, the blended lists and the particles, each
//! group followed by its hook list. Its two list orderings are modelled
//! here, and the pass order is the contract Exarust's render graph follows:
//! Bevy's opaque, alpha-mask and transparent phases take the place of the
//! fixed list walk, and the custom nodes (`exa_cube_shadows`, `exa_gi`,
//! `exa_post`) fill the per-frame slots.
//!
//! Rules: `docs/RENDERING.md` (Pass order).

/// The per-material list a material record's class word belongs to
/// (`0x04D860`): the alpha-tested bit `0x8000` selects the shadow list
/// whatever the low nibble; otherwise the low nibble `0` is the opaque
/// list, `1`, `2` and `3` the blended list, and every other value has no
/// list (the original returns nothing for it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaterialList {
    Opaque,
    Shadow,
    Blended,
}

/// The list a class word selects.
pub fn material_list(class_word: u32) -> Option<MaterialList> {
    if class_word & 0x8000 != 0 {
        return Some(MaterialList::Shadow);
    }
    match class_word & 0xF {
        0 => Some(MaterialList::Opaque),
        1..=3 => Some(MaterialList::Blended),
        _ => None,
    }
}

/// The entry of one class list holding material `id`, searched from the
/// first in order; `None` when the list does not hold it.
pub fn find_class_entry(entries: &[u32], id: u32) -> Option<usize> {
    entries.iter().position(|&entry| entry == id)
}

/// One slot of the renderer's fixed frame walk, in the order the original
/// draws it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pass {
    /// The environment's per-frame setup (fog, background colour) and the
    /// clear. The environment's own sky hooks around the lists are empty, so
    /// no sky is drawn here.
    Clear,
    /// The alpha-tested class (`0x8000`), with depth writes.
    AlphaTest,
    /// The opaque class, with depth writes.
    Opaque,
    /// The hook list after the opaque lists.
    AfterOpaqueHooks,
    /// Two more hook lists with depth writes explicitly on, then the depth
    /// test on and depth writes off.
    DepthHooks,
    /// The `0x10000` class (decals), with depth writes off and every view's
    /// projection depth term raised by 2·10⁻⁵ (a depth bias that stays on
    /// through the blended lists).
    Decals,
    /// The hook list after the decals, once the region scissor is off.
    AfterDecalHooks,
    /// The blended class: one list for a sectorless scene, or one per scene
    /// sector, depth writes off and still depth-biased.
    Blended,
    /// The hook list after the blended lists (the projection bias is gone).
    AfterBlendedHooks,
    /// The particles' depth-only pre-pass (colour writes off, depth writes
    /// on).
    ParticleDepth,
    /// The particles' colour, with depth writes off.
    ParticleColour,
    /// The environment's weather effects after the particles.
    EnvironmentEffects,
    /// The closing hook list, then depth writes back on.
    EndHooks,
}

/// The frame's pass order.
pub const PASS_ORDER: [Pass; 13] = [
    Pass::Clear,
    Pass::AlphaTest,
    Pass::Opaque,
    Pass::AfterOpaqueHooks,
    Pass::DepthHooks,
    Pass::Decals,
    Pass::AfterDecalHooks,
    Pass::Blended,
    Pass::AfterBlendedHooks,
    Pass::ParticleDepth,
    Pass::ParticleColour,
    Pass::EnvironmentEffects,
    Pass::EndHooks,
];

/// The depth bias the decal class draws with: added to the projection's
/// `[2][2]` term of every view from the decals to the end of the blended
/// lists.
pub const DECAL_DEPTH_BIAS: f32 = 2.0e-5;

impl Pass {
    /// Whether the pass draws with depth writes on. The original draws both
    /// opaque classes and the first hook lists with them on, switches them
    /// off before the decals, and switches them on only for the particles'
    /// depth pre-pass until the end of the frame.
    pub fn writes_depth(self) -> bool {
        matches!(
            self,
            Self::AlphaTest
                | Self::Opaque
                | Self::AfterOpaqueHooks
                | Self::DepthHooks
                | Self::ParticleDepth
        )
    }

    /// The Bevy phase or node that stands in for the pass. `None` marks a
    /// pass Exarust has no separate equivalent for.
    pub fn bevy_phase(self) -> Option<&'static str> {
        match self {
            Self::Clear => Some("the camera's clear colour"),
            Self::AlphaTest => Some("AlphaMask3d"),
            Self::Opaque => Some("Opaque3d"),
            Self::AfterOpaqueHooks | Self::DepthHooks => None,
            Self::Decals => Some("Transparent3d"),
            Self::AfterDecalHooks | Self::AfterBlendedHooks => None,
            Self::Blended => Some("Transparent3d"),
            Self::ParticleDepth => None,
            Self::ParticleColour => Some("Transparent3d"),
            Self::EnvironmentEffects => None,
            Self::EndHooks => None,
        }
    }
}

/// The blend layer a render state's blend word names: bits 4–7, 0–15. The
/// sorting routines read exactly these four bits; the low four bits are
/// destination blend factors and play no part in the order.
pub fn blend_layer(blend_word: u16) -> u8 {
    ((blend_word & 0xF0) >> 4) as u8
}

/// One render-state list record.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderEntry {
    /// The material program key (a short in the original's state record).
    pub key: i16,
    /// The blend layer, the blend word's bits 4–7 (0–15; higher bits are
    /// ignored).
    pub layer: u8,
    /// The material's opacity.
    pub opacity: f32,
    /// The material's id, for the test's bookkeeping.
    pub material: u32,
}

/// The blended list's order: the lowest layer first, and within a layer the
/// highest opacity first.
///
/// The original is a selection pass, not a comparison sort. Each output slot
/// scans every unused entry, keeps the first with a layer below the current
/// best (starting from 16, so every 4-bit layer is below it), and within the
/// same layer replaces the best only when the candidate's opacity is
/// strictly greater. The opacity starts at −1 for each output slot and is
/// not reset when a lower layer is found, so an opacity seen in a higher
/// layer still has to be beaten. A NaN candidate never compares greater (the
/// unordered compare is skipped), so an entry whose opacity is NaN is picked
/// only by being the first of a lower layer.
pub fn sort_blended(entries: &[RenderEntry]) -> Vec<RenderEntry> {
    let mut used = vec![false; entries.len()];
    let mut out = Vec::with_capacity(entries.len());
    for _ in 0..entries.len() {
        let mut best: Option<usize> = None;
        let mut best_layer = 16u8;
        let mut best_opacity = -1.0f32;
        for (i, entry) in entries.iter().enumerate() {
            if used[i] {
                continue;
            }
            let layer = entry.layer & 0x0f;
            if best_layer > layer {
                best_layer = layer;
                best = Some(i);
            } else if best_layer == layer && best_opacity < entry.opacity && !entry.opacity.is_nan()
            {
                best_opacity = entry.opacity;
                best = Some(i);
            }
        }
        match best {
            Some(i) => {
                used[i] = true;
                out.push(entries[i]);
            }
            None => break,
        }
    }
    out
}

/// The opaque list's order: the original groups entries that share a program
/// key so the program changes as rarely as possible.
///
/// The first entry stays first. Each following slot takes the first unused
/// entry with the current key; when none is left, it takes the last unused
/// entry, which becomes the next key, and that group follows. The result is
/// not an ascending sort: for keys `[5, 1, 2]` it is `[5, 2, 1]`. The
/// ordering is kept exactly, because it decides draw order within a pass.
pub fn group_by_key(entries: &[RenderEntry]) -> Vec<RenderEntry> {
    if entries.is_empty() {
        return Vec::new();
    }
    let mut used = vec![false; entries.len()];
    used[0] = true;
    let mut out = Vec::with_capacity(entries.len());
    out.push(entries[0]);
    let mut key = entries[0].key;
    for _ in 1..entries.len() {
        let mut pick = None;
        for (i, entry) in entries.iter().enumerate() {
            if used[i] {
                continue;
            }
            pick = Some(i);
            if entry.key == key {
                break;
            }
        }
        match pick {
            Some(i) => {
                used[i] = true;
                out.push(entries[i]);
                key = entries[i].key;
            }
            None => break,
        }
    }
    out
}

/// The render-state list a material's flag word files it in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateList {
    /// Opaque, drawn with depth writes on.
    Opaque,
    /// The alpha-tested class.
    AlphaTest,
    /// The decal class, drawn after depth writes go off. The second renderer
    /// mode files everything but a blending word here too.
    Decal,
    /// Blended.
    Blend,
    /// The second renderer mode's blended list.
    SecondBlend,
}

/// The list a material's flag word picks, or `None` when the word gets no
/// state (a low nibble above 4 in either mode).
///
/// The original checks the bits in this order: `0x10000` first, then
/// `0x8000`, then the low nibble. In the second renderer mode every class
/// but a blending nibble 1-4 goes to the decal list, and a blending nibble
/// has a list of its own.
pub fn state_list(flags: u32, second_mode: bool) -> Option<StateList> {
    if flags & 0x10000 != 0 {
        return Some(StateList::Decal);
    }
    if flags & 0x8000 != 0 {
        return Some(if second_mode {
            StateList::Decal
        } else {
            StateList::AlphaTest
        });
    }
    match flags & 0xF {
        0 => Some(if second_mode {
            StateList::Decal
        } else {
            StateList::Opaque
        }),
        1..=4 => Some(if second_mode {
            StateList::SecondBlend
        } else {
            StateList::Blend
        }),
        _ => None,
    }
}

/// The list the renderer's own lookup walks when it finds an existing state
/// for a material. It reads the flag word differently from [`state_list`]
/// and ignores the renderer mode: the `0x10000` bit is ignored, `0x8000`
/// picks the alpha-tested list, a nibble of 0 the opaque list, 1-4 the
/// blended list and 5-15 nothing.
pub fn existing_state_list(flags: u32) -> Option<StateList> {
    if flags & 0x8000 != 0 {
        return Some(StateList::AlphaTest);
    }
    match flags & 0xF {
        0 => Some(StateList::Opaque),
        1..=4 => Some(StateList::Blend),
        _ => None,
    }
}

/// The flag word a released render state carries. A
/// material below detail 3 loses bit 21 and one below 4 loses bit 23; a
/// renderer with its first flag byte set returns only the `0x0060_0011` bits
/// of the word (the detail clears are discarded); otherwise its second flag
/// byte clears bits 1 and 2. The two flag bytes' meanings are not traced.
pub fn released_flags(flags: u32, detail_level: i32, first_flag: bool, second_flag: bool) -> u32 {
    let mut adjusted = flags;
    if detail_level < 3 {
        adjusted &= !0x0020_0000;
    }
    if detail_level < 4 {
        adjusted &= !0x0080_0000;
    }
    if first_flag {
        return flags & 0x0060_0011;
    }
    if second_flag {
        adjusted &= !6;
    }
    adjusted
}

/// A render state as a list draw reads it: the entry's visibility bits and
/// its opacity. The program key is the group's, not the entry's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StateRecord {
    /// The entry's visibility flag word (the scene node's word). A sector's
    /// own word decides whether the sector is walked at all.
    pub flags: u32,
    /// The material's opacity, used by the blended list as `1 − opacity`.
    pub opacity: f32,
}

/// A material group of a render-state list: the entries that share one
/// compiled program.
#[derive(Clone, Debug, PartialEq)]
pub struct DrawGroup {
    /// The program key the group is bound by.
    pub key: i16,
    /// The group's entries, in list order.
    pub entries: Vec<StateRecord>,
}

/// One program bind and the entries it draws: indices into its group's
/// entry list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListDraw {
    /// The group's index in the list.
    pub group: usize,
    /// The indices of the entries the bind draws, in list order.
    pub entries: Vec<usize>,
}

/// Whether an entry or sector is drawn under the renderer's visibility mask:
/// the original tests `mask & word != 0`, so a word with no bit in common is
/// skipped and a zero mask draws nothing.
pub fn visible(mask: u32, word: u32) -> bool {
    mask & word != 0
}

/// The binds one list makes under a visibility mask.
///
/// The list keeps its entries grouped by material program; the draw walks
/// the groups in list order, binds each group's program once, and then draws
/// only the entries whose flag word shares a bit with the mask. A group with
/// no visible entry is not bound at all. Within a group the entries keep
/// their list order.
pub fn plan_list(groups: &[DrawGroup], mask: u32) -> Vec<ListDraw> {
    let mut out = Vec::new();
    for (group, entries) in groups.iter().enumerate() {
        let draws: Vec<usize> = entries
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| visible(mask, entry.flags))
            .map(|(i, _)| i)
            .collect();
        if !draws.is_empty() {
            out.push(ListDraw {
                group,
                entries: draws,
            });
        }
    }
    out
}

/// The sectors a list is drawn into: their indices in scene order, keeping
/// only the sectors whose flag word shares a bit with the mask.
pub fn visible_sectors(words: &[u32], mask: u32) -> Vec<usize> {
    words
        .iter()
        .enumerate()
        .filter(|(_, word)| visible(mask, **word))
        .map(|(i, _)| i)
        .collect()
}

/// The bit of an entry's flag word that turns the scissor test on while the
/// entry is drawn. A sector with its own rectangle sets it before its lists
/// are drawn and turns the test off; the list draw then switches the test
/// on for entries with this bit and off for entries without it, only when
/// the bit differs from the last visible entry's (see [`scissor_switches`]).
pub const ENTRY_SCISSOR_BIT: u32 = 0x1000_0000;

/// The scissor switches a list draw makes, one per entry in walk order:
/// `Some(true)` turns the test on, `Some(false)` off, `None` leaves it.
///
/// The draw remembers the last visible entry's scissor bit across the
/// whole list (all groups), starting from "off", and switches the test
/// only when a visible entry's bit differs; invisible entries (no bit in
/// common with `mask`) change nothing. `entries` are the flag words of
/// every group's entries, concatenated in list order.
pub fn scissor_switches(entries: &[u32], mask: u32) -> Vec<Option<bool>> {
    let mut last = 0;
    entries
        .iter()
        .map(|&flags| {
            if !visible(mask, flags) {
                return None;
            }
            let bit = flags & ENTRY_SCISSOR_BIT;
            if bit == last {
                None
            } else {
                last = bit;
                Some(bit != 0)
            }
        })
        .collect()
}

/// The bit of a sector's flag word that draws the particles' lists.
pub const SECTOR_PARTICLE_BIT: u32 = 1;

/// The blend word a list's state carries by default (blended, depth writes
/// off).
pub const LIST_BLEND_WORD: u16 = 0x100;
/// The word the blended list's per-entry draw switches to while one entry is
/// drawn: additive, so a premultiplied colour adds.
pub const ENTRY_BLEND_WORD: u16 = 0x203;

/// The alpha a blended list's entry is drawn with: `1 − opacity` in single
/// precision, the group's alpha state set for that one entry and restored
/// after it.
pub fn entry_alpha(opacity: f32) -> f32 {
    1.0 - opacity
}

/// The particle pass's visibility mask with super-sampling on: entries are
/// drawn when their flag word shares a bit with it. The pass swaps the
/// renderer's mask for it and restores the old mask at the end.
pub const PARTICLE_MASK_SUPERSAMPLED: u32 = 0x2000_0000;
/// The particle pass's visibility mask for a single view.
pub const PARTICLE_MASK_SINGLE: u32 = 0x6000_0000;

/// The two particle lists, in the order the pass draws them. The names are
/// a guess from the draws: only the second list's groups bind a texture in
/// the depth pre-pass (alpha-tested sprites), the first draws untextured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParticleList {
    /// The vertex-coloured particles (the pass's first list).
    VertexColour,
    /// The textured particles (its second list).
    Textured,
}

/// Which of the particles' stages a draw belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParticleStage {
    /// Depth only, untextured: colour writes masked off, depth writes on,
    /// no state bind per group.
    Depth,
    /// Depth only, with each group's texture bound once, at its first
    /// visible entry.
    TexturedDepth,
    /// Colour: colour writes on, depth writes off, the full state bound per
    /// visible entry with the group's alpha at `1 − opacity` under
    /// [`ENTRY_BLEND_WORD`] ([`entry_alpha`]). The textured list's colour
    /// draws clear one bit (`0x80`) of the shared program state and restore
    /// it after the sector.
    Colour,
}

/// One draw of the particle pass: a stage, a list and the sector it belongs
/// to (an index into the scene's particle sectors).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParticleDraw {
    /// The stage the draw belongs to.
    pub stage: ParticleStage,
    /// The list drawn.
    pub list: ParticleList,
    /// The particle sector's index.
    pub sector: usize,
}

/// The particle pass's draw order for `sectors` particle sectors.
///
/// The pass masks the colour writes, draws the first list of every
/// particle sector depth-only in sector order, then the second list of
/// every sector depth-only with its groups' textures, restores the colour
/// mask with depth writes off, and then draws, per sector, the first list
/// and the second in colour. The sectors walked are the scene's sectors
/// whose flag word has bit 0 ([`SECTOR_PARTICLE_BIT`]).
pub fn particle_pass(sectors: usize) -> Vec<ParticleDraw> {
    let mut out = Vec::with_capacity(sectors * 4);
    for sector in 0..sectors {
        out.push(ParticleDraw {
            stage: ParticleStage::Depth,
            list: ParticleList::VertexColour,
            sector,
        });
    }
    for sector in 0..sectors {
        out.push(ParticleDraw {
            stage: ParticleStage::TexturedDepth,
            list: ParticleList::Textured,
            sector,
        });
    }
    for sector in 0..sectors {
        for list in [ParticleList::VertexColour, ParticleList::Textured] {
            out.push(ParticleDraw {
                stage: ParticleStage::Colour,
                list,
                sector,
            });
        }
    }
    out
}

impl ParticleStage {
    /// Whether the stage writes colour.
    pub fn writes_colour(self) -> bool {
        matches!(self, Self::Colour)
    }

    /// Whether the stage writes depth: the depth pre-pass does, the colour
    /// stage does not.
    pub fn writes_depth(self) -> bool {
        !self.writes_colour()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(layer: u8, opacity: f32, material: u32) -> RenderEntry {
        RenderEntry {
            key: 0,
            layer,
            opacity,
            material,
        }
    }

    #[test]
    fn the_pass_order_is_the_original_frame_walk() {
        assert_eq!(PASS_ORDER.len(), 13);
        assert_eq!(PASS_ORDER[0], Pass::Clear);
        assert_eq!(PASS_ORDER[PASS_ORDER.len() - 1], Pass::EndHooks);
        // Depth writes are on from the alpha-tested class through the depth
        // hooks, off from the decals on, except for the particle pre-pass.
        let writes: Vec<bool> = PASS_ORDER.iter().map(|p| p.writes_depth()).collect();
        assert_eq!(
            writes,
            [
                false, true, true, true, true, false, false, false, false, true, false, false,
                false
            ]
        );
        // The alpha-tested class comes before the opaque one, the decals
        // before the blended lists, the blended lists before the particles.
        let at = |pass| PASS_ORDER.iter().position(|p| *p == pass).unwrap();
        assert!(at(Pass::AlphaTest) < at(Pass::Opaque));
        assert!(at(Pass::Decals) < at(Pass::Blended));
        assert!(at(Pass::Blended) < at(Pass::ParticleDepth));
        assert!(at(Pass::ParticleDepth) < at(Pass::ParticleColour));
    }

    #[test]
    fn blended_entries_sort_by_layer_then_reverse_opacity() {
        let entries = [
            entry(2, 0.5, 1),
            entry(0, 0.1, 2),
            entry(2, 0.9, 3),
            entry(1, 0.4, 4),
            entry(0, 0.8, 5),
            entry(2, 0.5, 6),
            entry(1, 0.2, 7),
        ];
        let order: Vec<u32> = sort_blended(&entries).iter().map(|e| e.material).collect();
        // Layer 0's higher opacity first. Within a layer the first entry
        // chosen by layer alone does not set the opacity, so the next entry
        // of that layer (material 6 at 0.5) can come before it (material 1
        // also at 0.5).
        assert_eq!(order, [5, 2, 4, 7, 3, 6, 1]);
    }

    #[test]
    fn an_opacity_from_a_higher_layer_is_carried_down() {
        // Layer 2's 0.9 is still the opacity to beat when layer 1 is found,
        // so layer 1's 0.6 cannot replace its 0.5 in the first slot.
        let entries = [
            entry(2, 0.1, 1),
            entry(2, 0.9, 2),
            entry(1, 0.5, 3),
            entry(1, 0.6, 4),
        ];
        let order: Vec<u32> = sort_blended(&entries).iter().map(|e| e.material).collect();
        assert_eq!(order, [3, 4, 2, 1]);
        // Only the layer's four bits count.
        let entries = [
            entry(0x12, 0.5, 1),
            entry(0x01, 0.5, 2),
            entry(0x00, 0.5, 3),
        ];
        let order: Vec<u32> = sort_blended(&entries).iter().map(|e| e.material).collect();
        assert_eq!(order, [3, 2, 1]);
    }

    #[test]
    fn the_blend_layer_is_bits_four_to_seven() {
        assert_eq!(blend_layer(0x0000), 0);
        assert_eq!(blend_layer(0x0010), 1);
        assert_eq!(blend_layer(0x0212), 1);
        assert_eq!(blend_layer(0x0073), 7);
        assert_eq!(blend_layer(0x00F0), 15);
        assert_eq!(blend_layer(0xFFFF), 15);
    }

    #[test]
    fn a_nan_opacity_never_wins_a_tie() {
        let entries = [entry(1, f32::NAN, 1), entry(1, 0.5, 2), entry(1, 0.2, 3)];
        let order: Vec<u32> = sort_blended(&entries).iter().map(|e| e.material).collect();
        // The NaN entry is picked by layer first, but the finite candidates
        // after it are greater than the initial −1 and take its place: the
        // layer's highest finite opacity comes first.
        assert_eq!(order, [2, 3, 1]);
        let entries = [entry(2, f32::NAN, 1), entry(1, 0.5, 2)];
        let order: Vec<u32> = sort_blended(&entries).iter().map(|e| e.material).collect();
        assert_eq!(order, [2, 1]);
        // All-NaN at the same layer: the first is picked, then the next is
        // picked by layer alone, since the opacity never compares greater.
        let entries = [entry(0, f32::NAN, 1), entry(0, f32::NAN, 2)];
        let order: Vec<u32> = sort_blended(&entries).iter().map(|e| e.material).collect();
        assert_eq!(order, [1, 2]);
    }

    #[test]
    fn keyed_entries_group_without_sorting() {
        let entries: Vec<RenderEntry> = [5, 1, 2]
            .into_iter()
            .enumerate()
            .map(|(i, key)| RenderEntry {
                key,
                layer: 0,
                opacity: 1.0,
                material: i as u32,
            })
            .collect();
        let order: Vec<u32> = group_by_key(&entries).iter().map(|e| e.material).collect();
        assert_eq!(order, [0, 2, 1]);
        // Equal keys stay together in their original order.
        let entries: Vec<RenderEntry> = [1, 1, 2, 2, 1]
            .into_iter()
            .enumerate()
            .map(|(i, key)| RenderEntry {
                key,
                layer: 0,
                opacity: 1.0,
                material: i as u32,
            })
            .collect();
        let order: Vec<u32> = group_by_key(&entries).iter().map(|e| e.material).collect();
        assert_eq!(order, [0, 1, 4, 3, 2]);
        assert!(group_by_key(&[]).is_empty());
    }

    #[test]
    fn the_flag_word_picks_the_state_list() {
        // First renderer mode: the bit order is 0x10000, 0x8000, nibble.
        assert_eq!(state_list(0x10000, false), Some(StateList::Decal));
        assert_eq!(state_list(0x18000, false), Some(StateList::Decal));
        assert_eq!(state_list(0x8000, false), Some(StateList::AlphaTest));
        assert_eq!(state_list(0x8001, false), Some(StateList::AlphaTest));
        assert_eq!(state_list(0x0000, false), Some(StateList::Opaque));
        assert_eq!(state_list(0x0004, false), Some(StateList::Blend));
        for nibble in 1..=4 {
            assert_eq!(state_list(nibble, false), Some(StateList::Blend));
        }
        // 5 and above create no state at all.
        assert_eq!(state_list(5, false), None);
        assert_eq!(state_list(0xF, false), None);
        // Second mode: the alpha-tested and opaque words join the decals,
        // and a blending nibble 1-4 gets its own list.
        assert_eq!(state_list(0x10000, true), Some(StateList::Decal));
        assert_eq!(state_list(0x8000, true), Some(StateList::Decal));
        assert_eq!(state_list(0, true), Some(StateList::Decal));
        assert_eq!(state_list(1, true), Some(StateList::SecondBlend));
        assert_eq!(state_list(4, true), Some(StateList::SecondBlend));
        assert_eq!(state_list(5, true), None);
    }

    #[test]
    fn the_lookup_ignores_the_decal_bit() {
        assert_eq!(existing_state_list(0x10000), Some(StateList::Opaque));
        assert_eq!(existing_state_list(0x8000), Some(StateList::AlphaTest));
        assert_eq!(existing_state_list(0), Some(StateList::Opaque));
        assert_eq!(existing_state_list(3), Some(StateList::Blend));
        assert_eq!(existing_state_list(5), None);
    }

    #[test]
    fn the_release_clears_bits_by_detail_and_flags() {
        let all = 0x00FF_FFFF;
        assert_eq!(released_flags(all, 4, false, false), all);
        assert_eq!(released_flags(all, 3, false, false), all & !0x0080_0000);
        assert_eq!(
            released_flags(all, 2, false, false),
            all & !(0x0080_0000 | 0x0020_0000)
        );
        // The first flag returns only the original word's kept bits, even
        // when the detail would have cleared more.
        assert_eq!(released_flags(all, 2, true, false), all & 0x0060_0011);
        assert_eq!(released_flags(all, 2, true, true), all & 0x0060_0011);
        // Without it, the second flag clears the two low blend bits.
        assert_eq!(
            released_flags(all, 2, false, true),
            (all & !(0x0080_0000 | 0x0020_0000)) & !6
        );
    }

    fn group(key: i16, entries: &[(u32, f32)]) -> DrawGroup {
        DrawGroup {
            key,
            entries: entries
                .iter()
                .map(|&(flags, opacity)| StateRecord { flags, opacity })
                .collect(),
        }
    }

    #[test]
    fn a_list_binds_a_group_only_when_one_entry_is_visible() {
        let groups = [
            group(1, &[(0x1, 1.0), (0x2, 1.0)]),
            group(2, &[(0x4, 1.0)]),
            group(3, &[(0x8, 1.0), (0x1, 1.0), (0x10, 1.0)]),
        ];
        // A mask of 1: groups 0 and 2 are bound, group 1 is skipped
        // entirely, and within a group only the visible entries draw.
        let plan = plan_list(&groups, 0x1);
        assert_eq!(
            plan,
            [
                ListDraw {
                    group: 0,
                    entries: vec![0],
                },
                ListDraw {
                    group: 2,
                    entries: vec![1],
                },
            ]
        );
        // A zero mask draws nothing, an all-bits mask draws everything in
        // list order.
        assert!(plan_list(&groups, 0).is_empty());
        let plan = plan_list(&groups, !0);
        assert_eq!(plan.len(), 3);
        assert_eq!(plan[2].entries, [0, 1, 2]);
        // Equal-key groups keep their own order; grouping happened when the
        // list was built.
        assert!(plan_list(&[], !0).is_empty());
    }

    #[test]
    fn sectors_are_walked_in_scene_order_under_the_mask() {
        let words = [0x1, 0x2, ENTRY_SCISSOR_BIT | 0x4, 0];
        assert_eq!(visible_sectors(&words, 0x1), [0]);
        assert_eq!(visible_sectors(&words, 0x6), [1, 2]);
        assert!(visible_sectors(&words, 0).is_empty());
        // The scissor and particle bits are ordinary visibility bits too.
        assert!(visible(0x1000_0000, ENTRY_SCISSOR_BIT));
        assert!(visible(1, SECTOR_PARTICLE_BIT));
        assert!(!visible(1, 2));
    }

    #[test]
    fn the_scissor_test_follows_the_visible_entries_bit() {
        let on = ENTRY_SCISSOR_BIT | 1;
        // Starts off: a plain entry changes nothing, the first scissored one
        // turns it on, repeats leave it, a hidden entry is skipped, and the
        // next visible plain entry turns it off.
        assert_eq!(
            scissor_switches(&[1, on, on, ENTRY_SCISSOR_BIT | 2, 2, 1, on], 1),
            [None, Some(true), None, None, None, Some(false), Some(true)]
        );
        // A hidden scissored entry never switches anything.
        assert_eq!(scissor_switches(&[ENTRY_SCISSOR_BIT], 1), [None]);
        assert!(scissor_switches(&[], 1).is_empty());
        // With the bit itself in the mask the entry is visible through it.
        assert_eq!(
            scissor_switches(&[ENTRY_SCISSOR_BIT], ENTRY_SCISSOR_BIT),
            [Some(true)]
        );
        assert_eq!(PARTICLE_MASK_SUPERSAMPLED, 0x2000_0000);
        assert_eq!(PARTICLE_MASK_SINGLE, 0x6000_0000);
    }

    #[test]
    fn a_blended_entry_draws_at_one_less_its_opacity() {
        assert_eq!(entry_alpha(1.0), 0.0);
        assert_eq!(entry_alpha(0.25), 0.75);
        assert_eq!(entry_alpha(0.0), 1.0);
        // Single precision, and a NaN stays NaN.
        assert_eq!(entry_alpha(0.1), 0.9);
        assert!(entry_alpha(f32::NAN).is_nan());
        assert_eq!(LIST_BLEND_WORD, 0x100);
        assert_eq!(ENTRY_BLEND_WORD, 0x203);
    }

    #[test]
    fn the_particle_pass_draws_depth_first_then_colour() {
        let plan = particle_pass(2);
        assert_eq!(plan.len(), 8);
        // Both lists are drawn depth-only for every sector first: the first
        // list for all sectors, then the second with its textures.
        assert_eq!(
            plan[..4],
            [
                ParticleDraw {
                    stage: ParticleStage::Depth,
                    list: ParticleList::VertexColour,
                    sector: 0
                },
                ParticleDraw {
                    stage: ParticleStage::Depth,
                    list: ParticleList::VertexColour,
                    sector: 1
                },
                ParticleDraw {
                    stage: ParticleStage::TexturedDepth,
                    list: ParticleList::Textured,
                    sector: 0
                },
                ParticleDraw {
                    stage: ParticleStage::TexturedDepth,
                    list: ParticleList::Textured,
                    sector: 1
                },
            ]
        );
        // The colour half is per sector, the first list then the second.
        assert_eq!(
            plan[4..],
            [
                ParticleDraw {
                    stage: ParticleStage::Colour,
                    list: ParticleList::VertexColour,
                    sector: 0
                },
                ParticleDraw {
                    stage: ParticleStage::Colour,
                    list: ParticleList::Textured,
                    sector: 0
                },
                ParticleDraw {
                    stage: ParticleStage::Colour,
                    list: ParticleList::VertexColour,
                    sector: 1
                },
                ParticleDraw {
                    stage: ParticleStage::Colour,
                    list: ParticleList::Textured,
                    sector: 1
                },
            ]
        );
        assert!(particle_pass(0).is_empty());
        for draw in particle_pass(1) {
            assert_eq!(draw.stage.writes_colour(), !draw.stage.writes_depth());
        }
        assert!(ParticleStage::Depth.writes_depth());
        assert!(ParticleStage::Colour.writes_colour());
        assert!(ParticleStage::TexturedDepth.writes_depth());
        assert!(!ParticleStage::TexturedDepth.writes_colour());
    }
    #[test]
    fn a_class_word_picks_its_material_list_and_the_id_is_found_in_order() {
        assert_eq!(material_list(0x000), Some(MaterialList::Opaque));
        assert_eq!(material_list(0x101), Some(MaterialList::Blended));
        assert_eq!(material_list(0x102), Some(MaterialList::Blended));
        assert_eq!(material_list(0x103), Some(MaterialList::Blended));
        assert_eq!(material_list(0x104), None);
        assert_eq!(material_list(0x8004), Some(MaterialList::Shadow));
        assert_eq!(material_list(0x8000), Some(MaterialList::Shadow));
        assert_eq!(material_list(0xFFFF_8000), Some(MaterialList::Shadow));
        assert_eq!(find_class_entry(&[4, 7, 9], 7), Some(1));
        assert_eq!(find_class_entry(&[4, 7, 9], 5), None);
        assert_eq!(find_class_entry(&[], 0), None);
    }
}
