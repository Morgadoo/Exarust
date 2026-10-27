//! The original's terrain composite: the texture sets it bakes a 6 m block
//! into, the pass keys and uniforms that bake it, and the selection that
//! decides which block is rebaked each frame.
//!
//! The original renders, per 6 m block, a colour, tangent-space normal, gloss
//! and half-size parallax-height texture from the palette's layers, four
//! layers to a pass with additive extra passes ([`PassKey`]). Near blocks
//! bake into a 1024² set and farther ones into a 512² set, from pools of 21
//! and 43 sets ([`CompositePools`]); the blocks are chosen around a look-ahead
//! point ([`CompositeSelector`]). Exarust evaluates the same layer stacking
//! per pixel in `terrain.wesl`, so the sets and passes have no separate GPU
//! consumer; the numbers, orders and state transitions below are the
//! original's, pinned by tests.
//!
//! Evidence: REA on the compositor driver, its mask and height passes, the
//! set allocator, the block selector and the frame hook. `docs/RENDERING.md`
//! (Terrain) records the pass table and `docs/formats/TERRAIN.md` the
//! palette fields.

use bevy::math::Vec2;
use exa_formats::terrain::{TerrainMaterial as PaletteMaterial, TerrainTexture};
use exa_physics::engine_math::round_to_i64;

/// Side of a composite texture set at level 0 (the original's stored 32).
pub const TEXTURE_BASE: u32 = 32;
/// High-resolution sets in the pool (21).
pub const HIGH_SETS: usize = 21;
/// Low-resolution sets in the pool (43).
pub const LOW_SETS: usize = 43;
/// Blocks the selector keeps in its list (189).
pub const SELECTED_LIMIT: usize = 189;
/// Blocks along a 48 m tile's side (8).
pub const BLOCK_SIDE: u32 = 8;
/// The tile's block level (3): the selector and `TexPos` use it.
pub const TILE_LEVEL: u32 = 3;
/// A block's side in cells (30), the composite's mask upload span.
pub const BLOCK_CELLS: u32 = 30;
/// The mask upload's side in texels (32, one 256-texel atlas split in eighths).
pub const MASK_UPLOAD: u32 = 32;
/// The palette scale the compositor's quads use (the original's `4`).
pub const PALETTE_SCALE: f32 = 4.0;
/// The distance (squared centimetres) below which a block is high resolution:
/// 15 m.
pub const NEAR_D2: f32 = 2_250_000.0;
/// The distance (squared centimetres) past which a block gets no composite:
/// 44.4 m.
pub const FAR_D2: f32 = 19_713_600.0;
/// The detail texture the terrain tries to load once. The shipped install
/// has no such entry, and no pass reads the handle even when it exists.
pub const DETAIL_TEXTURE: &str = "trndtl01d.rfi";
/// The default composite ring the selection starts from: the priority base
/// with no visible block.
pub const RING_START: i32 = 6000;
/// A block with no light in reach ranks at this many centimetres.
const NO_LIGHT_DISTANCE: f32 = 65_000.0;
/// Blocks without a light rank above the visible ones by this much.
const LIGHT_SORT_BIAS: u32 = 0x1_0000;
/// The largest visible-block sort key (0xffff).
const SORT_LIMIT: u32 = 0xffff;
/// The width of the original's scratch mask atlas (2 MB).
pub const MASK_ATLAS_BYTES: usize = 0x20_0000;
/// The blur kernel's radius, in terrain vertices (the stored 120).
pub const KERNEL_RADIUS: usize = 120;
/// The noise maps' side (480 vertices, shared with the blend rules).
pub const NOISE_SIDE: usize = 480;

/// The original's integer log2 helper: `floor(log2(value))`, 0 for 0 and 1.
pub fn floor_log2(mut value: u32) -> i32 {
    let mut result = 0;
    while value > 1 {
        value >>= 1;
        result += 1;
    }
    result
}

/// The detail texture's side from the texture-detail setting: 4096 below 3,
/// 8192 at 3 and above (the terrain config record's word).
pub fn detail_size(texture_detail: i32) -> u32 {
    if texture_detail < 3 { 4096 } else { 8192 }
}

/// The high-resolution composite level from the detail texture's side:
/// `log2(size / 8) − 5`. 8192 gives 5 (a 1024² colour set); 4096 gives 4.
pub fn high_level(detail_size: u32) -> i32 {
    floor_log2(detail_size >> 3) - 5
}

/// The side of a set's colour, normal and gloss targets at `level`.
pub fn set_size(level: i32) -> u32 {
    if !(0..=15).contains(&level) {
        return 0;
    }
    TEXTURE_BASE << level
}

/// A composite set's four targets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompositeTarget {
    Colour,
    Normal,
    Gloss,
    Displacement,
}

impl CompositeTarget {
    /// The target's side at `level`; the displacement target is half size.
    pub fn size(self, level: i32) -> u32 {
        let size = set_size(level);
        match self {
            Self::Displacement => size >> 1,
            _ => size,
        }
    }

    /// The target's channel count (RGB8, RG8, R8 and R8).
    pub fn channels(self) -> u32 {
        match self {
            Self::Colour => 3,
            Self::Normal => 2,
            Self::Gloss | Self::Displacement => 1,
        }
    }

    /// The target's bytes per texel.
    pub fn bytes_per_texel(self) -> u32 {
        self.channels()
    }
}

/// Which composite passes the compositor runs. All three are on at the
/// shipped settings; a pass that is off has no target and no program bit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompositeFeatures {
    pub normal: bool,
    pub gloss: bool,
    pub displacement: bool,
}

impl Default for CompositeFeatures {
    fn default() -> Self {
        Self {
            normal: true,
            gloss: true,
            displacement: true,
        }
    }
}

/// One composite set: the original's record of its level and four target
/// texture ids.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompositeSet {
    /// Pool id, 0..=63 (0..=20 high, 21..=63 low).
    pub id: u32,
    pub level: i32,
    pub normal: bool,
    pub gloss: bool,
    pub displacement: bool,
}

impl CompositeSet {
    /// The set's texture ids, in the original's allocation order: colour,
    /// normal, gloss, displacement (a target the features turn off is absent).
    pub fn targets(&self) -> Vec<CompositeTarget> {
        let mut out = vec![CompositeTarget::Colour];
        if self.normal {
            out.push(CompositeTarget::Normal);
        }
        if self.gloss {
            out.push(CompositeTarget::Gloss);
        }
        if self.displacement {
            out.push(CompositeTarget::Displacement);
        }
        out
    }

    /// A target's side in this set.
    pub fn target_size(&self, target: CompositeTarget) -> u32 {
        target.size(self.level)
    }
}

/// The two composite pools: 21 sets at the high level (0..=20) and 43 at the
/// low level (21..=63), handed out from the ends of their free lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompositePools {
    sets: Vec<CompositeSet>,
    high_free: Vec<u32>,
    low_free: Vec<u32>,
}

impl CompositePools {
    /// Builds the pools from the detail texture's side, as the original's
    /// pool allocator does. The detail size must be a power of two; a zero
    /// high level gives zero-sized sets, which callers never take.
    pub fn new(detail_size: u32, features: CompositeFeatures) -> Self {
        let high = high_level(detail_size);
        let low = high - 1;
        let mut sets = Vec::with_capacity(HIGH_SETS + LOW_SETS);
        let mut high_free = Vec::with_capacity(HIGH_SETS);
        let mut low_free = Vec::with_capacity(LOW_SETS);
        for id in 0..(HIGH_SETS + LOW_SETS) as u32 {
            let is_high = (id as usize) < HIGH_SETS;
            sets.push(CompositeSet {
                id,
                level: if is_high { high } else { low },
                normal: features.normal,
                gloss: features.gloss,
                displacement: features.displacement,
            });
            if is_high {
                high_free.push(id);
            } else {
                low_free.push(id);
            }
        }
        Self {
            sets,
            high_free,
            low_free,
        }
    }

    /// The high-resolution level.
    pub fn high_level(&self) -> i32 {
        self.sets.first().map_or(0, |s| s.level)
    }

