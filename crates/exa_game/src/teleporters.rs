//! Using placed teleporter and reconstruction devices, and the level-load
//! strip they lead to.
//!
//! The rules live in [`exa_world::teleporters`]; this module is the bridge
//! to the Bevy world: it answers a [`UseDevice`] request from a device's
//! `[` slot and the placed objects around it, and turns the result into
//! requests the other modules consume. The destination screen and the scan
//! aura are drawn by the effects modules: this module only asks for them,
//! with the parameters the original gives them.
//!
//! A reconstruction-device use raises the transition latch with the user's
//! [`RecordIdentity`]. A level load sends [`ArriveAtLevel`] for the arriving
//! character; the first load with the remembered identity strips the
//! character once (hands and worn items cleared, the underwear rule, health
//! and stamina at half). The strip's record rule is
//! [`exa_world::teleporters::Record::strip`]; the wardrobe here has no
//! undress pass, so only the components' data is cleared.
//!
//! Evidence: the device-use rules, the 12 ms device clock, the latch's
//! consumer in the level load and the strip. The original's shared use gate
//! is [`DeviceUseGate`]; what shuts it is not established.

use bevy::prelude::*;
use exa_character::{Outfit, WearsUnderwear, Wielded};
use exa_formats::objdb::operative;
use exa_world::teleporters::{
    self, Child, Latch, Object, PortalUse, SLOT_PREFIX, ScanTiming, ScanUse, ScreenData,
};
use exa_world::{PlacedItem, TeleportDevice};

/// The device clock's tick: the original's 12 ms simulation tick.
pub const DEVICE_TICK_MS: u32 = 12;
/// The scan effect's close length, from the scan's own rule.
pub const SCAN_SIZE_MS: u32 = 800;
/// The scan opens over this many close lengths.
pub const SCAN_OPEN_FACTOR: u32 = 6;
/// How long a scan keeps a second use from starting another (ms): its delay,
/// its opening and its close.
pub const SCAN_ACTIVE_MS: u32 = teleporters::SCAN_DELAY_MS + SCAN_SIZE_MS * (SCAN_OPEN_FACTOR + 1);

/// The devices' millisecond clock, advanced once a tick.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DeviceClock {
    pub now_ms: u32,
}

/// The original's shared device-use gate: a use is refused while it is shut.
/// What shuts it (a modal screen, a script) is not established.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DeviceUseGate {
    pub blocked: bool,
}

/// A request to use a placed device, from the interaction code: the player
/// clicked it, or a script asked for it.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct UseDevice {
    pub device: Entity,
    /// The character using it; the original always uses the player for these
    /// two devices.
    pub user: Entity,
}

/// The character record's identity word, the value a reconstruction-device
/// use remembers and a level load compares (the original keeps it in the
/// record's role word; a new record starts at 0).
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RecordIdentity(pub u32);

/// A scan is running on this character: another use starts nothing.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanInProgress {
    /// The clock time the scan's effect is gone (ms). Guess: the original
    /// tests for the effect node itself, so this stand-in lasts the effect's
    /// delay, opening and close.
    pub until_ms: u32,
}

/// Open the teleporter's destination screen (the UI module consumes this).
#[derive(Message, Debug, Clone, PartialEq)]
pub struct OpenPortalScreen {
    pub device: Entity,
    pub user: Entity,
    /// The crystal that pays for the journey: the screen consumes it.
    pub item: Entity,
    /// The crystal is the large-crystal adapter, not a small crystal.
    pub adapter: bool,
    /// The atlas, rows and level ids the record names.
    pub screen: ScreenData,
}

/// Start the teleporter scan on a character (the effects module consumes
/// this). The scan waits for `at_ms`, in the waiting state, and its close
/// loops with the device's colour scale.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct SpawnScanAura {
    pub user: Entity,
    pub at_ms: u32,
    pub state: i32,
    pub scale: f32,
    pub looping: bool,
}

/// The device's glow pulse after a scan (the effects module consumes this):
/// the strength rises over `rise_ms` and falls over `fall_ms`.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DevicePulse {
    pub device: Entity,
    pub rise_ms: u32,
    pub fall_ms: u32,
}

