//! The super-sampling view's sub-frame matrices and clocks.
//!
//! When the renderer super-samples, a frame is drawn in quarters of up to
//! four jittered sub-frames each. For every sub-frame the 3-D view builds a
//! projection from the camera's near-plane half extents shifted by a
//! pre-computed jitter, a view matrix from the camera's world transform, and
//! the composite `view · projection` the draw multiplies the object's world
//! matrix by. The draw uploads `ObjMat[0]` (the object) and `ObjMat[1]` (the
//! object times the sub-frame composite), shifts the viewport by the passed
//! sample's whole-pixel offset, and, for a geometry that asks for it, rewrites
//! the projection's depth lane with the sub-frame's share of the shutter.
//!
//! The engine's matrices are stored row by row (a `vec*` product on the CPU);
//! GL receives the same bytes, so the shader's `matrix * vec` is that product.
//! [`Matrix`] keeps the engine's row-major order, and [`multiply`] the
//! engine's summation order.
//!
//! Exarust renders its super-sampled scene at twice the window's resolution
//! instead of jittering four sub-frames, so this module is the original's own
//! fill, tested beside [`crate::supersample`]; Bevy owns the real draws
//! (`docs/RENDERING.md`, Shader builder).

/// A 4×4 matrix in the engine's row-major order: `m[row][col]`.
pub type Matrix = [[f32; 4]; 4];

/// The setting's sample count, from the view's level table; an unknown
/// setting takes one.
pub fn sample_count(index: u32) -> u32 {
    match index {
        0..=7 => crate::view::SupersampleLevel(index as u8).subviews() as u32,
        _ => 1,
    }
}

/// The sub-frames a quarter draws: the sample count capped at four.
pub fn passes(index: u32) -> u32 {
    sample_count(index).min(4)
}

/// The jitter of one sample of a setting, in window pixels, from the view's
/// table; zero past the setting's samples.
pub fn jitter(index: u32, quarter: u32, subframe: u32) -> [f32; 2] {
    if index > 7 {
        return [0.0; 2];
    }
    let (x, y) =
        crate::view::SupersampleLevel(index as u8).jitter(quarter as usize, subframe as usize);
    [x, y]
}

/// The whole-pixel viewport offset of a sub-frame.
pub fn viewport_shift(subframe: u32) -> [i32; 2] {
    crate::view::viewport_shift(subframe as usize)
}

/// The quarter's sub-frame time offset, the value `Global.ViewPos.w` and
/// `Global.TimeDlt.x` carry: `0.5 − (4·quarter + 0.5)/samples`, in single
/// precision.
pub fn frame_offset(quarter: u32, samples: u32) -> f32 {
    let samples = samples.max(1) as f32;
    let index = quarter.wrapping_mul(4) as f32;
    0.5 - (index + 0.5) / samples
}

/// The quarter's sample fraction, `(4·quarter + 1)/samples`. The engine keeps
/// it as the capture's weight (`1/(4·quarter + 1)` is its reciprocal).
pub fn sample_fraction(quarter: u32, samples: u32) -> f32 {
    let samples = samples.max(1) as f32;
    (quarter.wrapping_mul(4) as f32 + 1.0) / samples
}

/// The step between a frame's sub-frame animation clocks: the frame's time
/// over the sample count.
pub fn animation_step(frame_time: f32, samples: u32) -> f32 {
    frame_time / samples.max(1) as f32
}

/// The first sub-frame's animation clock in a quarter:
/// `step·0.5 + frame_time·(−0.5)`, plus the whole samples of the earlier
/// quarters (`quarter·step·4`). Single precision in the engine's order.
pub fn animation_base(quarter: u32, frame_time: f32, samples: u32) -> f32 {
    let step = animation_step(frame_time, samples);
    (quarter as f32 * step) * 4.0 + step * 0.5 + frame_time * -0.5
}

/// One sub-frame's animation clock: `subframe·step + base`.
pub fn animation_clock(subframe: u32, quarter: u32, frame_time: f32, samples: u32) -> f32 {
    subframe as f32 * animation_step(frame_time, samples)
        + animation_base(quarter, frame_time, samples)
}

/// A sub-frame's depth lane for a geometry that asks for it: the lane's
/// unmodified value `base` plus the sample's share of the shutter, in the
/// engine's order `((slot + 0.5)·duration/samples + base) − duration·0.5`.
/// `slot` is the sample's index in the frame (`4·quarter + subframe`).
pub fn depth_lane(base: f32, slot: u32, duration: f32, samples: u32) -> f32 {
    ((slot as f32 + 0.5) * duration / samples.max(1) as f32 + base) - duration * 0.5
}