    /// The low-resolution level.
    pub fn low_level(&self) -> i32 {
        self.sets
            .get(HIGH_SETS)
            .map_or(self.high_level() - 1, |s| s.level)
    }

    /// A set by pool id.
    pub fn set(&self, id: u32) -> Option<&CompositeSet> {
        self.sets.get(id as usize)
    }

    /// The high sets still free.
    pub fn high_free(&self) -> usize {
        self.high_free.len()
    }

    /// The low sets still free.
    pub fn low_free(&self) -> usize {
        self.low_free.len()
    }

    /// Takes a high set from the end of its free list.
    pub fn take_high(&mut self) -> Option<u32> {
        self.high_free.pop()
    }

    /// Takes a low set from the end of its free list.
    pub fn take_low(&mut self) -> Option<u32> {
        self.low_free.pop()
    }

    /// Hands a high set back.
    pub fn give_high(&mut self, id: u32) {
        if (id as usize) < HIGH_SETS && !self.high_free.contains(&id) {
            self.high_free.push(id);
        }
    }

    /// Hands a low set back.
    pub fn give_low(&mut self, id: u32) {
        if (id as usize) >= HIGH_SETS
            && (id as usize) < HIGH_SETS + LOW_SETS
            && !self.low_free.contains(&id)
        {
            self.low_free.push(id);
        }
    }
}

/// The per-material mask plane's texels: the original keeps 256 × 256 bytes
/// per material channel.
pub const MASK_PLANE_TEXELS: usize = 256 * 256;

/// The mask/height upload's texel offset for block (`bx`, `by`) in the
/// 256-wide per-material plane: `(by·256 + bx)·30`, the block's 30-cell
/// stride.
pub fn mask_texel_offset(bx: u32, by: u32) -> usize {
    ((by * 256 + bx) * BLOCK_CELLS) as usize
}

/// The per-level working framebuffers and textures the compositor allocates
/// once, before any set: two framebuffers with depth at half
/// and full size, two without depth at half and full size, a mask framebuffer
/// at a quarter, and four 32 × 32 scratch textures of one to four channels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LevelTargets {
    /// The half-size framebuffer with a depth buffer, `32 << (level − 1)`.
    pub depth_half: u32,
    /// The full-size framebuffer with a depth buffer, `32 << level`.
    pub depth_full: u32,
    /// The half-size framebuffer without depth.
    pub plain_half: u32,
    /// The full-size framebuffer without depth.
    pub plain_full: u32,
    /// The mask framebuffer, `32 << (level − 2)`.
    pub mask: u32,
    /// The working textures' side (32) and channel counts (1, 2, 3, 4).
    pub scratch_side: u32,
    pub scratch_channels: [u32; 4],
}

impl LevelTargets {
    /// The targets of composite `level`.
    pub fn new(level: i32) -> Self {
        let half = level - 1;
        let quarter = level - 2;
        let at = |level: i32| {
            if (0..=15).contains(&level) {
                TEXTURE_BASE << level
            } else {
                0
            }
        };
        Self {
            depth_half: at(half),
            depth_full: at(level),
            plain_half: at(half),
            plain_full: at(level),
            mask: at(quarter),
            scratch_side: TEXTURE_BASE,
            scratch_channels: [1, 2, 3, 4],
        }
    }
}

/// The original's texture filter word: the initial `0x30` in both composite
/// filter slots.
pub const FILTER_WORD: u32 = 0x30;
/// The filter word's mipmap-generation bit. The composites have no mipmaps
/// at the shipped filter word.
pub const MIPMAP_BIT: u32 = 0x40;

/// Whether a filter word asks the compositor to generate mipmaps.
pub fn mipmaps(filter_word: u32) -> bool {
    filter_word & MIPMAP_BIT != 0
}

/// What a block is baked into: high or low resolution, or nothing yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resolution {
    None,
    Low,
    High,
}

impl Resolution {
    /// The resolution of a set level against the high level.
    pub fn of(level: i32, high: i32) -> Self {
        if level == high {
            Self::High
        } else if level == high - 1 {
            Self::Low
        } else {
            Self::None
        }
    }
}

/// The wet-floor source's threshold: a palette above this turns the term on
/// (the sewer's 0.5).
pub const WET_FLOOR_THRESHOLD: f32 = 0.4;
/// The terrain draw's diffuse factor (`MatDif`).
pub const MAT_DIF: f32 = 0.75;
/// The wet-floor source's offset into the draw's specular level
/// (`MatSpc.w`, unused while the gloss pass is on).
pub const SPECULAR_LEVEL_OFFSET: f32 = 0.3;
/// The terrain's Fresnel F0 (the lit program's constant).
pub const FRESNEL_F0: f32 = 0.04;
/// The parallax scale in 6 m block units (the lit program's constant).
pub const PARALLAX_SCALE: f32 = 0.007;

/// The depth-only shadow draw: all visible
/// blocks or one. The draw needs the block's bounds, its shadow-bias level and
/// the shadow map's viewport; Exarust's Bevy shadow pass performs the same
/// depth-only draw, so no separate builder is needed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowDraw {
    /// The block's bounds (centre and half extents, metres).
    pub centre: bevy::math::Vec3,
    pub half_extents: bevy::math::Vec3,
    /// The floor shadow-bias level the draw applies.
    pub shadow_bias: u32,
    /// The shadow cascade's viewport, when the caller draws one block.
    pub viewport: Option<[u32; 4]>,
}

/// The height pass's clear value (the original clears its height target to
/// 0.5 in every channel).
pub const HEIGHT_CLEAR: f32 = 0.5;
/// The height pass's opacity boost (`min(1.5·a, 1)`).
pub const HEIGHT_OPACITY_BOOST: f32 = 1.5;

/// The targets a set copy writes, with the source's and the target set's
/// sides: the original binds each target of the destination framebuffer,
/// attaches the source's same target and draws a full-screen quad
/// A target the features disable is skipped.
pub fn copy_targets(
    source: &CompositeSet,
    target: &CompositeSet,
) -> Vec<(CompositeTarget, u32, u32)> {
    target
        .targets()
        .into_iter()
        .map(|kind| (kind, source.target_size(kind), target.target_size(kind)))
        .collect()
}

/// The targets the block compositor generates mipmaps for after the passes
/// (colour and normal, when the filter word asks): the gloss and displacement
/// targets never get a mip chain.
pub const MIPMAP_TARGETS: [CompositeTarget; 2] = [CompositeTarget::Colour, CompositeTarget::Normal];

/// The six programs a composite pass can run (the original's per-combination
/// dictionary collapses to one program here, but the keys are the same).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassProgram {
    Normal,
    Colour,
    Gloss,
    Displacement,
    Mask,
    Height,
}

impl PassProgram {
    /// Whether the pass samples each layer's own texture (the mask and
    /// height passes sample only their mask layer).
    pub fn samples_layer(self) -> bool {
        !matches!(self, Self::Mask | Self::Height)
    }

    /// The layer texture slot the pass samples.
    pub fn layer_slot(self) -> Option<exa_formats::terrain::TerrainTexture> {
        use exa_formats::terrain::TerrainTexture as T;
        match self {
            Self::Normal => Some(T::Normal),
            Self::Colour => Some(T::Diffuse),
            Self::Gloss => Some(T::Gloss),
            Self::Displacement => Some(T::Bump),
            Self::Mask | Self::Height => None,
        }
    }

    /// The composite target a pass writes, when the pass is enabled. The
    /// mask and height passes write their working target instead of a set.
    pub fn target(self, features: CompositeFeatures) -> Option<CompositeTarget> {
        match self {
            Self::Normal if features.normal => Some(CompositeTarget::Normal),
            Self::Colour => Some(CompositeTarget::Colour),
            Self::Gloss if features.gloss => Some(CompositeTarget::Gloss),
            Self::Displacement if features.displacement => Some(CompositeTarget::Displacement),
            _ => None,
        }
    }

    /// The pass's clear colour: the mask pass clears to 0 and the height pass
    /// to 0.5 in every channel; the set passes overwrite every texel and are
    /// not cleared.
    pub fn clear(self) -> Option<[f32; 4]> {
        match self {
            Self::Mask => Some([0.0; 4]),
            Self::Height => Some([HEIGHT_CLEAR; 4]),
            _ => None,
        }
    }
}

