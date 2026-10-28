//! The five materials the Force and Displacement effect nodes wear, the
//! noise picture the barrier and the aura bind, the material signatures'
//! defaults, and the shader programs those materials keep.
//!
//! Each material routine runs in the same order: it makes the material's
//! shader object once (and registers its release with the frame's deferred
//! list), selects the material's shader parameters, creates the named
//! material, binds the material to the node's first sub-object, and — for
//! the aura and the shell — gives the node one sub-object of kind 35044
//! whose parent points back at the node. The force and tendril materials
//! mark the node with flag 0x20; the aura and shell materials mark it with
//! flags 0x9000. The barrier and aura materials share one shader signature
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
//! field generated from the engine's noise object: four octaves, a 0.8 s
//! phase, amplitude 1, both mixing weights one half. Its parameter word is
//! 0x2f0, or 0x33 while either display switch is on; the upload generates
//! mipmaps only when bit 0x40 is set, so the reduced word does not. The
//! texture is created with internal format 0x8229 (one byte a channel) and
//! 0x10000 bytes.
//!
//! Evidence: the five material routines, their signature records, the noise
//! picture's creation and upload and the deferred shader release, read from
//! `Exanima.exe`.

/// The material the blast family wears.
pub const FORCE_MATERIAL: &str = "_ForceMat01";
/// The material the domes and the barricade wear.
pub const BARRIER_MATERIAL: &str = "_ForceBarrier01";
/// The material the shell (and its decorative copy) wears.
pub const AURA_MATERIAL: &str = "_ForceAura01";
/// The material the tendrils wear.
pub const TENDRIL_MATERIAL: &str = "_TDispMat01";
/// The material the displacement shell wears.
pub const SHELL_MATERIAL: &str = "_EffectShell01";

/// The force texture the force material's signature names.
pub const FORCE_SIGNATURE_TEXTURE: &str = "_ForceBarrier01";
/// The aura texture the shared barrier and aura signature names.
pub const AURA_SIGNATURE_TEXTURE: &str = "_ForceAura01";

/// The parameter block size the force, aura, tendril and shell materials
/// keep.
pub const FULL_BLOCK: usize = 0xf0;
/// The barrier material's parameter block size.
pub const BARRIER_BLOCK: usize = 0xb8;
/// The four parameters the force and aura materials bind.
pub const MATERIAL_PARAMS: [f32; 4] = [0.25, 0.25, 0.25, 1.0];
/// The signature record's flags word.
pub const SIGNATURE_FLAGS: u32 = 0x0048_0018;
/// The layer count every effect material's signature answers.
pub const TEXTURE_LAYERS: u32 = 4;

/// The animated noise picture's name.
pub const NOISE_TEXTURE: &str = "_ForceNoise";
/// Its side, in pixels.
pub const NOISE_SIDE: u32 = 0x100;
/// The octaves its generator is set to.
pub const NOISE_OCTAVES: u32 = 4;
/// Its bytes: one channel a pixel.
pub const NOISE_BYTES: usize = 0x1_0000;
/// The parameter word with mipmaps.
pub const NOISE_WORD: u32 = 0x2f0;
/// The parameter word the display switches reduce it to.
pub const NOISE_WORD_REDUCED: u32 = 0x33;
/// The texture's usage word.
pub const NOISE_USAGE: u32 = 0x2007_0000;
/// Its sampler word.
pub const NOISE_FLAGS: u32 = 0x0100_4200;
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

/// The kind word of the sub-object the aura and shell materials add.
pub const SUB_OBJECT_KIND: u32 = 35_044;
/// The flag the force and tendril materials add to their node.
pub const MATERIAL_FLAG: u32 = 0x20;
/// The flags the aura and shell materials add to their node.
pub const AURA_FLAGS: u32 = 0x9000;
/// The shell creator's extra flag, and the shared shell's.
pub const SHELL_FLAG: u32 = 0x20;
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
    /// The flag the routine adds to the node's words, if any.
    pub node_flag: u32,
    /// The routine adds one sub-object of [`SUB_OBJECT_KIND`], points its
    /// parent back at the node and binds the material to it. The other
    /// routines bind to the sub-object the geometry already has.
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
                adds_subobject: false,
                noise: false,
            },
            Self::Barrier => MaterialBinding {
                name: BARRIER_MATERIAL,
                block: BARRIER_BLOCK,
                node_flag: 0,
                adds_subobject: false,
                noise: true,
            },
            Self::Aura => MaterialBinding {
                name: AURA_MATERIAL,
                block: FULL_BLOCK,
                node_flag: AURA_FLAGS,
                adds_subobject: true,
                noise: true,
            },
            Self::Tendril => MaterialBinding {
                name: TENDRIL_MATERIAL,
                block: FULL_BLOCK,
                node_flag: MATERIAL_FLAG,
                adds_subobject: false,
                noise: false,
            },
            Self::Shell => MaterialBinding {
                name: SHELL_MATERIAL,
                block: FULL_BLOCK,
                node_flag: AURA_FLAGS,
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
            size: (NOISE_SIDE, NOISE_SIDE, NOISE_OCTAVES),
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

/// The upload of `pixels` into the picture. The picture holds exactly
/// [`NOISE_BYTES`] bytes; any other length is refused.
pub fn noise_upload(texture: &NoiseTexture, pixels: &[u8]) -> Option<NoiseUpload> {
    if pixels.len() != texture.bytes {
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

/// The shader programs the effect materials keep. The original frees eleven
/// global shader and buffer handles and the material table's entries when
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

/// The eleven global handles the effect unit releases at shutdown.
pub const GLOBAL_HANDLES: usize = 11;
/// The two tendril handles the displacement material releases.
pub const TENDRIL_HANDLES: usize = 2;

impl EffectShaders {
    /// Registers a material, once.
    pub fn register(&mut self, material: &'static str) {
        if !self.materials.contains(&material) {
            self.materials.push(material);
        }
    }

    /// Releases the effect shaders: the eleven global handles and the
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
        assert_eq!(aura.node_flag, 0x9000);
        assert!(aura.adds_subobject);
        assert!(aura.noise);
        let tendril = MaterialKind::Tendril.binding();
        assert_eq!((tendril.name, tendril.node_flag), ("_TDispMat01", 0x20));
        let shell = MaterialKind::Shell.binding();
        assert_eq!((shell.name, shell.block), ("_EffectShell01", 0xf0));
        assert_eq!(shell.node_flag, 0x9000);
        assert!(shell.adds_subobject);
        assert!(!shell.noise);
        assert_eq!(SUB_OBJECT_KIND, 35_044);
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
        let upload = noise_upload(&texture, &pixels).expect("an upload");
        assert_eq!(upload.width, 256);
        assert_eq!(upload.height, 256);
        assert_eq!((upload.level, upload.border), (0, 0));
        assert!(upload.mipmaps);
        assert!(noise_upload(&texture, &pixels[1..]).is_none());
        let reduced = NoiseTexture::new(true);
        assert!(!noise_upload(&reduced, &pixels).expect("an upload").mipmaps);
    }

    #[test]
    fn the_shader_table_releases_its_handles() {
        let mut shaders = EffectShaders {
            materials: vec!["_ForceMat01", "_ForceBarrier01", "_ForceAura01"],
            tendril: vec!["tendril", "shell"],
        };
        assert_eq!(shaders.release(), 11 + 3 + 2);
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
