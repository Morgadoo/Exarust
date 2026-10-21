//! The thaumaturgy skill tree's spline lines, as the original draws them: a
//! tessellated 2D polyline turned into a quad strip, drawn with a small
//! built-in anti-aliased line program.
//!
//! [`StripBuffer`] is the pre-built index list: 2 048 quads, two triangles
//! each, with the per-vertex strip coordinates (`u` the segment index, `v`
//! 0 or 1 across the line). [`SplineStrip`] tessellates a spline into that
//! strip: every new point becomes two vertices offset either side of the
//! line by its width along the segment's 2D normal, the strip's pixel width
//! coming from the shader's `lnwdt` uniform. The last pair is dropped from
//! the index run, as the original draws `2·(points − 1)` vertices.
//!
//! The line program's fragment rule is reproduced by [`LINE_FRAGMENT_WGSL`]:
//! the soft edge is computed from the strip coordinate, the width scales the
//! coverage, and the vertex colour (stored as bytes, scaled by 1/255) is
//! multiplied by it.
//!
//! Guesses: the original's three helper routines are not in the pack's
//! disassembly, so their argument order is taken from the pseudo-code
//! ([`sub`] subtracts, [`add`] adds, [`scale_to_length`] normalises); the
//! vertex layout follows the attribute pointers (position, texture
//! coordinate, strip coordinate, four colour bytes, stride 32).

use bevy::math::Vec2;

/// Quads the index list holds.
pub const MAX_QUADS: usize = 2048;
/// Floats per vertex: position (2), texture coordinate (2), strip
/// coordinate (2), colour bytes as one word (1) and padding (1).
pub const VERTEX_FLOATS: usize = 8;
/// Bytes per vertex (the original's attribute stride).
pub const VERTEX_STRIDE: usize = 32;

/// The pre-built index list for up to [`MAX_QUADS`] quads: for quad `i`, the
/// triangles (2i, 2i+1, 2i+3) and (2i, 2i+3, 2i+2).
pub fn quad_indices() -> Vec<u16> {
    let mut out = Vec::with_capacity(MAX_QUADS * 6);
    for i in 0..MAX_QUADS {
        let v = (2 * i) as u16;
        out.extend_from_slice(&[v, v + 1, v + 3, v, v + 3, v + 2]);
    }
    out
}

/// The strip coordinates of quad `i`: `(u, 1)` on the first vertex and
/// `(u, 0)` on the second, `u` being the segment index as a float.
pub fn strip_coords(i: usize) -> [[f32; 2]; 2] {
    let u = i as f32;
    [[u, 1.0], [u, 0.0]]
}

/// The line program's three width values, from the draw call's width:
/// `x = width/2 + √(4·width)`, `y = −(0.3·width + 0.15)` in double
/// precision, `z = √(6·width)`.
pub fn line_widths(width: f32) -> [f32; 3] {
    let x = width * 0.5 + (width * 4.0).sqrt();
    let y = -((f64::from(width) * 0.3 + 0.15) as f32);
    let z = (width * 6.0).sqrt();
    [x, y, z]
}

/// The line program's two viewport reciprocals: `1/(right − left)` and
/// `1/(bottom − top)`, the integer extents converted to float.
pub fn viewport_reciprocals(left: i32, right: i32, top: i32, bottom: i32) -> [f32; 2] {
    let x = (right as i64 - left as i64) as f32;
    let y = (bottom as i64 - top as i64) as f32;
    [1.0 / x, 1.0 / y]
}

/// The fragment rule of the built-in line program, as WGSL. The original's
/// GLSL names are kept only in comments; this is Exarust's own program.
pub const LINE_FRAGMENT_WGSL: &str = r#"
// v_tex: the strip coordinate (u = segment, v = 0 or 1 across the line).
// v_strip: x = the segment's curvature term.
// v_colour: the vertex colour, stored as bytes and scaled by 1/255.
// widths: x = width/2 + sqrt(4*width), y = -(0.3*width + 0.15), z = sqrt(6*width).
@fragment
fn fragment(
    v_tex: vec2<f32>,
    v_strip: vec2<f32>,
    v_colour: vec4<f32>,
    widths: vec3<f32>,
) -> @location(0) vec4<f32> {
    var a = abs(0.5 - fract(v_tex.x));
    a = sqrt(1.0 - a * 2.0);
    a = clamp(0.5 + a * v_strip.x, 0.0, 1.0);
    a = 0.5 - abs(v_tex.y - a);
    let b = clamp(a * widths.x + widths.y, 0.0, 1.0);
    let rgb = v_colour.rgb * b;
    a = clamp(a * widths.z * 0.2, 0.0, 1.0);
    a = clamp(a + b, 0.0, 1.0);
    return vec4<f32>(rgb, v_colour.a * a * a);
}
"#;