/// Which of a layer's textures a pass finds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LayerFlags {
    /// The pass's own texture is present.
    pub textured: bool,
    /// The layer has a blend mask.
    pub masked: bool,
}

/// One pass's program key: the layers in the group, whether it is an extra
/// additive group, which layers are textured and masked, and whether the
/// normal pass adds the geometry normal (`BaseNrm`, off at the shipped
/// settings).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PassKey {
    pub layers: u8,
    pub add: bool,
    pub textured: u8,
    pub masked: u8,
    pub base_normal: bool,
}

impl PassKey {
    /// The original's program word.
    pub fn word(self) -> u32 {
        let mut word = u32::from(self.layers);
        if self.add {
            word |= 0x1000_0000;
        }
        word |= u32::from(self.textured) << 4;
        word |= u32::from(self.masked) << 8;
        if self.base_normal {
            word |= 0x2000_0000;
        }
        word
    }

    /// Decodes a program word.
    pub fn from_word(word: u32) -> Self {
        Self {
            layers: (word & 0xf) as u8,
            add: word & 0x1000_0000 != 0,
            textured: ((word >> 4) & 0xf) as u8,
            masked: ((word >> 8) & 0xf) as u8,
            base_normal: word & 0x2000_0000 != 0,
        }
    }
}

/// One layer's per-pass values, as the compositor uploads them.
///
/// The colour pass's vector is the linear tint times `1.5·min(f9, 1)` with
/// the cover `f9` in alpha; the gloss pass reuses the same vector's shape but
/// a scalar `f10·1.5·min(f9, 1)` for its RGB; the normal pass uploads `f11`
/// as its amount; the height, displacement and mask passes upload four
/// floats with the last forced to 1. The mask sharpness pair is also sent
/// stretched to `(s, t/s + 1)` (the original's shader form).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayerPass {
    pub colour: bevy::math::Vec4,
    /// The cover strength f9, uploaded as the pass's amount.
    pub cover: f32,
    pub gloss: f32,
    pub normal_amount: f32,
    pub colour_sharpness: [f32; 2],
    pub normal_sharpness: [f32; 2],
    /// f16, f17, f18, 1.
    pub height: [f32; 4],
    /// f20, f21, f22, 1.
    pub displacement: [f32; 4],
    /// f12, f13, f14, 1.
    pub mask_colour: [f32; 4],
    /// f0, or 1 when not positive.
    pub tile: f32,
}

impl LayerPass {
    /// The values of a palette material.
    pub fn of(material: &PaletteMaterial) -> Self {
        let look = super::Look::of(material);
        Self {
            colour: look.colour,
            cover: look.cover,
            gloss: look.gloss,
            normal_amount: look.normal_strength,
            colour_sharpness: look.sharpness,
            normal_sharpness: look.normal_sharpness,
            height: [look.height[0], look.height[1], look.height[2], 1.0],
            displacement: [look.parallax[0], look.parallax[1], look.parallax[2], 1.0],
            mask_colour: [
                look.mask_colour[0],
                look.mask_colour[1],
                look.mask_colour[2],
                1.0,
            ],
            tile: look.tile,
        }
    }

    /// The sharpness pair a pass uses: the normal pair for the normal and
    /// displacement passes, the colour pair for colour, gloss, mask and
    /// height.
    pub fn sharpness(&self, program: PassProgram) -> [f32; 2] {
        match program {
            PassProgram::Normal | PassProgram::Displacement => self.normal_sharpness,
            _ => self.colour_sharpness,
        }
    }

    /// Whether the pass samples the material's own texture for `program`.
    pub fn textured(&self, program: PassProgram, material: &PaletteMaterial) -> bool {
        program
            .layer_slot()
            .is_some_and(|slot| material.texture(slot).is_some())
    }

    /// Whether the material has a blend mask.
    pub fn masked(material: &PaletteMaterial) -> bool {
        material.texture(TerrainTexture::Mask).is_some()
    }
}

/// The original's shader form of a sharpness pair: `(s, t/s + 1)`.
pub fn stretched_sharpness([s, t]: [f32; 2]) -> [f32; 2] {
    [s, t / s + 1.0]
}

/// The key of one group of up to four layers, starting at `start` of
/// `total`. `layers[k]` describes layer `start + k`; `program` decides
/// whether the pass's own texture contributes a `Textured` bit. Only the
/// normal pass can set `BaseNrm`.
pub fn pass_key(
    program: PassProgram,
    start: usize,
    total: usize,
    layers: &[LayerFlags],
    base_normal: bool,
) -> PassKey {
    let count = total.saturating_sub(start).min(4);
    let mut key = PassKey {
        layers: count as u8,
        add: start > 0,
        ..PassKey::default()
    };
    for (lane, flags) in layers.iter().take(count).enumerate() {
        if program.samples_layer() && flags.textured {
            key.textured |= 1 << lane;
        }
        if flags.masked {
            key.masked |= 1 << lane;
        }
    }
    key.base_normal = program == PassProgram::Normal && base_normal;
    key
}

/// Splits a pass's layers into the original's groups of four: the first
/// group is the base pass, every later one is additive. `layers[k]`
/// describes layer `k`.
pub fn pass_groups(program: PassProgram, layers: &[LayerFlags], base_normal: bool) -> Vec<PassKey> {
    let total = layers.len();
    let mut out = Vec::new();
    let mut start = 0;
    while start < total {
        let count = (total - start).min(4);
        let group = &layers[start..start + count];
        out.push(pass_key(program, start, total, group, base_normal));
        start += count;
    }
    out
}

/// A block's composite state bits, as the original keeps them.
pub mod block_flags {
    /// The block needs a composite this frame.
    pub const WANTED: u32 = 0x2;
    /// Its composite is up to date.
    pub const CLEAN: u32 = 0x4;
    /// It wants the low-resolution set.
    pub const LOW_RES: u32 = 0x100;
    /// It wants the high-resolution set.
    pub const HIGH_RES: u32 = 0x200;
    /// It holds a low-resolution set.
    pub const LOW_SET: u32 = 0x400;
    /// It holds a high-resolution set.
    pub const HIGH_SET: u32 = 0x800;
}

/// One 6 m block the selector tracks.
#[derive(Clone, Debug, PartialEq)]
pub struct BlockComposite {
    /// The block's centre on the X/Z plane, metres.
    pub centre: Vec2,
    /// Whether the block's grid cell lets it composite (the original's cell
    /// flag bit 8; always true outside a grid level).
    pub enabled: bool,
    /// Whether the block's scene node is visible this frame.
    pub visible: bool,
    /// Its priority: the distance rank, then the composite ring once baked.
    pub priority: u32,
    /// The nearest light's distance (metres? centimetres: the original's
    /// light test answers centimetres), or `None` for no light in reach.
    pub light_distance: Option<f32>,
    state: u32,
    set: Option<u32>,
    selection: Option<usize>,
    sort_key: u32,
}

impl BlockComposite {
    /// A new block, in the original's initial state: wanted, clean, no set,
    /// not in the selection list.
    pub fn new(centre: Vec2) -> Self {
        Self {
            centre,
            enabled: true,
            visible: false,
            priority: 0,
            light_distance: None,
            state: block_flags::WANTED | block_flags::CLEAN,
            set: None,
            selection: None,
            sort_key: 0,
        }
    }

    /// The state bits.
    pub fn state(&self) -> u32 {
        self.state
    }

    /// The set it holds, if any.
    pub fn set(&self) -> Option<u32> {
        self.set
    }

    /// Its index in the selection list, if selected.
    pub fn selection(&self) -> Option<usize> {
        self.selection
    }

    /// The key the selection sorts it by.
    pub fn sort_key(&self) -> u32 {
        self.sort_key
    }

    /// Marks it for a composite again (the tile hook does this when a tile
    /// is (re)inserted).
    pub fn want(&mut self) {
        self.state |= block_flags::WANTED;
    }
}

