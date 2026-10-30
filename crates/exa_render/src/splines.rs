//! The thaumaturgy skill tree's spline lines, as the original draws them: a
//! tessellated 2D polyline turned into a quad strip, drawn with a small
//! built-in anti-aliased line program.
//!
//! The render object owns a pre-built index list and, in each vertex slot,
//! the strip coordinate that goes with it ([`quad_indices`],
//! [`strip_coords`]): quad `i` uses the vertex pair `2i, 2i + 1`, whose
//! texture coordinates are `(i, 1)` and `(i, 0)`. [`SplineStrip`] fills the
//! other attributes: every new point becomes two vertices offset either side
//! of the line by its width along the chord's 2D normal, with the join's
//! curvature term and the vertex colour. Closing the strip adds one last
//! pair (at the explicit end point, or the previous point extended through
//! the current one) and drops it from the draw, which submits `3 · count`
//! indices.
//!
//! The line program's fragment rule is reproduced by [`LINE_FRAGMENT_WGSL`].
//! Nothing draws splines yet: Exarust has no thaumaturgy tree, so there is
//! no caller, and the rule is documented rather than compiled into a
//! pipeline.
//!
//! The draw routine also computes `1 / (right − left)` and `1 / (bottom −
//! top)` of the viewport, but neither value reaches the program (only the
//! three width values are uploaded), so they are not modelled.

/// Quads the index list holds: the original's table loop runs one more time
/// than the 2 048 its constant suggests (it counts from 0 to 0x800
/// inclusive), so the list covers 2 049 quads and 4 098 vertices.
pub const MAX_QUADS: usize = 2049;
/// Vertices of the pre-built strip, two per quad.
pub const MAX_VERTICES: usize = 2 * MAX_QUADS;
/// Bytes per vertex (the original's attribute stride): position (2 floats),
/// texture coordinate (2), the curvature term and an unused float, the
/// four colour bytes and four bytes of padding.
pub const VERTEX_STRIDE: usize = 32;

/// The pre-built index list for [`MAX_QUADS`] quads: for quad `i` the
/// triangles `(2i, 2i+1, 2i+3)` and `(2i, 2i+3, 2i+2)`, as 16-bit indices.
pub fn quad_indices() -> Vec<u16> {
    let mut out = Vec::with_capacity(MAX_QUADS * 6);
    for i in 0..MAX_QUADS {
        let v = (2 * i) as u16;
        out.extend_from_slice(&[v, v + 1, v + 3, v, v + 3, v + 2]);
    }
    out
}

/// The texture coordinates of quad `i`'s two vertices: `(i, 1)` on the first
/// and `(i, 0)` on the second, `i` as a float.
pub fn strip_coords(i: usize) -> [[f32; 2]; 2] {
    let u = i as f32;
    [[u, 1.0], [u, 0.0]]
}

/// The line program's three width values from the line width: `x = width/2
/// + √(4·width)` and `z = √(6·width)` in single precision, `y = −(0.3·width
/// + 0.15)` computed in double precision and rounded once.
pub fn line_widths(width: f32) -> [f32; 3] {
    let x = width * 0.5 + (width * 4.0).sqrt();
    let y = -((f64::from(width) * 0.3 + 0.15) as f32);
    let z = (width * 6.0).sqrt();
    [x, y, z]
}

/// The line draw's uniform values: the three width values ([`line_widths`])
/// and the reciprocal viewport width and height in single precision.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SplineUniforms {
    pub widths: [f32; 3],
    pub inv_width: f32,
    pub inv_height: f32,
}

/// The uniforms of a strip drawn at `width` into the viewport rectangle
/// `(left, top, right, bottom)` in pixels: the reciprocal of its width and
/// height, each difference widened to 64 bits and narrowed before the
/// division, as the original does.
pub fn line_uniforms(width: f32, viewport: [i32; 4]) -> SplineUniforms {
    let inv_width = 1.0 / (i64::from(viewport[2]) - i64::from(viewport[0])) as f32;
    let inv_height = 1.0 / (i64::from(viewport[3]) - i64::from(viewport[1])) as f32;
    SplineUniforms {
        widths: line_widths(width),
        inv_width,
        inv_height,
    }
}

