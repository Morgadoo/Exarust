//! The original's procedural material registry: the 25 materials an item can
//! be painted with, their shader families, passes and library textures, the
//! random sets an object record draws its materials from, and the tables the
//! factory's material and colour rolls read.
//!
//! Each material names a *family*: the program that paints its colour,
//! surface and normal maps. A family may share another's normal program
//! (copper satin uses copper alloy's normals) or have a separate specular
//! family (silk). Pass flags select the pattern, painted-pattern (emblem),
//! alpha and alternate-normal programs.
//!
//! Evidence: the material registry and its singletons, the material class
//! and default finish tables, and the random-set table (`docs/formats/OTHER.md`,
//! Apparel painting and Weapon factories).

use std::sync::LazyLock;

/// Which program family paints a material's maps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Family {
    Cloth,
    Velvet,
    Leather,
    Suede,
    Wood,
    Steel,
    SteelBlade,
    SteelPlate,
    SteelPainted,
    Chain,
    CopperAlloy,
    CopperSatin,
    Gemstone,
    Resin,
    PlanksPainted,
}

/// Optional programs a material's family compiles besides its colour,
/// specular and normal passes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Passes {
    /// The alpha program (`<family>_a`).
    pub alpha: bool,
    /// A pattern pass, which multiplies the mask alpha by another texture.
    pub pattern: bool,
    /// A painted pattern pass, with the pattern's own position.
    pub painted_pattern: bool,
    /// An alternate normal pass.
    pub alternate_normal: bool,
}

/// One procedural material.
#[derive(Debug, Clone)]
pub struct Material {
    pub family: Family,
    pub passes: Passes,
    /// Colour-pass textures in sampler order (`Tex2…`).
    pub colour: Vec<&'static str>,
    /// Specular-pass textures in sampler order.
    pub specular: Vec<&'static str>,
    /// Normal-pass textures in sampler order.
    pub normal: Vec<&'static str>,
    /// How strongly ageing and gloss apply (`MatCol.w`) on apparel; a
    /// factory layer takes its finish level instead.
    pub wear: f32,
}

