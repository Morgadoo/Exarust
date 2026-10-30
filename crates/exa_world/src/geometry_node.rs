//! A drawable scene node's state (`TGeometry`): the shadow-bias scales,
//! the per-part flags its activation derives from a material, the
//! displacement bound and the master/instance lists.
//!
//! The constructor starts a geometry node at class 9 with the shadow step
//! and bias at 1, the emissive scale at 1, the alpha threshold at 0.5 and
//! the sort key at −100 000. Activation resets the bound to zero, derives
//! the shadow step and bias from the option word's `0x80000` and `0x70000`
//! bits (in double precision, then narrowed), clears the node word's
//! `0x30000`, sets the option word's `0x10`, then for each part: sets the
//! part's flags from its material ([`PartFlags::of`]), raises the node's
//! `0x10000` when the part casts and the node is not external (`0x80000`),
//! clears the option word's `0x10` when the material has a blend class, and
//! raises the bound to the material's (ordered compares, so a NaN is not
//! taken). Then a node with any of `0x70000000` and `0x1000000` drops
//! `0x1000000` unless its parent is a geometry node; the threshold becomes
//! 1 with `0x8000000`, else 0.5; and a non-external node whose squared bound
//! exceeds the engine's epsilon gains `0x20000`.
//!
//! The material to part-flag mapping is the original's:
//!
//! - the part casts unless the material's shadow byte is zero;
//! - the part is alpha-tested by the shadow maps for the material class's
//!   `0x4000`, or an opacity map (`0x400000`) with an opacity above 0.75;
//! - the part's material word gets `0x40` when the material class's low
//!   nibble is set, and `0x80000` when the class is zero;
//! - the part's mode word starts at 1 and follows the material's extra
//!   flags (`2` → `2`, `0x10` → `0x10`, `4` → `8`, `0x100` → `4`), plus
//!   `0x40` when the node word has `0x8000`;
//! - the part culls back faces unless the material is two-sided (`0x10`).
//!
//! The per-part bits whose draw-side meaning is not established are kept
//! as the original stores them. Nothing in the running game builds a
//! [`GeometryState`] yet: Exarust's spawned models take their part flags
//! from `exa_assets` directly. Rules: `docs/RENDERING.md` (Surfaces).

/// The class code of a geometry node.
pub const CLASS_GEOMETRY: u32 = 9;
/// The option word's shadow bit: the step and bias are `1 + level·0.005`.
pub const SHADOW_ADD: u32 = 0x0008_0000;
/// The option word's shadow-level field (bits 16–18).
pub const SHADOW_LEVEL: u32 = 0x0007_0000;
/// The option word's bit activation sets and a blend-class material
/// clears.
pub const OPTION_NO_BLEND_CLASS: u32 = 0x10;
/// The node word's external-draw bit.
pub const EXTERNAL_DRAW: u32 = 0x0008_0000;
/// The node word's bound-present bit activation sets.
pub const HAS_BOUNDS: u32 = 0x0002_0000;
/// The node word's bit a casting part sets.
pub const CASTS_SHADOW: u32 = 0x0001_0000;
/// The node word's two bits activation clears first.
pub const NODE_CLASS_MASK: u32 = 0x0003_0000;
/// The node word's bit that selects the alpha threshold 1 (else 0.5).
pub const THRESHOLD_FULL: u32 = 0x0800_0000;
/// The node word's bit a part's mode word copies as `0x40`.
pub const SHADOW_ALL: u32 = 0x0000_8000;
/// The node word's bit activation drops under a non-geometry parent when
/// any of [`DROP_GROUP`] is set.
pub const PARENT_GEOMETRY_ONLY: u32 = 0x0100_0000;
/// The node word's bits that make [`PARENT_GEOMETRY_ONLY`] depend on the
/// parent.
pub const DROP_GROUP: u32 = 0x7000_0000;
/// The engine's epsilon the squared bound must exceed.
pub const BOUND_EPSILON: f32 = f32::from_bits(0x33ff_ffd7);
/// The part material word's blend-class bit.
pub const PART_BLEND_CLASS: u32 = 0x40;
/// The part material word's class-less bit.
pub const PART_NO_BLEND: u32 = 0x0008_0000;
/// The constructor's sort key.
pub const SORT_KEY_NONE: f32 = -100_000.0;
/// The constructor's shadow step and bias.
pub const DEFAULT_SHADOW: f32 = 1.0;
/// The constructor's emissive scale.
pub const DEFAULT_EMISSIVE: f32 = 1.0;
/// The constructor's alpha threshold.
pub const DEFAULT_THRESHOLD: f32 = 0.5;

