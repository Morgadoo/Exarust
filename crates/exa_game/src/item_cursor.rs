//! The item cursor: the item the left button lifts onto the cursor, the way
//! the mouse turns it, where it may be put down, and what its release does.
//! Rules: `docs/PLAYER.md` (The hand).
//!
//! **The lift.** Without Ctrl the left press on a prop the hand can move
//! lifts it: the item leaves the world's hands and rides the cursor, held by
//! a grab of strength 1 with 0.003 m of slack, while the item GUI shows its
//! icon (`window`). Its frame at the lift is saved. The item cursor's own
//! frame turns with the mouse: the mouse or the pad's look moves a two-axis
//! offset, and the offset turns the item around the camera's right axis and
//! the world's up axis. The turn starts at 0.25 of the unit speed, and once
//! the cursor has been turning for more than 300 ms it ramps to 1.0; 500 ms
//! without a call drops it back to the base speed. The unit speed is the
//! camera's own sensitivity times 0.0025. A stick that is under 0.1 long
//! (double precision) is read as the four turn keys (W/S across, A/D up and
//! down).
//!
//! **Where it may go.** Each frame the cursor's ray is picked against the
//! level; a hit closer than the placement limit (100000 cm away, which no
//! pick reaches) places the item there. With no hit the item goes where the
//! ray crosses the plane 100000 cm ahead; a ray that climbs (its height
//! component over −1.19209e−7) refuses the placement. The item's box, its
//! rest points and half extents in its turned saved frame, must stay at
//! most 280 cm high and at least −1000 cm low (`hand::box_ok_local`).
//!
//! **The release.** An item dropped while the player walks is cancelled
//! (put back). Over a GUI item slot the GUI takes it back. With the "give
//! back" hover bit the item stays where it is (the hold flag clears); any
//! other release puts it back at the saved frame, as the original's
//! deferred restore does. The GUI drop spawn path takes an inventory entry
//! whose object has a body, puts it into the scene, and starts the lift on
//! it; an object with no body is the original's content error and is not
//! put into the scene.
//!
//! **The drop's scripts.** An item put down runs the scripts of the scene
//! objects within 5 cm² of the point (`player_controls::nearby_scripts`,
//! squared distance under 5).
//!
//! Evidence: the controls' lift, item-cursor turn, placement, release and
//! drop routines, read in the instructions
//! (`context/re/player-control-hand.md`).

use bevy::math::{Affine3A, IVec2, Mat3, Vec3};
use bevy::prelude::{Entity, Resource};

use crate::control_nodes::Frame;
use crate::hand;

/// The turn's base speed factor.
pub const BASE_TURN: f32 = 0.25;
/// The speed factor the turn ramps to.
pub const FAST_TURN: f32 = 1.0;
/// The turn ramps to the fast factor after this long, ms.
pub const RAMP_MS: u64 = 300;
/// The ramp's rate per millisecond (double), so 300 ms covers the remaining
/// 0.75 of the gap.
pub const RAMP_RATE: f64 = 0.003_333_333_333_333_333_5;
/// The share of the 0.25–1.0 gap the ramp covers (double).
pub const RAMP_SHARE: f64 = 0.75;
/// A cursor left alone this long drops back to the base speed, ms.
pub const IDLE_MS: u64 = 500;
/// The unit turn speed is the camera's sensitivity times this.
pub const TURN_SENSITIVITY: f32 = 0.0025;
/// A look stick shorter than this is read as the turn keys (double).
pub const STICK_DEAD_ZONE: f64 = 0.1;
/// A turn under this is no turn (the engine's float epsilon).
pub const TURN_EPSILON: f32 = 1.192_09e-7;
/// The placement's far limit: the fallback point is where the ray crosses
/// this height (cm).
pub const PLACEMENT_LIMIT_CM: f32 = -100_000.0;
/// The grab's slack (m), the original's link slack.
pub const LIFT_SLACK: f32 = 0.003;
/// The lift's grab strength.
pub const LIFT_STRENGTH: f32 = 1.0;
/// The hover word's bits: over a GUI item slot, over the world, and the
/// "give back" word that leaves the item where it is.
pub const HOVER_GUI: u32 = 1;
pub const HOVER_WORLD: u32 = 0x10;
pub const HOVER_GIVE_BACK: u32 = 0x30;

