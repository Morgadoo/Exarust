//! The procedural-texture programs of `textures.flb`: a named fragment
//! shader, one of the engine's six built-in vertex shaders and the fixed
//! projection matrix `MPrj` the engine gives them.
//!
//! The texture path compiles one library entry as a fragment shader (not
//! the surfaces' `shading_v` plus `<set>_f` pair), links it with a built-in
//! vertex shader picked by the draw state's stream word, binds the
//! attributes by name, looks up `MPrj` and uploads a constant matrix, then
//! draws the generated texture into a render target. The entry sees the same
//! template language and the surface builder's `{$Interface}` callback, so
//! it expands like a surface fragment program. `MPrj` maps a texture
//! coordinate pair in [0, 1] to the clip square: `x' = 2x − 1`,
//! `y' = 2y − 1`, `z' = −0.001`, `w' = 1`.
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

/// The projection uniform the engine looks up on a texture program.
pub const MPRJ_UNIFORM: &str = "MPrj";

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

/// One of the engine's six built-in vertex programs for texture generation.
///
/// The draw state's stream word picks one: bit `0x20` adds the second
/// texture coordinate (and the first with it), `0x4` the vertex colour and
/// `0x10` the texture coordinate; the six names cover the combinations the
/// state can hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureVertex {
    /// `vPos` only.
    Position,
    /// `vPos` and `vTex`.
    PositionTex,
    /// `vPos`, `vCol` and `vTex`.
    PositionColourTex,
    /// `vPos` and `vCol`.
    PositionColour,
    /// `vPos`, `vTex` and `vTx2`.
    PositionTexTex2,
    /// `vPos`, `vTex`, `vTx2` and `vCol`.
    PositionTexTex2Colour,
}

impl TextureVertex {
    /// The built-in vertex program the draw state's stream word selects:
    /// the second texture coordinate wins over the colour and the colour
    /// over the first texture coordinate.
    pub fn for_stream(word: u32) -> Self {
        let second = word & 0x20 != 0;
        let colour = word & 0x4 != 0;
        let tex = word & 0x10 != 0;
        match (second, colour, tex) {
            (true, true, _) => Self::PositionTexTex2Colour,
            (true, false, _) => Self::PositionTexTex2,
            (false, true, true) => Self::PositionColourTex,
            (false, true, false) => Self::PositionColour,
            (false, false, true) => Self::PositionTex,
            (false, false, false) => Self::Position,
        }
    }

    /// The program's GLSL source, as the engine's built-in shader text
    /// stores it (the entry alone is a fragment shader; the vertex program
    /// is compiled from this text with the `#version 150` line in front).
    pub fn source(self) -> &'static str {
        match self {
            Self::Position => {
                "#version 150\r\n\
in vec3 vPos;\r\n\
uniform mat4 MPrj;\r\n\
void main() {\r\n\
gl_Position = MPrj * vec4(vPos,1.0);\r\n\
}"
            }
            Self::PositionTex => {
                "#version 150\r\n\
in vec3 vPos;\r\n\
in vec2 vTex;\r\n\
out vec2 vrTex;\r\n\
uniform mat4 MPrj;\r\n\
void main() {\r\n\
gl_Position = MPrj * vec4(vPos,1.0);\r\n\
vrTex = vTex;\r\n\
}"
            }
            Self::PositionColourTex => {
                "#version 150\r\n\
in vec3 vPos;\r\n\
in vec4 vCol;\r\n\
in vec2 vTex;\r\n\
out vec4 vrCol;\r\n\
out vec2 vrTex;\r\n\
uniform mat4 MPrj;\r\n\
void main() {\r\n\
gl_Position = MPrj * vec4(vPos,1.0);\r\n\
vrTex = vTex;\r\n\
vrCol = vCol * 0.003921568;\r\n\
}"
            }
            Self::PositionColour => {
                "#version 150\r\n\
in vec3 vPos;\r\n\
in vec4 vCol;\r\n\
out vec4 vrCol;\r\n\
uniform mat4 MPrj;\r\n\
void main() {\r\n\
gl_Position = MPrj * vec4(vPos,1.0);\r\n\
vrCol = vCol * 0.003921568;\r\n\
}"
            }
            Self::PositionTexTex2 => {
                "#version 150\r\n\
in vec3 vPos;\r\n\
in vec2 vTex;\r\n\
in vec2 vTx2;\r\n\
out vec2 vrTex;\r\n\
out vec2 vrTx2;\r\n\
uniform mat4 MPrj;\r\n\
void main() {\r\n\
gl_Position = MPrj * vec4(vPos,1.0);\r\n\
vrTex = vTex;\r\n\
vrTx2 = vTx2;\r\n\
}"
            }
            Self::PositionTexTex2Colour => {
                "#version 150\r\n\
in vec3 vPos;\r\n\
in vec2 vTex;\r\n\
in vec2 vTx2;\r\n\
in vec4 vCol;\r\n\
out vec2 vrTex;\r\n\
out vec2 vrTx2;\r\n\
out vec4 vrCol;\r\n\
uniform mat4 MPrj;\r\n\
void main() {\r\n\
gl_Position = MPrj * vec4(vPos,1.0);\r\n\
vrTex = vTex;\r\n\
vrTx2 = vTx2;\r\n\
vrCol = vCol * 0.003921568;\r\n\
}"
            }
        }
    }
}

