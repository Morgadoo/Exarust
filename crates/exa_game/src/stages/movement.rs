//! The movement stage of a character's tick: the original's movement
//! decision from the character's locomotion request (walk or stop, brake,
//! the gates, the body's turn toward its heading at the original's rates,
//! the alignment of its pelvis with its heading), which its capsule
//! executes at Exarust's speeds until the body walks by its carry
//! ([`decide_movement`]); then, while the body moves, how high the lowest
//! ankle of its walk's first pose sits (its gait floor); the step its
//! controller made since the last aim, left out of the carry (the
//! controller is Exarust's locomotion until the body walks by its carry);
//! then its balance and the ground under its feet against the level.
//! Exarust measures these here, before the sampler, whose pose hold reads
//! this tick's support; the original measures them in the carry stage,
//! after its sampler.

use bevy::prelude::*;
use exa_anim::{Animator, BodyPart, Motions, Stature};
use exa_combat::swing::{Swing, SwingState};
use exa_combat::{Condition, Fighter};
use exa_formats::encumbrance::run_cap;
use exa_formats::races::BRAKE_DISTANCES;
use exa_physics::body::locomotion::{
    self, CapsuleGaits, Footing, Locomotion, Locomotor, StopDistances, TurnBy, flags, forward,
};
use exa_physics::body::{Fallen, Ragdoll, TickedCharacter};
use exa_physics::{CharacterBody, Physics};

use crate::limp::{gait_floor, sprinting};
use crate::player::PlayerBody;

/// Centimetres per metre, and body ticks a second.
const CM: f32 = 100.0;
const TICKS_A_SECOND: f32 = 83.0;
/// A human's jog and sprint thrusts a second (the gait table's rows):
/// sprinting, the capsule's run end is its run times the feral sprint
/// row's thrust over the jog's.
const JOG_THRUST: f32 = 350.0;
const SPRINT_THRUST: f32 = 472.2;

/// The capsule's gaits of a body that sprints (Exarust's own, until the
/// body walks by its carry): the run end scaled by the feral sprint row
/// (the sprint blended toward the jog by the `feral` level) over the jog.
fn sprint_gaits(gaits: CapsuleGaits, feral: f32) -> CapsuleGaits {
    let f = feral.clamp(0.0, 1.0);
    let row = SPRINT_THRUST * (1.0 - f) + JOG_THRUST * f;
    CapsuleGaits {
        run: gaits.run * row / JOG_THRUST,
        ..gaits
    }
}

