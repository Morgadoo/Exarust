//! Teleporter and reconstruction devices: a pedestal whose `[` slot takes a
//! power crystal and opens the destination screen, and the device that
//! scans the player so that a death in the story brings them back.
//!
//! Units are the original's: centimetres and milliseconds of game time. A
//! device use is refused while the player possesses another body, when the
//! player's point is more than 100 cm from a point 16 cm above the device,
//! when the device has no `[` child, when no scene object lies within 1 cm
//! of that child, or when that object is not the device's crystal. The
//! distance and the proximity follow the original's comparisons: a NaN
//! distance passes the reach test and is never near, and a squared distance
//! of exactly 10 000 cm² is inside while exactly 1 cm² is not near.
//!
//! The teleporter takes a Power Crystal A (431) or its large-crystal
//! adapter (1423), compared by the object's whole record id, that is less
//! than 0x55 (85) spent; its operative record gives the destination level
//! ids (the locale list's ids) and whether the screen shows one row or
//! four. The reconstruction device takes a Power Crystal L (500), whatever
//! its charge, and starts the scan and the device's glow pulse.
//!
//! A scan is a scan effect on the player that waits 1600 ms (the waiting
//! state, word 8) and closes with the 3.0 flash. A use while one is already
//! on the player starts nothing. The device's glow rises over 1500 ms,
//! holds 3800 ms and falls over 1500 ms. The use remembers the player
//! record's identity word and raises the reconstruction latch. When the
//! player dies in the story with the same identity, the session ends in a
//! reconstruction instead of a death: the first time, the body is stripped
//! (hands and worn items cleared, the underwear rule, health and stamina at
//! 0x7FFF of 0xFFFF) and an unstripped copy of its record is left behind as
//! the corpse; later deaths reconstruct without a strip. Taking the crystal
//! off the device's slot plays the power-down sound and clears the latch.
//!
//! Evidence: the ten routines of the teleporter unit, checked against the
//! assembly for the comparisons, the order of the squared sum and the row
//! fill; the session end that consumes the latch, the glow pulse task and
//! the grab release that powers a device down were read at their call
//! sites. The scan, bubble and portal visuals belong to the effects
//! modules.

use bevy::math::Vec3;
use bevy::prelude::Resource;
use exa_formats::objdb::{DESTINATIONS, Operative};

use crate::rooms::{self, Gate};

/// First byte of the name of a device's item-slot child.
pub const SLOT_PREFIX: char = '[';
/// A scene object closer than this to the slot is on it, squared (cm²).
pub const NEAR_SQUARED_CM: f32 = 1.0;
/// The player must be within this of the device point, squared (cm²).
pub const REACH_SQUARED_CM: f32 = 10_000.0;
/// The device point is this far above the device's origin (cm).
pub const DEVICE_POINT_CM: f32 = 16.0;
/// Power Crystal A, the teleporter's small crystal.
pub const POWER_CRYSTAL: u32 = 0x1AF;
/// Power Crystal L, the reconstruction device's crystal.
pub const LARGE_POWER_CRYSTAL: u32 = 0x1F4;
/// PwrCrsLAdptA02, the large crystal's adapter, which the teleporter also
/// takes.
pub const CRYSTAL_ADAPTER: u32 = 0x58F;
/// A crystal spent at least this much is refused by the teleporter.
pub const CHARGE_LIMIT: u8 = 0x55;
/// The symbol atlas the destination screen draws. The use loads this file
/// once, by name, whatever the record holds.
pub const SCREEN_ATLAS: &str = "prtlsmbla.rfi";
/// The scan effect starts this long after the use (ms).
pub const SCAN_DELAY_MS: u32 = 1600;
/// The scan's waiting state word at creation.
pub const SCAN_STATE: i32 = 8;
/// The flash scale a scanner device gives the scan's looping close.
pub const SCAN_SCALE: f32 = 3.0;
/// The scanner's sound and the power sound, with their volume and pitch.
pub const SCAN_SOUND: &str = "devicescna01";
pub const POWER_SOUND: &str = "devicepwra01";
pub const SCAN_VOLUME: f32 = 0.3;
pub const POWER_VOLUME: f32 = 0.1;
pub const SOUND_PITCH: f32 = 1.0;
/// The device glow's rise (and fall) and its hold, milliseconds.
pub const PULSE_RAMP_MS: u32 = 1500;
pub const PULSE_HOLD_MS: u32 = 3800;
/// The glow is over once more than the two ramps and the hold have passed
/// (ms).
pub const PULSE_END_MS: u32 = 2 * PULSE_RAMP_MS + PULSE_HOLD_MS;
/// An object let go closer than this to a device's slot point takes the
/// device's crystal off it, squared (cm²).
pub const SLOT_RELEASE_SQUARED_CM: f32 = 5.0;
/// The red health under which the player is dead when the session ends,
/// compared in double precision.
pub const DEATH_RED: f64 = 0.01;
/// The health and stamina words a strip leaves (of 0xFFFF).
pub const STRIPPED_WORD: u16 = 0x7FFF;
/// The share of full health and stamina a strip leaves.
pub const STRIPPED_SHARE: f32 = 0x7FFF as f32 / 0xFFFF as f32;
/// The character's shared-record bit: a shared record is stripped in place
/// and no corpse record is left.
pub const SHARED_RECORD: u32 = 0x0008_0000;
/// The bit the left-behind record gets in its status word.
pub const LEFT_BEHIND_MARK: u32 = 0x40;
/// The roles the left-behind record is given, after its own are dropped.
pub const LEFT_BEHIND_ROLES: [i32; 2] = [1, 0xC2];

