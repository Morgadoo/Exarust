//! The GI light shapes the running effects register: the force blast's and
//! the force torrent's multi-point lights and the teleporter bubble's
//! pulsing shell, injected into the voxel GI as
//! [`exa_render::gi::GiShape`]s.
//!
//! As the original does it:
//!
//! - **Register**: the effect's creator makes its emissive and appends it to
//!   the scene's GI manager list (only while the scene has a GI manager;
//!   Exarust's pass draws the listed shapes only with the GI on). Here the
//!   shape is its own entity, listed in [`exa_render::gi::GiShapes`] while
//!   it lives, owned by the effect ([`EffectLight`], [`LightOwner`]).
//! - **Update**: every frame the effect's frame refreshes the emissive's
//!   box, and the GI pass reads its values from the effect: the blast's
//!   points are its chunks' light points ([`Blast::frame_lights`]), the
//!   torrent's its gathered light points, the bubble's its phase.
//! - **Remove**: the manager drops the emissive by swap removal and frees
//!   it when the blast's frame builds nothing (once, [`EffectLightFreed`]),
//!   and when any of them is destroyed (the effect's entity leaves the
//!   scene and [`drop_orphan_lights`] despawns its shape).
//!
//! The values are [`crate::effects::emissive`]'s, scaled by the manager's
//! light scale [`GI_LIGHT_SCALE`]. [inf: the teleporter bubble's `Radius`
//! is the 120 cm sphere times its swell scale and its frame the bubble's
//! place, unturned and unscaled; the original reads both from the bubble's
//! mesh node. Its `Tex0` is the pass's shared stand-in texture.]
//!
//! Evidence: the blast's, torrent's and teleporter bubble's creators,
//! destroy slots and frames, the manager's list append and swap removal,
//! and the three emissives' bounds and draws, read from `Exanima.exe`.

use bevy::prelude::*;
use exa_render::gi::{GiShape, GiShapeProgram, ShapeVolume};

use super::blast_live::LiveBlast;
use super::torrent_live::LiveTorrent;
use crate::effects::Frame as ViewFrame;
use crate::effects::blast::{Blast, THROWN};
use crate::effects::bubble::{Bubble, BubbleEffect, SIZE_CM};
use crate::effects::emissive::{self, FALLOFF_CM, MultiPointLight};
use crate::effects::torrent::Torrent;

/// Centimetres a metre.
const CM: f32 = 100.0;

/// The GI manager's light scale, which every emissive colour takes: 0.09
/// at the high shading quality (the default), 0.1 below it.
pub const GI_LIGHT_SCALE: f32 = 0.09;

/// The effect's registered light shape.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectLight(pub Entity);

/// The effect whose light this shape is.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LightOwner(pub Entity);

/// The effect's light has been freed and is not registered again (the
/// blast's frame sets this once it builds nothing).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectLightFreed;

/// A box in centimetres (centre, half extents) as the pass's volume in
/// metres.
fn volume_of(centre_cm: Vec3, half_cm: Vec3) -> ShapeVolume {
    ShapeVolume {
        position: centre_cm / CM,
        half: half_cm / CM,
    }
}

/// A multi-point light as a shape: its points (world cm) and attenuation.
fn multipoint(light: MultiPointLight) -> GiShape {
    GiShape {
        program: GiShapeProgram::Multipoint,
        atten: light.attenuation,
        points: light.points.into(),
        ..default()
    }
}

/// The blast's shape this frame: the multi-point light of every chunk's
/// life record ([`emissive::blast_light`], thrown from state 1 on) and the
/// box of its light points plus 120 cm, moved by its node's place (the
/// blast's node stands at the world's origin: its strip is built in world
/// space).
pub fn blast_shape(blast: &Blast, gain: f32) -> Option<(GiShape, ShapeVolume)> {
    let points = blast.light_points();
    let places: Vec<Vec3> = points.iter().map(|&(at, _)| at).collect();
    let (centre, half) = emissive::point_box(&places)?;
    let (centre, half) = emissive::node_box(centre, half, Vec3::ZERO);
    let light = emissive::blast_light(
        &points,
        blast.state >= THROWN,
        blast.charge,
        blast.life_ticks,
        blast.setup.light,
        gain,
    );
    Some((multipoint(light), volume_of(centre, half)))
}