/// The block selector: the original's selected list, pools, steal cursor and
/// composite ring.
#[derive(Clone, Debug, PartialEq)]
pub struct CompositeSelector {
    blocks: Vec<BlockComposite>,
    selected: Vec<usize>,
    pools: CompositePools,
    cursor: usize,
    ring: i32,
}

impl Default for CompositeSelector {
    fn default() -> Self {
        Self::new(
            Vec::new(),
            CompositePools::new(detail_size(3), CompositeFeatures::default()),
        )
    }
}

impl CompositeSelector {
    pub fn new(blocks: Vec<BlockComposite>, pools: CompositePools) -> Self {
        Self {
            blocks,
            selected: Vec::new(),
            pools,
            cursor: 0,
            ring: RING_START,
        }
    }

    pub fn blocks(&self) -> &[BlockComposite] {
        &self.blocks
    }

    pub fn block_mut(&mut self, index: usize) -> Option<&mut BlockComposite> {
        self.blocks.get_mut(index)
    }

    /// The selected block indices, in composite order.
    pub fn selected(&self) -> &[usize] {
        &self.selected
    }

    pub fn pools(&self) -> &CompositePools {
        &self.pools
    }

    pub fn ring(&self) -> i32 {
        self.ring
    }

    pub fn set_ring(&mut self, ring: i32) {
        self.ring = ring;
    }

    /// Runs one frame of the selection: neighbours in and out of the list,
    /// sort keys, the composite order, and at most one block baked once a
    /// non-clean block needs work. Returns whether anything was baked.
    pub fn reselect(&mut self, focus: Vec2) -> bool {
        self.select(focus);
        self.assign_keys();
        self.order();
        self.composite()
    }

    /// The distance test: a block within 44.4 m is selected, a NaN distance
    /// kept as low; within 15 m it becomes high resolution. A block out of
    /// reach leaves the list and hands its set back.
    fn select(&mut self, focus: Vec2) {
        let (fx, fz) = (focus.x * 100.0, focus.y * 100.0);
        for index in 0..self.blocks.len() {
            let centre = self.blocks[index].centre;
            let dx = fx - centre.x * 100.0;
            let dz = fz - centre.y * 100.0;
            let d2 = dz * dz + dx * dx;
            // Kept when the squared distance does not compare above the far
            // cut-off; NaN is kept.
            let kept = d2 <= FAR_D2 || d2.is_nan();
            if kept {
                if self.blocks[index].selection.is_none() {
                    if self.selected.len() >= SELECTED_LIMIT {
                        continue;
                    }
                    self.blocks[index].selection = Some(self.selected.len());
                    self.selected.push(index);
                }
                // NaN falls into the low branch (its compare is unordered).
                let low = d2.is_nan() || d2 >= NEAR_D2;
                let block = &mut self.blocks[index];
                if low {
                    block.state = (block.state & !block_flags::HIGH_RES) | block_flags::LOW_RES;
                } else {
                    if block.state & (block_flags::LOW_RES | block_flags::HIGH_RES)
                        != block_flags::HIGH_RES
                    {
                        block.state |= block_flags::WANTED;
                    }
                    block.state = (block.state & !block_flags::LOW_RES) | block_flags::HIGH_RES;
                }
            } else if let Some(at) = self.blocks[index].selection.take() {
                let last = self.selected.len() - 1;
                if at != last {
                    let moved = self.selected[last];
                    self.selected[at] = moved;
                    self.blocks[moved].selection = Some(at);
                }
                self.selected.pop();
                let state = self.blocks[index].state;
                self.blocks[index].state = block_flags::WANTED | block_flags::CLEAN;
                if let Some(set) = self.blocks[index].set.take() {
                    if state & block_flags::LOW_SET != 0 {
                        self.pools.give_low(set);
                    } else if state & block_flags::HIGH_SET != 0 {
                        self.pools.give_high(set);
                    }
                }
            }
        }
    }

    /// The sort key: a visible clean block first (0), a visible block that
    /// wants work by `0xffff − (ring − priority)` clamped to 1, an invisible
    /// block by its nearest light's rounded distance plus 0x10000.
    fn assign_keys(&mut self) {
        let ring = self.ring as u32;
        for &bi in &self.selected {
            let block = &mut self.blocks[bi];
            block.sort_key = if !block.visible {
                let mut nearest = NO_LIGHT_DISTANCE;
                if let Some(distance) = block.light_distance
                    && !distance.is_nan()
                    && nearest > distance
                {
                    nearest = distance;
                }
                (round_to_i64(nearest) + i64::from(LIGHT_SORT_BIAS)) as u32
            } else if block.state & block_flags::CLEAN != 0 {
                0
            } else {
                // The original compares the 64-bit difference as a signed
                // value: a ring behind the block's priority is negative, and
                // then the key wraps upwards from 0xffff.
                let difference = u64::from(ring).wrapping_sub(u64::from(block.priority)) as i64;
                if difference < i64::from(SORT_LIMIT) {
                    SORT_LIMIT.wrapping_sub(ring.wrapping_sub(block.priority))
                } else {
                    1
                }
            };
        }
    }

    /// The stable ascending insertion sort of the original, by sort key.
    fn order(&mut self) {
        let mut order: Vec<(usize, u32)> = self
            .selected
            .iter()
            .map(|&bi| (bi, self.blocks[bi].sort_key))
            .collect();
        order.sort_by_key(|&(_, key)| key);
        self.selected.clear();
        self.selected.extend(order.into_iter().map(|(bi, _)| bi));
        for (at, &bi) in self.selected.iter().enumerate() {
            self.blocks[bi].selection = Some(at);
        }
    }

    /// Walks the selected blocks in order, baking the first one that needs
    /// work; once one is baked, a later block that is not clean stops the
    /// walk (the original's "one block a frame" rule).
    fn composite(&mut self) -> bool {
        self.cursor = self.selected.len().saturating_sub(1);
        let mut changed = false;
        let mut at = 0;
        while at < self.selected.len() {
            let bi = self.selected[at];
            self.blocks[bi].selection = Some(at);
            if !self.blocks[bi].enabled {
                at += 1;
                continue;
            }
            let key = self.blocks[bi].sort_key;
            if changed && key != 0 {
                for (rest, &bj) in self.selected.iter().enumerate().skip(at) {
                    self.blocks[bj].selection = Some(rest);
                }
                break;
            }
            let state = self.blocks[bi].state;
            if state & block_flags::WANTED == 0 {
                at += 1;
                continue;
            }
            let baked = self.bake(bi, at, state);
            changed |= baked;
            at += 1;
        }
        if changed {
            self.ring = if self.ring == -1 { 0 } else { self.ring + 1 };
        }
        changed
    }

    /// One block's transitions, exactly as the original's state machine.
    fn bake(&mut self, block: usize, at: usize, state: u32) -> bool {
        if state & 0xc00 == 0 {
            if state & block_flags::HIGH_RES == 0 {
                if let Some(set) = self.take_low(at) {
                    self.blocks[block].set = Some(set);
                    self.composited(block);
                    self.blocks[block].state = 0x500;
                    return true;
                }
            } else if self.pools.high_free() > 0 {
                let set = self.pools.take_high().expect("a high set is free");
                self.blocks[block].set = Some(set);
                self.composited(block);
                self.blocks[block].state = 0xa00;
                return true;
            } else if let Some(other) = self.steal_high() {
                let set = self.blocks[other].set;
                self.blocks[block].set = set;
                self.steal_high_share(other);
                self.composited(block);
                self.blocks[block].state = 0xa00;
                return true;
            }
            return false;
        }
        if state & 0xf00 == 0x600 {
            if self.pools.high_free() > 0 {
                if let Some(low) = self.blocks[block].set.take() {
                    self.pools.give_low(low);
                }
                let set = self.pools.take_high().expect("a high set is free");
                self.blocks[block].set = Some(set);
                self.composited(block);
                self.blocks[block].state = 0xa00;
                return true;
            }
            if let Some(other) = self.steal_high() {
                let low = self.blocks[block].set.take();
                let high = self.blocks[other].set;
                self.blocks[block].set = high;
                self.steal_high_swap(other, low);
                self.composited(block);
                self.blocks[block].state = 0xa00;
                return true;
            }
            return false;
        }
        self.composited(block);
        self.blocks[block].state &= !(block_flags::WANTED | block_flags::CLEAN);
        true
    }