/// The sub-frame's share of a shutter alone, relative to the unmodified
/// lane: `((slot + 0.5)·duration/samples − duration·0.5)`.
pub fn shutter_offset(slot: u32, duration: f32, samples: u32) -> f32 {
    depth_lane(0.0, slot, duration, samples)
}

/// A perspective projection from the frustum's edges, in the engine's
/// arithmetic and matrix order.
///
/// `near` is doubled before the divisions; the depth lane takes the infinite
/// branch when `far` is zero (an unordered compare sends a NaN `far` to the
/// finite branch), with `m22 = −1` and `m32 = −2·near`. Other entries are the
/// identity's zeroes; `m23` is −1 and `m33` 0.
pub fn projection(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Matrix {
    let mut m = [[0.0f32; 4]; 4];
    m[1][1] = 1.0;
    m[2][2] = 1.0;
    m[3][3] = 1.0;
    let width = right - left;
    let height = top - bottom;
    m[0][0] = (2.0 * near) / width;
    m[1][1] = (2.0 * near) / height;
    m[2][0] = (right + left) / width;
    m[2][1] = (top + bottom) / height;
    // `comiss` with an unordered result takes the finite branch; zero (either
    // sign) takes the infinite one.
    if far.is_nan() || far != 0.0 {
        m[2][2] = -(far + near) / (far - near);
        m[3][2] = -(2.0 * far * near) / (far - near);
    } else {
        m[2][2] = -1.0;
        m[3][2] = near * -2.0;
    }
    m[2][3] = -1.0;
    m[3][3] = 0.0;
    m
}

/// One sub-frame's projection: the camera's stored near-plane half extents
/// (negative, as the view keeps them) shifted by the sample's jitter.
///
/// The shift is `2·half·jitter/viewport` on each axis (the camera's extents
/// are negative, so the shift runs opposite the jitter's sign), and the
/// frustum's edges are the shifted half extents. Single precision in the
/// engine's order.
pub fn subframe_projection(
    half: [f32; 2],
    jitter: [f32; 2],
    viewport: [u32; 2],
    near: f32,
    far: f32,
) -> Matrix {
    let dx = crate::view::jitter_offset(jitter[0], half[0], viewport[0]);
    let dy = crate::view::jitter_offset(jitter[1], half[1], viewport[1]);
    let right = -half[0] + dx;
    let left = half[0] + dx;
    let bottom = half[1] + dy;
    let top = -half[1] + dy;
    projection(left, right, bottom, top, near, far)
}

/// The view matrix of a camera whose transform is four rows of three numbers:
/// three world axes and a translation row.
///
/// The rotation is transposed and the translation becomes `−Rᵀ·t`; the
/// fourth column is `(0, 0, 0, 1)`. Each translation component is summed in
/// the engine's order, `(−t.z·r.z + −t.x·r.x) + −t.y·r.y`.
pub fn view_from_camera(camera: &[[f32; 3]; 4]) -> Matrix {
    let r0 = [camera[0][0], camera[1][0], camera[2][0]];
    let r1 = [camera[0][1], camera[1][1], camera[2][1]];
    let r2 = [camera[0][2], camera[1][2], camera[2][2]];
    let t = camera[3];
    let row3 = |r: [f32; 3]| ((-t[2] * r[2]) + (-t[0] * r[0])) + (-t[1] * r[1]);
    [
        [r0[0], r0[1], r0[2], 0.0],
        [r1[0], r1[1], r1[2], 0.0],
        [r2[0], r2[1], r2[2], 0.0],
        [row3(r0), row3(r1), row3(r2), 1.0],
    ]
}

/// The product of two row-major matrices, summed as the engine does: each
/// component is `(a·b + a·b) + (a·b + a·b)`.
pub fn multiply(a: &Matrix, b: &Matrix) -> Matrix {
    let mut out = [[0.0f32; 4]; 4];
    for (i, row) in out.iter_mut().enumerate() {
        for (j, value) in row.iter_mut().enumerate() {
            *value =
                (a[i][0] * b[0][j] + a[i][1] * b[1][j]) + (a[i][2] * b[2][j] + a[i][3] * b[3][j]);
        }
    }
    out
}

/// A point transformed by the matrices' first three columns (the engine's
/// point helper): `((x·row0 + y·row1) + (z·row2 + row3))` per component.
pub fn transform_point(m: &Matrix, point: [f32; 3]) -> [f32; 3] {
    let mut out = [0.0f32; 3];
    for (j, value) in out.iter_mut().enumerate() {
        *value = (point[0] * m[0][j] + point[1] * m[1][j]) + (point[2] * m[2][j] + m[3][j]);
    }
    out
}

/// The values of one view a frame's sub-frames are built from.
#[derive(Clone, Copy, Debug)]
pub struct ViewFrame {
    /// The camera's near-plane half extents as the view stores them: the
    /// vertical one and the horizontal one (aspect times it). They are
    /// negative in the engine (the projection's frustum edges are their
    /// negations). `[guess on the sign: the view's own builder negates the
    /// near plane before dividing]`.
    pub half_extents: [f32; 2],
    /// The window's width and height, texels.
    pub viewport: [u32; 2],
    /// The camera's near and far planes, cm.
    pub near: f32,
    pub far: f32,
    /// The camera's world transform: three axes and a translation row.
    pub camera: [[f32; 3]; 4],
    /// The frame's time (the view's shutter length), which spaces the
    /// sub-frames' animation clocks and their depth offsets.
    pub frame_time: f32,
}

impl Default for ViewFrame {
    fn default() -> Self {
        Self {
            half_extents: [-10.0, -10.0],
            viewport: [1920, 1080],
            near: 100.0,
            far: 12_500.0,
            camera: [
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 0.0],
            ],
            frame_time: 1.0 / 60.0,
        }
    }
}

