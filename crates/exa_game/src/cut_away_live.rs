//! The camera's cut-away fade, live: each frame the original's camera
//! driver casts from the looked-at point back past the eye and marks the
//! cut-away meshes it meets (roofs, awnings, overhangs); each mesh's fade
//! then moves 0.003 a millisecond toward its target, and the renderer draws
//! it solid, blended, or settled. The rules are the scene unit's
//! ([`exa_world::scene_nodes::Scene::detect_cut_away`],
//! [`exa_world::scene_nodes::fade_step`]) with the mesh callbacks of
//! [`exa_world::cut_away`]; the segment is the navigation unit's
//! ([`crate::nav_camera::cut_away_segment`]). Rendering only: no body,
//! pick or simulation reads the fade. Rules: `docs/RENDERING.md`
//! (Cut-away fade).
//!
//! How the original draws a mesh by its mask, and what Exarust draws:
//!
//! - solid ([`MAIN`]): its own material;
//! - fading ([`DITHERED`]): after the blended surfaces, depth first, then
//!   blended at an opacity of one less the fade amount (its `MatDif.a`);
//!   Exarust gives the part a blended copy of its material at that opacity
//!   that writes depth;
//! - settled below 0.999 ([`PARTIAL`]): without supersampling as fading;
//!   supersampled, opaque into half of its jittered sub-frames, which
//!   Exarust's 2 × 2 supersampled scene stands for by keeping one diagonal
//!   of each block;
//! - settled at or above 0.999: not drawn; Exarust draws it at opacity 0 so
//!   that it keeps casting its shadows, as the original's shadow passes do
//!   not read the fade.
//!
//! A part whose own material is blended is not drawn while its mesh fades
//! or has settled: the original's faded pass walks only the opaque and
//! alpha-tested lists, and its blended pass the solid meshes.

use bevy::camera::visibility::VisibilitySystems;
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use exa_formats::rfc::Node;
use exa_physics::intersect::segment_sphere;
use exa_render::material::{ExaMaterial, LateDraw, flags2};
use exa_render::passes::{FADED_COLOUR_BIAS, FADED_DEPTH_BIAS};
use exa_world::cut_away::{CAST_RADIUS, CutAway, ray_cast, sphere_cast};
use exa_world::scene_nodes::{CUT_AWAY_OVERHEAD_HEIGHT, DITHERED, MAIN, PARTIAL, SEEN, fade_step};

/// What the camera's cut-away driver casts from: the looked-at point, the
/// camera's backward unit vector and its distance (m). The chase camera
/// writes it each frame; `ready` stays false until it does (a fixed or
/// fly camera has no driver).
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct CutAwayCamera {
    pub focus: Vec3,
    pub backward: Vec3,
    pub distance: f32,
    pub ready: bool,
}

/// `EXA_CUT_AWAY=0` turns the fade off (every mesh drawn solid), for
/// before-and-after shots.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct CutAwayEnabled(pub bool);

impl Default for CutAwayEnabled {
    fn default() -> Self {
        Self(std::env::var("EXA_CUT_AWAY").map_or(true, |v| v != "0"))
    }
}

pub struct CutAwayPlugin;

impl Plugin for CutAwayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CutAwayCamera>()
            .init_resource::<CutAwayEnabled>()
            .add_systems(
                PostUpdate,
                (
                    detect_cut_away,
                    fade_cut_away,
                    draw_cut_away.after(
                        bevy::pbr::check_entities_needing_specialization::<
                            exa_render::material::ExaMaterial,
                        >,
                    ),
                )
                    .chain()
                    .after(TransformSystems::Propagate)
                    .after(VisibilitySystems::MarkNewlyHiddenEntitiesInvisible)
                    .in_set(exa_physics::FramePostPhase::View),
            );
    }
}

/// How a part is drawn while its mesh is cut away.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Look {
    /// Blended at this opacity (0: not drawn, shadows kept).
    Blend(f32),
    /// Opaque into one diagonal of each supersampled 2 × 2 block.
    Half,
    /// Not drawn at all (a part whose own material is blended).
    Hidden,
}

