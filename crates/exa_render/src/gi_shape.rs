//! The manager's emissive light shapes: the original's `TGIEmissive`
//! subclasses inject light into the GI volume through their own fragment
//! programs, drawn over the column box [`crate::gi::shape_draw`] clips.
//!
//! The manager's side is the registry, the shape's box and quad and the
//! per-program uniforms `SliceSz` and `ZTrn`; everything else a body reads
//! (`Colour`, `bMat`, `bExt`, `Radius`, the line and point lists) is the
//! shape's own class's. The bodies below are the original's
//! own `shading.flb` fragment programs (`gi_dome`, `gi_cuboid`, `gi_bubble`,
//! `gi_telebubble`, `gi_gradline`, `gi_multipoint`), evaluated per voxel: a
//! shape's fragment reads only its own uniforms and the voxel's position, so
//! evaluating it per voxel is what its quad-per-slice draw computes.
//!
//! Exarust has no gameplay effect that registers a shape yet (the powers do
//! not draw), so these run as the tested reference the renderer will use
//! when an effect registers one; `GiShapes` keeps the registry live so a
//! shape entity is listed and removed exactly as the original lists it.
//! Rules: `docs/RENDERING.md` (Voxel GI).

use std::sync::Arc;

use bevy::prelude::*;

/// The shader-library program a registered GI shape injects with. The
/// manager links each with `gi_inject_v` and `gi_inject_g`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GiShapeProgram {
    /// `gi_dome`: a sphere shell with a projected dome texture.
    Dome,
    /// `gi_cuboid`: a box with a soft edge falloff.
    Cuboid,
    /// `gi_bubble`: a sphere shell with a bubble texture.
    Bubble,
    /// `gi_telebubble`: the teleporter's pulsing bubble.
    TeleBubble,
    /// `gi_gradline`: a line light with a gradient along it.
    GradLine,
    /// `gi_multipoint`: a run of point lights.
    Multipoint,
}

impl GiShapeProgram {
    /// Every program, in the shader library's order.
    pub const ALL: [Self; 6] = [
        Self::Dome,
        Self::Cuboid,
        Self::Bubble,
        Self::TeleBubble,
        Self::GradLine,
        Self::Multipoint,
    ];

    /// The entry's name in the shader library (the manager links
    /// `gi_inject_v` and `gi_inject_g` with it).
    pub fn name(self) -> &'static str {
        match self {
            Self::Dome => "gi_dome",
            Self::Cuboid => "gi_cuboid",
            Self::Bubble => "gi_bubble",
            Self::TeleBubble => "gi_telebubble",
            Self::GradLine => "gi_gradline",
            Self::Multipoint => "gi_multipoint",
        }
    }

    /// The program a shader-library name names, if the manager knows it.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.name() == name)
    }
}

/// The uniforms a shape's fragment reads, as the shape's own class sets
/// them. Every program declares all of them; the ones its body does not
/// read are ignored, as the shader ignores them.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct GiShape {
    /// Which fragment body the manager links.
    pub program: GiShapeProgram,
    /// `Colour`: the injected colour, before attenuation.
    pub colour: Vec3,
    /// `Atten`: the falloff scale of `gi_gradline` and `gi_multipoint` (the
    /// shell programs declare it and do not read it).
    pub atten: f32,
    /// `bMat`: the shape's frame, applied to the volume position.
    pub matrix: Mat4,
    /// `bExt`: the box's half extents in the shape's space: whatever `bMat`
    /// maps the voxel position into (the bodies' constants, a 100-unit dome
    /// and `d²·0.001` falloffs, are the original's centimetres).
    pub half: Vec3,
    /// `TexPos`: the texture window's origin.
    pub tex_pos: Vec2,
    /// `Radius`: the shell radius of `gi_bubble` and `gi_telebubble`.
    pub radius: f32,
    /// `Fade`: the shell's fade (both bubble programs).
    pub fade: f32,
    /// `Pulse`: `gi_telebubble`'s pulse strength.
    pub pulse: f32,
    /// `Anim`: `gi_telebubble`'s animation phase.
    pub anim: f32,
    /// `Line[2]`: the lit segment's ends (`gi_gradline`).
    pub line: [Vec3; 2],
    /// `Grad`: the segment's split point and its two end weights
    /// (`gi_gradline`).
    pub grad: Vec3,
    /// `PointLgt`: an extra point light (position, weight) for
    /// `gi_gradline`.
    pub point_light: Vec4,
    /// `Light[2i]`, `Light[2i+1]`: each point's position and colour
    /// (`gi_multipoint`).
    pub points: Arc<[(Vec3, Vec3)]>,
}

