//! The player's hand on the world: what the left button takes hold of, the
//! drag that follows the cursor, and whether the point under the cursor is
//! within reach. Rules: `docs/PLAYER.md` (The hand).
//!
//! **The press.** A left press does nothing while combat mode keeps the
//! button for attacks (it does unless Ctrl is held or the hand already
//! holds something), while the point under the cursor is out of reach, or
//! while the player is down. Otherwise the pick under the cursor is named
//! ([`crate::pick::classify`]):
//! - a prop or item the hand can move, not while the player locomotes: it
//!   holds an object (the standing quarter turns stop) and, with Ctrl held,
//!   drags it: a grab on the part the pick hit, at the hit point, strength
//!   1, in the tick's first two passes. Without Ctrl the original lifts it
//!   onto the cursor (the item drag); Exarust has no item cursor, so the
//!   press only holds until the release;
//! - a corpse: dragged by the limb the pick hit, strength 0.5, in every
//!   pass;
//! - a door, a wheel or a sim object: their drags (not taken: Exarust's
//!   doors, wheels and cloth are not under the hand yet);
//! - anything else: refused until the release, the cursor a cross and the
//!   movement request left as it was.
//!
//! **The drag.** Every frame the grabbed point is let go once it is farther
//! than 330 cm from the player's hip (squared, in double precision), and
//! when the body is a character that lives. Otherwise, while the point under
//! the cursor is within reach, the anchor is where the cursor's ray meets
//! the level plane at the grab's height, kept within 200 cm of the hip
//! across the ground (at the grab's height), and the body is woken to 256;
//! out of reach the anchor stays on the point. The release lets go. A
//! release after the lifting press ends the object's hold; the drag's
//! release does not (the original's quirk: the quarter turns stay off until
//! a lift is released).
//!
//! **The reach** is measured each frame while the right button is not
//! walking to the cursor: a held drag is always in reach, else the pick
//! under the cursor must be within [`crate::pick::within_reach`] of the
//! hip. The player down, collapsing or dead lets go of everything and is
//! out of reach. A refused press and a lift leave the reach alone.
//!
//! The pick looks 40 m along the cursor's ray at the level (prop stand-ins
//! included) and the characters' collision spheres (the original tests each
//! object's meshes), the nearest hit first.
//!
//! Evidence: the original's left-press, drag and release handlers, the
//! per-frame request's reach and the controls' frame, read in the
//! instructions (`context/re/player-control-hand.md`).

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_combat::Condition;
use exa_physics::Physics;
use exa_physics::body::locomotion::Locomotor;
use exa_physics::body::{Ragdoll, WAKE_COUNT};
use exa_physics::prop::{PropBody, StandIns};
use exa_ui::ScriptedPointer;

use crate::bindings::{Action, Bindings};
use crate::pick::{self, Body, Pick, ReachTo, Target};
use crate::player::{ChaseCamera, Player, PlayerBody, window_ray};

/// Centimetres per metre: the original's rules are in cm.
const CM: f32 = 100.0;
/// How far the pick looks along the cursor's ray (m; the original's 4000 cm).
const PICK_RANGE: f32 = 40.0;
/// A prop's grab: strength 1, in the passes up to the second (from 0).
const PROP_STRENGTH: f32 = 1.0;
const PROP_LAST_PASS: u32 = 1;
/// A corpse's grab: strength 0.5, in every pass.
const CORPSE_STRENGTH: f32 = 0.5;
/// The grabbed point is let go beyond this squared distance (cm²) from the
/// hip, compared in double precision.
const LET_GO2: f64 = 108_900.0;
/// The anchor stays within this distance (cm) of the hip across the ground.
const ANCHOR_REACH: f32 = 200.0;
/// While dragging, the body is woken to at least this countdown.
const DRAG_WAKE: i32 = WAKE_COUNT;
/// Without a body, the hip is this high over the feet (m).
const HIP: f32 = 0.9;

