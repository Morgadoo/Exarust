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
//!   1, in the tick's first two passes. Without Ctrl it is lifted onto the
//!   item cursor ([`crate::item_cursor`]), which rides the cursor until its
//!   release;
//! - a corpse: dragged by the limb the pick hit, strength 0.5, in every
//!   pass;
//! - a hinged door's leaf (`exa_physics::body::DoorBody`): a latched one
//!   rattles and refuses; otherwise a link (slack 1 cm, strength 0.003, the
//!   tick's first three passes) takes the leaf point 100 cm from the door's
//!   origin toward the hit (at the hit's height), and each frame its anchor
//!   is the point 100 cm from that origin toward the cursor's point on the
//!   plane at that height, the link disabled while the hip is farther than
//!   350 cm from the door; the release removes it;
//! - a lever's arm (`exa_physics::body::LeverBody`, the wheel fitting of
//!   its operative class 3): no link; the press marks the lever held and
//!   keeps the angle between the arm and the cursor's point on the plane
//!   through the node across the axle; each frame, while the hip is within
//!   300 cm of the node, the lever's hand angle is the cursor's angle on that
//!   plane plus the kept one (or, while the view lies across the axle by
//!   less than 0.2, the heading of the cursor's sideways offset on the plane
//!   across the arm at 30 cm), clamped to the swing; out of reach the lever
//!   is let go and woken; the lever's own step turns its arm 0.02 rad a tick
//!   toward that angle; the release lets go;
//! - a sliding door: the drag is recorded until the release (sliding doors
//!   are static level geometry unless `--sliding-doors`); a
//!   scene object's mesh that is a cloth takes the vertex within 10 cm of
//!   the point [a stand-in for the mesh element the original's pick hit],
//!   and each frame the vertex is pulled to the cursor's point on the cloth
//!   while the pick stays within its radius, else to the cursor's ray on
//!   the plane through the press point facing the camera;
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
//! the heading takes the cursor's direction. A double click (the
//! interaction) on a push surface whose operative flag 4 is clear pushes it
//! (`exa_physics::body::PlateBody::push`) while the point is in reach. (The original then names or
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
//! object's meshes), the nearest hit first. A static mesh under a named scene
//! node is that node ([`Hit::SceneNode`]), whose name a click hook can
//! compare, as the original's scene pick returns the node; generated level
//! geometry and every other collider stay [`Hit::Level`].
//!
//! Evidence: the original's left-press, drag and release handlers, the
//! per-frame request's reach and the controls' frame, read in the
//! instructions (`context/re/player-control-hand.md`).

use bevy::math::Affine3A;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_combat::Condition;
use exa_physics::ColliderNodes;
use exa_physics::Physics;
use exa_physics::body::locomotion::{Locomotion, Locomotor};
use exa_physics::body::{DoorBody, LeverBody, PlateBody, Ragdoll, WAKE_COUNT};
use exa_physics::prop::{PropBody, StandIns};
use exa_ui::ScriptedPointer;

use crate::bindings::{Action, Bindings};
use crate::control_nodes::Frame;
use crate::gui_screens::{ContainerRequest, ScreenRequest};
use crate::item_cursor::{self, HeldItem, ItemCursor, Release};
use crate::object_click::ObjectClick;
use crate::pick::{self, Body, Pick, ReachTo, Target};
use crate::player::{ChaseCamera, Player, PlayerBody, window_ray};
use crate::player_controls::{self, CharacterInteraction, ClickOutcome, ClickTarget};
use crate::thaum::kinesis_live::{KinesisPickHook, KinesisPicked};
use exa_ui::hud::HudShowUntil;
use exa_world::markers::{PlacedTemplate, PlacementMarkerNode};

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
    /// The item cursor's lift: the item rides the cursor, held by a grab
    /// with a slack, and turns with the mouse; the release puts it back at
    /// the frame its lift saved (or leaves it, or the GUI takes it).
    Lift,
    /// A door's, a wheel's or a scene's drag, until the release. Unlike a
    /// lift or a refusal it leaves the movement request to the keys and the
    /// right button, as the original's door, wheel and cloth drags do.
    Mechanism(Pick),
    /// A hinged door's leaf pulled by the hand's link
    /// (`exa_physics::body::DoorBody`), until the release. Like a
    /// mechanism's drag it leaves the movement request to the keys.
    Door { body: Entity, grip: DoorGrip },
    /// A lever's arm turned by the wheel drag
    /// (`exa_physics::body::LeverBody`), until the release: the lever is
    /// held and each frame sets its hand angle from the cursor; `grabbed`
    /// is the angle between the arm and the cursor's point at the press.
    Wheel { body: Entity, grabbed: f32 },
    /// A cloth's vertex under the hand: the cloth, the vertex the press
    /// took, and the drag plane the cursor pulls along when the pick leaves
    /// the vertex: its normal (the camera's view axis at the press) and its
    /// offset `normal·point` (m) through the press point.
    Cloth {
        cloth: Entity,
        vertex: usize,
        normal: Vec3,
        offset: f32,
    },
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
    /// A scene press's point, for the cloth grab that follows it.
    pub scene_press: Option<Vec3>,
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
            Holding::Mechanism(Pick::Hinge) | Holding::Door { .. } => DragState::Door,
            Holding::Mechanism(Pick::Wheel) | Holding::Wheel { .. } => DragState::Wheel,
            Holding::Mechanism(_) | Holding::Cloth { .. } => DragState::Cloth,
            Holding::Refused => DragState::Refused,
        }
    }

    /// Whether some interaction holds the cursor: any drag, lift, mechanism
    /// or refusal (the original's drag-state word is not zero).
    pub fn interacting(&self) -> bool {
        self.holding != Holding::Nothing
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
/// While a door is dragged, the pull is suppressed once the hip is farther
/// than this squared distance (cm²; 350 cm) from the door's origin.
pub const DOOR_REACH2: f32 = 122_500.0;
/// While a wheel is turned, the cursor is out of reach once the hip is
/// farther than this squared distance (cm²; 300 cm) from the lever's node.
pub const WHEEL_REACH2: f32 = 90_000.0;
/// The first wheel grip (the plane across the axle) is used while the
/// camera's view axis lies along the axle by at least this (a double
/// compare; a NaN takes the first grip).
pub const FIRST_GRIP: f64 = 0.2;

/// Where a ray meets the plane `normal·x = offset`; `None` when it runs
/// along the plane.
pub fn ray_plane(origin: Vec3, dir: Vec3, normal: Vec3, offset: f32) -> Option<Vec3> {
    let t = (offset - normal.dot(origin)) / normal.dot(dir);
    let at = origin + dir * t;
    at.is_finite().then_some(at)
}

/// Whether the wheel turns by its first grip: the camera's view axis lies
/// along the lever's axle by at least 0.2 (its absolute dot with the first
/// plane's normal, widened to double; a NaN takes the first grip).
pub fn first_wheel_grip(view: Vec3, across: Vec3) -> bool {
    let along = (view.x * across.x + view.y * across.y + view.z * across.z).abs();
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    let below = f64::from(along) < FIRST_GRIP;
    !below
}

/// A door's drag link: its flags (the iteration limit alone), its last
/// pass, its slack (cm) and its strength. The press passes slack 1.0 and
/// strength 0.003 to the engine's link-adding routine, which stores the
/// slack squared; `exa_physics::body::door_body` holds the link.
pub const DOOR_LINK_FLAGS: u32 = 0x4000;
pub const DOOR_LINK_LIMIT: u32 = 2;
pub const DOOR_LINK_SLACK_CM: f32 = 1.0;
pub const DOOR_LINK_STRENGTH: f32 = 0.003;

/// A door's drag as the press records it: the hinge-side origin it pulls
/// about, the leaf point its link holds, whether the leaf slides, and
/// whether it was latched.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DoorGrip {
    /// The door node's origin at the hit's height (a hinged leaf's pulls
    /// are 100 cm from it).
    pub origin: Vec3,
    /// The leaf point the link holds: 100 cm from the origin toward the hit
    /// for a hinged leaf, the hit itself for a sliding one.
    pub point: Vec3,
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

/// A door's dragged anchor (m). A hinged leaf is pulled to the point 100
/// cm from its origin toward the cursor's point (on the plane at the grip's
/// height), wherever the cursor is. A sliding leaf's point is moved by the
/// cursor's offset projected onto its own sliding axis `axis` (unit,
/// across its frame), so it only moves along the slide.
pub fn door_pull_anchor(grip: DoorGrip, cursor_point: Vec3, axis: Vec3) -> Vec3 {
    if grip.sliding {
        grip.point + axis * (cursor_point - grip.point).dot(axis)
    } else {
        let delta = cursor_point - grip.origin;
        grip.origin + delta * (1.0 / delta.length())
    }
}

/// One frame of a door drag: the link's stop bit while the movement
/// request suppresses the pull, else the anchor the leaf is pulled to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DoorFrame {
    pub anchor: Option<Vec3>,
    /// The link word's stop bit (`0x100000`).
    pub stop: bool,
}