/// The per-sub-frame matrices of one quarter.
#[derive(Clone, Debug)]
pub struct Subframes {
    setting: u32,
    quarter: u32,
    projections: Vec<Matrix>,
    views: Vec<Matrix>,
    composites: Vec<Matrix>,
    frame_time: f32,
}

impl Subframes {
    /// Build a quarter's sub-frames: one projection per pass, the view (the
    /// same for a static camera) and their product.
    ///
    /// `camera_for` returns the camera transform for a sub-frame, in the
    /// engine's descending pass order, so a caller can animate it between
    /// passes as the view does.
    pub fn build(
        setting: u32,
        quarter: u32,
        view: &ViewFrame,
        mut camera_for: impl FnMut(u32) -> [[f32; 3]; 4],
    ) -> Self {
        let passes = passes(setting);
        let mut projections = Vec::with_capacity(passes as usize);
        for i in 0..passes {
            let j = jitter(setting, quarter, i);
            projections.push(subframe_projection(
                view.half_extents,
                j,
                view.viewport,
                view.near,
                view.far,
            ));
        }
        let mut views = vec![[[0.0f32; 4]; 4]; passes as usize];
        let mut composites = vec![[[0.0f32; 4]; 4]; passes as usize];
        for i in (0..passes).rev() {
            let view_matrix = view_from_camera(&camera_for(i));
            views[i as usize] = view_matrix;
            composites[i as usize] = multiply(&view_matrix, &projections[i as usize]);
        }
        Self {
            setting,
            quarter,
            projections,
            views,
            composites,
            frame_time: view.frame_time,
        }
    }

    /// The setting this quarter was built for.
    pub fn setting(&self) -> u32 {
        self.setting
    }

    /// The quarter's index.
    pub fn quarter(&self) -> u32 {
        self.quarter
    }

    /// The sub-frames drawn.
    pub fn len(&self) -> usize {
        self.projections.len()
    }

    pub fn is_empty(&self) -> bool {
        self.projections.is_empty()
    }

    pub fn projection(&self, subframe: usize) -> Option<&Matrix> {
        self.projections.get(subframe)
    }

    pub fn view(&self, subframe: usize) -> Option<&Matrix> {
        self.views.get(subframe)
    }

    pub fn composite(&self, subframe: usize) -> Option<&Matrix> {
        self.composites.get(subframe)
    }

    /// The `Global` frame offset of the quarter.
    pub fn frame_offset(&self) -> f32 {
        frame_offset(self.quarter, sample_count(self.setting))
    }

    /// A sub-frame's animation clock, for the draw to set on an animated
    /// node.
    pub fn clock(&self, subframe: u32) -> f32 {
        animation_clock(
            subframe,
            self.quarter,
            self.frame_time,
            sample_count(self.setting),
        )
    }

    /// The whole-pixel viewport shift of a sub-frame.
    pub fn viewport_shift(&self, subframe: u32) -> [i32; 2] {
        viewport_shift(subframe)
    }