/// A part drawn with a cut-away copy of its material.
#[derive(Component, Debug, Clone)]
pub struct CutAwayPart {
    original: Handle<ExaMaterial>,
    faded: Option<Handle<ExaMaterial>>,
    look: Look,
    /// The depth half of the faded pass: a child drawing the part's mesh
    /// with colour writes off, and its material.
    depth: Option<(Entity, Handle<ExaMaterial>)>,
}

/// The child that lays a fading part's depth before any faded colour.
#[derive(Component, Debug, Clone, Copy)]
pub struct CutAwayDepth;

/// The parts' view visibility, and whether the fade hid them.
type Views<'w, 's> = Query<'w, 's, (&'static ViewVisibility, Option<&'static CutAwayPart>)>;

/// Whether the mesh is seen by a view this frame: the stand-in for the
/// original's drawn sector and for the mesh's own main and seen bits. A
/// mesh with parts is seen when one of them is (a part the fade hid counts,
/// so that its mesh can come back); a mesh that draws nothing (the levels'
/// fade planes, whose marks carry to the meshes under them) when its node
/// is not hidden.
fn seen(c: &CutAway, node: Option<&InheritedVisibility>, views: &Views) -> bool {
    if c.parts.is_empty() {
        return node.is_some_and(|v| v.get());
    }
    c.parts.iter().any(|&p| {
        views
            .get(p)
            .is_ok_and(|(v, r)| v.get() || r.is_some_and(|r| r.look == Look::Hidden))
    })
}

/// The detector: every seen candidate, in spawn order, against the camera's
/// segment (the overhead kind against a 10 m probe straight down onto the
/// looked-at point), its bounding sphere first and then its callback in
/// its own space. A hit marks the mesh, its parent with the parent carry,
/// and (overhead kind only) its mesh children with the children carry; a
/// miss on a marked mesh clears the same, the children whatever the kind.
pub fn detect_cut_away(
    camera: Res<CutAwayCamera>,
    enabled: Res<CutAwayEnabled>,
    mut nodes: Query<(
        Entity,
        &mut CutAway,
        &GlobalTransform,
        Option<&InheritedVisibility>,
    )>,
    views: Views,
    names: Query<&Name>,
) {
    if !camera.ready {
        return;
    }
    if !enabled.0 {
        for (_, mut c, _, _) in &mut nodes {
            if c.cut {
                c.cut = false;
            }
        }
        return;
    }
    let (from, direction, length) =
        crate::nav_camera::cut_away_segment(camera.focus, camera.backward, camera.distance);
    let segment = direction * length;
    let top = camera.focus + Vec3::Y * CUT_AWAY_OVERHEAD_HEIGHT;
    let down = Vec3::NEG_Y * CUT_AWAY_OVERHEAD_HEIGHT;
    let mut order: Vec<Entity> = nodes
        .iter()
        .filter(|(_, c, _, _)| c.candidate())
        .map(|(e, _, _, _)| e)
        .collect();
    order.sort_unstable();
    for e in order {
        let Ok((_, c, frame, shown)) = nodes.get(e) else {
            continue;
        };
        if !seen(c, shown, &views) {
            continue;
        }
        let centre = frame.transform_point(c.centre);
        let inverse = frame.affine().inverse();
        let turn = frame.rotation().inverse();
        let kind = c.kind();
        let hit = match kind {
            0x6000_0000 => {
                segment_sphere(top, down, centre, c.radius)
                    && c.triangles.as_ref().is_none_or(|t| {
                        ray_cast(
                            t,
                            inverse.transform_point3(top),
                            turn * Vec3::NEG_Y,
                            CUT_AWAY_OVERHEAD_HEIGHT,
                        )
                        .is_some()
                    })
            }
            0x3000_0000 => segment_sphere(from, segment, centre, c.radius),
            0x2000_0000 => {
                segment_sphere(from, segment, centre, c.radius)
                    && c.triangles.as_ref().is_none_or(|t| {
                        sphere_cast(
                            t,
                            inverse.transform_point3(from),
                            turn * direction,
                            length,
                            CAST_RADIUS,
                        )
                        .is_some()
                    })
            }
            _ => false,
        };
        if !hit && !c.cut {
            continue;
        }
        let mut marked = vec![e];
        if c.word & Node::CUT_AWAY_PARENT != 0
            && let Some(parent) = c.parent
        {
            marked.push(parent);
        }
        if c.word & Node::CUT_AWAY_CHILDREN != 0 && (!hit || kind == 0x6000_0000) {
            marked.extend(c.children.iter().copied());
        }
        for node in marked {
            if let Ok((_, mut record, _, _)) = nodes.get_mut(node)
                && record.cut != hit
            {
                record.cut = hit;
                let name = names.get(node).map_or("?", |n| n.as_str());
                info!(
                    "cut-away: {name} {}",
                    if hit { "marked" } else { "cleared" }
                );
            }
        }
    }
}