/// The mouse turn's running speed: the frame clock, the ramp's start and
/// whether the cursor was turning last call.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TurnRamp {
    /// The previous call's game clock (ms).
    pub frame_ms: u64,
    /// When the turn's ramp started, ms.
    pub ramp_ms: u64,
    pub turning: bool,
}

impl TurnRamp {
    /// The speed factor at `now_ms`: the base while the cursor is not
    /// turning, the fast factor after [`RAMP_MS`] of turning, else
    /// `0.25 + (now − ramp)·0.003333…·0.75`, the elapsed time in double and
    /// the sum narrowed once. A call more than [`IDLE_MS`] after the last
    /// starts from not turning.
    pub fn factor(&mut self, now_ms: u64) -> f32 {
        if self.frame_ms.saturating_add(IDLE_MS) < now_ms {
            self.turning = false;
        }
        self.frame_ms = now_ms;
        if !self.turning {
            BASE_TURN
        } else if self.ramp_ms.saturating_add(RAMP_MS) < now_ms {
            FAST_TURN
        } else {
            let elapsed = (now_ms.saturating_sub(self.ramp_ms)) as f64;
            (elapsed * RAMP_RATE * RAMP_SHARE + f64::from(BASE_TURN)) as f32
        }
    }

    /// One frame's turn: the speed factor times the camera's sensitivity
    /// times [`TURN_SENSITIVITY`], and the two-axis offset it moves,
    /// `offset`. A look under the dead zone (its length in double) is read
    /// as the turn keys across (W, S) and up and down (A, D), each ± the
    /// speed. Returns the added offset and whether any axis moved by more
    /// than [`TURN_EPSILON`]; the ramp starts on the first turning frame.
    pub fn frame(
        &mut self,
        now_ms: u64,
        camera_sensitivity: f32,
        look: Vec2f,
        keys: [bool; 4],
    ) -> (Vec2f, bool) {
        let speed = self.factor(now_ms) * camera_sensitivity * TURN_SENSITIVITY;
        let length = f64::from((look.x * look.x + look.y * look.y).sqrt());
        let delta = if length <= STICK_DEAD_ZONE {
            let across = (keys[0] as i32 - keys[1] as i32) as f32;
            let up = (keys[2] as i32 - keys[3] as i32) as f32;
            Vec2f::new(across * speed, up * speed)
        } else {
            look * speed
        };
        let turned = delta.x.abs() > TURN_EPSILON || delta.y.abs() > TURN_EPSILON;
        if turned {
            if !self.turning {
                self.ramp_ms = now_ms;
                self.turning = true;
            }
        } else {
            self.turning = false;
        }
        (delta, turned)
    }
}

/// A two-axis offset in the original's order (x across, y up and down).
pub type Vec2f = bevy::math::Vec2;

/// One turn's rotation: about the camera's `right` axis by the offset's
/// across part and about the world's up axis by its up part, composed with
/// the across one first. The offset is in radians; the caller scales it by
/// the turn speed and the frame time.
pub fn turn_delta(right: Vec3, offset: Vec2f) -> Mat3 {
    let across = Mat3::from_axis_angle(right, offset.x);
    let up = Mat3::from_axis_angle(Vec3::Y, offset.y);
    across * up
}

/// The item cursor's shown rotation after a turn of `offset` from `saved`.
/// The original turns the held frame a piece every frame, so repeated turns
/// compose; this one-shot form is for a single turn.
pub fn turned_rotation(saved: Mat3, right: Vec3, offset: Vec2f) -> Mat3 {
    saved * turn_delta(right, offset)
}

/// Where the cursor's ray puts the item (m): the hit when it is at least
/// `limit` away (the original's `hit distance < limit` test), else the
/// point where the ray crosses the height `limit`. A ray climbing more than
/// the float epsilon (its height part over −[`TURN_EPSILON`]) gives no
/// point. A comparison with a NaN passes as a hit.
pub fn place_point(origin: Vec3, dir: Vec3, hit: Option<(Vec3, f32)>, limit: f32) -> Option<Vec3> {
    match hit {
        Some((at, distance)) if distance.is_nan() || distance >= limit => Some(at),
        _ => {
            if dir.y > -TURN_EPSILON {
                return None;
            }
            let t = (limit / 100.0 - origin.y) / dir.y;
            let at = origin + dir * t;
            at.is_finite().then_some(at)
        }
    }
}

