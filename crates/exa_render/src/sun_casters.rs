//! The sun shadow map's caster pass: which nodes and parts draw into the map,
//! in which order, and with which state.
//!
//! The original's render routine walks the scene's shadow-caster node list in
//! order. A node without the caster flag is skipped, so only registered
//! casters are ever drawn. A flagged node first clears its map-visible bit.
//! The list's head is drawn without the volume test and without moving the
//! volume — the original treats it as the level's own node — and every later
//! node is tested against the caster volume: a node whose box lies wholly
//! beyond one of the volume's planes is skipped (its bit stays clear), and a
//! kept node's stored vector then pushes the volume's faces in place
//! ([`ShadowVolume::push_faces`]), for its own parts and every node after
//! it. A kept node (the head included) sets its bit. The node's parts are
//! tested the same way: a part without the caster flag is left alone (its
//! map-visible bit keeps whatever it had), a part outside the volume has its
//! visible bit cleared and draws nothing, and a part inside sets the bit and
//! draws its meshes.
//!
//! A drawn part culls the faces its own flag asks for: front faces for a
//! flagged part, back faces otherwise, through the cached face switch the
//! cube pass also uses ([`crate::shadow_map::CasterCull`]; the pass starts
//! with back culling and the flag cleared, and ends with back culling). The
//! switch is made once per part, before its meshes. Each of the part's
//! meshes with its draw flag set then draws: plainly, or, when the mesh is
//! marked alpha-cut, through the alpha-tested program with the mesh's
//! texture bound and culling disabled (both sides cast), after which the
//! plain program and culling are restored.
//!
//! The pass sets the depth function to less, enables the polygon offset with
//! the fit's [`crate::sun_fit::CasterBias`] and disables it again at the end.
//! The alpha-tested program is `if (texture(Tex0, vrTex).w < 0.5) discard;`
//! over the part's cut texture, drawn with culling disabled (the pass turns
//! `GL_CULL_FACE` off around the part and back on after it).
//!
//! Bevy's shadow pass draws Exarust's sun map, and the walk above is not
//! what drives it (Bevy culls casters by the map's frustum). What reaches
//! Bevy's pass is the per-draw state ([`specialize_sun_caster`]): every
//! caster's fragment takes the polygon offset (`sun_caster.wesl`, the factor
//! and units from [`crate::sun_fit::SunShadowFilter`]), plain casters cull
//! their back faces, and alpha-tested ones cull nothing and cut by their
//! texture's alpha alone. A caster's draw does not discard otherwise.
//!
//! Everything is in the engine's centimetres. Rules: `docs/RENDERING.md`
//! (Sun shadow fit, the caster pass).

use bevy::math::Vec3;
use exa_physics::intersect::{ConvexVolume, Plane};

use crate::shadow_map::{CasterCull, CullFace};

/// An oriented box as the scene stores one: a centre, the bounding sphere's
/// radius and three axes with their half lengths, cm. A box of the
/// original's sort is what the volume test reads: the centre and radius for
/// its early-out, the axes for the exact support.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrientedBox {
    pub centre: Vec3,
    /// The bounding sphere's radius.
    pub radius: f32,
    /// The three axes, unit length.
    pub axes: [Vec3; 3],
    /// The half length along each axis.
    pub half: Vec3,
}

impl OrientedBox {
    /// The axis-aligned box `min..max`.
    pub fn from_min_max(min: Vec3, max: Vec3) -> Self {
        let half = (max - min) * 0.5;
        Self {
            centre: (min + max) * 0.5,
            radius: half.length(),
            axes: [Vec3::X, Vec3::Y, Vec3::Z],
            half,
        }
    }

    /// How far the box reaches along `normal`: `Σ |normal · axis_i| ·
    /// half_i`, the sum in the original's order.
    pub fn support(&self, normal: Vec3) -> f32 {
        (normal.dot(self.axes[0]).abs() * self.half.x
            + normal.dot(self.axes[1]).abs() * self.half.y)
            + normal.dot(self.axes[2]).abs() * self.half.z
    }
}

/// The caster volume: the visible region as a convex volume whose faces keep
/// the inside where `normal · p + offset ≤ 0`, as the engine's volumes do.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ShadowVolume {
    pub volume: ConvexVolume,
}

