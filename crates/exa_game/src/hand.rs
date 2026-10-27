//! The player's hand on the world: what the left button takes hold of, the
//! drag that follows the cursor, and whether the point under the cursor is
//! within reach. Rules: `docs/PLAYER.md` (The hand).
//!
//! **The press** is the left button's drag start: the pointer, the button
//! held, moves from where it was pressed by more than 2 pixels on either
//! axis, or within the first 190 ms by more than 2 × ((200 − t)/20)
//! pixels, t the milliseconds since the press (so 20 at once). It does
//! nothing while combat mode keeps the button for attacks (it does unless
//! Ctrl is held or the hand already holds something), while the point under
//! the cursor is out of reach, or while the player is down. Otherwise the
//! pick at the pressed point is named ([`crate::pick::classify`]):
//! - a prop or item the hand can move, not while the player locomotes:
//!   while the player's kinesis waits for its pick, the lift hands the prop
//!   to the kinesis instead (`crate::thaum::kinesis_live`). While it waits
//!   the kinesis pick alone answers the reach ([`press_out_of_reach`]): a
//!   prop the hand can move is in reach wherever it is, anything else out
//!   of it. Otherwise it
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
//! **A click** (released before the drag starts) turns the player, outside
//! combat mode, toward the cursor's point once it is farther than 70 cm:
//! the heading takes the cursor's direction. (The original then names or
//! uses what was clicked; Exarust has no description to show yet.)
//!
//! **The reach** is measured each frame: outside combat mode while the
//! right button is not walking to the cursor, in combat mode only with Ctrl
//! held or something held (the right button aside). A held drag is always in reach, else the pick
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
use exa_physics::body::locomotion::{Locomotion, Locomotor};
use exa_physics::body::{Ragdoll, WAKE_COUNT};
use exa_physics::prop::{PropBody, StandIns};
use exa_ui::ScriptedPointer;

use crate::bindings::{Action, Bindings};
use crate::gui_screens::ScreenRequest;
use crate::pick::{self, Body, Pick, ReachTo, Target};
use crate::player::{ChaseCamera, Player, PlayerBody, window_ray};
use crate::player_controls::{self, CharacterInteraction, ClickOutcome, ClickTarget};
use crate::thaum::kinesis_live::{KinesisPickHook, KinesisPicked};
use exa_ui::hud::HudShowUntil;

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
/// The pointer moves more than this many pixels from the press to start a
/// drag, and the early allowance's step and span (ms).
const DRAG_PIXELS: i64 = 2;
const EARLY_MS: i64 = 191;
const EARLY_FROM_MS: i64 = 200;
const EARLY_STEP_MS: i64 = 20;
/// A click turns the player toward a cursor farther than this (cm).
const TURN_BEYOND_CM: f32 = 70.0;

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
    /// A door's, a wheel's or a cloth's drag, until the release. Unlike a
    /// lift or a refusal it leaves the movement request to the keys and the
    /// right button, as the original's door, wheel and cloth drags do.
    Mechanism(Pick),
    /// A press on something the hand does not use, until the release.
    Refused,
}

/// The hand's state between frames.
#[derive(Resource, Debug, Default)]
pub struct Hand {
    pub holding: Holding,
    /// The point under the cursor is out of reach.
    pub out_of_reach: bool,
    /// The left button's press, until its release.
    pub left: Option<LeftPress>,
    /// The previous press's pointer and real time (ms), for the double
    /// click test.
    pub last_click: Option<(IVec2, i64)>,
}

/// Where and when the left button went down, and whether its drag started.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LeftPress {
    /// The pointer (logical window pixels) and in window pixels.
    pub at: Vec2,
    pub pixels: IVec2,
    /// Real time (ms).
    pub ms: i64,
    pub dragging: bool,
    /// The press was the second of a double click.
    pub double: bool,
}

/// The pointer's allowance (pixels) `elapsed` ms after the press: 2 × ((200
/// − t) / 20) within 190 ms (integer division), else 2.
pub fn drag_threshold(elapsed: i64) -> i64 {
    if elapsed < EARLY_MS {
        (EARLY_FROM_MS - elapsed) / EARLY_STEP_MS * DRAG_PIXELS
    } else {
        DRAG_PIXELS
    }
}

/// Whether the pointer at `now` starts the drag of a press at `from`,
/// `elapsed` ms after it.
pub fn starts_drag(from: IVec2, now: IVec2, elapsed: i64) -> bool {
    let limit = drag_threshold(elapsed);
    let dx = (i64::from(now.x) - i64::from(from.x)).abs();
    let dy = (i64::from(now.y) - i64::from(from.y)).abs();
    dx > limit || dy > limit
}

/// Whether a click turns the player toward a cursor `distance_cm` away.
pub fn click_turns(distance_cm: f32) -> bool {
    TURN_BEYOND_CM < distance_cm
}