/// The ticked character's movement decision, when it has a locomotion
/// request: its speed along its heading (its capsule's, in cm a tick), the
/// decision with its gates (its energy from its body, its balance read
/// steady), the turn toward its heading by the table (its swing's attack
/// law while it attacks; held while it falls or its condition is
/// helpless), the alignment, its run blend and stop distance, the
/// cooldown's count; then its capsule's velocity for the next step, its
/// yaw on its body, and what the body tick's muscles read of it (moving —
/// or stepping in Exarust's interim footwork —, the run blend, the turn
/// left).
#[allow(clippy::type_complexity)]
pub(crate) fn decide_movement(
    ticked: Res<TickedCharacter>,
    motions: Option<Res<Motions>>,
    mut controllers: Query<(&mut CharacterBody, &Transform), Without<Locomotion>>,
    mut bodies: Query<
        (
            (&mut Locomotion, &mut Locomotor, &CapsuleGaits),
            &mut Transform,
            Option<&mut Ragdoll>,
            Option<&Fighter>,
            Option<&Condition>,
            Has<Fallen>,
            Option<(&Animator, &Swing, &SwingState)>,
            Option<&PlayerBody>,
        ),
        Without<CharacterBody>,
    >,
) {
    let Ok((
        (mut req, mut body, gaits),
        mut t,
        mut ragdoll,
        fighter,
        condition,
        fallen,
        swinging,
        player,
    )) = bodies.get_mut(ticked.body)
    else {
        return;
    };
    let mut controller = ticked.character.and_then(|c| controllers.get_mut(c).ok());
    let standing = controller
        .as_ref()
        .map_or(t.translation, |(_, at)| at.translation);
    // Its speed along its heading: the capsule's, as the decision last set
    // it (its measured step stalls for a tick on the level mesh's seams,
    // which would stop a braking body short).
    let velocity = controller
        .as_ref()
        .map_or(Vec3::ZERO, |(c, _)| c.desired_velocity);
    let speed = (velocity.with_y(0.0).dot(forward(req.heading)) * CM / TICKS_A_SECOND).abs();
    let state = condition.copied().unwrap_or_default();
    let held = fallen || state.helpless();
    if held {
        req.moving = false;
    }
    let (encumbrance, scale, sprint) =
        fighter.map_or((0.0, 1.0, StopDistances::HUMAN.sprint), |f| {
            (
                f.stats.encumbrance,
                f.stats.scale,
                f.stats.sprint.brake_distance,
            )
        });
    let stops = StopDistances {
        walk: BRAKE_DISTANCES[0],
        jog: BRAKE_DISTANCES[1],
        sprint,
        back: BRAKE_DISTANCES[3],
    };
    // The balance gates read a steady body (Exarust's own): until the body
    // takes recovery steps, its standing balance can settle under 0.99
    // with nothing to restore it, which would hold it for good
    // (`Footing::balanced_by` reads the body's once it can).
    let mut at = Footing::of(ragdoll.as_deref(), standing, speed);
    at.stops = stops;
    let decided = locomotion::decide(&mut req, &mut body, &at);
    if decided.land
        && let Some(d) = ragdoll.as_mut().and_then(|r| r.drive.as_mut())
    {
        d.land();
    }
    if held {
        locomotion::hold_turn(&mut body);
    } else {
        let attack = swinging.filter(|_| state == Condition::ATTACKING).and_then(
            |(anim, swing, swinging)| {
                let clip = motions.as_deref()?.clip(anim, &anim.action)?;
                Some(swing.turn_rate(
                    anim.clip_progress(clip),
                    swinging.strength,
                    encumbrance,
                    scale,
                ))
            },
        );
        let by = TurnBy {
            free_walk: req.free_walk,
            slow: req.slow_turn,
            attack,
            encumbrance,
            scale,
            crouch: player.map_or(0.0, PlayerBody::crouch),
        };
        let rate = locomotion::turn_rate(&by, body.speed);
        let keys = req.moving && req.has(flags::KEYS);
        locomotion::turn(&mut body, req.heading, rate, scale, keys);
    }
    let facing = body.facing();
    let pelvis = ragdoll
        .as_deref()
        .map_or(facing, |r| r.pelvis_forward(facing));
    let target = locomotion::align_target(
        req.moving,
        req.has(flags::KEYS),
        forward(req.heading),
        pelvis,
        forward(req.desired),
        facing,
        encumbrance,
    );
    locomotion::align(&mut body, target);
    let level = ragdoll
        .as_deref()
        .and_then(|r| r.drive.as_ref())
        .map_or(0.0, |d| d.balance().level());
    let cap = run_cap(encumbrance, req.has(flags::SPRINT));
    locomotion::locomote(&req, &mut body, cap, &stops, level);
    locomotion::count_down(&mut body);
    // Sprinting (the latch, a stance level over 0.9 and over 3.5 cm a
    // tick), the capsule's run takes the feral sprint row.
    let gaits = if sprinting(req.has(flags::SPRINT), level, body.speed) {
        sprint_gaits(*gaits, fighter.map_or(0.0, |f| f.stats.feral))
    } else {
        *gaits
    };
    if let Some((c, _)) = controller.as_mut() {
        c.desired_velocity = if held {
            Vec3::ZERO
        } else {
            locomotion::capsule_velocity(&req, &body, &gaits, c.desired_velocity, standing)
        };
    }
    t.rotation = Quat::from_rotation_y(body.yaw);
    if let Some(d) = ragdoll.as_mut().and_then(|r| r.drive.as_mut()) {
        d.bearing.moving = body.moving() || req.stepping.is_some();
        d.bearing.run = body.run;
        d.bearing.turn = if held { 0.0 } else { req.turn_left(&body) };
    }
}

