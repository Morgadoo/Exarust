//! The five materials the Force and Displacement effect nodes wear, the
//! noise picture the barrier and the aura bind, the material signatures'
//! defaults, the shader programs those materials keep, the node transition
//! each material routine performs ([`dress`], [`MaterialNode`],
//! [`ShaderBook`]) and the shared `gi_multipoint` emissive table
//! ([`GiPrograms`]).
//!
//! Each material routine runs in the same order: it makes the material's
//! shader object once (and registers its release with the frame's deferred
//! list), selects the material's shader parameters, creates the named
//! material, binds the material to the node's first sub-object, and — for
//! the aura and the shell — replaces the node's sub-objects with one new
//! sub-object (buffer usage 35044, the GL static-draw word) whose parent
//! points back at the node. The force and tendril materials set bit 0x20
//! in the node's flags word; the aura and shell materials set bits 0x9000
//! in its state word (the word whose top bit marks a dead node). The barrier and aura materials share one shader signature
//! and also make (and bind) the animated `_ForceNoise` picture.
//!
//! The material signature is a 0x18-byte record: the draw callback, a
//! callback answering the texture layer count (4 for every effect
//! material), and flags 0x480018, then the material's defaults. The force
//! signature holds 6.0, the float pairs (0.16, 0.08) and (0.12, 0.06), the
//! texture `_ForceBarrier01` and the parameters 0.25, 0.25, 0.25, 1.0; the
//! barrier and aura signature holds the noise word 0x2f0, 5.0, the pairs
//! (0.14, 0.08) and (0.10, 0.06), the texture `_ForceAura01` and the same
//! parameters.
//!
//! The noise picture is a 256 by 256 single-channel (0x1903) unsigned-byte
//! field generated from the engine's noise object
//! ([`super::visuals::noise_field`]): four octaves, amplitude 1, both mixing
//! weights one half, then four octaves, 0.8 and 1 (read as a 0.8 s period
//! and a target amplitude of 1). The upload runs only when the noise object
//! reports a new picture. Its parameter word is
//! 0x2f0, or 0x33 while either display switch is on; the upload generates
//! mipmaps only when bit 0x40 is set, so the reduced word does not. The
//! texture is created with internal format 0x8229 (one byte a channel) and
//! 0x10000 bytes.
//!
//! Evidence: the five material routines, their signature records, the noise
//! picture's creation and upload and the deferred shader release, read from
//! `Exanima.exe`.

pub use super::force::{
    BARRIER_PARAM_SIZE as BARRIER_BLOCK, FORCE_AURA_MATERIAL as AURA_MATERIAL,
    FORCE_BARRIER_MATERIAL as BARRIER_MATERIAL, FORCE_MATERIAL, FORCE_PARAM_SIZE as FULL_BLOCK,
    MATERIAL_PARAMS, NOISE_SIZE, NOISE_TEXTURE, NOISE_WORD, NOISE_WORD_REDUCED,
};
pub use super::visuals::{NOISE_FLAGS, NOISE_USAGE, SHELL_FLAG, SHELL_MATERIAL, TENDRIL_MATERIAL};
pub use crate::effects::effect_geometry::SUB_OBJECT_KIND as SUB_OBJECT_USAGE;

/// The force texture the force material's signature names.
pub const FORCE_SIGNATURE_TEXTURE: &str = "_ForceBarrier01";
/// The aura texture the shared barrier and aura signature names.
pub const AURA_SIGNATURE_TEXTURE: &str = "_ForceAura01";

/// The signature record's flags word.
pub const SIGNATURE_FLAGS: u32 = 0x0048_0018;
/// The layer count every effect material's signature answers.
pub const TEXTURE_LAYERS: u32 = 4;

/// Its side, in pixels.
pub const NOISE_SIDE: u32 = NOISE_SIZE.0;
/// Its bytes: one channel a pixel.
pub const NOISE_BYTES: usize = 0x1_0000;
/// The texture target.
pub const TEXTURE_2D: u32 = 0x0de1;
/// The internal format: one unsigned byte a channel.
pub const NOISE_INTERNAL: u32 = 0x8229;
/// The pixel format: red.
pub const NOISE_FORMAT: u32 = 0x1903;
/// The pixel type: unsigned byte.
pub const NOISE_TYPE: u32 = 0x1401;
/// The mipmap bit of the parameter word.
pub const NOISE_MIPMAP: u32 = 0x40;