impl ShadowVolume {
    /// A volume of these faces.
    pub fn new(planes: Vec<Plane>) -> Self {
        Self {
            volume: ConvexVolume { planes },
        }
    }

    /// The axis-aligned box `min..max`, cm.
    pub fn from_min_max(min: Vec3, max: Vec3) -> Self {
        let mut planes = Vec::with_capacity(6);
        for axis in 0..3 {
            let mut normal = Vec3::ZERO;
            normal[axis] = 1.0;
            planes.push(Plane::new(-normal, min[axis]));
            planes.push(Plane::new(normal, -max[axis]));
        }
        Self::new(planes)
    }

    /// Pushes every face by `by`'s projection on it: `offset += by ·
    /// normal`, which moves the face at `normal · p + offset = 0` by `−by`
    /// along its normal. The render routine does this to the volume for
    /// every kept caster, with the vector stored with the caster. [guess:
    /// what that stored vector is; the arithmetic is the callee's]
    pub fn push_faces(&mut self, by: Vec3) {
        for plane in &mut self.volume.planes {
            plane.offset += by.dot(plane.normal);
        }
    }

    /// Whether `box` may reach into the volume, as the original tests a
    /// caster: for each face, `f = normal · centre + offset`;
    /// * `f` beyond the box's radius leaves the box wholly outside;
    /// * with `f` positive, a box whose support along the normal is still
    ///   short of `f` is wholly outside too.
    ///
    /// A NaN in either comparison counts as inside, as the original's
    /// unordered `comiss` does.
    pub fn reaches(&self, box3: &OrientedBox) -> bool {
        for plane in &self.volume.planes {
            let f = plane.distance(box3.centre);
            if box3.radius < f {
                return false;
            }
            if 0.0 < f && box3.support(plane.normal) < f {
                return false;
            }
        }
        true
    }
}

/// One mesh of a caster part, with the two flags the pass reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CasterMesh {
    /// The mesh draws at all; a clear flag skips it.
    pub drawn: bool,
    /// The mesh cuts its shadow by its texture's alpha: it draws through the
    /// alpha-tested program with that texture bound.
    pub alpha_cut: bool,
}

/// One part of a caster node: its box, its flags and its meshes.
#[derive(Debug, Clone, PartialEq)]
pub struct CasterPart {
    /// The part's caster flag: only flagged parts are tested and drawn.
    pub casts: bool,
    pub box3: OrientedBox,
    /// The part culls front faces instead of back ones.
    pub front_cull: bool,
    pub meshes: Vec<CasterMesh>,
}

/// One node of the scene's caster list.
#[derive(Debug, Clone, PartialEq)]
pub struct CasterNode {
    /// The node's caster flag; the list holds casters, so this is normally
    /// set, and a clear flag skips the node entirely.
    pub casts: bool,
    pub box3: OrientedBox,
    /// The vector the kept node pushes the volume's faces by
    /// ([`ShadowVolume::push_faces`]).
    pub offset: Vec3,
    pub parts: Vec<CasterPart>,
}

/// One mesh draw the pass issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CasterDraw {
    /// The node in the list.
    pub node: usize,
    /// The part within the node.
    pub part: usize,
    /// The mesh within the part.
    pub mesh: usize,
    /// The face switch made for this part before its meshes, on the part's
    /// first draw only; `None` when the cached face already fitted.
    pub cull: Option<CullFace>,
    /// The draw goes through the alpha-tested program with the mesh's
    /// texture bound.
    pub alpha_cut: bool,
}

/// What one run of the caster pass did.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CasterPass {
    pub draws: Vec<CasterDraw>,
    /// Per inside part, the face switch made before its meshes (`None` when
    /// the cache already fitted), in walk order, also for a part whose
    /// meshes all skip.
    pub part_culls: Vec<(usize, usize, Option<CullFace>)>,
    /// Per node: the map-visible bit the pass left (clear for a flagged node
    /// that failed the test; an unflagged node's bit is left, false here).
    pub node_visible: Vec<bool>,
    /// Per node, per part: the map-visible bit the pass left.
    pub visible: Vec<Vec<bool>>,
    /// The cached cull face at the end of the walk.
    pub cull: CasterCull,
}