/// The 25 materials by id, index 0 unused (ids 1..25).
pub static MATERIALS: LazyLock<[Material; 26]> = LazyLock::new(|| {
    let cloth = |colour, spec, normal| Material {
        family: Family::Cloth,
        passes: Passes::default(),
        colour,
        specular: spec,
        normal,
        wear: 1.0,
    };
    let mut m = std::array::from_fn::<_, 26, _>(|_| cloth(Vec::new(), Vec::new(), Vec::new()));
    m[1] = Material {
        passes: Passes {
            painted_pattern: true,
            ..default_passes()
        },
        ..cloth(
            vec!["clothrgha03d", "leatherdmga02d", "clothdmga01d", "drtapr01"],
            vec!["clothrgha03s", "leatherdmga02d", "drtapr01"],
            vec!["clothrgha03n", "leatherdmga02d", "leatherdmga02n"],
        )
    };
    let cloth_fine = |family, normal0| Material {
        family,
        passes: Passes::default(),
        colour: vec!["clothfina01d", "leatherdmga02d", "clothdmga01d", "drtapr01"],
        specular: vec!["clothfina01g", "leatherdmga02d", "drtapr01"],
        normal: vec![normal0, "leatherdmga02d", "leatherdmga02n"],
        wear: 1.0,
    };
    m[2] = cloth_fine(Family::Cloth, "clothfina01n");
    m[3] = cloth_fine(Family::Cloth, "clothfina01n");
    m[4] = Material {
        wear: 0.8,
        ..cloth_fine(Family::Velvet, "clothvlva01n")
    };
    m[5] = Material {
        family: Family::Cloth,
        passes: Passes::default(),
        colour: vec!["clthp01d", "leatherdmga02d", "clothdmga01d", "drtapr01"],
        specular: vec!["clthp01s", "leatherdmga02d", "drtapr01"],
        normal: vec!["clthp01n", "leatherdmga02d", "leatherdmga02n"],
        wear: 1.0,
    };
    m[6] = Material {
        colour: vec!["clthp02d", "leatherdmga02d", "clothdmga01d", "drtapr01"],
        specular: vec!["clthp02s", "leatherdmga02d", "drtapr01"],
        normal: vec!["clthp02n", "leatherdmga02d", "leatherdmga02n"],
        ..m[5].clone()
    };
    let leather = |stem: &'static str, painted: bool| Material {
        family: Family::Leather,
        passes: Passes {
            painted_pattern: painted,
            ..default_passes()
        },
        colour: vec![stem_colour(stem), "leatherdmga02d", "drtapr01"],
        specular: vec![stem_spec(stem), "leatherdmga02d", "drtapr01"],
        normal: vec![stem_normal(stem), "leatherdmga02d", "leatherdmga02n"],
        wear: 1.0,
    };
    m[7] = leather("leathera02", true);
    m[8] = leather("leathera01", true);
    let suede = Material {
        family: Family::Suede,
        passes: Passes::default(),
        colour: vec!["suedea01d", "leatherdmga02d", "drtapr01"],
        specular: vec!["suedea01g", "leatherdmga02d", "drtapr01"],
        normal: vec!["suedea01n", "leatherdmga02d", "leatherdmga02n"],
        wear: 1.0,
    };
    m[9] = suede.clone();
    m[10] = suede;
    m[11] = Material {
        family: Family::Chain,
        passes: Passes::default(),
        colour: vec![
            "mailstla01d",
            "mtldirta01d",
            "mtlrusta02d",
            "mtlscufa01",
            "mtlrusta01g",
        ],
        specular: vec![
            "mailstla01d",
            "mailstla01o",
            "mtlscufa01",
            "mtlrusta01g",
            "scuffsgnra01",
        ],
        normal: vec![
            "mailstla01n",
            "mtlirnrst01n",
            "mtlscufa01",
            "mtlrusta01g",
            "mtlscufa01n",
        ],
        wear: 1.0,
    };
    // SteelGen and IronGen take the A01 rust grain, the plates the A04 one.
    let steel = |family, rust_normal, grain, wear, passes| Material {
        family,
        passes,
        colour: vec![
            "mtlstlbas01d",
            "mtlimpure01d",
            "mtlrusta03d",
            "mtlscufa01",
            grain,
        ],
        specular: vec![
            "mtlstlbas01d",
            "mtlimpure01d",
            "mtlscufa01",
            grain,
            "scuffsgnra01",
        ],
        normal: vec![
            "mtlimpure02n",
            rust_normal,
            "mtlscufa01",
            grain,
            "mtlscufa01n",
        ],
        wear,
    };
    m[12] = steel(
        Family::Steel,
        "mtlrusta03n",
        "mtlrusta01g",
        0.75,
        Passes {
            alternate_normal: true,
            ..default_passes()
        },
    );
    m[13] = steel(
        Family::SteelPlate,
        "mtlrusta04n",
        "mtlrusta04g",
        1.0,
        Passes::default(),
    );
    m[14] = Material {
        family: Family::SteelPainted,
        passes: Passes::default(),
        colour: vec![
            "mtlstlbas01d",
            "mtlimpure01d",
            "mtlrusta03d",
            "mtlscufa01",
            "mtlrusta01g",
            "mtldirta01d",
        ],
        specular: vec![
            "mtlstlbas01d",
            "mtlimpure01d",
            "mtlscufa01",
            "mtlrusta01g",
            "scuffsgnra01",
        ],
        normal: vec![
            "mtlimpure02n",
            "mtlrusta03n",
            "mtlscufa01",
            "mtlrusta01g",
            "mtlscufa01n",
        ],
        wear: 1.0,
    };
    m[15] = steel(
        Family::SteelPlate,
        "mtlrusta04n",
        "mtlrusta04g",
        1.0,
        Passes::default(),
    );
    m[16] = Material {
        family: Family::SteelBlade,
        passes: Passes::default(),
        colour: vec![
            "mtlstlbld01d",
            "mtlrough01d",
            "mtlrusta02d",
            "mtlscufa01",
            "mtlrusta01g",
        ],
        specular: vec![
            "mtlstlbld01d",
            "mtlrough01d",
            "mtlscufa01",
            "mtlrusta01g",
            "scuffsgnra01",
        ],
        normal: vec![
            "mtlrough01n",
            "mtlrusta03n",
            "mtlscufa01",
            "mtlrusta01g",
            "mtlscufa01n",
        ],
        wear: 1.0,
    };
    let wood = |colour, spec, normal| Material {
        family: Family::Wood,
        passes: Passes::default(),
        colour: vec![colour, "mtlscufa01", "mtldirta01d"],
        specular: vec![spec, "mtlscufa01", "mtldirta01d"],
        normal: vec![normal, "mtldirta01d", "mtlscufa01n"],
        wear: 1.0,
    };
    m[17] = wood("woodfinea01d", "woodfinea01g", "woodfinea01n");
    m[18] = wood("wodhft01d", "wodhft01g", "wodhft01n");
    m[19] = steel(
        Family::Steel,
        "mtlrusta03n",
        "mtlrusta01g",
        1.0,
        Passes {
            alternate_normal: true,
            ..default_passes()
        },
    );
    let copper = |family, rust, spec_rust| Material {
        family,
        passes: Passes::default(),
        colour: vec![
            "mtlstlbas01d",
            "mtlimpure01d",
            rust,
            "mtlscufa01",
            "mtlrusta01g",
            "verdigris01d",
        ],
        specular: vec![
            spec_rust,
            "mtlimpure01d",
            "mtlscufa01",
            "scuffsgnra01",
            "verdigris01d",
        ],
        normal: vec![
            "mtlimpure02n",
            "mtlrusta03n",
            "mtlscufa01",
            "mtlrusta01g",
            "mtlscufa01n",
        ],
        wear: 1.0,
    };
    m[20] = copper(Family::CopperAlloy, "mtlrustc01d", "mtlrustc01d");
    m[21] = copper(Family::CopperSatin, "mtlrustc01d", "mtlrustc01d");
    // The wear table halves satin copper, the grip leather and the three
    // last materials, and takes three quarters for SteelGen.
    m[21].wear = 0.5;
    m[22] = Material {
        family: Family::Leather,
        passes: Passes::default(),
        colour: vec!["gripltra02d", "leatherdmga02d", "drtapr01"],
        specular: vec!["gripltra02g", "leatherdmga02d", "drtapr01"],
        normal: vec!["gripltra02n", "leatherdmga02d", "leatherdmga02n"],
        wear: 0.5,
    };
    let gem = |family, passes| Material {
        family,
        passes,
        colour: vec![
            "mtlstlbas01d",
            "mtlimpure01d",
            "mtlrustc01d",
            "mtlscufa01",
            "mtlrusta01g",
            "dirtbrowna01",
        ],
        specular: vec![
            "mtlrustc01d",
            "mtlimpure01d",
            "mtlscufa01",
            "scuffsgnra01",
            "dirtbrowna01",
        ],
        normal: vec![
            "mtlimpure02n",
            "mtlrusta03n",
            "mtlscufa01",
            "mtlrusta01g",
            "mtlscufa01n",
        ],
        wear: 0.5,
    };
    m[23] = gem(
        Family::Gemstone,
        Passes {
            alpha: true,
            pattern: true,
            ..default_passes()
        },
    );
    m[24] = gem(
        Family::Resin,
        Passes {
            alpha: true,
            ..default_passes()
        },
    );
    m[25] = Material {
        family: Family::PlanksPainted,
        passes: Passes {
            painted_pattern: true,
            ..default_passes()
        },
        colour: vec!["wodplnkp01d", "mtlscufa01", "mtldirta01d", "paintmska01"],
        specular: vec!["wodplnkp01g", "mtlscufa01", "mtldirta01d"],
        normal: vec!["wodplnkp01n", "mtldirta01d", "mtlscufa01n"],
        wear: 0.5,
    };
    m
});