/// The fade step of every record, on the frame clock's whole milliseconds.
pub fn fade_cut_away(
    time: Res<Time>,
    mut clock: Local<Option<u64>>,
    mut nodes: Query<(&mut CutAway, Option<&InheritedVisibility>)>,
    views: Views,
) {
    let now = (time.elapsed_secs_f64() * 1000.0).floor() as u64;
    let dt = clock
        .map_or(0, |last| now.saturating_sub(last))
        .min(i32::MAX as u64) as i32;
    *clock = Some(now);
    for (mut c, shown) in &mut nodes {
        let mask = if seen(&c, shown, &views) {
            MAIN | SEEN
        } else {
            0
        };
        let mut fade = c.fade;
        let mask = fade_step(mask, c.cut, &mut fade, c.target, dt);
        if c.mask != mask || c.fade != fade {
            c.mask = mask;
            c.fade = fade;
        }
    }
}

/// How a mesh with mask `mask` and fade `fade` is drawn: `None` solid.
fn look_of(mask: u32, fade: f32, supersampled: bool) -> Option<Look> {
    if mask & MAIN != 0 {
        None
    } else if mask & DITHERED != 0 {
        Some(Look::Blend(1.0 - fade))
    } else if mask & PARTIAL != 0 {
        Some(if supersampled {
            Look::Half
        } else {
            Look::Blend(1.0 - fade)
        })
    } else {
        Some(Look::Blend(0.0))
    }
}

/// The cut-away copy of `original` for `look`. A blended one draws in the
/// original's faded pass: after the blended lists, as its colour half
/// (blended at the opacity, depth writes off), keeping the record's alpha
/// test.
fn faded_copy(original: &ExaMaterial, look: Look) -> ExaMaterial {
    let mut m = original.clone();
    match look {
        Look::Blend(opacity) => {
            m.alpha_tested = original.is_alpha_tested();
            m.alpha_mode = AlphaMode::Blend;
            m.params.flags2 |= flags2::CUT_AWAY_BLEND;
            m.params.cut_away_opacity = opacity;
            m.late = LateDraw::FadedColour;
            m.draw_order = FADED_COLOUR_BIAS;
        }
        Look::Half => m.params.flags2 |= flags2::CUT_AWAY_HALF,
        Look::Hidden => {}
    }
    m
}

/// The faded pass's depth half of a blended cut-away copy: the same
/// surface with colour writes off and depth writes on, ahead of every
/// faded colour.
fn depth_copy(faded: &ExaMaterial) -> ExaMaterial {
    let mut m = faded.clone();
    m.late = LateDraw::FadedDepth;
    m.draw_order = FADED_DEPTH_BIAS;
    m
}

/// Whether `look` draws the faded pass's depth half: a blended look the
/// faded pass draws (opacity above zero).
fn lays_depth(look: Look) -> bool {
    matches!(look, Look::Blend(opacity) if opacity > 0.0)
}

/// Whether the surface is one of the original's blended ones.
fn is_blended(m: &ExaMaterial) -> bool {
    m.blend_state.is_some()
        || matches!(
            m.alpha_mode,
            AlphaMode::Blend | AlphaMode::Add | AlphaMode::Premultiplied | AlphaMode::Multiply
        )
}