    /// A block that just turned high takes a high set still held by a block
    /// that is now low resolution. If that holder is not clean, it gets a
    /// low set with a copy of its composite; if it is clean, it keeps
    /// sharing the high set.
    fn steal_high_share(&mut self, other: usize) {
        let state = self.blocks[other].state;
        if state & block_flags::CLEAN != 0 {
            return;
        }
        if let Some(low) = self.take_low(other) {
            self.blocks[other].set = Some(low);
            self.blocks[other].state = (state & block_flags::WANTED) | 0x500;
            self.composited(other);
        } else {
            self.blocks[other].set = None;
            self.blocks[other].state = block_flags::WANTED | block_flags::CLEAN;
        }
    }

    /// The same for a holder that is itself turning high and gives its low
    /// set away: the old low set is reused for the holder's copy (or simply
    /// handed to it when the holder is clean).
    fn steal_high_swap(&mut self, other: usize, low: Option<u32>) {
        let state = self.blocks[other].state;
        if state & block_flags::CLEAN != 0 {
            self.blocks[other].state = 0x506;
            self.blocks[other].set = low;
            return;
        }
        if let Some(low) = low {
            self.blocks[other].set = Some(low);
            self.blocks[other].state = (state & block_flags::WANTED) | 0x500;
            self.composited(other);
        } else {
            self.blocks[other].set = None;
            self.blocks[other].state = block_flags::WANTED | block_flags::CLEAN;
        }
    }

    /// The original's set hand-out helper: a free low set, else steal the
    /// low set of a later selected block that is not visible.
    fn take_low(&mut self, at: usize) -> Option<u32> {
        if let Some(set) = self.pools.take_low() {
            return Some(set);
        }
        self.take_low_for(at)
    }

    /// The steal scan alone, used when a block's set is replaced.
    fn take_low_for(&mut self, at: usize) -> Option<u32> {
        if at + 1 > self.cursor {
            return None;
        }
        let mut scan = self.cursor;
        loop {
            self.cursor = scan.saturating_sub(1);
            let bi = self.selected[scan];
            let block = &mut self.blocks[bi];
            if block.state & block_flags::LOW_SET != 0 && !block.visible {
                let set = block.set.take();
                block.state = block_flags::WANTED | block_flags::CLEAN;
                return set;
            }
            if scan <= at + 1 {
                break;
            }
            scan -= 1;
        }
        None
    }

    /// The last selected block holding a high set at low resolution
    /// (`0x900`), whose set a block that just turned high can take.
    fn steal_high(&self) -> Option<usize> {
        self.selected
            .iter()
            .rev()
            .copied()
            .find(|&bi| self.blocks[bi].state & 0xf00 == 0x900)
    }

    /// Records a bake: the block's priority becomes the ring it was baked in.
    fn composited(&mut self, block: usize) {
        self.blocks[block].priority = self.ring as u32;
    }
}

/// The original's `TexPos`: the lit program's composite coordinate for a
/// block. A whole-tile block (level 0) is (0.5, 0.5); at deeper levels the
/// block's tile coordinate is shifted down to its own level and subtracted
/// from half the split. The third component the original computes (the
/// block's side over the tile side) is not uploaded.
pub fn tex_pos(terrain_level: u32, level: u32, bx: u32, by: u32) -> Vec2 {
    if level < 1 {
        return Vec2::splat(0.5);
    }
    let half = 1u32 << (level - 1);
    let shift = terrain_level.saturating_sub(level);
    Vec2::new(
        (i64::from(half) - i64::from(bx >> shift)) as f32,
        (i64::from(half) - i64::from(by >> shift)) as f32,
    )
}

/// One composite pass's quad: its four vertices in the palette's tile-scaled
/// coordinates and their mask-upload texcoords.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlockQuad {
    /// Palette coordinates at each corner, in the order the original emits
    /// them (low x and high v, high x and high v, high x and low v, low x and
    /// low v). The layer sampler multiplies these by its tiling.
    pub vertices: [Vec2; 4],
    /// The same corners in the 32 × 32 mask upload, vertex centred.
    pub mask_uv: [Vec2; 4],
}

impl Default for BlockQuad {
    fn default() -> Self {
        block_quad(0, 0)
    }
}

/// The quad of block (`bx`, `by`) at the original's 8 blocks a side, palette
/// scale 4, 30 cells and 32-texel mask upload.
pub fn block_quad(bx: u32, by: u32) -> BlockQuad {
    let side = BLOCK_SIDE as f32;
    let step = PALETTE_SCALE / side;
    let x0 = bx as f32 * step;
    let x1 = (bx + 1) as f32 * step;
    let v0 = -(by as f32 * step);
    let v1 = -((by + 1) as f32 * step);
    let texel = 0.5 / MASK_UPLOAD as f32;
    let span = (0.5 + BLOCK_CELLS as f32) / MASK_UPLOAD as f32;
    BlockQuad {
        vertices: [
            Vec2::new(x0, v0),
            Vec2::new(x1, v0),
            Vec2::new(x1, v1),
            Vec2::new(x0, v1),
        ],
        mask_uv: [
            Vec2::splat(texel),
            Vec2::new(span, texel),
            Vec2::splat(span),
            Vec2::new(texel, span),
        ],
    }
}

/// The wet-floor gloss: a wet floor's gloss rises by eight times the square
/// of the original's wetness term, capped at 0.85. `height_cm` is the world
/// height in centimetres: nothing changes at or above −35 cm, and lower
/// ground grows glossier as `max((y + 35) · −0.03, 0)² · normal y`.
pub fn wet_floor_gloss(gloss: f32, height_cm: f32, normal_y: f32) -> f32 {
    let m = ((height_cm + 35.0) * -0.03).max(0.0);
    let c = m * m * normal_y;
    (gloss + 8.0 * c * c).min(0.85)
}

/// The wetness path's roughness: `mix(1 − G, (1 − G/(G + 0.2))², W·n.y⁶⁴)`,
/// with `G` the composited gloss and `W` the scene's wetness. The shipped
/// levels set `W` to 0, where it is exactly `1 − G`.
pub fn wet_roughness(gloss: f32, normal_y: f32, wetness: f32) -> f32 {
    let g = gloss.max(0.0);
    let dry = 1.0 - g;
    let wet = (1.0 - g / (g + 0.2)).powi(2);
    dry + (wet - dry) * (wetness * normal_y.max(0.0).powi(64))
}

/// The compositor's working buffers: the 2 MB material-mask atlas, the blur
/// plane and the height rows, sized once at start-up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScratchSizes {
    /// The mask atlas: eight 256 × 256 byte channels.
    pub mask_atlas: usize,
    /// The blur's float plane, `(2·120 + 241)²` floats.
    pub blur_plane: usize,
    /// The height blur's `(2·120 + 242)` rows of 244 floats.
    pub height_rows: usize,
    /// The per-tile height plane, `(241·244 + 4)` floats.
    pub height_plane: usize,
    /// The blur kernel's radius (120) and entries (241).
    pub kernel_radius: usize,
    pub kernel_len: usize,
}