/// The flag the force and tendril materials add to their node's flags word.
pub const MATERIAL_FLAG: u32 = 0x20;
/// The bits the aura and shell materials add to their node's state word
/// (the word whose top bit marks a dead node), not to its flags word.
pub const AURA_FLAGS: u32 = 0x9000;
/// The flags-word bit the shared displacement shell's creator adds when it
/// is asked for a shared shell.
pub const SHARED_SHELL_FLAG: u32 = 0x0004_0000;

/// One of the five effect materials.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MaterialKind {
    Force,
    Barrier,
    Aura,
    Tendril,
    Shell,
}

/// What a material routine does to the node it is bound to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialBinding {
    /// The material's name.
    pub name: &'static str,
    /// The parameter block size.
    pub block: usize,
    /// The bit the routine adds to the node's flags word, if any.
    pub node_flag: u32,
    /// The bits the routine adds to the node's state word, if any.
    pub state_flags: u32,
    /// The routine replaces the node's sub-objects with one new sub-object
    /// (buffer usage [`SUB_OBJECT_USAGE`], the GL static-draw word), points
    /// its parent back at the node and binds the material to it. The other
    /// routines bind to the first sub-object the geometry already has.
    pub adds_subobject: bool,
    /// The routine makes and binds the `_ForceNoise` picture.
    pub noise: bool,
}

impl MaterialKind {
    /// The material's name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Force => FORCE_MATERIAL,
            Self::Barrier => BARRIER_MATERIAL,
            Self::Aura => AURA_MATERIAL,
            Self::Tendril => TENDRIL_MATERIAL,
            Self::Shell => SHELL_MATERIAL,
        }
    }

    /// The routine's binding.
    pub const fn binding(self) -> MaterialBinding {
        match self {
            Self::Force => MaterialBinding {
                name: FORCE_MATERIAL,
                block: FULL_BLOCK,
                node_flag: MATERIAL_FLAG,
                state_flags: 0,
                adds_subobject: false,
                noise: false,
            },
            Self::Barrier => MaterialBinding {
                name: BARRIER_MATERIAL,
                block: BARRIER_BLOCK,
                node_flag: 0,
                state_flags: 0,
                adds_subobject: false,
                noise: true,
            },
            Self::Aura => MaterialBinding {
                name: AURA_MATERIAL,
                block: FULL_BLOCK,
                node_flag: 0,
                state_flags: AURA_FLAGS,
                adds_subobject: true,
                noise: true,
            },
            Self::Tendril => MaterialBinding {
                name: TENDRIL_MATERIAL,
                block: FULL_BLOCK,
                node_flag: MATERIAL_FLAG,
                state_flags: 0,
                adds_subobject: false,
                noise: false,
            },
            Self::Shell => MaterialBinding {
                name: SHELL_MATERIAL,
                block: FULL_BLOCK,
                node_flag: 0,
                state_flags: AURA_FLAGS,
                adds_subobject: true,
                noise: false,
            },
        }
    }

    /// The draw callback its signature names, as the pass planner's kind.
    pub const fn draw(self) -> super::effect_pass::PassKind {
        use super::effect_pass::PassKind;
        match self {
            Self::Force => PassKind::Material,
            Self::Barrier | Self::Aura => PassKind::Shader,
            Self::Tendril => PassKind::Tendril,
            Self::Shell => PassKind::Shell,
        }
    }
}

/// One material signature's defaults.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Signature {
    pub flags: u32,
    /// The layer count the signature's second callback answers.
    pub layers: u32,
    /// The scalar the signature holds where the force record has one (6)
    /// and the barrier and aura record another (5). Its reader is outside
    /// this unit, so its meaning is a guess.
    pub constant: f32,
    /// The two float pairs the record holds, in its order.
    pub levels: [[f32; 2]; 2],
    /// The texture the signature names.
    pub texture: Option<&'static str>,
    /// The parameters the material binds.
    pub params: [f32; 4],
}

