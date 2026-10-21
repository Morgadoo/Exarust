//! Using placed teleporter and reconstruction devices, and the
//! reconstruction a device's scan leads to.
//!
//! The rules live in [`exa_world::teleporters`]; this module is the bridge
//! to the Bevy world. It answers a [`UseDevice`] request from a device's
//! `[` slot and the placed objects around it:
//!
//! - a teleporter asks for the destination screen ([`OpenPortalScreen`]);
//! - a reconstruction device puts a waiting scan effect on the player
//!   ([`crate::effects::scan::ScanEffect`], 1600 ms ahead, closing with the
//!   flash), plays `devicescna01` at the player, glows (the device's voxel
//!   lights follow the 1500/3800/1500 ms pulse, [`DeviceGlow`]) and raises
//!   the reconstruction latch with the player's [`RecordIdentity`]. A scan
//!   effect already on the player makes the use start nothing.
//!
//! [`SlotReleased`] (an object let go off a device's slot point) powers a
//! reconstruction device down: `devicepwra01` at the device and the latch
//! cleared. [`EndSession`] decides how the player's session ends
//! ([`teleporters::session_end`]); a reconstruction strips the player the
//! first time (hand and worn data cleared, health just under half) and is
//! reported as [`SessionEnded`].
//!
//! All times are the effects' game clock ([`EffectClock`]). The scan and
//! the glow are the device unit's own effects; the destination screen and
//! its journey are [`crate::destination`]; the grab code that sends
//! [`SlotReleased`], the level change and the corpse record a strip leaves
//! behind belong to other modules that are not in yet.

use bevy::prelude::*;
use exa_character::{Outfit, WearsUnderwear, Wielded};
use exa_formats::objdb::operative;
use exa_render::gi::VoxelLight;
use exa_world::teleporters::{
    self, Child, Latch, Object, PortalUse, SLOT_PREFIX, ScanTiming, ScanUse, ScreenData, SessionEnd,
};
use exa_world::{PlacedItem, TeleportDevice};

use crate::effects::EffectClock;
use crate::effects::scan::{Scan, ScanEffect};

/// The original's possession: while the player controls another body, the
/// devices refuse every use.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Possession {
    pub active: bool,
}

/// A request to use a placed device, from the interaction code: the player
/// clicked it.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct UseDevice {
    pub device: Entity,
    /// The player: the original measures the reach from the player and
    /// scans the player, whoever asked.
    pub user: Entity,
}

/// The character record's identity word, the value a reconstruction-device
/// use remembers and the session end compares (a copied record starts at
/// 0).
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RecordIdentity(pub u32);

/// Open the teleporter's destination screen (the screen module consumes
/// this).
#[derive(Message, Debug, Clone, PartialEq)]
pub struct OpenPortalScreen {
    pub device: Entity,
    pub user: Entity,
    /// The crystal that pays for the journey.
    pub item: Entity,
    /// The crystal is the large-crystal adapter, not a small crystal.
    pub adapter: bool,
    /// The atlas, rows and level ids.
    pub screen: ScreenData,
}

/// A device's glow pulse, stepped on the game clock until it is over.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceGlow {
    pub start_ms: u32,
    pub ramp_ms: u32,
    pub hold_ms: u32,
}

/// A voxel light's colour before a glow scaled it.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct GlowBase(pub Vec3);

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

/// An object was let go off this device's slot point (within
/// [`teleporters::SLOT_RELEASE_SQUARED_CM`] of it), from the grab code.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotReleased {
    pub device: Entity,
}

/// The play session ends for the player `user`; `story` is whether it is
/// the story mode.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndSession {
    pub user: Entity,
    pub story: bool,
}

/// How the session ended.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionEnded {
    pub user: Entity,
    pub end: SessionEnd,
}

pub struct TeleportPlugin;

impl Plugin for TeleportPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EffectClock>()
            .init_resource::<Possession>()
            .init_resource::<Latch>()
            .add_message::<UseDevice>()
            .add_message::<OpenPortalScreen>()
            .add_message::<DeviceSound>()
            .add_message::<SlotReleased>()
            .add_message::<EndSession>()
            .add_message::<SessionEnded>()
            .add_systems(
                FixedUpdate,
                (use_devices, glow_devices, power_down_devices, end_sessions)
                    .chain()
                    .after(crate::effects::advance_clock),
            );
    }
}

