//! Still-image capture: the original renders a view several times with
//! sub-pixel jitter, accumulates each result, runs the post chain and
//! downsamples to half size with a contrast-adaptive sharpen (the icon
//! variant also widens alpha from the neighbours), then reads the pixels
//! back into an RGBA image.
//!
//! The two downsample shader sources and the accumulation weights are the
//! original's. The jitter table's first four entries are the bytes the pack
//! dumps; the last four are a labelled guess. The GL programs, framebuffer
//! copies and readback are engine work; this module keeps the plan
//! testable. Rules: `docs/RENDERING.md` (Icons and thumbnails).

/// The number of jittered samples one capture takes.
pub const SAMPLE_COUNT: usize = 8;

/// The sub-pixel jitter of each sample, in pixels. The first four pairs are
/// the original's table; the last four are a guess (the dump stops at the
/// referenced bytes).
pub const JITTER: [[f32; 2]; SAMPLE_COUNT] = [
    [0.164_215_99, -0.054_399],
    [-0.183_9, 0.082_1],
    [0.102_25, 0.299_1],
    [-0.079_3, -0.317_4],
    [-0.164_215_99, 0.054_399],
    [0.183_9, -0.082_1],
    [-0.102_25, -0.299_1],
    [0.079_3, 0.317_4],
];

/// The accumulation weight of sample `index`: the frame is blended towards
/// the new sample by `1 − 1/(index + 1)`, so sample 0 replaces the frame
/// (weight 0), sample 1 halves, sample 2 two-thirds, and so on.
pub fn accumulate_weight(index: usize) -> f32 {
    1.0 - 1.0 / (index as f32 + 1.0)
}

/// The projection offset a sample adds: twice its jitter (the original's
/// `scale · 2 · jitter / size` with the view's scale equal to its size).
pub fn projection_offset(index: usize) -> Option<[f32; 2]> {
    JITTER.get(index).map(|[x, y]| [x * 2.0, y * 2.0])
}

/// The captured image's size: half the view's, both sides shifted right.
pub fn capture_size(width: u32, height: u32) -> (u32, u32) {
    (width >> 1, height >> 1)
}

/// The capture's pixel format word (`RGBA8`) and target kind (`2D`).
pub const CAPTURE_FORMAT: u32 = 0x0100_E608;
pub const CAPTURE_TARGET: u32 = 0x2000_0000;

/// The downsample shader that only sharpens.
pub const DOWNSAMPLE_SHADER: &str = "in vec2 vrTex;\r\nout vec4 FragColor;\r\nuniform sampler2D Tex;\r\nvoid main() {\r\nvec4 tcol = texture(Tex, vrTex);\r\nvec2 dx = dFdx(vrTex.xy) * 0.5;vec2 dy = dFdy(vrTex.xy) * 0.5;vec3 sc = texture(Tex, vrTex - dx - dy).rgb;vec3 sb = tcol.rgb - sc;float a = dot(sb,sb);sb = texture(Tex, vrTex + dx - dy).rgb;sc += sb;sb = tcol.rgb - sb;a = a + dot(sb, sb);sb = texture(Tex, vrTex + dx + dy).rgb;sc += sb;sb = tcol.rgb - sb;a = a + dot(sb, sb);sb = texture(Tex, vrTex - dx + dy).rgb;sc += sb;sb = tcol.rgb - sb;a = a + dot(sb, sb);a = a * 48;a = max(1.0 - a * a, 0.0) * 0.5;tcol.rgb = tcol.rgb * (1.0+a) - sc * 0.25 * a;FragColor = tcol;}";

/// The downsample shader that also widens alpha from four neighbours
/// (three texels away on each axis), used for icons with transparency.
pub const DOWNSAMPLE_ALPHA_SHADER: &str = "in vec2 vrTex;\r\nout vec4 FragColor;\r\nuniform sampler2D Tex;\r\nvoid main() {\r\nvec4 tcol = texture(Tex, vrTex);\r\nvec2 dx = dFdx(vrTex) * 0.5;vec2 dy = dFdy(vrTex) * 0.5;vec3 sc = texture(Tex, vrTex - dx - dy).rgb;vec3 sb = tcol.rgb - sc;float a = dot(sb,sb);sb = texture(Tex, vrTex + dx - dy).rgb;sc += sb;sb = tcol.rgb - sb;a = a + dot(sb, sb);sb = texture(Tex, vrTex + dx + dy).rgb;sc += sb;sb = tcol.rgb - sb;a = a + dot(sb, sb);sb = texture(Tex, vrTex - dx + dy).rgb;sc += sb;sb = tcol.rgb - sb;a = a + dot(sb, sb);a = a * 48;a = max(1.0 - a * a, 0.0);a = (a - 1.0 + tcol.a) * 0.5;tcol.rgb = tcol.rgb * (1.0+a) - sc * 0.25 * a;a = textureOffset(Tex, vrTex, ivec2(-3, 0)).a;\r\na = a + textureOffset(Tex, vrTex, ivec2(3, 0)).a;\r\na = a + textureOffset(Tex, vrTex, ivec2(0,-3)).a;\r\na = a + textureOffset(Tex, vrTex, ivec2(0, 3)).a;\r\ntcol.a = (tcol.a + a) * 0.5;\r\nFragColor = tcol;\r\n}";

/// One step of the capture: the sample it draws and the projection offset
/// it adds before drawing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaptureStep {
    pub sample: usize,
    pub weight: f32,
    pub projection: [f32; 2],
}

/// The capture's sample walk. Sample 0 draws without a blend; the later
/// samples blend at their accumulation weight.
pub fn capture_steps() -> Vec<CaptureStep> {
    (0..SAMPLE_COUNT)
        .map(|sample| CaptureStep {
            sample,
            weight: accumulate_weight(sample),
            projection: projection_offset(sample).unwrap_or([0.0, 0.0]),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accumulation_weights_are_one_minus_the_inverse() {
        assert_eq!(accumulate_weight(0), 0.0);
        assert_eq!(accumulate_weight(1), 0.5);
        assert!((accumulate_weight(2) - 2.0 / 3.0).abs() < 1e-7);
        assert_eq!(accumulate_weight(3), 0.75);
        assert!((accumulate_weight(7) - 0.875).abs() < 1e-7);
    }

    #[test]
    fn the_capture_walks_eight_jittered_samples() {
        let steps = capture_steps();
        assert_eq!(steps.len(), 8);
        assert_eq!(steps[0].projection, [0.328_431_98, -0.108_798]);
        assert_eq!(steps[0].weight, 0.0);
        assert_eq!(steps[1].weight, 0.5);
        assert_eq!(capture_size(1024, 768), (512, 384));
        assert_eq!(capture_size(1, 1), (0, 0));
        assert_eq!(CAPTURE_FORMAT, 0x0100_E608);
    }

    #[test]
    fn the_two_downsample_shaders_differ_in_alpha() {
        assert!(DOWNSAMPLE_ALPHA_SHADER.contains("textureOffset(Tex, vrTex, ivec2(-3, 0))"));
        assert!(!DOWNSAMPLE_SHADER.contains("textureOffset"));
        // Both sharpen with the original's 48× contrast and ¼ neighbour sum.
        for shader in [DOWNSAMPLE_SHADER, DOWNSAMPLE_ALPHA_SHADER] {
            assert!(shader.contains("a = a * 48;"));
            assert!(shader.contains("sc * 0.25 * a"));
        }
    }
}