/// Gives each cut-away part the material its mesh's draw asks, and gives
/// it back its own once the mesh is solid again. A part the faded pass
/// draws also gets a depth child ([`CutAwayDepth`]) that lays its depth,
/// with every other faded part's, before any faded colour.
#[allow(clippy::type_complexity)]
pub fn draw_cut_away(
    mut commands: Commands,
    nodes: Query<&CutAway, Changed<CutAway>>,
    mut parts: Query<(
        &mut MeshMaterial3d<ExaMaterial>,
        Option<&mut CutAwayPart>,
        &mut Visibility,
        Option<&Mesh3d>,
    )>,
    mut materials: ResMut<Assets<ExaMaterial>>,
    supersampling: Option<Res<exa_render::supersample::SuperSampling>>,
) {
    let supersampled = supersampling.is_some_and(|s| s.0);
    for c in &nodes {
        // A mesh no view sees keeps its last draw.
        if c.mask & (MAIN | SEEN | DITHERED | PARTIAL) == 0 {
            continue;
        }
        let wanted = look_of(c.mask, c.fade, supersampled);
        for &part in &c.parts {
            let Ok((mut handle, record, mut visibility, mesh)) = parts.get_mut(part) else {
                continue;
            };
            match (wanted, record) {
                (None, None) => {}
                (None, Some(record)) => {
                    if record.faded.as_ref().is_none_or(|f| *f == handle.0) {
                        handle.0 = record.original.clone();
                    }
                    if let Some(faded) = &record.faded {
                        materials.remove(faded);
                    }
                    drop_depth(&mut commands, &mut materials, record.depth.as_ref());
                    if record.look == Look::Hidden {
                        *visibility = Visibility::Inherited;
                    }
                    commands.entity(part).remove::<CutAwayPart>();
                }
                (Some(look), record) => {
                    let original = match &record {
                        Some(r) => r.original.clone(),
                        None => handle.0.clone(),
                    };
                    let Some(source) = materials.get(&original) else {
                        continue;
                    };
                    let look = if is_blended(source) {
                        Look::Hidden
                    } else {
                        look
                    };
                    if record.as_ref().is_some_and(|r| r.look == look) {
                        continue;
                    }
                    let old_depth = record.as_ref().and_then(|r| r.depth.clone());
                    if look == Look::Hidden {
                        *visibility = Visibility::Hidden;
                        if let Some(r) = &record
                            && let Some(faded) = &r.faded
                        {
                            materials.remove(faded);
                        }
                        drop_depth(&mut commands, &mut materials, old_depth.as_ref());
                        commands.entity(part).insert(CutAwayPart {
                            original,
                            faded: None,
                            look,
                            depth: None,
                        });
                        continue;
                    }
                    let copy = faded_copy(source, look);
                    // The depth half: made once while the part draws in the
                    // faded pass (its opacity plays no part in it).
                    let depth = match (lays_depth(look), old_depth, mesh) {
                        (true, Some(depth), _) => Some(depth),
                        (true, None, Some(mesh)) => {
                            let material = materials.add(depth_copy(&copy));
                            let child = commands
                                .spawn((
                                    Name::new("cut-away depth"),
                                    CutAwayDepth,
                                    Mesh3d(mesh.0.clone()),
                                    MeshMaterial3d(material.clone()),
                                    bevy::light::NotShadowCaster,
                                    Transform::IDENTITY,
                                    Visibility::Inherited,
                                    ChildOf(part),
                                ))
                                .id();
                            Some((child, material))
                        }
                        (_, old, _) => {
                            drop_depth(&mut commands, &mut materials, old.as_ref());
                            None
                        }
                    };
                    let faded = match record.as_ref().and_then(|r| r.faded.clone()) {
                        Some(faded) => {
                            if let Some(mut slot) = materials.get_mut(&faded) {
                                *slot = copy;
                            }
                            faded
                        }
                        None => materials.add(copy),
                    };
                    handle.0 = faded.clone();
                    commands.entity(part).insert(CutAwayPart {
                        original,
                        faded: Some(faded),
                        look,
                        depth,
                    });
                }
            }
        }
    }
}

