//! Teleporter and reconstruction devices: a pedestal whose `[` slot takes a
//! power crystal and opens the destination screen, and the device that scans
//! a body before a level transition.
//!
//! Units are the original's: centimetres and milliseconds. A device use is
//! refused while the shared use gate is shut, when the user's point is more
//! than 100 cm from a point 16 cm above the device, when the device has no
//! `[` child, when no scene object lies within 1 cm of that child, or when
//! that object is not the device's crystal. The gate, the distance and the
//! proximity all follow the original's comparisons: a NaN distance passes
//! the reach test, and a squared distance of exactly 1 or 10 000 cm² is
//! inside.
//!
//! The teleporter takes a Power Crystal A (431) or its large-crystal
//! adapter (1423) that is less than 0x55 (85) spent; its operative record
//! names the symbol atlas and the destination level ids (the locale list's
//! ids). The reconstruction device takes a Power Crystal L (500), whatever
//! its charge, and starts the scan and the device's glow pulse.
//!
//! A scan begins 1600 ms after the use. Its effect is the waiting state
//! (word 8) with the 3.0 flashing close; the device's glow rises over
//! 1500 ms and falls over 3800 ms. The use remembers the character record's
//! identity word and raises the transition latch: the next level load whose
//! arriving character has the same identity strips the character (hands and
//! worn items cleared, the underwear rule, health and stamina at 0x7FFF of
//! 0xFFFF) once, and the latch settles on arrived.
//!
//! Evidence: the ten routines of the teleporter unit, checked against the
//! assembly for the comparisons and the order of the squared sum. The scan,
//! bubble and portal visuals belong to the effects modules; this module
//! only gives their requests.

use bevy::math::Vec3;
use bevy::prelude::Resource;
use exa_formats::objdb::{DESTINATIONS, Operative};

use crate::rooms::{self, Gate};