/// What the left button holds.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum Holding {
    #[default]
    Nothing,
    /// A grab on a body, pulled along the plane at `height` (m).
    Drag {
        body: Entity,
        height: f32,
        /// The body is a character's (let go once it lives).
        character: bool,
    },
    /// The lifting press (the original's item drag), until the release.
    Lift,
    /// A press on something the hand does not use, until the release.
    Refused,
}

/// The hand's state between frames.
#[derive(Resource, Debug, Default)]
pub struct Hand {
    pub holding: Holding,
    /// The point under the cursor is out of reach.
    pub out_of_reach: bool,
}

impl Hand {
    /// Whether the frame's movement request is left as it was: while a
    /// press is refused or a lift holds the cursor.
    pub fn keeps_the_request(&self) -> bool {
        matches!(self.holding, Holding::Refused | Holding::Lift)
    }

    /// Whether the cursor shows the cross whatever else holds: a refused
    /// press.
    pub fn refused(&self) -> bool {
        self.holding == Holding::Refused
    }
}

/// What a left press finds, before the pick is named.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PressState {
    /// Combat mode keeps the button for attacks.
    pub kept_for_combat: bool,
    pub out_of_reach: bool,
    /// The player is down (condition bit 0x200).
    pub down: bool,
    pub ctrl: bool,
    /// The player locomotes.
    pub moving: bool,
}

/// What a left press starts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Press {
    /// Nothing happens.
    Ignored,
    /// A grab with `strength`, in the passes up to `last_pass`. `holds`: the
    /// object's hold begins (the standing quarter turns stop).
    Drag {
        strength: f32,
        last_pass: Option<u32>,
        holds: bool,
    },
    /// The lift onto the cursor; the object's hold begins.
    Lift,
    /// A door's, a wheel's or a sim object's drag (not taken in Exarust).
    Mechanism(Pick),
    Refused,
}

/// What a left press on `named` (the pick's name; `None` when it hit
/// nothing) starts in `state`.
pub fn press(state: PressState, named: Option<Pick>) -> Press {
    if state.kept_for_combat || state.out_of_reach || state.down {
        return Press::Ignored;
    }
    let Some(named) = named else {
        return Press::Ignored;
    };
    match named {
        n if n.movable() => {
            if state.moving {
                Press::Ignored
            } else if state.ctrl {
                Press::Drag {
                    strength: PROP_STRENGTH,
                    last_pass: Some(PROP_LAST_PASS),
                    holds: true,
                }
            } else {
                Press::Lift
            }
        }
        Pick::Corpse => Press::Drag {
            strength: CORPSE_STRENGTH,
            last_pass: None,
            holds: false,
        },
        Pick::Hinge | Pick::Wheel | Pick::Scene => Press::Mechanism(named),
        _ => Press::Refused,
    }
}

/// Whether the grabbed point at `point` lets go, the hip at `hip` (both
/// m): beyond 330 cm in any direction.
pub fn lets_go(point: Vec3, hip: Vec3) -> bool {
    let d = (point - hip) * CM;
    let d2 = d.x * d.x + d.y * d.y + d.z * d.z;
    f64::from(d2) > LET_GO2
}

/// The drag's anchor (m): where the cursor's ray from `origin` along `dir`
/// meets the level plane at `height`, kept within 200 cm of `hip` across
/// the ground (and then at `height`). `None` when the ray never meets the
/// plane (the original would take the division's infinity).
pub fn drag_anchor(origin: Vec3, dir: Vec3, height: f32, hip: Vec3) -> Option<Vec3> {
    let t = (height - origin.y) / dir.y;
    let at = (origin + dir * t) * CM;
    if !at.is_finite() {
        return None;
    }
    let hip = hip * CM;
    let mut off = at - hip;
    off.y = 0.0;
    let len2 = off.x * off.x + off.y * off.y + off.z * off.z;
    if len2 > ANCHOR_REACH * ANCHOR_REACH {
        let mut kept = hip + off * (ANCHOR_REACH / len2.sqrt());
        kept.y = height * CM;
        return Some(kept / CM);
    }
    Some(at / CM)
}