/// The torrent's shape this frame: its gathered light points (world cm, as
/// its frame left them) at its level, the torrent's charge
/// ([`emissive::torrent_light`]), in their box plus 120 cm. Without points
/// it injects nothing, in an empty box.
pub fn torrent_shape(torrent: &Torrent, gain: f32) -> (GiShape, ShapeVolume) {
    let points: Vec<(Vec3, f32)> = torrent
        .lights
        .points
        .iter()
        .map(|&(at, weight, _)| (at, weight))
        .collect();
    let places: Vec<Vec3> = points.iter().map(|&(at, _)| at).collect();
    let (centre, half) = emissive::point_box(&places).unwrap_or((Vec3::ZERO, Vec3::ZERO));
    let light = emissive::torrent_light(&points, torrent.charge, gain);
    (multipoint(light), volume_of(centre, half))
}

/// The teleporter bubble's shape at `now_ms`, the bubble at `at_cm`: the
/// `gi_telebubble` shell with [`emissive::teleporter_light`]'s colour,
/// fade, pulse and phase, its texture window, and the mesh node's world box
/// plus 60 cm ([`emissive::world_box`]).
pub fn bubble_shape(
    bubble: &Bubble,
    at_cm: Vec3,
    now_ms: u32,
    gain: f32,
) -> (GiShape, ShapeVolume) {
    let radius = SIZE_CM * bubble.uniforms(now_ms).scale;
    let light = emissive::teleporter_light(
        bubble.state.word() as u32,
        bubble.fade,
        bubble.charge,
        bubble.stamp,
        now_ms,
        gain,
    );
    let (centre, half) = emissive::world_box(
        at_cm,
        [Vec3::X, Vec3::Y, Vec3::Z],
        Vec3::splat(radius),
        FALLOFF_CM,
    );
    let shape = GiShape {
        program: GiShapeProgram::TeleBubble,
        colour: light.colour,
        matrix: Mat4::from_translation(-at_cm),
        half: Vec3::splat(radius),
        tex_pos: Vec2::from(bubble.tex_pos),
        radius,
        fade: light.fade,
        pulse: light.pulse,
        anim: light.anim,
        ..default()
    };
    (shape, volume_of(centre, half))
}

/// The force Barrier's dome shape: the `gi_dome` program, its extent
/// (the node scale less 60 across and 40 up), its colour (the light's,
/// already at the manager's light scale, as the emissive's draw takes it
/// once) and the inverse of the cap node's frame (`bMat`), in the node's
/// world box plus 60 cm. `axes` are the node's turned axes (columns) and
/// `at_cm` its place.
pub fn barrier_shape(
    light: &emissive::FacedLight,
    at_cm: Vec3,
    axes: [Vec3; 3],
    scale_cm: Vec3,
) -> (GiShape, ShapeVolume) {
    let (centre, half) = emissive::world_box(at_cm, axes, scale_cm, FALLOFF_CM);
    let node = Mat4::from_cols(
        axes[0].extend(0.0),
        axes[1].extend(0.0),
        axes[2].extend(0.0),
        at_cm.extend(1.0),
    );
    let shape = GiShape {
        program: GiShapeProgram::Dome,
        colour: light.colour,
        matrix: node.inverse(),
        half: light.extent,
        ..default()
    };
    (shape, volume_of(centre, half))
}

/// The force Bubble's shell shape: the `gi_bubble` program with the light's
/// radius plus five, its colour (already at the manager's light scale),
/// its fade and its frame, in the node's world box plus 60 cm.
pub fn force_bubble_shape(
    light: &emissive::BubbleLight,
    at_cm: Vec3,
    scale_cm: Vec3,
) -> (GiShape, ShapeVolume) {
    let (centre, half) =
        emissive::world_box(at_cm, [Vec3::X, Vec3::Y, Vec3::Z], scale_cm, FALLOFF_CM);
    let shape = GiShape {
        program: GiShapeProgram::Bubble,
        colour: light.colour,
        matrix: Mat4::from_translation(-at_cm),
        half: Vec3::splat(light.radius),
        radius: light.radius,
        fade: light.fade,
        ..default()
    };
    (shape, volume_of(centre, half))
}

/// A cuboid emissive's box shape: the `gi_cuboid` program, `max(scale −
/// 20, 10)` half extents, the light's own colour times the manager's scale
/// and the node's frame, in the node's world box plus 80 cm.
pub fn cuboid_shape(
    at_cm: Vec3,
    axes: [Vec3; 3],
    scale_cm: Vec3,
    colour: Vec3,
) -> (GiShape, ShapeVolume) {
    let light = emissive::cuboid_light(scale_cm, colour * GI_LIGHT_SCALE);
    let (centre, half) = emissive::world_box(at_cm, axes, scale_cm, emissive::CUBOID_FALLOFF_CM);
    let shape = GiShape {
        program: GiShapeProgram::Cuboid,
        colour: light.colour,
        matrix: Mat4::from_translation(-at_cm),
        half: light.extent,
        ..default()
    };
    (shape, volume_of(centre, half))
}

