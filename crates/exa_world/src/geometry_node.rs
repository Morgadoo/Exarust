//! A drawable scene node's state (`TGeometry`): the emissive and fade scales,
//! the shadow-bias scales, the per-part flags its activation derives from a
//! material, and the master/instance hand-off.
//!
//! The constructor starts a geometry node at class 9 with emissive and fade
//! scales 1, shadow scales 1/1 and bounds reset to `-100000`. Activation
//! derives the shadow step and bias from the node word's `0x80000` and
//! `0x70000` bits (in double precision, then narrowed), sets every part's
//! flags from its material, takes the parts' maximum displacement bounds and
//! marks the node external when its bounds are non-zero without the external
//! bit. A flagged instance deep-copies the master's parts; an unflagged one
//! shares them, and a master hands its role to an instance when it is deleted.
//!
//! The material to part-flag mapping is the original's:
//!
//! - cast (part byte `0x109`) is set unless the material's no-shadow byte is;
//! - the alpha-tested cut (`0x10A`) is the material class's `0x4000`, or an
//!   opacity map (`0x400000`) with an opacity above 0.75;
//! - the part word's `0x40` follows the material class's low nibble; its
//!   `0x80000` marks a material with no blend class;
//! - the part follows the material's flag byte (`0x204` bits 2, `0x10`, 4,
//!   `0x100`) into its own `0x1C` word, plus `0x40` when the node's own
//!   `0x8000` bit is set;
//! - the part's cull byte (`0x108`) is set unless the material is two-sided
//!   (`0x10`).
//!
//! The per-part flag bits whose draw-side meaning is not established are
//! kept as the original stores them. Rules: `docs/RENDERING.md` (Surfaces).

/// The class code of a geometry node.
pub const CLASS_GEOMETRY: u32 = 9;
/// The node word's shadow-fields bit: the step and bias are added to one.
pub const SHADOW_ADD: u32 = 0x0008_0000;
/// The node word's shadow-level field.
pub const SHADOW_LEVEL: u32 = 0x0007_0000;
/// The node word's external-draw bit.
pub const EXTERNAL_DRAW: u32 = 0x0008_0000;
/// The node word's bounds-non-zero bit the activation sets.
pub const HAS_BOUNDS: u32 = 0x0002_0000;
/// The node word's bit that selects the 0.5/1 alpha threshold.
pub const THRESHOLD_FULL: u32 = 0x0800_0000;
/// The smallest non-zero squared bound the activation marks as present.
pub const BOUND_EPSILON: f32 = 1.192_089_98e-7;
/// The node word's class word the activation clears (`0x30000`).
pub const NODE_CLASS_MASK: u32 = 0x0003_0000;
/// The node word's per-part shadow flag.
pub const SHADOW_PART: u32 = 0x0001_0000;
/// The node word's all-shadow flag.
pub const SHADOW_ALL: u32 = 0x0000_8000;
/// A part is cast by the shadow maps (`+0x109`).
pub const PART_CASTS: u8 = 1;
/// A part is alpha-tested by the shadow maps (`+0x10A`).
pub const PART_CUTS: u8 = 2;
/// The part word's blend-class-present bit.
pub const PART_BLEND_CLASS: u32 = 0x40;
/// The part word's no-blend-class bit.
pub const PART_NO_BLEND: u32 = 0x0008_0000;
/// The part word's double-sided bit: clear when the material is two-sided.
pub const PART_CULL: u32 = 0x100;
/// The constructor's reset sort key.
pub const SORT_KEY_NONE: f32 = -100_000.0;
/// The constructor's shadow step and bias.
pub const DEFAULT_SHADOW: f32 = 1.0;
/// The constructor's emissive and fade scales.
pub const DEFAULT_SCALE: f32 = 1.0;
/// The constructor's alpha threshold (the original's `+0x22C`).
pub const DEFAULT_THRESHOLD: f32 = 0.5;

