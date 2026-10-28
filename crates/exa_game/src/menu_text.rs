//! The menu rows' text shader: the game-main unit's one embedded fragment
//! program and the colour rule it applies to a glyph texel.
//!
//! The menu form's text is drawn with a small GLSL program of the unit's own
//! (the only shader source the unit keeps, released with its other tables at
//! shutdown). It samples the font's texture, warms the glyph by a colour
//! mixed from its own red channel, tone-maps it and multiplies by the vertex
//! colour. `GCol` is declared but the body never reads it.
//!
//! [`page_rows`](crate::menu_pages::page_rows) in [`crate::menu_pages`] and
//! [`crate::menu_screen`] paint the rows; they still draw the base font
//! without this program, so [`texel`] is the rule the draw will use, tested
//! on its own.
//!
//! Evidence: the unit's shader string and its registration beside the menu
//! themes; the arithmetic is the program's own.

/// The menu text shader's source, exactly as the unit stores it.
pub const MENU_TEXT_SHADER: &str = "in vec2 vrTex;\r\nin vec4 vrCol;\r\nout vec4 FragColor;\r\nuniform vec4 GCol;\r\nuniform sampler2D Tex;\r\nvoid main() {\r\nvec4 tcol = texture(Tex, vrTex);\r\nvec3 c = mix(vec3(1.0, 0.75, 0.4), vec3(1.0), tcol.x);\r\ntcol.rgb = tcol.rgb * c * 1.1;\r\ntcol.rgb = tcol.rgb / (0.5 + tcol.rgb) * 1.5;\r\nFragColor.rgb = vrCol.rgb * tcol.rgb;FragColor.a = vrCol.a * tcol.a;}";

/// The glyph tint's darker end, mixed from the glyph's red channel: the
/// shader's `vec3(1.0, 0.75, 0.4)`.
pub const TINT_LOW: [f32; 3] = [1.0, 0.75, 0.4];
/// The tint's colour at a red channel of 1: white.
pub const TINT_HIGH: [f32; 3] = [1.0, 1.0, 1.0];
/// The warm gain before the tone map.
pub const TINT_GAIN: f32 = 1.1;
/// The tone map's half point and scale.
pub const TONE_OFFSET: f32 = 0.5;
pub const TONE_SCALE: f32 = 1.5;

/// The shader's per-channel colour for a glyph texel `tcol` and vertex colour
/// `vr_col`, in the program's order: mix the tint by `tcol.x`, multiply by
/// the gain, tone-map, then modulate rgb by the vertex colour and alpha by
/// the texel's.
pub fn texel(tcol: [f32; 4], vr_col: [f32; 4]) -> [f32; 4] {
    let a = tcol[0];
    let c = [
        TINT_LOW[0] * (1.0 - a) + TINT_HIGH[0] * a,
        TINT_LOW[1] * (1.0 - a) + TINT_HIGH[1] * a,
        TINT_LOW[2] * (1.0 - a) + TINT_HIGH[2] * a,
    ];
    // `tcol.rgb * c * 1.1` then `tcol.rgb / (0.5 + tcol.rgb) * 1.5`, each in
    // the program's left-to-right order.
    let rgb = {
        let lit = [
            tcol[0] * c[0] * TINT_GAIN,
            tcol[1] * c[1] * TINT_GAIN,
            tcol[2] * c[2] * TINT_GAIN,
        ];
        [
            lit[0] / (TONE_OFFSET + lit[0]) * TONE_SCALE,
            lit[1] / (TONE_OFFSET + lit[1]) * TONE_SCALE,
            lit[2] / (TONE_OFFSET + lit[2]) * TONE_SCALE,
        ]
    };
    [
        vr_col[0] * rgb[0],
        vr_col[1] * rgb[1],
        vr_col[2] * rgb[2],
        vr_col[3] * tcol[3],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) {
        assert!((a - b).abs() < 1e-6, "{a} != {b}");
    }

    #[test]
    fn the_menu_text_shader_is_the_units_own_source() {
        assert!(MENU_TEXT_SHADER.starts_with("in vec2 vrTex;\r\nin vec4 vrCol;"));
        assert!(MENU_TEXT_SHADER.ends_with("FragColor.a = vrCol.a * tcol.a;}"));
        // `GCol` is declared and never read.
        assert!(MENU_TEXT_SHADER.contains("uniform vec4 GCol;"));
        assert!(!MENU_TEXT_SHADER.contains("GCol."));
        assert_eq!(
            MENU_TEXT_SHADER.matches("mix(vec3(1.0, 0.75, 0.4)").count(),
            1
        );
    }

    #[test]
    fn a_glyph_texel_is_tinted_and_tone_mapped() {
        // A quarter red channel: the tint is (1, 0.8125, 0.55) between the
        // low and the white end, and every channel is warmed, tone-mapped.
        let out = texel([0.25, 0.5, 0.5, 1.0], [1.0, 1.0, 1.0, 1.0]);
        close(out[0], 0.532_258_06);
        close(out[1], 0.707_920_79);
        close(out[2], 0.565_420_56);
        close(out[3], 1.0);
        // A full red channel takes the white end of the tint.
        let white = texel([1.0, 0.0, 0.0, 0.5], [1.0, 1.0, 1.0, 1.0]);
        close(white[0], 1.031_25);
        close(white[1], 0.0);
        close(white[2], 0.0);
        close(white[3], 0.5);
        // The vertex colour multiplies the tone-mapped glyph.
        let half = texel([0.25, 0.5, 0.5, 1.0], [0.5, 0.5, 0.5, 0.5]);
        close(half[0], out[0] * 0.5);
        close(half[3], 0.5);
    }
}
