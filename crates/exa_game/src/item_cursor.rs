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
//! first contact (at most 1000 cm on, [`per_sphere_contact`]): every
//! collision sphere of every part is swept against the level's meshes, and
//! the nearest contact wins; a cursor that picks nothing keeps the last
//! point (its pick has no navigation mesh behind every pixel). A level
//! placement marker named for the item's object or model snaps the item
//! onto the marker's frame when the cursor's ray passes within the marker's
//! radius and the item fits there ([`snap_to_marker`]); its link byte `L`
//! gives the give-back word. Exarust's level loader does not register the
//! marker nodes yet, so the cursor's marker list is empty in the game.
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
/// The hover word's bits. [The live item cursor keeps them as the earlier
/// pass named them; in the original the lift's frame sets 1 for a free
/// world placement, 0x10 for an item snapped onto a matching level
/// placement marker, and 0x30 for a marker whose link byte reads `L`.]
pub const HOVER_GUI: u32 = 1;
pub const HOVER_WORLD: u32 = 0x10;
pub const HOVER_GIVE_BACK: u32 = 0x30;
/// The placement marker's link byte that gives the give-back word.
pub const GIVE_BACK_LINK: u8 = b'L';

/// The lift's hover word for a frame, in the original's order: a level
/// placement marker the item snaps onto gives [`HOVER_GIVE_BACK`] when its
/// link byte is [`GIVE_BACK_LINK`] and [`HOVER_WORLD`] otherwise; else a
/// free placement that passed its checks gives [`HOVER_GUI`] (the bit 1
/// word); else none. `marker` is the snapped marker's link byte.
pub fn hover_word(marker: Option<u8>, placed: bool) -> u32 {
    match marker {
        Some(GIVE_BACK_LINK) => HOVER_GIVE_BACK,
        Some(_) => HOVER_WORLD,
        None if placed => HOVER_GUI,
        None => 0,
    }
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

/// The part a collision sphere belongs to, so a hit can name the part the
/// placement stopped against (the original sweeps every part's spheres).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SphereOf {
    pub part: usize,
    pub sphere: SweepSphere,
}

/// A sphere's first contact: the part and the distance along the ray, in cm
/// from the sweep's start.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SphereHit {
    pub part: usize,
    /// The distance along the ray in cm, from the sweep's start.
    pub contact: f32,
}

/// The first contact of every collision sphere of every part of the dragged
/// item, in part and sphere order: `sweep` takes the part, the sphere's
/// centre, the unit direction, the reach and the radius (all m) and gives
/// the distance along the ray (m), or none when that sphere misses. A hit
/// is kept only when it is nearer than the best so far (the original's
/// ordered compare, so a NaN never is), and the best starts at the reach.
/// A contact exactly at the reach is not nearer and is not kept.
pub fn per_sphere_contact(
    spheres: impl IntoIterator<Item = SphereOf>,
    direction: Vec3,
    mut sweep: impl FnMut(usize, Vec3, Vec3, f32, f32) -> Option<f32>,
) -> Option<SphereHit> {
    let reach = DRAG_REACH_CM;
    let mut best: Option<SphereHit> = None;
    for sphere in spheres {
        let Some(toi) = sweep(
            sphere.part,
            sphere.sphere.centre,
            direction,
            DRAG_REACH_CM / 100.0,
            sphere.sphere.radius,
        ) else {
            continue;
        };
        let contact = toi * 100.0;
        if contact < reach && best.is_none_or(|b| contact < b.contact) {
            best = Some(SphereHit {
                part: sphere.part,
                contact,
            });
        }
    }
    best
}

/// How far before the picked point the placement sweep starts (cm): the
/// original backs the ray's point off by 600 along the ray and then
/// searches [`DRAG_REACH_CM`] on from there.
pub const SWEEP_BACK_CM: f32 = 600.0;