/// The force material's signature.
pub const FORCE_SIGNATURE: Signature = Signature {
    flags: SIGNATURE_FLAGS,
    layers: TEXTURE_LAYERS,
    constant: 6.0,
    levels: [[0.16, 0.08], [0.12, 0.06]],
    texture: Some(FORCE_SIGNATURE_TEXTURE),
    params: MATERIAL_PARAMS,
};

/// The barrier and aura materials' signature.
pub const AURA_SIGNATURE: Signature = Signature {
    flags: SIGNATURE_FLAGS,
    layers: TEXTURE_LAYERS,
    constant: 5.0,
    levels: [[0.14, 0.08], [0.10, 0.06]],
    texture: Some(AURA_SIGNATURE_TEXTURE),
    params: MATERIAL_PARAMS,
};

/// The signature a material uses; the tendril and shell signatures' extra
/// words are geometry data (bounds and sub-object kinds), not material
/// defaults, and are recorded with their geometries.
pub fn signature(kind: MaterialKind) -> Option<Signature> {
    match kind {
        MaterialKind::Force => Some(FORCE_SIGNATURE),
        MaterialKind::Barrier | MaterialKind::Aura => Some(AURA_SIGNATURE),
        MaterialKind::Tendril | MaterialKind::Shell => None,
    }
}

/// The four shader objects the material routines make: the force material's,
/// the barrier and aura materials' shared one, the tendril's and the
/// displacement shell's. Each is made once and never freed by these units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderSlot {
    Force,
    BarrierAura,
    Tendril,
    Shell,
}

/// The two release callbacks the material routines register with the
/// deferred list, each once per run behind its own "registered" byte: the
/// Force unit's (registered by whichever of the force, barrier and aura
/// materials is made first; it is the unit's shutdown free, the twelve
/// program handles and the GI table) and the Displacement unit's
/// (registered by the tendril or shell material; it frees two program
/// handles). Neither frees the material shader objects themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReleaseUnit {
    Force,
    Displacement,
}

impl ReleaseUnit {
    /// The program handles the callback frees when every one is live. The
    /// Force callback also frees the GI table ([`GiPrograms::cleanup`]).
    pub const fn handles(self) -> usize {
        match self {
            Self::Force => GLOBAL_HANDLES,
            Self::Displacement => TENDRIL_HANDLES,
        }
    }
}

impl ShaderSlot {
    /// The release callback this slot's material registers.
    pub const fn unit(self) -> ReleaseUnit {
        match self {
            Self::Force | Self::BarrierAura => ReleaseUnit::Force,
            Self::Tendril | Self::Shell => ReleaseUnit::Displacement,
        }
    }
}

/// A material node as the five material routines leave it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MaterialNode {
    /// The material created for the node, if any.
    pub material: Option<&'static str>,
    /// The material's parameter block size.
    pub block: usize,
    /// The geometry node's flags word (`+0xc`).
    pub flags: u32,
    /// The geometry node's render word (`+8`); the aura and shell add
    /// [`AURA_FLAGS`].
    pub state: u32,
    /// The node's sub-objects. The aura and shell replace them with one.
    pub subobjects: usize,
    /// The replacement sub-object's buffer usage.
    pub subobject_usage: i32,
    /// The replacement sub-object's parent points back at the node.
    pub subobject_parent: bool,
    /// The noise picture bound to the material's `+0x70` (the barrier and
    /// the aura bind it; the shell does not).
    pub noise: Option<&'static str>,
}

/// What one material routine did to its node and the shader table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaterialOutcome {
    pub slot: ShaderSlot,
    /// This call made the shader object.
    pub shader_created: bool,
    /// This call made the `_ForceNoise` picture.
    pub noise_created: bool,
}

/// The shader objects the effect materials hold, made once each; the
/// barrier and the aura share one signature.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShaderBook {
    pub made: Vec<ShaderSlot>,
    /// The release callbacks registered with the deferred list, once each.
    pub releasing: Vec<ReleaseUnit>,
    /// The callbacks that have run; each zeroes the handles it frees, so a
    /// second run frees none.
    pub released: Vec<ReleaseUnit>,
    /// The `_ForceNoise` picture exists.
    pub noise: bool,
}