/// The sound a device makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundKind {
    /// The scan's `devicescna01`.
    Scan,
    /// The power-down `devicepwra01`.
    Power,
}

/// A device sound request, at a world position in metres.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct DeviceSound {
    pub at: Vec3,
    pub kind: SoundKind,
    pub volume: f32,
    pub pitch: f32,
}

impl DeviceSound {
    /// The scan's sound: `devicescna01`, 0.3, pitch 1.
    pub fn scan(at: Vec3) -> Self {
        Self {
            at,
            kind: SoundKind::Scan,
            volume: teleporters::SCAN_VOLUME,
            pitch: teleporters::SOUND_PITCH,
        }
    }

    /// The power-down sound: `devicepwra01`, 0.1, pitch 1.
    pub fn power(at: Vec3) -> Self {
        Self {
            at,
            kind: SoundKind::Power,
            volume: teleporters::POWER_VOLUME,
            pitch: teleporters::SOUND_PITCH,
        }
    }
}

/// A character arrived through a teleporter and was stripped.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stripped {
    pub user: Entity,
}

/// The level load reached: `user` arrived with the record identity
/// `identity`. The latch decides whether this load is the one that strips.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArriveAtLevel {
    pub user: Entity,
    pub identity: u32,
}

pub struct TeleportPlugin;

impl Plugin for TeleportPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DeviceClock>()
            .init_resource::<DeviceUseGate>()
            .init_resource::<Latch>()
            .add_message::<UseDevice>()
            .add_message::<OpenPortalScreen>()
            .add_message::<SpawnScanAura>()
            .add_message::<DevicePulse>()
            .add_message::<DeviceSound>()
            .add_message::<Stripped>()
            .add_message::<ArriveAtLevel>()
            .add_systems(
                FixedUpdate,
                (
                    advance_device_clock,
                    use_devices,
                    expire_scans,
                    power_down_devices,
                    arrive_level,
                )
                    .chain(),
            );
    }
}

/// A reconstruction device that leaves the world powers down: the latch
/// clears and `devicepwra01` plays where the device still has a transform.
fn power_down_devices(
    mut removed: RemovedComponents<TeleportDevice>,
    transforms: Query<&GlobalTransform>,
    mut latch: ResMut<Latch>,
    mut sounds: MessageWriter<DeviceSound>,
) {
    for entity in removed.read() {
        // The sound's handle is loaded whenever the sound pack is; a missing
        // entry is skipped by the audio hook.
        let played = latch.power_down(true);
        if played && let Ok(transform) = transforms.get(entity) {
            sounds.write(DeviceSound::power(transform.translation()));
        }
    }
}

fn advance_device_clock(mut clock: ResMut<DeviceClock>) {
    clock.now_ms = clock.now_ms.wrapping_add(DEVICE_TICK_MS);
}

/// The first descendant of `root` whose name starts with the slot prefix.
fn find_slot_entity(
    root: Entity,
    children: &Query<&Children>,
    names: &Query<&Name>,
) -> Option<Entity> {
    let mut stack = vec![root];
    let mut visited = 0usize;
    while let Some(entity) = stack.pop() {
        visited += 1;
        if visited > 4096 {
            // A malformed hierarchy must not spin; a device has far fewer
            // nodes.
            break;
        }
        if names
            .get(entity)
            .is_ok_and(|name| name.as_str().starts_with(SLOT_PREFIX))
        {
            return Some(entity);
        }
        if let Ok(kids) = children.get(entity) {
            stack.extend(kids.iter());
        }
    }
    None
}