const fn default_passes() -> Passes {
    Passes {
        alpha: false,
        pattern: false,
        painted_pattern: false,
        alternate_normal: false,
    }
}

/// Texture stems are stored lowercase, like the pack's entries.
fn stem_colour(stem: &'static str) -> &'static str {
    match stem {
        "leathera01" => "leathera01d",
        _ => "leathera02d",
    }
}

fn stem_spec(stem: &'static str) -> &'static str {
    match stem {
        "leathera01" => "leathera01g",
        _ => "leathera02g",
    }
}

fn stem_normal(stem: &'static str) -> &'static str {
    match stem {
        "leathera01" => "leathera01n",
        _ => "leathera02n",
    }
}

/// A GPU program variant of a material (`TProcMaterial`'s compile pass).
///
/// The original compiles up to seven variants for one material: the colour
/// program and its pattern and painted-pattern defines, the alpha program,
/// the specular program and the normal program with its alternate-normal
/// define. Each variant declares its material-layer samplers (`Tex0…`), the
/// uniforms the draw passes fetch by name, and the texture unit its first
/// material layer binds to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    /// Which pass this variant draws.
    pub pass: Pass,
    /// The program's name, `<family>_<letter>`.
    pub name: String,
    /// The define word the variant compiles with (0 for the plain program).
    pub define: u32,
    /// Material-layer samplers the program declares, `Tex0…`; sampler `i`
    /// is bound to texture unit `i` (see [`sampler_name`]).
    pub textures: usize,
    /// An extra sampler bound after the layer samplers: its name and unit.
    pub extra_sampler: Option<(String, u32)>,
    /// The texture unit the first material layer binds to: 2 for the
    /// colour, pattern, painted-pattern, alpha and specular draws, 3 for
    /// the normal ones. The draws bind every layer, one unit each.
    pub units_from: u32,
    /// Uniform names in the order the compile fetches them.
    pub uniforms: &'static [&'static str],
}

/// The passes a procedural material can draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pass {
    Colour,
    Pattern,
    PaintedPattern,
    Alpha,
    Specular,
    Normal,
    AlternateNormal,
}