impl ShaderBook {
    /// Makes the slot's shader object if the table has none and, the first
    /// time any material of its unit is made, registers the unit's release
    /// callback. Answers whether this call made the shader object.
    pub fn ensure(&mut self, slot: ShaderSlot) -> bool {
        if self.made.contains(&slot) {
            return false;
        }
        self.made.push(slot);
        if !self.releasing.contains(&slot.unit()) {
            self.releasing.push(slot.unit());
        }
        true
    }

    /// The material routine's noise creation: the barrier and the aura call
    /// it before they make their shader. Answers whether this call made it.
    pub fn ensure_noise(&mut self) -> bool {
        if self.noise {
            return false;
        }
        self.noise = true;
        true
    }

    /// Runs the release callback `slot`'s material registered. The shader
    /// objects stay made (the routines never make them again, and the
    /// "registered" byte keeps the callback from being added twice); the
    /// callback frees its unit's program handles and zeroes them, so a
    /// second run frees none. Answers the handles freed.
    pub fn release(&mut self, slot: ShaderSlot) -> usize {
        let unit = slot.unit();
        if !self.releasing.contains(&unit) || self.released.contains(&unit) {
            return 0;
        }
        self.released.push(unit);
        unit.handles()
    }

    /// Shutdown: every registered callback runs.
    pub fn release_all(&mut self) -> usize {
        let units = self.releasing.clone();
        units
            .iter()
            .map(|unit| match unit {
                ReleaseUnit::Force => self.release(ShaderSlot::Force),
                ReleaseUnit::Displacement => self.release(ShaderSlot::Tendril),
            })
            .sum()
    }
}

impl MaterialKind {
    /// The shader object this material's routine makes.
    pub const fn shader_slot(self) -> ShaderSlot {
        match self {
            Self::Force => ShaderSlot::Force,
            Self::Barrier | Self::Aura => ShaderSlot::BarrierAura,
            Self::Tendril => ShaderSlot::Tendril,
            Self::Shell => ShaderSlot::Shell,
        }
    }
}

/// Dresses `node` as one of the five material routines does, in the
/// routine's own order: the noise picture (the barrier and the aura), the
/// shader object (made once, its release deferred), the parameter block, the
/// node's flag or state bits, the sub-object replacement (the aura and the
/// shell), the material, and the noise bound to the material's `+0x70`.
pub fn dress(
    kind: MaterialKind,
    node: &mut MaterialNode,
    book: &mut ShaderBook,
) -> MaterialOutcome {
    let binding = kind.binding();
    let noise_created = binding.noise && book.ensure_noise();
    let shader_created = book.ensure(kind.shader_slot());
    node.block = binding.block;
    node.flags |= binding.node_flag;
    node.state |= binding.state_flags;
    if binding.adds_subobject {
        node.subobjects = 1;
        node.subobject_usage = SUB_OBJECT_USAGE;
        node.subobject_parent = true;
    }
    node.material = Some(binding.name);
    if binding.noise {
        node.noise = Some(NOISE_TEXTURE);
    }
    MaterialOutcome {
        slot: kind.shader_slot(),
        shader_created,
        noise_created,
    }
}

/// The shared emissive program's name.
pub const GI_NAME: &str = "gi_multipoint";
/// The flags word of a GI entry whose program could not be made.
pub const GI_FAILED: u32 = 0x8000_0000;
/// The flags word a new GI entry starts with: its id with bit 30.
pub const GI_CREATED: u32 = 0x4000_0000;
/// The uniform names the program binds.
pub const GI_UNIFORMS: [&str; 2] = ["Light", "Atten"];
/// The parameter block the program's two uniform locations live in.
pub const GI_PARAMS: usize = 0x14;
/// The render state a new GI entry leaves: mode 0, flags failed, word 0,
/// the two values 0x100.
pub const GI_START_WORD: u32 = 0x100;

/// One GI program the emissive geometries share, looked up by a material's
/// id word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GiProgram {
    pub handle: u32,
    pub params: u32,
    pub light: u32,
    pub atten: u32,
}

/// One entry of the shared GI program table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GiProgramEntry {
    pub id: u32,
    pub flags: u32,
    pub program: Option<GiProgram>,
}