#[allow(clippy::too_many_arguments)]
fn use_devices(
    mut commands: Commands,
    mut uses: MessageReader<UseDevice>,
    devices: Query<(&TeleportDevice, &GlobalTransform)>,
    names: Query<&Name>,
    children: Query<&Children>,
    placed: Query<(Entity, &PlacedItem, &GlobalTransform)>,
    transforms: Query<&GlobalTransform>,
    identities: Query<&RecordIdentity>,
    scans: Query<&ScanInProgress>,
    gate: Res<DeviceUseGate>,
    clock: Res<DeviceClock>,
    mut latch: ResMut<Latch>,
    mut screens: MessageWriter<OpenPortalScreen>,
    mut auras: MessageWriter<SpawnScanAura>,
    mut pulses: MessageWriter<DevicePulse>,
    mut sounds: MessageWriter<DeviceSound>,
) {
    for use_it in uses.read() {
        let Ok((device, device_transform)) = devices.get(use_it.device) else {
            continue;
        };
        let Ok(user_transform) = transforms.get(use_it.user) else {
            continue;
        };
        let Some(slot) = find_slot_entity(use_it.device, &children, &names) else {
            continue;
        };
        let Ok(slot_transform) = transforms.get(slot) else {
            continue;
        };
        let slot_name = names.get(slot).map(|n| n.as_str()).unwrap_or("[slot]");
        let slot_cm = slot_transform.translation() * 100.0;
        let children = [Child {
            name: slot_name,
            at: slot_cm,
        }];
        let device_cm = device_transform.translation() * 100.0;
        let user_cm = user_transform.translation() * 100.0;
        let mut objects = Vec::new();
        let mut entities = Vec::new();
        for (entity, item, transform) in &placed {
            let at = transform.translation() * 100.0;
            if !at.is_finite() {
                continue;
            }
            objects.push(Object {
                id: item.id,
                condition: item.condition,
                at,
            });
            entities.push(entity);
        }
        match device.kind {
            operative::TELEPORTER => {
                let PortalUse::Open {
                    item,
                    adapter,
                    screen,
                } = teleporters::use_portal(
                    gate.blocked,
                    device_cm,
                    Some(user_cm),
                    &children,
                    &objects,
                    Some(&device.operative),
                    false,
                )
                else {
                    continue;
                };
                let Some(index) = objects.iter().position(|object| std::ptr::eq(object, item))
                else {
                    continue;
                };
                screens.write(OpenPortalScreen {
                    device: use_it.device,
                    user: use_it.user,
                    item: entities[index],
                    adapter,
                    screen,
                });
            }
            operative::RES_DEVICE => {
                let scanning = scans.contains(use_it.user);
                let ScanUse::Started { .. } = teleporters::use_res_device(
                    gate.blocked,
                    device_cm,
                    Some(user_cm),
                    &children,
                    &objects,
                    scanning,
                ) else {
                    continue;
                };
                let identity = identities.get(use_it.user).map(|i| i.0).unwrap_or(0);
                latch.begin(identity);
                let ScanTiming {
                    effect_at_ms,
                    pulse_rise_ms,
                    pulse_fall_ms,
                    ..
                } = teleporters::scan_timing(clock.now_ms);
                auras.write(SpawnScanAura {
                    user: use_it.user,
                    at_ms: effect_at_ms,
                    state: teleporters::SCAN_STATE,
                    scale: teleporters::SCAN_SCALE,
                    looping: true,
                });
                pulses.write(DevicePulse {
                    device: use_it.device,
                    rise_ms: pulse_rise_ms,
                    fall_ms: pulse_fall_ms,
                });
                sounds.write(DeviceSound::scan(user_transform.translation()));
                commands.entity(use_it.user).insert(ScanInProgress {
                    until_ms: clock.now_ms.wrapping_add(SCAN_ACTIVE_MS),
                });
            }
            _ => {}
        }
    }
}

fn expire_scans(
    mut commands: Commands,
    clock: Res<DeviceClock>,
    scans: Query<(Entity, &ScanInProgress)>,
) {
    for (entity, scan) in &scans {
        if clock.now_ms.wrapping_sub(scan.until_ms) < 0x8000_0000 {
            commands.entity(entity).remove::<ScanInProgress>();
        }
    }
}