impl Default for GiShape {
    fn default() -> Self {
        Self {
            program: GiShapeProgram::Cuboid,
            colour: Vec3::ONE,
            atten: 0.0,
            matrix: Mat4::IDENTITY,
            half: Vec3::ZERO,
            tex_pos: Vec2::ZERO,
            radius: 0.0,
            fade: 0.0,
            pulse: 0.0,
            anim: 0.0,
            line: [Vec3::ZERO; 2],
            grad: Vec3::ZERO,
            point_light: Vec4::ZERO,
            points: Arc::new([]),
        }
    }
}

impl GiShape {
    /// A cuboid shape of `half` extents centred at `at`, both in the space
    /// the voxel position is given in, injecting `colour` with the program's
    /// soft falloff. `bMat` takes a voxel position into the shape's space,
    /// so the frame is the translation by `−at`.
    pub fn cuboid(at: Vec3, half: Vec3, colour: Vec3) -> Self {
        Self {
            program: GiShapeProgram::Cuboid,
            colour,
            half,
            matrix: Mat4::from_translation(-at),
            ..default()
        }
    }

    /// The light a shape adds to one voxel, as its fragment program computes
    /// it: `at` is the voxel's position in the volume's texture units (x,
    /// Z depth, and the height over the block's width, the original's
    /// `ZTrn.x` scale), `occupancy` the volume's `OVox` sample there (0..1),
    /// and `texel` the shape's `Tex0` red channel at a `(u, v)`.
    pub fn radiance(&self, at: Vec3, occupancy: f32, texel: impl Fn(Vec2) -> f32) -> GiShapeSample {
        match self.program {
            GiShapeProgram::Dome => self.dome(at, occupancy, &texel),
            GiShapeProgram::Cuboid => self.cuboid_body(at, occupancy),
            GiShapeProgram::Bubble => self.bubble(at, occupancy, &texel),
            GiShapeProgram::TeleBubble => self.telebubble(at, occupancy, &texel),
            GiShapeProgram::GradLine => self.gradline(at),
            GiShapeProgram::Multipoint => self.multipoint(at, occupancy),
        }
    }

    /// The shape's frame applied to a volume position, as every shell body
    /// does (`vxPos = (bMat * vec4(vxPos, 1.0)).xyz`).
    fn framed(&self, at: Vec3) -> Vec3 {
        self.matrix.transform_point3(at)
    }

    /// `gi_dome`: a shell at radius 100 in the shape's space, with the dome
    /// texture's ridge (`1 − |t − 0.4|·2`) cubed, a 70-unit inner cap and a
    /// `1 − d²/1000` shell falloff, times 6.
    fn dome(&self, at: Vec3, occupancy: f32, texel: &impl Fn(Vec2) -> f32) -> GiShapeSample {
        let p = self.framed(at);
        // The body reads the occupancy and never uses it.
        let _ = gl_min(occupancy * 20.0, 1.0);
        // `vxPos.xy * (0.75) * 0.00375`, multiplied left to right.
        let tc = p.xy() * 0.75 * 0.00375 + self.tex_pos;
        let a = texel(tc);
        let a = 1.0 - (a - 0.4).abs() * 2.0;
        let mut l_dst = p.length();
        l_dst = (100.0 - l_dst).abs();
        l_dst *= l_dst;
        let v = p - Vec3::new(0.0, 0.0, 100.0);
        let mut b = gl_max(v.length() - 70.0, 0.0);
        b = gl_max(1.0 - b * 0.02, 0.0);
        let atn = gl_max(1.0 - l_dst * 0.001, 0.0);
        let atn = atn * b;
        let atn = atn * 6.0 * a * a * a;
        GiShapeSample {
            colour: (self.colour * atn).max(Vec3::ZERO),
            alpha: 0.0,
        }
    }