impl Hand {
    /// The drag state word the press left.
    pub fn drag_state(&self) -> DragState {
        match self.holding {
            Holding::Nothing => DragState::Idle,
            Holding::Lift => DragState::Lift,
            Holding::Drag { .. } => DragState::Physical,
            Holding::Mechanism(Pick::Hinge) => DragState::Door,
            Holding::Mechanism(Pick::Wheel) => DragState::Wheel,
            Holding::Mechanism(_) => DragState::Cloth,
            Holding::Refused => DragState::Refused,
        }
    }

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

/// Whether a grabbed box may be taken: every placed corner is at most 280 cm
/// high and at least −1000 cm (the original refuses a box outside those; an
/// unordered height passes).
pub fn box_ok(corners: impl IntoIterator<Item = Vec3>) -> bool {
    for corner in corners {
        let y = corner.y * CM;
        if y > 280.0 || y < -1000.0 {
            return false;
        }
    }
    true
}

/// The same test on a box in its own frame: the eight corners `centre ±
/// half` are placed by the affine `frame` (rotation and translation), in
/// the original's order (x, then y, then z, the low corner before the high
/// each time), and the first corner over 280 cm or under −1000 cm refuses
/// the box. The placed height sums as the original's point transform does:
/// `(x·r0 + y·r1) + (z·r2 + t)`.
pub fn box_ok_local(centre: Vec3, half: Vec3, frame: bevy::math::Affine3A) -> bool {
    let low = centre - half;
    let high = centre + half;
    let xs = [low.x, high.x];
    let ys = [low.y, high.y];
    let zs = [low.z, high.z];
    for &x in &xs {
        for &y in &ys {
            for &z in &zs {
                let m = frame.matrix3;
                let placed_y =
                    (x * m.x_axis.y + y * m.y_axis.y) + (z * m.z_axis.y + frame.translation.y);
                let height = placed_y * CM;
                if height > 280.0 || height < -1000.0 {
                    return false;
                }
            }
        }
    }
    true
}

/// The controls' drag state word, as the hand's press leaves it. The bits
/// name the handler the release calls; a door's rattle may add the refused
/// bit while it is latched.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DragState {
    #[default]
    Idle,
    /// An item is being used on the world.
    Use,
    /// A GUI item drag is pending.
    GuiPending,
    /// A prop is lifted onto the cursor.
    Lift,
    /// A door or a lever is pulled.
    Door,
    /// A prop or a corpse is dragged.
    Physical,
    /// A wheel is turned.
    Wheel,
    /// A sim object (cloth) is pulled.
    Cloth,
    /// A GUI callback's drag.
    GuiCallback,
    /// The press was refused.
    Refused,
}

impl DragState {
    /// The state word the original keeps.
    pub fn word(self) -> u32 {
        match self {
            DragState::Idle => 0,
            DragState::Use => 0x100,
            DragState::GuiPending => 0x1000,
            DragState::Lift => 0x2000,
            DragState::Door => 0x4000,
            DragState::Physical => 0x8000,
            DragState::Wheel => 0x10000,
            DragState::Cloth => 0x20000,
            DragState::GuiCallback => 0x40000,
            DragState::Refused => 0x80000,
        }
    }

    /// The state a word names. The release dispatch tests `0x3000` (the
    /// GUI item drag and the lift), then the door, physical, wheel, GUI and
    /// cloth bits, so the more specific states come first.
    pub fn from_word(word: u32) -> Self {
        if word & 0x3000 != 0 {
            if word & 0x2000 != 0 {
                DragState::Lift
            } else {
                DragState::GuiPending
            }
        } else if word & 0x4000 != 0 {
            DragState::Door
        } else if word & 0x8000 != 0 {
            DragState::Physical
        } else if word & 0x10000 != 0 {
            DragState::Wheel
        } else if word & 0x40000 != 0 {
            DragState::GuiCallback
        } else if word & 0x20000 != 0 {
            DragState::Cloth
        } else if word & 0x80000 != 0 {
            DragState::Refused
        } else if word & 0x100 != 0 {
            DragState::Use
        } else {
            DragState::Idle
        }
    }
}

/// The distance (cm) a closed door is pulled toward the cursor, out of its
/// own hinge-side point.
pub const DOOR_PULL_CM: f32 = 100.0;
/// A door's drag link flags, its pass limit and its slack.
pub const DOOR_LINK_FLAGS: u32 = 0x4000;
pub const DOOR_LINK_LIMIT: u32 = 2;
pub const DOOR_LINK_SLACK: f32 = 0.003;

/// A door's drag as the press records it: where its pull started, whether
/// the leaf slides, and whether it was latched.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DoorGrip {
    /// The point the pull started from: the hinge-side point of a closing
    /// door, or the hit point of a sliding one.
    pub anchor: Vec3,
    pub sliding: bool,
    /// The door was latched and rattled instead of dragging.
    pub latched: bool,
}