/// What the pick hit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Hit {
    /// The level (a mesh with no body).
    Level,
    /// A prop's part, through its stand-in.
    Prop { body: Entity, part: usize },
    /// A character's part, through its collision spheres.
    Character {
        body: Entity,
        part: usize,
        dead: bool,
    },
}

/// The pick under the cursor: the point (m) and what holds it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Picked {
    pub at: Vec3,
    pub hit: Hit,
}

/// How far along the unit ray (`origin`, `dir`) a sphere at `centre` of
/// `radius` is first met, within `max`.
fn ray_sphere(origin: Vec3, dir: Vec3, centre: Vec3, radius: f32, max: f32) -> Option<f32> {
    let to = centre - origin;
    let along = to.dot(dir);
    let miss2 = to.length_squared() - along * along;
    let r2 = radius * radius;
    if miss2 > r2 {
        return None;
    }
    let half = (r2 - miss2).sqrt();
    let t = if along - half >= 0.0 {
        along - half
    } else if along + half >= 0.0 {
        0.0
    } else {
        return None;
    };
    (t <= max).then_some(t)
}

/// The nearest character sphere along the ray: the distance, the body, the
/// part and whether it is dead.
fn character_hit<'a>(
    origin: Vec3,
    dir: Vec3,
    max: f32,
    characters: impl Iterator<Item = (Entity, &'a Ragdoll, Condition)>,
) -> Option<(f32, Hit)> {
    let mut best: Option<(f32, Hit)> = None;
    for (body, ragdoll, condition) in characters {
        for (part, p) in ragdoll.parts.iter().enumerate() {
            let at = p.transform();
            for &(centre, radius) in &p.spheres {
                let reach = best.map_or(max, |b| b.0);
                if let Some(t) = ray_sphere(origin, dir, at.transform_point3(centre), radius, reach)
                    && best.is_none_or(|b| t < b.0)
                {
                    best = Some((
                        t,
                        Hit::Character {
                            body,
                            part,
                            dead: condition.is_dead(),
                        },
                    ));
                }
            }
        }
    }
    best
}

/// The pick along the cursor's ray: the level and the props' stand-ins,
/// then the characters' spheres, the nearest first.
fn pick_along<'a>(
    ray: Ray3d,
    physics: &Physics,
    stand_ins: &StandIns,
    characters: impl Iterator<Item = (Entity, &'a Ragdoll, Condition)>,
) -> Option<Picked> {
    let (origin, dir) = (ray.origin, *ray.direction);
    let fixed = physics
        .fixed_pick(origin, dir, PICK_RANGE)
        .map(|(handle, t)| {
            let hit = stand_ins
                .owner_of(handle)
                .map_or(Hit::Level, |(body, part)| Hit::Prop { body, part });
            (t, hit)
        });
    let reach = fixed.map_or(PICK_RANGE, |f| f.0);
    let near = match (fixed, character_hit(origin, dir, reach, characters)) {
        (Some(f), Some(c)) if c.0 < f.0 => c,
        (Some(f), _) => f,
        (None, Some(c)) => c,
        (None, None) => return None,
    };
    Some(Picked {
        at: origin + dir * near.0,
        hit: near.1,
    })
}

/// The name of what the pick hit; `top_mobility` gives a prop body's top
/// part's mobility (the object's own body).
fn name_of(hit: Hit, top_mobility: impl Fn(Entity) -> Option<f32>) -> Pick {
    let target = match hit {
        Hit::Level => Target::default(),
        Hit::Character { dead, .. } => Target {
            body: Some(Body::Character { dead }),
            ..Target::default()
        },
        Hit::Prop { body, .. } => Target {
            body: Some(Body::Prop),
            mobility: top_mobility(body).unwrap_or(0.0),
            ..Target::default()
        },
    };
    pick::classify(&target)
}