/// The uniforms every pass fetches, in the compile's order.
pub const PASS_UNIFORMS: &[&str] = &[
    "MatTile", "DmgTile", "MatCol", "Damage", "Dirt", "Quality", "ShdTile", "UVScale",
];

/// The specular pass's uniforms: the material-scale tile of the painted
/// normal map sits between the shade tile and the texture scale.
pub const SPECULAR_UNIFORMS: &[&str] = &[
    "MatTile", "DmgTile", "MatCol", "Damage", "Dirt", "Quality", "ShdTile", "NrmTile", "UVScale",
];

/// The painted-pattern pass's uniforms: `PtrnPos` follows the texture scale.
pub const PAINTED_PATTERN_UNIFORMS: &[&str] = &[
    "MatTile", "DmgTile", "MatCol", "Damage", "Dirt", "Quality", "ShdTile", "UVScale", "PtrnPos",
];

/// Shader define words.
pub const DEFINE_PATTERN: u32 = 0x2_0000;
pub const DEFINE_ALTERNATE_NORMAL: u32 = 0x20_0000;
pub const DEFINE_PAINTED_PATTERN: u32 = 0x40_0000;

/// The family name a material's programs compile from, and the optional
/// alternate specular and normal families. The subclasses' setups name
/// them.
fn families(id: u8, m: &Material) -> (&'static str, Option<&'static str>, Option<&'static str>) {
    match m.family {
        Family::Cloth => {
            // ClothSilk's specular family is silk; the padded cloths take
            // the padded normal family.
            let alt = (id == 3).then_some("silk");
            let nrm = matches!(id, 5 | 6).then_some("clothp");
            ("cloth", alt, nrm)
        }
        Family::Velvet => ("velvet", None, None),
        Family::Leather | Family::Suede => ("leather", None, None),
        Family::Wood => ("wood", None, None),
        Family::Steel => ("steel", None, None),
        Family::SteelBlade => ("steelbld", None, None),
        Family::SteelPlate => ("steelplt", None, None),
        Family::SteelPainted => ("steelpnt", None, None),
        Family::Chain => ("steelchn", None, None),
        Family::CopperAlloy => ("copperal", None, None),
        Family::CopperSatin => ("coppersa", None, Some("copperal")),
        Family::Gemstone => ("gemstone", None, None),
        Family::Resin => ("resin", None, None),
        Family::PlanksPainted => ("plankpai", None, None),
    }
}

/// The name of layer sampler `index`: `Tex` and one character, `'0'` plus
/// the index, as the original writes it (index 10 would be `Tex:`; the
/// shipped materials stay below it).
pub fn sampler_name(index: usize) -> String {
    let c = char::from(b'0'.wrapping_add(index as u8));
    format!("Tex{c}")
}