    /// `gi_cuboid`: the distance outside the box (`max(|p| − bExt, 0)`),
    /// falling off as `1 − d²·0.0002`.
    fn cuboid_body(&self, at: Vec3, occupancy: f32) -> GiShapeSample {
        let p = self.framed(at);
        let _ = gl_min(occupancy * 20.0, 1.0);
        let v = (p.abs() - self.half).max(Vec3::ZERO);
        let l_dst = v.dot(v);
        let atn = gl_max(1.0 - l_dst * 0.0002, 0.0);
        GiShapeSample {
            colour: (self.colour * atn).max(Vec3::ZERO),
            alpha: 0.0,
        }
    }

    /// `gi_bubble`: a shell at `Radius` with the bubble texture cubed, the
    /// vertical `Fade` cut and 4× the shell falloff.
    fn bubble(&self, at: Vec3, occupancy: f32, texel: &impl Fn(Vec2) -> f32) -> GiShapeSample {
        let p = self.framed(at);
        let _ = gl_min(occupancy * 20.0, 1.0);
        let tc = Vec2::new(p.x, p.z) * 0.75 * 0.00375 + self.tex_pos;
        let a = texel(tc);
        let a = 1.0 - (a - 0.4).abs() * 2.0;
        let a = a * a * a;
        let b = p.y / p.length();
        let b = (1.5 + b) * self.fade;
        let a = a * gl_clamp(8.0 - b * 16.0, 0.0, 1.0);
        let mut l_dst = p.length();
        l_dst = (self.radius - l_dst).abs();
        l_dst *= l_dst;
        let atn = gl_max(1.0 - l_dst * 0.001, 0.0);
        let atn = atn * 4.0 * a;
        GiShapeSample {
            colour: (self.colour * atn).max(Vec3::ZERO),
            alpha: 0.0,
        }
    }

    /// `gi_telebubble`: the teleporter's pulsing bubble. Its texture is read
    /// on three planes weighted by the squared normal, its pulse ripples
    /// with the vertical normal, and the colour is squared twice at the end.
    fn telebubble(&self, at: Vec3, occupancy: f32, texel: &impl Fn(Vec2) -> f32) -> GiShapeSample {
        let p = self.framed(at);
        let _ = gl_min(occupancy * 20.0, 1.0);
        let l_dst = p.length();
        let nrm = p / l_dst;
        let mut f = nrm.y * 0.5 + 0.5;
        let a = (1.0 - f * 0.5) * 0.007;
        let mut tcz = Vec2::new(p.x, p.y) * a;
        let mut tcy = Vec2::new(p.x, p.z) * a;
        let mut tcx = Vec2::new(p.y, p.z) * a;
        tcx.x += gl_sign(nrm.x) * 0.333;
        tcy.y += gl_sign(nrm.y) * 0.333;
        tcz.x -= gl_sign(nrm.z) * 0.333;
        let tbw = nrm * nrm;
        let tbw = tbw.abs() / (tbw.x + tbw.y + tbw.z);
        let mut i1 = texel(tcz * 0.5 + self.tex_pos) * tbw.z;
        i1 += texel(tcx * 0.5 + self.tex_pos) * tbw.x;
        i1 += texel(tcy * 0.5 + self.tex_pos) * tbw.y;
        let i1 = i1 * i1 * 2.0;
        let a = 1.0 - (f * 2.0 - 1.0).abs();
        let mut a = gl_mix(a, 1.0 - i1, 0.4);
        f = gl_max(self.fade, 0.0);
        a = a.powf(16.0 * f);
        let c = a;
        // The body then squares `1 − |0.5 − c|·2` and overwrites it with the
        // pulse term before reading it, so the squared value never reaches
        // the colour.
        let mut s = i1 * gl_round(c);
        s *= 1.0 + self.fade.abs();
        let mut a = nrm.y.abs() - self.anim;
        a -= gl_round(a);
        let a = a * a * self.pulse * 4.0;
        s += a * i1;
        f = gl_max(f - 0.25, 0.0);
        f = 1.0 + f * 4.0;
        s = (s + a * 0.5) * f;
        let l_dst = (self.radius - l_dst).abs();
        let l_dst = l_dst * l_dst;
        let atn = gl_max(1.0 - l_dst * 0.001, 0.0);
        let out = self.colour * s * 4.0;
        let out = out * out * atn * 4.0;
        GiShapeSample {
            colour: out.max(Vec3::ZERO),
            alpha: 0.0,
        }
    }