/// The fragment rule of the built-in line program, as a WGSL function. The
/// vertex stage passes the texture coordinate, the curvature term and the
/// colour (bytes scaled by 0.003921568, the original's constant, not an
/// exact 1/255); `widths` is [`line_widths`].
pub const LINE_FRAGMENT_WGSL: &str = r#"
// tex: x = quad index along the line (its fraction is the position across
//      a segment), y = 1 on the first vertex of a pair and 0 on the second.
// curvature: the join's curvature term.
// colour: the vertex colour, bytes scaled by 0.003921568.
// widths: x = width/2 + sqrt(4*width), y = -(0.3*width + 0.15),
//         z = sqrt(6*width).
fn line_colour(
    tex: vec2<f32>,
    curvature: f32,
    colour: vec4<f32>,
    widths: vec3<f32>,
) -> vec4<f32> {
    var a = abs(0.5 - fract(tex.x));
    a = sqrt(1.0 - a * 2.0);
    a = clamp(0.5 + a * curvature, 0.0, 1.0);
    a = 0.5 - abs(tex.y - a);
    let b = clamp(a * widths.x + widths.y, 0.0, 1.0);
    let rgb = colour.rgb * b;
    a = clamp(a * widths.z * 0.2, 0.0, 1.0);
    a = clamp(a + b, 0.0, 1.0);
    return vec4<f32>(rgb, colour.a * a * a);
}
"#;

/// One strip vertex: position, texture coordinate, the curvature term and
/// the four colour bytes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplineVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub curvature: f32,
    pub colour: [u8; 4],
}

impl SplineVertex {
    /// The vertex as the original's 32-byte record: position, texture
    /// coordinate, the curvature term, one unused float, the colour bytes
    /// and four bytes of padding.
    pub fn to_bytes(self) -> [u8; VERTEX_STRIDE] {
        let mut out = [0u8; VERTEX_STRIDE];
        let mut put = |at: usize, f: f32| {
            out[at..at + 4].copy_from_slice(&f.to_le_bytes());
        };
        put(0, self.position[0]);
        put(4, self.position[1]);
        put(8, self.uv[0]);
        put(12, self.uv[1]);
        put(16, self.curvature);
        out[24..28].copy_from_slice(&self.colour);
        out
    }
}

fn sub(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

fn add(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] + b[0], a[1] + b[1]]
}

fn dot(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}

/// `a + (b − a) · t` in single precision.
fn lerp(a: [f32; 2], b: [f32; 2], t: f32) -> [f32; 2] {
    let d = sub(b, a);
    [a[0] + d[0] * t, a[1] + d[1] * t]
}

/// `CVTSD2SI` to 64 bits: ties to even, and the "integer indefinite" value
/// for anything that does not fit (including NaN and infinity).
fn convert_to_i64(x: f64) -> i64 {
    const TWO_POW_63: f64 = 9_223_372_036_854_775_808.0;
    let r = x.round_ties_even();
    if (-TWO_POW_63..TWO_POW_63).contains(&r) {
        r as i64
    } else {
        i64::MIN
    }
}

/// One tessellated spline: the strip's vertices in draw order.
#[derive(Clone, Debug, Default)]
pub struct SplineStrip {
    vertices: Vec<SplineVertex>,
    /// The previous and current spline points.
    previous: [f32; 2],
    current: [f32; 2],
    /// The vertex count the draw is made from (two per point).
    count: i32,
    width: f32,
    /// The spline's length in segments; the fade divides each vertex's
    /// index by it.
    pub segments: i32,
    /// The colour of the vertices added next. A new strip's colour is all
    /// zero, like the rest of the freshly allocated object.
    pub colour: [u8; 4],
    /// When set, each pair's alpha byte is replaced by the colour's alpha ×
    /// (first vertex index ÷ `segments`) × ½, rounded to even and kept to
    /// its low byte, as the original writes it.
    pub fade: bool,
}

impl SplineStrip {
    /// An empty strip: zero width, zero colour, no fade.
    pub fn new() -> Self {
        Self::default()
    }

    /// The draw state: stores the line width, which the tessellation scales
    /// the normal to, and returns the three width values the program takes.
    pub fn draw_state(&mut self, width: f32) -> [f32; 3] {
        self.width = width;
        line_widths(width)
    }