/// A device child: its name and position (cm).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Child<'a> {
    pub name: &'a str,
    pub at: Vec3,
}

impl Child<'_> {
    /// Whether this child is the device's item slot.
    pub fn is_slot(&self) -> bool {
        self.name.starts_with(SLOT_PREFIX)
    }
}

/// A scene object the device can find: its record id, how spent it is and
/// its position (cm).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Object {
    pub id: u32,
    /// Bits 16..23 of its placement word: 0 fresh, 255 spent.
    pub condition: u8,
    pub at: Vec3,
}

impl Object {
    /// Whether this is an unspent-enough teleporter crystal: a Power
    /// Crystal A or the large-crystal adapter, below [`CHARGE_LIMIT`]. The
    /// id is compared whole: a record id carrying a tier bit is not a
    /// crystal.
    pub fn is_portal_crystal(&self) -> bool {
        matches!(self.id, POWER_CRYSTAL | CRYSTAL_ADAPTER) && self.condition < CHARGE_LIMIT
    }

    /// Whether this is the reconstruction device's Power Crystal L.
    pub fn is_res_crystal(&self) -> bool {
        self.id == LARGE_POWER_CRYSTAL
    }
}

/// The squared distance between two points (cm), summed in the original's
/// order: the x and y squares, then the z square.
pub fn squared_distance_cm(a: Vec3, b: Vec3) -> f32 {
    let d = a - b;
    (d.x * d.x + d.y * d.y) + d.z * d.z
}

/// Whether `point` is within 1 cm of `origin`, as the original's strict
/// `distance < 1.0` test (a NaN distance is not near).
pub fn near_cm(origin: Vec3, point: Vec3) -> bool {
    squared_distance_cm(origin, point) < NEAR_SQUARED_CM
}

/// Whether the player at `user` (cm) can use a device at `device` (cm): the
/// squared distance from the point 16 cm above the device must not be more
/// than 10 000 cm². The original's unordered compare lets a NaN pass.
pub fn in_reach_cm(device: Vec3, user: Vec3) -> bool {
    let point = device + Vec3::Y * DEVICE_POINT_CM;
    let squared = squared_distance_cm(point, user);
    // Only an ordered "greater" refuses, so NaN passes.
    squared.is_nan() || squared <= REACH_SQUARED_CM
}

/// The first child that is the device's `[` slot.
pub fn find_slot<'a>(children: &'a [Child<'a>]) -> Option<&'a Child<'a>> {
    children.iter().find(|child| child.is_slot())
}

/// The first scene object within 1 cm of `slot`, in scene order.
pub fn find_near(slot: Vec3, objects: &[Object]) -> Option<&Object> {
    objects.iter().find(|object| near_cm(slot, object.at))
}

/// Whether an object let go at `released` (cm) leaves the slot point at
/// `slot` (cm): closer than [`SLOT_RELEASE_SQUARED_CM`] squared.
pub fn released_from_slot(slot: Vec3, released: Vec3) -> bool {
    squared_distance_cm(released, slot) < SLOT_RELEASE_SQUARED_CM
}

/// What the teleporter's record tells the destination screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenData {
    /// The operative flags; bit 2 makes the screen show one row instead of
    /// four.
    pub flags: u16,
    /// The symbol atlas the screen draws ([`SCREEN_ATLAS`]).
    pub atlas: &'static str,
    /// The destination level ids from the record, zero entries included.
    pub destinations: [u32; DESTINATIONS],
}

/// The operative flag that shows a single row.
pub const ROW_SINGLE: u16 = 4;
/// The rows a screen shows by default.
pub const ROW_COUNT: usize = 4;

impl ScreenData {
    /// The screen of an operative record.
    pub fn of(record: &Operative) -> Self {
        Self {
            flags: record.flags,
            atlas: SCREEN_ATLAS,
            destinations: record.destinations,
        }
    }

    /// The rows the screen lays out: four, or one when the record's flag
    /// bit 2 is set.
    pub fn rows(&self) -> usize {
        if self.flags & ROW_SINGLE != 0 {
            1
        } else {
            ROW_COUNT
        }
    }

    /// The non-zero destination level ids, in record order.
    pub fn reachable(&self) -> impl Iterator<Item = u32> + '_ {
        self.destinations.iter().copied().filter(|&id| id != 0)
    }

    /// The record entry each screen row shows. The original walks the
    /// first `rows()` entries in order: an empty (zero) entry, or one met
    /// after rows 1.. are all taken, goes to row 0 (the last such entry
    /// wins); any other entry takes the next of rows 1... A row nothing
    /// reaches keeps entry 0. The shipped destinations 7, 10, 12, 13 show
    /// as 13, 7, 10, 12.
    pub fn row_entries(&self) -> Vec<usize> {
        let count = self.rows();
        let mut rows = vec![0usize; count];
        let mut next = 1usize;
        for (i, &destination) in self.destinations.iter().enumerate().take(count) {
            if destination == 0 || next >= count {
                rows[0] = i;
            } else {
                rows[next] = i;
                next += 1;
            }
        }
        rows
    }

    /// The record entry a screen row shows.
    pub fn row_index(&self, row: usize) -> Option<usize> {
        self.row_entries().get(row).copied()
    }

    /// The destination level id a screen row shows.
    pub fn row_destination(&self, row: usize) -> Option<u32> {
        let index = self.row_index(row)?;
        self.destinations.get(index).copied()
    }
}