/// The level load's strip: the first matching arrival after a scan clears
/// the character's hands and worn data and leaves half health. The
/// original's record strip also leaves one empty action and, on a fresh
/// record, two roles ([`teleporters::Record`]); the wardrobe here has no
/// undress pass, so its garments stay drawn.
fn arrive_level(
    mut arrivals: MessageReader<ArriveAtLevel>,
    mut latch: ResMut<Latch>,
    mut strips: MessageWriter<Stripped>,
    mut commands: Commands,
    mut fighters: Query<&mut exa_combat::Fighter>,
) {
    for arrival in arrivals.read() {
        let Some(strip_now) = latch.arrive(arrival.identity) else {
            continue;
        };
        if !strip_now {
            continue;
        }
        if let Ok(mut fighter) = fighters.get_mut(arrival.user) {
            let share = teleporters::stripped_share();
            fighter.health.yellow = fighter.health.max * share;
            fighter.health.red = fighter.health.max * share;
        }
        commands
            .entity(arrival.user)
            .remove::<Wielded>()
            .insert((Outfit::default(), WearsUnderwear));
        strips.write(Stripped { user: arrival.user });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_combat::Fighter;
    use exa_formats::character_stats::CharacterStats;
    use exa_world::teleporters::LatchPhase;

    #[test]
    fn a_scan_keeps_the_next_use_off_for_seven_seconds() {
        assert_eq!(SCAN_ACTIVE_MS, 1_600 + 800 * 7);
        assert_eq!(SCAN_ACTIVE_MS, 7_200);
        let timing = teleporters::scan_timing(1_000);
        assert_eq!(timing.effect_at_ms, 2_600);
        assert_eq!(timing.pulse_end_ms, 7_800);
        assert_eq!(SCAN_ACTIVE_MS, teleporters::SCAN_DELAY_MS + 5_600);
    }

    #[test]
    fn the_device_clock_advances_one_tick() {
        let mut clock = DeviceClock::default();
        clock.now_ms = clock.now_ms.wrapping_add(DEVICE_TICK_MS);
        assert_eq!(clock.now_ms, 12);
    }

    /// The two device sounds and their levels: `devicescna01` at 0.3 and
    /// `devicepwra01` at 0.1, both at pitch 1.
    #[test]
    fn the_device_sounds_are_pinned() {
        let scan = DeviceSound::scan(Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(scan.kind, SoundKind::Scan);
        assert_eq!((scan.volume, scan.pitch), (0.3, 1.0));
        assert_eq!(scan.at, Vec3::new(1.0, 2.0, 3.0));
        let power = DeviceSound::power(Vec3::ZERO);
        assert_eq!(power.kind, SoundKind::Power);
        assert_eq!((power.volume, power.pitch), (0.1, 1.0));
        assert_eq!(teleporters::SCAN_SOUND, "devicescna01");
        assert_eq!(teleporters::POWER_SOUND, "devicepwra01");
    }

    fn app() -> App {
        let mut app = App::new();
        app.add_message::<ArriveAtLevel>()
            .add_message::<Stripped>()
            .init_resource::<Latch>()
            .add_systems(Update, arrive_level);
        app
    }

    fn arrived(app: &mut App, user: Entity, identity: u32) {
        app.world_mut()
            .write_message(ArriveAtLevel { user, identity });
        app.update();
    }

    fn fighter(app: &mut App, user: Entity) -> exa_combat::Health {
        app.world().get::<Fighter>(user).unwrap().health
    }

    #[test]
    fn the_matching_arrival_strips_once() {
        let mut app = app();
        let user = app
            .world_mut()
            .spawn((
                Fighter::of_stats(
                    None,
                    CharacterStats {
                        max_health: 100.0,
                        yellow: 100.0,
                        red: 100.0,
                        ..Default::default()
                    },
                ),
                Outfit::default(),
                Wielded::model("ub05ssworda"),
            ))
            .id();
        app.world_mut().resource_mut::<Latch>().begin(7);
        // Another identity does nothing.
        arrived(&mut app, user, 8);
        assert!(app.world().get::<Wielded>(user).is_some());
        assert_eq!(fighter(&mut app, user).yellow, 100.0);
        // The matching arrival clears the hand and leaves half health.
        arrived(&mut app, user, 7);
        assert!(app.world().get::<Wielded>(user).is_none());
        let health = fighter(&mut app, user);
        assert!((health.yellow - 49.999_237).abs() < 1e-3, "{health:?}");
        assert!((health.red - 49.999_237).abs() < 1e-3, "{health:?}");
        assert_eq!(app.world().resource::<Latch>().phase, LatchPhase::Arrived);
        // A second load keeps the health it has.
        arrived(&mut app, user, 7);
        assert_eq!(fighter(&mut app, user).yellow, health.yellow);
    }
}