    /// The line width.
    pub fn width(&self) -> f32 {
        self.width
    }

    /// Starts a spline at `start`: no vertices, the previous and current
    /// points both there, and the length in segments stored for the fade.
    pub fn begin(&mut self, start: [f32; 2], segments: i32) -> &mut Self {
        self.count = 0;
        self.vertices.clear();
        self.segments = segments;
        self.previous = start;
        self.current = start;
        self
    }

    /// The vertices written so far, two per added point.
    pub fn vertices(&self) -> &[SplineVertex] {
        &self.vertices
    }

    /// The vertex count the draw uses: two per point, less the final pair
    /// once the strip is closed.
    pub fn count(&self) -> i32 {
        self.count
    }

    /// Adds `end` and closes the strip. Without an explicit point it uses
    /// the previous point extended through the current one to twice the
    /// distance (`previous + (current − previous) · 2`), so the last real
    /// point gets the normal of a straight continuation. The extra pair is
    /// then dropped from the draw.
    pub fn end(&mut self, end: Option<[f32; 2]>) -> &mut Self {
        let point = end.unwrap_or_else(|| lerp(self.previous, self.current, 2.0));
        self.add_point(point, 0);
        self.count -= 2;
        self
    }

    /// The number of indices the draw submits: `3 · count`.
    pub fn index_count(&self) -> usize {
        (self.count.max(0) as usize) * 3
    }