/// The state word the GI manager keeps for the program in use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GiState {
    pub id: u32,
    pub mode: u32,
    pub flags: u32,
    pub word: u32,
    pub first: u32,
    pub scale: u32,
}

impl Default for GiState {
    fn default() -> Self {
        Self {
            id: 0,
            mode: 0,
            flags: GI_FAILED,
            word: 0,
            first: GI_START_WORD,
            scale: GI_START_WORD,
        }
    }
}

/// The emissive program table. An id is looked up in order; a found entry
/// whose flags are [`GI_FAILED`] answers none, any other is made current.
/// A missing id appends an entry, sets the manager's state word and asks
/// [`GI_NAME`] to be made; a failed make marks the entry [`GI_FAILED`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GiPrograms {
    pub entries: Vec<GiProgramEntry>,
    pub current: Option<usize>,
    pub state: GiState,
}

impl GiPrograms {
    /// Looks `id` up and makes its program when missing. `make` is the
    /// renderer's program creation and uniform lookup, keyed by [`GI_NAME`];
    /// it answers none when the program cannot be made.
    pub fn acquire(
        &mut self,
        id: u32,
        make: impl FnOnce(&'static str, [&'static str; 2]) -> Option<GiProgram>,
    ) -> Option<usize> {
        if let Some(at) = self.entries.iter().position(|entry| entry.id == id) {
            if self.entries[at].flags == GI_FAILED {
                return None;
            }
            self.current = Some(at);
            return Some(at);
        }
        let flags = id | GI_CREATED;
        self.entries.push(GiProgramEntry {
            id,
            flags,
            program: None,
        });
        let at = self.entries.len() - 1;
        self.state = GiState {
            id,
            ..GiState::default()
        };
        match make(GI_NAME, GI_UNIFORMS) {
            Some(program) => {
                self.entries[at].program = Some(program);
                self.current = Some(at);
                Some(at)
            }
            None => {
                self.entries[at].flags = GI_FAILED;
                None
            }
        }
    }

    /// Shutdown: the twelve global handles and each entry's program and
    /// parameter block are freed, then the table empties. A failed entry
    /// holds neither, so it frees nothing. Answers how many handles went.
    pub fn cleanup(&mut self) -> usize {
        let made = self
            .entries
            .iter()
            .filter(|entry| entry.program.is_some())
            .count();
        let freed = GLOBAL_HANDLES + made * 2;
        self.entries.clear();
        self.current = None;
        self.state = GiState::default();
        freed
    }
}

/// The noise picture's creation: its generator settings and its texture
/// object's words.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoiseTexture {
    pub name: &'static str,
    /// The generator's width, height and octaves.
    pub size: (u32, u32, u32),
    /// The generator's amplitude.
    pub amplitude: f32,
    /// The generator's two mixing weights.
    pub mix: (f32, f32),
    /// The phase length, seconds.
    pub phase: f32,
    /// The parameter word, which also says whether it mipmaps.
    pub word: u32,
    /// The usage word and the sampler word.
    pub usage: u32,
    pub sampler: u32,
    /// The texture target and internal format.
    pub target: u32,
    pub internal: u32,
    /// The pixel format and type.
    pub format: u32,
    pub kind: u32,
    /// The bytes the generator fills.
    pub bytes: usize,
}

impl NoiseTexture {
    /// The picture as the material makes it. `reduced` is either display
    /// switch being on.
    pub const fn new(reduced: bool) -> Self {
        Self {
            name: NOISE_TEXTURE,
            size: NOISE_SIZE,
            amplitude: 1.0,
            mix: (0.5, 0.5),
            phase: 0.8,
            word: if reduced {
                NOISE_WORD_REDUCED
            } else {
                NOISE_WORD
            },
            usage: NOISE_USAGE,
            sampler: NOISE_FLAGS,
            target: TEXTURE_2D,
            internal: NOISE_INTERNAL,
            format: NOISE_FORMAT,
            kind: NOISE_TYPE,
            bytes: NOISE_BYTES,
        }
    }

    /// Whether the upload generates mipmaps: the parameter word's bit 0x40.
    pub const fn mipmaps(&self) -> bool {
        self.word & NOISE_MIPMAP != 0
    }
}