/// The player's hip (m): its pelvis, or its feet raised to hip height
/// while it has no body.
fn hip_of(feet: Vec3, body: Option<&Ragdoll>) -> Vec3 {
    body.and_then(|b| b.parts.iter().find(|p| p.parent.is_none()))
        .map_or(feet + Vec3::Y * HIP, |pelvis| {
            pelvis.transform().translation.into()
        })
}

pub struct HandPlugin;

/// The hand's frame: the press, the hold and the reach.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HandSystems;

impl Plugin for HandPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Hand>().add_systems(
            Update,
            (press_hand, hold_hand, reach_hand)
                .chain()
                .in_set(HandSystems)
                .after(crate::player::chase_camera)
                .before(crate::cursor::pick_cursor),
        );
    }
}

/// The player's body and what the hand reads of it.
type PlayerParts = (
    &'static mut PlayerBody,
    Option<&'static Ragdoll>,
    Option<&'static Condition>,
    Option<&'static Locomotor>,
);

/// Every body but the player's: its condition when it is a character's,
/// and whether it is a prop's.
type OtherBodies = (
    Entity,
    &'static mut Ragdoll,
    Option<&'static Condition>,
    Has<PropBody>,
);

/// The cursor's ray, from the chase camera through the pointer.
fn cursor_ray(
    scripted: &ScriptedPointer,
    windows: &Query<&Window, With<PrimaryWindow>>,
    cameras: &Query<(&Camera, &GlobalTransform), With<ChaseCamera>>,
) -> Option<Ray3d> {
    let window = windows.single().ok()?;
    let (camera, eye) = cameras.single().ok()?;
    let at = scripted.position(window)?;
    window_ray(camera, eye, window, at)
}

/// The pick under the cursor among the level, the props and every
/// character, the player's body (`player`) included.
fn pick_with(
    ray: Option<Ray3d>,
    physics: &Physics,
    stand_ins: &StandIns,
    bodies: &Query<OtherBodies, Without<PlayerBody>>,
    player: Option<(Entity, &Ragdoll, Condition)>,
) -> Option<Picked> {
    let others = bodies.iter().filter_map(|(e, r, c, prop)| match (c, prop) {
        (Some(c), false) => Some((e, r, *c)),
        _ => None,
    });
    pick_along(ray?, physics, stand_ins, others.chain(player))
}

/// The player's body entity (the child of the player's root carrying its
/// controls).
fn player_child(
    players: &Query<(&GlobalTransform, &Children), With<Player>>,
    player_bodies: &Query<PlayerParts, Without<PropBody>>,
) -> Option<(Vec3, Entity)> {
    players.iter().find_map(|(at, children)| {
        children
            .iter()
            .find(|&c| player_bodies.contains(c))
            .map(|c| (at.translation(), c))
    })
}