    /// Apply the depth offset of a geometry that asks for it: the projection's
    /// depth lane becomes its old value plus the sub-frame's share of the
    /// shutter, and the composite is rebuilt from the unchanged view.
    ///
    /// The engine applies this to the plain and the reflection matrix pairs
    /// alike, over every pass, when one of the pass's geometry records has
    /// flag `0x20`.
    pub fn apply_depth_offset(&mut self, duration: f32) {
        let samples = sample_count(self.setting);
        let first = self.quarter.wrapping_mul(4);
        // The engine reads the first pass's lane once, before the loop, and
        // writes every pass's lane from it (so the offsets do not compound
        // pass by pass).
        let Some(base) = self.projections.first().map(|p| p[2][2]) else {
            return;
        };
        for i in 0..self.projections.len() {
            let slot = first + i as u32;
            let projection = &mut self.projections[i];
            projection[2][2] = depth_lane(base, slot, duration, samples);
            self.composites[i] = multiply(&self.views[i], projection);
        }
    }

    /// The two matrices a draw uploads for one pass: the object's world
    /// matrix and the object times the pass's composite.
    pub fn object_pair(&self, object: &Matrix, subframe: usize) -> Option<[Matrix; 2]> {
        let composite = self.composites.get(subframe)?;
        Some([*object, multiply(object, composite)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn static_camera(_: u32) -> [[f32; 3]; 4] {
        [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 2.0, 3.0],
        ]
    }

    fn view() -> ViewFrame {
        ViewFrame {
            half_extents: [-10.0, -5.0],
            viewport: [800, 600],
            near: 1.0,
            far: 100.0,
            camera: static_camera(0),
            frame_time: 0.1,
        }
    }

    fn j(index: usize) -> [f32; 2] {
        let (x, y) = crate::view::JITTERS[index];
        [x, y]
    }

    #[test]
    fn the_settings_come_from_the_views_level_table() {
        assert_eq!(
            (0..8).map(sample_count).collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 8, 15, 24, 66]
        );
        assert_eq!(
            (0..8).map(passes).collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 4, 4, 4, 4]
        );
        // A setting past the table takes one sample.
        assert_eq!(sample_count(8), 1);
        assert_eq!(passes(8), 1);
    }

    #[test]
    fn the_jitter_table_holds_the_engines_offsets() {
        assert_eq!(crate::view::JITTERS.len(), 124);
        // The first sample of the 4x setting is entry 6.
        assert_eq!(jitter(3, 0, 0)[0].to_bits(), 0xBE95D313);
        assert_eq!(jitter(3, 0, 0)[1].to_bits(), 0xBE198B2F);
        // The second sample of a quarter is the next entry.
        assert_eq!(jitter(3, 0, 1)[0].to_bits(), 0x3E98066C);
        // The 4x setting's four samples are entries 6 through 9.
        assert_eq!(jitter(3, 0, 0), j(6));
        assert_eq!(jitter(3, 0, 3), j(9));
        // A later quarter continues the setting's sequence: the 8-sample
        // setting's second quarter starts at entry 14.
        assert_eq!(jitter(4, 1, 0), j(14));
        // The 66-sample setting's last sample is entry 122; entry 123 is
        // the table's zero padding.
        assert_eq!(jitter(7, 16, 1), j(122));
        assert_eq!(j(123), [0.0, 0.0]);
        // A sample with no jitter is zero.
        assert_eq!(jitter(0, 0, 0), [0.0, 0.0]);
        assert_eq!(jitter(8, 0, 0), [0.0, 0.0]);
        assert_eq!(viewport_shift(0), [0, 0]);
        assert_eq!(viewport_shift(1), [1, 0]);
        assert_eq!(viewport_shift(2), [0, 1]);
        assert_eq!(viewport_shift(3), [1, 1]);
        assert_eq!(viewport_shift(4), [0, 0]);
    }

    #[test]
    fn the_quarters_offset_and_weight_follow_the_sample_count() {
        // 0.5 − 0.5/4 and 0.5 − 4.5/4.
        assert_eq!(frame_offset(0, 4).to_bits(), 0.375f32.to_bits());
        assert_eq!(frame_offset(1, 4), 0.5 - 4.5 / 4.0);
        assert_eq!(frame_offset(0, 15), 0.5 - 0.5 / 15.0);
        assert_eq!(sample_fraction(0, 4), 0.25);
        assert_eq!(sample_fraction(1, 4), 1.25);
    }