    /// `gi_gradline`: a lit segment with a two-ended gradient and one extra
    /// point light, its colour times the gradient times 3 and the light
    /// times 1.
    fn gradline(&self, at: Vec3) -> GiShapeSample {
        let p = at;
        let lv = self.line[1] - self.line[0];
        let sv = p - self.line[0];
        let l = gl_clamp(lv.dot(sv) / lv.dot(lv), 0.0, 1.0);
        let sv = self.line[0] + lv * l;
        let a = gl_max(self.grad.x - l, 0.0);
        let b = gl_max(l - self.grad.x, 0.0);
        let l = a * self.grad.y + b * self.grad.z + (1.0 - a - b);
        let dir = p - sv;
        let d = dir.dot(dir);
        let atn = gl_max(1.0 - d * self.atten, 0.0);
        let atn = atn * atn;
        let mut out = self.colour * atn * l * 3.0;
        let dir = p - self.point_light.truncate();
        let d = dir.dot(dir);
        let atn = gl_max(1.0 - d * self.atten, 0.0);
        let atn = atn * atn;
        out += self.colour * atn * self.point_light.w * l;
        GiShapeSample {
            colour: out.max(Vec3::ZERO),
            alpha: 1.0,
        }
    }

    /// `gi_multipoint`: each point adds its colour times
    /// `max(1 − d²·Atten, 0)²·0.25`. The body computes a term it never
    /// applies (the occlusion cut), as the original's does.
    fn multipoint(&self, at: Vec3, occupancy: f32) -> GiShapeSample {
        let p = at;
        let occ = gl_min(occupancy * 20.0, 1.0);
        let mut out = Vec3::ZERO;
        for &(position, colour) in self.points.iter() {
            let dir = p - position;
            let d = dir.dot(dir);
            let atn = gl_max(1.0 - d * self.atten, 0.0);
            let atn = atn * atn;
            let _sdw = gl_max(2.0 - (0.75 / (atn * atn)), 0.0);
            let _sdw = 1.0 - occ * (1.0 - _sdw);
            out += colour * atn * 0.25;
        }
        GiShapeSample {
            colour: out.max(Vec3::ZERO),
            alpha: 1.0,
        }
    }
}

/// One shape's fragment output: the colour it adds and the alpha it writes
/// (0 for the shells, 1 for the line and multi-point programs).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GiShapeSample {
    pub colour: Vec3,
    pub alpha: f32,
}

/// `max` as the shaders' `max(x, x)`: `x < y ? y : x`, so a not-a-number
/// `x` is kept rather than replaced.
fn gl_max(x: f32, y: f32) -> f32 {
    if x < y { y } else { x }
}