/// A left press: the pick named and its interaction started.
#[allow(clippy::too_many_arguments)]
fn press_hand(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    bindings: Res<Bindings>,
    scripted: Res<ScriptedPointer>,
    physics: Res<Physics>,
    stand_ins: Res<StandIns>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<ChaseCamera>>,
    mut hand: ResMut<Hand>,
    players: Query<(&GlobalTransform, &Children), With<Player>>,
    mut player_bodies: Query<PlayerParts, Without<PropBody>>,
    mut bodies: Query<OtherBodies, Without<PlayerBody>>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Some((_, child)) = player_child(&players, &player_bodies) else {
        return;
    };
    let ctrl = bindings.pressed(&keys, Action::Interact);
    let ray = cursor_ray(&scripted, &windows, &cameras);
    let Ok((pb, own, condition, locomotor)) = player_bodies.get(child) else {
        return;
    };
    let condition = condition.copied().unwrap_or_default();
    let picked = pick_with(
        ray,
        &physics,
        &stand_ins,
        &bodies,
        own.map(|r| (child, r, condition)),
    );
    let state = PressState {
        kept_for_combat: pb.combat && !ctrl && hand.holding == Holding::Nothing,
        out_of_reach: hand.out_of_reach,
        down: condition.off_feet(),
        ctrl,
        moving: locomotor.is_some_and(Locomotor::moving),
    };
    let named = picked.map(|p| {
        name_of(p.hit, |body| {
            bodies
                .get(body)
                .ok()
                .and_then(|(_, r, ..)| r.parts.iter().find(|p| p.parent.is_none()))
                .map(|p| p.inverse_mass)
        })
    });
    // Every press starts from nothing held.
    hand.holding = Holding::Nothing;
    let Ok((mut pb, ..)) = player_bodies.get_mut(child) else {
        return;
    };
    match press(state, named) {
        Press::Ignored | Press::Mechanism(_) => {}
        Press::Refused => hand.holding = Holding::Refused,
        Press::Lift => {
            pb.hold_object(true);
            hand.holding = Holding::Lift;
        }
        Press::Drag {
            strength,
            last_pass,
            holds,
        } => {
            let Some(p) = picked else { return };
            let (body, part, character) = match p.hit {
                Hit::Prop { body, part } => (body, part, false),
                Hit::Character { body, part, .. } => (body, part, true),
                Hit::Level => return,
            };
            if holds {
                pb.hold_object(true);
            }
            if let Ok((_, mut ragdoll, ..)) = bodies.get_mut(body)
                && ragdoll.grab(part, p.at, strength, last_pass)
            {
                info!(
                    "hand: drag {} part {part} at ({:.2}, {:.2}, {:.2}), strength {strength}",
                    if character { "corpse" } else { "prop" },
                    p.at.x,
                    p.at.y,
                    p.at.z
                );
                hand.holding = Holding::Drag {
                    body,
                    height: p.at.y,
                    character,
                };
            }
        }
    }
}

/// The hold each frame: the drag follows the cursor, the release lets go.
#[allow(clippy::too_many_arguments)]
fn hold_hand(
    buttons: Res<ButtonInput<MouseButton>>,
    scripted: Res<ScriptedPointer>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<ChaseCamera>>,
    mut hand: ResMut<Hand>,
    players: Query<(&GlobalTransform, &Children), With<Player>>,
    mut player_bodies: Query<PlayerParts, Without<PropBody>>,
    mut bodies: Query<OtherBodies, Without<PlayerBody>>,
) {
    let released = buttons.just_released(MouseButton::Left);
    let player = player_child(&players, &player_bodies);
    match hand.holding {
        Holding::Nothing => {}
        Holding::Refused => {
            if released {
                hand.holding = Holding::Nothing;
            }
        }
        Holding::Lift => {
            if released {
                hand.holding = Holding::Nothing;
                if let Some((_, child)) = player
                    && let Ok((mut pb, ..)) = player_bodies.get_mut(child)
                {
                    pb.hold_object(false);
                }
            }
        }
        Holding::Drag {
            body,
            height,
            character,
        } => {
            let hip = player.and_then(|(feet, child)| {
                player_bodies
                    .get(child)
                    .ok()
                    .map(|(_, own, ..)| hip_of(feet, own))
            });
            let ray = cursor_ray(&scripted, &windows, &cameras);
            let Ok((_, mut ragdoll, condition, _)) = bodies.get_mut(body) else {
                hand.holding = Holding::Nothing;
                return;
            };
            let lives = character && condition.is_some_and(|c| !c.is_dead());
            let far = match (ragdoll.grab_point(), hip) {
                (Some(point), Some(hip)) => lets_go(point, hip),
                _ => false,
            };
            if released || lives || far {
                ragdoll.release_grab();
                hand.holding = Holding::Nothing;
                return;
            }
            if hand.out_of_reach {
                ragdoll.hold_grab_still();
            } else if let (Some(ray), Some(hip)) = (ray, hip)
                && let Some(anchor) = drag_anchor(ray.origin, *ray.direction, height, hip)
            {
                ragdoll.pull_grab_to(anchor);
                ragdoll.wake_to(DRAG_WAKE);
            }
        }
    }
}