/// The item's placed frame at `at` (m): the cursor's turned `rotation`,
/// positioned at `at`. Returns `None` when the box at that frame does not
/// fit (`hand::box_ok_local`), as the original clears its grab and refuses
/// the spot.
pub fn place_frame(rotation: Mat3, at: Vec3, centre: Vec3, half: Vec3) -> Option<Frame> {
    let frame = Frame::new(at, rotation);
    let affine = Affine3A::from_mat3_translation(rotation, at);
    hand::box_ok_local(centre, half, affine).then_some(frame)
}

/// The box of a part's rest points: its centre and half extents in the
/// part's own frame.
pub fn rest_box(rest: [Vec3; 4]) -> (Vec3, Vec3) {
    let mut low = rest[0];
    let mut high = rest[0];
    for p in rest.iter().skip(1) {
        low = low.min(*p);
        high = high.max(*p);
    }
    ((low + high) * 0.5, (high - low) * 0.5)
}

/// What a lift's release does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Release {
    /// Put the item back where its lift saved it.
    PutBack,
    /// Leave it where it is: the hold flag clears.
    Drop,
    /// The GUI item slot took it back.
    ToGui,
}

/// The release's outcome, in the original's order: a cancelled (walking)
/// lift goes back to a GUI slot when it is over one and is put back
/// otherwise; a release over a GUI slot is the GUI's; a release with the
/// give-back hover bit leaves the item where it is; any other release puts
/// it back.
pub fn release(walking: bool, hover: u32) -> Release {
    if walking {
        return if hover & HOVER_GUI != 0 {
            Release::ToGui
        } else {
            Release::PutBack
        };
    }
    if hover & HOVER_GUI != 0 {
        Release::ToGui
    } else if hover & HOVER_WORLD != 0 {
        Release::Drop
    } else {
        Release::PutBack
    }
}

/// What the GUI's drop of an inventory entry asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drop {
    /// Nothing is held.
    Nothing,
    /// The drag is not a drag out of an inventory window: the scene is left
    /// alone.
    NotFromGui,
    /// The entry's object has no body: the original's "Game object has no
    /// physics" content error, and nothing is spawned.
    NoBody,
    /// The object is put into the scene as a prop and lifted onto the
    /// cursor.
    Spawn,
}

/// The GUI drop's branch: an entry, whether the drag came out of an
/// inventory window, and whether the object has a body.
pub fn drop(has_entry: bool, from_gui: bool, has_body: bool) -> Drop {
    if !has_entry {
        Drop::Nothing
    } else if !from_gui {
        Drop::NotFromGui
    } else if !has_body {
        Drop::NoBody
    } else {
        Drop::Spawn
    }
}

/// Where a spawned item goes: under the cursor, offset by the camera.
pub fn spawn_frame(cursor: Frame, camera: Frame) -> Frame {
    cursor.under(camera)
}

/// The item the cursor holds: its body, the part the lift took, the frames
/// every part had at the lift, and the box the placement tests.
#[derive(Debug, Clone, PartialEq)]
pub struct HeldItem {
    pub body: Entity,
    pub part: usize,
    /// Every part's world frame at the lift.
    pub saved: Vec<Affine3A>,
    /// The lifted node's frame at the lift.
    pub saved_frame: Frame,
    /// The frame the cursor shows now.
    pub shown: Frame,
    /// The box's centre and half extents in the part's frame (m).
    pub centre: Vec3,
    pub half: Vec3,
    /// The last point the box fitted at (m).
    pub target: Vec3,
}

/// A GUI drop waiting to run: the inventory entry, whether the drag came
/// out of a GUI window, and the pointer it was dropped at. The original
/// hands the task to a worker thread; Exarust runs it on the next frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DropTask {
    /// The dragged item's database id.
    pub item: u32,
    pub from_gui: bool,
    pub pointer: IVec2,
}

/// The GUI item drag's payload kinds, an item and a scene object.
pub const ITEM_PAYLOAD: u32 = 0xFB01;
pub const OBJECT_PAYLOAD: u32 = 0xFB02;

/// What the controls do with a GUI drag that starts over the world view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragClaim {
    /// The drag is the controls': the drop task is queued and the release
    /// callback installed.
    Take,
    /// An item is already held: the drag is left to it.
    Busy,
    /// A dialogue, a suppressed movement request, a pending drop or
    /// another payload: the drag is not the controls'.
    Ignore,
}