/// The shadow-bias scales activation derives from the option word: `(step,
/// bias)`. With `0x80000` both are `1 + level·0.005`; without it the bias is
/// `1 − level·0.005` and the step is 0.01 below it (in double precision
/// from the narrowed bias); with no level both are 1. The level is bits
/// 16–18.
pub fn shadow_scales(word: u32) -> (f32, f32) {
    let level = ((word & SHADOW_LEVEL) >> 16) as f64;
    if word & SHADOW_ADD != 0 {
        let both = (level * 0.005 + 1.0) as f32;
        (both, both)
    } else if word & SHADOW_LEVEL != 0 {
        let bias = (1.0 - level * 0.005) as f32;
        let step = (f64::from(bias) - 0.01) as f32;
        (step, bias)
    } else {
        (DEFAULT_SHADOW, DEFAULT_SHADOW)
    }
}

/// One part's flags as activation derives them from its material.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PartFlags {
    /// The part's mode word.
    pub word: u32,
    /// The part's material word bits [`PART_BLEND_CLASS`] and
    /// [`PART_NO_BLEND`] (its other bits are the part's own).
    pub material_word: u32,
    /// The part is drawn into the shadow maps.
    pub casts: bool,
    /// The part is alpha-tested by the shadow maps.
    pub cuts: bool,
    /// The part culls its back faces.
    pub cull: bool,
    /// The material has a blend class (non-zero class word).
    pub has_class: bool,
    /// The runtime material's per-axis bound the node's bound takes the
    /// maximum of (its units are the material's; not traced further).
    pub bound: [f32; 3],
}

impl PartFlags {
    /// Derives a part's flags from its material's words. `class` is the
    /// material class word, `flags` its render flags, `extra` its extra
    /// flags, `opacity` its opacity, `casts_shadow` the material's shadow
    /// byte (false when zero) and `bound` its displacement bound.
    /// `node_word` is the geometry node's own word.
    pub fn of(
        class: u32,
        flags: u32,
        extra: u32,
        opacity: f32,
        casts_shadow: bool,
        bound: [f32; 3],
        node_word: u32,
    ) -> Self {
        let mut word = 1;
        if extra & 2 != 0 {
            word |= 2;
        }
        if extra & 0x10 != 0 {
            word |= 0x10;
        }
        if extra & 4 != 0 {
            word |= 8;
        }
        if extra & 0x100 != 0 {
            word |= 4;
        }
        if node_word & SHADOW_ALL != 0 {
            word |= 0x40;
        }
        let mut material_word = 0;
        if class & 0xF != 0 {
            material_word |= PART_BLEND_CLASS;
        }
        if class == 0 {
            material_word |= PART_NO_BLEND;
        }
        Self {
            word,
            material_word,
            casts: casts_shadow,
            // An ordered compare with 0.75: a NaN opacity does not cut.
            cuts: class & 0x4000 != 0 || (flags & 0x40_0000 != 0 && opacity > 0.75),
            cull: flags & 0x10 == 0,
            has_class: class != 0,
            bound,
        }
    }
}

/// The geometry node's state beside its parts.
#[derive(Debug, Clone, PartialEq)]
pub struct GeometryState {
    /// The node word.
    pub word: u32,
    /// The option word (shadow level and the blend-class bit).
    pub option: u32,
    /// Emissive scale.
    pub emissive: f32,
    /// Shadow step and bias.
    pub shadow: (f32, f32),
    /// The alpha threshold.
    pub threshold: f32,
    /// The parts' maximum bound.
    pub bounds: [f32; 3],
    /// The parts' material-derived flags, in part order.
    pub parts: Vec<PartFlags>,
    /// The master this node instances, if any.
    pub master: Option<usize>,
    /// The instances this node masters.
    pub instances: Vec<usize>,
}