/// The door drag's frame: a suppressed request sets the link's stop bit
/// and leaves the leaf's point alone; otherwise the leaf is pulled to
/// [`door_pull_anchor`] and the bit clears. A plain door drag therefore
/// never turns the player's request off.
pub fn door_drag_frame(
    grip: DoorGrip,
    cursor_point: Vec3,
    axis: Vec3,
    suppressed: bool,
) -> DoorFrame {
    if suppressed {
        DoorFrame {
            anchor: None,
            stop: true,
        }
    } else {
        DoorFrame {
            anchor: Some(door_pull_anchor(grip, cursor_point, axis)),
            stop: false,
        }
    }
}

/// The cloth vertex a press grabbed: the first of the cloth's local
/// vertices within `radius` (m) of the press point in the cloth's frame.
pub fn cloth_vertex(vertices: &[Vec3], at: Vec3, radius: f32) -> Option<usize> {
    let r2 = radius * radius;
    vertices
        .iter()
        .position(|v| (*v - at).length_squared() < r2)
}

/// The radius (cm) a press takes a cloth vertex within, and the radius the
/// pick must stay within to keep pulling on the vertex itself [a stand-in:
/// the original keeps each grabbed element's own radius].
pub const CLOTH_GRAB_CM: f32 = 10.0;

/// One frame of a cloth drag's target in the cloth's own frame
/// (centimetres): the picked point while it lies strictly within `radius`
/// of the grabbed vertex, else the plane's point. The original's grab
/// record holds the vertex's own radius and receives this target.
pub fn cloth_drag_frame(
    vertex_cm: Vec3,
    picked_cm: Option<Vec3>,
    plane_cm: Vec3,
    radius_cm: f32,
) -> Vec3 {
    match picked_cm {
        Some(p) if (vertex_cm - p).length_squared() < radius_cm * radius_cm => p,
        _ => plane_cm,
    }
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
            // The press keeps the node's origin at the hit's height; a
            // hinged leaf's link holds the point 100 cm from it toward the
            // hit, a sliding leaf's the hit itself.
            let origin = Vec3::new(node.x, hit.y, node.z);
            let delta = hit - origin;
            let point = if state.sliding {
                hit
            } else {
                origin + delta * (1.0 / delta.length())
            };
            MechanismPress::Door(DoorGrip {
                origin,
                point,
                sliding: state.sliding,
                latched: state.latched,
            })
        }
        Pick::Wheel => MechanismPress::Wheel,
        Pick::Scene => MechanismPress::Cloth,
        _ => MechanismPress::Refused,
    }
}

/// The handler the forced release (the movement request's cancel, a fall,
/// the kinesis taking over) picks for a drag state word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseHandler {
    None,
    /// The lift's end (the item cursor).
    Lift,
    Door,
    Physical,
    /// The wheel's end, which also ends a cloth drag.
    Wheel,
    GuiCallback,
}

/// The forced release's handler, in the original's test order: the two
/// GUI/lift bits together, then the door, physical, wheel, GUI callback
/// and cloth bits. A cloth drag's bit calls the wheel's handler, not the
/// cloth's own (the press installs the cloth's for the button's release;
/// the forced release reaches the wheel's, which lets go of the dragged
/// node and wakes its body without ending the cloth's grab). After any
/// handler the state word and the GUI drag hooks clear, and so does the
/// behaviour byte that lets the combat frame run the movement request.
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
        ReleaseHandler::Wheel
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
    /// Plain level geometry (a mesh with no body and no scene node): a wall,
    /// a floor, the terrain.
    Level,
    /// A named scene node: static scenery whose mesh was built under it. The
    /// original's pick returns the node itself; its name is compared by the
    /// click hooks.
    SceneNode { node: Entity },
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

/// A left click that did not become a drag: what it hit and where the
/// pointer was, in physical window pixels. The hub's click hook reads it;
/// the world's own click path turns the player toward the point.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct WorldClick {
    pub hit: Hit,
    pub pointer: IVec2,
    /// The press was the second of a double click.
    pub double: bool,
}