/// A packed vertex: position, texture coordinate, strip coordinate and the
/// four colour bytes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplineVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub strip: [f32; 2],
    pub colour: [u8; 4],
}

impl SplineVertex {
    /// The vertex as the original's 32-byte record: two floats each for the
    /// position, texture and strip coordinates, then the colour bytes and
    /// four bytes of padding.
    pub fn to_bytes(self) -> [u8; VERTEX_STRIDE] {
        let mut out = [0u8; VERTEX_STRIDE];
        let mut put = |at: usize, f: f32| {
            out[at..at + 4].copy_from_slice(&f.to_le_bytes());
        };
        put(0, self.position[0]);
        put(4, self.position[1]);
        put(8, self.uv[0]);
        put(12, self.uv[1]);
        put(16, self.strip[0]);
        put(20, self.strip[1]);
        out[24..28].copy_from_slice(&self.colour);
        out
    }
}

/// Vertex subtraction, `a − b`.
fn sub(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

/// Vertex addition, `a + b`.
fn add(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] + b[0], a[1] + b[1]]
}

/// `v` scaled to `length`: `v · (length / |v|)` in single precision, as the
/// original's normalise-and-scale helper.
fn scale_to_length(v: [f32; 2], length: f32) -> [f32; 2] {
    let len = (v[0] * v[0] + v[1] * v[1]).sqrt();
    let s = length / len;
    [v[0] * s, v[1] * s]
}

/// One tessellated spline: the strip's vertices in draw order.
#[derive(Clone, Debug, Default)]
pub struct SplineStrip {
    vertices: Vec<SplineVertex>,
    /// The previous and current spline points.
    previous: [f32; 2],
    current: [f32; 2],
    /// The vertex count the original keeps.
    count: i32,
    /// The strip's width.
    width: f32,
    /// The spline's length in segments, which the fade divides each
    /// vertex's share by.
    pub segments: i32,
    /// The colour of the vertices added next.
    pub colour: [u8; 4],
    /// The line's curvature term at the current segment (the strip
    /// coordinate's `x`).
    pub curvature: f32,
    /// When set, each vertex's alpha byte is the segment share
    /// (`index / segments · 0x?.? · 0.5`, rounded to even); the original
    /// writes it in the vertex's alpha byte when the render object's flag
    /// byte is set.
    pub fade: bool,
    /// The alpha the fade writes at full share.
    pub fade_alpha: u8,
}

impl SplineStrip {
    /// A strip of the given width, with black opaque vertices.
    pub fn new(width: f32) -> Self {
        Self {
            width,
            colour: [0, 0, 0, 255],
            ..Default::default()
        }
    }

    /// Starts a spline at `start`: no vertices, the previous and current
    /// points both there, and the width stored. Returns the strip for
    /// chaining.
    pub fn begin(&mut self, start: [f32; 2], width: f32) -> &mut Self {
        self.count = 0;
        self.vertices.clear();
        self.width = width;
        self.previous = start;
        self.current = start;
        self
    }

    /// The strip's width.
    pub fn width(&self) -> f32 {
        self.width
    }

    /// The vertices written so far (two per added point, less the dropped
    /// final pair after [`SplineStrip::end`]).
    pub fn vertices(&self) -> &[SplineVertex] {
        &self.vertices
    }

    /// The vertex count the original tracks.
    pub fn count(&self) -> i32 {
        self.count
    }