impl Default for GeometryState {
    fn default() -> Self {
        Self::new()
    }
}

impl GeometryState {
    /// The constructor's state.
    pub fn new() -> Self {
        Self {
            word: 0,
            option: 0,
            emissive: DEFAULT_EMISSIVE,
            shadow: (DEFAULT_SHADOW, DEFAULT_SHADOW),
            threshold: DEFAULT_THRESHOLD,
            bounds: [0.0; 3],
            parts: Vec::new(),
            master: None,
            instances: Vec::new(),
        }
    }

    /// Activation, in the original's order (see the module notes).
    /// `parent_is_geometry` is whether the node's parent is a geometry
    /// node.
    pub fn activate(&mut self, parts: &[PartFlags], parent_is_geometry: bool) {
        self.bounds = [0.0; 3];
        self.shadow = shadow_scales(self.option);
        self.word &= !NODE_CLASS_MASK;
        self.option |= OPTION_NO_BLEND_CLASS;
        for part in parts {
            if part.casts && self.word & EXTERNAL_DRAW == 0 {
                self.word |= CASTS_SHADOW;
            }
            if part.has_class {
                self.option &= !OPTION_NO_BLEND_CLASS;
            }
            for (bound, &material) in self.bounds.iter_mut().zip(&part.bound) {
                // Ordered: a NaN material bound is not taken.
                if *bound < material {
                    *bound = material;
                }
            }
        }
        self.parts = parts.to_vec();
        if self.word & DROP_GROUP != 0
            && self.word & PARENT_GEOMETRY_ONLY != 0
            && !parent_is_geometry
        {
            self.word &= !PARENT_GEOMETRY_ONLY;
        }
        self.threshold = if self.word & THRESHOLD_FULL != 0 {
            1.0
        } else {
            DEFAULT_THRESHOLD
        };
        if self.word & EXTERNAL_DRAW == 0 && self.has_bounds() {
            self.word |= HAS_BOUNDS;
        }
    }

    /// Whether the node's squared bound exceeds the engine's epsilon,
    /// summed `(x² + y²) + z²` as the original.
    pub fn has_bounds(&self) -> bool {
        let [x, y, z] = self.bounds;
        z * z + (y * y + x * x) > BOUND_EPSILON
    }

    /// Appends an instance to this master's list, as the original does
    /// (no duplicate check).
    pub fn add_instance(&mut self, instance: usize) {
        self.instances.push(instance);
    }

    /// Removes an instance from the list by swapping the last entry into
    /// its place. Returns whether it was listed; the caller clears the
    /// master's has-instances bit when the list empties.
    pub fn remove_instance(&mut self, instance: usize) -> bool {
        let Some(at) = self.instances.iter().position(|&i| i == instance) else {
            return false;
        };
        self.instances.swap_remove(at);
        true
    }

    /// Whether this node is an instance rather than a master.
    pub fn is_instance(&self) -> bool {
        self.master.is_some()
    }
}