/// A tendril image's multi-point light: one point a strand anchor with its
/// intensity, [`emissive::tendril_light`]'s colour at the image's scale and
/// the manager's, attenuation 16000, in the points' box plus 120 cm moved
/// by the node's place.
pub fn tendril_shape(
    points: &[(Vec3, f32)],
    at_cm: Vec3,
    scale: f32,
    gain: f32,
) -> Option<(GiShape, ShapeVolume)> {
    let lit = emissive::tendril_light(points, scale, gain);
    let places: Vec<Vec3> = points.iter().map(|&(at, _)| at).collect();
    let (centre, half) = emissive::point_box(&places)?;
    let (centre, half) = emissive::node_box(centre, half, at_cm);
    let shape = GiShape {
        program: GiShapeProgram::Multipoint,
        atten: emissive::MULTIPOINT_ATTENUATION,
        points: lit.into(),
        ..default()
    };
    Some((shape, volume_of(centre, half)))
}

/// The force barriers' dome lights: each live barrier registers its dome
/// shape from its phase and share over the cap's 85 by 85 by 25 cm node,
/// and drops it when the barrier is gone.
#[allow(clippy::type_complexity)]
pub(crate) fn barrier_lights(
    mut commands: Commands,
    barriers: Query<(
        Entity,
        &super::barrier_live::LiveBarrier,
        Option<&EffectLight>,
    )>,
    mut shapes: Query<(&mut GiShape, &mut ShapeVolume)>,
) {
    for (entity, live, light) in &barriers {
        if live.barrier.dome.removed {
            if let Some(light) = light {
                commands.entity(light.0).despawn();
                commands.entity(entity).remove::<EffectLight>();
            }
            continue;
        }
        let dome = &live.barrier.dome;
        let scale = super::super::effects::force_dome::BARRIER_DRAW_SCALE;
        let lit = emissive::barrier_light(dome.phase, dome.shown_share, scale, GI_LIGHT_SCALE);
        // The cap's node: its drawn centre, turned to its facing. [inf: the
        // node's basis is taken as the engine's look basis of the facing
        // (z along it, y up), matching the keep-out's frame.]
        let facing = live.barrier.frame_facing(1.0);
        let turn = exa_physics::engine_math::look_rotation(facing);
        let axes = [
            Vec3::from(turn.x_axis),
            Vec3::from(turn.y_axis),
            Vec3::from(turn.z_axis),
        ];
        let placed = barrier_shape(&lit, live.barrier.drawn_centre(1.0), axes, scale);
        place_light(
            &mut commands,
            entity,
            light,
            &mut shapes,
            placed,
            "barrier light",
        );
    }
}

/// The force bubbles' shell lights: each live bubble registers its shell
/// shape from its phase, share and radius, and drops it when the bubble is
/// gone.
#[allow(clippy::type_complexity)]
pub(crate) fn force_bubble_lights(
    mut commands: Commands,
    bubbles: Query<(
        Entity,
        &super::bubble_live::LiveBubble,
        Option<&EffectLight>,
    )>,
    mut shapes: Query<(&mut GiShape, &mut ShapeVolume)>,
) {
    for (entity, live, light) in &bubbles {
        let dome = &live.bubble.dome;
        if dome.removed {
            if let Some(light) = light {
                commands.entity(light.0).despawn();
                commands.entity(entity).remove::<EffectLight>();
            }
            continue;
        }
        let lit = emissive::bubble_light(dome.phase, dome.shown_share, dome.radius, GI_LIGHT_SCALE);
        let placed = force_bubble_shape(&lit, dome.at, Vec3::splat(dome.radius));
        place_light(
            &mut commands,
            entity,
            light,
            &mut shapes,
            placed,
            "force bubble light",
        );
    }
}