/// The reach of the point under the cursor, as the frame's request finds
/// it.
#[allow(clippy::too_many_arguments)]
fn reach_hand(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    bindings: Res<Bindings>,
    scripted: Res<ScriptedPointer>,
    physics: Res<Physics>,
    stand_ins: Res<StandIns>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<ChaseCamera>>,
    mut hand: ResMut<Hand>,
    players: Query<(&GlobalTransform, &Children), With<Player>>,
    player_bodies: Query<PlayerParts, Without<PropBody>>,
    mut bodies: Query<OtherBodies, Without<PlayerBody>>,
) {
    let Some((feet, child)) = player_child(&players, &player_bodies) else {
        return;
    };
    let Ok((pb, own, condition, _)) = player_bodies.get(child) else {
        return;
    };
    let condition = condition.copied().unwrap_or_default();
    // Down, collapsing or dead: everything held is let go.
    if condition.fallen_or_dying() {
        if let Holding::Drag { body, .. } = hand.holding
            && let Ok((_, mut r, ..)) = bodies.get_mut(body)
        {
            r.release_grab();
        }
        hand.holding = Holding::Nothing;
        hand.out_of_reach = true;
        return;
    }
    if hand.keeps_the_request() {
        return;
    }
    // In combat mode the request runs only with Ctrl held or a hold; the
    // right button walking to the cursor shows chevrons, not the reach.
    let ctrl = bindings.pressed(&keys, Action::Interact);
    let walking = buttons.pressed(MouseButton::Right);
    if walking || (pb.combat && !ctrl && hand.holding == Holding::Nothing) {
        return;
    }
    let in_reach = match hand.holding {
        Holding::Drag { .. } => true,
        _ => {
            let hip = hip_of(feet, own);
            let ray = cursor_ray(&scripted, &windows, &cameras);
            pick_with(
                ray,
                &physics,
                &stand_ins,
                &bodies,
                own.map(|r| (child, r, condition)),
            )
            .is_some_and(|p| pick::within_reach(p.at.y * CM, (p.at - hip) * CM, ReachTo::Point))
        }
    };
    hand.out_of_reach = !in_reach;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exploring() -> PressState {
        PressState {
            kept_for_combat: false,
            out_of_reach: false,
            down: false,
            ctrl: false,
            moving: false,
        }
    }

    /// Ctrl drags a movable prop (strength 1, two passes) and holds the
    /// object; without Ctrl it is lifted; a corpse drags at 0.5 every
    /// pass; a person or the level is refused; a door is a mechanism.
    #[test]
    fn a_press_names_its_interaction() {
        let ctrl = PressState {
            ctrl: true,
            ..exploring()
        };
        assert_eq!(
            press(ctrl, Some(Pick::Loose)),
            Press::Drag {
                strength: 1.0,
                last_pass: Some(1),
                holds: true
            }
        );
        assert_eq!(
            press(ctrl, Some(Pick::Usable)),
            press(ctrl, Some(Pick::Loose))
        );
        assert_eq!(press(exploring(), Some(Pick::Loose)), Press::Lift);
        for state in [exploring(), ctrl] {
            assert_eq!(
                press(state, Some(Pick::Corpse)),
                Press::Drag {
                    strength: 0.5,
                    last_pass: None,
                    holds: false
                }
            );
        }
        assert_eq!(press(exploring(), Some(Pick::Person)), Press::Refused);
        assert_eq!(press(exploring(), Some(Pick::Nothing)), Press::Refused);
        assert_eq!(press(exploring(), Some(Pick::Crank)), Press::Refused);
        assert_eq!(
            press(exploring(), Some(Pick::Hinge)),
            Press::Mechanism(Pick::Hinge)
        );
        assert_eq!(press(exploring(), None), Press::Ignored);
    }

    /// Combat mode's hold on the button, an out-of-reach point and a down
    /// player ignore the press; moving ignores a prop but not a corpse.
    #[test]
    fn a_press_is_ignored_when_the_hand_is_not_free() {
        for blocked in [
            PressState {
                kept_for_combat: true,
                ..exploring()
            },
            PressState {
                out_of_reach: true,
                ..exploring()
            },
            PressState {
                down: true,
                ..exploring()
            },
        ] {
            assert_eq!(press(blocked, Some(Pick::Corpse)), Press::Ignored);
            assert_eq!(press(blocked, Some(Pick::Person)), Press::Ignored);
        }
        let moving = PressState {
            moving: true,
            ctrl: true,
            ..exploring()
        };
        assert_eq!(press(moving, Some(Pick::Loose)), Press::Ignored);
        assert!(matches!(
            press(moving, Some(Pick::Corpse)),
            Press::Drag { .. }
        ));
    }

    /// The point lets go beyond 330 cm in any direction.
    #[test]
    fn the_drag_lets_go_beyond_330_cm() {
        // 330 cm squared is 108 900 cm²: on it holds, a hair beyond lets go.
        let hip = Vec3::ZERO;
        assert!(!lets_go(Vec3::new(3.3, 0.0, 0.0), hip));
        assert!(lets_go(Vec3::new(3.301, 0.0, 0.0), hip));
        assert!(lets_go(Vec3::new(2.0, 2.0, 2.0), hip));
        assert!(!lets_go(Vec3::new(0.0, -3.3, 0.0), hip));
        let up = Vec3::new(1.0, 1.0, 1.0);
        assert!(lets_go(up + Vec3::new(1.0, 3.2, 0.0), up));
    }

    /// The anchor is the ray's point on the plane at the grab's height,
    /// within 200 cm of the hip across the ground.
    #[test]
    fn the_anchor_follows_the_cursor_within_200_cm() {
        let hip = Vec3::new(0.0, 0.9, 0.0);
        // Straight down onto the plane at 0.3 m, 1.5 m away: kept.
        let a = drag_anchor(Vec3::new(1.5, 10.0, 0.0), Vec3::NEG_Y, 0.3, hip).unwrap();
        assert!((a - Vec3::new(1.5, 0.3, 0.0)).length() < 1e-5, "{a}");
        // 3 m and 4 m away (5 m): pulled to 2 m along it, at the height.
        let a = drag_anchor(Vec3::new(3.0, 10.0, 4.0), Vec3::NEG_Y, 0.3, hip).unwrap();
        assert!((a - Vec3::new(1.2, 0.3, 1.6)).length() < 1e-5, "{a}");
        // A level ray never meets it.
        assert_eq!(drag_anchor(Vec3::Y, Vec3::X, 0.3, hip), None);
    }

    #[test]
    fn a_ray_meets_the_near_side_of_a_sphere() {
        let t = ray_sphere(Vec3::ZERO, Vec3::X, Vec3::new(5.0, 0.3, 0.0), 0.5, 40.0).unwrap();
        assert!((t - (5.0 - 0.4)).abs() < 1e-5, "{t}");
        assert_eq!(
            ray_sphere(Vec3::ZERO, Vec3::X, Vec3::new(5.0, 0.6, 0.0), 0.5, 40.0),
            None
        );
        assert_eq!(
            ray_sphere(Vec3::ZERO, Vec3::X, Vec3::new(-5.0, 0.0, 0.0), 0.5, 40.0),
            None
        );
        assert_eq!(
            ray_sphere(Vec3::ZERO, Vec3::X, Vec3::new(5.0, 0.0, 0.0), 0.5, 4.0),
            None
        );
    }

    /// A body of one part with one 20 cm sphere at `at`.
    fn ball_body(at: Vec3) -> Ragdoll {
        Ragdoll::rigid(&[exa_physics::body::RigidPart {
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: bevy::math::Affine3A::from_translation(at),
            rest: [
                Vec3::new(-0.2, 0.0, -0.2),
                Vec3::new(0.2, 0.0, -0.2),
                Vec3::new(0.0, 0.0, 0.2),
                Vec3::new(0.0, 0.4, 0.0),
            ],
            point_masses: [4.0; 4],
            mobility: 1.0,
            spheres: vec![(Vec3::ZERO, 0.2)],
            origin: Vec3::ZERO,
            links: Vec::new(),
        }])
    }

    /// With nothing fixed along the ray, the nearest character sphere is the
    /// pick: a corpse's limb when it is dead.
    #[test]
    fn the_pick_meets_the_nearest_character() {
        let near = Entity::from_raw_u32(1).unwrap();
        let far = Entity::from_raw_u32(2).unwrap();
        let (a, b) = (
            ball_body(Vec3::new(0.0, 0.0, 3.0)),
            ball_body(Vec3::Z * 6.0),
        );
        let ray = Ray3d::new(Vec3::ZERO, Dir3::Z);
        let physics = Physics::default();
        let stand_ins = StandIns::default();
        let bodies = [(far, &b, Condition::NORMAL), (near, &a, Condition::DEAD)];
        let picked = pick_along(ray, &physics, &stand_ins, bodies.into_iter()).unwrap();
        assert!((picked.at.z - 2.8).abs() < 1e-4, "{}", picked.at);
        assert_eq!(
            picked.hit,
            Hit::Character {
                body: near,
                part: 0,
                dead: true
            }
        );
        assert_eq!(name_of(picked.hit, |_| None), Pick::Corpse);
        let aside = Ray3d::new(Vec3::X, Dir3::Z);
        let bodies = [(near, &a, Condition::DEAD)];
        assert_eq!(
            pick_along(aside, &physics, &stand_ins, bodies.into_iter()),
            None
        );
        // A prop's name reads its top part's mobility.
        let prop = Hit::Prop {
            body: near,
            part: 0,
        };
        assert_eq!(name_of(prop, |_| Some(0.5)), Pick::Loose);
        assert_eq!(name_of(prop, |_| Some(0.05)), Pick::Nothing);
        assert_eq!(name_of(Hit::Level, |_| None), Pick::Nothing);
    }

    /// The hand's systems run together in one frame (their queries do not
    /// clash), and with no player they leave the hand alone.
    #[test]
    fn the_hand_systems_run_without_a_player() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<Bindings>()
            .init_resource::<ScriptedPointer>()
            .init_resource::<StandIns>()
            .insert_resource(Physics::default())
            .init_resource::<Hand>()
            .add_systems(Update, (press_hand, hold_hand, reach_hand).chain());
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        let hand = app.world().resource::<Hand>();
        assert_eq!(hand.holding, Holding::Nothing);
        assert!(!hand.out_of_reach);
    }

    /// The hand's state keeps the request while a press is refused or a
    /// lift holds, and shows the cross only for the refusal.
    #[test]
    fn a_refused_press_keeps_the_request_and_shows_the_cross() {
        let mut hand = Hand::default();
        assert!(!hand.keeps_the_request() && !hand.refused());
        hand.holding = Holding::Refused;
        assert!(hand.keeps_the_request() && hand.refused());
        hand.holding = Holding::Lift;
        assert!(hand.keeps_the_request() && !hand.refused());
        hand.holding = Holding::Drag {
            body: Entity::PLACEHOLDER,
            height: 0.0,
            character: false,
        };
        assert!(!hand.keeps_the_request());
    }
}