    #[test]
    fn the_animation_clock_spans_the_shutter_across_the_quarters() {
        // Four samples over a 0.1 s frame: step 0.025, the first clock
        // −0.0375, then −0.0125, 0.0125, 0.0375.
        let step = animation_step(0.1, 4);
        assert_eq!(step, 0.025);
        let base = animation_base(0, 0.1, 4);
        assert_eq!(base, -0.0375);
        let times: Vec<f32> = (0..4).map(|i| animation_clock(i, 0, 0.1, 4)).collect();
        assert_eq!(times[0], base);
        assert_eq!(times[3], 3.0 * step + base);
        // The next quarter continues after four steps.
        assert_eq!(
            animation_base(1, 0.1, 4),
            1.0 * step * 4.0 + step * 0.5 - 0.05
        );
        assert_eq!(animation_clock(0, 1, 0.1, 4), animation_base(1, 0.1, 4));
        // The offsets across the frame are the shutter's sample centres.
        for i in 0..4 {
            let expected = ((i as f32 + 0.5) / 4.0 - 0.5) * 0.1;
            assert!((animation_clock(i, 0, 0.1, 4) - expected).abs() < 1e-7);
        }
    }

    #[test]
    fn the_shutter_offset_is_centred_on_the_frame() {
        // (((slot + 0.5)·0.1)/4) − 0.05.
        assert_eq!(shutter_offset(0, 0.1, 4), 0.0125 - 0.05);
        assert_eq!(shutter_offset(3, 0.1, 4), 0.0875 - 0.05);
        assert_eq!(shutter_offset(0, 0.0, 4), 0.0);
        assert_eq!(shutter_offset(0, 0.1, 0), shutter_offset(0, 0.1, 1));
    }

    #[test]
    fn the_projection_is_the_engines_frustum() {
        let m = projection(-10.0, 10.0, -5.0, 5.0, 1.0, 100.0);
        assert_eq!(m[0][0], 0.1);
        assert_eq!(m[1][1], 0.2);
        assert_eq!(m[2][0], 0.0);
        assert_eq!(m[2][1], 0.0);
        assert_eq!(m[2][2], -(100.0 + 1.0) / (100.0 - 1.0));
        assert_eq!(m[3][2], -(2.0 * 100.0 * 1.0) / (100.0 - 1.0));
        assert_eq!(m[2][3], -1.0);
        assert_eq!(m[3][3], 0.0);
        // An asymmetric frustum offsets the centre.
        let m = projection(-8.0, 12.0, -4.0, 6.0, 2.0, 50.0);
        assert_eq!(m[0][0], 4.0 / 20.0);
        assert_eq!(m[2][0], 4.0 / 20.0);
        assert_eq!(m[1][1], 4.0 / 10.0);
        assert_eq!(m[2][1], 2.0 / 10.0);
    }

    #[test]
    fn a_zero_far_plane_takes_the_infinite_branch() {
        let m = projection(-10.0, 10.0, -5.0, 5.0, 2.0, 0.0);
        assert_eq!(m[2][2], -1.0);
        assert_eq!(m[3][2], -4.0);
        // A NaN far plane takes the finite branch (the compare's unordered
        // result does not jump to the infinite one).
        let m = projection(-10.0, 10.0, -5.0, 5.0, 2.0, f32::NAN);
        assert!(m[2][2].is_nan());
    }

    #[test]
    fn a_subframe_projection_shifts_a_stored_negative_half_extent() {
        // No jitter: the stored extents are negated into the frustum edges.
        let m = subframe_projection([-10.0, -5.0], [0.0, 0.0], [800, 600], 1.0, 100.0);
        assert_eq!(m[0][0], 0.1);
        assert_eq!(m[1][1], 0.2);
        // One pixel of jitter: dx = (−10·2·1)/800 = −0.025, so the right edge
        // moves from 10 to 9.975 and the left from −10 to −10.025.
        let m = subframe_projection([-10.0, -5.0], [1.0, 1.0], [800, 600], 1.0, 100.0);
        assert_eq!(m[0][0], 2.0 / (9.975 - -10.025));
        assert_eq!(m[2][0], (9.975 + -10.025) / (9.975 - -10.025));
        // dy = (-5*2*1)/600; the frustum's bottom edge is half.y + dy and its
        // top -half.y + dy, so the centre moves with dy.
        let dy = (-5.0 * 2.0 * 1.0) / 600.0;
        let bottom = -5.0 + dy;
        let top = 5.0 + dy;
        assert_eq!(m[2][1], (top + bottom) / (top - bottom));
    }