    /// Adds `end` and closes the strip: without an explicit point it takes
    /// the midpoint of the previous and current points (the original's
    /// 2-point interpolation at 0.5). Then the final pair is dropped from
    /// the index run.
    pub fn end(&mut self, end: Option<[f32; 2]>) -> &mut Self {
        match end {
            Some(p) => {
                self.add_point(p, 0);
            }
            None => {
                let mid = [
                    (self.previous[0] + self.current[0]) * 0.5,
                    (self.previous[1] + self.current[1]) * 0.5,
                ];
                self.add_point(mid, 0);
            }
        }
        self.count -= 2;
        self
    }

    /// The number of indices the draw submits: `3 · count`, i.e. two
    /// triangles per segment.
    pub fn index_count(&self) -> usize {
        (self.count.max(0) as usize) * 3
    }

    /// Adds one spline point, with `mode` 2 writing a zero curvature term
    /// (a hard join), any other mode computing the join's curvature from the
    /// chord normal against the two segment directions.
    pub fn add_point(&mut self, point: [f32; 2], mode: i32) -> &mut Self {
        let d = sub(point, self.previous);
        // The 2D normal of the chord from the previous point: (−dy, dx),
        // scaled to the strip's width.
        let normal = scale_to_length([-d[1], d[0]], self.width);
        let index = self.vertices.len() / 2;
        let strip = strip_coords(index);
        let mut colour = self.colour;
        if self.fade {
            let segments = f64::from(self.segments.max(1));
            let share = (f64::from(index as i32) / segments) * 0.5;
            colour[3] = (f64::from(self.fade_alpha) * share).round_ties_even() as u8;
        }
        let curvature = if mode == 2 {
            0.0
        } else {
            self.join_curvature(point)
        };
        // First vertex: the current point less the normal, u = segment,
        // v = 1.
        self.push(SplineVertex {
            position: sub(self.current, normal),
            uv: strip[0],
            strip: [curvature, 0.0],
            colour,
        });
        self.count += 1;
        // Second vertex: the current point plus the normal, v = 0.
        self.push(SplineVertex {
            position: add(self.current, normal),
            uv: strip[1],
            strip: [curvature, 0.0],
            colour,
        });
        self.count += 1;
        self.previous = self.current;
        self.current = point;
        self
    }

    fn push(&mut self, v: SplineVertex) {
        self.vertices.push(v);
    }

    /// The join's curvature term, from the chord normal `n`: the dot of `n`
    /// with the incoming segment less the dot with the outgoing one, times
    /// −0.09, over the width squared. The two dots and the width's square
    /// are single precision; the product and division are double, as the
    /// original does (its segment directions are not normalised).
    fn join_curvature(&self, point: [f32; 2]) -> f32 {
        let d = sub(point, self.previous);
        let normal = scale_to_length([-d[1], d[0]], self.width);
        let into = sub(self.current, self.previous);
        let out = sub(point, self.current);
        let dot = |a: [f32; 2], b: [f32; 2]| a[0] * b[0] + a[1] * b[1];
        let a = dot(normal, into);
        let b = dot(normal, out);
        let numerator = f64::from(a - b) * -0.09;
        let w = self.width * self.width;
        (numerator / f64::from(w)) as f32
    }
}

