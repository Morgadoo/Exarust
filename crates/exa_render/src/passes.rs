//! The renderer's pass order and render-state lists.
//!
//! The original walks per-material render-state records twice a frame: an
//! opaque pass with depth writes, then the depth-write-free passes, the
//! refraction copy, the blended lists and the particles, each with its own
//! hook list. Its two list orderings are modelled here, and the pass order
//! is the contract Exarust's render graph follows: Bevy's opaque, alpha-mask
//! and transparent phases take the place of the fixed list walk, and the
//! custom nodes (`exa_cube_shadows`, `exa_gi`, `exa_post`) fill the
//! per-frame slots.
//!
//! Rules: `docs/RENDERING.md` (Blending, Supersampling).

/// One slot of the renderer's fixed frame walk, in the order the original
/// draws it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pass {
    /// The scene environment's sky, before any list.
    Sky,
    /// The alpha-tested class (`0x8000`), with depth writes.
    AlphaTest,
    /// The opaque class, with depth writes.
    Opaque,
    /// Hooks run once with depth writes still on.
    AfterOpaqueHooks,
    /// Depth writes go off for everything after this point.
    DepthWritesOff,
    /// The `0x10000` class, drawn after depth writes are switched off.
    DepthIndependent,
    /// A copy of the frame for refracting surfaces.
    RefractionCopy,
    /// The hook list for overlays on the copied frame.
    AfterCopyHooks,
    /// The blended class: one list per scene sector, or one list for a
    /// sectorless scene.
    Blended,
    /// The particles' depth pre-pass.
    ParticleDepth,
    /// The particles' colour, with depth writes off.
    ParticleColour,
    /// The closing hook list, then depth writes back on.
    EndHooks,
}

/// The frame's pass order.
pub const PASS_ORDER: [Pass; 12] = [
    Pass::Sky,
    Pass::AlphaTest,
    Pass::Opaque,
    Pass::AfterOpaqueHooks,
    Pass::DepthWritesOff,
    Pass::DepthIndependent,
    Pass::RefractionCopy,
    Pass::AfterCopyHooks,
    Pass::Blended,
    Pass::ParticleDepth,
    Pass::ParticleColour,
    Pass::EndHooks,
];

impl Pass {
    /// Whether the pass writes depth. The original draws both opaque
    /// classes before switching depth writes off; the particles' pre-pass
    /// switches them back on for its depth only.
    pub fn writes_depth(self) -> bool {
        matches!(
            self,
            Self::AlphaTest | Self::Opaque | Self::AfterOpaqueHooks | Self::ParticleDepth
        )
    }

    /// The Bevy phase or node that stands in for the pass. `None` marks a
    /// pass Exarust has no separate equivalent for.
    pub fn bevy_phase(self) -> Option<&'static str> {
        match self {
            Self::Sky => Some("the environment's sky camera"),
            Self::AlphaTest => Some("AlphaMask3d"),
            Self::Opaque => Some("Opaque3d"),
            Self::AfterOpaqueHooks => None,
            Self::DepthWritesOff => Some("Transparent3d setup"),
            Self::DepthIndependent => Some("Transparent3d"),
            Self::RefractionCopy => None,
            Self::AfterCopyHooks => None,
            Self::Blended => Some("Transparent3d"),
            Self::ParticleDepth => None,
            Self::ParticleColour => Some("Transparent3d"),
            Self::EndHooks => None,
        }
    }
}

/// One render-state list record.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderEntry {
    /// The material program key (a short in the original's state record).
    pub key: i16,
    /// The blend layer in the blend word's bits 4–7.
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
/// best, and within the same layer replaces the best only when the
/// candidate's opacity is strictly greater. The opacity starts at −1 for
/// each output slot and a NaN candidate never compares greater (the
/// unordered compare is skipped), so an entry whose opacity is NaN is picked
/// only by being the first of a lower layer. When nothing can be picked the
/// output ends early.
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
            if best_layer > entry.layer {
                best_layer = entry.layer;
                best = Some(i);
            } else if best_layer == entry.layer
                && best_opacity < entry.opacity
                && !entry.opacity.is_nan()
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
        assert_eq!(
            PASS_ORDER,
            [
                Pass::Sky,
                Pass::AlphaTest,
                Pass::Opaque,
                Pass::AfterOpaqueHooks,
                Pass::DepthWritesOff,
                Pass::DepthIndependent,
                Pass::RefractionCopy,
                Pass::AfterCopyHooks,
                Pass::Blended,
                Pass::ParticleDepth,
                Pass::ParticleColour,
                Pass::EndHooks,
            ]
        );
        // Depth writes are on for the two opaque classes and off from the
        // depth-independent class onward; the particle pre-pass is the one
        // exception.
        let depth_off = PASS_ORDER
            .iter()
            .position(|p| *p == Pass::DepthWritesOff)
            .unwrap();
        // `Sky` is a camera clear rather than a listed draw; the two
        // opaque classes before the switch write depth.
        assert!(!Pass::Sky.writes_depth());
        for pass in &PASS_ORDER[1..depth_off] {
            assert!(pass.writes_depth(), "{pass:?}");
        }
        for pass in &PASS_ORDER[depth_off + 1..] {
            assert_eq!(
                pass.writes_depth(),
                *pass == Pass::ParticleDepth,
                "{pass:?}"
            );
        }
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
}
