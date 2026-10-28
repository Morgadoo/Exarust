//! The procedural-texture programs of `textures.flb`: a named fragment
//! shader and the fixed projection matrix `MPrj` the engine gives it.
//!
//! The texture path compiles one library entry as a fragment shader (not
//! the surfaces' `shading_v` plus `<set>_f` pair), links it with one of the
//! engine's six built-in vertex shaders (picked by the caller's stream bits;
//! those are not library text and are not reproduced here), looks up `MPrj`
//! and uploads a constant matrix, then draws the generated texture
//! into a render target. The entry sees the same template language and the
//! surface builder's `{$Interface}` callback, so it expands like a surface
//! fragment program. `MPrj` maps a texture coordinate pair in [0, 1] to the
//! clip square: `x' = 2x − 1`, `y' = 2y − 1`, `z' = −0.001`, `w' = 1`.
//!
//! Exarust bakes those textures on the CPU instead (`exa_assets::paint`,
//! `decomp`, `skin_composite`), so this module is the original's program
//! side, kept for comparison and tests (`docs/RENDERING.md`, Shader
//! builder).

use crate::shader_template::{Includes, InterfaceMode, TemplateContext, TemplateError};

/// The `MPrj` constant, in the engine's column order: the four columns of
/// the matrix `glUniformMatrix4fv` uploads without transposing.
pub const MPRJ: [[f32; 4]; 4] = [
    [2.0, 0.0, 0.0, 0.0],
    [0.0, 2.0, 0.0, 0.0],
    [0.0, 0.0, -0.001, 0.0],
    [-1.0, -1.0, -0.001, 1.0],
];

/// A texture coordinate through `MPrj`: the clip-space point the program
/// draws its texel at (the input's `z` is zero, so the constant's `z`
/// column contributes only through its translation).
pub fn mprj_project(x: f32, y: f32) -> [f32; 4] {
    [
        MPRJ[0][0] * x + MPRJ[1][0] * y + MPRJ[3][0],
        MPRJ[0][1] * x + MPRJ[1][1] * y + MPRJ[3][1],
        MPRJ[0][2] * x + MPRJ[1][2] * y + MPRJ[3][2],
        1.0,
    ]
}

/// One procedural texture program: its expanded fragment source and the
/// projection uniform the caller uploads.
#[derive(Clone, Debug, PartialEq)]
pub struct TextureProgram {
    pub fragment: String,
    pub projection: [[f32; 4]; 4],
}

/// Expand the library entry `name` as a texture program.
///
/// The builder's stage is the fragment stage, so `{$Interface}` expands
/// the builder's fragment callback; the caller's flags are what it sees.
pub fn build(
    library: &impl Includes,
    name: &str,
    context: &TemplateContext,
) -> Result<TextureProgram, TemplateError> {
    let mut context = context.clone();
    context.interface = InterfaceMode::Fragment;
    let fragment = crate::shader_template::expand_with(library, name, &mut context)?;
    Ok(TextureProgram {
        fragment,
        projection: MPRJ,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn the_projection_maps_the_unit_square_to_the_clip_square() {
        assert_eq!(mprj_project(0.0, 0.0), [-1.0, -1.0, -0.001, 1.0]);
        assert_eq!(mprj_project(1.0, 1.0), [1.0, 1.0, -0.001, 1.0]);
        assert_eq!(mprj_project(0.5, 0.5), [0.0, 0.0, -0.001, 1.0]);
        // The constant, read from the engine's data.
        assert_eq!(MPRJ[0], [2.0, 0.0, 0.0, 0.0]);
        assert_eq!(MPRJ[1], [0.0, 2.0, 0.0, 0.0]);
        assert_eq!(MPRJ[2], [0.0, 0.0, -0.001, 0.0]);
        assert_eq!(MPRJ[3], [-1.0, -1.0, -0.001, 1.0]);
    }

    #[test]
    fn a_texture_entry_expands_with_the_fragment_interface() {
        let mut library = HashMap::new();
        library.insert(
            "proc_f".to_string(),
            "out vec4 C;\r\nvoid main(){C=vec4(1.0);}\r\n{$Interface}".to_string(),
        );
        let program = build(&library, "proc_f", &TemplateContext::default()).unwrap();
        assert!(program.fragment.starts_with("#version 150\r\n"));
        // The fragment interface's `Global` block comes from the callback.
        assert!(program.fragment.contains("layout (std140) uniform Global"));
        assert_eq!(program.projection, MPRJ);
        // A missing entry is an error, not a panic.
        assert!(matches!(
            build(&library, "missing_f", &TemplateContext::default()),
            Err(TemplateError::MissingEntry(_))
        ));
    }

    #[test]
    fn every_installed_texture_entry_expands() {
        let Some(library) = textures() else { return };
        assert!(!library.is_empty());
        assert!(library.get("chardecomp_d").is_some());
        for entry in library.entries() {
            let program = build(&library, &entry.name, &TemplateContext::default()).unwrap();
            assert!(
                !program.fragment.contains("{?")
                    && !program.fragment.contains("{%")
                    && !program.fragment.contains("{$"),
                "{}: unexpanded marker",
                entry.name
            );
            assert!(program.fragment.starts_with("#version 150\r\n"));
        }
    }

    /// The installed `textures.flb`, or `None` without the game.
    fn textures() -> Option<exa_formats::flb::ShaderLibrary> {
        let dir = exa_rpk::locate_install(None)?;
        let game = exa_rpk::GameData::open(&dir).ok()?;
        let resource = game.pack(exa_rpk::PackKind::Resource)?;
        exa_formats::flb::ShaderLibrary::parse(resource.get("textures.flb")?).ok()
    }
}