/// The shadow-bias scales activation derives from the node word: `(step,
/// bias)`. With `0x80000` both are `1 + level·0.005`; without it the bias is
/// `1 − level·0.005` and the step is 0.01 below it; with no level both are 1.
/// The level is bits 16–18.
pub fn shadow_scales(word: u32) -> (f32, f32) {
    let level = ((word & SHADOW_LEVEL) >> 16) as f64;
    if word & SHADOW_ADD != 0 {
        let both = (1.0 + level * 0.005) as f32;
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PartFlags {
    /// The part's mode word (`+0x1C`).
    pub word: u32,
    /// The cast byte (`+0x109`).
    pub casts: bool,
    /// The alpha-tested cut byte (`+0x10A`).
    pub cuts: bool,
    /// The material's blend class is present (`+0xF0` bit `0x40`).
    pub blend_class: bool,
    /// The material has no blend class (`+0xF0` bit `0x80000`).
    pub no_blend: bool,
    /// The part is culled by its back faces (`+0x108`).
    pub cull: bool,
}

impl PartFlags {
    /// Derives a part's flags from its material's words. `class` is the
    /// material class word (`+0x1CC`), `flags` its render flags (`+0x1D0`),
    /// `extra` its extra flags (`+0x204`), `opacity` its opacity, and
    /// `casts_shadow` the material's shadow byte (0 when it casts none).
    /// `node_word` is the geometry node's own `+0xC` word for the shadow-mode
    /// bit.
    pub fn of(
        class: u32,
        flags: u32,
        extra: u32,
        opacity: f32,
        casts_shadow: bool,
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
        Self {
            word,
            casts: casts_shadow,
            // The original compares the material's opacity with 0.75 with
            // an ordered compare, so a NaN does not cut.
            cuts: class & 0x4000 != 0 || (flags & 0x40_0000 != 0 && opacity > 0.75),
            blend_class: class & 0xF != 0,
            no_blend: class == 0,
            cull: flags & 0x10 == 0,
        }
    }

    /// The part word with the blend class cleared and the no-blend bit set,
    /// as activation leaves a class-less material's part.
    pub fn without_blend_class(mut self) -> Self {
        self.blend_class = false;
        self.no_blend = true;
        self.word = (self.word & !PART_BLEND_CLASS) | PART_NO_BLEND;
        self
    }
}

/// The geometry node's state beside its parts.
#[derive(Debug, Clone, PartialEq)]
pub struct GeometryState {
    /// The node word (`+0xC`).
    pub word: u32,
    /// The option word (`+0x204`).
    pub option: u32,
    /// Emissive scale (`+0x244`).
    pub emissive: f32,
    /// Fade scale (`+0x22C`).
    pub fade: f32,
    /// Shadow step and bias (`+0x224`, `+0x228`).
    pub shadow: (f32, f32),
    /// The alpha threshold (`+0x22C`).
    pub threshold: f32,
    /// The parts' maximum bound (`+0x1F8..0x200`).
    pub bounds: [f32; 3],
    /// The first part's material-derived flags.
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
            emissive: DEFAULT_SCALE,
            fade: DEFAULT_SCALE,
            shadow: (DEFAULT_SHADOW, DEFAULT_SHADOW),
            threshold: DEFAULT_THRESHOLD,
            bounds: [0.0; 3],
            parts: Vec::new(),
            master: None,
            instances: Vec::new(),
        }
    }

    /// Activation: derives the shadow scales, clears the node's class bits
    /// and takes the parts' maximum bound. The caller supplies each part's
    /// [`PartFlags`] and the bound the parts' materials give.
    pub fn activate(&mut self, parts: &[PartFlags], bounds: [f32; 3]) {
        self.shadow = shadow_scales(self.option);
        self.word &= !NODE_CLASS_MASK;
        self.option |= 0x10;
        self.bounds = bounds;
        self.parts = parts.to_vec();
        let any_cast = self.parts.iter().any(|part| part.casts);
        if any_cast && self.word & EXTERNAL_DRAW == 0 {
            self.word |= SHADOW_PART;
        }
        self.fade = if self.word & THRESHOLD_FULL != 0 {
            1.0
        } else {
            DEFAULT_THRESHOLD
        };
        if self.word & EXTERNAL_DRAW == 0 && self.has_bounds() {
            self.word |= HAS_BOUNDS;
        }
    }

    /// Sets the raw part flags, as activation does before the per-part
    /// fields are computed.
    pub fn with_parts(mut self, parts: Vec<PartFlags>) -> Self {
        self.parts = parts;
        self
    }

    /// Whether the node's bounds are non-zero.
    pub fn has_bounds(&self) -> bool {
        self.bounds[0] * self.bounds[0]
            + self.bounds[1] * self.bounds[1]
            + self.bounds[2] * self.bounds[2]
            > 0.0
    }

    /// Adds an instance to this node's list (`shared` only affects the
    /// caller's deep-copy decision).
    pub fn add_instance(&mut self, instance: usize) {
        if !self.instances.contains(&instance) {
            self.instances.push(instance);
        }
    }

    /// Removes an instance from the list, clearing the master when the list
    /// becomes empty.
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
/// states, as the node class does when a master is deleted. `separate`
/// makes the new node an instance with its own parts (the old master's
/// remaining part is not copied); otherwise the new node becomes the master
/// and takes the old instance list. The old node's list empties either way.
pub fn hand_off_master(states: &mut [GeometryState], old: usize, new: usize, separate: bool) {
    let Some(old_instances) = states.get(old).map(|s| s.instances.clone()) else {
        return;
    };
    if new >= states.len() {
        return;
    }
    let new_instances = if !separate {
        let mut list: Vec<usize> = old_instances
            .iter()
            .copied()
            .filter(|&i| i != new)
            .collect();
        list.retain(|&i| i != old);
        list.push(old);
        list
    } else if old_instances.len() <= 1 {
        Vec::new()
    } else {
        let mut kept: Vec<usize> = old_instances
            .iter()
            .copied()
            .filter(|&i| i != new)
            .collect();
        kept.truncate(old_instances.len() - 1);
        kept
    };
    for &instance in &new_instances {
        if instance != new
            && let Some(i) = states.get_mut(instance)
        {
            i.master = Some(new);
        }
    }
    states[new].master = None;
    states[new].instances = new_instances;
    states[old].instances.clear();
    states[old].master = Some(new);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shadow_scales_follow_the_node_word() {
        assert_eq!(shadow_scales(0), (1.0, 1.0));
        // Level 4, additive: 1 + 4·0.005 = 1.02.
        assert_eq!(shadow_scales(0x0008_0000 | 4 << 16), (1.02, 1.02));
        // Level 4, subtractive: bias 0.98, step 0.97.
        let (step, bias) = shadow_scales(4 << 16);
        assert_eq!(bias, 0.98);
        assert_eq!(step, (0.98f64 - 0.01) as f32);
    }

    #[test]
    fn the_constructor_and_activation_set_the_scales() {
        let mut g = GeometryState::new();
        assert_eq!((g.emissive, g.fade), (1.0, 1.0));
        assert_eq!(g.threshold, 0.5);
        g.option = 3 << 16;
        g.activate(&[], [0.0; 3]);
        assert_eq!(g.shadow, shadow_scales(3 << 16));
        assert_eq!(g.option & 0x10, 0x10);
        // A casting part sets the node's shadow bit unless the node is
        // external; a non-zero bound sets the bounds bit.
        let cast = PartFlags::of(0x3, 0, 0, 1.0, true, 0);
        g.word = 0;
        g.activate(&[cast], [0.2, 0.0, 0.0]);
        assert_eq!(g.word & SHADOW_PART, SHADOW_PART);
        assert_eq!(g.word & HAS_BOUNDS, HAS_BOUNDS);
        g.word = EXTERNAL_DRAW;
        g.activate(&[cast], [0.2, 0.0, 0.0]);
        assert_eq!(g.word & SHADOW_PART, 0);
        assert_eq!(g.word & HAS_BOUNDS, 0);
    }

    #[test]
    fn a_part_takes_its_flags_from_the_material_words() {
        let p = PartFlags::of(
            0x4003,
            0x40_0000 | 0x10,
            0x10 | 4 | 0x100,
            0.8,
            true,
            0x8000,
        );
        assert!(p.casts);
        assert!(p.cuts);
        assert!(p.blend_class);
        assert!(!p.no_blend);
        assert!(!p.cull);
        assert_eq!(p.word, 1 | 0x10 | 8 | 4 | 0x40);
        assert!(!PartFlags::of(0x3, 0, 0, 0.0, false, 0).casts);
        assert!(PartFlags::of(0x3, 0, 0, 0.0, true, 0).casts);
        assert!(!PartFlags::of(0x3, 0x40_0000, 0, 0.75, false, 0).cuts);
        assert!(!PartFlags::of(0x3, 0x40_0000, 0, f32::NAN, false, 0).cuts);
        assert!(PartFlags::of(0x3, 0x40_0000, 0, 0.750_001, false, 0).cuts);
        assert_eq!(
            PartFlags::of(0, 0, 0, 0.0, false, 0)
                .without_blend_class()
                .word,
            PART_NO_BLEND | 1
        );
    }

    #[test]
    fn a_master_hands_its_role_to_an_instance() {
        let mut states = vec![GeometryState::new(); 4];
        states[0].instances = vec![1, 2];
        states[1].master = Some(0);
        states[2].master = Some(0);
        hand_off_master(&mut states, 0, 1, false);
        assert!(states[0].instances.is_empty());
        assert_eq!(states[1].master, None);
        assert_eq!(states[1].instances, vec![2, 0]);
        assert_eq!(states[2].master, Some(1));
        assert_eq!(states[0].master, Some(1));
        // A separate hand-off leaves the new node a master of the rest.
        let mut states = vec![GeometryState::new(); 4];
        states[0].instances = vec![1, 2];
        hand_off_master(&mut states, 0, 1, true);
        assert_eq!(states[1].instances, vec![2]);
        assert!(states[0].instances.is_empty());
        assert_eq!(states[0].master, Some(1));
    }

    #[test]
    fn instances_are_added_and_removed_once() {
        let mut g = GeometryState::new();
        g.add_instance(3);
        g.add_instance(3);
        assert_eq!(g.instances, [3]);
        assert!(g.remove_instance(3));
        assert!(!g.remove_instance(3));
        assert!(g.instances.is_empty());
    }
}