/// Hands a master role from `old` to `new` over a shared list of geometry
/// states, as the node class does when a master is deleted. Every entry of
/// the old list takes `new` as its master; the entry that is `new` itself
/// becomes `old` when `separate` is false, else the list's last entry. The
/// new master takes the old list (all of it, or without its last slot when
/// `separate`, and nothing when that leaves fewer than one), and the old
/// node's list empties.
pub fn hand_off_master(states: &mut [GeometryState], old: usize, new: usize, separate: bool) {
    let Some(mut list) = states.get(old).map(|s| s.instances.clone()) else {
        return;
    };
    if new >= states.len() {
        return;
    }
    let last = list.last().copied();
    for slot in list.iter_mut() {
        let entry = *slot;
        if entry != new
            && let Some(state) = states.get_mut(entry)
        {
            state.master = Some(new);
        }
        if entry == new {
            *slot = if separate { last.unwrap_or(new) } else { old };
        }
    }
    let new_list = if !separate {
        list
    } else if list.len() < 2 {
        Vec::new()
    } else {
        list.truncate(list.len() - 1);
        list
    };
    states[new].master = None;
    states[new].instances = new_list;
    states[old].instances.clear();
    states[old].master = Some(new);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn part(class: u32, casts: bool, bound: [f32; 3]) -> PartFlags {
        PartFlags::of(class, 0, 0, 1.0, casts, bound, 0)
    }

    #[test]
    fn the_shadow_scales_follow_the_option_word() {
        assert_eq!(shadow_scales(0), (1.0, 1.0));
        // Level 4, additive: 4·0.005 + 1 = 1.02 in double, narrowed.
        assert_eq!(shadow_scales(0x0008_0000 | 4 << 16), (1.02, 1.02));
        // Level 4, subtractive: bias 0.98, step 0.98 (narrowed) − 0.01.
        let (step, bias) = shadow_scales(4 << 16);
        assert_eq!(bias, 0.98);
        assert_eq!(step, (f64::from(0.98f32) - 0.01) as f32);
    }

    #[test]
    fn activation_sets_the_scales_bits_threshold_and_bound() {
        let mut g = GeometryState::new();
        assert_eq!((g.emissive, g.threshold, g.shadow), (1.0, 0.5, (1.0, 1.0)));
        g.option = 3 << 16;
        g.activate(&[], false);
        assert_eq!(g.shadow, shadow_scales(3 << 16));
        assert_eq!(g.option & OPTION_NO_BLEND_CLASS, OPTION_NO_BLEND_CLASS);
        // A casting part sets the node's cast bit unless the node is
        // external; a part with a class clears the option bit; the bound is
        // the parts' per-axis maximum from zero.
        g.word = 0;
        g.activate(
            &[
                part(3, true, [0.2, -1.0, 0.1]),
                part(0, false, [0.1, 0.0, 0.3]),
            ],
            false,
        );
        assert_eq!(g.word & CASTS_SHADOW, CASTS_SHADOW);
        assert_eq!(g.option & OPTION_NO_BLEND_CLASS, 0);
        assert_eq!(g.bounds, [0.2, 0.0, 0.3]);
        assert_eq!(g.word & HAS_BOUNDS, HAS_BOUNDS);
        // Only class-less parts keep the option bit.
        g.activate(&[part(0, false, [0.0; 3])], false);
        assert_eq!(g.option & OPTION_NO_BLEND_CLASS, OPTION_NO_BLEND_CLASS);
        g.word = EXTERNAL_DRAW;
        g.activate(&[part(3, true, [0.2, 0.0, 0.0])], false);
        assert_eq!(g.word & (CASTS_SHADOW | HAS_BOUNDS), 0);
        // A NaN material bound is not taken.
        g.word = 0;
        g.activate(&[part(3, false, [f32::NAN, 0.0, 0.0])], false);
        assert_eq!(g.bounds, [0.0; 3]);
        assert_eq!(g.word & HAS_BOUNDS, 0);
        // A squared bound at the epsilon does not set the bit:
        // 0.000_345² ≈ 1.19e-7 just under it.
        g.activate(&[part(3, false, [0.000_345, 0.0, 0.0])], false);
        assert_eq!(g.word & HAS_BOUNDS, 0);
        g.activate(&[part(3, false, [0.000_346, 0.0, 0.0])], false);
        assert_eq!(g.word & HAS_BOUNDS, HAS_BOUNDS);
        // The threshold follows 0x8000000.
        g.word = THRESHOLD_FULL;
        g.activate(&[], false);
        assert_eq!(g.threshold, 1.0);
        g.word = 0;
        g.activate(&[], false);
        assert_eq!(g.threshold, 0.5);
        // 0x1000000 needs a geometry parent when a 0x70000000 bit is set.
        g.word = 0x1000_0000 | PARENT_GEOMETRY_ONLY;
        g.activate(&[], false);
        assert_eq!(g.word & PARENT_GEOMETRY_ONLY, 0);
        g.word = 0x1000_0000 | PARENT_GEOMETRY_ONLY;
        g.activate(&[], true);
        assert_eq!(g.word & PARENT_GEOMETRY_ONLY, PARENT_GEOMETRY_ONLY);
        g.word = PARENT_GEOMETRY_ONLY;
        g.activate(&[], false);
        assert_eq!(g.word & PARENT_GEOMETRY_ONLY, PARENT_GEOMETRY_ONLY);
    }

    #[test]
    fn a_part_takes_its_flags_from_the_material_words() {
        let p = PartFlags::of(
            0x4003,
            0x40_0000 | 0x10,
            0x10 | 4 | 0x100,
            0.8,
            true,
            [0.0; 3],
            0x8000,
        );
        assert!(p.casts && p.cuts && p.has_class && !p.cull);
        assert_eq!(p.material_word, PART_BLEND_CLASS);
        assert_eq!(p.word, 1 | 0x10 | 8 | 4 | 0x40);
        assert_eq!(PartFlags::of(2, 0, 2, 0.0, false, [0.0; 3], 0).word, 1 | 2);
        let none = PartFlags::of(0, 0, 0, 0.0, false, [0.0; 3], 0);
        assert_eq!(none.material_word, PART_NO_BLEND);
        assert!(none.cull && !none.has_class && !none.casts);
        // A class with only high bits is neither blend-class nor class-less.
        assert_eq!(part(0x4000, false, [0.0; 3]).material_word, 0);
        let cut = |opacity| PartFlags::of(3, 0x40_0000, 0, opacity, false, [0.0; 3], 0).cuts;
        assert!(!cut(0.75));
        assert!(!cut(f32::NAN));
        assert!(cut(0.750_001));
        assert!(!PartFlags::of(3, 0, 0, 0.9, false, [0.0; 3], 0).cuts);
    }

    #[test]
    fn a_master_hands_its_role_to_an_instance() {
        let mut states = vec![GeometryState::new(); 5];
        states[0].instances = vec![1, 2];
        hand_off_master(&mut states, 0, 1, false);
        assert!(states[0].instances.is_empty());
        assert_eq!(states[1].master, None);
        assert_eq!(states[1].instances, vec![0, 2]);
        assert_eq!(states[2].master, Some(1));
        assert_eq!(states[0].master, Some(1));
        // A separate hand-off swaps the last entry into the new master's
        // slot and drops the last slot: [3, 1, 2, 4] → [3, 4, 2].
        let mut states = vec![GeometryState::new(); 5];
        states[0].instances = vec![3, 1, 2, 4];
        hand_off_master(&mut states, 0, 1, true);
        assert_eq!(states[1].instances, vec![3, 4, 2]);
        assert_eq!(states[4].master, Some(1));
        assert!(states[0].instances.is_empty());
        // Alone in the list, a separate hand-off leaves no instances.
        let mut states = vec![GeometryState::new(); 2];
        states[0].instances = vec![1];
        hand_off_master(&mut states, 0, 1, true);
        assert!(states[1].instances.is_empty());
        // When the new master is not listed the list is copied unchanged;
        // separate still drops the last slot.
        let mut states = vec![GeometryState::new(); 5];
        states[0].instances = vec![2, 3];
        hand_off_master(&mut states, 0, 1, false);
        assert_eq!(states[1].instances, vec![2, 3]);
        let mut states = vec![GeometryState::new(); 5];
        states[0].instances = vec![2, 3, 4];
        hand_off_master(&mut states, 0, 1, true);
        assert_eq!(states[1].instances, vec![2, 3]);
        assert_eq!(states[4].master, Some(1));
    }

    #[test]
    fn instances_append_and_swap_out() {
        let mut g = GeometryState::new();
        g.add_instance(3);
        g.add_instance(4);
        g.add_instance(5);
        assert_eq!(g.instances, [3, 4, 5]);
        assert!(g.remove_instance(3));
        assert_eq!(g.instances, [5, 4]);
        assert!(!g.remove_instance(3));
    }
}