/// The device's model nodes the original sees as its children: the model's
/// roots under the placement and their children, in order.
fn device_nodes(device: Entity, children: &Query<&Children>) -> Vec<Entity> {
    let mut nodes = Vec::new();
    let Ok(roots) = children.get(device) else {
        return nodes;
    };
    for &root in roots {
        nodes.push(root);
        if let Ok(kids) = children.get(root) {
            nodes.extend(kids.iter());
        }
    }
    nodes
}

/// The device's slot: the first of its nodes named with `[`.
fn find_slot_entity(
    device: Entity,
    children: &Query<&Children>,
    names: &Query<&Name>,
) -> Option<Entity> {
    device_nodes(device, children).into_iter().find(|&node| {
        names
            .get(node)
            .is_ok_and(|name| name.as_str().starts_with(SLOT_PREFIX))
    })
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
    scans: Query<(), With<ScanEffect>>,
    possession: Res<Possession>,
    clock: Res<EffectClock>,
    mut latch: ResMut<Latch>,
    mut screens: MessageWriter<OpenPortalScreen>,
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
        let children_cm = [Child {
            name: slot_name,
            at: slot_transform.translation() * 100.0,
        }];
        let device_cm = device_transform.translation() * 100.0;
        // Guess: the original measures from a point on the player's body
        // whose role is not established; the root (the feet) stands in.
        let user_cm = user_transform.translation() * 100.0;
        let mut objects = Vec::new();
        let mut entities = Vec::new();
        for (entity, item, transform) in &placed {
            objects.push(Object {
                id: item.id,
                condition: item.condition,
                at: transform.translation() * 100.0,
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
                    possession.active,
                    device_cm,
                    Some(user_cm),
                    &children_cm,
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
                let scanning = children
                    .get(use_it.user)
                    .is_ok_and(|kids| kids.iter().any(|kid| scans.contains(kid)));
                let ScanUse::Started { .. } = teleporters::use_res_device(
                    possession.active,
                    device_cm,
                    Some(user_cm),
                    &children_cm,
                    &objects,
                    scanning,
                ) else {
                    continue;
                };
                let ScanTiming {
                    effect_at_ms,
                    pulse_ramp_ms,
                    pulse_hold_ms,
                    ..
                } = teleporters::scan_timing(clock.now_ms);
                commands.spawn((
                    Name::new(crate::effects::scan::MATERIAL),
                    ScanEffect(scan_for_device(effect_at_ms)),
                    Transform::default(),
                    ChildOf(use_it.user),
                ));
                sounds.write(DeviceSound::scan(user_transform.translation()));
                let identity = identities.get(use_it.user).map(|i| i.0).unwrap_or(0);
                latch.begin(identity);
                commands.entity(use_it.device).insert(DeviceGlow {
                    start_ms: clock.now_ms,
                    ramp_ms: pulse_ramp_ms,
                    hold_ms: pulse_hold_ms,
                });
            }
            _ => {}
        }
    }
}

/// The scan a reconstruction device starts: waiting for `at_ms`, its close
/// looping with the flash. Its two stored random values are never read (the
/// scan's shader has no texture position), so they are 0 here.
pub fn scan_for_device(at_ms: u32) -> Scan {
    let mut scan = Scan::delayed(at_ms, [0.0, 0.0]);
    scan.looping = true;
    scan
}

/// Step every device glow and scale the voxel lights of the device's nodes
/// by it (the original also scales their emissive parts, which Exarust's
/// materials do not expose yet). The last step leaves the lights at 0.
fn glow_devices(
    mut commands: Commands,
    clock: Res<EffectClock>,
    glows: Query<(Entity, &DeviceGlow)>,
    children: Query<&Children>,
    mut lights: Query<(Entity, &mut VoxelLight, Option<&GlowBase>)>,
) {
    for (device, glow) in &glows {
        let step = teleporters::pulse_step(
            clock.now_ms.wrapping_sub(glow.start_ms),
            glow.ramp_ms,
            glow.hold_ms,
        );
        for node in device_nodes(device, &children) {
            let Ok((entity, mut light, base)) = lights.get_mut(node) else {
                continue;
            };
            let base = match base {
                Some(base) => base.0,
                None => {
                    commands.entity(entity).insert(GlowBase(light.colour));
                    light.colour
                }
            };
            light.colour = glowing(base, step.strength);
        }
        if step.done {
            commands.entity(device).remove::<DeviceGlow>();
        }
    }
}