/// Whether the controls take a GUI drag of `kind`: no dialogue, the
/// movement request free (a lift or a walk not pending), no drop already
/// queued, and an item or object payload; an item already held makes it
/// busy instead.
pub fn claim_drag(
    kind: u32,
    dialogue: bool,
    request_suppressed: bool,
    pending: bool,
    holding: bool,
) -> DragClaim {
    if dialogue || request_suppressed || pending || (kind != ITEM_PAYLOAD && kind != OBJECT_PAYLOAD)
    {
        return DragClaim::Ignore;
    }
    if holding {
        DragClaim::Busy
    } else {
        DragClaim::Take
    }
}

/// The item cursor's state: the item it holds, the turn's ramp, the
/// accumulated turn, what is under the cursor, and the drops waiting.
#[derive(Resource, Debug, Default)]
pub struct ItemCursor {
    pub held: Option<HeldItem>,
    pub turn: TurnRamp,
    /// The turn's accumulated two-axis offset (radians).
    pub offset: Vec2f,
    /// The item's shown rotation, turned piece by piece as the original's
    /// held frame is.
    pub rotation: Mat3,
    /// The hover word: [`HOVER_GUI`], [`HOVER_WORLD`], [`HOVER_GIVE_BACK`]
    /// or zero.
    pub hover: u32,
    /// The GUI drops queued for the next frame, in order.
    pub drops: Vec<DropTask>,
}

impl ItemCursor {
    /// Begins a lift with `held`.
    pub fn begin(&mut self, held: HeldItem) {
        self.rotation = held.saved_frame.rotation;
        self.held = Some(held);
        self.offset = Vec2f::ZERO;
        self.hover = 0;
        self.turn = TurnRamp::default();
    }

    /// Ends the lift: the outcome for the hover and whether the player is
    /// walking, and the item that was held.
    pub fn end(&mut self, walking: bool) -> Option<(Release, HeldItem)> {
        let held = self.held.take()?;
        let outcome = release(walking, self.hover);
        self.offset = Vec2f::ZERO;
        self.hover = 0;
        Some((outcome, held))
    }

    /// Queues a GUI drop for the next frame.
    pub fn queue_drop(&mut self, task: DropTask) {
        self.drops.push(task);
    }

    /// Takes the queued drops in order, leaving the queue empty.
    pub fn take_drops(&mut self) -> Vec<DropTask> {
        std::mem::take(&mut self.drops)
    }

    /// One frame of the cursor's turn: the offset's new piece and whether
    /// it turned, updating the accumulated offset.
    pub fn turn_frame(
        &mut self,
        now_ms: u64,
        camera_sensitivity: f32,
        look: Vec2f,
        keys: [bool; 4],
        right: Vec3,
    ) -> bool {
        let (delta, turned) = self.turn.frame(now_ms, camera_sensitivity, look, keys);
        if turned {
            self.offset += delta;
            self.rotation *= turn_delta(right, delta);
        }
        turned
    }
}