/// The rattle of a latched door: its volume and pitch, drawn in that order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rattle {
    pub volume: f32,
    pub pitch: f32,
}

/// A latched door's rattle: pitch `0.8 + 0.2·draw`, then volume `0.15 +
/// 0.1·draw`, each draw a unit random. Returns `None` when the door is
/// already being forced open.
pub fn door_rattle(latched: bool, forcing: bool, mut draw: impl FnMut() -> f32) -> Option<Rattle> {
    if !latched || forcing {
        return None;
    }
    let pitch = draw() * 0.2 + 0.8;
    let volume = draw() * 0.1 + 0.15;
    Some(Rattle { volume, pitch })
}

/// A ray's point on the horizontal plane at `height` (m): `origin +
/// dir·((height − origin.y)/dir.y)`. A ray parallel to the plane gives no
/// finite point.
pub fn plane_pick(origin: Vec3, dir: Vec3, height: f32) -> Option<Vec3> {
    let at = origin + dir * ((height - origin.y) / dir.y);
    at.is_finite().then_some(at)
}

/// A door's dragged anchor (m). A closing leaf is pulled 100 cm from its
/// hinge-side point toward the cursor's point, wherever the cursor is. A
/// sliding leaf's point is projected onto its own sliding axis `axis`
/// (unit, across its frame), so it only moves along the slide.
pub fn door_pull_anchor(grip: DoorGrip, cursor_point: Vec3, axis: Vec3) -> Vec3 {
    let delta = cursor_point - grip.anchor;
    if grip.sliding {
        grip.anchor + axis * delta.dot(axis)
    } else {
        grip.anchor + delta * (1.0 / delta.length())
    }
}

/// A wheel's asked angle after a frame: the gap `wanted − current` is taken
/// as no turn when it is negative and stops at `limit` when it is longer;
/// the current angle plus that turn is then wrapped into ±π, compared in
/// double precision against the double π and moved by the float 2π. The
/// gap is not wrapped before the clamp, as the original computes it, and a
/// NaN gap passes both clamps.
pub fn wheel_turn(current: f32, wanted: f32, limit: f32) -> f32 {
    let delta = wanted - current;
    let delta = if delta < 0.0 {
        0.0
    } else if delta > limit {
        limit
    } else {
        delta
    };
    let mut out = current + delta;
    if f64::from(out) < -std::f64::consts::PI {
        out += std::f32::consts::TAU;
    } else if f64::from(out) > std::f64::consts::PI {
        out -= std::f32::consts::TAU;
    }
    out
}

/// Whether the first wheel grip is the one the view picks: its direction
/// from the wheel, on the camera's forward axis, is at least 0.2 in
/// absolute value; otherwise the second grip is used.
pub fn uses_first_wheel_grip(alignment: f32) -> bool {
    alignment.abs() >= 0.2
}

/// The cloth vertex a press grabbed: the first of the cloth's local
/// vertices within `radius` (m) of the press point in the cloth's frame.
pub fn cloth_vertex(vertices: &[Vec3], at: Vec3, radius: f32) -> Option<usize> {
    let r2 = radius * radius;
    vertices
        .iter()
        .position(|v| (*v - at).length_squared() < r2)
}

/// A cloth drag's anchor each frame: the grabbed vertex while the pick
/// still finds it within `radius` (m), else the ray's point on the drag
/// plane.
pub fn cloth_drag_anchor(
    vertex: Option<Vec3>,
    picked: Option<Vec3>,
    plane: Vec3,
    radius: f32,
) -> Vec3 {
    match picked {
        Some(p) if vertex.is_some_and(|v| (v - p).length_squared() < radius * radius) => p,
        _ => plane,
    }
}

/// What the press knows of the mechanism under the cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MechanismState {
    /// The body's latched byte.
    pub latched: bool,
    /// The body is being forced, so a latch does not rattle.
    pub forcing: bool,
    /// The leaf slides instead of hinging.
    pub sliding: bool,
    /// The node has a body under it.
    pub has_body: bool,
}

/// The mechanism press's branch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MechanismPress {
    /// No body under the node, or a latched door that refuses the pull.
    Refused,
    /// A latched door rattled; `still_drags` is true for a sliding leaf,
    /// whose drag continues after the rattle.
    Rattle {
        rattle: Rattle,
        still_drags: bool,
    },
    /// A door (or lever) drag starts at this grip.
    Door(DoorGrip),
    Wheel,
    Cloth,
}