/// `min` as the shaders' `min(x, y)`: `y < x ? y : x`.
fn gl_min(x: f32, y: f32) -> f32 {
    if y < x { y } else { x }
}

/// `clamp` as the shaders' nested `min(max(x, lo), hi)`.
fn gl_clamp(x: f32, lo: f32, hi: f32) -> f32 {
    gl_min(gl_max(x, lo), hi)
}

/// `mix` as the shaders' `x·(1 − a) + y·a`.
fn gl_mix(x: f32, y: f32, a: f32) -> f32 {
    x * (1.0 - a) + y * a
}

/// `sign`, which keeps a not-a-number argument as it is.
fn gl_sign(x: f32) -> f32 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        x * 0.0
    }
}

/// `round`, the nearest integer with a half rounded away from zero (the
/// rounding the shipped drivers' `RND` gives; the spec leaves it
/// implementation-defined). [guess: ties away from zero]
fn gl_round(x: f32) -> f32 {
    x.round()
}

/// The registered shapes, in registration order. The original appends a
/// shape's pointer (storing the manager in the shape), and
/// unregisters by walking the list and swapping its shortened end over every
/// match, so a shape listed twice has both entries removed.
#[derive(Resource, Default, Clone, Debug)]
pub struct GiShapes(Vec<Entity>);

impl GiShapes {
    /// Registers a shape at the end.
    pub fn register(&mut self, shape: Entity) {
        self.0.push(shape);
    }

    /// Unregisters a shape: every entry equal to it is replaced by the
    /// shortened list's last entry and the list loses one entry, as the
    /// original's walk does.
    pub fn unregister(&mut self, shape: Entity) {
        let old = self.0.len();
        let mut list = self.0.clone();
        let mut count = old;
        for i in 0..old {
            if list[i] == shape {
                count -= 1;
                list[i] = list[count];
            }
        }
        list.truncate(count);
        self.0 = list;
    }

    /// The registered shapes, in list order.
    pub fn shapes(&self) -> &[Entity] {
        &self.0
    }
}

/// Lists every new [`GiShape`] and drops every removed one, as the effects'
/// constructors and destructors register and unregister theirs.
fn track_shapes(
    mut shapes: ResMut<GiShapes>,
    added: Query<Entity, Added<GiShape>>,
    mut removed: RemovedComponents<GiShape>,
) {
    for entity in &added {
        shapes.register(entity);
    }
    for entity in removed.read() {
        shapes.unregister(entity);
    }
}

/// The shapes' registry, its tracking systems and the shape programs'
/// shared uniform layout.
pub struct GiShapePlugin;