/// Rotates every part's world frame about `centre` by `rotation`, as the
/// held item cursor's turn writes its node frame.
pub fn turn_parts(worlds: &mut [Affine3A], centre: Vec3, rotation: Mat3) {
    let around = Affine3A::from_translation(centre)
        * Affine3A::from_mat3(rotation)
        * Affine3A::from_translation(-centre);
    for world in worlds {
        *world = around * *world;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The turn starts at 0.25, ramps linearly to 1.0 over 300 ms (the
    /// elapsed time in double), and a 500 ms gap resets it; the unit speed
    /// is the camera's sensitivity times 0.0025.
    #[test]
    fn the_turn_ramps_and_resets() {
        let mut ramp = TurnRamp::default();
        // Frame 1: not turning yet, so the base factor, which starts the
        // ramp. Sensitivity 100 makes the unit 0.25, so 0.25·0.25.
        assert_eq!(
            ramp.frame(1000, 100.0, Vec2f::ZERO, [true, false, false, false]),
            (Vec2f::new(0.0625, 0.0), true)
        );
        // 150 ms later: 0.25 + 150·0.003333·0.75 = 0.625 → 0.625·0.25.
        let (d, _) = ramp.frame(1150, 100.0, Vec2f::ZERO, [true, false, false, false]);
        assert!((d.x - 0.156_25).abs() < 1e-6, "{d}");
        // 301 ms after the ramp began: the fast factor.
        let (d, _) = ramp.frame(1301, 100.0, Vec2f::ZERO, [true, false, false, false]);
        assert_eq!(d.x, 0.25);
        // No turn this frame drops the factor back to the base next frame.
        assert_eq!(
            ramp.frame(1310, 100.0, Vec2f::ZERO, [false; 4]),
            (Vec2f::ZERO, false)
        );
        let (d, _) = ramp.frame(1311, 100.0, Vec2f::ZERO, [false, false, true, false]);
        assert_eq!(d.y, 100.0 * 0.0025 * 0.25);
        // A gap of over 500 ms resets the turning state even after 301 ms.
        let mut ramp = TurnRamp::default();
        ramp.frame(0, 100.0, Vec2f::ZERO, [true, false, false, false]);
        ramp.frame(301, 100.0, Vec2f::ZERO, [true, false, false, false]);
        assert!(ramp.turning);
        let (d, _) = ramp.frame(802, 100.0, Vec2f::new(1.0, 0.0), [false; 4]);
        // The gap resets turning, so the base factor applies to the look.
        assert!((d.x - 1.0 * 100.0 * 0.0025 * 0.25).abs() < 1e-6, "{d}");
        assert!(ramp.turning);
    }

    /// A look stick longer than 0.1 is scaled by the turn speed; its length
    /// is taken in single then compared with the double 0.1, so a single
    /// 0.1 already uses the stick; a NaN look uses the stick branch.
    #[test]
    fn the_stick_is_read_past_the_dead_zone() {
        let mut ramp = TurnRamp::default();
        let (d, turned) = ramp.frame(0, 100.0, Vec2f::new(1.0, 0.0), [true, true, true, true]);
        assert!(turned && (d.x - 0.0625).abs() < 1e-6, "{d}");
        let mut ramp = TurnRamp::default();
        // A 0.09-long look is inside the dead zone: no key is held, so it
        // does not turn.
        let (d, _) = ramp.frame(
            0,
            100.0,
            Vec2f::new(0.09, 0.0),
            [false, false, false, false],
        );
        assert_eq!(d, Vec2f::ZERO);
        // The single 0.1 is just over the double 0.1: it takes the stick
        // branch.
        let mut ramp = TurnRamp::default();
        let (d, _) = ramp.frame(0, 100.0, Vec2f::new(0.1, 0.0), [false, false, false, false]);
        assert!((d.x - 0.1 * 0.25 * 0.25).abs() < 1e-7, "{d}");
        let mut ramp = TurnRamp::default();
        let (_, turned) = ramp.frame(0, 100.0, Vec2f::new(f32::NAN, 0.0), [false; 4]);
        assert!(!turned);
    }

    /// The turn is composed across the camera's right axis then up the
    /// world's axis; the saved rotation is applied first.
    #[test]
    fn the_turn_composes_two_rotations() {
        let saved = Mat3::IDENTITY;
        let turned = turned_rotation(saved, Vec3::X, Vec2f::new(std::f32::consts::FRAC_PI_2, 0.0));
        let v = turned * Vec3::Y;
        assert!((v - Vec3::Z).length() < 1e-5, "{v}");
        // The up turn alone leaves the world's up alone.
        let turned = turned_rotation(saved, Vec3::X, Vec2f::new(0.0, std::f32::consts::FRAC_PI_2));
        assert!((turned * Vec3::Y - Vec3::Y).length() < 1e-5);
        assert!((turned * Vec3::X - Vec3::NEG_Z).length() < 1e-5);
    }

    /// A hit at least as far as the limit is taken; a nearer hit and a miss
    /// fall back to the ray's crossing of the limit plane, and a climbing
    /// ray has none.
    #[test]
    fn the_placement_point() {
        let o = Vec3::new(0.0, 2.0, 0.0);
        let down = Vec3::NEG_Y;
        let at = Vec3::new(0.5, 0.0, 0.5);
        assert_eq!(place_point(o, down, Some((at, 2.0)), -5.0), Some(at));
        // A hit nearer than the limit falls back to the plane at the limit
        // (5 cm, 0.05 m), which the descending ray meets behind the origin.
        let back = place_point(o, down, Some((at, 2.0)), 5.0).unwrap();
        assert!((back.y - 0.05).abs() < 1e-6, "{back}");
        // The lift's own limit is 1000 m below: a hit is always taken, and
        // a miss lands there.
        let far = place_point(o, down, None, PLACEMENT_LIMIT_CM).unwrap();
        assert!((far.y + 1000.0).abs() < 1e-3, "{far}");
        assert!((far.x - 0.0).abs() < 1e-6);
        assert_eq!(
            place_point(o, down, None, PLACEMENT_LIMIT_CM).map(|p| p.y),
            Some(-1000.0)
        );
        assert_eq!(place_point(o, Vec3::Y, None, PLACEMENT_LIMIT_CM), None);
        assert_eq!(place_point(o, Vec3::X, None, PLACEMENT_LIMIT_CM), None);
        // A NaN hit passes the "not nearer" test.
        assert_eq!(place_point(o, down, Some((at, f32::NAN)), 0.0), Some(at));
        // A NaN distance is a hit too.
        assert_eq!(place_point(o, down, Some((at, f32::NAN)), 5.0), Some(at));
    }

    /// The placement frame is refused when the item's box leaves 280 cm up
    /// or −1000 cm down in the turned frame.
    #[test]
    fn the_placement_frame_refuses_a_bad_box() {
        let centre = Vec3::ZERO;
        let half = Vec3::new(0.1, 0.1, 0.1);
        assert!(place_frame(Mat3::IDENTITY, Vec3::new(0.0, 1.0, 0.0), centre, half).is_some());
        assert!(place_frame(Mat3::IDENTITY, Vec3::new(0.0, 3.0, 0.0), centre, half).is_none());
        // Turned a quarter about X, a long box that fitted upright no
        // longer does: its 3 m half length lies across and its top leaves
        // the 280 cm limit.
        let long = Vec3::new(0.05, 0.05, 3.0);
        let turned = turned_rotation(
            Mat3::IDENTITY,
            Vec3::X,
            Vec2f::new(std::f32::consts::FRAC_PI_2, 0.0),
        );
        assert!(place_frame(turned, Vec3::new(0.0, 1.0, 0.0), centre, long).is_none());
    }

    /// The GUI drag claim: only an item or object payload with no dialogue,
    /// no suppressed request and no queued drop is taken; an item already
    /// held makes it busy.
    #[test]
    fn the_gui_drag_claim() {
        assert_eq!(
            claim_drag(ITEM_PAYLOAD, false, false, false, false),
            DragClaim::Take
        );
        assert_eq!(
            claim_drag(OBJECT_PAYLOAD, false, false, false, false),
            DragClaim::Take
        );
        assert_eq!(
            claim_drag(ITEM_PAYLOAD, true, false, false, false),
            DragClaim::Ignore
        );
        assert_eq!(
            claim_drag(ITEM_PAYLOAD, false, true, false, false),
            DragClaim::Ignore
        );
        assert_eq!(
            claim_drag(ITEM_PAYLOAD, false, false, true, false),
            DragClaim::Ignore
        );
        assert_eq!(
            claim_drag(0x8004, false, false, false, false),
            DragClaim::Ignore
        );
        assert_eq!(
            claim_drag(ITEM_PAYLOAD, false, false, false, true),
            DragClaim::Busy
        );
    }

    /// A part's rest points box: the centre is their middle, the half the
    /// extremes.
    #[test]
    fn the_rest_box() {
        let (c, h) = rest_box([
            Vec3::new(-1.0, 0.0, -2.0),
            Vec3::new(1.0, 0.0, -2.0),
            Vec3::new(0.0, 2.0, 2.0),
            Vec3::new(0.0, 0.0, 2.0),
        ]);
        assert_eq!(c, Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(h, Vec3::new(1.0, 1.0, 2.0));
    }

    /// The release: a cancelled lift goes to the GUI or back; a GUI slot
    /// takes it; the give-back hover leaves it; anything else puts it back.
    #[test]
    fn the_release_order() {
        assert_eq!(release(true, 0), Release::PutBack);
        assert_eq!(release(true, HOVER_GUI), Release::ToGui);
        assert_eq!(release(true, HOVER_GIVE_BACK), Release::PutBack);
        assert_eq!(release(false, 0), Release::PutBack);
        assert_eq!(release(false, HOVER_GUI), Release::ToGui);
        assert_eq!(release(false, HOVER_GUI | HOVER_WORLD), Release::ToGui);
        assert_eq!(release(false, HOVER_WORLD), Release::Drop);
        assert_eq!(release(false, HOVER_GIVE_BACK), Release::Drop);
    }

    /// The GUI drop: a drag out of an inventory with a body spawns; none of
    /// the other branches do.
    #[test]
    fn the_gui_drop() {
        assert_eq!(drop(false, true, true), Drop::Nothing);
        assert_eq!(drop(true, false, true), Drop::NotFromGui);
        assert_eq!(drop(true, true, false), Drop::NoBody);
        assert_eq!(drop(true, true, true), Drop::Spawn);
    }
}