/// The current point of a spline as a [`Vec2`], for callers that keep their
/// points in Bevy's vector type.
pub fn vec2(p: [f32; 2]) -> Vec2 {
    Vec2::new(p[0], p[1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_index_table_builds_two_triangles_per_quad() {
        let indices = quad_indices();
        assert_eq!(indices.len(), MAX_QUADS * 6);
        assert_eq!(&indices[..6], &[0, 1, 3, 0, 3, 2]);
        assert_eq!(&indices[6..12], &[2, 3, 5, 2, 5, 4]);
        // The last quad stays inside the vertex range.
        // The original's last quad reaches vertex 4 097 (its table overruns the
        // 4 096 strip coordinates by two; no spline gets that long).
        assert_eq!(indices[indices.len() - 1], 4096);
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
        // The reciprocal viewport extents use the integer spans.
        assert_eq!(
            viewport_reciprocals(10, 110, 20, 120),
            [1.0 / 100.0, 1.0 / 100.0]
        );
    }

    #[test]
    fn a_straight_spline_puts_its_vertices_either_side() {
        let mut s = SplineStrip::new(2.0);
        s.begin([0.0, 0.0], 2.0);
        s.add_point([0.0, 10.0], 2);
        s.end(Some([0.0, 20.0]));
        // Two vertices per point, the last pair still in the vertex list but
        // dropped from the index run: two points → 4 vertices, count 2.
        assert_eq!(s.vertices().len(), 4);
        assert_eq!(s.count(), 2);
        assert_eq!(s.index_count(), 6);
        // Along +y the normal is (−1, 0) × width 2: the current point less
        // and plus it, at (0, 0) then (0, 10).
        let v = s.vertices();
        assert_eq!(v[0].position, [2.0, 0.0]);
        assert_eq!(v[1].position, [-2.0, 0.0]);
        assert_eq!(v[2].position, [2.0, 10.0]);
        assert_eq!(v[3].position, [-2.0, 10.0]);
        assert_eq!(v[0].uv, [0.0, 1.0]);
        assert_eq!(v[1].uv, [0.0, 0.0]);
        assert_eq!(v[2].uv, [1.0, 1.0]);
        assert_eq!(v[0].colour, [0, 0, 0, 255]);
        // Mode 2 leaves the curvature term at zero.
        assert_eq!(v[0].strip, [0.0, 0.0]);
    }

    #[test]
    fn a_closing_strip_without_a_point_uses_the_midpoint() {
        let mut s = SplineStrip::new(1.0);
        s.begin([0.0, 0.0], 1.0);
        s.add_point([10.0, 0.0], 2);
        s.end(None);
        // The midpoint (5, 0) only gives the last pair its normal: the pair
        // is drawn at the current point (10, 0), and that pair is then
        // dropped from the index run.
        assert_eq!(s.vertices()[2].position, [10.0, -1.0]);
        assert_eq!(s.vertices()[3].position, [10.0, 1.0]);
        assert_eq!(s.count(), 2);
    }

    #[test]
    fn the_curvature_term_is_the_turn_over_the_width_squared() {
        // A 90° joint, width 2: the normal (−√2, √2) dotted with the 10 cm
        // incoming segment and the 10 cm outgoing one gives −28.28, times
        // −0.09 over 4.
        let mut s = SplineStrip::new(2.0);
        s.begin([0.0, 0.0], 2.0);
        s.add_point([10.0, 0.0], 0);
        s.end(Some([10.0, 10.0]));
        let turn = s.vertices()[2].strip[0];
        assert!((turn - 0.636_396).abs() < 1e-4, "{turn}");
    }

    #[test]
    fn the_fade_share_uses_the_segment_index() {
        let mut s = SplineStrip::new(1.0);
        s.segments = 4;
        s.fade = true;
        s.fade_alpha = 200;
        s.begin([0.0, 0.0], 1.0);
        s.add_point([1.0, 0.0], 2);
        s.add_point([2.0, 0.0], 2);
        // 200 · (0/4) · 0.5 = 0 for the first pair, 200 · (1/4) · 0.5 = 25
        // for the second.
        assert_eq!(s.vertices()[0].colour[3], 0);
        assert_eq!(s.vertices()[2].colour[3], 25);
    }

    #[test]
    fn vertices_pack_to_the_thirty_two_byte_layout() {
        let v = SplineVertex {
            position: [1.0, 2.0],
            uv: [3.0, 4.0],
            strip: [5.0, 6.0],
            colour: [7, 8, 9, 10],
        };
        let bytes = v.to_bytes();
        assert_eq!(bytes.len(), 32);
        assert_eq!(&bytes[0..4], &1f32.to_le_bytes());
        assert_eq!(&bytes[4..8], &2f32.to_le_bytes());
        assert_eq!(&bytes[8..12], &3f32.to_le_bytes());
        assert_eq!(&bytes[12..16], &4f32.to_le_bytes());
        assert_eq!(&bytes[16..20], &5f32.to_le_bytes());
        assert_eq!(&bytes[20..24], &6f32.to_le_bytes());
        assert_eq!(&bytes[24..28], &[7, 8, 9, 10]);
        assert_eq!(&bytes[28..32], &[0, 0, 0, 0]);
    }
}