impl Default for ScratchSizes {
    fn default() -> Self {
        let apron = 2 * KERNEL_RADIUS;
        Self {
            mask_atlas: MASK_ATLAS_BYTES,
            blur_plane: (apron + 241) * (apron + 241) * 4,
            height_rows: (apron + 242) * 244 * 4,
            height_plane: (241 * 244 + 4) * 4,
            kernel_radius: KERNEL_RADIUS,
            kernel_len: KERNEL_RADIUS * 2 + 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pools() -> CompositePools {
        CompositePools::new(detail_size(3), CompositeFeatures::default())
    }

    fn selector(centres: &[Vec2]) -> CompositeSelector {
        CompositeSelector::new(
            centres.iter().copied().map(BlockComposite::new).collect(),
            pools(),
        )
    }

    #[test]
    fn the_detail_levels_and_set_sizes_follow_the_texture_detail() {
        // 8192 / 8 = 1024, log2 = 10, minus 5 → level 5 and level 4.
        assert_eq!(detail_size(3), 8192);
        assert_eq!(detail_size(2), 4096);
        assert_eq!(floor_log2(1024), 10);
        assert_eq!(floor_log2(511), 8);
        assert_eq!(floor_log2(1), 0);
        assert_eq!(floor_log2(0), 0);
        assert_eq!(high_level(8192), 5);
        assert_eq!(high_level(4096), 4);
        assert_eq!(set_size(5), 1024);
        assert_eq!(set_size(4), 512);
        let high = CompositePools::new(8192, CompositeFeatures::default());
        assert_eq!((high.high_level(), high.low_level()), (5, 4));
        let low = CompositePools::new(4096, CompositeFeatures::default());
        assert_eq!((low.high_level(), low.low_level()), (4, 3));
        // The targets: RGB8, RG8 and R8 at the set's size, the displacement
        // at half of it.
        let set = high.set(0).unwrap();
        assert_eq!(set.target_size(CompositeTarget::Colour), 1024);
        assert_eq!(set.target_size(CompositeTarget::Normal), 1024);
        assert_eq!(set.target_size(CompositeTarget::Gloss), 1024);
        assert_eq!(set.target_size(CompositeTarget::Displacement), 512);
        assert_eq!(
            [
                CompositeTarget::Colour.channels(),
                CompositeTarget::Normal.channels(),
                CompositeTarget::Gloss.channels(),
                CompositeTarget::Displacement.channels(),
            ],
            [3, 2, 1, 1]
        );
        assert_eq!(
            set.targets(),
            [
                CompositeTarget::Colour,
                CompositeTarget::Normal,
                CompositeTarget::Gloss,
                CompositeTarget::Displacement
            ]
        );
    }

    #[test]
    fn the_pools_hand_sets_out_from_the_ends() {
        let mut pools = pools();
        assert_eq!((pools.high_free(), pools.low_free()), (21, 43));
        // The high list is 0..=20 and the low list 21..=63, popped from the
        // end: 20 first, then 19; 63 first, then 62.
        assert_eq!(pools.take_high(), Some(20));
        assert_eq!(pools.take_high(), Some(19));
        assert_eq!(pools.take_low(), Some(63));
        assert_eq!(pools.take_low(), Some(62));
        assert_eq!((pools.high_free(), pools.low_free()), (19, 41));
        pools.give_high(20);
        pools.give_low(63);
        // Giving the same set twice does not duplicate it.
        pools.give_high(20);
        pools.give_low(63);
        assert_eq!((pools.high_free(), pools.low_free()), (20, 42));
        assert_eq!(pools.set(0).unwrap().level, 5);
        assert_eq!(pools.set(21).unwrap().level, 4);
    }

    #[test]
    fn pass_keys_are_the_originals_words() {
        let all = [LayerFlags {
            textured: true,
            masked: true,
        }; 4];
        let key = pass_key(PassProgram::Colour, 0, 4, &all, false);
        assert_eq!(key.layers, 4);
        assert!(!key.add);
        assert_eq!(key.textured, 0b1111);
        assert_eq!(key.masked, 0b1111);
        assert_eq!(key.word(), 0xff4);
        let key = pass_key(PassProgram::Normal, 0, 4, &all, true);
        assert_eq!(key.word(), 0x2000_0ff4);
        // The second group of six layers is additive and has two layers.
        let key = pass_key(PassProgram::Colour, 4, 6, &all[..2], false);
        assert_eq!(key.layers, 2);
        assert!(key.add);
        assert_eq!(key.word(), 0x1000_0332);
        // The mask and height passes sample only the blend mask, so they set
        // no Textured bit.
        let key = pass_key(PassProgram::Mask, 0, 3, &all[..3], false);
        assert_eq!(key.word(), 0x703);
        // Six layers are a base group of four and an additive group of two.
        let six = [all[0]; 6];
        let groups = pass_groups(PassProgram::Colour, &six, false);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].word(), 0xff4);
        assert_eq!(groups[1].word(), 0x1000_0332);
        assert_eq!(
            pass_groups(PassProgram::Height, &all[..1], false)[0].word(),
            0x101
        );
        assert_eq!(PassKey::from_word(key.word()), key);
        assert_eq!(PassKey::from_word(0), PassKey::default());
        assert!(PassProgram::Normal.samples_layer());
        assert!(!PassProgram::Mask.samples_layer());
        assert_eq!(
            PassProgram::Displacement.layer_slot(),
            Some(exa_formats::terrain::TerrainTexture::Bump)
        );
        assert_eq!(PassProgram::Height.layer_slot(), None);
    }

    #[test]
    fn tex_pos_maps_a_block_to_its_composite_coordinate() {
        assert_eq!(tex_pos(3, 0, 7, 5), Vec2::splat(0.5));
        // The 8 blocks of a tile: half the split (4) less the block index.
        assert_eq!(tex_pos(3, 3, 0, 0), Vec2::new(4.0, 4.0));
        assert_eq!(tex_pos(3, 3, 7, 7), Vec2::new(-3.0, -3.0));
        assert_eq!(tex_pos(3, 3, 3, 5), Vec2::new(1.0, -1.0));
        // A shallower split shifts the tile coordinate down: level 2 has two
        // blocks a side of the tile's eight, so block 7 is block 3.
        assert_eq!(tex_pos(3, 2, 6, 7), Vec2::new(-1.0, -1.0));
    }

    #[test]
    fn a_block_quad_covers_its_palette_slice_and_mask_window() {
        let q = block_quad(0, 0);
        assert_eq!(
            q.vertices,
            [
                Vec2::new(0.0, 0.0),
                Vec2::new(0.5, 0.0),
                Vec2::new(0.5, -0.5),
                Vec2::new(0.0, -0.5)
            ]
        );
        let texel = 0.5 / 32.0;
        let span = 30.5 / 32.0;
        assert_eq!(
            q.mask_uv,
            [
                Vec2::splat(texel),
                Vec2::new(span, texel),
                Vec2::splat(span),
                Vec2::new(texel, span)
            ]
        );
        // Block (3, 5): x 1.5..2, v −2.5..−3 (v falls with +Z).
        let q = block_quad(3, 5);
        assert_eq!(q.vertices[0], Vec2::new(1.5, -2.5));
        assert_eq!(q.vertices[1], Vec2::new(2.0, -2.5));
        assert_eq!(q.vertices[2], Vec2::new(2.0, -3.0));
        assert_eq!(q.vertices[3], Vec2::new(1.5, -3.0));
        // The whole tile is 8×8 such slices, so the UV span is 4 (the
        // palette scale).
        assert_eq!(block_quad(7, 0).vertices[1].x, 4.0);
    }

    #[test]
    fn the_mask_upload_window_is_a_thirty_texel_block_in_a_256_plane() {
        assert_eq!(mask_texel_offset(0, 0), 0);
        assert_eq!(mask_texel_offset(1, 0), 30);
        assert_eq!(mask_texel_offset(0, 1), 7_680);
        assert_eq!(mask_texel_offset(7, 7), 53_970);
        assert_eq!(MASK_PLANE_TEXELS, 65_536);
        assert_eq!(MASK_UPLOAD, 32);
        assert_eq!(BLOCK_CELLS, 30);
    }

    #[test]
    fn the_wet_floor_term_only_lifts_ground_below_minus_thirty_five_cm() {
        assert_eq!(wet_floor_gloss(0.5, 0.0, 1.0), 0.5);
        assert_eq!(wet_floor_gloss(0.5, -35.0, 1.0), 0.5);
        assert_eq!(wet_floor_gloss(0.5, 0.0, 0.0), 0.5);
        // At −40 cm: m = 0.15, c = 0.0225, 8c² = 0.00405.
        let g = wet_floor_gloss(0.5, -40.0, 1.0);
        assert!((g - 0.504_05).abs() < 1e-6, "{g}");
        // At −100 cm the cap holds.
        assert_eq!(wet_floor_gloss(0.5, -100.0, 1.0), 0.85);
        assert_eq!(wet_floor_gloss(0.9, -100.0, 1.0), 0.85);
        // A horizontal normal sees none of it.
        assert_eq!(wet_floor_gloss(0.5, -100.0, 0.0), 0.5);
    }