impl Default for NoiseTexture {
    fn default() -> Self {
        Self::new(false)
    }
}

/// One upload of the noise picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoiseUpload {
    pub target: u32,
    pub level: i32,
    pub internal: u32,
    pub width: u32,
    pub height: u32,
    pub border: i32,
    pub format: u32,
    pub kind: u32,
    pub mipmaps: bool,
}

/// The upload of `pixels` into the picture (a sub-image of level 0). It
/// runs only when the noise object's tick reported a new picture (`fresh`,
/// [`exa_render::image_noise::NoiseField::tick`]). The picture holds exactly
/// [`NOISE_BYTES`] bytes; any other length is refused.
pub fn noise_upload(texture: &NoiseTexture, fresh: bool, pixels: &[u8]) -> Option<NoiseUpload> {
    if !fresh || pixels.len() != texture.bytes {
        return None;
    }
    Some(NoiseUpload {
        target: texture.target,
        level: 0,
        internal: texture.internal,
        width: NOISE_SIDE,
        height: NOISE_SIDE,
        border: 0,
        format: texture.format,
        kind: texture.kind,
        mipmaps: texture.mipmaps(),
    })
}

/// The shader programs the effect materials keep. The original frees twelve
/// global program handles and the material table's entries when
/// the renderer shuts down, then empties the table. Exarust's renderer owns
/// the GPU objects; this keeps the material names so the table's lifecycle
/// is still testable.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EffectShaders {
    /// The materials the five routines made, once each.
    pub materials: Vec<&'static str>,
    /// The two draw signatures the displacement material owns.
    pub tendril: Vec<&'static str>,
}

/// The twelve global program handles the Force unit's shutdown frees.
pub const GLOBAL_HANDLES: usize = 12;
/// The two tendril handles the displacement material releases.
pub const TENDRIL_HANDLES: usize = 2;

impl EffectShaders {
    /// Registers a material, once.
    pub fn register(&mut self, material: &'static str) {
        if !self.materials.contains(&material) {
            self.materials.push(material);
        }
    }