/// One attribute binding of a linked texture program: the GLSL name and
/// the location the linker gives it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttributeBinding {
    pub name: &'static str,
    pub location: u32,
}

/// The attribute locations the engine binds before linking, from the draw
/// state's stream word.
///
/// `vPos` is always location 0; `vTex` follows at 1 for bit `0x10`, `vTx2`
/// at 4 for `0x20`, `vNrm` at 2 for `0x2`, `vCol` at 3 for `0x4`, `vTng` at
/// 6 for `0x8` and `vDlt` at 7 for `0x40`.
pub fn attribute_bindings(state: u32) -> Vec<AttributeBinding> {
    let mut out = vec![AttributeBinding {
        name: "vPos",
        location: 0,
    }];
    if state & 0x10 != 0 {
        out.push(AttributeBinding {
            name: "vTex",
            location: 1,
        });
    }
    if state & 0x20 != 0 {
        out.push(AttributeBinding {
            name: "vTx2",
            location: 4,
        });
    }
    if state & 0x2 != 0 {
        out.push(AttributeBinding {
            name: "vNrm",
            location: 2,
        });
    }
    if state & 0x4 != 0 {
        out.push(AttributeBinding {
            name: "vCol",
            location: 3,
        });
    }
    if state & 0x8 != 0 {
        out.push(AttributeBinding {
            name: "vTng",
            location: 6,
        });
    }
    if state & 0x40 != 0 {
        out.push(AttributeBinding {
            name: "vDlt",
            location: 7,
        });
    }
    out
}

/// One procedural texture program: the six built-in vertex shaders and its
/// expanded fragment source, the attribute bindings of the draw state's
/// stream word, and the projection uniform the caller uploads.
#[derive(Clone, Debug, PartialEq)]
pub struct TextureProgram {
    pub vertex: TextureVertex,
    pub vertex_source: &'static str,
    pub fragment: String,
    pub attributes: Vec<AttributeBinding>,
    pub projection: [[f32; 4]; 4],
}

impl TextureProgram {
    /// Whether the two shaders link. The engine's link succeeds when both
    /// compiled sources are non-empty; the caller of this model has already
    /// expanded the fragment.
    pub fn links(&self) -> bool {
        !self.fragment.is_empty() && !self.vertex_source.is_empty()
    }
}

/// Expand the library entry `name` as a texture program and pair it with
/// the built-in vertex shader a stream word of zero picks.
///
/// The builder's stage is the fragment stage, so `{$Interface}` expands
/// the builder's fragment callback; the caller's flags are what it sees.
pub fn build(
    library: &impl Includes,
    name: &str,
    context: &TemplateContext,
) -> Result<TextureProgram, TemplateError> {
    build_with_stream(library, name, context, 0)
}