/// What using a teleporter did.
#[derive(Debug, Clone, PartialEq)]
pub enum PortalUse<'a> {
    /// The use was refused.
    Refused,
    /// A destination screen is already open; the use still succeeds and
    /// opens nothing.
    AlreadyOpen,
    /// Open the screen for this crystal.
    Open {
        item: &'a Object,
        /// The crystal is the large-crystal adapter rather than a small
        /// crystal.
        adapter: bool,
        screen: ScreenData,
    },
}

/// Use the teleporter `children`/`objects` stand for. `possessing` is
/// whether the player controls another body; `user` is the player's point,
/// `None` when there is no player (the reach test is then skipped).
/// `record` is the operative section of the device after its record was
/// resolved; `None` refuses. `screen_open` is whether a destination screen
/// is already shown. The checks run in the original's order: the
/// possession, the reach, the slot, the object near it, its id and charge,
/// the record, and only then the open screen.
pub fn use_portal<'a>(
    possessing: bool,
    device: Vec3,
    user: Option<Vec3>,
    children: &[Child<'_>],
    objects: &'a [Object],
    record: Option<&Operative>,
    screen_open: bool,
) -> PortalUse<'a> {
    if possessing {
        return PortalUse::Refused;
    }
    if let Some(user) = user
        && !in_reach_cm(device, user)
    {
        return PortalUse::Refused;
    }
    let Some(slot) = find_slot(children) else {
        return PortalUse::Refused;
    };
    let Some(item) = find_near(slot.at, objects) else {
        return PortalUse::Refused;
    };
    if item.id == 0 || !item.is_portal_crystal() {
        return PortalUse::Refused;
    }
    let Some(record) = record else {
        return PortalUse::Refused;
    };
    if screen_open {
        return PortalUse::AlreadyOpen;
    }
    PortalUse::Open {
        item,
        adapter: item.id == CRYSTAL_ADAPTER,
        screen: ScreenData::of(record),
    }
}

/// What using a reconstruction device did.
#[derive(Debug, Clone, PartialEq)]
pub enum ScanUse<'a> {
    /// The use was refused.
    Refused,
    /// A scan effect is already on the player; the use still succeeds and
    /// starts nothing.
    AlreadyScanning,
    /// Scan the player with this crystal.
    Started { item: &'a Object },
}

/// Use the reconstruction device `children`/`objects` stand for. Its checks
/// are the teleporter's without the charge test, and with the Power Crystal
/// L as the only accepted object. `scanning` is whether a scan effect is
/// already a child of the player.
pub fn use_res_device<'a>(
    possessing: bool,
    device: Vec3,
    user: Option<Vec3>,
    children: &[Child<'_>],
    objects: &'a [Object],
    scanning: bool,
) -> ScanUse<'a> {
    if possessing {
        return ScanUse::Refused;
    }
    if let Some(user) = user
        && !in_reach_cm(device, user)
    {
        return ScanUse::Refused;
    }
    let Some(slot) = find_slot(children) else {
        return ScanUse::Refused;
    };
    let Some(item) = find_near(slot.at, objects) else {
        return ScanUse::Refused;
    };
    if item.id == 0 || !item.is_res_crystal() {
        return ScanUse::Refused;
    }
    if scanning {
        return ScanUse::AlreadyScanning;
    }
    ScanUse::Started { item }
}

/// What a scan use schedules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanTiming {
    /// The scan effect's start stamp (game ms).
    pub effect_at_ms: u32,
    /// The device glow's ramp and hold, and the millisecond after which it
    /// is over.
    pub pulse_ramp_ms: u32,
    pub pulse_hold_ms: u32,
    pub pulse_end_ms: u32,
}

/// The scan a use at `now_ms` schedules.
pub fn scan_timing(now_ms: u32) -> ScanTiming {
    ScanTiming {
        effect_at_ms: now_ms.wrapping_add(SCAN_DELAY_MS),
        pulse_ramp_ms: PULSE_RAMP_MS,
        pulse_hold_ms: PULSE_HOLD_MS,
        pulse_end_ms: now_ms.wrapping_add(PULSE_END_MS),
    }
}

/// One step of a device's glow pulse.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PulseStep {
    /// The factor the device's lights and emissive parts are scaled by.
    pub strength: f32,
    /// The pulse is over: this step wrote its last value (0).
    pub done: bool,
}

/// The glow pulse `elapsed_ms` after its start (the 32-bit clock
/// difference): rising over `ramp_ms`, holding at one for `hold_ms`, then
/// falling over `ramp_ms` again, its fractions taken in double precision.
/// The sums are compared in 64 bits, but twice the ramp is doubled in 32.
pub fn pulse_step(elapsed_ms: u32, ramp_ms: u32, hold_ms: u32) -> PulseStep {
    let elapsed = u64::from(elapsed_ms);
    let ramp = u64::from(ramp_ms);
    let hold = u64::from(hold_ms);
    if u64::from(ramp_ms.wrapping_shl(1)) + hold < elapsed {
        return PulseStep {
            strength: 0.0,
            done: true,
        };
    }
    let strength = if elapsed_ms < ramp_ms {
        (elapsed as f64 / ramp as f64) as f32
    } else if elapsed > ramp + hold {
        let fallen = (elapsed - hold - ramp) as i64 as f64;
        (1.0 - fallen / ramp as f64) as f32
    } else {
        1.0
    };
    PulseStep {
        strength,
        done: false,
    }
}