impl Plugin for GiShapePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GiShapes>()
            .add_systems(PostUpdate, track_shapes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ident(program: GiShapeProgram) -> GiShape {
        GiShape {
            program,
            ..default()
        }
    }

    /// The manager lists a new shape and drops a removed one, including a
    /// shape registered twice.
    #[test]
    fn shapes_register_at_the_end_and_leave_by_swap_removal() {
        let mut world = World::new();
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();
        let c = world.spawn_empty().id();
        let mut shapes = GiShapes::default();
        for e in [a, b, c] {
            shapes.register(e);
        }
        assert_eq!(shapes.shapes(), &[a, b, c]);
        shapes.unregister(b);
        assert_eq!(shapes.shapes(), &[a, c]);
        shapes.register(c);
        shapes.unregister(c);
        assert_eq!(shapes.shapes(), &[a]);
        shapes.unregister(a);
        assert!(shapes.shapes().is_empty());
    }

    /// The tracking systems list entities as they gain a shape and drop
    /// them as they lose it.
    #[test]
    fn a_shape_entity_is_listed_while_it_has_its_component() {
        let mut app = App::new();
        app.add_plugins(GiShapePlugin);
        let e = app.world_mut().spawn(GiShape::default()).id();
        app.update();
        assert_eq!(app.world().resource::<GiShapes>().shapes(), &[e]);
        app.world_mut().entity_mut(e).remove::<GiShape>();
        app.update();
        assert!(app.world().resource::<GiShapes>().shapes().is_empty());
    }

    /// A program is named by its shader-library entry.
    #[test]
    fn a_shape_program_is_named_by_its_shader_library_entry() {
        assert_eq!(GiShapeProgram::Dome.name(), "gi_dome");
        assert_eq!(GiShapeProgram::Multipoint.name(), "gi_multipoint");
        for program in GiShapeProgram::ALL {
            assert_eq!(GiShapeProgram::from_name(program.name()), Some(program));
        }
        assert_eq!(GiShapeProgram::from_name("gi_inject_f"), None);
    }

    /// `gi_cuboid`: inside the box the light is the colour; outside it falls
    /// off as `1 − d²·0.0002`.
    #[test]
    fn the_cuboid_falls_off_outside_its_box() {
        let shape = GiShape {
            colour: Vec3::new(1.0, 2.0, 3.0),
            half: Vec3::splat(1.0),
            ..ident(GiShapeProgram::Cuboid)
        };
        let cube = |at: Vec3| shape.radiance(at, 0.0, |_| 0.0).colour;
        assert_eq!(cube(Vec3::ZERO), Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(cube(Vec3::new(1.0, 0.0, 0.0)), Vec3::new(1.0, 2.0, 3.0));
        // Half a voxel out: d² = 0.25, 1 − 0.00005.
        assert_eq!(
            cube(Vec3::new(1.5, 0.0, 0.0)),
            Vec3::new(1.0, 2.0, 3.0) * 0.999_95
        );
        // Three axes 0.5 out: d² = 0.75.
        assert_eq!(
            cube(Vec3::splat(1.5)),
            Vec3::new(1.0, 2.0, 3.0) * (1.0 - 0.75 * 0.0002)
        );
        // Far outside the falloff the light is gone.
        assert_eq!(cube(Vec3::splat(100.0)), Vec3::ZERO);
        assert_eq!(shape.radiance(Vec3::ZERO, 0.0, |_| 0.0).alpha, 0.0);
        // The frame moves the box.
        let moved = GiShape {
            matrix: Mat4::from_translation(Vec3::new(5.0, 0.0, 0.0)),
            ..shape.clone()
        };
        assert_eq!(
            moved
                .radiance(Vec3::new(-5.0, 0.0, 0.0), 0.0, |_| 0.0)
                .colour,
            Vec3::new(1.0, 2.0, 3.0)
        );
        assert_eq!(
            moved.radiance(Vec3::ZERO, 0.0, |_| 0.0).colour,
            Vec3::new(1.0, 2.0, 3.0) * (1.0 - 16.0 * 0.0002)
        );
        // The constructor centres the box on its point: the frame is the
        // translation by minus it.
        let placed = GiShape::cuboid(Vec3::new(5.0, 0.0, 0.0), Vec3::ONE, Vec3::ONE);
        assert_eq!(
            placed
                .radiance(Vec3::new(5.0, 0.0, 0.0), 0.0, |_| 0.0)
                .colour,
            Vec3::ONE
        );
        assert_eq!(
            placed
                .radiance(Vec3::new(5.0, 0.0, 0.0), 0.0, |_| 0.0)
                .colour,
            placed
                .radiance(Vec3::new(6.0, 1.0, 1.0), 0.0, |_| 0.0)
                .colour
        );
    }

    /// `gi_dome`: at the dome's centre the ridge texture is cubed and the
    /// shell weight 6 applies; the 70-unit cap cuts the near edge.
    #[test]
    fn the_dome_shell_weights_its_texture_and_cap() {
        let shape = GiShape {
            colour: Vec3::splat(0.5),
            ..ident(GiShapeProgram::Dome)
        };
        // The shell is the sphere of radius 100 about the origin: its top
        // point (0, 0, 100) is on it and inside the 70-unit cap, the texture's
        // ridge at 0.4 gives a = 1, so the colour is 0.5·6 = 3.
        let on = Vec3::new(0.0, 0.0, 100.0);
        assert_eq!(shape.radiance(on, 0.0, |_| 0.4).colour, Vec3::splat(3.0));
        // A ridge at 0.5 gives a = 1 − 0.2 = 0.8 (the 0.2 is 0.1·2 in
        // single precision) and 0.8³ = 0.512.
        let ridge = shape.radiance(on, 0.0, |_| 0.5).colour;
        assert!((ridge - Vec3::splat(3.0 * 0.512)).abs().max_element() < 1e-6);
        // 1 unit under it the shell term is 1 − 0.001 and the cap still 1.
        let inside = Vec3::new(0.0, 0.0, 99.0);
        assert_eq!(
            shape.radiance(inside, 0.0, |_| 0.4).colour,
            Vec3::splat(3.0 * 0.999)
        );
        // At the origin the shell is 100 units away: 1 − 10 000·0.001 < 0.
        assert_eq!(shape.radiance(Vec3::ZERO, 0.0, |_| 0.4).colour, Vec3::ZERO);
        // A point on the shell 141 units from the cap's centre is outside
        // the 70-unit cap: v = p − (0, 0, 100) = (100, 0, −100).
        let side = Vec3::new(100.0, 0.0, 0.0);
        assert_eq!(shape.radiance(side, 0.0, |_| 0.4).colour, Vec3::ZERO);
    }

    /// `gi_bubble`: the shell at `Radius` with the vertical fade.
    #[test]
    fn the_bubble_falls_off_from_its_radius() {
        let shape = GiShape {
            colour: Vec3::splat(0.25),
            radius: 8.0,
            fade: 0.0,
            ..ident(GiShapeProgram::Bubble)
        };
        // On the shell, level with its centre: the fade cut is 1.
        let on = Vec3::new(8.0, 0.0, 0.0);
        assert_eq!(
            shape.radiance(on, 0.0, |_| 0.4).colour,
            Vec3::splat(0.25 * 4.0)
        );
        // 1 unit inside: 1 − 1·0.001.
        let inside = Vec3::new(7.0, 0.0, 0.0);
        assert_eq!(
            shape.radiance(inside, 0.0, |_| 0.4).colour,
            Vec3::splat(0.25 * 4.0 * 0.999)
        );
        // Above the centre the fade cuts in: b = 1.5 + 1 = 2.5 at fade 0.5,
        // 8 − 2.5·16 = −32 → clamped to 0.
        let up = Vec3::new(0.0, 8.0, 0.0);
        let faded = GiShape {
            fade: 0.5,
            ..shape.clone()
        };
        assert_eq!(faded.radiance(up, 0.0, |_| 0.4).colour, Vec3::ZERO);
    }

    /// `gi_gradline`: the segment's gradient, the 3× line term and the
    /// point light's own term.
    #[test]
    fn the_gradline_weights_its_segment_and_point() {
        let shape = GiShape {
            colour: Vec3::splat(0.1),
            atten: 0.01,
            line: [Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0)],
            grad: Vec3::new(0.5, 2.0, 3.0),
            point_light: Vec4::new(0.0, 0.0, 0.0, 0.5),
            ..ident(GiShapeProgram::GradLine)
        };
        // The middle of the segment: l = 0.5, so the gradient picks 1.
        let mid = Vec3::new(5.0, 0.0, 0.0);
        let out = shape.radiance(mid, 0.0, |_| 0.0);
        // Line: 0.1 · 1 · 1 · 3 = 0.3; point: d² = 25,
        // (1 − 0.25)² = 0.5625, × 0.5 × 1 → 0.028125.
        assert!((out.colour.x - (0.3 + 0.028_125)).abs() < 1e-6);
        assert_eq!(out.alpha, 1.0);
        // Past the segment's end the gradient's second weight applies:
        // l = 1, a = 0, b = 1 − grad.x = 0.5, and the leftover 1 − a − b
        // adds another 0.5, so l = 2. The point is 20 m out, past
        // 1/Atten = 100 m², so its term is 0.
        // At the end point the point light is 100 m² away, past its reach.
        let end = Vec3::new(10.0, 0.0, 0.0);
        let out = shape.radiance(end, 0.0, |_| 0.0);
        assert!((out.colour.x - 0.1 * 2.0 * 3.0).abs() < 1e-6);
    }

    /// `gi_multipoint`: each point adds its colour times the squared
    /// falloff times a quarter; the occlusion term is computed and left
    /// unused, as the original does.
    #[test]
    fn the_multipoint_adds_its_points_at_a_quarter() {
        let shape = GiShape {
            atten: 0.001,
            points: Arc::new([
                (Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0)),
                (Vec3::new(10.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0)),
            ]),
            ..ident(GiShapeProgram::Multipoint)
        };
        let out = shape.radiance(Vec3::ZERO, 0.0, |_| 0.0);
        // d² = 0 → 1 · 0.25; d² = 100 → (1 − 0.1)² = 0.81 · 0.25.
        assert!((out.colour.x - 0.25).abs() < 1e-6);
        assert!((out.colour.y - 0.2025).abs() < 1e-6);
        assert_eq!(out.colour.z, 0.0);
        assert_eq!(out.alpha, 1.0);
        // Fully occluded the light is unchanged: the body never applies the
        // occlusion cut.
        assert_eq!(shape.radiance(Vec3::ZERO, 1.0, |_| 0.0), out);
    }

    /// `gi_telebubble` at its shell, level with the centre: the texture is
    /// read on one plane, its squared sum doubles and the colour is squared
    /// twice.
    #[test]
    fn the_telebubble_pulses_at_its_shell() {
        let shape = GiShape {
            colour: Vec3::splat(0.5),
            radius: 4.0,
            fade: 0.0,
            pulse: 0.0,
            anim: 0.0,
            ..ident(GiShapeProgram::TeleBubble)
        };
        // On +x: nrm = (1, 0, 0), f = 0.5, a = 0.00525, the only textured
        // plane is x (tbw = (1, 0, 0)), at (0.1665, 0) + TexPos.
        let on = Vec3::new(4.0, 0.0, 0.0);
        let samples = std::cell::RefCell::new(Vec::new());
        let out = shape.radiance(on, 0.0, |uv| {
            samples.borrow_mut().push(uv);
            0.5
        });
        // The three planes, in the body's order: z, x (the normal's own
        // axis, shifted by +0.333), y.
        let got = samples.borrow();
        let want = [
            Vec2::new(0.0105, 0.0),
            Vec2::new(0.1665, 0.0),
            Vec2::new(0.0105, 0.0),
        ];
        for (a, b) in got.iter().zip(want) {
            assert!((*a - b).abs().max_element() < 1e-7, "{a} != {b}");
        }
        // i1 = 0.5²·2 = 0.5; a = mix(1, 0.5, 0.4) = 0.8, ^0 = 1, c = 1,
        // a = 1 − |0.5 − 1|·2 = 0, s = 0.5·round(1) = 0.5, f = 1,
        // out = 0.5·0.5·4 = 1, squared × 4 = 4.
        assert_eq!(out.colour, Vec3::splat(4.0));
        assert_eq!(out.alpha, 0.0);
        // The texture's own window moves with TexPos.
        let shifted = GiShape {
            tex_pos: Vec2::new(0.25, 0.5),
            ..shape.clone()
        };
        drop(got);
        samples.borrow_mut().clear();
        shifted.radiance(on, 0.0, |uv| {
            samples.borrow_mut().push(uv);
            0.5
        });
        assert!(
            (samples.borrow()[1] - Vec2::new(0.4165, 0.5))
                .abs()
                .max_element()
                < 1e-7
        );
    }
}
