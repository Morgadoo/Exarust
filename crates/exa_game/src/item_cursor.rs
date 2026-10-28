//! The item cursor: the item the left button lifts onto the cursor, the way
//! the mouse turns it, where it may be put down, and what its release does.
//! Rules: `docs/PLAYER.md` (The hand).
//!
//! **The lift.** Without Ctrl the left press on a prop the hand can move
//! lifts it: the item leaves the world's hands and rides the cursor, held by
//! the item cursor (Exarust holds it with a plain grab of strength 1, its
//! stand-in for the original moving the node itself), while the item GUI
//! shows its icon (`window`). Its rest points at the lift are saved. The
//! item cursor's own frame turns with the movement inputs, which the lift
//! leaves to it: the pad's raw move stick, or the move keys when the stick's
//! squared length is not over 0.1 (double precision; a NaN uses the keys).
//! The stick's forward part turns the item about the camera's axis and its
//! sideways part about the world's up axis; Forward/Back give ± the speed
//! about the camera's axis (Forward winning when both are held), Left/Right
//! ± the speed about the up axis (Left winning). The mouse does not turn
//! the item. The turn starts at 0.25 of the unit speed, and once the cursor
//! has been turning for more than 300 ms it ramps to 1.0; 500 ms without a
//! call drops it back to the base speed. The unit speed is the scene's last
//! frame time in milliseconds times 0.0025 rad, multiplied in double
//! precision, so the turn is about 0.6 rad/s at the base speed and 2.5 rad/s
//! at the fast one.
//!
//! **Where it may go.** Each frame the cursor's ray is picked against the
//! level; a hit whose height is not under the placement limit (−100000 cm,
//! which no level reaches) places the item there. Otherwise the item goes
//! where the ray crosses that height; a ray that climbs (its height
//! component over −1.19209e−7) refuses the placement. The item is put there
//! whether or not its box fits: the box test (its rest points and half
//! extents in its turned saved frame at most 280 cm high and at least
//! −1000 cm low, `hand::box_ok_local`) only decides the routine's answer.
//! The original then slides the item along the ray from 600 cm back to its
//! first contact (at most 1000 cm on, [`drag_hit`]); Exarust has no mesh
//! sweep and pulls the grab to the picked point, and a cursor that picks
//! nothing keeps the last point (its pick has no navigation mesh behind
//! every pixel).
//!
//! **The release.** A plain release leaves the item where the cursor put
//! it: the original's release hands the saved rest points to the deferred
//! restore only to move the body out of its old sector, and the body keeps
//! its current points. A release over a GUI item slot is the GUI's. A lift
//! cancelled by the movement request goes to the GUI when it is over a
//! slot, stays where it is with the world hover bit, and is otherwise put
//! back at the saved rest points. The GUI drop spawn path takes an inventory entry
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
/// The unit turn speed is the frame time (ms) times this (double, rad).
pub const TURN_PER_MS: f64 = 0.0025;
/// A move stick whose squared length is not over this is read as the turn
/// keys (double).
pub const STICK_DEAD_ZONE: f64 = 0.1;
/// A turn under this is no turn (the engine's float epsilon).
pub const TURN_EPSILON: f32 = 1.192_09e-7;
/// The placement's height limit (cm): a hit under it, or no hit, falls
/// back to where the ray crosses this height.
pub const PLACEMENT_LIMIT_CM: f32 = -100_000.0;
/// The placement sweep's reach (cm): the original's nearest-contact search
/// starts from it and keeps only nearer contacts.
pub const DRAG_REACH_CM: f32 = 1000.0;
/// The lift's grab strength (Exarust's stand-in hold).
pub const LIFT_STRENGTH: f32 = 1.0;
/// The hover word's bits: over a GUI item slot, over the world, and the
/// "give back" word that leaves the item where it is.
pub const HOVER_GUI: u32 = 1;
pub const HOVER_WORLD: u32 = 0x10;
pub const HOVER_GIVE_BACK: u32 = 0x30;
/// The link byte a world node carries when the item cursor must give the
/// item back rather than leave it there.
pub const GIVE_BACK_LINK: u8 = b'L';

/// A GUI item slot under the cursor: the item it shows, its top-left and
/// its side in the same pixels as the pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemSlot {
    pub id: u32,
    pub at: (i32, i32),
    pub side: i32,
}