/// The reconstruction latch the device use raises and the session end
/// consumes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LatchPhase {
    /// No reconstruction is pending.
    #[default]
    Idle,
    /// A scan started; the next death of the same character strips it.
    Charging,
    /// The character was reconstructed; later deaths reconstruct it
    /// without a strip.
    Reconstructed,
}

/// The state behind a use of a reconstruction device and the death at the
/// other end.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Default)]
pub struct Latch {
    pub phase: LatchPhase,
    /// The player record's identity word the scan was started for.
    pub remembered: u32,
    /// A word the scan clears (0 at the start; its reader is not
    /// established).
    pub value: f32,
}

impl Latch {
    /// A scan of the character whose identity word is `remembered`.
    pub fn begin(&mut self, remembered: u32) {
        self.phase = LatchPhase::Charging;
        self.remembered = remembered;
        self.value = 0.0;
    }

    /// The crystal left the device: clear the latch, whatever its phase.
    pub fn power_down(&mut self) {
        self.phase = LatchPhase::Idle;
    }

    /// The player died with the record identity `dying`. With a pending
    /// scan for that identity the latch settles on reconstructed and the
    /// answer is whether this death strips (only the first after a scan
    /// does); `None` is an ordinary death.
    pub fn on_death(&mut self, dying: u32) -> Option<bool> {
        if self.phase == LatchPhase::Idle || dying != self.remembered {
            return None;
        }
        let strip = self.phase == LatchPhase::Charging;
        self.phase = LatchPhase::Reconstructed;
        Some(strip)
    }
}

/// How a play session ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionEnd {
    /// The player left alive.
    Left,
    /// The player died. In the story this is the end of the game.
    Died,
    /// The player died in the story after a scan of the same identity and
    /// is reconstructed; `stripped` is whether this death stripped them.
    Reconstructed { stripped: bool },
}

/// The session end for a player with red health `red` and record identity
/// `identity`: dead below [`DEATH_RED`] (widened to double; a NaN is
/// alive), and only in the story does the latch turn a death into a
/// reconstruction.
pub fn session_end(red: f32, story: bool, identity: u32, latch: &mut Latch) -> SessionEnd {
    let red = f64::from(red);
    if red.is_nan() || red >= DEATH_RED {
        return SessionEnd::Left;
    }
    if !story {
        return SessionEnd::Died;
    }
    match latch.on_death(identity) {
        Some(stripped) => SessionEnd::Reconstructed { stripped },
        None => SessionEnd::Died,
    }
}

/// The character-record fields a strip changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// The character's shared-record bit ([`SHARED_RECORD`]).
    pub shared: bool,
    /// The record's use count.
    pub uses: u32,
    /// The record's status word ([`LEFT_BEHIND_MARK`]).
    pub status: u32,
    /// The record's identity word.
    pub identity: u32,
    /// The four hand item ids.
    pub held: [u32; 4],
    /// The worn item ids.
    pub worn: Vec<u32>,
    /// Health and stamina words; 0xFFFF is full.
    pub health: u16,
    pub stamina: u16,
    /// The queued actions: one zeroed entry is left.
    pub actions: Vec<u32>,
    /// The record's roles.
    pub roles: Vec<i32>,
}

impl Record {
    /// The strip of a reconstructed body. An unshared record is first
    /// copied, and the copy, returned, is what the dead body keeps: one use,
    /// the left-behind mark, identity 0, its roles replaced by
    /// [`LEFT_BEHIND_ROLES`] and its health and stamina at 0, with every
    /// item still on it. The record itself, the one the player goes on
    /// with, then loses both hands and the worn list, keeps one empty
    /// action and is left at [`STRIPPED_WORD`] health and stamina; the
    /// underwear rule runs after this, on the cleared worn list. A shared
    /// record is stripped in place and counts one more use.
    pub fn strip(&mut self) -> Option<Record> {
        let left_behind = (!self.shared).then(|| {
            let mut copy = self.clone();
            copy.uses = 1;
            copy.status |= LEFT_BEHIND_MARK;
            copy.identity = 0;
            copy.roles = LEFT_BEHIND_ROLES.to_vec();
            copy.health = 0;
            copy.stamina = 0;
            copy
        });
        self.held = [0; 4];
        self.worn.clear();
        self.actions = vec![0];
        self.health = STRIPPED_WORD;
        self.stamina = STRIPPED_WORD;
        if self.shared {
            self.uses += 1;
        }
        left_behind
    }
}

/// The share of full health and stamina the strip leaves
/// (0x7FFF / 0xFFFF, about 0.4999924: just under half).
pub fn stripped_share() -> f32 {
    STRIPPED_SHARE
}

/// The level a chosen destination id loads, through the locale list's
/// names. The journey the screen starts carries a level id; the entrance is
/// not part of it, so it is 0 here.
pub fn destination_gate(level: u32) -> Gate {
    Gate { level, entrance: 0 }
}