/// The item's first contact along the cursor's ray, in cm from the sweep's
/// start, through [`per_sphere_contact`]: every sphere is swept and only a
/// contact nearer than the best so far (starting at [`DRAG_REACH_CM`]) is
/// kept, so a NaN never is. `sweep` takes a sphere's centre, the unit
/// direction, the reach and the radius (all m) and gives the distance along
/// the ray (m). [The original also pulls a sphere already under the level's
/// support plane back along the ray before its cast; that offset is not
/// modelled.]
pub fn sweep_to_contact(
    spheres: &[SweepSphere],
    direction: Vec3,
    mut sweep: impl FnMut(Vec3, Vec3, f32, f32) -> Option<f32>,
) -> Option<f32> {
    per_sphere_contact(
        spheres.iter().map(|sphere| SphereOf {
            part: 0,
            sphere: *sphere,
        }),
        direction,
        |_, centre, dir, reach, radius| sweep(centre, dir, reach, radius),
    )
    .map(|hit| hit.contact)
}

/// Where the placement stops along the ray (cm from its origin): the
/// sweep's contact when it is nearer than the picked point, else the
/// pick (a NaN contact keeps the pick).
pub fn placement_distance(hit: f32, contact: Option<f32>) -> f32 {
    match contact {
        Some(contact) if contact < hit => contact,
        _ => hit,
    }
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

/// A level placement marker an item can snap onto: a node of the level's
/// scene named for the item's object or model, its frame, the radius the
/// cursor's ray must pass within, and the link byte that decides what the
/// release does (`L` gives the item back).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlacementMarker {
    /// The marker's eight-byte name, read as a little-endian word.
    pub name: u64,
    pub frame: Frame,
    /// The radius of the placement sphere (m).
    pub radius: f32,
    /// The marker's link byte.
    pub link: u8,
}

/// Where a placement marker takes the item: the marker's frame and the
/// lift's hover word ([`HOVER_WORLD`], or [`HOVER_GIVE_BACK`] for an `L`
/// link).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub frame: Frame,
    pub hover: u32,
}

/// The nearest point on a segment `origin + dir·t`, `t` in [0, max], to
/// `point`: the parameter and the squared distance, or none for a
/// zero-length segment. The parameter is clamped by ordered compares as the
/// original clamps it, so a NaN parameter becomes zero.
pub fn segment_closest(origin: Vec3, dir: Vec3, max: f32, point: Vec3) -> Option<(f32, f32)> {
    let length2 = dir.length_squared();
    if length2 == 0.0 {
        return None;
    }
    let t = (point - origin).dot(dir) / length2;
    let t = if t >= 0.0 {
        if t > max { max } else { t }
    } else {
        0.0
    };
    let d = origin + dir * t - point;
    Some((t, d.length_squared()))
}

/// The first placement marker the cursor's ray meets, in node order: the
/// ray must descend (its height part at most −[`TURN_EPSILON`]; the original
/// refuses a ray that climbs), it must pass within the marker's radius
/// somewhere on its 10 000 cm length, the marker's name must be `item` or
/// `model`, and the item must fit the marker's frame (`fits`). The original
/// then puts the item's frame on the marker's and reports the hover word
/// from the marker's link byte.
pub fn snap_to_marker(
    origin: Vec3,
    dir: Vec3,
    item: u64,
    model: u64,
    markers: &[PlacementMarker],
    mut fits: impl FnMut(Frame) -> bool,
) -> Option<Placement> {
    if dir.y > -TURN_EPSILON {
        return None;
    }
    let max = DRAG_REACH_CM / 10.0;
    for marker in markers {
        let Some((_, distance2)) = segment_closest(origin, dir, max, marker.frame.position) else {
            continue;
        };
        if distance2 > marker.radius * marker.radius {
            continue;
        }
        if marker.name != item && marker.name != model {
            continue;
        }
        if !fits(marker.frame) {
            continue;
        }
        let hover = if marker.link == GIVE_BACK_LINK {
            HOVER_GIVE_BACK
        } else {
            HOVER_WORLD
        };
        return Some(Placement {
            frame: marker.frame,
            hover,
        });
    }
    None
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

/// The record a GUI drag spawns: the dragged entry's item id (as the
/// object database resolves it) and the condition the drag carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpawnRecord {
    pub item: u32,
    pub condition: u32,
}

/// The spawned record of a GUI drag whose current event is `event`: only
/// the two drag events ([`ITEM_PAYLOAD`], [`OBJECT_PAYLOAD`]) spawn, both
/// the same way, from the GUI's dragged entry and the drag's condition.
/// Any other event spawns nothing and answers false.
pub fn spawn_record(event: u32, item: u32, condition: u32) -> Option<SpawnRecord> {
    (event == ITEM_PAYLOAD || event == OBJECT_PAYLOAD).then_some(SpawnRecord { item, condition })
}