/// Every program variant a material compiles, in the compile pass's order:
/// colour, pattern, painted pattern, alpha, specular, normal and alternate
/// normal. A variant whose pass flag is clear is not compiled.
///
/// The program names follow the setup's families: `<family>_d` for the
/// colour family's three variants, `<family>_a` for the alpha pass,
/// `<alt>_s` (the alternate family when the material sets one, else the
/// colour family) for the specular pass, and `<nrm>_n` for the normal pass.
pub fn programs(id: u8) -> Vec<Program> {
    let m = material(id);
    let (family, alt, nrm) = families(id, m);
    let spec_family = alt.unwrap_or(family);
    let nrm_family = nrm.unwrap_or(family);
    let mut out = Vec::new();
    let colour = m.colour.len();
    let spec = m.specular.len();
    let normal = m.normal.len();
    out.push(Program {
        pass: Pass::Colour,
        name: format!("{family}_d"),
        define: 0,
        textures: colour + 2,
        extra_sampler: None,
        units_from: 2,
        uniforms: PASS_UNIFORMS,
    });
    if m.passes.pattern {
        out.push(Program {
            pass: Pass::Pattern,
            name: format!("{family}_d"),
            define: DEFINE_PATTERN,
            textures: colour + 3,
            extra_sampler: None,
            units_from: 2,
            uniforms: PASS_UNIFORMS,
        });
    }
    if m.passes.painted_pattern {
        out.push(Program {
            pass: Pass::PaintedPattern,
            name: format!("{family}_d"),
            define: DEFINE_PAINTED_PATTERN,
            textures: colour + 2,
            extra_sampler: Some((sampler_name(colour + 2), 8)),
            units_from: 2,
            uniforms: PAINTED_PATTERN_UNIFORMS,
        });
    }
    if m.passes.alpha {
        out.push(Program {
            pass: Pass::Alpha,
            name: format!("{family}_a"),
            define: 0,
            textures: colour + 2,
            extra_sampler: None,
            units_from: 2,
            uniforms: PASS_UNIFORMS,
        });
    }
    out.push(Program {
        pass: Pass::Specular,
        name: format!("{spec_family}_s"),
        define: 0,
        textures: spec + 2,
        // The specular program reads the painted normal target through a
        // sampler of its own, at unit 7.
        extra_sampler: Some(("Nrm".to_string(), 7)),
        units_from: 2,
        uniforms: SPECULAR_UNIFORMS,
    });
    out.push(Program {
        pass: Pass::Normal,
        name: format!("{nrm_family}_n"),
        define: 0,
        textures: normal + 3,
        extra_sampler: None,
        units_from: 3,
        uniforms: PASS_UNIFORMS,
    });
    if m.passes.alternate_normal {
        out.push(Program {
            pass: Pass::AlternateNormal,
            name: format!("{nrm_family}_n"),
            define: DEFINE_ALTERNATE_NORMAL,
            textures: normal + 3,
            extra_sampler: Some((sampler_name(normal + 3), 8)),
            units_from: 3,
            uniforms: PASS_UNIFORMS,
        });
    }
    out
}

/// The material with `id`, or ClothBasic for an unknown id (the original
/// reports "Invalid Procedural Material ID." and falls back to it).
pub fn material(id: u8) -> &'static Material {
    if id == 0 || id > 25 {
        &MATERIALS[1]
    } else {
        &MATERIALS[id as usize]
    }
}

/// A weighted material class's weight: common, uncommon, rare, very rare.
pub const CLASS_WEIGHTS: [u32; 4] = [100, 50, 20, 10];

/// Finish levels by two-bit index; an area's finish byte is two of these
/// nibbles.
pub const FINISH_LEVELS: [u8; 4] = [3, 7, 11, 15];

/// The finish byte an object record without stored materials gets for a
/// material id: the same two 8-level nibbles for every known material.
pub fn default_finish(material: u8) -> u8 {
    if (1..=25).contains(&material) {
        0x88
    } else {
        0
    }
}

/// Colour class per material id (the table the colour roll switches on).
/// Classes not listed have no rule and give colour 0.
pub const COLOUR_CLASS: [u8; 26] = [
    0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 4, 4, 4, 5, 6, 4, 7, 7, 8, 9, 9, 1, 16, 17, 18,
];

/// Emblem ids the factory's pattern areas choose from.
pub const EMBLEM_IDS: [u8; 40] = [
    1, 2, 4, 5, 6, 7, 9, 10, 11, 12, 8, 13, 14, 17, 19, 20, 22, 23, 24, 26, 27, 28, 29, 30, 31, 32,
    33, 34, 35, 36, 39, 43, 44, 45, 46, 47, 48, 49, 50, 51,
];