/// Expand the library entry `name` as a texture program for a draw state's
/// stream word: its built-in vertex shader and attribute bindings.
pub fn build_with_stream(
    library: &impl Includes,
    name: &str,
    context: &TemplateContext,
    stream_word: u32,
) -> Result<TextureProgram, TemplateError> {
    let mut context = context.clone();
    context.interface = InterfaceMode::Fragment;
    let fragment = crate::shader_template::expand_with(library, name, &mut context)?;
    let vertex = TextureVertex::for_stream(stream_word);
    Ok(TextureProgram {
        vertex,
        vertex_source: vertex.source(),
        fragment,
        attributes: attribute_bindings(stream_word),
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
        assert_eq!(MPRJ_UNIFORM, "MPrj");
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
        // The default stream word picks the position-only vertex program and
        // its one binding.
        assert_eq!(program.vertex, TextureVertex::Position);
        assert_eq!(program.attributes.len(), 1);
        assert_eq!(program.attributes[0].name, "vPos");
        assert!(program.links());
        // A missing entry is an error, not a panic.
        assert!(matches!(
            build(&library, "missing_f", &TemplateContext::default()),
            Err(TemplateError::MissingEntry(_))
        ));
    }

    #[test]
    fn the_stream_word_picks_the_builtin_vertex_program() {
        // No bits: position only.
        assert_eq!(TextureVertex::for_stream(0), TextureVertex::Position);
        // 0x10: the texture coordinate.
        assert_eq!(TextureVertex::for_stream(0x10), TextureVertex::PositionTex);
        // 0x4 alone: the colour.
        assert_eq!(
            TextureVertex::for_stream(0x4),
            TextureVertex::PositionColour
        );
        // 0x4 and 0x10: colour and texture.
        assert_eq!(
            TextureVertex::for_stream(0x14),
            TextureVertex::PositionColourTex
        );
        // 0x20 adds the second texture coordinate; the colour then takes the
        // all-four program.
        assert_eq!(
            TextureVertex::for_stream(0x10 | 0x20),
            TextureVertex::PositionTexTex2
        );
        assert_eq!(
            TextureVertex::for_stream(0x10 | 0x20 | 0x4),
            TextureVertex::PositionTexTex2Colour
        );
        // The second coordinate wins even without the first bit.
        assert_eq!(
            TextureVertex::for_stream(0x20),
            TextureVertex::PositionTexTex2
        );
        // The vertex programs carry the projection and the colour scale.
        assert!(
            TextureVertex::Position
                .source()
                .contains("MPrj * vec4(vPos,1.0)")
        );
        assert!(
            TextureVertex::PositionColour
                .source()
                .contains("vCol * 0.003921568")
        );
        let program = build_with_stream(
            &HashMap::<String, String>::new(),
            "x",
            &TemplateContext::default(),
            0x10 | 0x20 | 0x4,
        );
        // The fragment entry is missing, so the build errors rather than
        // inventing a program.
        assert!(program.is_err());
    }

    #[test]
    fn attribute_bindings_follow_the_stream_word() {
        let names = |state| {
            attribute_bindings(state)
                .into_iter()
                .map(|b| (b.name, b.location))
                .collect::<Vec<_>>()
        };
        assert_eq!(names(0), vec![("vPos", 0)]);
        assert_eq!(
            names(0x10 | 0x20 | 0x2 | 0x4 | 0x8 | 0x40),
            vec![
                ("vPos", 0),
                ("vTex", 1),
                ("vTx2", 4),
                ("vNrm", 2),
                ("vCol", 3),
                ("vTng", 6),
                ("vDlt", 7),
            ]
        );
        // The draw state's other bits bind nothing.
        assert_eq!(names(0x1 | 0x80), vec![("vPos", 0)]);
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
            assert!(program.links());
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