/// The top-most item slot holding the point. The panel keeps its entries
/// in draw order, the last on top, and an icon is its square from its
/// top-left, the right and bottom edges excluded as the panel's hit test
/// does.
pub fn slot_at(slots: &[ItemSlot], x: i32, y: i32) -> Option<u32> {
    slots.iter().rev().find_map(|slot| {
        (x >= slot.at.0
            && y >= slot.at.1
            && x < slot.at.0.saturating_add(slot.side)
            && y < slot.at.1.saturating_add(slot.side))
        .then_some(slot.id)
    })
}

/// The item cursor's hover word for a frame: a GUI slot takes the item back
/// ([`HOVER_GUI`]); otherwise a world hit leaves it where it is
/// ([`HOVER_WORLD`]), and a node whose link byte reads [`GIVE_BACK_LINK`]
/// carries the give-back word ([`HOVER_GIVE_BACK`]). Nothing under the
/// cursor is no word.
pub fn hover_word(slot: Option<u32>, world: bool, give_back: bool) -> u32 {
    if slot.is_some() {
        HOVER_GUI
    } else if world {
        if give_back {
            HOVER_GIVE_BACK
        } else {
            HOVER_WORLD
        }
    } else {
        0
    }
}

/// Whether a world node's link byte asks the item back.
pub fn gives_back(link: u8) -> bool {
    link == GIVE_BACK_LINK
}