    /// Adds one spline point. The pair lies on the current point, offset by
    /// the normal of the chord from the previous point to `point`, scaled to
    /// the strip's width. `mode` 2 zeroes the curvature term of the first
    /// vertex and mode 1 that of the second (a join that ends or starts a
    /// line); otherwise both get the join's curvature.
    pub fn add_point(&mut self, point: [f32; 2], mode: i32) -> &mut Self {
        let chord = sub(point, self.previous);
        // (−dy, dx), scaled to the width: width ÷ its length.
        let mut normal = [-chord[1], chord[0]];
        let scale = self.width / (normal[1] * normal[1] + normal[0] * normal[0]).sqrt();
        normal = [normal[0] * scale, normal[1] * scale];
        let into = sub(self.current, self.previous);
        let out = sub(point, self.current);
        let curvature = {
            let turn = dot(normal, into) - dot(normal, out);
            let numerator = f64::from(turn) * -0.09;
            let width_squared = self.width * self.width;
            (numerator / f64::from(width_squared)) as f32
        };

        let first = self.vertices.len();
        let uv = strip_coords(first / 2);
        let mut colour = self.colour;
        if self.fade {
            let share = (first as f64 / f64::from(self.segments)) * 0.5;
            let alpha = f64::from(self.colour[3]) * share;
            colour[3] = convert_to_i64(alpha) as u8;
        }
        self.vertices.push(SplineVertex {
            position: sub(self.current, normal),
            uv: uv[0],
            curvature: if mode == 2 { 0.0 } else { curvature },
            colour,
        });
        self.vertices.push(SplineVertex {
            position: add(self.current, normal),
            uv: uv[1],
            curvature: if mode == 1 { 0.0 } else { curvature },
            colour,
        });
        self.count += 2;
        self.previous = self.current;
        self.current = point;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_index_table_builds_two_triangles_per_quad() {
        let indices = quad_indices();
        assert_eq!(MAX_QUADS, 2049);
        assert_eq!(indices.len(), MAX_QUADS * 6);
        assert_eq!(&indices[..6], &[0, 1, 3, 0, 3, 2]);
        assert_eq!(&indices[6..12], &[2, 3, 5, 2, 5, 4]);
        // The last quad reaches vertex 4 099, two past the 4 098 vertex
        // slots: the original's own arithmetic (no spline gets that long).
        assert_eq!(
            &indices[indices.len() - 6..],
            &[4096, 4097, 4099, 4096, 4099, 4098]
        );
        assert_eq!(MAX_VERTICES, 4098);
        assert_eq!(strip_coords(0), [[0.0, 1.0], [0.0, 0.0]]);
        assert_eq!(strip_coords(7), [[7.0, 1.0], [7.0, 0.0]]);
    }

    #[test]
    fn the_line_widths_follow_the_original_arithmetic() {
        // width 4: 2 + √16 = 6; −(1.2 + 0.15); √24.
        let w = line_widths(4.0);
        assert_eq!(w[0], 6.0);
        assert!((w[1] + 1.35).abs() < 1e-6);
        assert!((w[2] - 24f32.sqrt()).abs() < 1e-6);
        let mut strip = SplineStrip::new();
        assert_eq!(strip.draw_state(4.0), w);
        assert_eq!(strip.width(), 4.0);
    }

    #[test]
    fn a_straight_spline_puts_its_vertices_either_side() {
        let mut s = SplineStrip::new();
        s.draw_state(2.0);
        s.begin([0.0, 0.0], 2);
        s.add_point([0.0, 10.0], 2);
        s.end(Some([0.0, 20.0]));
        // Two vertices per point; the closing pair is dropped from the draw:
        // 4 vertices − 2 = 2, so 6 indices.
        assert_eq!(s.vertices().len(), 4);
        assert_eq!(s.count(), 2);
        assert_eq!(s.index_count(), 6);
        // The first chord runs along +y, so its normal (−10, 0) scales to
        // (−2, 0); the pair sits on the start point, less and plus the
        // normal. The second pair sits on (0, 10) with the same normal.
        let v = s.vertices();
        assert_eq!(v[0].position, [2.0, 0.0]);
        assert_eq!(v[1].position, [-2.0, 0.0]);
        assert_eq!(v[2].position, [2.0, 10.0]);
        assert_eq!(v[3].position, [-2.0, 10.0]);
        assert_eq!(v[0].uv, [0.0, 1.0]);
        assert_eq!(v[1].uv, [0.0, 0.0]);
        assert_eq!(v[2].uv, [1.0, 1.0]);
        // Colour starts all zero.
        assert_eq!(v[0].colour, [0, 0, 0, 0]);
        // Mode 2 zeroes only the first vertex's term; a straight line has
        // none to give the others.
        assert_eq!(v[0].curvature, 0.0);
        assert_eq!(v[1].curvature, 0.0);
    }

    #[test]
    fn closing_without_a_point_extends_the_last_segment() {
        let mut s = SplineStrip::new();
        s.draw_state(1.0);
        s.begin([0.0, 0.0], 3);
        s.add_point([10.0, 0.0], 0);
        s.add_point([10.0, 10.0], 0);
        s.end(None);
        // The end point is (10, 0) + (0, 10) · 2 = (10, 20): the chord from
        // (10, 0) is straight up, so the last pair sits on (10, 10) with
        // the normal (−1, 0).
        let v = s.vertices();
        assert_eq!(v.len(), 6);
        assert!((v[4].position[0] - 11.0).abs() < 1e-5, "{:?}", v[4]);
        assert!((v[4].position[1] - 10.0).abs() < 1e-5);
        assert!((v[5].position[0] - 9.0).abs() < 1e-5);
        assert_eq!(s.count(), 4);
        assert_eq!(s.index_count(), 12);
    }

    #[test]
    fn the_curvature_term_is_the_turn_over_the_width_squared() {
        // A 90 degree joint, width 2: the second chord's normal
        // (−√2, √2) dotted with the 10 cm incoming segment and the 10 cm
        // outgoing one gives −14.14 and 14.14; (−28.28) · −0.09 / 4.
        let mut s = SplineStrip::new();
        s.draw_state(2.0);
        s.begin([0.0, 0.0], 2);
        s.add_point([10.0, 0.0], 0);
        s.end(Some([10.0, 10.0]));
        let v = s.vertices();
        assert!(
            (v[2].curvature - 0.636_396).abs() < 1e-4,
            "{}",
            v[2].curvature
        );
        assert_eq!(v[2].curvature, v[3].curvature);
    }

    #[test]
    fn modes_one_and_two_zero_the_second_and_first_vertex() {
        let mut s = SplineStrip::new();
        s.draw_state(2.0);
        s.begin([0.0, 0.0], 2);
        s.add_point([10.0, 0.0], 0);
        s.add_point([10.0, 10.0], 1);
        s.add_point([20.0, 10.0], 2);
        let v = s.vertices();
        assert_ne!(v[2].curvature, 0.0);
        assert_eq!(v[3].curvature, 0.0);
        assert_eq!(v[4].curvature, 0.0);
        assert_ne!(v[5].curvature, 0.0);
    }

    #[test]
    fn the_fade_uses_the_first_vertex_index_and_rounds_to_even() {
        let mut s = SplineStrip::new();
        s.draw_state(1.0);
        s.colour = [10, 20, 30, 5];
        s.fade = true;
        s.begin([0.0, 0.0], 4);
        for x in 1..=3 {
            s.add_point([x as f32, 0.0], 0);
        }
        let alpha: Vec<u8> = s.vertices().iter().map(|v| v.colour[3]).collect();
        // 5 · (0/4) · ½ = 0; 5 · (2/4) · ½ = 1.25 → 1; 5 · (4/4) · ½ = 2.5
        // → 2 (ties to even). Both vertices of a pair share the value.
        assert_eq!(alpha, [0, 0, 1, 1, 2, 2]);
        assert_eq!(s.vertices()[0].colour[..3], [10, 20, 30]);
    }

    #[test]
    fn the_fade_keeps_only_the_low_byte() {
        let mut s = SplineStrip::new();
        s.draw_state(1.0);
        s.colour = [0, 0, 0, 200];
        s.fade = true;
        s.begin([0.0, 0.0], 1);
        for x in 1..=3 {
            s.add_point([x as f32, 0.0], 0);
        }
        // Pair 2 starts at vertex 4: 200 · (4/1) · ½ = 400 → 0x190 → 0x90.
        assert_eq!(s.vertices()[4].colour[3], 0x90);
        // Without the flag the colour is copied untouched.
        s.fade = false;
        s.begin([0.0, 0.0], 1);
        s.add_point([1.0, 0.0], 0);
        assert_eq!(s.vertices()[0].colour[3], 200);
    }

    #[test]
    fn a_zero_length_spline_gives_zero_fade() {
        let mut s = SplineStrip::new();
        s.draw_state(1.0);
        s.colour = [0, 0, 0, 255];
        s.fade = true;
        s.begin([0.0, 0.0], 0);
        s.add_point([1.0, 0.0], 0);
        s.add_point([2.0, 0.0], 0);
        // 0/0 and x/0 convert to the indefinite integer, whose low byte is 0.
        assert_eq!(s.vertices()[0].colour[3], 0);
        assert_eq!(s.vertices()[2].colour[3], 0);
    }

    #[test]
    fn vertices_pack_to_the_thirty_two_byte_layout() {
        let v = SplineVertex {
            position: [1.0, 2.0],
            uv: [3.0, 4.0],
            curvature: 5.0,
            colour: [7, 8, 9, 10],
        };
        let bytes = v.to_bytes();
        assert_eq!(bytes.len(), 32);
        assert_eq!(&bytes[0..4], &1f32.to_le_bytes());
        assert_eq!(&bytes[4..8], &2f32.to_le_bytes());
        assert_eq!(&bytes[8..12], &3f32.to_le_bytes());
        assert_eq!(&bytes[12..16], &4f32.to_le_bytes());
        assert_eq!(&bytes[16..20], &5f32.to_le_bytes());
        assert_eq!(&bytes[20..24], &[0, 0, 0, 0]);
        assert_eq!(&bytes[24..28], &[7, 8, 9, 10]);
        assert_eq!(&bytes[28..32], &[0, 0, 0, 0]);
    }
    #[test]
    fn the_line_uniforms_are_single_precision() {
        let widths = line_widths(0.1);
        assert_eq!(widths[0], 0.1f32 * 0.5 + (0.1f32 * 4.0).sqrt());
        // y is summed in double and rounded once.
        assert_eq!(widths[1], -((f64::from(0.1f32) * 0.3 + 0.15) as f32));
        assert_eq!(widths[2], (0.1f32 * 6.0).sqrt());
        let u = line_uniforms(0.1, [0, 0, 1920, 1080]);
        assert_eq!(u.inv_width, 1.0 / 1920.0f32);
        assert_eq!(u.inv_height, 1.0 / 1080.0f32);
        assert_eq!(u.widths, widths);
    }
}