/// The press on a mechanism, in the original's order: a latched closing
/// door rattles and refuses (a sliding one rattles and still drags); no
/// body refuses; else the door, wheel or cloth drag starts.
pub fn mechanism_press(
    pick: Pick,
    node: Vec3,
    hit: Vec3,
    state: MechanismState,
    mut draw: impl FnMut() -> f32,
) -> MechanismPress {
    match pick {
        Pick::Hinge => {
            if let Some(rattle) = door_rattle(state.latched, state.forcing, &mut draw) {
                return MechanismPress::Rattle {
                    rattle,
                    still_drags: state.sliding,
                };
            }
            if !state.has_body {
                return MechanismPress::Refused;
            }
            // The press stores the hinge-side point 100 cm toward the hit;
            // a sliding leaf stores the hit itself.
            let anchor = if state.sliding {
                hit
            } else {
                node + (hit - node).normalize_or_zero()
            };
            MechanismPress::Door(DoorGrip {
                anchor,
                sliding: state.sliding,
                latched: state.latched,
            })
        }
        Pick::Wheel => MechanismPress::Wheel,
        Pick::Scene => MechanismPress::Cloth,
        _ => MechanismPress::Refused,
    }
}

/// The handler the release dispatch picks for a drag state word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseHandler {
    None,
    /// The lift's end (the item cursor).
    Lift,
    Door,
    Physical,
    Wheel,
    GuiCallback,
    Cloth,
}

/// The release handler, in the original's test order: the two GUI/lift
/// bits together, then the door, physical, wheel, GUI and cloth bits.
pub fn release_handler(word: u32) -> ReleaseHandler {
    if word & 0x3000 != 0 {
        ReleaseHandler::Lift
    } else if word & 0x4000 != 0 {
        ReleaseHandler::Door
    } else if word & 0x8000 != 0 {
        ReleaseHandler::Physical
    } else if word & 0x10000 != 0 {
        ReleaseHandler::Wheel
    } else if word & 0x40000 != 0 {
        ReleaseHandler::GuiCallback
    } else if word & 0x20000 != 0 {
        ReleaseHandler::Cloth
    } else {
        ReleaseHandler::None
    }
}

/// Whether a press finds its point out of reach. While the player's
/// kinesis waits for its pick the original asks the kinesis's pick callback
/// instead of measuring: the point is in reach exactly when the kinesis can
/// take it, wherever it is. Otherwise the hand's own measure (`measured`)
/// answers. [inf: the callback tests the cursor ray against the scene's
/// kinesis nodes; Exarust takes a prop the hand can move.]
pub fn press_out_of_reach(
    measured: bool,
    waiting: bool,
    hit: Option<Hit>,
    named: Option<Pick>,
) -> bool {
    if waiting {
        !(matches!(hit, Some(Hit::Prop { .. })) && named.is_some_and(Pick::movable))
    } else {
        measured
    }
}