/// First byte of the name of a device's item-slot child.
pub const SLOT_PREFIX: char = '[';
/// A scene object closer than this to the slot is on it, squared (cm²).
pub const NEAR_SQUARED_CM: f32 = 1.0;
/// The user must be within this of the device point, squared (cm²).
pub const REACH_SQUARED_CM: f32 = 10_000.0;
/// The device point is this far above the device's origin (cm).
pub const DEVICE_POINT_CM: f32 = 16.0;
/// The object id's local and world tier bits are dropped before it is
/// compared with a device's crystal.
pub const OBJECT_ID_MASK: u32 = 0x3FFF_FFFF;
/// Power Crystal A, the teleporter's small crystal.
pub const POWER_CRYSTAL: u32 = 0x1AF;
/// Power Crystal L, the reconstruction device's crystal.
pub const LARGE_POWER_CRYSTAL: u32 = 0x1F4;
/// PwrCrsLAdptA02, the large crystal's adapter, which the teleporter also
/// takes.
pub const CRYSTAL_ADAPTER: u32 = 0x58F;
/// A crystal spent at least this much is refused by the teleporter.
pub const CHARGE_LIMIT: u8 = 0x55;
/// A spent crystal's condition.
pub const SPENT: u8 = 0xFF;
/// The scan effect starts this long after the use (ms).
pub const SCAN_DELAY_MS: u32 = 1600;
/// The scan's waiting state word at creation.
pub const SCAN_STATE: i32 = 8;
/// The colour/size scale a scanner device gives the scan's close.
pub const SCAN_SCALE: f32 = 3.0;
/// The scanner's sound and the power sound, with their volume and pitch.
pub const SCAN_SOUND: &str = "devicescna01";
pub const POWER_SOUND: &str = "devicepwra01";
pub const SCAN_VOLUME: f32 = 0.3;
pub const POWER_VOLUME: f32 = 0.1;
pub const SOUND_PITCH: f32 = 1.0;
/// The device glow's rise and fall, milliseconds.
pub const PULSE_RISE_MS: u32 = 1500;
pub const PULSE_FALL_MS: u32 = 3800;
/// The glow ends when twice the rise and the fall have passed (ms).
pub const PULSE_END_MS: u32 = 2 * PULSE_RISE_MS + PULSE_FALL_MS;
/// The health and stamina words a teleporter strip leaves (of 0xFFFF).
pub const STRIPPED_WORD: u16 = 0x7FFF;
/// The share of full health and stamina a strip leaves.
pub const STRIPPED_SHARE: f32 = 0x7FFF as f32 / 0xFFFF as f32;
/// The record word's shared bit: a shared record is stripped in place.
pub const SHARED_RECORD: u32 = 0x0008_0000;
/// Roles a teleporter gives the fresh record it strips.
pub const STRIP_ROLES: [i32; 2] = [1, 0xC2];

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
    /// The id without its local/world tier bits.
    pub fn base_id(&self) -> u32 {
        self.id & OBJECT_ID_MASK
    }

    /// Whether this is an unspent-enough teleporter crystal: a Power
    /// Crystal A or the large-crystal adapter, below [`CHARGE_LIMIT`].
    pub fn is_portal_crystal(&self) -> bool {
        matches!(self.base_id(), POWER_CRYSTAL | CRYSTAL_ADAPTER) && self.condition < CHARGE_LIMIT
    }

    /// Whether this is the reconstruction device's Power Crystal L.
    pub fn is_res_crystal(&self) -> bool {
        self.base_id() == LARGE_POWER_CRYSTAL
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

/// Whether a user at `user` (cm) can use a device at `device` (cm): the
/// squared distance from the point 16 cm above the device must not be more
/// than 10 000 cm². The original's unordered compare lets a NaN pass.
pub fn in_reach_cm(device: Vec3, user: Vec3) -> bool {
    let point = device + Vec3::Y * DEVICE_POINT_CM;
    let squared = squared_distance_cm(point, user);
    // The original's unordered compare: only a greater ordered distance
    // refuses, so NaN passes.
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

/// What the teleporter's record tells the destination screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenData {
    /// The operative flags; bit 2 makes the screen show one row instead of
    /// four.
    pub flags: u16,
    /// The symbol atlas the screen draws (`PrtlSmblA.rfi`).
    pub atlas: String,
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
            atlas: record.atlas_name().to_string(),
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

    /// The record entry a screen row shows. The original fills row 0 with
    /// the last entry it reaches and the rows after it with the earlier
    /// entries in order, so the shipped four destinations 7, 10, 12, 13
    /// appear on rows 0..3 as 13, 7, 10, 12. A zero entry ends the fill
    /// there and leaves the later rows on entry 0.
    pub fn row_index(&self, row: usize) -> Option<usize> {
        let count = self.rows();
        if row >= count {
            return None;
        }
        let mut index = vec![0usize; count];
        let mut later = index.iter_mut().skip(1);
        let mut first = 0usize;
        for i in 0..count {
            let destination = self.destinations.get(i).copied().unwrap_or(0);
            if destination == 0 {
                first = i;
                break;
            }
            match later.next() {
                Some(slot) => *slot = i,
                None => {
                    first = i;
                    break;
                }
            }
        }
        index[0] = first;
        index.get(row).copied()
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

/// Use the teleporter `children`/`objects` stand for. `record` is the
/// operative section of the device after its record was resolved; `None`
/// refuses. `row_screen_open` is whether a screen for this device is
/// already shown. The checks run in the original's order: the use gate,
/// the reach, the slot, the object near it, its id and charge, the record,
/// and only then the open screen.
pub fn use_portal<'a>(
    blocked: bool,
    device: Vec3,
    user: Option<Vec3>,
    children: &[Child<'_>],
    objects: &'a [Object],
    record: Option<&Operative>,
    row_screen_open: bool,
) -> PortalUse<'a> {
    if blocked {
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
    if item.base_id() == 0 || !item.is_portal_crystal() {
        return PortalUse::Refused;
    }
    let Some(record) = record else {
        return PortalUse::Refused;
    };
    if row_screen_open {
        return PortalUse::AlreadyOpen;
    }
    PortalUse::Open {
        item,
        adapter: item.base_id() == CRYSTAL_ADAPTER,
        screen: ScreenData::of(record),
    }
}

/// What using a reconstruction device did.
#[derive(Debug, Clone, PartialEq)]
pub enum ScanUse<'a> {
    /// The use was refused.
    Refused,
    /// A scan of this body is already running; the use still succeeds and
    /// starts nothing.
    AlreadyScanning,
    /// Scan this crystal's user.
    Started { item: &'a Object },
}

/// Use the reconstruction device `children`/`objects` stand for. Its checks
/// are the teleporter's without the charge test, and with the Power Crystal
/// L as the only accepted object.
pub fn use_res_device<'a>(
    blocked: bool,
    device: Vec3,
    user: Option<Vec3>,
    children: &[Child<'_>],
    objects: &'a [Object],
    scanning: bool,
) -> ScanUse<'a> {
    if blocked {
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
    if item.base_id() == 0 || !item.is_res_crystal() {
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
    /// The scan effect's start (ms on the device clock).
    pub effect_at_ms: u32,
    /// The device glow's rise and fall, and the millisecond it ends.
    pub pulse_rise_ms: u32,
    pub pulse_fall_ms: u32,
    pub pulse_end_ms: u32,
}

/// The scan a use at `now_ms` schedules.
pub fn scan_timing(now_ms: u32) -> ScanTiming {
    ScanTiming {
        effect_at_ms: now_ms.wrapping_add(SCAN_DELAY_MS),
        pulse_rise_ms: PULSE_RISE_MS,
        pulse_fall_ms: PULSE_FALL_MS,
        pulse_end_ms: now_ms.wrapping_add(PULSE_END_MS),
    }
}

/// The transition latch the device use raises and the level load consumes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LatchPhase {
    /// No transition is pending.
    #[default]
    Idle,
    /// A scan started; the next load of the same character strips it.
    Charging,
    /// The matching character arrived and was stripped (once).
    Arrived,
}

/// The state behind a use of a reconstruction device and the strip at the
/// other end.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Default)]
pub struct Latch {
    pub phase: LatchPhase,
    /// The character record's identity word the scan was started for.
    pub remembered: u32,
    /// The scan's stored value (0 at the start; its later reader is not
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

    /// The device was freed: clear the latch. Returns whether the power
    /// sound plays (`sound` is whether its handle is loaded).
    pub fn power_down(&mut self, sound: bool) -> bool {
        self.phase = LatchPhase::Idle;
        sound
    }

    /// A level load brought the character with `arriving`. If a scan is
    /// pending for that identity, settle the latch and report whether this
    /// arrival is the one that strips: the first matching load strips, the
    /// later ones only keep the settled latch.
    pub fn arrive(&mut self, arriving: u32) -> Option<bool> {
        if self.phase == LatchPhase::Idle || arriving != self.remembered {
            return None;
        }
        let strip = self.phase == LatchPhase::Charging;
        self.phase = LatchPhase::Arrived;
        Some(strip)
    }
}

/// The character-record fields a teleporter strip changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// The record's shared bit ([`SHARED_RECORD`]): a shared record is
    /// stripped in place, an unshared one works on a fresh copy first.
    pub shared: bool,
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
    /// The strip: clear both hands and the worn list, leave health and
    /// stamina at [`STRIPPED_WORD`], keep one empty action, and give a
    /// fresh record the two roles. The underwear rule runs after this, on
    /// the cleared worn list.
    pub fn strip(&mut self) {
        if !self.shared {
            self.roles.clear();
            self.roles.extend_from_slice(&STRIP_ROLES);
        }
        self.held = [0; 4];
        self.worn.clear();
        self.health = STRIPPED_WORD;
        self.stamina = STRIPPED_WORD;
        self.actions = vec![0];
    }
}

/// The share of full health and stamina the strip leaves
/// (0x7FFF / 0xFFFF, about 0.4999924: just under half).
pub fn stripped_share() -> f32 {
    STRIPPED_SHARE
}

/// The level a chosen destination id loads, through the locale list's
/// names. The original's travel job carries a level id and a second word;
/// the entrance is not part of the id, so it is 0 here.
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

/// Fill in the locale ids and names the shipped teleporter reaches, for
/// tests and callers without the level list: Exanima05, Exanima07,
/// Exanima08 and Exanima67.
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
        // The tier bit is ignored.
        assert!(crystal(0x8000_01AF, 0, Vec3::ZERO).is_portal_crystal());
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
        let objects = [crystal(0x8000_01AF, 0, Vec3::new(0.2, 16.0, 0.0))];
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
        assert_eq!(item.id, 0x8000_01AF);
        assert!(!adapter);
        assert_eq!(screen.atlas, "PrtlSmblA.rfi");
        assert_eq!(screen.rows(), 4);
        // A shut use gate refuses even with a crystal in place.
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

        // A zero entry stops the fill: row 0 takes that empty entry and the
        // rows already filled keep the earlier entries.
        let mut zero = record();
        zero.destinations[2] = 0;
        let screen = ScreenData::of(&zero);
        assert_eq!(screen.row_index(0), Some(2));
        assert_eq!(screen.row_destination(0), Some(0));
        assert_eq!(screen.row_destination(1), Some(7));
        assert_eq!(screen.row_destination(2), Some(10));
        // A row the fill never reached keeps entry 0.
        assert_eq!(screen.row_destination(3), Some(7));
    }

    #[test]
    fn the_reconstruction_device_takes_only_the_large_crystal() {
        let children = [child("[000001F4L]", Vec3::ZERO)];
        let large = [crystal(LARGE_POWER_CRYSTAL, 0xFF, Vec3::ZERO)];
        let small = [crystal(POWER_CRYSTAL, 0, Vec3::ZERO)];
        assert!(matches!(
            use_res_device(false, Vec3::ZERO, None, &children, &large, false),
            ScanUse::Started { item } if item.id == LARGE_POWER_CRYSTAL
        ));
        // Its charge is not read: a spent crystal still works.
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
        assert_eq!(timing.pulse_rise_ms, 1_500);
        assert_eq!(timing.pulse_fall_ms, 3_800);
        assert_eq!(timing.pulse_end_ms, 7_800);
        assert_eq!(SCAN_STATE, 8);
        assert_eq!(SCAN_SCALE, 3.0);
        // The clock wraps like the original's 32-bit millisecond stamp.
        assert_eq!(scan_timing(u32::MAX - 5).effect_at_ms, 1_594);
    }

    #[test]
    fn the_latch_strips_the_matching_character_once() {
        let mut latch = Latch::default();
        assert_eq!(latch.phase, LatchPhase::Idle);
        assert_eq!(latch.arrive(0), None);
        latch.begin(0x00C1_0ED0);
        assert_eq!(latch.phase, LatchPhase::Charging);
        assert_eq!(latch.remembered, 0x00C1_0ED0);
        assert_eq!(latch.value, 0.0);
        // Another character does not consume it.
        assert_eq!(latch.arrive(0x00C1_0ED1), None);
        assert_eq!(latch.phase, LatchPhase::Charging);
        // The matching arrival strips once.
        assert_eq!(latch.arrive(0x00C1_0ED0), Some(true));
        assert_eq!(latch.phase, LatchPhase::Arrived);
        assert_eq!(latch.arrive(0x00C1_0ED0), Some(false));
        // The power sound clears it and reports whether it played.
        assert!(!latch.power_down(false));
        assert_eq!(latch.phase, LatchPhase::Idle);
        latch.begin(7);
        assert!(latch.power_down(true));
        assert_eq!(latch.phase, LatchPhase::Idle);
        assert_eq!(latch.arrive(7), None);
    }

    #[test]
    fn the_strip_clears_the_body_and_leaves_half_health() {
        let mut record = Record {
            shared: false,
            held: [10, 11, 12, 13],
            worn: vec![20, 21],
            health: 0xFFFF,
            stamina: 0xFFFF,
            actions: vec![7, 8, 9],
            roles: vec![9],
        };
        record.strip();
        assert_eq!(record.held, [0; 4]);
        assert!(record.worn.is_empty());
        assert_eq!((record.health, record.stamina), (0x7FFF, 0x7FFF));
        assert_eq!(record.actions, [0]);
        assert_eq!(record.roles, [1, 0xC2]);
        // A shared record keeps its roles.
        let mut shared = Record {
            shared: true,
            roles: vec![9],
            ..record.clone()
        };
        shared.strip();
        assert_eq!(shared.roles, [9]);
        assert_eq!(shared.held, [0; 4]);
        // 32767 of 65535.
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
