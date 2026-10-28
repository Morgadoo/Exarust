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
}