/// Stem of the emblem background pictures (`Emblems.rpk`); a factory
/// pattern layer's picture is one of them.
pub const EMBLEM_BACKGROUND_STEM: &str = "emblembkg";
/// Stem of the emblem design pictures.
pub const EMBLEM_DESIGN_STEM: &str = "emblemdes";

/// An emblem picture's name: the stem, a `0`, then the index's tens and
/// ones as digit characters (`emblembkg012` for 12). Indices of 100 or more
/// give characters past `9`, as the original's byte arithmetic does.
pub fn emblem_picture(stem: &str, index: u32) -> String {
    let tens = b'0'.wrapping_add((index / 10) as u8);
    let ones = b'0'.wrapping_add((index % 10) as u8);
    format!("{stem}0{}{}", tens as char, ones as char)
}

/// The materials a random-set layer can take. Sets 1 and 5 are the same
/// list, as are several cloth sets.
pub const RANDOM_SETS: [&[u8]; 16] = [
    &[],
    &[1, 2, 3, 4, 7, 8, 9],
    &[5, 6, 10],
    &[1, 2, 3, 4],
    &[7, 8, 9],
    &[1, 2, 3, 4, 7, 8, 9],
    &[1, 2, 3, 4],
    &[13, 20],
    &[1, 2, 3, 4, 17, 13, 20],
    &[5, 6],
    &[7],
    &[11],
    &[13, 14],
    &[13, 14, 15],
    &[20],
    &[23],
];