/// Gives the ticked body its gait floor while it moves, takes its
/// controller's step out of its carry and measures its balance and ground.
pub(crate) fn measure_ground(
    ticked: Res<TickedCharacter>,
    motions: Option<Res<Motions>>,
    physics: Option<Res<Physics>>,
    controllers: Query<&CharacterBody>,
    mut bodies: Query<(&mut Ragdoll, &Animator, Option<&Stature>)>,
    parents: Query<&ChildOf>,
    nodes: Query<(&Transform, Option<&BodyPart>)>,
) {
    let Some(motions) = motions else { return };
    let root = ticked.body;
    let Ok((mut body, anim, stature)) = bodies.get_mut(root) else {
        return;
    };
    let stature = stature.map_or(1.0, |s| s.0);
    if body.drive.as_ref().is_some_and(|d| d.bearing.moving) {
        let floor = gait_floor(&motions, anim, stature, &body, root, &parents, &nodes);
        if let Some(drive) = body.drive.as_mut() {
            drive.bearing.gait_floor = floor;
        }
    }
    if let Some(c) = ticked.character.and_then(|c| controllers.get(c).ok()) {
        body.followed(c.followed());
    }
    if let Some(physics) = &physics {
        body.measure_balance(&physics.0);
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_2;

    use exa_physics::body::locomotion::Stop;

    use super::*;

    /// An app running the movement decision of one character: its capsule
    /// root at the origin and its body facing +Z with a request.
    fn setup() -> (App, Entity, Entity) {
        let mut app = App::new();
        app.add_systems(Update, decide_movement);
        let root = app
            .world_mut()
            .spawn((CharacterBody::default(), Transform::default()))
            .id();
        let model = app
            .world_mut()
            .spawn((Locomotion::facing(0.0), Transform::default(), ChildOf(root)))
            .id();
        app.insert_resource(TickedCharacter {
            body: model,
            character: Some(root),
        });
        (app, root, model)
    }

    fn request(app: &mut App, model: Entity) -> Mut<'_, Locomotion> {
        app.world_mut().get_mut::<Locomotion>(model).unwrap()
    }

    fn velocity(app: &App, root: Entity) -> Vec3 {
        app.world()
            .get::<CharacterBody>(root)
            .unwrap()
            .desired_velocity
    }

    /// A free walk by the keys toward a heading 90° to the left: the capsule
    /// takes the walk along the heading (1.4 m/s), the body turns 240°/s ×
    /// 0.9 a tick (free walk at 0 cm a tick, moving by the keys) and its
    /// model takes the yaw; released, the body stops and the capsule with
    /// it.
    #[test]
    fn a_free_walk_drives_the_capsule_and_turns_the_body() {
        let (mut app, root, model) = setup();
        {
            let mut req = request(&mut app, model);
            req.heading = FRAC_PI_2;
            req.moving = true;
            req.free_walk = true;
            req.flags = flags::MOVE | flags::KEYS;
        }
        app.update();
        assert!((velocity(&app, root) - Vec3::X * 1.4).length() < 1e-5);
        let body = *app.world().get::<Locomotor>(model).unwrap();
        assert_eq!(body.stop, Stop::Moving);
        let step = locomotion::BASE_TURN * 0.9;
        assert!((body.yaw - step).abs() < 1e-6, "{}", body.yaw);
        let t = app.world().get::<Transform>(model).unwrap();
        assert!(t.rotation.angle_between(Quat::from_rotation_y(step)) < 1e-5);
        request(&mut app, model).moving = false;
        app.update();
        assert_eq!(velocity(&app, root), Vec3::ZERO);
        let stop = app.world().get::<Locomotor>(model).unwrap().stop;
        assert_eq!(stop, Stop::Standing);
    }

    /// A body down (its condition helpless) is asked for nothing, neither
    /// moves nor turns: no error, its capsule still.
    #[test]
    fn a_helpless_body_neither_moves_nor_turns() {
        let (mut app, root, model) = setup();
        app.world_mut().entity_mut(model).insert(Condition::DOWN);
        {
            let mut req = request(&mut app, model);
            req.heading = FRAC_PI_2;
            req.moving = true;
            req.free_walk = true;
        }
        app.update();
        let body = *app.world().get::<Locomotor>(model).unwrap();
        assert_eq!(
            (body.yaw, body.error, body.stop),
            (0.0, 0.0, Stop::Standing)
        );
        assert_eq!(velocity(&app, root), Vec3::ZERO);
        assert!(!app.world().get::<Locomotion>(model).unwrap().moving);
    }

    /// Standing in combat (no slow turn) the body turns 300°/s; attacking
    /// halfway through its clip with its fists, by the swing's law: (1 −
    /// 0.25·0.5)·B·0.8 a tick (168°/s).
    #[test]
    fn standing_300_degrees_and_attacking_by_the_swing_law() {
        let (mut app, _, model) = setup();
        request(&mut app, model).heading = FRAC_PI_2;
        app.update();
        let yaw = app.world().get::<Locomotor>(model).unwrap().yaw;
        assert!((yaw - locomotion::BASE_TURN * 1.25).abs() < 1e-6, "{yaw}");
        let (mut app, _, model) = setup();
        request(&mut app, model).heading = FRAC_PI_2;
        let mut anim = Animator::new("Default", "Idle");
        anim.time = 15.0;
        app.insert_resource(crate::limp::tests::motions());
        app.world_mut().entity_mut(model).insert((
            Condition::ATTACKING,
            anim,
            Swing::FISTS,
            SwingState::default(),
        ));
        app.update();
        let body = *app.world().get::<Locomotor>(model).unwrap();
        let law = Swing::FISTS.turn_rate(0.5, 1.0, 0.0, 1.0);
        assert!((body.yaw - law).abs() < 1e-6, "{} {law}", body.yaw);
        assert!((law.to_degrees() * 83.0 - 168.0).abs() < 1e-2);
    }

    /// The capsule moves at its character's own walk (the duellist's 1.2
    /// m/s) and takes the interim footwork's velocity while it steps (the
    /// body does not locomote then).
    #[test]
    fn the_capsule_takes_its_gaits_and_the_footwork() {
        let (mut app, root, model) = setup();
        app.world_mut().entity_mut(model).insert(CapsuleGaits {
            walk: 1.2,
            run: 1.2,
            back: 1.2,
        });
        {
            let mut req = request(&mut app, model);
            req.moving = true;
            req.free_walk = true;
        }
        app.update();
        assert!((velocity(&app, root) - Vec3::Z * 1.2).length() < 1e-5);
        request(&mut app, model).stepping = Some(Vec3::X * 1.4);
        app.update();
        assert_eq!(velocity(&app, root), Vec3::X * 1.4);
        let body = app.world().get::<Locomotor>(model).unwrap();
        assert_eq!(body.stop, Stop::Stepping);
    }

    /// The sprint latch tightens the run cap: at encumbrance 0.5 a run
    /// with the latch tops its blend at 1 − 0.4·0.5 = 0.8 within 40 ticks;
    /// without it (W + Shift, the latch spent) at 1 − 0.3·0.5 = 0.85.
    #[test]
    fn the_sprint_latch_tightens_the_run_cap() {
        let (mut app, _, model) = setup();
        let mut fighter = Fighter::default();
        fighter.stats.encumbrance = 0.5;
        app.world_mut().entity_mut(model).insert(fighter);
        {
            let mut req = request(&mut app, model);
            req.moving = true;
            req.free_walk = true;
            req.flags = flags::MOVE | flags::RUN | flags::SPRINT;
        }
        for _ in 0..40 {
            app.update();
        }
        let run = |app: &App| app.world().get::<Locomotor>(model).unwrap().run;
        assert_eq!(run(&app), run_cap(0.5, true));
        assert!((run(&app) - 0.8).abs() < 1e-6);
        request(&mut app, model).flags = flags::MOVE | flags::RUN | flags::KEYS;
        for _ in 0..40 {
            app.update();
        }
        assert_eq!(run(&app), run_cap(0.5, false));
        assert!((run(&app) - 0.85).abs() < 1e-6);
    }

    /// Sprinting, the capsule's run end is the jog's 3.6 m/s times the
    /// feral sprint thrust over the jog thrust: 472.2 / 350 at feral 0,
    /// the jog's own at feral 1; the walk and the backpedal keep theirs.
    #[test]
    fn a_sprint_runs_the_capsule_at_the_feral_sprint_row() {
        let gaits = CapsuleGaits::default();
        let sprint = sprint_gaits(gaits, 0.0);
        assert!(
            (sprint.run - 3.6 * 472.2 / 350.0).abs() < 1e-4,
            "{}",
            sprint.run
        );
        assert_eq!((sprint.walk, sprint.back), (gaits.walk, gaits.back));
        assert!((sprint_gaits(gaits, 1.0).run - 3.6).abs() < 1e-5);
        assert!((sprint_gaits(gaits, 0.5).run - 3.6 * 411.1 / 350.0).abs() < 1e-4);
    }

    /// Crouched beyond 0.5 (the player's crouch) the body turns at 0.6 of
    /// its rate: standing in combat 300°/s × 0.6 = 180°/s.
    #[test]
    fn a_crouched_body_turns_at_six_tenths() {
        let (mut app, _, model) = setup();
        app.world_mut()
            .entity_mut(model)
            .insert(PlayerBody::new(0.0).crouching(0.75));
        request(&mut app, model).heading = FRAC_PI_2;
        app.update();
        let yaw = app.world().get::<Locomotor>(model).unwrap().yaw;
        let step = (f64::from(locomotion::BASE_TURN * 1.25) * 0.6) as f32;
        assert!((yaw - step).abs() < 1e-6, "{yaw} {step}");
    }
}