/// Whether the frame measures the reach: in combat mode only with Ctrl
/// held or something held, whatever the right button does; outside it while
/// the right button is not walking to the cursor (it shows chevrons then).
pub fn measures_reach(combat: bool, ctrl: bool, holding: bool, walking: bool) -> bool {
    if combat { ctrl || holding } else { !walking }
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
pub(crate) fn character_hit<'a>(
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
    real: Res<Time<Real>>,
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
    kinesis: Option<Res<KinesisPickHook>>,
    mut kinesis_picks: Option<ResMut<Messages<KinesisPicked>>>,
) {
    let kinesis = kinesis.and_then(|k| k.0);
    let now = real.elapsed().as_millis() as i64;
    let pointer = windows.single().ok().and_then(|w| {
        scripted
            .position(w)
            .map(|p| (p, (p * w.scale_factor()).as_ivec2()))
    });
    if buttons.just_pressed(MouseButton::Left) {
        let double = pointer.is_some_and(|(_, px)| {
            hand.last_click.is_some_and(|(was, ms)| {
                player_controls::is_double_click(
                    (was.x, was.y),
                    ms as u64,
                    (px.x, px.y),
                    now as u64,
                )
            })
        });
        hand.left = pointer.map(|(at, pixels)| LeftPress {
            at,
            pixels,
            ms: now,
            dragging: false,
            double,
        });
        if let Some((_, pixels)) = pointer {
            hand.last_click = Some((pixels, now));
        }
        return;
    }
    let Some(mut left) = hand.left.filter(|l| !l.dragging) else {
        return;
    };
    let moved = pointer.is_some_and(|(_, px)| starts_drag(left.pixels, px, now - left.ms));
    if !buttons.pressed(MouseButton::Left) || !moved {
        return;
    }
    left.dragging = true;
    hand.left = Some(left);
    let Some((_, child)) = player_child(&players, &player_bodies) else {
        return;
    };
    let ctrl = bindings.pressed(&keys, Action::Interact);
    let ray = windows
        .single()
        .ok()
        .zip(cameras.single().ok())
        .and_then(|(window, (camera, eye))| window_ray(camera, eye, window, left.at));
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
    let named = picked.map(|p| {
        name_of(p.hit, |body| {
            bodies
                .get(body)
                .ok()
                .and_then(|(_, r, ..)| r.parts.iter().find(|p| p.parent.is_none()))
                .map(|p| p.inverse_mass)
        })
    });
    let state = PressState {
        kept_for_combat: pb.combat && !ctrl && hand.holding == Holding::Nothing,
        out_of_reach: press_out_of_reach(
            hand.out_of_reach,
            kinesis.is_some(),
            picked.map(|p| p.hit),
            named,
        ),
        down: condition.off_feet(),
        ctrl,
        moving: locomotor.is_some_and(Locomotor::moving),
    };
    // Every press starts from nothing held.
    hand.holding = Holding::Nothing;
    let Ok((mut pb, ..)) = player_bodies.get_mut(child) else {
        return;
    };
    match press(state, named) {
        Press::Ignored => {}
        // Exarust's doors, wheels and cloth are not bodies under the hand:
        // the press records the drag until the release.
        Press::Mechanism(pick) => hand.holding = Holding::Mechanism(pick),
        Press::Refused => hand.holding = Holding::Refused,
        Press::Lift => {
            if let (
                Some(caster),
                Some(Picked {
                    hit: Hit::Prop { body, .. },
                    ..
                }),
            ) = (kinesis, picked)
            {
                if let Some(messages) = kinesis_picks.as_deref_mut() {
                    messages.write(KinesisPicked { caster, body });
                }
                return;
            }
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
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    bindings: Res<Bindings>,
    physics: Res<Physics>,
    stand_ins: Res<StandIns>,
    scripted: Res<ScriptedPointer>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<ChaseCamera>>,
    mut hand: ResMut<Hand>,
    players: Query<(&GlobalTransform, &Children), With<Player>>,
    mut player_bodies: Query<PlayerParts, Without<PropBody>>,
    mut requests: Query<&mut Locomotion, With<PlayerBody>>,
    mut bodies: Query<OtherBodies, Without<PlayerBody>>,
    mut screens: Option<ResMut<Messages<ScreenRequest>>>,
    (time, mut show_bars): (Option<Res<Time<Virtual>>>, Option<ResMut<HudShowUntil>>),
) {
    let released = buttons.just_released(MouseButton::Left);
    let player = player_child(&players, &player_bodies);
    // A release before the drag started is a click: outside combat mode
    // the player turns toward the cursor, and a click on the player's own
    // body shows the health and focus bars for 5000 ms. A double click
    // (with Ctrl held in combat mode) then interacts with what is there.
    // The original takes the double click on the second press; Exarust
    // reads it on that press's release.
    if released {
        let left = hand.left.take();
        if let Some(left) = left.filter(|l| !l.dragging)
            && let Some((_, child)) = player
            && let Ok((pb, own, condition, _)) = player_bodies.get(child)
        {
            let combat = pb.combat;
            if !combat
                && let Some((yaw, distance)) = pb.cursor()
                && click_turns(distance * CM)
                && let Ok(mut req) = requests.get_mut(child)
            {
                req.heading = yaw;
            }
            let interact = left.double && (!combat || bindings.pressed(&keys, Action::Interact));
            if !combat || interact {
                let condition = condition.copied().unwrap_or_default();
                let ray =
                    windows.single().ok().zip(cameras.single().ok()).and_then(
                        |(window, (camera, eye))| window_ray(camera, eye, window, left.at),
                    );
                let picked = pick_with(
                    ray,
                    &physics,
                    &stand_ins,
                    &bodies,
                    own.map(|r| (child, r, condition)),
                );
                let target = picked.and_then(|picked| match picked.hit {
                    Hit::Character { body, .. } if body == child => Some(ClickTarget::Player),
                    Hit::Character { dead, .. } if dead => Some(ClickTarget::DeadCharacter),
                    Hit::Character { .. } => Some(ClickTarget::LivingCharacter),
                    Hit::Prop { .. } => Some(ClickTarget::plain()),
                    Hit::Level => None,
                });
                if let Some(target) = target {
                    if !combat {
                        let outcome = player_controls::click(target);
                        debug!("click {target:?}: {outcome:?}");
                        if let ClickOutcome::ShowBars { ms } = outcome
                            && let Some(show) = show_bars.as_deref_mut()
                            && let Some(time) = time.as_deref()
                        {
                            show.show_for(time.elapsed().as_millis() as i64, i64::from(ms));
                        }
                    }
                    if interact {
                        let interaction = player_controls::interact_character(target, false, false);
                        debug!("double click {target:?}: {interaction:?}");
                        // The player's own body opens Exarust's inventory
                        // screen; the loot and talk windows have no
                        // Exarust form yet.
                        if interaction == CharacterInteraction::Inventory
                            && let Some(messages) = screens.as_deref_mut()
                        {
                            messages.write(ScreenRequest {
                                screen: exa_ui::game_gui::GameScreen::Inventory,
                                player_ready: !condition.dying(),
                                player_alive: !condition.is_dead(),
                            });
                        }
                    }
                }
            }
        }
    }
    match hand.holding {
        Holding::Nothing => {}
        Holding::Refused => {
            if released {
                hand.holding = Holding::Nothing;
            }
        }
        Holding::Mechanism(_) => {
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
    let ctrl = bindings.pressed(&keys, Action::Interact);
    let walking = buttons.pressed(MouseButton::Right);
    if !measures_reach(pb.combat, ctrl, hand.holding != Holding::Nothing, walking) {
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

    /// The drag starts past 2 pixels, or early on past 2 × ((200 − t)/20):
    /// 20 at once, 2 from 190 ms; a click turns past 70 cm.
    #[test]
    fn the_drag_starts_past_its_allowance() {
        assert_eq!(drag_threshold(0), 20);
        assert_eq!(drag_threshold(19), 18);
        assert_eq!(drag_threshold(100), 10);
        assert_eq!(drag_threshold(181), 0);
        assert_eq!(drag_threshold(190), 0);
        assert_eq!(drag_threshold(191), 2);
        let at = IVec2::new(500, 300);
        assert!(!starts_drag(at, IVec2::new(520, 300), 0));
        assert!(starts_drag(at, IVec2::new(521, 300), 0));
        assert!(!starts_drag(at, IVec2::new(502, 298), 500));
        assert!(starts_drag(at, IVec2::new(500, 303), 500));
        assert!(starts_drag(at, IVec2::new(501, 300), 185));
        assert!(!click_turns(70.0));
        assert!(click_turns(70.1));
    }

    /// Outside combat mode the right button's walk skips the reach; in
    /// combat mode Ctrl or a hold measures it, the right button aside.
    #[test]
    fn when_the_reach_is_measured() {
        assert!(measures_reach(false, false, false, false));
        assert!(!measures_reach(false, true, true, true));
        assert!(!measures_reach(true, false, false, false));
        assert!(measures_reach(true, true, false, true));
        assert!(measures_reach(true, false, true, true));
    }

    /// While the kinesis waits, its pick alone answers the reach: a movable
    /// prop is in reach wherever it is, anything else out of it.
    #[test]
    fn the_waiting_kinesis_answers_the_reach() {
        let prop = Hit::Prop {
            body: Entity::PLACEHOLDER,
            part: 0,
        };
        let corpse = Hit::Character {
            body: Entity::PLACEHOLDER,
            part: 0,
            dead: true,
        };
        // Waiting: a movable prop is in reach even when measured out of it.
        assert!(!press_out_of_reach(
            true,
            true,
            Some(prop),
            Some(Pick::Loose)
        ));
        // Waiting: a near corpse is out of reach all the same.
        assert!(press_out_of_reach(
            false,
            true,
            Some(corpse),
            Some(Pick::Corpse)
        ));
        assert!(press_out_of_reach(false, true, Some(prop), None));
        assert!(press_out_of_reach(false, true, None, None));
        // Not waiting: the measure answers.
        assert!(press_out_of_reach(
            true,
            false,
            Some(prop),
            Some(Pick::Loose)
        ));
        assert!(!press_out_of_reach(
            false,
            false,
            Some(corpse),
            Some(Pick::Corpse)
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
            parent_world: bevy::math::Affine3A::IDENTITY,
            mobility: 1.0,
            spheres: vec![(Vec3::ZERO, 0.2)],
            origin: Vec3::ZERO,
            links: Vec::new(),
            mesh: None,
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
            .init_resource::<Time<Real>>()
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
    /// lift holds (not while a mechanism is dragged), and shows the cross
    /// only for the refusal; a grabbed box must stay within 280 cm up and
    /// 1000 cm down.
    #[test]
    fn a_refused_press_keeps_the_request_and_shows_the_cross() {
        let mut hand = Hand::default();
        assert!(!hand.keeps_the_request() && !hand.refused());
        hand.holding = Holding::Refused;
        assert!(hand.keeps_the_request() && hand.refused());
        hand.holding = Holding::Lift;
        assert!(hand.keeps_the_request() && !hand.refused());
        // A door's drag leaves the request to the keys.
        hand.holding = Holding::Mechanism(Pick::Hinge);
        assert!(!hand.keeps_the_request() && !hand.refused());
        hand.holding = Holding::Drag {
            body: Entity::PLACEHOLDER,
            height: 0.0,
            character: false,
        };
        assert!(!hand.keeps_the_request());
        assert!(box_ok([
            Vec3::new(0.0, 2.8, 0.0),
            Vec3::new(0.0, -10.0, 0.0)
        ]));
        assert!(!box_ok([
            Vec3::new(0.0, 2.8, 0.0),
            Vec3::new(0.0, 2.81, 0.0)
        ]));
        assert!(!box_ok([Vec3::new(0.0, -10.01, 0.0)]));
        assert!(box_ok([Vec3::new(0.0, f32::NAN, 0.0)]));
    }

    /// The box's eight corners are tested in the original's order; one over
    /// 280 cm or under −1000 cm refuses it, and a NaN corner passes.
    #[test]
    fn a_box_is_placed_before_it_is_refused() {
        use bevy::math::Affine3A;
        let centre = Vec3::ZERO;
        let half = Vec3::new(0.1, 1.4, 0.1);
        assert!(box_ok_local(centre, half, Affine3A::IDENTITY));
        assert!(!box_ok_local(
            centre,
            Vec3::new(0.1, 2.81, 0.1),
            Affine3A::IDENTITY
        ));
        // Rotated so a corner reaches the ceiling while the unrotated box
        // does not: 2 m wide, 2.9 m tall at 45° over 2.8 m.
        let tall = Vec3::new(2.0, 2.0, 0.05);
        assert!(box_ok_local(centre, tall, Affine3A::IDENTITY));
        let tilt = Affine3A::from_rotation_z(std::f32::consts::FRAC_PI_4);
        assert!(!box_ok_local(centre, tall, tilt));
        // The frame's translation counts: lifted 1.5 m, the 1.4 m half
        // height reaches 2.9 m; sunk 9 m, the low corner is under −10 m.
        let lift = Affine3A::from_translation(Vec3::Y * 1.5);
        assert!(!box_ok_local(centre, half, lift));
        let sink = Affine3A::from_translation(Vec3::Y * -9.0);
        assert!(!box_ok_local(centre, half, sink));
        // A NaN height is an unordered pass.
        assert!(box_ok_local(
            centre,
            Vec3::new(0.0, f32::NAN, 0.0),
            Affine3A::IDENTITY
        ));
    }

    /// A closing door pulls 100 cm from its hinge-side point toward the
    /// cursor; a sliding one projects onto its own axis; a latch rattles
    /// with the drawn volume and pitch.
    #[test]
    fn the_door_and_wheel_grips() {
        let grip = DoorGrip {
            anchor: Vec3::ZERO,
            sliding: false,
            latched: false,
        };
        let a = door_pull_anchor(grip, Vec3::new(0.0, 0.0, 3.0), Vec3::X);
        assert!((a - Vec3::new(0.0, 0.0, 1.0)).length() < 1e-5, "{a}");
        let sliding = DoorGrip {
            sliding: true,
            ..grip
        };
        let a = door_pull_anchor(sliding, Vec3::new(0.3, 1.0, 2.0), Vec3::Z);
        assert!((a - Vec3::new(0.0, 0.0, 2.0)).length() < 1e-5, "{a}");
        let rattle = door_rattle(true, false, || 0.5).unwrap();
        assert!((rattle.pitch - 0.9).abs() < 1e-6);
        assert!((rattle.volume - 0.2).abs() < 1e-6);
        assert_eq!(door_rattle(true, true, || 0.5), None);
        assert_eq!(door_rattle(false, false, || 0.5), None);
        // The plane pick at a height, and no point for a level ray.
        assert!(
            (plane_pick(Vec3::Y * 2.0, Vec3::NEG_Y, 0.5).unwrap() - Vec3::Y * 0.5).length() < 1e-5
        );
        assert_eq!(plane_pick(Vec3::Y, Vec3::X, 0.5), None);
    }

    /// The wheel stops at its limit, never turns the other way, and picks
    /// its first grip at an alignment of 0.2. The cloth grabs the first
    /// vertex within its radius and follows the picked point.
    #[test]
    fn the_wheel_and_cloth_drags() {
        assert!((wheel_turn(0.5, 2.0, 1.0) - 1.5).abs() < 1e-6);
        assert!((wheel_turn(1.5, 2.0, 1.0) - 2.0).abs() < 1e-6);
        assert!((wheel_turn(0.5, 0.0, 1.0) - 0.5).abs() < 1e-6);
        // The gap is not wrapped: −6 is no turn, so the angle stays.
        assert_eq!(wheel_turn(3.0, -3.0, 1.0), 3.0);
        // The float π is over the double π: it wraps to −π.
        assert_eq!(
            wheel_turn(3.0, std::f32::consts::PI, 1.0),
            std::f32::consts::PI - std::f32::consts::TAU
        );
        assert!(uses_first_wheel_grip(0.2));
        assert!(!uses_first_wheel_grip(0.19));
        assert!(uses_first_wheel_grip(-0.3));
        let v = [Vec3::ZERO, Vec3::X, Vec3::Y];
        assert_eq!(cloth_vertex(&v, Vec3::new(0.99, 0.0, 0.0), 0.05), Some(1));
        assert_eq!(cloth_vertex(&v, Vec3::new(0.5, 0.0, 0.0), 0.05), None);
        let grabbed = Vec3::new(1.0, 0.0, 0.0);
        assert_eq!(
            cloth_drag_anchor(
                Some(grabbed),
                Some(Vec3::new(1.01, 0.0, 0.0)),
                Vec3::Y,
                0.05
            ),
            Vec3::new(1.01, 0.0, 0.0)
        );
        assert_eq!(
            cloth_drag_anchor(Some(grabbed), Some(Vec3::new(2.0, 0.0, 0.0)), Vec3::Y, 0.05),
            Vec3::Y
        );
    }

    /// The mechanism press: a latch rattles and refuses a closing leaf but
    /// a sliding one still drags; no body refuses; a door grip is 100 cm
    /// from the node toward the hit; a wheel and cloth are their drags.
    #[test]
    fn the_mechanism_press() {
        let state = MechanismState {
            latched: false,
            forcing: false,
            sliding: false,
            has_body: true,
        };
        let node = Vec3::ZERO;
        let hit = Vec3::new(0.0, 1.0, 3.0);
        let pressed = mechanism_press(Pick::Hinge, node, hit, state, || 0.5);
        let MechanismPress::Door(grip) = pressed else {
            panic!("{pressed:?}");
        };
        assert!((grip.anchor - hit.normalize()).length() < 1e-5);
        assert!(!grip.sliding && !grip.latched);
        let sliding = MechanismState {
            sliding: true,
            ..state
        };
        let MechanismPress::Door(grip) = mechanism_press(Pick::Hinge, node, hit, sliding, || 0.5)
        else {
            panic!();
        };
        assert_eq!(grip.anchor, hit);
        let latched = MechanismState {
            latched: true,
            ..state
        };
        let pressed = mechanism_press(Pick::Hinge, node, hit, latched, || 0.5);
        let MechanismPress::Rattle {
            rattle,
            still_drags,
        } = pressed
        else {
            panic!("{pressed:?}");
        };
        assert!(!still_drags);
        assert!((rattle.pitch - 0.9).abs() < 1e-6);
        let pressed = mechanism_press(
            Pick::Hinge,
            node,
            hit,
            MechanismState {
                sliding: true,
                ..latched
            },
            || 0.5,
        );
        assert!(matches!(
            pressed,
            MechanismPress::Rattle {
                still_drags: true,
                ..
            }
        ));
        assert_eq!(
            mechanism_press(
                Pick::Hinge,
                node,
                hit,
                MechanismState {
                    has_body: false,
                    ..state
                },
                || 0.5
            ),
            MechanismPress::Refused
        );
        assert_eq!(
            mechanism_press(Pick::Wheel, node, hit, state, || 0.5),
            MechanismPress::Wheel
        );
        assert_eq!(
            mechanism_press(Pick::Scene, node, hit, state, || 0.5),
            MechanismPress::Cloth
        );
    }

    /// The release dispatch tests the two GUI/lift bits first, then door,
    /// physical, wheel, GUI and cloth.
    #[test]
    fn the_release_dispatch_order() {
        assert_eq!(release_handler(0), ReleaseHandler::None);
        assert_eq!(release_handler(0x2000), ReleaseHandler::Lift);
        assert_eq!(release_handler(0x1000), ReleaseHandler::Lift);
        assert_eq!(release_handler(0x4000), ReleaseHandler::Door);
        assert_eq!(release_handler(0x8000), ReleaseHandler::Physical);
        assert_eq!(release_handler(0x10000), ReleaseHandler::Wheel);
        assert_eq!(release_handler(0x40000), ReleaseHandler::GuiCallback);
        assert_eq!(release_handler(0x20000), ReleaseHandler::Cloth);
        // The first matching bit wins.
        assert_eq!(release_handler(0x4000 | 0x8000), ReleaseHandler::Door);
        assert_eq!(release_handler(0x10000 | 0x20000), ReleaseHandler::Wheel);
    }

    /// The state word round-trips and a hand's hold maps onto it.
    #[test]
    fn the_drag_state_word() {
        for state in [
            DragState::Idle,
            DragState::Use,
            DragState::GuiPending,
            DragState::Lift,
            DragState::Door,
            DragState::Physical,
            DragState::Wheel,
            DragState::Cloth,
            DragState::GuiCallback,
            DragState::Refused,
        ] {
            assert_eq!(DragState::from_word(state.word()), state, "{state:?}");
        }
        let mut hand = Hand::default();
        assert_eq!(hand.drag_state(), DragState::Idle);
        hand.holding = Holding::Lift;
        assert_eq!(hand.drag_state(), DragState::Lift);
        hand.holding = Holding::Mechanism(Pick::Hinge);
        assert_eq!(hand.drag_state(), DragState::Door);
        hand.holding = Holding::Mechanism(Pick::Wheel);
        assert_eq!(hand.drag_state(), DragState::Wheel);
        hand.holding = Holding::Mechanism(Pick::Scene);
        assert_eq!(hand.drag_state(), DragState::Cloth);
        hand.holding = Holding::Refused;
        assert_eq!(hand.drag_state(), DragState::Refused);
    }
}