/// Writes `shape` to the effect's light, registering one first if it has
/// none.
fn place_light(
    commands: &mut Commands,
    owner: Entity,
    light: Option<&EffectLight>,
    shapes: &mut Query<(&mut GiShape, &mut ShapeVolume)>,
    (shape, volume): (GiShape, ShapeVolume),
    name: &'static str,
) {
    match light.and_then(|light| shapes.get_mut(light.0).ok()) {
        Some((mut current, mut placed)) => {
            *current = shape;
            *placed = volume;
        }
        None => {
            debug!(
                "{name} registered for {owner:?}: {} points, box {:?} ± {:?} m",
                shape.points.len(),
                volume.position,
                volume.half
            );
            let id = commands
                .spawn((
                    Name::new(name),
                    shape,
                    volume,
                    Transform::default(),
                    LightOwner(owner),
                ))
                .id();
            commands.entity(owner).insert(EffectLight(id));
        }
    }
}

/// The blasts' lights: each blast registers its shape and refreshes it from
/// this frame's light points; the frame that builds nothing frees it.
#[allow(clippy::type_complexity)]
pub(crate) fn blast_lights(
    mut commands: Commands,
    mut blasts: Query<(
        Entity,
        &mut LiveBlast,
        Option<&EffectLight>,
        Has<EffectLightFreed>,
    )>,
    mut shapes: Query<(&mut GiShape, &mut ShapeVolume)>,
    cameras: Query<&GlobalTransform, exa_render::supersample::SceneCamera>,
    time: Res<Time<Fixed>>,
) {
    let Some(camera) = cameras.iter().next() else {
        return;
    };
    let view = ViewFrame {
        x: camera.right().as_vec3(),
        y: camera.up().as_vec3(),
        z: camera.back().as_vec3(),
        at: camera.translation() * CM,
    };
    let alpha = time.overstep_fraction();
    for (entity, mut live, light, freed) in &mut blasts {
        if freed {
            continue;
        }
        if live.blast.builds_nothing() {
            if let Some(light) = light {
                commands.entity(light.0).despawn();
                debug!("blast light freed for {entity:?}");
            }
            commands
                .entity(entity)
                .remove::<EffectLight>()
                .insert(EffectLightFreed);
            continue;
        }
        live.blast.frame_lights(alpha, &view);
        if let Some(placed) = blast_shape(&live.blast, GI_LIGHT_SCALE) {
            place_light(
                &mut commands,
                entity,
                light,
                &mut shapes,
                placed,
                "blast light",
            );
        }
    }
}

/// The torrents' lights, refreshed after their frame gathered the points;
/// a torrent's light leaves with it.
pub(crate) fn torrent_lights(
    mut commands: Commands,
    torrents: Query<(Entity, &LiveTorrent, Option<&EffectLight>)>,
    mut shapes: Query<(&mut GiShape, &mut ShapeVolume)>,
) {
    for (entity, live, light) in &torrents {
        let placed = torrent_shape(&live.torrent, GI_LIGHT_SCALE);
        place_light(
            &mut commands,
            entity,
            light,
            &mut shapes,
            placed,
            "torrent light",
        );
    }
}

/// The teleporter bubbles' lights, refreshed from their phase every frame;
/// a bubble's light leaves with it.
pub(crate) fn bubble_lights(
    mut commands: Commands,
    bubbles: Query<(
        Entity,
        &BubbleEffect,
        &GlobalTransform,
        Option<&EffectLight>,
    )>,
    mut shapes: Query<(&mut GiShape, &mut ShapeVolume)>,
    clock: Option<Res<crate::effects::EffectClock>>,
) {
    let Some(clock) = clock else {
        return;
    };
    for (entity, bubble, at, light) in &bubbles {
        let placed = bubble_shape(
            &bubble.0,
            at.translation() * CM,
            clock.now_ms,
            GI_LIGHT_SCALE,
        );
        place_light(
            &mut commands,
            entity,
            light,
            &mut shapes,
            placed,
            "teleporter bubble light",
        );
    }
}

/// Despawns the light shapes whose effect has left the scene: the effect's
/// destroy takes its emissive out of the manager's list and frees it.
pub(crate) fn drop_orphan_lights(
    mut commands: Commands,
    lights: Query<(Entity, &LightOwner)>,
    owners: Query<(), With<EffectLight>>,
) {
    for (entity, owner) in &lights {
        if owners.get(owner.0).is_err() {
            commands.entity(entity).despawn();
            debug!(
                "effect light {entity:?} freed with its effect {:?}",
                owner.0
            );
        }
    }
}