    #[test]
    fn the_view_transposes_the_camera_and_negates_its_position() {
        let m = view_from_camera(&static_camera(0));
        assert_eq!(m[0], [1.0, 0.0, 0.0, 0.0]);
        assert_eq!(m[1], [0.0, 1.0, 0.0, 0.0]);
        assert_eq!(m[2], [0.0, 0.0, 1.0, 0.0]);
        assert_eq!(m[3], [-1.0, -2.0, -3.0, 1.0]);
        // A 90° rotation about z transposes into the view.
        let camera = [
            [0.0, 1.0, 0.0],
            [-1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 0.0],
        ];
        let m = view_from_camera(&camera);
        assert_eq!(m[0], [0.0, -1.0, 0.0, 0.0]);
        assert_eq!(m[1], [1.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn the_product_sums_in_the_engines_order() {
        let a = [
            [1.0, 2.0, 3.0, 4.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [5.0, 6.0, 7.0, 8.0],
        ];
        let b = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [1.0, 1.0, 1.0, 1.0],
        ];
        let m = multiply(&a, &b);
        assert_eq!(m[0], [5.0, 6.0, 7.0, 4.0]);
        assert_eq!(m[3], [13.0, 14.0, 15.0, 8.0]);
        // The identity leaves both sides alone.
        let identity = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        assert_eq!(multiply(&a, &identity), a);
        assert_eq!(multiply(&identity, &a), a);
    }

    #[test]
    fn a_point_takes_the_translation_row() {
        let m = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [2.0, 3.0, 4.0, 1.0],
        ];
        assert_eq!(transform_point(&m, [1.0, 1.0, 1.0]), [3.0, 4.0, 5.0]);
        let m = [
            [0.0, 1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        assert_eq!(transform_point(&m, [1.0, 0.0, 0.0]), [0.0, 1.0, 0.0]);
    }

    #[test]
    fn a_quarter_builds_a_pair_per_pass() {
        let frames = Subframes::build(3, 0, &view(), static_camera);
        assert_eq!(frames.len(), 4);
        assert_eq!(frames.setting(), 3);
        assert_eq!(frames.quarter(), 0);
        assert_eq!(frames.frame_offset(), 0.375);
        assert_eq!(frames.clock(0), animation_clock(0, 0, 0.1, 4));
        assert_eq!(frames.viewport_shift(2), [0, 1]);
        // The first pass has the 4× setting's first jitter.
        assert_eq!(
            frames.projection(0),
            Some(&subframe_projection(
                [-10.0, -5.0],
                j(6),
                [800, 600],
                1.0,
                100.0,
            ))
        );
        // The composite is the view times the projection.
        let v = view_from_camera(&static_camera(0));
        assert_eq!(
            frames.composite(0),
            Some(&multiply(&v, frames.projection(0).unwrap()))
        );
        // The object pair is the object and the object times the composite.
        let object = [
            [2.0, 0.0, 0.0, 0.0],
            [0.0, 2.0, 0.0, 0.0],
            [0.0, 0.0, 2.0, 0.0],
            [1.0, 2.0, 3.0, 1.0],
        ];
        let pair = frames.object_pair(&object, 0).unwrap();
        assert_eq!(pair[0], object);
        assert_eq!(pair[1], multiply(&object, frames.composite(0).unwrap()));
    }

    #[test]
    fn a_depth_offset_moves_the_projection_lane_and_rebuilds_the_composite() {
        let mut frames = Subframes::build(3, 0, &view(), static_camera);
        let before = *frames.projection(1).unwrap();
        frames.apply_depth_offset(0.1);
        let after = *frames.projection(1).unwrap();
        assert_eq!(after[2][2], depth_lane(before[2][2], 1, 0.1, 4));
        assert_eq!(after[0][0], before[0][0]);
        assert_eq!(
            frames.composite(1),
            Some(&multiply(
                frames.view(1).unwrap(),
                frames.projection(1).unwrap()
            ))
        );
    }

    #[test]
    fn a_three_sample_setting_draws_three_passes() {
        let frames = Subframes::build(2, 0, &view(), static_camera);
        assert_eq!(frames.len(), 3);
        // The third sample's jitter is entry 5 of the table.
        assert_eq!(
            frames.projection(2),
            Some(&subframe_projection(
                [-10.0, -5.0],
                j(5),
                [800, 600],
                1.0,
                100.0
            ))
        );
    }
}