impl CasterPass {
    /// The number of parts the pass marked visible.
    pub fn visible_parts(&self) -> usize {
        self.visible.iter().flatten().filter(|v| **v).count()
    }
}

/// The caster pass's walk. `volume` is the caster volume (the visible region
/// extruded towards the sun); it is moved by each kept node's offset, as the
/// original does, so pass a scratch copy.
pub fn caster_draws(nodes: &[CasterNode], volume: &mut ShadowVolume) -> CasterPass {
    let mut pass = CasterPass {
        visible: nodes.iter().map(|n| vec![false; n.parts.len()]).collect(),
        node_visible: vec![false; nodes.len()],
        ..Default::default()
    };
    let mut cull = CasterCull::default();
    cull.begin_pass();
    for (index, node) in nodes.iter().enumerate() {
        if !node.casts {
            continue;
        }
        pass.node_visible[index] = false;
        // The list's head is drawn without the test and without the push, as
        // the original's first node is the level itself.
        if index > 0 {
            if !volume.reaches(&node.box3) {
                continue;
            }
            // A kept node pushes the volume in place, for its own parts and
            // every node after it.
            volume.push_faces(node.offset);
        }
        pass.node_visible[index] = true;
        for (part_index, part) in node.parts.iter().enumerate() {
            if !part.casts {
                continue;
            }
            if !volume.reaches(&part.box3) {
                pass.visible[index][part_index] = false;
                continue;
            }
            pass.visible[index][part_index] = true;
            let mut switch = cull.set(part.front_cull);
            pass.part_culls.push((index, part_index, switch));
            for (mesh_index, mesh) in part.meshes.iter().enumerate() {
                if !mesh.drawn {
                    continue;
                }
                pass.draws.push(CasterDraw {
                    node: index,
                    part: part_index,
                    mesh: mesh_index,
                    cull: switch.take(),
                    alpha_cut: mesh.alpha_cut,
                });
            }
        }
    }
    cull.end_pass();
    pass.cull = cull;
    pass
}

/// The faces a sun caster culls in Bevy's shadow pass: none for an
/// alpha-tested caster (the original disables culling around it); otherwise
/// the back faces, as the original's plain caster does. Exarust keeps two
/// exceptions of its own: a cloth draw culls the face it is the draw of (the
/// material's two passes together draw both), and a two-sided material culls
/// nothing, as Exarust's cube casters do \[the original's plain draw
/// does not read the material's two-sided bit; whether its drawing state
/// does was not traced].
pub fn sun_caster_cull(
    alpha_cut: bool,
    two_sided: bool,
    cloth: Option<bevy::render::render_resource::Face>,
) -> Option<bevy::render::render_resource::Face> {
    if let Some(face) = cloth {
        return Some(face);
    }
    if alpha_cut || two_sided {
        return None;
    }
    Some(bevy::render::render_resource::Face::Back)
}

/// The fragment shader a sun caster without a prepass shader of its own
/// takes ([`CasterShader::Plain`]), or the surface material's
/// ([`CasterShader::Material`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CasterShader {
    /// `sun_caster_prepass.wesl`: the polygon offset only.
    Plain,
    /// `exa_prepass.wesl`: the offset and the alpha test.
    Material,
}

static CASTER_SHADERS: std::sync::OnceLock<[bevy::asset::Handle<bevy::shader::Shader>; 2]> =
    std::sync::OnceLock::new();

/// Loads the two caster shaders once, for [`specialize_sun_caster`]. Without
/// an asset server (headless tests) the casters keep Bevy's state.
pub(crate) fn load_caster_shaders(app: &bevy::app::App) {
    use bevy::asset::{AssetPath, AssetServer, embedded_path};
    let Some(server) = app.world().get_resource::<AssetServer>() else {
        return;
    };
    let path = |p: std::path::PathBuf| AssetPath::from_path_buf(p).with_source("embedded");
    let plain = server.load(path(embedded_path!("sun_caster_prepass.wesl")));
    let material = server.load(path(embedded_path!("exa_prepass.wesl")));
    let _ = CASTER_SHADERS.set([plain, material]);
}