/// An object record's spawned node: its id, the condition its placement
/// asks for, and whether the new node still has to be attached to its
/// parent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpawnedObject {
    pub item: u32,
    pub condition: u32,
    pub attach: bool,
}

/// The node a record spawns: a record the database cannot make gives none
/// [guess: Exarust stands a zero id in for that record].
/// A node whose parent is already the record's parent needs no attach, as
/// the original's attach skips it.
pub fn spawn_object(item: u32, condition: u32, already_attached: bool) -> Option<SpawnedObject> {
    (item != 0).then_some(SpawnedObject {
        item,
        condition,
        attach: !already_attached,
    })
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
    /// The object's id and model id a level placement marker is named for
    /// (zero when the item carries neither).
    pub item: u64,
    pub model: u64,
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
    /// The level's placement markers. The original walks the scene's nodes
    /// for them; Exarust's level loader does not register the marker nodes
    /// yet, so the list is empty and a lift always makes a free placement.
    pub markers: Vec<PlacementMarker>,
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

    /// The placement marker the held item snaps onto, if the cursor's ray
    /// meets one named for the item or its model and the item fits there:
    /// the marker's frame and hover word. None without a held item or
    /// markers.
    pub fn snap(
        &self,
        origin: Vec3,
        dir: Vec3,
        fits: impl FnMut(Frame) -> bool,
    ) -> Option<Placement> {
        let held = self.held.as_ref()?;
        snap_to_marker(origin, dir, held.item, held.model, &self.markers, fits)
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
        assert_eq!(placement_distance(500.0, Some(300.0)), 300.0);
        assert_eq!(placement_distance(500.0, None), 500.0);
        assert_eq!(placement_distance(1200.0, Some(1300.0)), 1200.0);
        assert_eq!(placement_distance(500.0, Some(f32::NAN)), 500.0);
        assert_eq!(placement_point(Vec3::ZERO, Vec3::NEG_Y, f32::NAN), None);
        let at = placement_point(Vec3::ZERO, Vec3::NEG_Y, 300.0).unwrap();
        assert_eq!(at, Vec3::new(0.0, -3.0, 0.0));
    }

    /// The hover word: a placement marker's `L` link gives the give-back
    /// word, another marker the world word, a free placement bit 1.
    #[test]
    fn the_hover_words() {
        assert_eq!(hover_word(Some(GIVE_BACK_LINK), true), HOVER_GIVE_BACK);
        assert_eq!(hover_word(Some(b'K'), false), HOVER_WORLD);
        assert_eq!(hover_word(None, true), HOVER_GUI);
        assert_eq!(hover_word(None, false), 0);
    }

    /// Both drag events spawn the GUI's dragged entry with the drag's
    /// condition; another event spawns none.
    #[test]
    fn the_gui_drag_spawns_its_record() {
        assert_eq!(
            spawn_record(ITEM_PAYLOAD, 42, 0x10),
            Some(SpawnRecord {
                item: 42,
                condition: 0x10
            })
        );
        assert_eq!(
            spawn_record(OBJECT_PAYLOAD, 42, 3),
            Some(SpawnRecord {
                item: 42,
                condition: 3
            })
        );
        assert_eq!(spawn_record(0x8004, 42, 0), None);
        assert_eq!(spawn_object(0, 0, false), None);
        assert_eq!(
            spawn_object(42, 7, false),
            Some(SpawnedObject {
                item: 42,
                condition: 7,
                attach: true
            })
        );
        assert_eq!(
            spawn_object(42, 7, true),
            Some(SpawnedObject {
                item: 42,
                condition: 7,
                attach: false
            })
        );
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
            item: 0,
            model: 0,
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

    /// Every part's spheres are swept, in part and sphere order, and the
    /// nearest contact under the reach wins: a farther part hit does not
    /// replace a nearer one, a NaN never does and a 1000 cm contact is not
    /// nearer than the reach.
    #[test]
    fn every_part_sphere_is_swept() {
        let spheres = [
            SphereOf {
                part: 0,
                sphere: SweepSphere {
                    centre: Vec3::ZERO,
                    radius: 0.1,
                },
            },
            SphereOf {
                part: 1,
                sphere: SweepSphere {
                    centre: Vec3::Y,
                    radius: 0.2,
                },
            },
            SphereOf {
                part: 2,
                sphere: SweepSphere {
                    centre: Vec3::X,
                    radius: 0.3,
                },
            },
        ];
        let mut seen = Vec::new();
        let hit = per_sphere_contact(spheres, Vec3::NEG_Y, |part, centre, dir, reach, radius| {
            seen.push((part, centre, dir, reach, radius));
            match part {
                0 => Some(8.0),
                1 => Some(2.5),
                _ => Some(f32::NAN),
            }
        })
        .unwrap();
        assert_eq!(hit.part, 1);
        assert_eq!(hit.contact, 250.0);
        assert_eq!(seen.len(), 3);
        assert_eq!(seen[1].3, 10.0);
        // Only at the reach is not nearer.
        assert_eq!(
            per_sphere_contact(spheres, Vec3::NEG_Y, |_, _, _, _, _| Some(10.0)),
            None
        );
        // A NaN contact never replaces the best; a later one does.
        let hit = per_sphere_contact(spheres, Vec3::NEG_Y, |part, _, _, _, _| match part {
            0 => Some(f32::NAN),
            _ => Some(4.0),
        })
        .unwrap();
        assert_eq!(hit.part, 1);
    }

    /// A marker takes the item when the cursor's ray descends within its
    /// radius and its name is the item's or the model's; the item must fit
    /// the marker's frame. An `L` link gives the give-back word, another
    /// marker the world word.
    #[test]
    fn a_placement_marker_snaps_the_item() {
        let marker = |name: u64, at: Vec3, radius: f32, link: u8| PlacementMarker {
            name,
            frame: Frame::new(at, Mat3::IDENTITY),
            radius,
            link,
        };
        let markers = [
            marker(7, Vec3::new(0.5, 0.0, 10.0), 0.6, b'K'),
            marker(9, Vec3::new(0.0, 0.0, 10.0), 0.1, b'L'),
        ];
        // The ray down +Z does not descend enough to place anything.
        assert!(snap_to_marker(Vec3::ZERO, Vec3::Z, 7, 0, &markers, |_| true).is_none());
        // A ray down from above, 50 cm from the first marker and inside its
        // 60 cm radius, named for the item.
        let p = snap_to_marker(
            Vec3::new(0.0, 100.0, 10.0),
            Vec3::NEG_Y,
            7,
            0,
            &markers,
            |_| true,
        )
        .unwrap();
        assert_eq!(p.hover, HOVER_WORLD);
        assert_eq!(p.frame.position, Vec3::new(0.5, 0.0, 10.0));
        // The model id matches the second marker too, with the L link.
        let p = snap_to_marker(
            Vec3::new(0.0, 100.0, 10.0),
            Vec3::NEG_Y,
            0,
            9,
            &markers,
            |_| true,
        )
        .unwrap();
        assert_eq!(p.hover, HOVER_GIVE_BACK);
        assert_eq!(p.frame.position, Vec3::new(0.0, 0.0, 10.0));
        // A refused fit falls through to the next marker.
        let mut tried = 0;
        let p = snap_to_marker(
            Vec3::new(0.0, 100.0, 10.0),
            Vec3::NEG_Y,
            7,
            9,
            &markers,
            |_| {
                tried += 1;
                tried > 1
            },
        )
        .unwrap();
        assert_eq!(p.frame.position, Vec3::new(0.0, 0.0, 10.0));
        assert_eq!(tried, 2);
        // No name match gives none.
        assert!(
            snap_to_marker(
                Vec3::new(0.0, 100.0, 10.0),
                Vec3::NEG_Y,
                3,
                4,
                &markers,
                |_| true
            )
            .is_none()
        );
        // A ray just outside the radius misses; exactly on it is kept.
        let outside = Vec3::new(1.1 + f32::EPSILON, 100.0, 10.0);
        assert!(snap_to_marker(outside, Vec3::NEG_Y, 7, 0, &markers, |_| true).is_none());
        let edge = Vec3::new(1.1, 100.0, 10.0);
        assert!(snap_to_marker(edge, Vec3::NEG_Y, 7, 0, &markers, |_| true).is_some());
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