    #[test]
    fn the_wetness_path_is_the_dry_one_at_zero() {
        for gloss in [0.0, 0.2, 0.5, 0.9] {
            assert_eq!(wet_roughness(gloss, 1.0, 0.0), 1.0 - gloss);
        }
        // At full wetness on an up normal, G = 0.5 → (1 − 0.5/0.7)².
        let r = wet_roughness(0.5, 1.0, 1.0);
        assert!((r - (1.0 - 0.5 / 0.7f32).powi(2)).abs() < 1e-6, "{r}");
        // A side normal gets no wetness.
        assert_eq!(wet_roughness(0.5, 0.0, 1.0), 0.5);
    }

    #[test]
    fn the_scratch_buffers_are_the_originals_sizes() {
        let s = ScratchSizes::default();
        assert_eq!(s.mask_atlas, 2_097_152);
        assert_eq!(s.blur_plane, 481 * 481 * 4);
        assert_eq!(s.blur_plane, 925_444);
        assert_eq!(s.height_rows, 482 * 244 * 4);
        assert_eq!(s.height_rows, 470_432);
        assert_eq!(s.height_plane, 0x39_6e0);
        assert_eq!(s.height_plane, 235_232);
        assert_eq!((s.kernel_radius, s.kernel_len), (120, 241));
    }

    #[test]
    fn copying_a_set_writes_each_target_at_the_targets_size() {
        let low = CompositeSet {
            id: 21,
            level: 4,
            normal: true,
            gloss: true,
            displacement: true,
        };
        let high = CompositeSet {
            id: 0,
            level: 5,
            ..low
        };
        assert_eq!(
            copy_targets(&low, &high),
            vec![
                (CompositeTarget::Colour, 512, 1024),
                (CompositeTarget::Normal, 512, 1024),
                (CompositeTarget::Gloss, 512, 1024),
                (CompositeTarget::Displacement, 256, 512),
            ]
        );
        let no_normal = CompositeSet {
            normal: false,
            ..high
        };
        assert_eq!(
            copy_targets(&low, &no_normal),
            vec![
                (CompositeTarget::Colour, 512, 1024),
                (CompositeTarget::Gloss, 512, 1024),
                (CompositeTarget::Displacement, 256, 512),
            ]
        );
        assert_eq!(
            MIPMAP_TARGETS,
            [CompositeTarget::Colour, CompositeTarget::Normal]
        );
    }

    #[test]
    fn the_levels_working_targets_are_the_original_sizes() {
        // Level 5: 512² and 1024² framebuffers, a 256² mask target, and four
        // 32² scratch textures of one to four channels.
        let t = LevelTargets::new(5);
        assert_eq!(
            (
                t.depth_half,
                t.depth_full,
                t.plain_half,
                t.plain_full,
                t.mask
            ),
            (512, 1024, 512, 1024, 256)
        );
        assert_eq!(t.scratch_side, 32);
        assert_eq!(t.scratch_channels, [1, 2, 3, 4]);
        let t = LevelTargets::new(4);
        assert_eq!((t.depth_half, t.depth_full, t.mask), (256, 512, 128));
        // The shipped filter word has no mipmap bit; 0x70 would.
        assert_eq!(FILTER_WORD, 0x30);
        assert!(!mipmaps(FILTER_WORD));
        assert!(mipmaps(FILTER_WORD | MIPMAP_BIT));
        assert_eq!(Resolution::of(5, 5), Resolution::High);
        assert_eq!(Resolution::of(4, 5), Resolution::Low);
        assert_eq!(Resolution::of(3, 5), Resolution::None);
        assert_eq!(WET_FLOOR_THRESHOLD, 0.4);
        assert_eq!(MAT_DIF, 0.75);
        assert_eq!(SPECULAR_LEVEL_OFFSET, 0.3);
        assert_eq!(FRESNEL_F0, 0.04);
        assert_eq!(PARALLAX_SCALE, 0.007);
        assert_eq!((HEIGHT_CLEAR, HEIGHT_OPACITY_BOOST), (0.5, 1.5));
    }

    #[test]
    fn each_pass_writes_its_target_and_clears_only_its_work_target() {
        let all = CompositeFeatures::default();
        assert_eq!(
            PassProgram::Normal.target(all),
            Some(CompositeTarget::Normal)
        );
        assert_eq!(
            PassProgram::Colour.target(all),
            Some(CompositeTarget::Colour)
        );
        assert_eq!(PassProgram::Gloss.target(all), Some(CompositeTarget::Gloss));
        assert_eq!(
            PassProgram::Displacement.target(all),
            Some(CompositeTarget::Displacement)
        );
        assert_eq!(PassProgram::Mask.target(all), None);
        assert_eq!(PassProgram::Height.target(all), None);
        let no_normal = CompositeFeatures {
            normal: false,
            ..all
        };
        assert_eq!(PassProgram::Normal.target(no_normal), None);
        assert_eq!(PassProgram::Mask.clear(), Some([0.0; 4]));
        assert_eq!(PassProgram::Height.clear(), Some([0.5; 4]));
        assert_eq!(PassProgram::Colour.clear(), None);
        assert_eq!(PassProgram::Normal.clear(), None);
    }

    #[test]
    fn a_layer_pass_uploads_the_originals_values() {
        let mut params = vec![0.0f32; 29];
        params[0] = 2.0;
        params[2] = 3.0;
        params[3] = 1.0;
        params[4] = 4.0;
        params[5] = 2.0;
        params[6] = 1.0;
        params[7] = 1.0;
        params[8] = 1.0;
        params[9] = 0.5;
        params[10] = 0.4;
        params[11] = 0.8;
        params[12] = 0.2;
        params[13] = 0.7;
        params[14] = 0.1;
        params[16] = 0.6;
        params[17] = 0.4;
        params[18] = 0.9;
        params[20] = 0.5;
        params[21] = 0.3;
        params[22] = 0.7;
        let material = PaletteMaterial {
            name: "M".into(),
            flags: 0,
            textures: ["d", "n", "g", "m", "h", "b"].map(String::from),
            params,
        };
        let layer = LayerPass::of(&material);
        // A white tint stays 1; the gain is 1.5 × min(f9, 1) = 0.75 and the
        // alpha is the cover f9.
        assert_eq!(layer.colour, bevy::math::Vec4::new(0.75, 0.75, 0.75, 0.5));
        assert_eq!(layer.cover, 0.5);
        assert_eq!(layer.gloss, 0.4 * 0.75);
        assert_eq!(layer.normal_amount, 0.8);
        assert_eq!(layer.colour_sharpness, [3.0, 1.0]);
        assert_eq!(layer.normal_sharpness, [4.0, 2.0]);
        assert_eq!(layer.height, [0.6, 0.4, 0.9, 1.0]);
        assert_eq!(layer.displacement, [0.5, 0.3, 0.7, 1.0]);
        assert_eq!(layer.mask_colour, [0.2, 0.7, 0.1, 1.0]);
        assert_eq!(layer.tile, 2.0);
        assert_eq!(stretched_sharpness([3.0, 1.0]), [3.0, 1.0 / 3.0 + 1.0]);
        assert!(layer.textured(PassProgram::Colour, &material));
        assert!(!layer.textured(PassProgram::Mask, &material));
        assert!(LayerPass::masked(&material));
        // A tiling of 0 counts as 1.
        let mut flat = material.clone();
        flat.params[0] = 0.0;
        assert_eq!(LayerPass::of(&flat).tile, 1.0);
        // A material without textures gives no textured bit.
        let mut bare = material;
        bare.textures = Default::default();
        let layer = LayerPass::of(&bare);
        assert!(!layer.textured(PassProgram::Normal, &bare));
        assert!(!LayerPass::masked(&bare));
    }