/// The mouse turn's running speed: the frame clock, the ramp's start and
/// whether the cursor was turning last call.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TurnRamp {
    /// The previous call's game clock (ms).
    pub last_ms: u64,
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
        if self.last_ms.saturating_add(IDLE_MS) < now_ms {
            self.turning = false;
        }
        self.last_ms = now_ms;
        if !self.turning {
            BASE_TURN
        } else if self.ramp_ms.saturating_add(RAMP_MS) < now_ms {
            FAST_TURN
        } else {
            let elapsed = (now_ms.saturating_sub(self.ramp_ms)) as f64;
            (elapsed * RAMP_RATE * RAMP_SHARE + f64::from(BASE_TURN)) as f32
        }
    }

    /// One frame's turn: the speed factor times the frame time `frame_ms`
    /// (ms) times [`TURN_PER_MS`] (in double, narrowed once), and the
    /// two-axis offset it moves. `stick` is the pad's raw move stick: when
    /// its squared length (single, compared in double) is over
    /// [`STICK_DEAD_ZONE`] the offset's first part is its forward part times
    /// the speed and the second its sideways part; otherwise (a NaN too)
    /// `keys` = [forward, back, left, right] give +speed (forward, else
    /// −speed for back) on the first part and +speed (left, else −speed for
    /// right) on the second. Returns the offset and whether either part is
    /// over [`TURN_EPSILON`] in size; the ramp starts on the first turning
    /// frame.
    pub fn frame(
        &mut self,
        now_ms: u64,
        frame_ms: f32,
        stick: Vec2f,
        keys: [bool; 4],
    ) -> (Vec2f, bool) {
        let factor = self.factor(now_ms);
        let speed = (f64::from(factor) * (f64::from(frame_ms) * TURN_PER_MS)) as f32;
        let squared = f64::from(stick.x * stick.x + stick.y * stick.y);
        let delta = if squared > STICK_DEAD_ZONE {
            Vec2f::new(stick.y * speed, stick.x * speed)
        } else {
            let across = if keys[0] {
                speed
            } else if keys[1] {
                -speed
            } else {
                0.0
            };
            let up = if keys[2] {
                speed
            } else if keys[3] {
                -speed
            } else {
                0.0
            };
            Vec2f::new(across, up)
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

/// The placement sweep's nearest contact under [`DRAG_REACH_CM`], in cm: a
/// contact replaces the best so far only when it is nearer (the original's
/// ordered compare, so a NaN never replaces it); no contact under the reach
/// gives none (the original then keeps the reach itself).
pub fn drag_hit(hits: impl IntoIterator<Item = f32>) -> Option<f32> {
    hits.into_iter()
        .filter(|d| *d < DRAG_REACH_CM)
        .fold(None, |best: Option<f32>, d| {
            Some(best.map_or(d, |b| b.min(d)))
        })
}

/// One collision sphere of the dragged item, in its placed frame (m): the
/// centre offset from the item's point and the radius.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SweepSphere {
    pub centre: Vec3,
    pub radius: f32,
}

/// The item's first contact along the cursor's ray, in cm. The original
/// starts the search at [`DRAG_REACH_CM`] and pulls the item back to the
/// nearest sphere-versus-mesh contact; a contact replaces the best only
/// when it is nearer, so a NaN never does. `sweep` is the caller's shape
/// sweep: it takes a sphere's centre, the unit direction, the reach and
/// the radius (all m) and gives the distance along the ray (m). `None`
/// when nothing is nearer than the reach (the original then keeps the
/// reach itself).
pub fn sweep_to_contact(
    spheres: &[SweepSphere],
    direction: Vec3,
    mut sweep: impl FnMut(Vec3, Vec3, f32, f32) -> Option<f32>,
) -> Option<f32> {
    let reach = DRAG_REACH_CM / 100.0;
    let mut best = DRAG_REACH_CM;
    let mut found = false;
    for sphere in spheres {
        let Some(toi) = sweep(sphere.centre, direction, reach, sphere.radius) else {
            continue;
        };
        let cm = toi * 100.0;
        if cm < best {
            best = cm;
            found = true;
        }
    }
    found.then_some(best)
}

/// Where the placement sweep stops along a ray from `origin` (m): the
/// nearer of the ray's pick `hit` (cm) and the item's first contact
/// `contact` (cm), as a distance along the ray in cm. The original starts
/// its contact search at the 1000 cm reach and keeps the nearest; no
/// contact uses the pick. A point off the ray's finite end has none.
pub fn placement_distance(hit: f32, contact: Option<f32>) -> Option<f32> {
    let distance = match contact {
        Some(contact) if contact < hit => contact,
        _ => hit,
    };
    (0.0..=DRAG_REACH_CM)
        .contains(&distance)
        .then_some(distance)
}

/// The world point of a placement distance (cm) along a ray from `origin`
/// (m), or none when it is not finite.
pub fn placement_point(origin: Vec3, direction: Vec3, distance_cm: f32) -> Option<Vec3> {
    let at = origin + direction * (distance_cm / 100.0);
    at.is_finite().then_some(at)
}

/// Where the cursor's ray puts the item (m): the hit `at` when its height
/// (`height_cm`, the hit point's height in cm) is not under `limit` (cm;
/// the original's `height < limit` test, so a NaN height keeps the hit),
/// else the point where the ray crosses the height `limit`. A ray climbing
/// more than the float epsilon (its height part over −[`TURN_EPSILON`])
/// gives no point.
pub fn place_point(origin: Vec3, dir: Vec3, hit: Option<(Vec3, f32)>, limit: f32) -> Option<Vec3> {
    match hit {
        Some((at, height_cm)) if height_cm.is_nan() || height_cm >= limit => Some(at),
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
/// positioned at `at`, and whether its box fits there
/// (`hand::box_ok_local`). The original places the item either way; the
/// box only decides the placement routine's answer.
pub fn place_frame(rotation: Mat3, at: Vec3, centre: Vec3, half: Vec3) -> (Frame, bool) {
    let frame = Frame::new(at, rotation);
    let affine = Affine3A::from_mat3_translation(rotation, at);
    (frame, hand::box_ok_local(centre, half, affine))
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

/// The release's outcome. A lift the movement request cancelled goes, in
/// the original's order, to the GUI slot it is over, stays where it is with
/// the world hover bit (the give-back word also ends the hold), and is
/// otherwise put back at its saved rest points. A plain release is the
/// GUI's over a slot and otherwise leaves the item where the cursor put it:
/// the original's release passes the saved rest points to the deferred
/// restore only to take the body out of its old sector, and the body keeps
/// its current points. [The GUI slot's acceptance test of the cancelled
/// path is not modelled: a slot always takes it.]
pub fn release(cancelled: bool, hover: u32) -> Release {
    if hover & HOVER_GUI != 0 {
        Release::ToGui
    } else if !cancelled || hover & HOVER_WORLD != 0 {
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

/// The record a GUI drag spawns: the dragged item's database id and its
/// condition flags, for an item or scene-object payload. The original asks
/// the object database for a fresh record (same id, the payload's flags as
/// its condition) and attaches the new node under the viewport's world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpawnRecord {
    pub item: u32,
    pub flags: u32,
}

/// The spawned record of a GUI drag of `kind`: an item payload carries its
/// item id and flags; an object payload carries the condition word and no
/// item id. Any other payload starts nothing.
pub fn spawn_record(kind: u32, item: u32, flags: u32) -> Option<SpawnRecord> {
    match kind {
        ITEM_PAYLOAD => Some(SpawnRecord { item, flags }),
        OBJECT_PAYLOAD => Some(SpawnRecord { item: 0, flags }),
        _ => None,
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
    /// The drag is the controls': the release callback is installed, the
    /// drop task queued for an item payload only ([`ITEM_PAYLOAD`]; an
    /// object payload queues none), and the pending word set.
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

    /// Ends the lift: the outcome for the hover and whether the movement
    /// request cancelled it, and the item that was held.
    pub fn end(&mut self, cancelled: bool) -> Option<(Release, HeldItem)> {
        let held = self.held.take()?;
        let outcome = release(cancelled, self.hover);
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
        frame_ms: f32,
        stick: Vec2f,
        keys: [bool; 4],
        right: Vec3,
    ) -> bool {
        let (delta, turned) = self.turn.frame(now_ms, frame_ms, stick, keys);
        if turned {
            self.offset += delta;
            self.rotation *= turn_delta(right, delta);
        }
        turned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The turn starts at 0.25, ramps linearly to 1.0 over 300 ms (the
    /// elapsed time in double), and a 500 ms gap resets it; the unit speed
    /// is the frame time (ms) times 0.0025.
    #[test]
    fn the_turn_ramps_and_resets() {
        let mut ramp = TurnRamp::default();
        // Frame 1: not turning yet, so the base factor, which starts the
        // ramp. A 100 ms frame makes the unit 0.25, so 0.25·0.25.
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
        // The gap resets turning, so the base factor applies to the stick,
        // whose sideways part turns about the up axis.
        assert!((d.y - 1.0 * 100.0 * 0.0025 * 0.25).abs() < 1e-6, "{d}");
        assert_eq!(d.x, 0.0);
        assert!(ramp.turning);
    }

    /// The move stick is read when its squared length is over the double
    /// 0.1, its forward part on the first axis and its sideways part on the
    /// second; under that (or NaN) the keys are read, Forward over Back and
    /// Left over Right.
    #[test]
    fn the_stick_is_read_past_the_dead_zone() {
        let mut ramp = TurnRamp::default();
        // A full forward stick: 0.25 (base) · 100 · 0.0025 = 0.0625 on the
        // camera axis; the keys are ignored.
        let (d, turned) = ramp.frame(0, 100.0, Vec2f::new(0.0, 1.0), [false, true, false, true]);
        assert!(turned, "{d}");
        assert_eq!(d, Vec2f::new(0.0625, 0.0));
        // A stick 0.3 long has the squared length 0.09: inside the dead
        // zone, so with no key held it does not turn.
        let mut ramp = TurnRamp::default();
        let (d, turned) = ramp.frame(0, 100.0, Vec2f::new(0.3, 0.0), [false; 4]);
        assert!(!turned);
        assert_eq!(d, Vec2f::ZERO);
        // 0.33² = 0.1089 is over 0.1: the stick turns about the up axis.
        let mut ramp = TurnRamp::default();
        let (d, _) = ramp.frame(0, 100.0, Vec2f::new(0.33, 0.0), [false; 4]);
        assert!((d.y - 0.33 * 0.0625).abs() < 1e-7, "{d}");
        // A NaN stick reads the keys: Forward and Back held give +speed,
        // Right alone gives −speed.
        let mut ramp = TurnRamp::default();
        let (d, turned) = ramp.frame(
            0,
            100.0,
            Vec2f::new(f32::NAN, 0.0),
            [true, true, false, true],
        );
        assert!(turned);
        assert_eq!(d, Vec2f::new(0.0625, -0.0625));
        // Left wins over Right.
        let mut ramp = TurnRamp::default();
        let (d, _) = ramp.frame(0, 100.0, Vec2f::ZERO, [false, false, true, true]);
        assert_eq!(d, Vec2f::new(0.0, 0.0625));
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
    fn the_drag_reach() {
        assert_eq!(drag_hit([1200.0, 400.0]), Some(400.0));
        assert_eq!(drag_hit([1200.0, 1000.0]), None);
        assert_eq!(drag_hit([999.9]), Some(999.9));
        assert_eq!(drag_hit([f32::NAN, 5.0]), Some(5.0));
        assert_eq!(drag_hit([]), None);
    }

    /// The first contact starts at the 1000 cm reach, keeps only a strictly
    /// nearer sphere contact and ignores a NaN; a placement takes the
    /// nearer of the pick and the contact.
    #[test]
    fn the_sweep_finds_the_first_contact() {
        let spheres = [
            SweepSphere {
                centre: Vec3::new(0.0, 1.0, 0.0),
                radius: 0.1,
            },
            SweepSphere {
                centre: Vec3::new(0.0, 0.5, 0.0),
                radius: 0.2,
            },
        ];
        let mut seen = Vec::new();
        let contact = sweep_to_contact(&spheres, Vec3::NEG_Y, |centre, dir, max, radius| {
            seen.push((centre, dir, max, radius));
            // The higher sphere meets at 3 m, the lower at 7.5 m.
            if centre.y > 0.75 {
                Some(3.0)
            } else {
                Some(7.5)
            }
        });
        assert_eq!(contact, Some(300.0));
        assert_eq!(seen.len(), 2);
        assert_eq!(seen[0].2, 10.0);
        assert_eq!(seen[0].3, 0.1);
        // Nothing nearer than the reach gives none.
        assert_eq!(
            sweep_to_contact(&spheres, Vec3::NEG_Y, |_, _, _, _| Some(11.0)),
            None
        );
        // A NaN contact never replaces the best; the next one does.
        assert_eq!(
            sweep_to_contact(&spheres, Vec3::NEG_Y, |centre, _, _, _| {
                if centre.y > 0.75 {
                    Some(f32::NAN)
                } else {
                    Some(4.0)
                }
            }),
            Some(400.0)
        );
        assert_eq!(placement_distance(500.0, Some(300.0)), Some(300.0));
        assert_eq!(placement_distance(500.0, None), Some(500.0));
        assert_eq!(placement_distance(1200.0, Some(1300.0)), None);
        assert_eq!(placement_distance(-1.0, None), None);
        let at = placement_point(Vec3::ZERO, Vec3::NEG_Y, 300.0).unwrap();
        assert_eq!(at, Vec3::new(0.0, -3.0, 0.0));
    }

    /// A GUI slot is the top-most square under the point; the hover word is
    /// the slot, else the world word, the give-back word or none.
    #[test]
    fn the_item_slots_and_hover_words() {
        let slots = [
            ItemSlot {
                id: 7,
                at: (10, 20),
                side: 40,
            },
            ItemSlot {
                id: 9,
                at: (20, 25),
                side: 10,
            },
        ];
        assert_eq!(slot_at(&slots, 10, 20), Some(7));
        assert_eq!(slot_at(&slots, 25, 30), Some(9));
        assert_eq!(slot_at(&slots, 29, 30), Some(9));
        assert_eq!(slot_at(&slots, 30, 30), Some(7));
        assert_eq!(slot_at(&slots, 35, 30), Some(7));
        assert_eq!(slot_at(&slots, 50, 30), None);
        assert_eq!(slot_at(&slots, 9, 20), None);
        assert_eq!(hover_word(Some(9), true, true), HOVER_GUI);
        assert_eq!(hover_word(None, true, true), HOVER_GIVE_BACK);
        assert_eq!(hover_word(None, true, false), HOVER_WORLD);
        assert_eq!(hover_word(None, false, true), 0);
        assert!(gives_back(GIVE_BACK_LINK));
        assert!(!gives_back(b'K'));
    }

    /// A GUI drag spawns the dragged item with its flags, or a scene
    /// object's condition word; another payload spawns none.
    #[test]
    fn the_gui_drag_spawns_its_record() {
        assert_eq!(
            spawn_record(ITEM_PAYLOAD, 42, 0x10),
            Some(SpawnRecord {
                item: 42,
                flags: 0x10
            })
        );
        assert_eq!(
            spawn_record(OBJECT_PAYLOAD, 42, 3),
            Some(SpawnRecord { item: 0, flags: 3 })
        );
        assert_eq!(spawn_record(0x8004, 42, 0), None);
    }

    #[test]
    fn the_placement_point() {
        let o = Vec3::new(0.0, 2.0, 0.0);
        let down = Vec3::NEG_Y;
        let at = Vec3::new(0.5, 0.0, 0.5);
        assert_eq!(place_point(o, down, Some((at, 2.0)), -5.0), Some(at));
        // A hit 2 cm high is under a 5 cm limit: the point falls back to
        // the plane at the limit (0.05 m), which the descending ray meets
        // above the hit.
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
        // A NaN height is not "under" the limit: the hit is kept.
        assert_eq!(place_point(o, down, Some((at, f32::NAN)), 0.0), Some(at));
        assert_eq!(place_point(o, down, Some((at, f32::NAN)), 5.0), Some(at));
    }

    /// The placement frame is placed either way; its answer says whether
    /// the item's box stays within 280 cm up and −1000 cm down in the
    /// turned frame.
    #[test]
    fn the_placement_frame_refuses_a_bad_box() {
        let centre = Vec3::ZERO;
        let half = Vec3::new(0.1, 0.1, 0.1);
        assert!(place_frame(Mat3::IDENTITY, Vec3::new(0.0, 1.0, 0.0), centre, half).1);
        assert!(!place_frame(Mat3::IDENTITY, Vec3::new(0.0, 3.0, 0.0), centre, half).1);
        let (frame, _) = place_frame(Mat3::IDENTITY, Vec3::new(0.0, 3.0, 0.0), centre, half);
        assert_eq!(frame.position, Vec3::new(0.0, 3.0, 0.0));
        // Turned a quarter about X, a long box that fitted upright no
        // longer does: its 3 m half length lies across and its top leaves
        // the 280 cm limit.
        let long = Vec3::new(0.05, 0.05, 3.0);
        let turned = turned_rotation(
            Mat3::IDENTITY,
            Vec3::X,
            Vec2f::new(std::f32::consts::FRAC_PI_2, 0.0),
        );
        assert!(!place_frame(turned, Vec3::new(0.0, 1.0, 0.0), centre, long).1);
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

    /// The lift's lifecycle: the begin leaves a held item and its saved
    /// rotation, the turn adds up, the release decides by the hover, and the
    /// GUI drops wait in order for one frame.
    #[test]
    fn the_cursor_lifecycle_and_drop_queue() {
        let mut cursor = ItemCursor::default();
        cursor.begin(HeldItem {
            body: Entity::PLACEHOLDER,
            part: 0,
            saved: Vec::new(),
            saved_frame: Frame::default(),
            shown: Frame::default(),
            centre: Vec3::ZERO,
            half: Vec3::ONE,
            target: Vec3::ZERO,
        });
        assert!(cursor.held.is_some());
        assert_eq!(cursor.rotation, Mat3::IDENTITY);
        assert!(cursor.turn_frame(0, 100.0, Vec2f::new(0.0, 1.0), [false; 4], Vec3::X));
        assert!((cursor.offset.x - 0.0625).abs() < 1e-6);
        assert!((cursor.rotation * Vec3::Y - Vec3::Y).length() > 0.01);
        cursor.hover = HOVER_GUI;
        cursor.queue_drop(DropTask {
            item: 3,
            from_gui: true,
            pointer: IVec2::new(4, 5),
        });
        cursor.queue_drop(DropTask {
            item: 4,
            from_gui: false,
            pointer: IVec2::ZERO,
        });
        assert_eq!(cursor.take_drops().len(), 2);
        assert!(cursor.take_drops().is_empty());
        let (outcome, held) = cursor.end(false).unwrap();
        assert_eq!(outcome, Release::ToGui);
        assert_eq!(held.part, 0);
        assert!(cursor.held.is_none());
        assert_eq!(cursor.end(false), None);
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

    /// The release: a plain release leaves the item where it is unless a
    /// GUI slot takes it; a cancelled lift goes to the slot, stays with the
    /// world hover bit, and is otherwise put back.
    #[test]
    fn the_release_order() {
        assert_eq!(release(false, 0), Release::Drop);
        assert_eq!(release(false, HOVER_WORLD), Release::Drop);
        assert_eq!(release(false, HOVER_GUI), Release::ToGui);
        assert_eq!(release(true, 0), Release::PutBack);
        assert_eq!(release(true, HOVER_GUI), Release::ToGui);
        assert_eq!(release(true, HOVER_GUI | HOVER_WORLD), Release::ToGui);
        assert_eq!(release(true, HOVER_WORLD), Release::Drop);
        assert_eq!(release(true, HOVER_GIVE_BACK), Release::Drop);
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