/// The level name a chosen destination id loads, as [`rooms::enter_level`]
/// resolves a passage. An id the list does not name keeps `current`.
pub fn destination_level<'a>(
    current: &'a str,
    chosen: u32,
    names: &'a [(u32, &'a str)],
) -> &'a str {
    rooms::enter_level(current, destination_gate(chosen), names)
}

/// The locale ids and names the shipped teleporter reaches, for tests and
/// callers without the level list: Exanima05, Exanima07, Exanima08 and
/// Exanima67.
pub const SHIPPED_DESTINATIONS: [(u32, &str); 4] = [
    (7, "exanima05"),
    (10, "exanima07"),
    (12, "exanima08"),
    (13, "exanima67"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use exa_formats::objdb::operative;

    fn child(name: &'static str, at: Vec3) -> Child<'static> {
        Child { name, at }
    }

    fn record() -> Operative {
        let mut destinations = [0u32; DESTINATIONS];
        destinations[..4].copy_from_slice(&[7, 10, 12, 13]);
        Operative {
            class: operative::DEVICE,
            kind: operative::TELEPORTER,
            flags: 0x40,
            value: 0,
            time: 0,
            target: 0,
            second: 0,
            atlas: *b"PrtlSmblA.rfi\0\0\0",
            destinations,
        }
    }

    fn crystal(id: u32, condition: u8, at: Vec3) -> Object {
        Object { id, condition, at }
    }

    #[test]
    fn the_squared_sum_is_in_the_original_order() {
        // (−3)² + 4² + 12² = 9 + 16 + 144 = 169.
        assert_eq!(
            squared_distance_cm(Vec3::new(1.0, 2.0, 3.0), Vec3::new(4.0, -2.0, -9.0)),
            169.0
        );
        // The y square is added to the x square first: 1 + 49 + 0 = 50.
        assert_eq!(
            squared_distance_cm(Vec3::ZERO, Vec3::new(1.0, 7.0, 0.0)),
            50.0
        );
    }

    #[test]
    fn an_object_within_one_centimetre_is_on_the_slot() {
        let slot = Vec3::new(10.0, 0.0, 10.0);
        // Exactly 1 cm away: not near.
        assert!(!near_cm(slot, Vec3::new(11.0, 0.0, 10.0)));
        // 0.6² + 0.8² = 1.0: not near.
        assert!(!near_cm(slot, Vec3::new(10.6, 0.8, 10.0)));
        // 0.6² + 0.79² = 0.9841: near.
        assert!(near_cm(slot, Vec3::new(10.6, 0.79, 10.0)));
        assert!(!near_cm(slot, Vec3::new(f32::NAN, 0.0, 0.0)));
        // The first object in scene order wins.
        let objects = [
            crystal(1, 0, Vec3::new(20.0, 0.0, 0.0)),
            crystal(2, 0, Vec3::new(10.5, 0.0, 10.0)),
            crystal(3, 0, Vec3::new(10.1, 0.0, 10.0)),
        ];
        assert_eq!(find_near(slot, &objects).unwrap().id, 2);
    }

    #[test]
    fn the_use_reach_is_one_hundred_centimetres_and_sixteen_up() {
        let device = Vec3::ZERO;
        // The point compared is (0, 16, 0).
        assert!(in_reach_cm(device, Vec3::new(0.0, 16.0, 0.0)));
        // 96² + 28² = 10 000 exactly: inside.
        assert!(in_reach_cm(device, Vec3::new(96.0, -12.0, 0.0)));
        // A hair farther: refused.
        assert!(!in_reach_cm(device, Vec3::new(96.01, -12.0, 0.0)));
        // The original's unordered branch lets NaN through.
        assert!(in_reach_cm(device, Vec3::new(f32::NAN, 0.0, 0.0)));
        // 99 cm ahead and at the device origin: 99² + 16² > 10 000.
        assert!(!in_reach_cm(device, Vec3::new(99.0, 0.0, 0.0)));
    }

    #[test]
    fn the_slot_is_the_first_child_named_with_a_bracket() {
        let children = [
            child("frame", Vec3::ZERO),
            child("[000001AFL]", Vec3::new(0.0, 16.0, 0.0)),
            child("[other]", Vec3::new(5.0, 0.0, 0.0)),
        ];
        let slot = find_slot(&children).unwrap();
        assert_eq!(slot.name, "[000001AFL]");
        assert_eq!(slot.at, Vec3::new(0.0, 16.0, 0.0));
        assert!(find_slot(&[child("frame", Vec3::ZERO)]).is_none());
    }

    #[test]
    fn only_the_charged_portal_crystals_fit_the_teleporter() {
        assert!(crystal(POWER_CRYSTAL, 0, Vec3::ZERO).is_portal_crystal());
        assert!(crystal(CRYSTAL_ADAPTER, 0x54, Vec3::ZERO).is_portal_crystal());
        // The id is compared whole: a tier bit is not the crystal.
        assert!(!crystal(0x8000_01AF, 0, Vec3::ZERO).is_portal_crystal());
        assert!(!crystal(0x4000_01F4, 0, Vec3::ZERO).is_res_crystal());
        // 0x55 spent is refused, 0x54 accepted.
        assert!(!crystal(POWER_CRYSTAL, 0x55, Vec3::ZERO).is_portal_crystal());
        assert!(!crystal(POWER_CRYSTAL, 0xFF, Vec3::ZERO).is_portal_crystal());
        // A large crystal is the other device's.
        assert!(!crystal(LARGE_POWER_CRYSTAL, 0, Vec3::ZERO).is_portal_crystal());
        assert!(crystal(LARGE_POWER_CRYSTAL, 0xFF, Vec3::ZERO).is_res_crystal());
        assert!(!crystal(POWER_CRYSTAL, 0, Vec3::ZERO).is_res_crystal());
    }

    /// The portal rule with the user at the device's origin unless given.
    fn at<'a>(
        blocked: bool,
        user: Option<Vec3>,
        children: &[Child<'_>],
        objects: &'a [Object],
        record: Option<&'a Operative>,
        open: bool,
    ) -> PortalUse<'a> {
        use_portal(blocked, Vec3::ZERO, user, children, objects, record, open)
    }

    #[test]
    fn the_teleporter_opens_for_a_crystal_on_its_slot() {
        let children = [child("[000001AFL]", Vec3::new(0.0, 16.0, 0.0))];
        let objects = [crystal(POWER_CRYSTAL, 0, Vec3::new(0.2, 16.0, 0.0))];
        let record = record();
        let origin = Some(Vec3::new(0.0, 0.0, 0.0));
        let PortalUse::Open {
            adapter,
            screen,
            item,
        } = at(false, origin, &children, &objects, Some(&record), false)
        else {
            panic!("opens");
        };
        assert_eq!(item.id, POWER_CRYSTAL);
        assert!(!adapter);
        // The screen draws the atlas the use loads by name.
        assert_eq!(screen.atlas, "prtlsmbla.rfi");
        assert_eq!(screen.rows(), 4);
        // A possession refuses even with a crystal in place.
        assert_eq!(
            at(
                true,
                Some(Vec3::ZERO),
                &children,
                &objects,
                Some(&record),
                false
            ),
            PortalUse::Refused
        );
        // Beyond the reach refuses.
        assert_eq!(
            at(
                false,
                Some(Vec3::new(99.0, 0.0, 0.0)),
                &children,
                &objects,
                Some(&record),
                false
            ),
            PortalUse::Refused
        );
        // No user: the original's null character skips the reach test.
        assert!(matches!(
            at(false, None, &children, &objects, Some(&record), false),
            PortalUse::Open { .. }
        ));
        // No record refuses before the open-screen check.
        assert_eq!(
            at(false, None, &children, &objects, None, true),
            PortalUse::Refused
        );
        // An open screen succeeds without opening another.
        assert_eq!(
            at(false, None, &children, &objects, Some(&record), true),
            PortalUse::AlreadyOpen
        );
    }

    #[test]
    fn the_teleporter_refuses_an_empty_or_wrong_slot() {
        let record = record();
        let children = [child("[000001AFL]", Vec3::ZERO)];
        let none = [child("frame", Vec3::ZERO)];
        let wrong = [crystal(LARGE_POWER_CRYSTAL, 0, Vec3::ZERO)];
        let spent = [crystal(POWER_CRYSTAL, 0x55, Vec3::ZERO)];
        let far = [crystal(POWER_CRYSTAL, 0, Vec3::new(1.0, 0.0, 0.0))];
        for (kids, objects) in [
            (&children[..], &wrong[..]),
            (&children[..], &spent[..]),
            (&children[..], &far[..]),
            (&none[..], &wrong[..]),
        ] {
            assert_eq!(
                use_portal(false, Vec3::ZERO, None, kids, objects, Some(&record), false),
                PortalUse::Refused
            );
        }
        // The adapter opens as well and is flagged.
        let adapter = [crystal(CRYSTAL_ADAPTER, 0x54, Vec3::ZERO)];
        let PortalUse::Open { adapter: flag, .. } = use_portal(
            false,
            Vec3::ZERO,
            None,
            &children,
            &adapter,
            Some(&record),
            false,
        ) else {
            panic!("opens");
        };
        assert!(flag);
    }

    #[test]
    fn the_shipped_screen_lays_its_rows_out_in_the_originals_order() {
        let screen = ScreenData::of(&record());
        assert_eq!(screen.reachable().collect::<Vec<_>>(), [7, 10, 12, 13]);
        assert_eq!(screen.rows(), 4);
        // Row 0 takes the last destination, the rest the earlier ones.
        assert_eq!(screen.row_destination(0), Some(13));
        assert_eq!(screen.row_destination(1), Some(7));
        assert_eq!(screen.row_destination(2), Some(10));
        assert_eq!(screen.row_destination(3), Some(12));
        assert_eq!(screen.row_destination(4), None);
        assert_eq!(screen.row_index(0), Some(3));
        assert_eq!(screen.row_index(1), Some(0));

        // Flag bit 2 shows a single row, the first destination.
        let mut single = record();
        single.flags |= ROW_SINGLE;
        let screen = ScreenData::of(&single);
        assert_eq!(screen.rows(), 1);
        assert_eq!(screen.row_destination(0), Some(7));
        assert_eq!(screen.row_destination(1), None);

        // A zero entry goes to row 0 and the walk goes on: 7, 0, 12, 13
        // lays out as entries 1, 0, 2, 3 (the third and fourth entries
        // still take rows 2 and 3).
        let mut zero = record();
        zero.destinations[1] = 0;
        let screen = ScreenData::of(&zero);
        assert_eq!(screen.row_entries(), [1, 0, 2, 3]);
        assert_eq!(screen.row_destination(0), Some(0));
        assert_eq!(screen.row_destination(1), Some(7));
        assert_eq!(screen.row_destination(2), Some(12));
        assert_eq!(screen.row_destination(3), Some(13));
        // Two entries: rows 1 and 2 take them, row 0 the empty entry 3 (the
        // last zero wins), row 3 is never reached and keeps entry 0.
        let mut two = record();
        two.destinations[2] = 0;
        two.destinations[3] = 0;
        let screen = ScreenData::of(&two);
        assert_eq!(screen.row_entries(), [3, 0, 1, 0]);
        assert_eq!(screen.row_destination(3), Some(7));
        // Only the first `rows()` entries are walked: a fifth entry is not
        // shown.
        let mut five = record();
        five.destinations[4] = 99;
        assert_eq!(ScreenData::of(&five).row_entries(), [3, 0, 1, 2]);
    }

    #[test]
    fn the_reconstruction_device_takes_only_the_large_crystal() {
        let children = [child("[000001F4L]", Vec3::ZERO)];
        let large = [crystal(LARGE_POWER_CRYSTAL, 0xFF, Vec3::ZERO)];
        let small = [crystal(POWER_CRYSTAL, 0, Vec3::ZERO)];
        // Its charge is not read: a spent crystal still works.
        assert!(matches!(
            use_res_device(false, Vec3::ZERO, None, &children, &large, false),
            ScanUse::Started { item } if item.id == LARGE_POWER_CRYSTAL
        ));
        // A small crystal is refused.
        assert_eq!(
            use_res_device(false, Vec3::ZERO, None, &children, &small, false),
            ScanUse::Refused
        );
        // A running scan makes the use succeed without starting another.
        assert_eq!(
            use_res_device(false, Vec3::ZERO, None, &children, &large, true),
            ScanUse::AlreadyScanning
        );
        // The same gates as the teleporter.
        assert_eq!(
            use_res_device(true, Vec3::ZERO, None, &children, &large, false),
            ScanUse::Refused
        );
        assert_eq!(
            use_res_device(
                false,
                Vec3::ZERO,
                Some(Vec3::new(99.0, 0.0, 0.0)),
                &children,
                &large,
                false
            ),
            ScanUse::Refused
        );
        assert_eq!(
            use_res_device(false, Vec3::ZERO, None, &[], &large, false),
            ScanUse::Refused
        );
    }

    #[test]
    fn a_scan_starts_after_1600_ms_and_the_glow_runs_6800_ms() {
        let timing = scan_timing(1_000);
        assert_eq!(timing.effect_at_ms, 2_600);
        assert_eq!(timing.pulse_ramp_ms, 1_500);
        assert_eq!(timing.pulse_hold_ms, 3_800);
        assert_eq!(timing.pulse_end_ms, 7_800);
        assert_eq!(SCAN_STATE, 8);
        assert_eq!(SCAN_SCALE, 3.0);
        // The clock wraps like the original's 32-bit millisecond stamp.
        assert_eq!(scan_timing(u32::MAX - 5).effect_at_ms, 1_594);
    }

    #[test]
    fn the_glow_rises_holds_and_falls() {
        let step = |e| pulse_step(e, PULSE_RAMP_MS, PULSE_HOLD_MS);
        assert_eq!(step(0).strength, 0.0);
        // 750 / 1500 in double precision.
        assert_eq!(step(750).strength, 0.5);
        assert_eq!(step(1_499).strength, (1_499.0f64 / 1_500.0) as f32);
        // From the end of the ramp to ramp + hold it holds at one.
        assert_eq!(step(1_500).strength, 1.0);
        assert_eq!(step(5_300).strength, 1.0);
        // Then it falls: 1 - (5 301 - 3 800 - 1 500) / 1 500.
        assert_eq!(step(5_301).strength, (1.0 - 1.0f64 / 1_500.0) as f32);
        assert_eq!(step(6_050).strength, 0.5);
        // At exactly two ramps and the hold it is 0 but not yet over.
        assert_eq!(
            step(6_800),
            PulseStep {
                strength: 0.0,
                done: false
            }
        );
        assert_eq!(
            step(6_801),
            PulseStep {
                strength: 0.0,
                done: true
            }
        );
        assert_eq!(PULSE_END_MS, 6_800);
        // A stamp ahead of the clock wraps to a huge span: the pulse ends.
        assert!(pulse_step(0u32.wrapping_sub(1), 1_500, 3_800).done);
        // Twice the ramp is doubled in 32 bits: a ramp of 2³¹ doubles to 0,
        // so 1 ms past the hold is already over.
        assert!(pulse_step(11, 0x8000_0000, 10).done);
    }

    #[test]
    fn a_crystal_let_go_within_five_square_centimetres_leaves_the_slot() {
        let slot = Vec3::new(0.0, 16.0, 0.0);
        // 2² + 1² = 5: not closer, so not taken off.
        assert!(!released_from_slot(slot, Vec3::new(2.0, 17.0, 0.0)));
        // 2² + 0.9² = 4.81.
        assert!(released_from_slot(slot, Vec3::new(2.0, 16.9, 0.0)));
        assert!(!released_from_slot(slot, Vec3::splat(f32::NAN)));
    }

    #[test]
    fn the_latch_turns_the_matching_death_into_a_reconstruction() {
        let mut latch = Latch::default();
        assert_eq!(latch.phase, LatchPhase::Idle);
        assert_eq!(latch.on_death(0), None);
        latch.begin(0x00C1_0ED0);
        assert_eq!(latch.phase, LatchPhase::Charging);
        assert_eq!(latch.remembered, 0x00C1_0ED0);
        assert_eq!(latch.value, 0.0);
        // Another identity dies as usual.
        assert_eq!(latch.on_death(0x00C1_0ED1), None);
        assert_eq!(latch.phase, LatchPhase::Charging);
        // The matching death strips once; later ones reconstruct only.
        assert_eq!(latch.on_death(0x00C1_0ED0), Some(true));
        assert_eq!(latch.phase, LatchPhase::Reconstructed);
        assert_eq!(latch.on_death(0x00C1_0ED0), Some(false));
        // Taking the crystal off clears it.
        latch.power_down();
        assert_eq!(latch.phase, LatchPhase::Idle);
        assert_eq!(latch.on_death(0x00C1_0ED0), None);
    }

    #[test]
    fn the_session_ends_by_red_health_mode_and_latch() {
        let mut latch = Latch::default();
        latch.begin(7);
        // The compare is in double: 0.01f32 widens to 0.00999999977..., just
        // under the double 0.01, so it is dead; the next float up is alive.
        assert!(f64::from(0.01f32) < DEATH_RED);
        let above = f32::from_bits(0.01f32.to_bits() + 1);
        assert_eq!(session_end(above, true, 7, &mut latch), SessionEnd::Left);
        assert_eq!(session_end(f32::NAN, true, 7, &mut latch), SessionEnd::Left);
        assert_eq!(latch.phase, LatchPhase::Charging);
        // Outside the story a death is a death, and the latch is kept.
        assert_eq!(session_end(0.0099, false, 7, &mut latch), SessionEnd::Died);
        assert_eq!(latch.phase, LatchPhase::Charging);
        // Another identity dies.
        assert_eq!(session_end(0.0, true, 8, &mut latch), SessionEnd::Died);
        assert_eq!(
            session_end(0.0, true, 7, &mut latch),
            SessionEnd::Reconstructed { stripped: true }
        );
        assert_eq!(
            session_end(0.01, true, 7, &mut latch),
            SessionEnd::Reconstructed { stripped: false }
        );
    }

    #[test]
    fn the_strip_leaves_a_dressed_corpse_record_and_a_bare_body() {
        let mut record = Record {
            shared: false,
            uses: 3,
            status: 0x01,
            identity: 0x00C1_0ED0,
            held: [10, 11, 12, 13],
            worn: vec![20, 21],
            health: 0xFFFF,
            stamina: 0xC000,
            actions: vec![7, 8, 9],
            roles: vec![9],
        };
        let corpse = record.strip().expect("an unshared record leaves a copy");
        // The record the player keeps: bare, one empty action, just under
        // half health and stamina, its roles, uses and identity kept.
        assert_eq!(record.held, [0; 4]);
        assert!(record.worn.is_empty());
        assert_eq!((record.health, record.stamina), (0x7FFF, 0x7FFF));
        assert_eq!(record.actions, [0]);
        assert_eq!(record.roles, [9]);
        assert_eq!((record.uses, record.identity), (3, 0x00C1_0ED0));
        // The copy the dead body keeps: still dressed and armed, no health,
        // one use, the mark, identity 0 and the two roles.
        assert_eq!(corpse.held, [10, 11, 12, 13]);
        assert_eq!(corpse.worn, [20, 21]);
        assert_eq!(corpse.actions, [7, 8, 9]);
        assert_eq!((corpse.health, corpse.stamina), (0, 0));
        assert_eq!((corpse.uses, corpse.status), (1, 0x41));
        assert_eq!(corpse.identity, 0);
        assert_eq!(corpse.roles, [1, 0xC2]);
        // A shared record is stripped in place, counts a use, leaves none.
        let mut shared = Record {
            shared: true,
            held: [1, 0, 0, 0],
            ..corpse.clone()
        };
        assert!(shared.strip().is_none());
        assert_eq!(shared.held, [0; 4]);
        assert_eq!(shared.roles, [1, 0xC2]);
        assert_eq!(shared.uses, 2);
        // 32767 of 65535 is just under half.
        assert!((stripped_share() - 0.499_992_4).abs() < 1e-6);
        assert_eq!(STRIPPED_WORD, 32767);
    }

    #[test]
    fn a_chosen_destination_names_the_level_it_loads() {
        // The shipped teleporter's four ids are the four levels that hold
        // one: Exanima05, 07, 08 and 67.
        assert_eq!(
            destination_level("exanima01", 7, &SHIPPED_DESTINATIONS),
            "exanima05"
        );
        assert_eq!(
            destination_level("exanima01", 10, &SHIPPED_DESTINATIONS),
            "exanima07"
        );
        assert_eq!(
            destination_level("exanima05", 12, &SHIPPED_DESTINATIONS),
            "exanima08"
        );
        assert_eq!(
            destination_level("exanima05", 13, &SHIPPED_DESTINATIONS),
            "exanima67"
        );
        // An id the list does not name keeps the current level.
        assert_eq!(
            destination_level("exanima05", 99, &SHIPPED_DESTINATIONS),
            "exanima05"
        );
        assert_eq!(
            destination_gate(13),
            Gate {
                level: 13,
                entrance: 0
            }
        );
    }
}