    #[test]
    fn selection_keeps_the_cutoffs_and_bakes_all_clean_blocks_at_once() {
        // Four blocks around the focus: two within 15 m, two farther.
        let mut sel = selector(&[
            Vec2::new(0.0, 0.0),
            Vec2::new(14.0, 0.0),
            Vec2::new(20.0, 0.0),
            Vec2::new(44.0, 0.0),
        ]);
        for i in 0..4 {
            sel.block_mut(i).unwrap().visible = true;
        }
        assert!(sel.reselect(Vec2::ZERO));
        assert_eq!(sel.selected().len(), 4);
        // The first two are high (0xa00), the others low (0x500); all were
        // clean, so the original's walk bakes every one of them in the same
        // frame.
        assert_eq!(sel.blocks()[0].state(), 0xa00);
        assert_eq!(sel.blocks()[1].state(), 0xa00);
        assert_eq!(sel.blocks()[2].state(), 0x500);
        assert_eq!(sel.blocks()[3].state(), 0x500);
        assert_eq!((sel.pools().high_free(), sel.pools().low_free()), (19, 41));
        // The ring advanced once for the frame.
        assert_eq!(sel.ring(), RING_START + 1);
        // A block at 44.4 m exactly is kept; past it, dropped.
        let mut sel = selector(&[Vec2::new(44.4, 0.0), Vec2::new(44.41, 0.0)]);
        assert!(sel.reselect(Vec2::ZERO));
        assert_eq!(sel.selected(), [0]);
        // A NaN distance is kept, low.
        let mut sel = selector(&[Vec2::new(f32::NAN, 0.0)]);
        assert!(sel.reselect(Vec2::ZERO));
        assert_eq!(sel.blocks()[0].state() & 0xf00, 0x500);
        assert_eq!(sel.selected(), [0]);
    }

    #[test]
    fn out_of_reach_blocks_hand_their_sets_back() {
        let mut sel = selector(&[Vec2::ZERO, Vec2::new(10.0, 0.0)]);
        for i in 0..2 {
            sel.block_mut(i).unwrap().visible = true;
        }
        assert!(sel.reselect(Vec2::ZERO));
        // Both are within 15 m, so both took a high set.
        assert_eq!((sel.pools().high_free(), sel.pools().low_free()), (19, 43));
        // Move the focus past 44.4 m: both leave the list and their sets go
        // back to the pool.
        assert!(!sel.reselect(Vec2::new(100.0, 0.0)));
        assert!(sel.selected().is_empty());
        assert_eq!(sel.blocks()[0].state(), 6);
        assert_eq!(sel.blocks()[1].state(), 6);
        assert!(sel.blocks()[0].set().is_none());
        assert_eq!((sel.pools().high_free(), sel.pools().low_free()), (21, 43));
    }

    #[test]
    fn a_low_block_turns_high_and_trades_its_set() {
        let mut sel = selector(&[Vec2::ZERO]);
        sel.block_mut(0).unwrap().visible = true;
        // First bake it low, from 20 m.
        assert!(sel.reselect(Vec2::new(20.0, 0.0)));
        assert_eq!(sel.blocks()[0].state(), 0x500);
        assert_eq!(sel.pools().low_free(), 42);
        // Now it is within 15 m: the selection wants it high and its low set
        // goes back to the low pool while a high one is taken.
        assert!(sel.reselect(Vec2::ZERO));
        assert_eq!(sel.blocks()[0].state(), 0xa00);
        assert_eq!((sel.pools().high_free(), sel.pools().low_free()), (20, 43));
    }

    #[test]
    fn one_not_clean_block_is_baked_a_frame() {
        let mut sel = selector(&[Vec2::ZERO, Vec2::new(6.0, 0.0)]);
        for i in 0..2 {
            sel.block_mut(i).unwrap().visible = true;
        }
        // The first pass bakes both (they start clean).
        assert!(sel.reselect(Vec2::ZERO));
        assert_eq!(sel.ring(), RING_START + 1);
        // Ask for both again: they are not clean now, and the sort key of
        // the second is not 0, so the walk stops after the first.
        for i in 0..2 {
            sel.block_mut(i).unwrap().want();
        }
        let ring = sel.ring();
        assert!(sel.reselect(Vec2::ZERO));
        assert_eq!(sel.ring(), ring + 1);
        // One block now carries the ring it was baked in, the other still
        // its old priority.
        let newer = sel
            .blocks()
            .iter()
            .filter(|b| b.priority == ring as u32)
            .count();
        assert_eq!(newer, 1, "{:?}", sel.blocks());
        // Keep going: the other one bakes next frame.
        assert!(sel.reselect(Vec2::ZERO));
        let newer = sel
            .blocks()
            .iter()
            .filter(|b| b.priority == (ring + 1) as u32)
            .count();
        assert_eq!(newer, 1);
    }

    #[test]
    fn invisible_blocks_sort_after_visible_ones_by_their_nearest_light() {
        let mut sel = selector(&[
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(2.0, 0.0),
        ]);
        for i in 0..3 {
            sel.block_mut(i).unwrap().want();
        }
        // Make the keys differ: block 0 visible, 1 invisible far from a
        // light, 2 invisible close to one.
        sel.block_mut(0).unwrap().visible = true;
        sel.block_mut(1).unwrap().light_distance = None;
        sel.block_mut(2).unwrap().light_distance = Some(100.0);
        sel.reselect(Vec2::ZERO);
        // The visible block sorts first; the nearer invisible one after the
        // farther one.
        let order: Vec<u32> = sel
            .selected()
            .iter()
            .map(|&i| sel.blocks()[i].sort_key())
            .collect();
        assert_eq!(order[0], 0);
        assert_eq!(order[1], 100 + LIGHT_SORT_BIAS);
        assert_eq!(order[2], 65_000 + LIGHT_SORT_BIAS);
    }

    #[test]
    fn a_high_set_with_no_high_set_left_is_stolen_from_a_low_resolution_holder() {
        // A block that wants the high resolution, a clean high-resolution
        // block sharing a set, and a low-resolution block still holding a
        // high set (0x900). The high pool is empty.
        let mut sel = selector(&[Vec2::ZERO, Vec2::new(1.0, 0.0), Vec2::new(2.0, 0.0)]);
        sel.selected = vec![0, 1, 2];
        sel.cursor = 2;
        sel.pools.high_free.clear();
        sel.blocks[0].state = 0x900;
        sel.blocks[0].set = Some(5);
        sel.blocks[1].state = 0xa00;
        sel.blocks[1].set = Some(6);
        sel.blocks[2].state = block_flags::WANTED | block_flags::HIGH_RES;
        sel.blocks[2].set = None;
        assert!(sel.bake(2, 2, sel.blocks[2].state));
        // The newcomer took the high set; the holder got a low set with a
        // copy of its composite.
        assert_eq!(sel.blocks[2].state, 0xa00);
        assert_eq!(sel.blocks[2].set, Some(5));
        assert_eq!(sel.blocks[0].state, 0x500);
        assert_eq!(sel.blocks[0].set, Some(63));
        assert_eq!(sel.pools.high_free(), 0);
        assert_eq!(sel.pools.low_free(), 42);
    }

    #[test]
    fn a_low_set_is_traded_for_a_high_one_when_the_pool_is_empty() {
        // The same, from a block that itself turns high while holding a low
        // set (0x600): its old low set is reused for the holder's copy.
        let mut sel = selector(&[Vec2::ZERO, Vec2::new(1.0, 0.0)]);
        sel.selected = vec![0, 1];
        sel.cursor = 1;
        sel.pools.high_free.clear();
        sel.blocks[0].state = 0x900;
        sel.blocks[0].set = Some(5);
        sel.blocks[1].state = block_flags::WANTED | block_flags::LOW_SET | block_flags::HIGH_RES;
        sel.blocks[1].set = Some(7);
        assert!(sel.bake(1, 1, sel.blocks[1].state));
        assert_eq!(sel.blocks[1].state, 0xa00);
        assert_eq!(sel.blocks[1].set, Some(5));
        assert_eq!(sel.blocks[0].state, 0x500);
        assert_eq!(sel.blocks[0].set, Some(7));
        assert_eq!(sel.pools.high_free(), 0);
        // The traded low set never left the block pair, so no low set was
        // taken from the pool.
        assert_eq!(sel.pools.low_free(), 43);
    }
}