impl WorldClick {
    /// A click on `hit` at no particular pointer place, for tests.
    pub fn picked(hit: Hit) -> Self {
        Self {
            hit,
            pointer: IVec2::ZERO,
            double: false,
        }
    }
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
            for &(centre, radius) in p.spheres.iter() {
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
    collider_nodes: &ColliderNodes,
    characters: impl Iterator<Item = (Entity, &'a Ragdoll, Condition)>,
) -> Option<Picked> {
    let (origin, dir) = (ray.origin, *ray.direction);
    let fixed = physics
        .fixed_pick(origin, dir, PICK_RANGE)
        .map(|(handle, t)| {
            let hit = if let Some((body, part)) = stand_ins.owner_of(handle) {
                Hit::Prop { body, part }
            } else if let Some(node) = collider_nodes.node_of(handle) {
                Hit::SceneNode { node }
            } else {
                Hit::Level
            };
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
fn name_of(
    hit: Hit,
    top_mobility: impl Fn(Entity) -> Option<f32>,
    door: impl Fn(Entity) -> Option<bool>,
    lever: impl Fn(Entity) -> bool,
) -> Pick {
    let target = match hit {
        // A lever's arm: its operative record's class 3, the wheel fitting.
        Hit::Prop { body, .. } if lever(body) => Target {
            has_record: true,
            fitting: Some(pick::Fitting::Wheel),
            body: Some(Body::Prop),
            ..Target::default()
        },
        // A door's leaf: its record's hinge, blocked when the door is not
        // clickable (its flag 8).
        Hit::Prop { body, .. } if door(body).is_some() => Target {
            has_record: true,
            hinge: door(body).map(|blocked| pick::Hinge { blocked }),
            body: Some(Body::Prop),
            ..Target::default()
        },
        // Static scenery carries no object body, so the hand takes nothing
        // from it; the click hook still reads the node's name.
        Hit::Level | Hit::SceneNode { .. } => Target::default(),
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

/// Lets go of every door leaf the hand no longer pulls, however its drag
/// ended (the release, a fall, a forced release): its link is removed.
fn let_go_of_doors(hand: Res<Hand>, mut doors: Query<(Entity, &mut DoorBody, &mut Ragdoll)>) {
    let pulled = match hand.holding {
        Holding::Door { body, .. } => Some(body),
        _ => None,
    };
    for (e, mut glue, mut ragdoll) in &mut doors {
        if glue.door.held && Some(e) != pulled {
            glue.let_go(&mut ragdoll);
        }
    }
}

/// Lets go of every lever the hand no longer turns, however its drag
/// ended: the hold clears and the lever wakes.
fn let_go_of_levers(hand: Res<Hand>, mut levers: Query<(Entity, &mut LeverBody, &mut Ragdoll)>) {
    let turned = match hand.holding {
        Holding::Wheel { body, .. } => Some(body),
        _ => None,
    };
    for (e, mut glue, mut ragdoll) in &mut levers {
        if glue.lever.held && Some(e) != turned {
            glue.let_go(&mut ragdoll);
        }
    }
}

pub struct HandPlugin;

/// The hand's frame: the press, the hold and the reach.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HandSystems;

impl Plugin for HandPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Hand>()
            .init_resource::<ItemCursor>()
            .init_resource::<player_controls::PendingUses>()
            .init_resource::<player_controls::DeviceClock>()
            .add_message::<WorldClick>()
            .add_message::<ObjectClick>()
            .add_message::<crate::item_card::WorldCard>()
            .add_message::<exa_audio::PointSounded>()
            .add_message::<ContainerRequest>()
            .add_message::<exa_world::mechanisms::OpenDoorsUnder>()
            .add_message::<exa_world::nav::NavMoved>()
            .add_systems(
                Update,
                (
                    press_hand,
                    take_cloth,
                    gather_markers,
                    hold_hand,
                    crate::object_click::scripted_object_clicks,
                    crate::object_click::run_object_clicks,
                    reach_hand,
                    run_pending_uses,
                    let_go_of_doors,
                    let_go_of_levers,
                    send_let_go,
                )
                    .chain()
                    .in_set(HandSystems)
                    .after(crate::player::chase_camera)
                    .before(crate::cursor::pick_cursor),
            )
            .add_systems(
                Update,
                run_corpse_scripts
                    .after(HandSystems)
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

/// What the pick reads of the world beyond the characters: the colliders,
/// the props' stand-ins and the scene nodes some static colliders came from.
#[derive(bevy::ecs::system::SystemParam)]
struct PickSources<'w> {
    physics: Res<'w, Physics>,
    stand_ins: Res<'w, StandIns>,
    collider_nodes: Res<'w, ColliderNodes>,
}

/// Every body but the player's: its condition when it is a character's,
/// and whether it is a prop's.
type OtherBodies = (
    Entity,
    &'static mut Ragdoll,
    Option<&'static Condition>,
    Has<PropBody>,
);

/// What the press reads beyond the bodies: whether the pointer is over the
/// GUI, the placed objects' ids, the door leaves and the lever arms.
type PressMechanisms<'w, 's> = (
    Option<Res<'w, crate::panel_live::PointerOverGui>>,
    Query<'w, 's, ObjectIds>,
    Query<'w, 's, &'static mut DoorBody>,
    Query<'w, 's, &'static mut LeverBody>,
);

/// What a body's nodes say of the placed object they belong to: its id,
/// its template and the node above.
type ObjectIds = (
    Option<&'static exa_world::PlacedItem>,
    Option<&'static PlacedTemplate>,
    Option<&'static ChildOf>,
);

/// The object id and template of the placed object a body belongs to: the
/// first placed object at or above the body's node (the original's lift
/// stores the held node's id word, and the cursor asks the database for its
/// template, the record header's parent word). Zero for both when no
/// placed object is over it; the template is the id itself when the
/// object carries none, as the original's lookup returns the id it cannot
/// resolve.
fn object_ids(body: Entity, objects: &Query<ObjectIds>) -> (u32, u32) {
    let mut at = Some(body);
    while let Some(entity) = at {
        let Ok((placed, template, parent)) = objects.get(entity) else {
            break;
        };
        if let Some(placed) = placed {
            let template = template.map_or(placed.tagged_id, |t| t.0);
            return (placed.tagged_id, template);
        }
        at = parent.map(ChildOf::parent);
    }
    (0, 0)
}

/// The controls' frame step of the deferred list: while a player exists,
/// every entry whose time has come opens the doors under its target, in
/// order (the clock is the game's virtual one the double click stamps).
fn run_pending_uses(
    players: Query<(), With<Player>>,
    time: Option<Res<Time<Virtual>>>,
    mut uses: ResMut<player_controls::PendingUses>,
    mut doors: MessageWriter<exa_world::mechanisms::OpenDoorsUnder>,
) {
    if players.is_empty() || uses.0.entries.is_empty() {
        return;
    }
    let now = time.map_or(0, |time| time.elapsed().as_millis() as u32);
    for due in uses.0.tick(now) {
        info!("hand: deferred use opens the doors under {:#x}", due.target);
        doors.write(exa_world::mechanisms::OpenDoorsUnder { target: due.target });
    }
}

/// The level's placement markers while the item cursor holds something:
/// every registered marker node, in the order the scene query walks them,
/// with its bounding sphere and frame in the world. The list is emptied
/// once nothing is held, so the walk costs nothing outside a lift.
fn gather_markers(
    mut items: ResMut<ItemCursor>,
    markers: Query<(
        &PlacementMarkerNode,
        &exa_world::PartBounds,
        &GlobalTransform,
    )>,
) {
    if items.held.is_none() {
        if !items.markers.is_empty() {
            items.markers.clear();
        }
        return;
    }
    items.markers.clear();
    for (marker, bounds, transform) in &markers {
        let world = transform.affine();
        items.markers.push(item_cursor::PlacementMarker {
            name: marker.name,
            link: marker.link,
            centre: world.transform_point3(bounds.centre),
            radius: bounds.radius,
            frame: Frame::new(
                Vec3::from(world.translation),
                bevy::math::Mat3::from_cols(
                    Vec3::from(world.x_axis),
                    Vec3::from(world.y_axis),
                    Vec3::from(world.z_axis),
                ),
            ),
        });
    }
}

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
    collider_nodes: &ColliderNodes,
    bodies: &Query<OtherBodies, Without<PlayerBody>>,
    player: Option<(Entity, &Ragdoll, Condition)>,
) -> Option<Picked> {
    let others = bodies.iter().filter_map(|(e, r, c, prop)| match (c, prop) {
        (Some(c), false) => Some((e, r, *c)),
        _ => None,
    });
    pick_along(
        ray?,
        physics,
        stand_ins,
        collider_nodes,
        others.chain(player),
    )
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

/// The cloth vertex a press takes, if one is within 10 cm of the point in
/// the cloth's own frame [guess: the original takes the mesh element the
/// pick hit and its own radius; Exarust's cloth does not keep that radius].
fn grab_cloth(
    cloths: &Query<(Entity, &exa_world::cloth::ClothInstance, &GlobalTransform)>,
    at: Vec3,
) -> Option<(Entity, usize)> {
    for (entity, instance, transform) in cloths {
        let local = transform.affine().inverse().transform_point3(at) * CM;
        let vertices: Vec<Vec3> = instance
            .cloth
            .particles
            .iter()
            .map(|p| p.position)
            .collect();
        if let Some(vertex) = cloth_vertex(&vertices, local, CLOTH_GRAB_CM) {
            return Some((entity, vertex));
        }
    }
    None
}

/// A scene press on a cloth: the vertex under the point is grabbed, and
/// the drag plane is set through the point facing the camera (the
/// original takes the camera's view axis as the plane's normal).
fn take_cloth(
    mut hand: ResMut<Hand>,
    cloths: Query<(Entity, &exa_world::cloth::ClothInstance, &GlobalTransform)>,
    cameras: Query<&GlobalTransform, With<ChaseCamera>>,
) {
    let Some(at) = hand.scene_press.take() else {
        return;
    };
    let normal = cameras
        .single()
        .map_or(Vec3::NEG_Z, |eye| eye.forward().as_vec3());
    if let Some((cloth, vertex)) = grab_cloth(&cloths, at) {
        hand.holding = Holding::Cloth {
            cloth,
            vertex,
            normal,
            offset: normal.dot(at),
        };
    }
}

/// A left press: the pick named and its interaction started.
#[allow(clippy::too_many_arguments)]
fn press_hand(
    real: Res<Time<Real>>,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    bindings: Res<Bindings>,
    scripted: Res<ScriptedPointer>,
    sources: PickSources,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<ChaseCamera>>,
    mut hand: ResMut<Hand>,
    mut items: ResMut<ItemCursor>,
    players: Query<(&GlobalTransform, &Children), With<Player>>,
    mut player_bodies: Query<PlayerParts, Without<PropBody>>,
    mut bodies: Query<OtherBodies, Without<PlayerBody>>,
    kinesis: Option<Res<KinesisPickHook>>,
    mut kinesis_picks: Option<ResMut<Messages<KinesisPicked>>>,
    (over_gui, objects, mut doors, mut levers): PressMechanisms,
) {
    let kinesis = kinesis.and_then(|k| k.0);
    let now = real.elapsed().as_millis() as i64;
    let pointer = windows.single().ok().and_then(|w| {
        scripted
            .position(w)
            .map(|p| (p, (p * w.scale_factor()).as_ivec2()))
    });
    // The GUI's own press path takes a press over an open screen; the world
    // sees neither the pick nor the click.
    if over_gui.is_some_and(|over| over.0) && buttons.just_pressed(MouseButton::Left) {
        return;
    }
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
        &sources.physics,
        &sources.stand_ins,
        &sources.collider_nodes,
        &bodies,
        own.map(|r| (child, r, condition)),
    );
    let named = picked.map(|p| {
        name_of(
            p.hit,
            |body| {
                bodies
                    .get(body)
                    .ok()
                    .and_then(|(_, r, ..)| r.parts.iter().find(|p| p.parent.is_none()))
                    .map(|p| p.inverse_mass)
            },
            |body| {
                doors
                    .get(body)
                    .ok()
                    .map(|d| d.door.record.has(exa_physics::door::NOT_CLICKABLE))
            },
            |body| levers.contains(body),
        )
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
        Press::Mechanism(pick) => {
            // A hinged door's leaf takes the hand's link: a latched one
            // rattles and refuses (its sound is not played yet).
            if pick == Pick::Hinge
                && let Some(Picked {
                    at,
                    hit: Hit::Prop { body, .. },
                }) = picked
                && let Ok(mut glue) = doors.get_mut(body)
                && let Ok((_, mut ragdoll, ..)) = bodies.get_mut(body)
            {
                let state = MechanismState {
                    latched: glue.latched(),
                    forcing: glue.forcing(),
                    sliding: false,
                    has_body: true,
                };
                let origin = glue.origin(&ragdoll);
                match mechanism_press(Pick::Hinge, origin, at, state, || 0.5) {
                    MechanismPress::Door(grip) => {
                        glue.take_hold(&mut ragdoll, grip.point);
                        info!(
                            "hand: door pulled at ({:.2}, {:.2}, {:.2})",
                            grip.point.x, grip.point.y, grip.point.z
                        );
                        hand.holding = Holding::Door { body, grip };
                    }
                    _ => hand.holding = Holding::Refused,
                }
                return;
            }
            // A lever's arm takes the wheel's hold: the angle between the
            // arm and the cursor's point on the plane across its axle.
            if pick == Pick::Wheel
                && let Some(Picked {
                    hit: Hit::Prop { body, .. },
                    ..
                }) = picked
                && let Ok(mut glue) = levers.get_mut(body)
                && let Ok((_, mut ragdoll, ..)) = bodies.get_mut(body)
                && let Some(ray) = ray
            {
                let (across, _) = glue.grip_normals();
                let offset = across.dot(glue.origin());
                let point = ray_plane(ray.origin, *ray.direction, across, offset)
                    .unwrap_or_else(|| glue.arm_point(&ragdoll));
                let grabbed = glue.take_hold(&mut ragdoll, point);
                info!("hand: lever taken by the wheel, {grabbed:.3} rad from the arm");
                hand.holding = Holding::Wheel { body, grabbed };
                return;
            }
            hand.holding = Holding::Mechanism(pick);
            // A scene object's mesh can be a cloth: the next system takes a
            // vertex near the point when there is one.
            hand.scene_press = (pick == Pick::Scene)
                .then(|| picked.map(|p| p.at))
                .flatten();
        }
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
            // The lift rides the item cursor (Exarust holds it with a plain
            // grab), and saves the frames a cancelled release puts it back
            // to.
            let Some(Picked {
                at,
                hit: Hit::Prop { body, part },
            }) = picked
            else {
                return;
            };
            let Ok((_, mut ragdoll, ..)) = bodies.get_mut(body) else {
                return;
            };
            // A frozen object (one a lock marker holds) is thawed by the
            // press that takes it.
            if ragdoll.frozen() {
                ragdoll.freeze(false);
            }
            if !ragdoll.grab(part, at, item_cursor::LIFT_STRENGTH, None) {
                return;
            }
            let saved: Vec<bevy::math::Affine3A> =
                ragdoll.parts.iter().map(|p| p.transform()).collect();
            let (centre, half) = ragdoll
                .parts
                .get(part)
                .map(|p| item_cursor::rest_box(p.rest))
                .unwrap_or((Vec3::ZERO, Vec3::ZERO));
            // The lift saves the part's frame, rotation included, as the
            // original copies the held node's frame.
            let world = ragdoll
                .parts
                .get(part)
                .map(|p| p.transform())
                .unwrap_or_default();
            let saved_frame = Frame::new(
                Vec3::from(world.translation),
                bevy::math::Mat3::from_cols(
                    Vec3::from(world.x_axis),
                    Vec3::from(world.y_axis),
                    Vec3::from(world.z_axis),
                ),
            );
            pb.hold_object(true);
            let held_ids = object_ids(body, &objects);
            items.begin(HeldItem {
                body,
                part,
                saved,
                saved_frame,
                shown: saved_frame,
                centre,
                half,
                target: at,
                // The lift keeps the object's id and its template, which a
                // placement marker's name is compared with.
                item: held_ids.0,
                model: held_ids.1,
            });
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
                Hit::Level | Hit::SceneNode { .. } => return,
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
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn hold_hand(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    bindings: Res<Bindings>,
    sources: PickSources,
    scripted: Res<ScriptedPointer>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<ChaseCamera>>,
    (mut hand, mut items, mut doors, mut levers, mut plates): (
        ResMut<Hand>,
        ResMut<ItemCursor>,
        Query<&mut DoorBody>,
        Query<&mut LeverBody>,
        Query<&mut PlateBody>,
    ),
    players: Query<(&GlobalTransform, &Children), With<Player>>,
    mut player_bodies: Query<PlayerParts, Without<PropBody>>,
    mut requests: Query<&mut Locomotion, With<PlayerBody>>,
    mut bodies: Query<OtherBodies, Without<PlayerBody>>,
    mut cloths: Query<(&mut exa_world::cloth::ClothInstance, &GlobalTransform)>,
    mut screens: Option<ResMut<Messages<ScreenRequest>>>,
    (time, mut show_bars, pad, pad_status): (
        Option<Res<Time<Virtual>>>,
        Option<ResMut<HudShowUntil>>,
        Option<Res<crate::gamepad::PlayPad>>,
        Option<Res<crate::gamepad::PadStatus>>,
    ),
    (mut clicks, mut objects): (MessageWriter<WorldClick>, MessageWriter<ObjectClick>),
) {
    let released = buttons.just_released(MouseButton::Left);
    let player = player_child(&players, &player_bodies);
    // A release before the drag started is a click: outside combat mode
    // the player turns toward the cursor, the hub's click hook sees what
    // the click hit, and a click on the player's own body shows the health
    // and focus bars for 5000 ms. A double click (with Ctrl held in combat
    // mode) then interacts with what is there. The original takes the double
    // click on the second press; Exarust reads it on that press's release.
    if released {
        let left = hand.left.take();
        if let Some(left) = left.filter(|l| !l.dragging)
            && let Some((_, child)) = player
            && let Ok((pb, own, condition, _)) = player_bodies.get(child)
        {
            let combat = pb.combat;
            let condition = condition.copied().unwrap_or_default();
            let window = windows.single().ok();
            let ray = window
                .zip(cameras.single().ok())
                .and_then(|(window, (camera, eye))| window_ray(camera, eye, window, left.at));
            let picked = pick_with(
                ray,
                &sources.physics,
                &sources.stand_ins,
                &sources.collider_nodes,
                &bodies,
                own.map(|r| (child, r, condition)),
            );
            if let Some(picked) = picked {
                let pointer = window
                    .map(|window| (left.at * window.scale_factor()).as_ivec2())
                    .unwrap_or(IVec2::ZERO);
                clicks.write(WorldClick {
                    hit: picked.hit,
                    pointer,
                    double: left.double,
                });
            }
            if !combat
                && let Some((yaw, distance)) = pb.cursor()
                && click_turns(distance * CM)
                && let Ok(mut req) = requests.get_mut(child)
            {
                req.heading = yaw;
            }
            let interact = left.double && (!combat || bindings.pressed(&keys, Action::Interact));
            // The interaction on a push surface whose operative flag 4 is
            // clear pushes it (and wakes it), while the point is in reach.
            if interact
                && !hand.out_of_reach
                && let Some(Picked {
                    hit: Hit::Prop { body, .. },
                    ..
                }) = picked
                && let Ok(mut glue) = plates.get_mut(body)
                && glue.clickable()
                && let Ok((_, mut ragdoll, ..)) = bodies.get_mut(body)
            {
                glue.push(&mut ragdoll, true);
                info!("hand: push surface {:#x} pushed", glue.id);
            }
            // A click on anything but a character goes to the placed
            // objects' click handler: the single click outside combat mode,
            // the double click's interaction while in reach.
            if let Some(picked) = picked
                && !matches!(picked.hit, Hit::Character { .. })
            {
                let pointer = window
                    .map(|window| (left.at * window.scale_factor()).as_ivec2())
                    .unwrap_or(IVec2::ZERO);
                if !combat {
                    objects.write(ObjectClick {
                        hit: picked.hit,
                        at: picked.at,
                        pointer,
                        double: false,
                    });
                }
                if interact && !hand.out_of_reach {
                    objects.write(ObjectClick {
                        hit: picked.hit,
                        at: picked.at,
                        pointer,
                        double: true,
                    });
                }
            }
            if !combat || interact {
                let target = picked.and_then(|picked| match picked.hit {
                    Hit::Character { body, .. } if body == child => Some(ClickTarget::Player),
                    Hit::Character { dead, .. } if dead => Some(ClickTarget::DeadCharacter),
                    Hit::Character { .. } => Some(ClickTarget::LivingCharacter),
                    _ => None,
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
                        // screen; a dead body opens its loot window. The
                        // talk and dialogue windows have no Exarust form
                        // yet.
                        if interaction == CharacterInteraction::Inventory
                            && let Some(messages) = screens.as_deref_mut()
                        {
                            messages.write(ScreenRequest::screen(
                                exa_ui::game_gui::GameScreen::Inventory,
                                !condition.dying(),
                                !condition.is_dead(),
                            ));
                        }
                        if interaction == CharacterInteraction::Loot
                            && let Some(Hit::Character { body, .. }) = picked.map(|p| p.hit)
                            && let Some(messages) = screens.as_deref_mut()
                        {
                            messages.write(ScreenRequest {
                                screen: exa_ui::game_gui::GameScreen::Loot,
                                player_ready: !condition.dying(),
                                player_alive: !condition.is_dead(),
                                target: Some(body.index().index() as i32),
                                // The screen opener reads the body's record
                                // flag word from its possessions; 0 passes
                                // the show rule when it has none.
                                flags: 0,
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
        Holding::Wheel { body, grabbed } => {
            let (Ok(mut glue), Ok((_, mut ragdoll, ..))) =
                (levers.get_mut(body), bodies.get_mut(body))
            else {
                hand.holding = Holding::Nothing;
                return;
            };
            if released {
                glue.let_go(&mut ragdoll);
                hand.holding = Holding::Nothing;
                return;
            }
            // Out of reach the lever is let go (and woken) but the drag
            // stays; in reach each frame sets the hand angle.
            if hand.out_of_reach {
                glue.let_go(&mut ragdoll);
                return;
            }
            let Some(ray) = cursor_ray(&scripted, &windows, &cameras) else {
                return;
            };
            let view = cameras
                .single()
                .map(|(_, eye)| eye.forward().as_vec3())
                .unwrap_or(Vec3::NEG_Z);
            let (across, forward) = glue.grip_normals();
            let at = glue.origin();
            let (origin, dir) = (ray.origin, *ray.direction);
            let first = ray_plane(origin, dir, across, across.dot(at));
            let second = ray_plane(origin, dir, forward, forward.dot(at));
            if let (Some(first), Some(second)) = (first, second) {
                let grip = first_wheel_grip(view, across);
                glue.turn_to(&mut ragdoll, first, second, grip, grabbed);
            }
        }
        Holding::Door { body, grip } => {
            let (Ok(mut glue), Ok((_, mut ragdoll, ..))) =
                (doors.get_mut(body), bodies.get_mut(body))
            else {
                hand.holding = Holding::Nothing;
                return;
            };
            if released {
                glue.let_go(&mut ragdoll);
                hand.holding = Holding::Nothing;
                return;
            }
            // The request suppresses the pull while the hip is farther than
            // 350 cm from the door (the squared distance over 122 500 cm²).
            let hip = player.and_then(|(feet, child)| {
                player_bodies
                    .get(child)
                    .ok()
                    .map(|(_, own, ..)| hip_of(feet, own))
            });
            let far = hip.is_some_and(|hip| {
                ((hip - glue.origin(&ragdoll)) * CM).length_squared() > DOOR_REACH2
            });
            let cursor = cursor_ray(&scripted, &windows, &cameras)
                .and_then(|ray| plane_pick(ray.origin, *ray.direction, grip.origin.y));
            match cursor {
                Some(point) => {
                    let frame = door_drag_frame(grip, point, Vec3::X, far);
                    glue.pull_to(&mut ragdoll, frame.anchor);
                }
                None => glue.pull_to(&mut ragdoll, None),
            }
        }
        Holding::Cloth {
            cloth,
            vertex,
            normal,
            offset,
        } => {
            let Ok((mut instance, transform)) = cloths.get_mut(cloth) else {
                hand.holding = Holding::Nothing;
                return;
            };
            if released {
                instance.cloth.release_grab();
                hand.holding = Holding::Nothing;
            } else if let Some(ray) = cursor_ray(&scripted, &windows, &cameras) {
                // The picked point on the cloth, else the cursor's ray on the
                // drag plane, in the cloth's own frame; the grabbed vertex
                // keeps its own radius (a 10 cm stand-in). Exarust's pick
                // has no cloth triangles: the ray's nearest point to the
                // vertex stands in for the pick's point on the cloth.
                let (origin, dir) = (ray.origin, *ray.direction);
                let to_local = |w: Vec3| transform.affine().inverse().transform_point3(w) * CM;
                if let Some(world_vertex) = instance
                    .cloth
                    .particles
                    .get(vertex)
                    .map(|p| transform.transform_point(p.position / CM))
                {
                    let along = (world_vertex - origin).dot(dir).max(0.0);
                    let picked = origin + dir * along;
                    let across = normal.dot(dir);
                    let plane = (offset - normal.dot(origin)) / across;
                    let plane = if plane.is_finite() && plane > 0.0 {
                        origin + dir * plane
                    } else {
                        world_vertex
                    };
                    let target = cloth_drag_frame(
                        to_local(world_vertex),
                        Some(to_local(picked)),
                        to_local(plane),
                        CLOTH_GRAB_CM,
                    );
                    instance.cloth.grab(vertex, target);
                }
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
                // The lift's own out-of-reach answer cancels it.
                end_lift(&mut items, &mut bodies, hand.out_of_reach);
            } else {
                // The movement inputs turn the item (the lift keeps the
                // movement request): the pad's raw move stick, or the move
                // keys under its dead zone. The base speed ramps after
                // 300 ms of turning; 500 ms of quiet drops it back.
                let now = time
                    .as_deref()
                    .map_or(0, |t| t.elapsed().as_millis() as u64);
                let frame_ms = time.as_deref().map_or(0.0, |t| t.delta_secs() * 1000.0);
                let stick = if pad_status.as_ref().is_some_and(|s| s.active()) {
                    pad.as_ref().map_or(Vec2::ZERO, |p| p.frame().move_raw)
                } else {
                    Vec2::ZERO
                };
                let keys_held = [
                    bindings.pressed(&keys, Action::Forward),
                    bindings.pressed(&keys, Action::Back),
                    bindings.pressed(&keys, Action::Left),
                    bindings.pressed(&keys, Action::Right),
                ];
                let right = cameras
                    .single()
                    .map(|(_, eye)| eye.right().as_vec3())
                    .unwrap_or(Vec3::X);
                let (delta, turned) = items.turn.frame(now, frame_ms, stick, keys_held);
                if turned {
                    items.offset += delta;
                    items.rotation *= item_cursor::turn_delta(right, delta);
                }
                let Some(held) = items.held.as_ref() else {
                    hand.holding = Holding::Nothing;
                    return;
                };
                let (body, part, box_centre, box_half, held_target) =
                    (held.body, held.part, held.centre, held.half, held.target);
                if turned && let Ok((_, mut ragdoll, ..)) = bodies.get_mut(body) {
                    let centre = ragdoll
                        .parts
                        .get(part)
                        .map(|p| Vec3::from(p.transform().translation))
                        .unwrap_or(Vec3::ZERO);
                    ragdoll.turn_about(centre, item_cursor::turn_delta(right, delta));
                }
                // The cursor's ray places the item against the level, the
                // held item itself left out of the pick (as the original
                // flags its node while it picks). The item goes there
                // whether or not its box fits. A cursor over nothing keeps
                // the last point (its pick has no navigation mesh behind
                // every pixel). As in the original, the item then slides
                // along the ray from 600 cm before the picked point and
                // stops at its first contact: every collision sphere of
                // every part is swept from there, and the nearest contact
                // under 1000 cm of the sweep's start wins. A sweep that
                // starts inside the level is no contact, so the item keeps
                // the pick.
                let ray = cursor_ray(&scripted, &windows, &cameras);
                if let Some(ray) = ray {
                    let (origin, dir) = (ray.origin, *ray.direction);
                    let hit = sources
                        .physics
                        .fixed_pick_except(origin, dir, PICK_RANGE, &|h| {
                            sources
                                .stand_ins
                                .owner_of(h)
                                .is_some_and(|(owner, _)| owner == body)
                        });
                    // The spheres of every part, in world space, each with
                    // the part it belongs to, taken relative to the point
                    // the cursor places (the body's grab point): the sweep
                    // carries that point along the ray from its start.
                    let spheres: Vec<item_cursor::SphereOf> = bodies
                        .get(body)
                        .map(|(_, ragdoll, ..)| {
                            let anchor = ragdoll.grab_point().unwrap_or(held_target);
                            ragdoll
                                .parts
                                .iter()
                                .enumerate()
                                .flat_map(|(index, part)| {
                                    let frame = part.transform();
                                    part.spheres.iter().map(move |(centre, radius)| {
                                        item_cursor::SphereOf {
                                            part: index,
                                            sphere: item_cursor::SweepSphere {
                                                centre: frame.transform_point(*centre) - anchor,
                                                radius: *radius,
                                            },
                                        }
                                    })
                                })
                                .filter(|s| s.sphere.radius > 0.0)
                                .collect()
                        })
                        .unwrap_or_default();
                    let point = hit.and_then(|(_, t)| {
                        // The sweep starts 600 cm before the picked point
                        // (Exarust never starts it behind the eye).
                        let back = t.min(item_cursor::SWEEP_BACK_CM / CM);
                        let start = t - back;
                        let from = origin + dir * start;
                        let contact = item_cursor::per_sphere_contact(
                            spheres.iter().copied(),
                            dir,
                            |_, offset, dir, reach, radius| {
                                sources
                                    .physics
                                    .level_sweep(from + offset, dir, reach, radius)
                                    .filter(|toi| *toi > 0.0)
                            },
                        )
                        .map(|hit| start * CM + hit.contact);
                        let distance = item_cursor::placement_distance(t * CM, contact);
                        item_cursor::placement_point(origin, dir, distance)
                    });
                    // A placement marker named for the item (its id or its
                    // template) snaps it onto the marker's frame: the ray's
                    // segment runs to the picked point, or 10 000 cm
                    // without one. The held node takes the marker's frame
                    // whole, as the original copies the marker's matrix
                    // into it. A marker the item would overlap there is
                    // refused: the original asks the scene whether the
                    // item's body (or a child) meets another body at that
                    // frame. Exarust tests the item's own spheres against
                    // the other bodies' part spheres, its stand-in for the
                    // original's sphere-against-mesh narrow phase.
                    let length = hit.map_or(item_cursor::MARKER_RAY_CM / CM, |(_, t)| t);
                    // The spheres are gathered only when a marker named
                    // for the item is met, once per frame.
                    type Spheres = Vec<(Vec3, f32)>;
                    let mut gathered: Option<(Vec<(Affine3A, std::sync::Arc<Spheres>)>, Spheres)> =
                        None;
                    let bodies_ref = &bodies;
                    let held_part = items.held.as_ref().map(|held| held.part);
                    let overlaps = |frame: Frame| {
                        let Some(held_part) = held_part else {
                            return false;
                        };
                        let (held_frames, others) = gathered.get_or_insert_with(|| {
                            let held_frames = bodies_ref
                                .get(body)
                                .map(|(_, ragdoll, ..)| {
                                    ragdoll
                                        .parts
                                        .iter()
                                        .map(|p| (p.transform(), p.spheres.clone()))
                                        .collect()
                                })
                                .unwrap_or_default();
                            let others = bodies_ref
                                .iter()
                                .filter(|(entity, ..)| *entity != body)
                                .flat_map(|(_, ragdoll, ..)| {
                                    ragdoll.parts.iter().flat_map(|p| {
                                        let frame = p.transform();
                                        p.spheres.iter().map(move |(centre, radius)| {
                                            (frame.transform_point(*centre), *radius)
                                        })
                                    })
                                })
                                .collect();
                            (held_frames, others)
                        });
                        let parts: Vec<Affine3A> = held_frames.iter().map(|(f, _)| *f).collect();
                        let placed = item_cursor::snapped_parts(&parts, held_part, frame);
                        placed
                            .iter()
                            .zip(held_frames.iter())
                            .any(|(at, (_, spheres))| {
                                spheres.iter().any(|(centre, radius)| {
                                    let world = at.transform_point(*centre);
                                    others.iter().any(|(other, other_radius)| {
                                        world.distance_squared(*other)
                                            <= (*radius + *other_radius) * (*radius + *other_radius)
                                    })
                                })
                            })
                    };
                    let snapped = items.snap(origin, dir, length, overlaps);
                    // Each placement holds only within reach of the hip
                    // (and a free one only when its box fitted): the
                    // item's new place is measured as the walk-to point.
                    // A frame that does not hold puts the item back at the
                    // last placement that did, keeps the hover word, and
                    // counts as out of reach (the cursor a cross; a release
                    // then is a cancelled one).
                    let hip = player.and_then(|(feet, child)| {
                        player_bodies
                            .get(child)
                            .ok()
                            .map(|(_, own, ..)| hip_of(feet, own))
                    });
                    let in_reach = |at: Vec3| {
                        hip.is_some_and(|hip| {
                            pick::walk_within_reach(at * CM, hip * CM, pick::Region::None)
                        })
                    };
                    let held_frame = match (snapped, point) {
                        (Some(snap), _) => {
                            item_cursor::lift_frame(snap.hover, true, in_reach(snap.frame.position))
                                .map(|word| (word, snap.frame, None))
                        }
                        (None, Some(point)) => {
                            let (frame, fits) = item_cursor::place_frame(
                                items.rotation,
                                point,
                                box_centre,
                                box_half,
                            );
                            let centre = frame.position + frame.rotation * box_centre;
                            item_cursor::lift_frame(item_cursor::HOVER_FREE, fits, in_reach(centre))
                                .map(|word| (word, frame, Some(point)))
                        }
                        (None, None) => None,
                    };
                    hand.out_of_reach = held_frame.is_none();
                    match held_frame {
                        Some((word, frame, free)) => {
                            items.hover = word;
                            if let Some(held) = items.held.as_mut() {
                                held.shown = frame;
                                held.target = free.unwrap_or(frame.position);
                            }
                            if let Ok((_, mut ragdoll, ..)) = bodies.get_mut(body) {
                                match free {
                                    Some(point) => {
                                        ragdoll.pull_grab_to(point);
                                        ragdoll.wake_to(DRAG_WAKE);
                                    }
                                    // A marker's frame is copied into the
                                    // held node whole.
                                    None => {
                                        let worlds = item_cursor::snapped_parts(
                                            &ragdoll
                                                .parts
                                                .iter()
                                                .map(|p| p.transform())
                                                .collect::<Vec<_>>(),
                                            part,
                                            frame,
                                        );
                                        ragdoll.place_at(&worlds);
                                        ragdoll.hold_grab_still();
                                    }
                                }
                            }
                        }
                        None => {
                            if let Ok((_, mut ragdoll, ..)) = bodies.get_mut(body) {
                                ragdoll.pull_grab_to(held_target);
                            }
                        }
                    }
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

/// Ends the item cursor's lift, as the lift's release handler does: in
/// reach (`cancelled` false) the item is put down where it is; a cancelled
/// lift goes by [`item_cursor::release`]: placed again at its last fitting
/// point, left on its marker (frozen there for a lock marker) or put back
/// at the frames the lift saved. The body is woken unless frozen. Every
/// let-go queues the held node's box at the frame the lift saved for the
/// navigation grid's refresh, as the original's let-go rings it.
fn end_lift(
    items: &mut ItemCursor,
    bodies: &mut Query<OtherBodies, Without<PlayerBody>>,
    cancelled: bool,
) {
    // The last fitting placement is where the item already rests: a frame
    // that does not hold puts it back there, so it still fits.
    let Some((outcome, held)) = items.end(cancelled, true) else {
        return;
    };
    // The let-go queues the held node's box, at the frame the lift saved,
    // for the grid's refresh: the floor the item stood on is refreshed.
    if let Some(&at) = held.saved.get(held.part) {
        items.let_go.push(exa_world::nav::NavMoved {
            at,
            centre: held.centre,
            half: held.half,
        });
    }
    let Ok((_, mut ragdoll, ..)) = bodies.get_mut(held.body) else {
        return;
    };
    ragdoll.release_grab();
    match outcome {
        Release::Drop | Release::LastFit | Release::OnMarker => ragdoll.wake(),
        Release::Lock => ragdoll.freeze(true),
        Release::PutBack => {
            ragdoll.place_at(&held.saved);
            ragdoll.wake();
        }
    }
}

/// Hands the items let go this frame to the navigation grid's refresh.
fn send_let_go(mut items: ResMut<ItemCursor>, mut moved: MessageWriter<exa_world::nav::NavMoved>) {
    if !items.let_go.is_empty() {
        moved.write_batch(items.let_go.drain(..));
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
    sources: PickSources,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<ChaseCamera>>,
    (mut hand, mut items): (ResMut<Hand>, ResMut<ItemCursor>),
    players: Query<(&GlobalTransform, &Children), With<Player>>,
    player_bodies: Query<PlayerParts, Without<PropBody>>,
    mut bodies: Query<OtherBodies, Without<PlayerBody>>,
    levers: Query<&LeverBody>,
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
        // The forced release runs the lift's handler with the point out of
        // reach, so the lift ends as a cancelled one.
        if hand.holding == Holding::Lift {
            end_lift(&mut items, &mut bodies, true);
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
        Holding::Drag { .. } | Holding::Door { .. } => true,
        // A turned wheel is in reach while the hip is within 300 cm of the
        // lever's node.
        Holding::Wheel { body, .. } => {
            let hip = hip_of(feet, own);
            levers
                .get(body)
                .is_ok_and(|glue| ((hip - glue.origin()) * CM).length_squared() <= WHEEL_REACH2)
        }
        _ => {
            let hip = hip_of(feet, own);
            let ray = cursor_ray(&scripted, &windows, &cameras);
            pick_with(
                ray,
                &sources.physics,
                &sources.stand_ins,
                &sources.collider_nodes,
                &bodies,
                own.map(|r| (child, r, condition)),
            )
            .is_some_and(|p| {
                // A lever's reach is measured from its node.
                match p.hit {
                    Hit::Prop { body, .. } if levers.contains(body) => {
                        let node = levers.get(body).map_or(p.at, |g| g.origin());
                        pick::within_reach(p.at.y * CM, (node - hip) * CM, ReachTo::Wheel)
                    }
                    _ => pick::within_reach(p.at.y * CM, (p.at - hip) * CM, ReachTo::Point),
                }
            })
        }
    };
    hand.out_of_reach = !in_reach;
}

/// Runs a dead character's role scripts on a double click: the roles in
/// list order whose first Player-aimed entry carries the corpse section,
/// each with the corpse as actor 0 and the player as actor 1, and the
/// effects the scripts ask for go out as [`exa_script::ScriptEffect`].
fn run_corpse_scripts(
    mut clicks: MessageReader<WorldClick>,
    scripts: Option<Res<exa_script::Scripts>>,
    conversations: Option<ResMut<exa_script::Conversations>>,
    corpses: Query<(&exa_ai::roles::Roles, Option<&exa_ai::brain::CharacterId>)>,
    players: Query<Option<&exa_ai::brain::CharacterId>, With<PlayerBody>>,
    time: Option<Res<Time<Virtual>>>,
    mut effects: MessageWriter<exa_script::ScriptEffect>,
) {
    let (Some(scripts), Some(mut conversations)) = (scripts, conversations) else {
        return;
    };
    for click in clicks.read() {
        if !click.double {
            continue;
        }
        let Hit::Character {
            body, dead: true, ..
        } = click.hit
        else {
            continue;
        };
        let Ok((roles, id)) = corpses.get(body) else {
            continue;
        };
        let Some(corpse) = id.map(|id| id.0) else {
            continue;
        };
        let player = players
            .iter()
            .next()
            .and_then(|id| id.map(|id| id.0))
            .unwrap_or(exa_ai::brain::PLAYER_ID);
        let list = exa_ai::roles::corpse_scripts(&scripts.roles, roles);
        if list.is_empty() {
            continue;
        }
        let now = time.as_ref().map_or(0, |t| t.elapsed().as_millis() as u32);
        let exa_script::Conversations {
            engine,
            cast,
            player: scene_player,
            player2,
            world_base,
            locale,
            ..
        } = &mut *conversations;
        let mut scene = exa_script::dialogue::Scene {
            library: exa_script::dialogue::Library {
                roles: &scripts.roles,
                narrator: scripts.narrator.as_ref(),
            },
            cast,
            now,
            world_time: exa_script::topics::world_time(*world_base, now, 0),
            player: *scene_player,
            player2: *player2,
            locale: *locale,
            effects: Vec::new(),
        };
        for script in list {
            engine.run_script(
                &mut scene,
                vec![Some(corpse), Some(player)],
                script.role,
                script.script,
            );
        }
        for effect in scene.effects.drain(..) {
            effects.write(exa_script::ScriptEffect(effect));
        }
    }
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
            triangles: None,
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
        let collider_nodes = ColliderNodes::default();
        let bodies = [(far, &b, Condition::NORMAL), (near, &a, Condition::DEAD)];
        let picked = pick_along(
            ray,
            &physics,
            &stand_ins,
            &collider_nodes,
            bodies.into_iter(),
        )
        .unwrap();
        assert!((picked.at.z - 2.8).abs() < 1e-4, "{}", picked.at);
        assert_eq!(
            picked.hit,
            Hit::Character {
                body: near,
                part: 0,
                dead: true
            }
        );
        assert_eq!(
            name_of(picked.hit, |_| None, |_| None, |_| false),
            Pick::Corpse
        );
        let aside = Ray3d::new(Vec3::X, Dir3::Z);
        let bodies = [(near, &a, Condition::DEAD)];
        assert_eq!(
            pick_along(
                aside,
                &physics,
                &stand_ins,
                &collider_nodes,
                bodies.into_iter()
            ),
            None
        );
        // A prop's name reads its top part's mobility.
        let prop = Hit::Prop {
            body: near,
            part: 0,
        };
        assert_eq!(
            name_of(prop, |_| Some(0.5), |_| None, |_| false),
            Pick::Loose
        );
        assert_eq!(
            name_of(prop, |_| Some(0.05), |_| None, |_| false),
            Pick::Nothing
        );
        assert_eq!(
            name_of(Hit::Level, |_| None, |_| None, |_| false),
            Pick::Nothing
        );
        // A door's leaf is a hinge, a door that is not clickable nothing.
        assert_eq!(
            name_of(prop, |_| Some(0.05), |_| Some(false), |_| false),
            Pick::Hinge
        );
        assert_eq!(
            name_of(prop, |_| Some(0.05), |_| Some(true), |_| false),
            Pick::Nothing
        );
        // A lever's arm is the wheel fitting, whatever its mobility.
        assert_eq!(
            name_of(prop, |_| Some(0.05), |_| None, |_| true),
            Pick::Wheel
        );
        assert_eq!(
            name_of(Hit::SceneNode { node: near }, |_| None, |_| None, |_| false),
            Pick::Nothing
        );
    }

    /// A ray aimed at one of the installed hub's named scenery meshes finds
    /// that scene node, not the level: the shop's and the notice board's
    /// names are the two the hub's click hook compares. The level is loaded
    /// from `EXANIMA_DIR`; the check is skipped without it.
    #[test]
    fn arenahub_scenery_nodes_resolve_under_a_ray() {
        use bevy::ecs::world::CommandQueue;

        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let Ok(game) = exa_assets::GameData::open(&dir) else {
            return;
        };
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::transform::TransformPlugin,
            exa_physics::PhysicsPlugin,
        ))
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<exa_render::material::ExaMaterial>>()
        .init_resource::<Assets<Image>>()
        .insert_resource(exa_assets::ModelCache::default())
        .insert_resource(exa_assets::Game(std::sync::Arc::new(game)));
        // Spawn the level's own model, props and placed objects.
        let game = app.world().resource::<exa_assets::Game>().0.clone();
        let mut cache = app
            .world_mut()
            .remove_resource::<exa_assets::ModelCache>()
            .unwrap();
        let mut meshes = app.world_mut().remove_resource::<Assets<Mesh>>().unwrap();
        let mut materials = app
            .world_mut()
            .remove_resource::<Assets<exa_render::material::ExaMaterial>>()
            .unwrap();
        let mut images = app.world_mut().remove_resource::<Assets<Image>>().unwrap();
        let mut queue = CommandQueue::default();
        let root = {
            let mut loader = exa_assets::ModelLoader {
                game: &game,
                cache: &mut cache,
                meshes: &mut meshes,
                materials: &mut materials,
                images: &mut images,
                bc_supported: false,
            };
            let mut commands = Commands::new(&mut queue, app.world());
            exa_world::spawn_level(&mut commands, &mut loader, "arenahub").map(|(root, _)| root)
        };
        queue.apply(app.world_mut());
        app.world_mut().insert_resource(cache);
        app.world_mut().insert_resource(meshes);
        app.world_mut().insert_resource(materials);
        app.world_mut().insert_resource(images);
        let Some(root) = root else {
            return;
        };
        // The two names the hub compares, on the node the pick returns.
        let mut names = app.world_mut().query::<(Entity, &Name)>();
        let targets: Vec<(Entity, String)> = names
            .iter(app.world())
            .map(|(entity, name)| (entity, name.to_string()))
            .collect();
        let find = |wanted: &str| {
            targets
                .iter()
                .find(|(_, name)| name == wanted)
                .map(|(entity, _)| *entity)
        };
        let (Some(shop), Some(machine)) = (find("\u{aa}ArenaShop"), find("\u{aa}ArenaMach")) else {
            panic!("the hub's shop and notice board are missing");
        };
        // Build collision for just the two objects, not the whole hub.
        let mut tops = Vec::new();
        for node in [shop, machine] {
            let mut top = node;
            while let Some(child) = app.world().get::<ChildOf>(top) {
                let parent = child.parent();
                if parent == root {
                    break;
                }
                top = parent;
            }
            app.world_mut()
                .entity_mut(top)
                .insert(exa_physics::StaticGeometry);
            tops.push(top);
        }
        for _ in 0..20_000 {
            let building = tops.iter().any(|&top| {
                app.world()
                    .entity(top)
                    .contains::<exa_physics::StaticGeometry>()
            });
            if !building {
                break;
            }
            app.update();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }

        app.world_mut().resource_mut::<Physics>().0.step();
        // Aim at each node's own first usable triangle from just off it.
        let rays = [
            (shop, triangle_ray(app.world_mut(), shop)),
            (machine, triangle_ray(app.world_mut(), machine)),
        ];
        let physics = app.world().resource::<Physics>();
        let collider_nodes = app.world().resource::<ColliderNodes>();
        let stand_ins = StandIns::default();
        for (node, ray) in rays {
            let ray = ray.expect("the node has a mesh triangle");
            let (handle, _) = physics
                .fixed_pick(ray.origin, *ray.direction, PICK_RANGE)
                .expect("the ray meets the node");
            assert_eq!(collider_nodes.node_of(handle), Some(node));
            let picked = pick_along(ray, physics, &stand_ins, collider_nodes, std::iter::empty())
                .expect("the pick finds the node");
            assert_eq!(picked.hit, Hit::SceneNode { node });
        }
    }

    /// A ray from just off the node's first non-degenerate mesh triangle
    /// toward it, in world space.
    fn triangle_ray(world: &mut World, node: Entity) -> Option<Ray3d> {
        let shapes: Vec<(Handle<Mesh>, GlobalTransform, Entity)> = {
            let mut shapes = world.query::<(&Mesh3d, &GlobalTransform, &ChildOf)>();
            shapes
                .iter(world)
                .map(|(mesh, at, child)| (mesh.0.clone(), *at, child.parent()))
                .collect()
        };
        let meshes = world.resource::<Assets<Mesh>>();
        let mut found = None;
        for (mesh, at, mut under) in shapes {
            let mut below = under == node;
            while let Some(parent) = world.get::<ChildOf>(under) {
                under = parent.parent();
                if under == node {
                    below = true;
                    break;
                }
            }
            if !below {
                continue;
            }
            let Some(mesh) = meshes.get(&mesh) else {
                continue;
            };
            let Some(positions) = mesh
                .attribute(Mesh::ATTRIBUTE_POSITION)
                .and_then(|a| a.as_float3())
            else {
                continue;
            };
            let Some(indices) = mesh.indices() else {
                continue;
            };
            for triangle in indices.iter().collect::<Vec<_>>().chunks(3) {
                let [a, b, c] =
                    [0, 1, 2].map(|k| at.transform_point(Vec3::from(positions[triangle[k]])));
                let cross = (b - a).cross(c - a);
                if cross.length_squared() < 1e-12 {
                    continue;
                }
                let normal = cross.normalize();
                let centre = (a + b + c) / 3.0;
                found = Some(Ray3d::new(centre + normal * 0.05, Dir3::new(-normal).ok()?));
                break;
            }
            if found.is_some() {
                break;
            }
        }
        found
    }

    /// The hand's systems run together in one frame (their queries do not
    /// clash), and with no player they leave the hand alone.
    #[test]
    fn a_due_deferred_use_opens_the_doors_under_its_target() {
        let mut app = App::new();
        app.init_resource::<Time<Virtual>>()
            .init_resource::<player_controls::PendingUses>()
            .add_message::<exa_world::mechanisms::OpenDoorsUnder>()
            .add_systems(Update, run_pending_uses);
        let uses = |app: &mut App| {
            let mut q = app
                .world_mut()
                .resource_mut::<player_controls::PendingUses>();
            q.0.push(player_controls::PendingUse {
                due_ms: 0,
                target: 0x8000_0011,
            });
            q.0.push(player_controls::PendingUse {
                due_ms: 2_100,
                target: 0x8000_0012,
            });
        };
        uses(&mut app);
        // Without a player the list waits.
        app.update();
        assert_eq!(
            app.world()
                .resource::<player_controls::PendingUses>()
                .0
                .entries
                .len(),
            2
        );
        app.world_mut().spawn(Player);
        app.update();
        let sent: Vec<u32> = app
            .world_mut()
            .resource_mut::<Messages<exa_world::mechanisms::OpenDoorsUnder>>()
            .drain()
            .map(|order| order.target)
            .collect();
        assert_eq!(sent, vec![0x8000_0011]);
        let left = &app
            .world()
            .resource::<player_controls::PendingUses>()
            .0
            .entries;
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].target, 0x8000_0012);
    }

    #[test]
    fn the_hand_systems_run_without_a_player() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<Time<Real>>()
            .init_resource::<Bindings>()
            .init_resource::<ScriptedPointer>()
            .init_resource::<StandIns>()
            .init_resource::<ColliderNodes>()
            .insert_resource(Physics::default())
            .init_resource::<Hand>()
            .init_resource::<ItemCursor>()
            .add_message::<WorldClick>()
            .add_message::<ObjectClick>()
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
        assert!(!hand.interacting());
        assert!(!hand.keeps_the_request() && !hand.refused());
        hand.holding = Holding::Refused;
        assert!(hand.interacting() && hand.keeps_the_request() && hand.refused());
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
            origin: Vec3::ZERO,
            point: Vec3::new(0.0, 0.0, 1.0),
            sliding: false,
            latched: false,
        };
        let a = door_pull_anchor(grip, Vec3::new(0.0, 0.0, 3.0), Vec3::X);
        assert!((a - Vec3::new(0.0, 0.0, 1.0)).length() < 1e-5, "{a}");
        let sliding = DoorGrip {
            sliding: true,
            point: Vec3::ZERO,
            ..grip
        };
        let a = door_pull_anchor(sliding, Vec3::new(0.3, 1.0, 2.0), Vec3::Z);
        assert!((a - Vec3::new(0.0, 0.0, 2.0)).length() < 1e-5, "{a}");
        let frame = door_drag_frame(grip, Vec3::new(0.0, 0.0, 3.0), Vec3::X, false);
        assert!(!frame.stop);
        assert!((frame.anchor.unwrap().z - 1.0).abs() < 1e-6);
        let stopped = door_drag_frame(grip, Vec3::new(0.0, 0.0, 3.0), Vec3::X, true);
        assert!(stopped.stop);
        assert_eq!(stopped.anchor, None);
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

    /// The cloth grabs the first vertex within its radius and follows the
    /// picked point. (The wheel's turn is `exa_physics::body::LeverBody`'s.)
    #[test]
    fn the_cloth_drags() {
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
        // The origin takes the hit's height; the point is 100 cm from it
        // toward the hit, across the ground.
        assert_eq!(grip.origin, Vec3::new(0.0, 1.0, 0.0));
        assert!((grip.point - Vec3::new(0.0, 1.0, 1.0)).length() < 1e-5);
        assert!(!grip.sliding && !grip.latched);
        let sliding = MechanismState {
            sliding: true,
            ..state
        };
        let MechanismPress::Door(grip) = mechanism_press(Pick::Hinge, node, hit, sliding, || 0.5)
        else {
            panic!();
        };
        assert_eq!(grip.point, hit);
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
        // The cloth's bit reaches the wheel's handler.
        assert_eq!(release_handler(0x20000), ReleaseHandler::Wheel);
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

    /// A cloth's grabbed vertex follows the cursor while the pick is
    /// strictly within its radius and the plane's point otherwise, both in
    /// the cloth's own frame (cm).
    #[test]
    fn the_cloth_drag_frame() {
        let vertex = Vec3::new(10.0, 0.0, 0.0);
        let plane = Vec3::new(0.0, 0.0, 50.0);
        // The pick 9 cm from the vertex (within a 10 cm radius): the pick.
        let picked = vertex + Vec3::new(9.0, 0.0, 0.0);
        assert_eq!(cloth_drag_frame(vertex, Some(picked), plane, 10.0), picked);
        // 11 cm away: the plane's point.
        let picked = vertex + Vec3::new(11.0, 0.0, 0.0);
        assert_eq!(cloth_drag_frame(vertex, Some(picked), plane, 10.0), plane);
        assert_eq!(cloth_drag_frame(vertex, None, plane, 10.0), plane);
        // Exactly on the radius is not within it.
        let picked = vertex + Vec3::new(10.0, 0.0, 0.0);
        assert_eq!(cloth_drag_frame(vertex, Some(picked), plane, 10.0), plane);
    }

    /// The wheel's planes: a ray meets a plane at its offset; the first grip
    /// is used while the view lies along the axle by at least 0.2 (a double
    /// compare), and a NaN takes it too.
    #[test]
    fn the_wheel_picks_its_grip_by_the_view() {
        let at = ray_plane(Vec3::new(0.0, 1.0, -5.0), Vec3::Z, Vec3::Z, 2.0).unwrap();
        assert!(at.abs_diff_eq(Vec3::new(0.0, 1.0, 2.0), 1e-6));
        assert_eq!(ray_plane(Vec3::ZERO, Vec3::X, Vec3::Z, 1.0), None);
        assert!(first_wheel_grip(Vec3::X, Vec3::X));
        assert!(first_wheel_grip(Vec3::new(0.2, 0.0, 0.98), Vec3::X));
        assert!(!first_wheel_grip(Vec3::new(0.199_999, 0.0, 0.98), Vec3::X));
        // The single-precision 0.2 is above the double 0.2.
        assert!(first_wheel_grip(Vec3::new(-0.2, 0.0, 0.98), Vec3::X));
        assert!(first_wheel_grip(Vec3::NAN, Vec3::X));
    }
}