/// A debug helper (`--debug-force-cast <s>`): every `cast_every` seconds the
/// player's bar opens, picks its first slot, closes and uses it; a force
/// effect the player casts aims at the nearest other character instead of
/// the cursor. Every tenth frame it logs the registered light shapes and the
/// support-pin gain and landing weight of every body still loosened by a
/// hit, so a run shows the light and the recovery.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn debug_force_casts(
    debug: Res<super::live::ThaumDebug>,
    time: Res<Time>,
    players: Query<
        Entity,
        (
            With<crate::player::PlayerBody>,
            With<super::cast::PowerCaster>,
        ),
    >,
    bodies: Query<(Entity, &exa_physics::body::Ragdoll)>,
    mut blasts: Query<&mut LiveBlast>,
    mut torrents: Query<&mut LiveTorrent>,
    shapes: Query<(Entity, &GiShape, &ShapeVolume)>,
    mut bars: MessageWriter<super::cast::BarCommand>,
    mut next: Local<f32>,
    mut frame: Local<u32>,
) {
    use super::cast::{BarAction, BarCommand};
    let Some(player) = players.iter().next() else {
        return;
    };
    let every = debug.cast_every as f32;
    let now = time.elapsed_secs();
    if *next == 0.0 {
        *next = now + every;
    }
    if now >= *next {
        *next = now + every;
        for action in [
            BarAction::Toggle,
            BarAction::Pick(0),
            BarAction::Toggle,
            BarAction::Use,
        ] {
            bars.write(BarCommand {
                caster: player,
                action,
            });
        }
        info!("debug force cast: the player uses its first power");
    }
    let chest = |entity: Entity| {
        bodies
            .get(entity)
            .ok()
            .and_then(|(_, body)| super::blast_live::chest_point(body))
    };
    let nearest = |from: Vec3| {
        bodies
            .iter()
            .filter(|(entity, body)| *entity != player && body.drive.is_some())
            .filter_map(|(entity, body)| {
                super::blast_live::chest_point(body).map(|at| (entity, at.distance(from)))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(entity, _)| entity)
    };
    if let Some(from) = chest(player) {
        for mut live in &mut blasts {
            if live.caster == player && live.target.is_none() {
                live.target = nearest(from);
            }
        }
        for mut live in &mut torrents {
            if live.caster == player && live.target.is_none() {
                live.target = nearest(from);
            }
        }
    }
    *frame += 1;
    if !frame.is_multiple_of(10) {
        return;
    }
    for (entity, shape, volume) in &shapes {
        let lit: Vec3 = shape.points.iter().map(|&(_, c)| c).sum::<Vec3>() + shape.colour;
        info!(
            "gi shape {entity:?}: {} {} points, atten {}, light sum {:.4?}, box {:.2?} ± {:.2?} m",
            shape.program.name(),
            shape.points.len(),
            shape.atten,
            lit,
            volume.position,
            volume.half
        );
    }
    for (entity, body) in &bodies {
        let Some(drive) = body.drive.as_ref() else {
            continue;
        };
        if f64::from(drive.pin_gain) < 0.08 || drive.balance().landing_weight() > 0.05 {
            info!(
                "struck body {entity:?}: pin gain {:.4}, landing weight {:.2}",
                drive.pin_gain,
                drive.balance().landing_weight()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::Mt19937;
    use crate::effects::blast::{BlastView, CHARGING, CHUNKS};
    use crate::effects::bubble::BubbleState;
    use crate::effects::emissive::{BLAST_COLOUR, TELEPORTER_COLOUR, TORRENT_COLOUR};
    use crate::thaum::force::BLAST_SETUP;

    fn view() -> BlastView {
        BlastView {
            target: Vec3::new(0.0, 150.0, 600.0),
            hand_node: Vec3::new(0.0, 150.0, 0.0),
            hand: Vec3::new(0.0, 150.0, 0.0),
            facing: Vec3::Z,
        }
    }

    /// A camera turned 60° about x (a proper rotation, not its own inverse).
    fn camera() -> ViewFrame {
        let (s, c) = (60f32.to_radians().sin(), 60f32.to_radians().cos());
        ViewFrame {
            x: Vec3::X,
            y: Vec3::new(0.0, c, s),
            z: Vec3::new(0.0, -s, c),
            at: Vec3::new(0.0, 650.0, -200.0),
        }
    }

    /// A charging blast's light: sixteen points near the hand, each
    /// `(0.16, 0.08, 1)·0.09·0.6` times the charge curve
    /// `(0.6 − c)·c·11.11`, attenuation 5000, and the box of the points
    /// plus 120 cm.
    #[test]
    fn a_charging_blast_lights_its_chunks() {
        let mut random = Mt19937::new(5489);
        let mut blast = Blast::new(BLAST_SETUP, 0.1, 12, &view(), &mut random);
        blast.charge = 0.1;
        assert_eq!(blast.state, CHARGING);
        blast.frame_lights(0.5, &camera());
        let (shape, volume) = blast_shape(&blast, GI_LIGHT_SCALE).expect("points");
        assert_eq!(shape.program, GiShapeProgram::Multipoint);
        assert_eq!(shape.atten, 5000.0);
        assert_eq!(shape.points.len(), CHUNKS);
        let c = f64::from(0.1f32);
        let s = ((0.6 - c) * c * 11.11) as f32;
        assert_eq!(BLAST_SETUP.light, 0.6);
        let want = BLAST_COLOUR * ((GI_LIGHT_SCALE * s) * 0.6);
        for &(at, colour) in shape.points.iter() {
            assert_eq!(colour, want);
            assert!(at.is_finite());
            // Within a metre of the hand.
            assert!((at - Vec3::new(0.0, 150.0, 0.0)).length() < 100.0, "{at}");
        }
        // The box holds every point with the 120 cm margin, in metres.
        for &(at, _) in shape.points.iter() {
            let d = (at / CM - volume.position).abs();
            assert!(d.cmple(volume.half - Vec3::splat(1.2) + 1e-4).all());
        }
        assert!(volume.half.min_element() >= 1.2);
    }

    /// A thrown chunk's light point is its bent ribbon spine's second
    /// sample, taken into the camera's space and back: it stays on the
    /// chunk's trail, a few centimetres behind the chunk, whatever the
    /// camera's turn.
    #[test]
    fn a_thrown_chunks_light_point_trails_it() {
        let mut random = Mt19937::new(5489);
        let mut blast = Blast::new(BLAST_SETUP, 0.1, 12, &view(), &mut random);
        let mut body = crate::thaum::casting::CastBody::default();
        let mut ticks = 0;
        while blast.state != THROWN && ticks < 2000 {
            blast.tick(&view(), &mut body, false, &mut random);
            ticks += 1;
        }
        assert_eq!(blast.state, THROWN);
        for _ in 0..6 {
            blast.tick(&view(), &mut body, false, &mut random);
        }
        blast.frame_lights(0.5, &camera());
        let mut checked = 0;
        for (chunk, life) in blast.chunks.iter().zip(&blast.lives) {
            if life.ticks < 1 {
                continue;
            }
            let gap = (life.light_at - chunk.at).length();
            assert!(gap < 40.0, "{} from {}", life.light_at, chunk.at);
            checked += 1;
        }
        assert!(checked > 0);
    }

    /// A thrown blast's spent chunk is dark (`f = 1 > 0.666`, strength
    /// `max(4 − 4.5, 0)² = 0`) and a fresh one lit at full strength; the
    /// attenuation is the last chunk's.
    #[test]
    fn a_thrown_blasts_spent_chunks_are_dark() {
        let mut random = Mt19937::new(5489);
        let mut blast = Blast::new(BLAST_SETUP, 0.1, 12, &view(), &mut random);
        blast.state = THROWN;
        blast.lives[0].ticks = 0;
        let (shape, _) = blast_shape(&blast, GI_LIGHT_SCALE).expect("points");
        assert_eq!(shape.points[0].1, Vec3::ZERO);
        assert_eq!(
            shape.points[1].1,
            BLAST_COLOUR * ((GI_LIGHT_SCALE * 1.0) * 0.6)
        );
        // Fresh chunks: f = 0 < 0.3, attenuation 5000 − 0·13333.
        assert_eq!(shape.atten, 5000.0);
    }

    /// A torrent's light: each gathered point coloured
    /// `(0.12, 0.06, 1)·0.09·w/4·(2 − l)·l·1.2`, attenuation 2000.
    #[test]
    fn a_torrent_lights_its_gathered_points() {
        let view = crate::effects::torrent::TorrentView {
            target: Vec3::new(0.0, 150.0, 1000.0),
            hand_node: Vec3::new(0.0, 150.0, 0.0),
            hand: Vec3::new(0.0, 150.0, 0.0),
            facing: Vec3::Z,
            turned_aim: Vec3::Z,
        };
        let mut torrent = Torrent::new(
            0.2,
            12,
            &view,
            &mut Mt19937::new(1),
            crate::thaum::engine::Roll(7),
        );
        torrent.charge = 0.5;
        torrent.lights.points = vec![
            (Vec3::new(0.0, 150.0, 100.0), 2.0, 1),
            (Vec3::new(0.0, 150.0, 300.0), 1.0, 1),
        ];
        let (shape, volume) = torrent_shape(&torrent, GI_LIGHT_SCALE);
        assert_eq!(shape.atten, 2000.0);
        let curve = (f64::from((2.0f32 - 0.5) * 0.5) * 1.2) as f32;
        assert_eq!(
            shape.points[0].1,
            TORRENT_COLOUR * ((GI_LIGHT_SCALE * (2.0 * 0.25)) * curve)
        );
        assert_eq!(volume.position, Vec3::new(0.0, 1.5, 2.0));
        assert_eq!(volume.half, Vec3::new(1.2, 1.2, 2.2));
        // No points: an empty light.
        torrent.lights.points.clear();
        let (shape, _) = torrent_shape(&torrent, GI_LIGHT_SCALE);
        assert!(shape.points.is_empty());
    }

    /// The teleporter bubble's shell: radius 120 cm unswelled, colour
    /// `(0.75, 0.8, 1)·0.09`, fade `1 − fade` while growing, and the box
    /// of the sphere widened 10 cm, rounded, plus 60.
    #[test]
    fn a_teleporter_bubble_lights_its_shell() {
        let bubble = Bubble::new(0, false, [0.25, 0.5]);
        assert_eq!(bubble.state, BubbleState::Grow);
        let at = Vec3::new(100.0, 150.0, -40.0);
        let (shape, volume) = bubble_shape(&bubble, at, 250, GI_LIGHT_SCALE);
        assert_eq!(shape.program, GiShapeProgram::TeleBubble);
        assert_eq!(shape.radius, 120.0);
        assert_eq!(shape.colour, TELEPORTER_COLOUR * GI_LIGHT_SCALE);
        assert_eq!(shape.fade, 1.0);
        assert_eq!(shape.tex_pos, Vec2::new(0.25, 0.5));
        assert_eq!(shape.matrix.transform_point3(at), Vec3::ZERO);
        assert_eq!(volume.position, at / CM);
        assert_eq!(volume.half, Vec3::splat((120.0 + 10.0 + 60.0) / CM));
    }

    #[test]
    fn the_dome_cuboid_and_tendril_lights_take_their_boxes() {
        // The Barrier's dome: its extent, colour at the manager's scale and
        // its node's world box plus 60 cm.
        let extent = Vec3::new(25.0, 45.0, 25.0);
        let light = emissive::FacedLight {
            extent,
            colour: emissive::DOME_COLOUR * 0.5,
        };
        let at = Vec3::new(100.0, 50.0, 0.0);
        let scale = Vec3::splat(85.0);
        let unturned = [Vec3::X, Vec3::Y, Vec3::Z];
        let (shape, volume) = barrier_shape(&light, at, unturned, scale);
        assert_eq!(shape.program, GiShapeProgram::Dome);
        assert_eq!(shape.half, extent);
        // The light's colour already carries the manager's scale (the
        // emissive's draw multiplies by it once).
        assert_eq!(shape.colour, emissive::DOME_COLOUR * 0.5);
        assert_eq!(shape.matrix.transform_point3(at), Vec3::ZERO);
        assert_eq!(volume.position, at / CM);
        // The bounds are the scale plus 10 rounded, the half plus 60.
        assert_eq!(volume.half, Vec3::splat((85.0 + 10.0 + 60.0) / CM));
        // A cap facing +x: its 25 cm depth lies along x, its 85 cm width
        // along z, and bMat takes a point ahead of it onto its own z.
        let turned = [Vec3::NEG_Z, Vec3::Y, Vec3::X];
        let flat = Vec3::new(85.0, 85.0, 25.0);
        let (shape, volume) = barrier_shape(&light, at, turned, flat);
        assert!(
            (shape.matrix.transform_point3(at + Vec3::X * 10.0) - Vec3::Z * 10.0).length() < 1e-4
        );
        assert_eq!(volume.position, at / CM);
        assert_eq!(
            volume.half,
            Vec3::new(25.0 + 10.0 + 60.0, 85.0 + 10.0 + 60.0, 85.0 + 10.0 + 60.0) / CM
        );
        // The Bubble's shell takes the radius plus five and the fade.
        let light = emissive::BubbleLight {
            radius: 105.0,
            fade: 0.25,
            colour: emissive::BUBBLE_COLOUR * 0.5,
        };
        let (shape, _) = force_bubble_shape(&light, at, scale);
        assert_eq!(shape.program, GiShapeProgram::Bubble);
        assert_eq!((shape.radius, shape.fade), (105.0, 0.25));
        assert_eq!(shape.half, Vec3::splat(105.0));
        // A cuboid: max(scale − 20, 10) per axis, its own colour and the
        // node's box plus 80 cm.
        let (shape, volume) = cuboid_shape(
            at,
            [Vec3::X, Vec3::Y, Vec3::Z],
            Vec3::new(25.0, 50.0, 31.0),
            Vec3::X,
        );
        assert_eq!(shape.program, GiShapeProgram::Cuboid);
        assert_eq!(shape.half, Vec3::new(10.0, 30.0, 11.0));
        assert_eq!(shape.colour, Vec3::X * GI_LIGHT_SCALE);
        assert_eq!(
            volume.half,
            Vec3::new(25.0 + 10.0 + 80.0, 50.0 + 10.0 + 80.0, 31.0 + 10.0 + 80.0) / CM
        );
        // A tendril's points make one multipoint light each, attenuation
        // 16000, in their box plus 120 cm moved by the node.
        let points = [(Vec3::ZERO, 1.0), (Vec3::new(10.0, 0.0, 0.0), 2.0)];
        let (shape, volume) = tendril_shape(&points, at, 0.5, 0.5).expect("a shape");
        assert_eq!(shape.program, GiShapeProgram::Multipoint);
        assert_eq!(shape.atten, emissive::MULTIPOINT_ATTENUATION);
        assert_eq!(shape.points.len(), 2);
        assert_eq!(
            shape.points[1].1,
            emissive::TENDRIL_COLOUR * (0.5 * 2.0 * 0.5)
        );
        // The points' middle is 5 cm out from the node's place.
        assert_eq!(volume.position, (at + Vec3::X * 5.0) / CM);
        assert_eq!(volume.half, Vec3::new(5.0 + 120.0, 120.0, 120.0) / CM);
        assert_eq!(tendril_shape(&[], at, 1.0, 1.0), None);
    }

    /// A blast's light is registered on its first frame, kept while its
    /// chunks live, and freed (despawned, never registered again) on the
    /// frame that builds nothing; a blast leaving the scene frees its light.
    #[test]
    fn effect_lights_register_update_and_leave() {
        let mut app = App::new();
        app.add_plugins(exa_render::gi_shape::GiShapePlugin)
            .init_resource::<Time<Fixed>>()
            .add_systems(
                Update,
                (blast_lights, torrent_lights, drop_orphan_lights).chain(),
            );
        app.world_mut().spawn((
            Camera3d::default(),
            GlobalTransform::from_translation(Vec3::new(0.0, 1.7, -3.0)),
        ));
        let mut random = Mt19937::new(5489);
        let blast = Blast::new(BLAST_SETUP, 0.1, 12, &view(), &mut random);
        let caster = app.world_mut().spawn_empty().id();
        let effect = app
            .world_mut()
            .spawn(LiveBlast {
                blast,
                caster,
                target: None,
                hooked: false,
                generation: 0,
            })
            .id();
        app.update();
        app.update();
        let light = app
            .world()
            .get::<EffectLight>(effect)
            .expect("registered")
            .0;
        assert_eq!(
            app.world().resource::<exa_render::gi::GiShapes>().shapes(),
            &[light]
        );
        assert_eq!(
            app.world().get::<GiShape>(light).unwrap().points.len(),
            CHUNKS
        );
        // Every chunk spent: the frame builds nothing and frees the light.
        for life in &mut app
            .world_mut()
            .get_mut::<LiveBlast>(effect)
            .unwrap()
            .blast
            .lives
        {
            life.ticks = 0;
        }
        app.update();
        app.update();
        assert!(app.world().get_entity(light).is_err());
        assert!(app.world().get::<EffectLightFreed>(effect).is_some());
        assert!(
            app.world()
                .resource::<exa_render::gi::GiShapes>()
                .shapes()
                .is_empty()
        );
        app.update();
        assert!(app.world().get::<EffectLight>(effect).is_none());
        // A lit blast that leaves the scene takes its light with it.
        let mut random = Mt19937::new(7);
        let blast = Blast::new(BLAST_SETUP, 0.1, 12, &view(), &mut random);
        let other = app
            .world_mut()
            .spawn(LiveBlast {
                blast,
                caster,
                target: None,
                hooked: false,
                generation: 0,
            })
            .id();
        app.update();
        app.update();
        let light = app.world().get::<EffectLight>(other).expect("registered").0;
        app.world_mut().despawn(other);
        app.update();
        app.update();
        assert!(app.world().get_entity(light).is_err());
        assert!(
            app.world()
                .resource::<exa_render::gi::GiShapes>()
                .shapes()
                .is_empty()
        );
    }
}