/// Takes a part's depth child and its material away.
fn drop_depth(
    commands: &mut Commands,
    materials: &mut Assets<ExaMaterial>,
    depth: Option<&(Entity, Handle<ExaMaterial>)>,
) {
    if let Some((child, material)) = depth {
        commands.entity(*child).try_despawn();
        materials.remove(material);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_world::scene_nodes::FADE_RATE;

    #[test]
    fn the_draw_follows_the_mask() {
        assert_eq!(look_of(MAIN | SEEN, 0.0, true), None);
        assert_eq!(
            look_of(SEEN | DITHERED, 0.25, true),
            Some(Look::Blend(0.75))
        );
        assert_eq!(look_of(SEEN | PARTIAL, 0.5, true), Some(Look::Half));
        assert_eq!(look_of(SEEN | PARTIAL, 0.5, false), Some(Look::Blend(0.5)));
        assert_eq!(look_of(SEEN, 1.0, true), Some(Look::Blend(0.0)));
    }

    #[test]
    fn a_blended_copy_replaces_the_opacity_only() {
        let source = ExaMaterial::default();
        let copy = faded_copy(&source, Look::Blend(0.4));
        assert_eq!(copy.alpha_mode, AlphaMode::Blend);
        assert_eq!(copy.params.cut_away_opacity, 0.4);
        assert_eq!(copy.params.diffuse, source.params.diffuse);
        assert_ne!(copy.params.flags2 & flags2::CUT_AWAY_BLEND, 0);
        let half = faded_copy(&source, Look::Half);
        assert_eq!(half.alpha_mode, source.alpha_mode);
        assert_ne!(half.params.flags2 & flags2::CUT_AWAY_HALF, 0);
    }

    /// The faded pass: the blended copy draws as its colour half after the
    /// blended lists, its depth half ahead of it; an alpha-tested record
    /// keeps its test in both; a settled part (opacity 0) lays no depth.
    #[test]
    fn the_faded_pass_lays_depth_before_colour() {
        let source = ExaMaterial {
            alpha_mode: AlphaMode::Mask(0.5),
            ..default()
        };
        let colour = faded_copy(&source, Look::Blend(0.4));
        assert_eq!(colour.late, LateDraw::FadedColour);
        assert!(colour.is_alpha_tested());
        let depth = depth_copy(&colour);
        assert_eq!(depth.late, LateDraw::FadedDepth);
        assert!(depth.is_alpha_tested());
        use exa_render::passes::PhaseList;
        assert!(PhaseList::Blended.bias() + exa_render::passes::LIST_BLOCK <= depth.draw_order);
        assert!(depth.draw_order + 1000.0 < colour.draw_order);
        // After everything with no list (bias 0 plus a view depth).
        assert!(depth.draw_order > 1000.0);
        assert_eq!(PhaseList::of_sort_value(depth.draw_order), PhaseList::Late);
        assert!(lays_depth(Look::Blend(0.4)));
        assert!(!lays_depth(Look::Blend(0.0)));
        assert!(!lays_depth(Look::Half));
    }

    /// The live draw: a fading part takes its blended copy (the faded
    /// pass's colour half) and a depth child drawing its mesh as the depth
    /// half; once the mesh is solid again both go and its own material is
    /// back.
    #[test]
    fn a_fading_part_gets_a_depth_child_until_it_is_solid() {
        let mut app = App::new();
        app.init_resource::<Assets<ExaMaterial>>()
            .add_systems(Update, draw_cut_away);
        let own = app
            .world_mut()
            .resource_mut::<Assets<ExaMaterial>>()
            .add(ExaMaterial::default());
        let part = app
            .world_mut()
            .spawn((
                Mesh3d(Handle::default()),
                MeshMaterial3d(own.clone()),
                Visibility::default(),
            ))
            .id();
        let mut record = CutAway::new(0x6000_0000, None, Vec3::ZERO, Vec3::ONE);
        record.parts = vec![part];
        record.mask = SEEN | DITHERED;
        record.fade = 0.25;
        let roof = app.world_mut().spawn(record).id();
        app.update();
        let children = |app: &mut App| {
            app.world_mut()
                .query_filtered::<(&ChildOf, &MeshMaterial3d<ExaMaterial>), With<CutAwayDepth>>()
                .iter(app.world())
                .map(|(c, m)| (c.parent(), m.0.clone()))
                .collect::<Vec<_>>()
        };
        let depth = children(&mut app);
        assert_eq!(depth.len(), 1);
        assert_eq!(depth[0].0, part);
        let materials = app.world().resource::<Assets<ExaMaterial>>();
        assert_eq!(
            materials.get(&depth[0].1).unwrap().late,
            LateDraw::FadedDepth
        );
        let drawn = app
            .world()
            .get::<MeshMaterial3d<ExaMaterial>>(part)
            .unwrap();
        let colour = materials.get(&drawn.0).unwrap();
        assert_eq!(colour.late, LateDraw::FadedColour);
        assert_eq!(colour.params.cut_away_opacity, 0.75);
        // Solid again: the child and the copies go.
        app.world_mut().get_mut::<CutAway>(roof).unwrap().mask = MAIN | SEEN;
        app.update();
        assert!(children(&mut app).is_empty());
        assert_eq!(
            app.world()
                .get::<MeshMaterial3d<ExaMaterial>>(part)
                .unwrap()
                .0,
            own
        );
    }

    /// An app with a candidate roof over the origin and a camera probe
    /// looking down at the origin from the south.
    fn roof_app(kind: u32) -> (App, Entity, Entity) {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<CutAwayCamera>()
            .init_resource::<CutAwayEnabled>()
            .add_systems(Update, (detect_cut_away, fade_cut_away).chain());
        let part = app.world_mut().spawn(ViewVisibility::VISIBLE).id();
        let a = Vec3::new(-4.0, 0.0, -4.0);
        let b = Vec3::new(4.0, 0.0, -4.0);
        let c = Vec3::new(4.0, 0.0, 4.0);
        let d = Vec3::new(-4.0, 0.0, 4.0);
        let tris = std::sync::Arc::new(vec![[a, c, b], [a, d, c]]);
        let mut record = CutAway::new(kind, Some(tris), Vec3::ZERO, Vec3::new(4.0, 0.1, 4.0));
        record.parts = vec![part];
        let roof = app
            .world_mut()
            .spawn((record, GlobalTransform::from_translation(Vec3::Y * 3.0)))
            .id();
        *app.world_mut().resource_mut::<CutAwayCamera>() = CutAwayCamera {
            focus: Vec3::Y,
            backward: Vec3::new(0.0, 1.0, 1.0).normalize(),
            distance: 20.0,
            ready: true,
        };
        (app, roof, part)
    }

    #[test]
    fn an_overhead_roof_is_marked_and_fades() {
        let (mut app, roof, _) = roof_app(0x6000_0000);
        app.update();
        let c = app.world().get::<CutAway>(roof).unwrap();
        assert!(c.cut);
        // Advance the frame clock by 100 ms.
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(100));
        app.update();
        let c = app.world().get::<CutAway>(roof).unwrap();
        assert!((c.fade - 100.0 * FADE_RATE).abs() < 1e-5, "{}", c.fade);
        assert_ne!(c.mask & DITHERED, 0);
        // Out from under the roof the mark clears.
        app.world_mut().resource_mut::<CutAwayCamera>().focus = Vec3::new(20.0, 1.0, 0.0);
        app.update();
        assert!(!app.world().get::<CutAway>(roof).unwrap().cut);
    }

    #[test]
    fn a_segment_kind_needs_the_camera_line_through_it() {
        let (mut app, roof, _) = roof_app(0x2000_0000);
        app.update();
        // The segment from 40 m back down onto the point passes the roof.
        assert!(app.world().get::<CutAway>(roof).unwrap().cut);
        // Cast up through the roof from below (the point above it), the
        // sphere moves away from the face's front: no hit.
        *app.world_mut().resource_mut::<CutAwayCamera>() = CutAwayCamera {
            focus: Vec3::Y * 5.0,
            backward: Vec3::NEG_Y,
            distance: 20.0,
            ready: true,
        };
        app.update();
        assert!(!app.world().get::<CutAway>(roof).unwrap().cut);
    }

    #[test]
    fn turned_off_nothing_is_marked() {
        let (mut app, roof, _) = roof_app(0x6000_0000);
        app.insert_resource(CutAwayEnabled(false));
        app.update();
        assert!(!app.world().get::<CutAway>(roof).unwrap().cut);
    }
}