    /// Releases the effect shaders: the twelve global handles and the
    /// material table's entries are freed and the table emptied, then the
    /// tendril material frees its two. Returns how many handles went.
    pub fn release(&mut self) -> usize {
        let freed = GLOBAL_HANDLES + self.materials.len() + self.tendril.len();
        self.materials.clear();
        self.tendril.clear();
        freed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_material_binds_its_block_and_flags() {
        let force = MaterialKind::Force.binding();
        assert_eq!(force.name, "_ForceMat01");
        assert_eq!(force.block, 0xf0);
        assert_eq!(force.node_flag, 0x20);
        assert!(!force.adds_subobject);
        assert!(!force.noise);
        let barrier = MaterialKind::Barrier.binding();
        assert_eq!((barrier.name, barrier.block), ("_ForceBarrier01", 0xb8));
        assert_eq!(barrier.node_flag, 0);
        assert!(barrier.noise);
        let aura = MaterialKind::Aura.binding();
        assert_eq!((aura.name, aura.block), ("_ForceAura01", 0xf0));
        assert_eq!((aura.node_flag, aura.state_flags), (0, 0x9000));
        assert!(aura.adds_subobject);
        assert!(aura.noise);
        let tendril = MaterialKind::Tendril.binding();
        assert_eq!((tendril.name, tendril.node_flag), ("_TDispMat01", 0x20));
        let shell = MaterialKind::Shell.binding();
        assert_eq!((shell.name, shell.block), ("_EffectShell01", 0xf0));
        assert_eq!((shell.node_flag, shell.state_flags), (0, 0x9000));
        assert!(shell.adds_subobject);
        assert!(!shell.noise);
        assert_eq!(SUB_OBJECT_USAGE, 35_044);
        assert_eq!(force.state_flags, 0);
        assert_eq!(MATERIAL_PARAMS, [0.25, 0.25, 0.25, 1.0]);
    }

    #[test]
    fn the_signatures_hold_the_original_defaults() {
        let force = signature(MaterialKind::Force).expect("a signature");
        assert_eq!(force.constant, 6.0);
        assert_eq!(force.levels, [[0.16, 0.08], [0.12, 0.06]]);
        assert_eq!(force.texture, Some("_ForceBarrier01"));
        assert_eq!(force.layers, 4);
        assert_eq!(force.flags, 0x0048_0018);
        let aura = signature(MaterialKind::Aura).expect("a signature");
        assert_eq!(signature(MaterialKind::Barrier), Some(aura));
        assert_eq!(aura.constant, 5.0);
        assert_eq!(aura.levels, [[0.14, 0.08], [0.10, 0.06]]);
        assert_eq!(aura.texture, Some("_ForceAura01"));
        assert_eq!(signature(MaterialKind::Tendril), None);
        assert_eq!(signature(MaterialKind::Shell), None);
    }

    #[test]
    fn the_noise_picture_mipmaps_only_with_bit_forty() {
        let full = NoiseTexture::new(false);
        assert_eq!(full.size, (256, 256, 4));
        assert_eq!(
            (full.amplitude, full.mix, full.phase),
            (1.0, (0.5, 0.5), 0.8)
        );
        assert_eq!(full.word, 0x2f0);
        assert!(full.mipmaps());
        let reduced = NoiseTexture::new(true);
        assert_eq!(reduced.word, 0x33);
        assert!(!reduced.mipmaps());
        assert_eq!((full.usage, full.sampler), (0x2007_0000, 0x0100_4200));
        assert_eq!((full.target, full.internal), (0xde1, 0x8229));
        assert_eq!((full.format, full.kind), (0x1903, 0x1401));
        assert_eq!(full.bytes, 0x1_0000);
    }

    #[test]
    fn an_upload_needs_the_whole_picture() {
        let texture = NoiseTexture::default();
        let pixels = vec![0u8; NOISE_BYTES];
        let upload = noise_upload(&texture, true, &pixels).expect("an upload");
        assert_eq!(upload.width, 256);
        assert_eq!(upload.height, 256);
        assert_eq!((upload.level, upload.border), (0, 0));
        assert!(upload.mipmaps);
        assert!(noise_upload(&texture, true, &pixels[1..]).is_none());
        assert!(noise_upload(&texture, false, &pixels).is_none());
        let reduced = NoiseTexture::new(true);
        assert!(
            !noise_upload(&reduced, true, &pixels)
                .expect("an upload")
                .mipmaps
        );
    }

    #[test]
    fn the_material_routines_dress_the_node_in_their_order() {
        let mut book = ShaderBook::default();
        let mut force = MaterialNode::default();
        let made = dress(MaterialKind::Force, &mut force, &mut book);
        assert_eq!(force.material, Some("_ForceMat01"));
        assert_eq!((force.block, force.flags, force.state), (0xf0, 0x20, 0));
        assert_eq!(force.subobjects, 0);
        assert_eq!(force.noise, None);
        assert!(made.shader_created && !made.noise_created);
        assert_eq!(made.slot, ShaderSlot::Force);

        // The barrier makes the noise and its shader, and binds the picture
        // to the material.
        let mut barrier = MaterialNode::default();
        let made = dress(MaterialKind::Barrier, &mut barrier, &mut book);
        assert_eq!((barrier.block, barrier.flags, barrier.state), (0xb8, 0, 0));
        assert_eq!(barrier.noise, Some("_ForceNoise"));
        assert!(made.noise_created);

        // The aura shares the barrier's shader and the noise; it replaces
        // the sub-objects with one and sets bit 0x9000 in the state word.
        let mut aura = MaterialNode::default();
        let made = dress(MaterialKind::Aura, &mut aura, &mut book);
        assert_eq!(aura.state, 0x9000);
        assert_eq!((aura.subobjects, aura.subobject_usage), (1, 35_044));
        assert!(aura.subobject_parent);
        assert_eq!(aura.noise, Some("_ForceNoise"));
        assert!(!made.shader_created, "the barrier made the shared shader");
        assert!(!made.noise_created);
        assert_eq!(book.made, vec![ShaderSlot::Force, ShaderSlot::BarrierAura]);

        // The shell keeps its own shader and adds no noise; the tendril
        // sets the flag bit and keeps the geometry's sub-object.
        let mut shell = MaterialNode::default();
        let made = dress(MaterialKind::Shell, &mut shell, &mut book);
        assert_eq!(
            (shell.material, shell.state),
            (Some("_EffectShell01"), 0x9000)
        );
        assert_eq!((shell.subobjects, shell.noise), (1, None));
        assert!(made.shader_created);
        let mut tendril = MaterialNode::default();
        dress(MaterialKind::Tendril, &mut tendril, &mut book);
        assert_eq!((tendril.flags, tendril.state), (0x20, 0));
        assert_eq!(book.made.len(), 4);
    }

    #[test]
    fn the_shader_book_releases_each_slot_once() {
        let mut book = ShaderBook::default();
        assert!(book.ensure(ShaderSlot::Tendril));
        assert!(!book.ensure(ShaderSlot::Tendril));
        assert!(book.ensure(ShaderSlot::Shell));
        // The tendril and shell register one callback between them; it
        // frees the two program handles once, and the shaders stay made.
        assert_eq!(book.releasing, vec![ReleaseUnit::Displacement]);
        assert_eq!(book.release(ShaderSlot::Tendril), TENDRIL_HANDLES);
        assert_eq!(book.release(ShaderSlot::Tendril), 0);
        assert_eq!(book.release(ShaderSlot::Shell), 0);
        assert_eq!(book.made, vec![ShaderSlot::Tendril, ShaderSlot::Shell]);
        assert!(!book.ensure(ShaderSlot::Shell));
        // The force, barrier and aura materials share the Force unit's one
        // shutdown callback, the twelve program handles.
        assert!(book.ensure(ShaderSlot::Force));
        assert!(book.ensure(ShaderSlot::BarrierAura));
        assert_eq!(
            book.releasing,
            vec![ReleaseUnit::Displacement, ReleaseUnit::Force]
        );
        assert_eq!(book.release_all(), 12);
        assert_eq!(book.release(ShaderSlot::BarrierAura), 0);
        assert_eq!(book.release_all(), 0);
    }

    #[test]
    fn the_gi_table_creates_once_and_binds_its_uniforms() {
        let mut table = GiPrograms::default();
        let mut calls = 0;
        let made = table.acquire(0x201, |name, uniforms| {
            calls += 1;
            assert_eq!(name, "gi_multipoint");
            assert_eq!(uniforms, ["Light", "Atten"]);
            Some(GiProgram {
                handle: 7,
                params: 8,
                light: 9,
                atten: 10,
            })
        });
        assert_eq!(made, Some(0));
        assert_eq!(table.entries[0].flags, 0x4000_0201);
        assert_eq!(table.state.id, 0x201);
        assert_eq!((table.state.mode, table.state.flags), (0, GI_FAILED));
        assert_eq!((table.state.first, table.state.scale), (0x100, 0x100));
        // A second ask finds the entry and makes nothing.
        let _ = table.acquire(0x201, |_, _| {
            calls += 1;
            None
        });
        assert_eq!(calls, 1);
        assert_eq!(table.entries.len(), 1);

        // A program that cannot be made marks the entry failed and answers
        // none for it from then on.
        assert_eq!(table.acquire(0x202, |_, _| None), None);
        assert_eq!(table.entries[1].flags, GI_FAILED);
        assert_eq!(table.acquire(0x202, |_, _| None), None);
        // Twelve globals and the one made entry's program and block; the
        // failed entry holds neither.
        assert_eq!(table.cleanup(), 12 + 2);
        assert!(table.entries.is_empty());
    }

    #[test]
    fn the_shader_table_releases_its_handles() {
        let mut shaders = EffectShaders {
            materials: vec!["_ForceMat01", "_ForceBarrier01", "_ForceAura01"],
            tendril: vec!["tendril", "shell"],
        };
        assert_eq!(shaders.release(), 12 + 3 + 2);
        assert!(shaders.materials.is_empty());
        assert!(shaders.tendril.is_empty());
        shaders.register("_ForceMat01");
        shaders.register("_ForceMat01");
        assert_eq!(shaders.materials, vec!["_ForceMat01"]);
        assert_eq!(TENDRIL_HANDLES, 2);
        assert_eq!(
            MaterialKind::Aura.draw(),
            super::super::effect_pass::PassKind::Shader
        );
    }
}