/// The materials of random set `set`; empty for an unknown set.
pub fn random_set(set: u8) -> &'static [u8] {
    RANDOM_SETS.get(set as usize).copied().unwrap_or(&[])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_material_names_its_family_textures() {
        for id in 1..=25u8 {
            let m = material(id);
            assert!(!m.colour.is_empty(), "material {id}");
            assert!(!m.specular.is_empty(), "material {id}");
            assert!(!m.normal.is_empty(), "material {id}");
        }
        assert_eq!(material(1).family, Family::Cloth);
        assert_eq!(material(4).family, Family::Velvet);
        assert_eq!(material(11).family, Family::Chain);
        assert_eq!(material(12).family, Family::Steel);
        assert_eq!(material(13).family, Family::SteelPlate);
        assert_eq!(material(14).family, Family::SteelPainted);
        assert_eq!(material(16).family, Family::SteelBlade);
        assert_eq!(material(20).family, Family::CopperAlloy);
        assert_eq!(material(21).family, Family::CopperSatin);
        assert_eq!(material(23).family, Family::Gemstone);
        assert_eq!(material(24).family, Family::Resin);
        assert_eq!(material(25).family, Family::PlanksPainted);
    }

    #[test]
    fn plates_take_the_a04_rust_grain() {
        for id in [13u8, 15] {
            let m = material(id);
            assert_eq!(m.colour[4], "mtlrusta04g");
            assert_eq!(m.specular[3], "mtlrusta04g");
            assert_eq!(m.normal[1], "mtlrusta04n");
            assert_eq!(m.normal[3], "mtlrusta04g");
        }
        assert_eq!(material(12).colour[4], "mtlrusta01g");
    }

    #[test]
    fn wear_weights_follow_the_original_table() {
        let wear: Vec<f32> = (1..=25u8).map(|id| material(id).wear).collect();
        let mut expected = vec![1.0f32; 25];
        expected[3] = 0.8;
        expected[11] = 0.75;
        for w in &mut expected[20..] {
            *w = 0.5;
        }
        assert_eq!(wear, expected);
    }

    #[test]
    fn invalid_ids_fall_back_to_cloth_basic() {
        assert_eq!(material(0).family, Family::Cloth);
        assert_eq!(material(0).colour, material(1).colour);
        assert_eq!(material(200).colour, material(1).colour);
    }

    #[test]
    fn passes_match_the_material_flags() {
        assert!(material(1).passes.painted_pattern);
        assert!(material(8).passes.painted_pattern);
        assert!(material(12).passes.alternate_normal);
        assert!(material(19).passes.alternate_normal);
        assert!(!material(13).passes.alternate_normal);
        assert!(material(23).passes.alpha && material(23).passes.pattern);
        assert!(material(24).passes.alpha && !material(24).passes.pattern);
        assert!(material(25).passes.painted_pattern);
    }

    #[test]
    fn random_sets_are_the_originals_tables() {
        assert_eq!(random_set(1), &[1, 2, 3, 4, 7, 8, 9]);
        assert_eq!(random_set(2), &[5, 6, 10]);
        assert_eq!(random_set(7), &[13, 20]);
        assert_eq!(random_set(8), &[1, 2, 3, 4, 17, 13, 20]);
        assert_eq!(random_set(13), &[13, 14, 15]);
        assert_eq!(random_set(15), &[23]);
        assert!(random_set(0).is_empty());
        assert!(random_set(16).is_empty());
    }

    #[test]
    fn emblem_pictures_are_named_by_two_digits() {
        assert_eq!(emblem_picture(EMBLEM_BACKGROUND_STEM, 12), "emblembkg012");
        assert_eq!(emblem_picture(EMBLEM_BACKGROUND_STEM, 1), "emblembkg001");
        assert_eq!(emblem_picture(EMBLEM_DESIGN_STEM, 81), "emblemdes081");
    }

    #[test]
    fn default_finishes_and_emblem_ids() {
        assert_eq!(default_finish(0), 0);
        assert_eq!(default_finish(1), 0x88);
        assert_eq!(default_finish(25), 0x88);
        assert_eq!(default_finish(26), 0);
        assert_eq!(EMBLEM_IDS.len(), 40);
        assert_eq!(EMBLEM_IDS[0], 1);
        assert_eq!(EMBLEM_IDS[39], 51);
    }

    #[test]
    fn a_steel_material_compiles_its_four_programs() {
        let p = programs(12);
        assert_eq!(
            p.iter().map(|p| p.pass).collect::<Vec<_>>(),
            vec![
                Pass::Colour,
                Pass::Specular,
                Pass::Normal,
                Pass::AlternateNormal
            ]
        );
        assert_eq!(p[0].name, "steel_d");
        assert_eq!(p[0].textures, 7);
        assert_eq!(p[0].units_from, 2);
        assert_eq!(p[0].uniforms, PASS_UNIFORMS);
        assert_eq!(p[1].name, "steel_s");
        assert_eq!(p[1].extra_sampler, Some(("Nrm".to_string(), 7)));
        assert_eq!(p[1].uniforms, SPECULAR_UNIFORMS);
        assert_eq!(p[2].name, "steel_n");
        assert_eq!(p[2].textures, 8);
        assert_eq!(p[2].units_from, 3);
        assert_eq!(p[3].define, DEFINE_ALTERNATE_NORMAL);
        assert_eq!(p[3].extra_sampler, Some(("Tex8".to_string(), 8)));
    }

    #[test]
    fn plates_and_low_metals_skip_the_alternate_normal_variant() {
        assert!(programs(13).iter().all(|p| p.pass != Pass::AlternateNormal));
        assert!(programs(16).iter().all(|p| p.pass != Pass::AlternateNormal));
        assert!(programs(19).iter().any(|p| p.pass == Pass::AlternateNormal));
    }

    #[test]
    fn cloth_basic_compiles_the_painted_pattern_program() {
        let p = programs(1);
        let painted = p
            .iter()
            .find(|p| p.pass == Pass::PaintedPattern)
            .expect("painted pattern");
        assert_eq!(painted.name, "cloth_d");
        assert_eq!(painted.define, DEFINE_PAINTED_PATTERN);
        // Four colour layers, then Tex0..Tex5 for the layers and the
        // pattern at Tex6, unit 8.
        assert_eq!(painted.textures, 6);
        assert_eq!(painted.extra_sampler, Some(("Tex6".to_string(), 8)));
        assert_eq!(painted.uniforms, PAINTED_PATTERN_UNIFORMS);
        // Its plain specular and normal programs are the cloth ones.
        assert_eq!(programs(1)[0].name, "cloth_d");
    }

    #[test]
    fn silk_padded_and_copper_satin_compile_their_own_families() {
        let silk = programs(3);
        assert_eq!(
            silk.iter().find(|p| p.pass == Pass::Specular).unwrap().name,
            "silk_s"
        );
        let padded = programs(5);
        assert_eq!(
            padded.iter().find(|p| p.pass == Pass::Normal).unwrap().name,
            "clothp_n"
        );
        let satin = programs(21);
        assert_eq!(
            satin
                .iter()
                .find(|p| p.pass == Pass::Specular)
                .unwrap()
                .name,
            "coppersa_s"
        );
        assert_eq!(
            satin.iter().find(|p| p.pass == Pass::Normal).unwrap().name,
            "copperal_n"
        );
    }

    #[test]
    fn gemstone_and_resin_compile_the_alpha_programs() {
        let gem = programs(23);
        let passes: Vec<Pass> = gem.iter().map(|p| p.pass).collect();
        assert_eq!(
            passes,
            vec![
                Pass::Colour,
                Pass::Pattern,
                Pass::Alpha,
                Pass::Specular,
                Pass::Normal
            ]
        );
        assert_eq!(gem[1].define, DEFINE_PATTERN);
        assert_eq!(gem[2].name, "gemstone_a");
        let resin = programs(24);
        assert!(resin.iter().any(|p| p.pass == Pass::Alpha));
        assert!(!resin.iter().any(|p| p.pass == Pass::Pattern));
    }

    /// The three steel subclass constructors write their eight-byte code
    /// name and the colour, specular and normal layer counts before
    /// compiling: chain mail `steelchn` 5/5/5, painted steel `steelpnt`
    /// 6/5/5 and the plate `steelplt` 5/5/5.
    #[test]
    fn the_steel_family_constructors_words() {
        let family = |id: u8| {
            let m = material(id);
            (
                families(id, m).0,
                m.colour.len(),
                m.specular.len(),
                m.normal.len(),
            )
        };
        assert_eq!(family(11), ("steelchn", 5, 5, 5));
        assert_eq!(family(14), ("steelpnt", 6, 5, 5));
        assert_eq!(family(13), ("steelplt", 5, 5, 5));
        assert_eq!(family(15), ("steelplt", 5, 5, 5));
        // The chain's exact lists, in the constructor's order.
        let chain = material(11);
        assert_eq!(
            chain.colour,
            vec![
                "mailstla01d",
                "mtldirta01d",
                "mtlrusta02d",
                "mtlscufa01",
                "mtlrusta01g"
            ]
        );
        assert_eq!(
            chain.specular,
            vec![
                "mailstla01d",
                "mailstla01o",
                "mtlscufa01",
                "mtlrusta01g",
                "scuffsgnra01"
            ]
        );
        assert_eq!(
            chain.normal,
            vec![
                "mailstla01n",
                "mtlirnrst01n",
                "mtlscufa01",
                "mtlrusta01g",
                "mtlscufa01n"
            ]
        );
        // Painted steel adds the dirt layer to its colour pass.
        let painted = material(14);
        assert_eq!(
            painted.colour,
            vec![
                "mtlstlbas01d",
                "mtlimpure01d",
                "mtlrusta03d",
                "mtlscufa01",
                "mtlrusta01g",
                "mtldirta01d"
            ]
        );
        assert_eq!(
            painted.specular,
            vec![
                "mtlstlbas01d",
                "mtlimpure01d",
                "mtlscufa01",
                "mtlrusta01g",
                "scuffsgnra01"
            ]
        );
        assert_eq!(
            painted.normal,
            vec![
                "mtlimpure02n",
                "mtlrusta03n",
                "mtlscufa01",
                "mtlrusta01g",
                "mtlscufa01n"
            ]
        );
        // The plate takes the A04 rust grain in all three passes.
        let plate = material(15);
        assert_eq!(
            plate.colour,
            vec![
                "mtlstlbas01d",
                "mtlimpure01d",
                "mtlrusta03d",
                "mtlscufa01",
                "mtlrusta04g"
            ]
        );
        assert_eq!(
            plate.specular,
            vec![
                "mtlstlbas01d",
                "mtlimpure01d",
                "mtlscufa01",
                "mtlrusta04g",
                "scuffsgnra01"
            ]
        );
        assert_eq!(
            plate.normal,
            vec![
                "mtlimpure02n",
                "mtlrusta04n",
                "mtlscufa01",
                "mtlrusta04g",
                "mtlscufa01n"
            ]
        );
        assert_eq!(material(13).colour, plate.colour);
    }

    #[test]
    fn sampler_names_are_one_character_past_tex() {
        assert_eq!(sampler_name(0), "Tex0");
        assert_eq!(sampler_name(8), "Tex8");
        assert_eq!(sampler_name(10), "Tex:");
    }

    #[test]
    fn an_unknown_material_compiles_cloth_basic() {
        assert_eq!(programs(0), programs(1));
        assert_eq!(programs(200), programs(1));
    }
}