/// A voxel light's colour at `strength` times its intensity: the colour
/// counts the intensity by its square root.
pub fn glowing(base: Vec3, strength: f32) -> Vec3 {
    base * strength.max(0.0).sqrt()
}

/// A crystal taken off a reconstruction device: `devicepwra01` at the
/// device and the latch cleared. Other devices ignore it.
fn power_down_devices(
    mut released: MessageReader<SlotReleased>,
    devices: Query<(&TeleportDevice, &GlobalTransform)>,
    mut latch: ResMut<Latch>,
    mut sounds: MessageWriter<DeviceSound>,
) {
    for release in released.read() {
        let Ok((device, transform)) = devices.get(release.device) else {
            continue;
        };
        if device.operative.kind != operative::RES_DEVICE {
            continue;
        }
        sounds.write(DeviceSound::power(transform.translation()));
        latch.power_down();
    }
}

/// The session end: the player's red health, the mode and the latch decide
/// it. The first reconstruction after a scan strips the player: no hand or
/// worn data, the underwear rule, and yellow and red health at the record's
/// 0x7FFF share of the maximum. (The original also leaves the dressed
/// copy of the record behind as the corpse; Exarust has no record copy
/// yet, and the wardrobe has no undress pass, so drawn garments stay.)
fn end_sessions(
    mut ends: MessageReader<EndSession>,
    mut latch: ResMut<Latch>,
    mut ended: MessageWriter<SessionEnded>,
    mut commands: Commands,
    mut sprays: Option<ResMut<exa_character::spray::BloodSprays>>,
    identities: Query<&RecordIdentity>,
    mut fighters: Query<&mut exa_combat::Fighter>,
) {
    for end in ends.read() {
        let Ok(mut fighter) = fighters.get_mut(end.user) else {
            continue;
        };
        let identity = identities.get(end.user).map(|i| i.0).unwrap_or(0);
        let outcome = teleporters::session_end(fighter.health.red, end.story, identity, &mut latch);
        // The session's spray jets are cleared as the original clears them
        // at session end; a respawn starts with none.
        if let Some(sprays) = sprays.as_mut() {
            sprays.reset();
        }
        if outcome == (SessionEnd::Reconstructed { stripped: true }) {
            let share = teleporters::stripped_share();
            fighter.health.yellow = fighter.health.max * share;
            fighter.health.red = fighter.health.max * share;
            commands
                .entity(end.user)
                .remove::<Wielded>()
                .insert((Outfit::default(), WearsUnderwear));
        }
        ended.write(SessionEnded {
            user: end.user,
            end: outcome,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_combat::Fighter;
    use exa_formats::character_stats::CharacterStats;
    use exa_formats::objdb::{DESTINATIONS, Operative};
    use exa_world::teleporters::LatchPhase;

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

    /// The device's scan is the effects' scan in its waiting state with the
    /// device's flash: it opens after its stamp, over 4800 ms, closes over
    /// 800 ms and leaves 7200 ms after the use plus the steps' slack.
    #[test]
    fn the_device_scan_waits_opens_and_flashes_out() {
        assert_eq!(
            teleporters::SCAN_STATE,
            Scan::delayed(0, [0.0; 2]).state.word()
        );
        assert_eq!(teleporters::SCAN_SCALE, crate::effects::scan::FLASH_SCALE);
        let mut scan = scan_for_device(teleporters::scan_timing(1_000).effect_at_ms);
        assert!(scan.looping);
        assert_eq!(scan.stamp, 2_600);
        assert!(!scan.step(2_600).remove);
        // Strictly after the stamp it opens.
        scan.step(2_601);
        assert_eq!(scan.stamp, 2_601);
        scan.step(7_401);
        assert_eq!(scan.shown, 1.0);
        // Past one it closes at once, looping: the flash at three times the
        // fraction left.
        scan.step(7_402);
        scan.step(7_403);
        scan.step(7_803);
        assert!((scan.colour_scale - 1.5).abs() < 1e-6);
        scan.step(8_203);
        let step = scan.step(8_204);
        assert!(step.remove);
    }

    #[test]
    fn a_glowing_light_counts_its_strength_by_the_square_root() {
        assert_eq!(
            glowing(Vec3::new(2.0, 4.0, 0.0), 0.25),
            Vec3::new(1.0, 2.0, 0.0)
        );
        assert_eq!(glowing(Vec3::ONE, 0.0), Vec3::ZERO);
        assert_eq!(glowing(Vec3::ONE, -0.5), Vec3::ZERO);
    }

    fn operative(kind: u8) -> Operative {
        let mut destinations = [0u32; DESTINATIONS];
        destinations[..4].copy_from_slice(&[7, 10, 12, 13]);
        Operative {
            class: operative::DEVICE,
            kind,
            flags: 0,
            value: 0,
            time: 0,
            target: 0,
            second: 0,
            atlas: [0; 16],
            destinations,
        }
    }

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<EffectClock>()
            .init_resource::<Possession>()
            .init_resource::<Latch>()
            .add_message::<UseDevice>()
            .add_message::<OpenPortalScreen>()
            .add_message::<DeviceSound>()
            .add_message::<SlotReleased>()
            .add_message::<EndSession>()
            .add_message::<SessionEnded>()
            .add_systems(
                Update,
                (use_devices, glow_devices, power_down_devices, end_sessions).chain(),
            );
        app
    }

    /// A device at the origin whose model root carries the `[` slot 16 cm
    /// up and a voxel light; a crystal of `crystal` on the slot; the player
    /// beside it.
    fn device(app: &mut App, kind: u8, crystal: u32) -> (Entity, Entity) {
        let world = app.world_mut();
        let device = world
            .spawn((
                TeleportDevice {
                    kind,
                    operative: operative(kind),
                },
                GlobalTransform::IDENTITY,
            ))
            .id();
        let root = world
            .spawn((
                Name::new("frame"),
                GlobalTransform::IDENTITY,
                ChildOf(device),
            ))
            .id();
        world.spawn((
            Name::new("[000001F4L]"),
            GlobalTransform::from_translation(Vec3::new(0.0, 0.16, 0.0)),
            ChildOf(root),
        ));
        world.spawn((
            Name::new("glow"),
            VoxelLight {
                color: [0.0, 1.0, 0.0],
                intensity: 16.0,
                base_intensity: 16.0,
                radius: 0.0,
                active_radius: 0.0,
                colour: Vec3::new(0.0, 4.0, 0.0),
                flags: 0,
            },
            ChildOf(root),
        ));
        world.spawn((
            PlacedItem {
                id: crystal,
                condition: 0,
            },
            GlobalTransform::from_translation(Vec3::new(0.0, 0.16, 0.002)),
        ));
        let user = world
            .spawn((
                GlobalTransform::from_translation(Vec3::new(0.5, 0.0, 0.0)),
                RecordIdentity(7),
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
        (device, user)
    }

    fn at(app: &mut App, ms: u32) {
        app.world_mut().resource_mut::<EffectClock>().now_ms = ms;
        app.update();
    }

    fn scans_on(app: &mut App, user: Entity) -> usize {
        let world = app.world_mut();
        let mut query = world.query::<(&ScanEffect, &ChildOf)>();
        query
            .iter(world)
            .filter(|(_, parent)| parent.parent() == user)
            .count()
    }

    fn light(app: &mut App) -> Vec3 {
        let world = app.world_mut();
        let mut query = world.query::<&VoxelLight>();
        query.single(world).unwrap().colour
    }

    #[test]
    fn a_reconstruction_device_scans_glows_and_latches() {
        let mut app = app();
        let (device, user) = device(&mut app, operative::RES_DEVICE, 0x1F4);
        app.world_mut().write_message(UseDevice { device, user });
        at(&mut app, 1_000);
        assert_eq!(scans_on(&mut app, user), 1);
        let latch = *app.world().resource::<Latch>();
        assert_eq!((latch.phase, latch.remembered), (LatchPhase::Charging, 7));
        assert!(app.world().get::<DeviceGlow>(device).is_some());
        // A second use while the scan is on the player starts nothing more.
        app.world_mut().write_message(UseDevice { device, user });
        at(&mut app, 1_012);
        assert_eq!(scans_on(&mut app, user), 1);
        // The glow: half way up the ramp the light is at √0.5 of its base.
        at(&mut app, 1_750);
        assert!((light(&mut app) - Vec3::new(0.0, 4.0 * 0.5f32.sqrt(), 0.0)).length() < 1e-6);
        at(&mut app, 3_000);
        assert_eq!(light(&mut app), Vec3::new(0.0, 4.0, 0.0));
        // Over after 6800 ms: the light is left dark and the glow removed.
        at(&mut app, 7_801);
        assert_eq!(light(&mut app), Vec3::ZERO);
        assert!(app.world().get::<DeviceGlow>(device).is_none());
        // The crystal taken off powers the device down.
        app.world_mut().write_message(SlotReleased { device });
        at(&mut app, 7_813);
        assert_eq!(app.world().resource::<Latch>().phase, LatchPhase::Idle);
    }

    #[test]
    fn a_teleporter_asks_for_its_screen_and_refuses_a_large_crystal() {
        let mut app = app();
        let (device, user) = device(&mut app, operative::TELEPORTER, 0x1AF);
        app.world_mut().write_message(UseDevice { device, user });
        at(&mut app, 0);
        let screens: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<OpenPortalScreen>>()
            .drain()
            .collect();
        assert_eq!(screens.len(), 1);
        assert_eq!(screens[0].screen.row_destination(0), Some(13));
        assert!(!screens[0].adapter);
        // The same pedestal with the reconstruction crystal: refused.
        let mut app = self::app();
        let (device, user) = self::device(&mut app, operative::TELEPORTER, 0x1F4);
        app.world_mut().write_message(UseDevice { device, user });
        at(&mut app, 0);
        assert!(
            app.world()
                .resource::<Messages<OpenPortalScreen>>()
                .is_empty()
        );
        // A possession refuses every use.
        let mut app = self::app();
        let (device, user) = self::device(&mut app, operative::RES_DEVICE, 0x1F4);
        app.world_mut().resource_mut::<Possession>().active = true;
        app.world_mut().write_message(UseDevice { device, user });
        at(&mut app, 0);
        assert_eq!(scans_on(&mut app, user), 0);
    }

    fn ended(app: &mut App, user: Entity, story: bool) -> SessionEnd {
        app.world_mut().write_message(EndSession { user, story });
        app.update();
        let ends: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<SessionEnded>>()
            .drain()
            .collect();
        ends.last().unwrap().end
    }

    fn health(app: &App, user: Entity) -> exa_combat::Health {
        app.world().get::<Fighter>(user).unwrap().health
    }

    #[test]
    fn a_scanned_player_who_dies_in_the_story_is_reconstructed_stripped_once() {
        let mut app = app();
        let (_, user) = device(&mut app, operative::RES_DEVICE, 0x1F4);
        // Alive: the session is left and nothing changes.
        assert_eq!(ended(&mut app, user, true), SessionEnd::Left);
        app.world_mut().get_mut::<Fighter>(user).unwrap().health.red = 0.0;
        // Dead without a scan: an ordinary death.
        assert_eq!(ended(&mut app, user, true), SessionEnd::Died);
        app.world_mut().resource_mut::<Latch>().begin(7);
        // Outside the story the latch does not help.
        assert_eq!(ended(&mut app, user, false), SessionEnd::Died);
        assert!(app.world().get::<Wielded>(user).is_some());
        // In the story: reconstructed and stripped.
        assert_eq!(
            ended(&mut app, user, true),
            SessionEnd::Reconstructed { stripped: true }
        );
        assert!(app.world().get::<Wielded>(user).is_none());
        let stripped = health(&app, user);
        assert!((stripped.yellow - 49.999_237).abs() < 1e-3, "{stripped:?}");
        assert!((stripped.red - 49.999_237).abs() < 1e-3, "{stripped:?}");
        // Dying again reconstructs without a second strip.
        app.world_mut().get_mut::<Fighter>(user).unwrap().health.red = 0.005;
        assert_eq!(
            ended(&mut app, user, true),
            SessionEnd::Reconstructed { stripped: false }
        );
        assert_eq!(health(&app, user).red, 0.005);
    }
}