/// The shader reference of [`CasterShader::Plain`], for a material's
/// `prepass_fragment_shader`.
pub fn plain_caster_shader() -> bevy::shader::ShaderRef {
    use bevy::asset::{AssetPath, embedded_path};
    bevy::shader::ShaderRef::Path(
        AssetPath::from_path_buf(embedded_path!("sun_caster_prepass.wesl")).with_source("embedded"),
    )
}

/// Whether `mesh_key` is a draw into a directional light's map: Bevy's
/// shadow pass marks those depth-only, orthographic and unclipped.
pub fn is_sun_caster(mesh_key: bevy::pbr::MeshPipelineKey) -> bool {
    use bevy::pbr::MeshPipelineKey as K;
    mesh_key.contains(K::DEPTH_PREPASS | K::UNCLIPPED_DEPTH_ORTHO)
}

/// Gives a draw into the sun map the original's caster state: a fragment
/// stage with the polygon offset (`shader`'s, added when the draw had none,
/// with `SUN_CASTER`, and `SUN_CUT` for an alpha-tested caster) and the
/// faces [`sun_caster_cull`] picks. Other pipelines are left alone; returns
/// whether this was a sun caster.
pub fn specialize_sun_caster(
    descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
    mesh_key: bevy::pbr::MeshPipelineKey,
    shader: CasterShader,
    cull: Option<bevy::render::render_resource::Face>,
    alpha_cut: bool,
) -> bool {
    if !is_sun_caster(mesh_key) {
        return false;
    }
    descriptor.primitive.cull_mode = cull;
    let fragment = match descriptor.fragment.as_mut() {
        Some(fragment) => fragment,
        None => {
            let Some(shaders) = CASTER_SHADERS.get() else {
                return true;
            };
            let handle = match shader {
                CasterShader::Plain => shaders[0].clone(),
                CasterShader::Material => shaders[1].clone(),
            };
            descriptor
                .fragment
                .insert(bevy::render::render_resource::FragmentState {
                    shader: handle,
                    shader_defs: descriptor.vertex.shader_defs.clone(),
                    entry_point: None,
                    targets: Vec::new(),
                    constants: Vec::new(),
                })
        }
    };
    fragment.shader_defs.push("SUN_CASTER".into());
    if alpha_cut {
        fragment.shader_defs.push("SUN_CUT".into());
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn part(min: Vec3, max: Vec3) -> CasterPart {
        CasterPart {
            casts: true,
            box3: OrientedBox::from_min_max(min, max),
            front_cull: false,
            meshes: vec![CasterMesh {
                drawn: true,
                alpha_cut: false,
            }],
        }
    }

    fn node(min: Vec3, max: Vec3, parts: Vec<CasterPart>) -> CasterNode {
        CasterNode {
            casts: true,
            box3: OrientedBox::from_min_max(min, max),
            offset: Vec3::ZERO,
            parts,
        }
    }

    fn v(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3::new(x, y, z)
    }

    #[test]
    fn the_box_support_is_the_sum_of_its_axes() {
        let b = OrientedBox::from_min_max(v(-100.0, -200.0, -300.0), v(100.0, 200.0, 300.0));
        assert_eq!(b.centre, Vec3::ZERO);
        assert!((b.radius - (100f32 * 100.0 + 200.0 * 200.0 + 300.0 * 300.0).sqrt()).abs() < 1e-4);
        assert_eq!(b.support(Vec3::X), 100.0);
        assert_eq!(b.support(Vec3::Y), 200.0);
        // A diagonal normal: the sum of the three projections.
        let n = v(1.0, 1.0, 1.0).normalize();
        assert!((b.support(n) - (100.0 + 200.0 + 300.0) / 3f32.sqrt()).abs() < 1e-4);
        // A turned box projects onto its own axes.
        let turned = OrientedBox {
            centre: Vec3::ZERO,
            radius: 100.0,
            axes: [v(0.0, 1.0, 0.0), v(1.0, 0.0, 0.0), Vec3::Z],
            half: v(10.0, 20.0, 30.0),
        };
        assert_eq!(turned.support(Vec3::X), 20.0);
        assert_eq!(turned.support(Vec3::Y), 10.0);
        assert_eq!(turned.support(Vec3::Z), 30.0);
    }

    #[test]
    fn a_box_is_outside_only_beyond_a_whole_face() {
        let volume = ShadowVolume::from_min_max(v(-100.0, -100.0, -100.0), v(100.0, 100.0, 100.0));
        // Inside, straddling a face and touching a face all reach.
        assert!(volume.reaches(&OrientedBox::from_min_max(
            v(-1.0, -1.0, -1.0),
            v(1.0, 1.0, 1.0)
        )));
        assert!(volume.reaches(&OrientedBox::from_min_max(
            v(50.0, -10.0, -10.0),
            v(150.0, 10.0, 10.0)
        )));
        assert!(volume.reaches(&OrientedBox::from_min_max(
            v(100.0, -10.0, -10.0),
            v(120.0, 10.0, 10.0)
        )));
        // Wholly past +X: the centre is 210 cm out, farther than the 10 cm
        // box is deep, and farther than the box's radius.
        assert!(!volume.reaches(&OrientedBox::from_min_max(
            v(200.0, -10.0, -10.0),
            v(220.0, 10.0, 10.0)
        )));
        // A box whose radius reaches the face but whose depth along the
        // normal does not is still outside: 111 cm out, 10 cm deep, 12 cm
        // radius (a corner).
        let far = OrientedBox::from_min_max(v(200.0, 80.0, 80.0), v(220.0, 120.0, 120.0));
        assert!(!volume.reaches(&far));
        // Exactly on the face: kept (the compare is strict).
        let on = OrientedBox::from_min_max(v(100.0, -10.0, -10.0), v(120.0, 10.0, 10.0));
        assert!(volume.reaches(&on));
        // A NaN centre never leaves the volume (unordered compares keep it).
        let mut nan = OrientedBox::from_min_max(v(0.0, 0.0, 0.0), v(1.0, 1.0, 1.0));
        nan.centre.x = f32::NAN;
        assert!(volume.reaches(&nan));
    }

    #[test]
    fn a_pushed_volume_moves_against_the_vector() {
        let volume = ShadowVolume::from_min_max(v(-10.0, -10.0, -10.0), v(10.0, 10.0, 10.0));
        let mut moved = volume.clone();
        // Every face's offset grows by the vector's projection, so the
        // volume moves by −100 cm on x: its faces stand at −110 and −90.
        moved.push_faces(v(100.0, 0.0, 0.0));
        assert!(moved.reaches(&OrientedBox::from_min_max(
            v(-100.0, -1.0, -1.0),
            v(-95.0, 1.0, 1.0)
        )));
        assert!(!moved.reaches(&OrientedBox::from_min_max(
            v(0.0, -1.0, -1.0),
            v(5.0, 1.0, 1.0)
        )));
        // Pushing by zero changes nothing.
        let mut same = volume.clone();
        same.push_faces(Vec3::ZERO);
        assert_eq!(same, volume);
    }

    #[test]
    fn only_the_lists_head_skips_the_volume_test() {
        // A small volume at the origin; a far node is outside it.
        let volume = ShadowVolume::from_min_max(v(-10.0, -10.0, -10.0), v(10.0, 10.0, 10.0));
        let far_part = part(v(1000.0, 0.0, 0.0), v(1010.0, 10.0, 10.0));
        // As the head it is kept none the less: the parts are still tested.
        let nodes = vec![
            node(
                v(1000.0, 0.0, 0.0),
                v(1010.0, 10.0, 10.0),
                vec![far_part.clone()],
            ),
            node(v(1000.0, 0.0, 0.0), v(1010.0, 10.0, 10.0), vec![far_part]),
        ];
        let pass = caster_draws(&nodes, &mut volume.clone());
        // The head's parts are outside the volume, so nothing visible; the
        // second node is outside it and skipped with its bit cleared.
        assert_eq!(pass.draws.len(), 0);
        assert_eq!(pass.visible_parts(), 0);
        assert_eq!(pass.node_visible, vec![true, false]);
        // With a part inside, the head draws it and the second node draws
        // too (its box reaches the volume).
        let inside = part(v(-1.0, -1.0, -1.0), v(1.0, 1.0, 1.0));
        let nodes = vec![
            node(v(-1.0, -1.0, -1.0), v(1.0, 1.0, 1.0), vec![inside.clone()]),
            node(v(-1.0, -1.0, -1.0), v(1.0, 1.0, 1.0), vec![inside]),
        ];
        let pass = caster_draws(&nodes, &mut volume.clone());
        assert_eq!(pass.draws.len(), 2);
        assert_eq!(pass.visible_parts(), 2);
        assert_eq!(pass.node_visible, vec![true, true]);
    }

    #[test]
    fn an_unflagged_node_or_part_is_left_alone() {
        let volume = ShadowVolume::from_min_max(v(-100.0, -100.0, -100.0), v(100.0, 100.0, 100.0));
        let mut no_caster = node(
            v(0.0, 0.0, 0.0),
            v(1.0, 1.0, 1.0),
            vec![part(v(0.0, 0.0, 0.0), v(1.0, 1.0, 1.0))],
        );
        no_caster.casts = false;
        // A part with a clear caster flag keeps its visible bit (false here)
        // and draws nothing.
        let mut quiet = part(v(0.0, 0.0, 0.0), v(1.0, 1.0, 1.0));
        quiet.casts = false;
        let nodes = vec![
            no_caster,
            node(v(0.0, 0.0, 0.0), v(1.0, 1.0, 1.0), vec![quiet]),
        ];
        let pass = caster_draws(&nodes, &mut volume.clone());
        assert!(pass.draws.is_empty());
        assert_eq!(pass.visible, vec![vec![false], vec![false]]);
        assert_eq!(pass.node_visible, vec![false, true]);
    }

    #[test]
    fn a_kept_node_after_the_head_moves_the_volume() {
        // The caster volume is a small box at the origin and every part sits
        // 500 cm along +X.
        let volume = ShadowVolume::from_min_max(v(-1.0, -1.0, -1.0), v(1.0, 1.0, 1.0));
        let near_part = part(v(-1.0, -1.0, -1.0), v(1.0, 1.0, 1.0));
        let far_part = part(v(499.0, -1.0, -1.0), v(501.0, 1.0, 1.0));
        // The head's vector is never applied: with −500 cm on the head the
        // volume stays at the origin, so the head's far part is outside.
        let mut head = node(
            v(499.0, -1.0, -1.0),
            v(501.0, 1.0, 1.0),
            vec![far_part.clone()],
        );
        head.offset = v(-500.0, 0.0, 0.0);
        let mut moved = volume.clone();
        let pass = caster_draws(&[head], &mut moved);
        assert!(pass.draws.is_empty());
        assert_eq!(moved, volume);
        // A second node at the origin with −500 cm is kept, pushes the volume
        // onto +500 cm, and its own far part then draws while its near part
        // is outside; the node after it is tested against the moved volume.
        let mut second = node(
            v(-1.0, -1.0, -1.0),
            v(1.0, 1.0, 1.0),
            vec![near_part.clone(), far_part.clone()],
        );
        second.offset = v(-500.0, 0.0, 0.0);
        let nodes = vec![
            node(v(-1.0, -1.0, -1.0), v(1.0, 1.0, 1.0), vec![]),
            second,
            node(v(499.0, -1.0, -1.0), v(501.0, 1.0, 1.0), vec![far_part]),
        ];
        let mut moved = volume.clone();
        let pass = caster_draws(&nodes, &mut moved);
        assert_eq!(pass.visible, vec![vec![], vec![false, true], vec![true]]);
        assert_eq!(pass.draws.len(), 2);
        assert_eq!(pass.node_visible, vec![true, true, true]);
        // The volume was pushed in place: its +X face now stands at 501.
        let face = moved
            .volume
            .planes
            .iter()
            .find(|p| p.normal == Vec3::X)
            .unwrap();
        assert_eq!(face.offset, -501.0);
    }

    #[test]
    fn the_face_switch_follows_each_parts_flag() {
        let mut volume =
            ShadowVolume::from_min_max(v(-100.0, -100.0, -100.0), v(100.0, 100.0, 100.0));
        let back = part(v(-1.0, -1.0, -1.0), v(1.0, 1.0, 1.0));
        let mut front = back.clone();
        front.front_cull = true;
        // Back, back, front, back: only the changes are reported.
        let nodes = vec![node(
            v(-1.0, -1.0, -1.0),
            v(1.0, 1.0, 1.0),
            vec![back.clone(), back.clone(), front, back],
        )];
        let pass = caster_draws(&nodes, &mut volume);
        let culls: Vec<_> = pass.draws.iter().map(|d| d.cull).collect();
        assert_eq!(
            culls,
            vec![None, None, Some(CullFace::Front), Some(CullFace::Back)]
        );
        // The pass ends with back culling and the cache cleared.
        assert!(!pass.cull.front());
        assert_eq!(pass.cull.end_pass(), CullFace::Back);
    }

    #[test]
    fn each_drawn_mesh_draws_plain_or_alpha_cut() {
        let mut volume =
            ShadowVolume::from_min_max(v(-100.0, -100.0, -100.0), v(100.0, 100.0, 100.0));
        let mut mixed = part(v(-1.0, -1.0, -1.0), v(1.0, 1.0, 1.0));
        mixed.front_cull = true;
        mixed.meshes = vec![
            CasterMesh {
                drawn: true,
                alpha_cut: false,
            },
            CasterMesh {
                drawn: false,
                alpha_cut: true,
            },
            CasterMesh {
                drawn: true,
                alpha_cut: true,
            },
        ];
        // A part whose meshes all skip still switches the face.
        let mut silent = part(v(-1.0, -1.0, -1.0), v(1.0, 1.0, 1.0));
        silent.meshes[0].drawn = false;
        let nodes = vec![node(
            v(-1.0, -1.0, -1.0),
            v(1.0, 1.0, 1.0),
            vec![mixed, silent],
        )];
        let pass = caster_draws(&nodes, &mut volume);
        let draws: Vec<_> = pass
            .draws
            .iter()
            .map(|d| (d.mesh, d.cull, d.alpha_cut))
            .collect();
        // The face switch rides on the part's first draw only.
        assert_eq!(
            draws,
            vec![(0, Some(CullFace::Front), false), (2, None, true)]
        );
        assert_eq!(
            pass.part_culls,
            vec![(0, 0, Some(CullFace::Front)), (0, 1, Some(CullFace::Back))]
        );
        assert_eq!(pass.visible, vec![vec![true, true]]);
    }

    #[test]
    fn the_planes_of_the_volume_keep_the_inside_negative() {
        let volume = ShadowVolume::from_min_max(v(-10.0, -20.0, -30.0), v(10.0, 20.0, 30.0));
        assert_eq!(volume.volume.planes.len(), 6);
        for plane in &volume.volume.planes {
            assert!(plane.distance(Vec3::ZERO) < 0.0);
        }
        // The +X face is the one with normal +X and offset −10.
        let face = volume
            .volume
            .planes
            .iter()
            .find(|p| p.normal == Vec3::X)
            .unwrap();
        assert_eq!(face.offset, -10.0);
        assert_eq!(face.distance(v(10.0, 0.0, 0.0)), 0.0);
    }

    /// Alpha-tested casters cull nothing, plain ones their back faces; a
    /// cloth draw keeps its own face and a two-sided material culls nothing.
    #[test]
    fn sun_casters_cull_as_the_original_pass() {
        use bevy::render::render_resource::Face;
        assert_eq!(sun_caster_cull(false, false, None), Some(Face::Back));
        assert_eq!(sun_caster_cull(true, false, None), None);
        assert_eq!(sun_caster_cull(false, true, None), None);
        assert_eq!(
            sun_caster_cull(true, false, Some(Face::Front)),
            Some(Face::Front)
        );
    }

    /// Only Bevy's directional shadow draws are sun casters.
    #[test]
    fn only_directional_shadow_draws_are_sun_casters() {
        use bevy::pbr::MeshPipelineKey as K;
        assert!(is_sun_caster(
            K::DEPTH_PREPASS | K::VIEW_PROJECTION_ORTHOGRAPHIC | K::UNCLIPPED_DEPTH_ORTHO
        ));
        assert!(!is_sun_caster(
            K::DEPTH_PREPASS | K::VIEW_PROJECTION_PERSPECTIVE
        ));
        assert!(!is_sun_caster(K::VIEW_PROJECTION_ORTHOGRAPHIC));
    }
}
