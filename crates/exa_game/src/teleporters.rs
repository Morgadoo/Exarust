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
//!   flash), gives it the device unit's aura (the `_ScanAura01` material,
//!   its 184-byte grey block, the state bits 0x9000, one sub-object and the
//!   shared noise picture), plays `devicescna01` at the player, glows (the
//!   device's voxel lights follow the 1500/3800/1500 ms pulse,
//!   [`DeviceGlow`]) and raises the reconstruction latch with the player's
//!   [`RecordIdentity`]. A scan effect already on the player makes the use
//!   start nothing.
//!
//! [`SlotReleased`] (an object let go off a device's slot point) powers a
//! reconstruction device down: `devicepwra01` at the device and the latch
//! cleared. [`EndSession`] decides how the player's session ends
//! ([`teleporters::session_end`]); a reconstruction strips the player the
//! first time (hand and worn data cleared, health just under half) and is
//! reported as [`SessionEnded`]. The latch is the story session's
//! reconstruction word as well: a scan sets it to rebuildable and a power
//! down clears it, so the play flow's death handler sees the device's
//! write.
//!
//! All times are the effects' game clock ([`EffectClock`]). The scan and
//! the glow are the device unit's own effects; the destination screen and
//! its journey are [`crate::destination`]; a let-go within 5 cm² of a device's `[` slot sends [`SlotReleased`].
//! The level change and the corpse record a strip leaves behind are not in yet.

use bevy::prelude::*;
use exa_character::{Outfit, WearsUnderwear, Wielded};
use exa_formats::objdb::operative;
use exa_render::gi::VoxelLight;
use exa_world::teleporters::{
    self, Child, Latch, Object, PortalUse, SLOT_PREFIX, ScanTiming, ScanUse, ScreenData, SessionEnd,
};
use exa_world::{PlacedItem, TeleportDevice};

use crate::effects::EffectClock;
use crate::effects::scan::{self, Scan, ScanEffect};

/// The original's possession: while the player controls another body, the
/// devices refuse every use.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Possession {
    pub active: bool,
}

/// A request to use a placed device, from the interaction code: the player
/// clicked it ([`crate::object_click`], the click's device use).
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
        bridge_teleport_types(app);
        exa_physics::sim_app::add_frame_commands(app);
        exa_physics::sim_app::add_fixed_commands(app);
        app.init_resource::<EffectClock>()
            .init_resource::<exa_physics::sim_app::FrameCommands>()
            .init_resource::<Possession>()
            .init_resource::<Latch>()
            .init_resource::<scan::ScanBuild>()
            .init_resource::<DebugScan>()
            .add_message::<UseDevice>()
            .add_message::<OpenPortalScreen>()
            .add_message::<DeviceSound>()
            .add_message::<SlotReleased>()
            .add_message::<EndSession>()
            .add_message::<SessionEnded>()
            .add_systems(
                FixedUpdate,
                (
                    release_from_slots,
                    use_devices,
                    glow_devices,
                    power_down_devices,
                    end_sessions,
                )
                    .chain()
                    .after(crate::effects::advance_clock),
            )
            .add_systems(Update, (debug_scan, debug_slot));
    }
}

/// The data this plugin's fixed-step systems touch, for the simulation's
/// bridge (`exa_physics::sim_bridge`).
fn bridge_teleport_types(app: &mut App) {
    use exa_physics::sim_bridge::SimBridgeAppExt;
    app.bridge_message::<crate::teleporters::DeviceSound>()
        .bridge_message::<crate::teleporters::EndSession>()
        .bridge_message::<crate::teleporters::OpenPortalScreen>()
        .bridge_message::<crate::teleporters::SessionEnded>()
        .bridge_message::<crate::teleporters::SlotReleased>()
        .bridge_message::<crate::teleporters::UseDevice>()
        .bridge_copy_in_resource::<crate::destination::PortalScreenState>()
        .bridge_move_resource::<crate::effects::scan::ScanBuild>()
        .bridge_copy_in_resource::<crate::flow::Flow>()
        .bridge_copy_in_resource::<crate::item_cursor::ItemCursor>()
        .bridge_copy::<crate::teleporters::DeviceGlow>()
        .bridge_copy::<crate::teleporters::GlowBase>()
        .bridge_copy_in_resource::<crate::teleporters::Possession>()
        .bridge_copy::<crate::teleporters::RecordIdentity>()
        .bridge_copy::<exa_render::gi::VoxelLight>()
        .bridge_copy::<exa_world::TeleportDevice>()
        .bridge_move_resource::<exa_world::teleporters::Latch>();
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

/// What a teleporter's or reconstruction device's use decides, before it
/// acts.
#[derive(Debug, Clone, PartialEq)]
pub enum DeviceDecision {
    /// Refused: the use answers no.
    Refused,
    /// The use answers yes and starts nothing (a destination screen already
    /// open, or a scan already on the player).
    Nothing,
    /// Open the destination screen for this crystal.
    OpenScreen {
        item: Entity,
        adapter: bool,
        screen: ScreenData,
    },
    /// Scan the player.
    Scan,
}

impl DeviceDecision {
    /// The use's answer: yes unless refused.
    pub fn answer(&self) -> bool {
        !matches!(self, DeviceDecision::Refused)
    }
}

/// What a device's use reads: the device, its `[` slot, the placed objects
/// around it, the player, the open screen and the possession. The click
/// reads the answer from it as the original's single click does (the use
/// runs before the click decides its card or narration) and the fixed step
/// acts on the same decision.
#[derive(bevy::ecs::system::SystemParam)]
pub struct DeviceReads<'w, 's> {
    devices: Query<'w, 's, (&'static TeleportDevice, &'static GlobalTransform)>,
    names: Query<'w, 's, &'static Name>,
    children: Query<'w, 's, &'static Children>,
    placed: Query<'w, 's, (Entity, &'static PlacedItem, &'static GlobalTransform)>,
    transforms: Query<'w, 's, &'static GlobalTransform>,
    scans: Query<'w, 's, (), With<ScanEffect>>,
    possession: Option<Res<'w, Possession>>,
    open: Option<Res<'w, crate::destination::PortalScreenState>>,
}

impl DeviceReads<'_, '_> {
    /// Whether a destination screen is showing: the original finds a
    /// visible destination form with its own id on the interface and
    /// answers "already open" then.
    fn screen_open(&self) -> bool {
        self.open.as_ref().is_some_and(|state| state.open.is_some())
    }

    /// The use of `device` for `user`, as the original's own use of a
    /// teleporter (class 2 kind 0x0F) or a reconstruction device (kind
    /// 0x12) decides it; anything that is no such device, or misses a
    /// piece, is refused.
    pub fn decide(&self, device: Entity, user: Entity) -> DeviceDecision {
        let Ok((record, device_transform)) = self.devices.get(device) else {
            return DeviceDecision::Refused;
        };
        let Ok(user_transform) = self.transforms.get(user) else {
            return DeviceDecision::Refused;
        };
        let Some(slot) = find_slot_entity(device, &self.children, &self.names) else {
            return DeviceDecision::Refused;
        };
        let Ok(slot_transform) = self.transforms.get(slot) else {
            return DeviceDecision::Refused;
        };
        let slot_name = self.names.get(slot).map(|n| n.as_str()).unwrap_or("[slot]");
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
        for (entity, item, transform) in &self.placed {
            objects.push(Object {
                id: item.id,
                condition: item.condition,
                at: transform.translation() * 100.0,
            });
            entities.push(entity);
        }
        let possessing = self.possession.as_ref().is_some_and(|p| p.active);
        match record.kind {
            operative::TELEPORTER => match teleporters::use_portal(
                possessing,
                device_cm,
                Some(user_cm),
                &children_cm,
                &objects,
                Some(&record.operative),
                self.screen_open(),
            ) {
                PortalUse::Refused => DeviceDecision::Refused,
                PortalUse::AlreadyOpen => DeviceDecision::Nothing,
                PortalUse::Open {
                    item,
                    adapter,
                    screen,
                } => match objects.iter().position(|object| std::ptr::eq(object, item)) {
                    Some(index) => DeviceDecision::OpenScreen {
                        item: entities[index],
                        adapter,
                        screen,
                    },
                    None => DeviceDecision::Refused,
                },
            },
            operative::RES_DEVICE => {
                let scanning = self
                    .children
                    .get(user)
                    .is_ok_and(|kids| kids.iter().any(|kid| self.scans.contains(kid)));
                match teleporters::use_res_device(
                    possessing,
                    device_cm,
                    Some(user_cm),
                    &children_cm,
                    &objects,
                    scanning,
                ) {
                    ScanUse::Refused => DeviceDecision::Refused,
                    ScanUse::AlreadyScanning => DeviceDecision::Nothing,
                    ScanUse::Started { .. } => DeviceDecision::Scan,
                }
            }
            _ => DeviceDecision::Refused,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn use_devices(
    mut commands: Commands,
    mut uses: MessageReader<UseDevice>,
    reads: DeviceReads,
    identities: Query<&RecordIdentity>,
    clock: Res<EffectClock>,
    mut latch: ResMut<Latch>,
    mut build: ResMut<scan::ScanBuild>,
    (mut flow, mut frame): (
        Option<ResMut<crate::flow::Flow>>,
        ResMut<exa_physics::sim_app::FrameCommands>,
    ),
    mut screens: MessageWriter<OpenPortalScreen>,
    mut sounds: MessageWriter<DeviceSound>,
) {
    for use_it in uses.read() {
        match reads.decide(use_it.device, use_it.user) {
            DeviceDecision::Refused | DeviceDecision::Nothing => {}
            DeviceDecision::OpenScreen {
                item,
                adapter,
                screen,
            } => {
                screens.write(OpenPortalScreen {
                    device: use_it.device,
                    user: use_it.user,
                    item,
                    adapter,
                    screen,
                });
            }
            DeviceDecision::Scan => {
                let Ok(user_transform) = reads.transforms.get(use_it.user) else {
                    continue;
                };
                spawn_device_scan(&mut commands, &mut build, use_it.user, clock.now_ms);
                let ScanTiming {
                    pulse_ramp_ms,
                    pulse_hold_ms,
                    ..
                } = teleporters::scan_timing(clock.now_ms);
                sounds.write(DeviceSound::scan(user_transform.translation()));
                let identity = identities.get(use_it.user).map(|i| i.0).unwrap_or(0);
                latch.begin(identity);
                // The same words the story session reads when the player dies:
                // the reconstruction word and, beside it, the identity it
                // compares with the dying player's record.
                if let Some(flow) = flow.as_mut() {
                    frame.write_resource(flow, move |flow| {
                        flow.story.reconstruction = crate::flow::REBUILDABLE;
                        flow.story.checkpoint = identity;
                    });
                }
                commands.entity(use_it.device).insert(DeviceGlow {
                    start_ms: clock.now_ms,
                    ramp_ms: pulse_ramp_ms,
                    hold_ms: pulse_hold_ms,
                });
            }
        }
    }
}

/// The scan a reconstruction device starts: waiting for `at_ms`, its
/// colour scale at the device's 3, its close looping with the flash,
/// carrying the two creation draws the device's effect node rolls (the
/// original stores them and never reads them).
pub fn scan_for_device(at_ms: u32, tex_pos: [f32; 2]) -> Scan {
    let mut scan = Scan::delayed(at_ms, tex_pos);
    scan.looping = true;
    scan.colour_scale = teleporters::SCAN_SCALE;
    scan
}

/// Puts a reconstruction device's scan on `user` at `now_ms`: the effect
/// node is created first and rolls its two draws, then the device builds
/// its aura and starts it; the node waits 1600 ms as a child of the user.
pub fn spawn_device_scan(
    commands: &mut Commands,
    build: &mut scan::ScanBuild,
    user: Entity,
    now_ms: u32,
) -> Entity {
    let tex_pos = build.draws();
    let mut aura = scan::ScanAura::default();
    let scan::ScanBuild {
        aura: shader, lock, ..
    } = build;
    scan::build_aura(&mut aura, shader, lock, true);
    scan::start_aura(&mut aura);
    let timing = teleporters::scan_timing(now_ms);
    commands
        .spawn((
            Name::new(crate::effects::scan::MATERIAL),
            ScanEffect(scan_for_device(timing.effect_at_ms, tex_pos)),
            aura,
            Transform::default(),
            ChildOf(user),
        ))
        .id()
}

/// The devices' debug flags. `--debug-scan <frame>` (`scan`): at that
/// frame after the load the player gets a reconstruction device's scan, as
/// a use that passed the device's checks starts it, and its sound.
/// `--debug-slot <frame>` (`slot`): see [`debug_slot`].
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DebugScan {
    pub scan: Option<u32>,
    pub slot: Option<u32>,
}

#[allow(clippy::too_many_arguments)]
fn debug_scan(
    mut fixed: ResMut<exa_physics::sim_app::FixedCommands>,
    script: Res<DebugScan>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    clock: Res<EffectClock>,
    players: Query<(Entity, &GlobalTransform), With<crate::player::PlayerBody>>,
    mut sounds: MessageWriter<DeviceSound>,
    mut frames: Local<u32>,
) {
    let Some(frame) = script.scan else {
        return;
    };
    if loading.is_some() || *frames > frame {
        return;
    }
    *frames += 1;
    if *frames != frame {
        return;
    }
    let Some((player, at)) = players.iter().next() else {
        info!("debug scan: no player at frame {frame}");
        return;
    };
    // The scan's build belongs to the fixed step (a device's use builds
    // it there): the debug scan is built at the next step.
    let now_ms = clock.now_ms;
    fixed.post(move |world| {
        world.resource_scope(|world, mut build: Mut<scan::ScanBuild>| {
            let mut commands = world.commands();
            spawn_device_scan(&mut commands, &mut build, player, now_ms);
        });
        world.flush();
    });
    sounds.write(DeviceSound::scan(at.translation()));
    info!(
        "debug scan: on the player at {} ms, opening at {} ms",
        clock.now_ms,
        clock.now_ms.wrapping_add(teleporters::SCAN_DELAY_MS)
    );
}

/// The crystal a device's slot takes: Power Crystal A on a teleporter,
/// Power Crystal L on a reconstruction device.
pub fn slot_crystal(kind: u8) -> Option<u32> {
    match kind {
        operative::TELEPORTER => Some(teleporters::POWER_CRYSTAL),
        operative::RES_DEVICE => Some(teleporters::LARGE_POWER_CRYSTAL),
        _ => None,
    }
}

/// `--debug-slot FRAME` ([`DebugScan::slot`]): at that frame after the
/// load, every placed teleporter gets a fresh Power Crystal A and every
/// reconstruction device a Power Crystal L on its `[` slot point, as a
/// player who put one there leaves it, so a click on the device runs its
/// use. The crystal is a placed object with no model or body [debug
/// stand-in: the original's crystal is the item the player brought].
#[allow(clippy::too_many_arguments)]
fn debug_slot(
    mut fixed: ResMut<exa_physics::sim_app::FixedCommands>,
    script: Res<DebugScan>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    devices: Query<(Entity, &TeleportDevice)>,
    names: Query<&Name>,
    children: Query<&Children>,
    transforms: Query<&GlobalTransform>,
    mut frames: Local<u32>,
) {
    let Some(frame) = script.slot else {
        return;
    };
    if loading.is_some() || *frames > frame {
        return;
    }
    *frames += 1;
    if *frames != frame {
        return;
    }
    let mut crystals = Vec::new();
    for (device, record) in &devices {
        let Some(id) = slot_crystal(record.kind) else {
            continue;
        };
        let Some(at) = find_slot_entity(device, &children, &names)
            .and_then(|slot| transforms.get(slot).ok())
            .map(GlobalTransform::translation)
        else {
            info!("debug slot: device {device:?} has no slot");
            continue;
        };
        info!("debug slot: crystal {id:#x} on device {device:?}'s slot at {at:?}");
        crystals.push((id, at));
    }
    fixed.post(move |world| {
        for (id, at) in crystals {
            world.spawn((
                Name::new("debug slot crystal"),
                PlacedItem {
                    id,
                    tagged_id: id,
                    condition: 0,
                },
                Transform::from_translation(at),
                GlobalTransform::from_translation(at),
            ));
        }
    });
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
fn release_from_slots(
    mut items: Option<ResMut<crate::item_cursor::ItemCursor>>,
    mut frame: ResMut<exa_physics::sim_app::FrameCommands>,
    devices: Query<Entity, With<TeleportDevice>>,
    children: Query<&Children>,
    names: Query<&Name>,
    transforms: Query<&GlobalTransform>,
    mut released: MessageWriter<SlotReleased>,
) {
    let Some(items) = items.as_mut() else {
        return;
    };
    if items.released_cm.is_empty() {
        return;
    }
    // The points the hand queued, taken here and dropped from the frame's
    // own queue (which may have grown since) after the step.
    let n = items.released_cm.len();
    let points = frame.write_resource(items, move |items| {
        items
            .released_cm
            .drain(..n.min(items.released_cm.len()))
            .collect::<Vec<_>>()
    });
    for device in &devices {
        let Some(slot) = find_slot_entity(device, &children, &names) else {
            continue;
        };
        let Ok(at) = transforms.get(slot) else {
            continue;
        };
        let slot_cm = at.translation() * 100.0;
        if points
            .iter()
            .any(|point| teleporters::released_from_slot(slot_cm, *point))
        {
            released.write(SlotReleased { device });
        }
    }
}

fn power_down_devices(
    mut released: MessageReader<SlotReleased>,
    devices: Query<(&TeleportDevice, &GlobalTransform)>,
    mut latch: ResMut<Latch>,
    mut flow: Option<ResMut<crate::flow::Flow>>,
    mut frame: ResMut<exa_physics::sim_app::FrameCommands>,
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
        // The one reconstruction word: the story session no longer rebuilds.
        if let Some(flow) = flow.as_mut() {
            frame.write_resource(flow, |flow| flow.story.reconstruction = 0);
        }
    }
}

/// Strip a reconstructed player's live body: no hand or worn data, the
/// underwear rule, and yellow and red health at the record's 0x7FFF share
/// of the maximum. The original strips the character record (hands and worn
/// list cleared, one empty action, health and stamina at 0x7FFF), runs the
/// underwear rule on the cleared list, and leaves a dressed copy of the
/// record as the corpse; Exarust has no record copy, so the corpse is left
/// to the record owner. The emptied outfit and hand refit the live body
/// (`exa_character::refit`): its garments and weapon come off.
pub fn strip_body(commands: &mut Commands, entity: Entity, fighter: &mut exa_combat::Fighter) {
    let share = teleporters::stripped_share();
    fighter.health.yellow = fighter.health.max * share;
    fighter.health.red = fighter.health.max * share;
    commands
        .entity(entity)
        .remove::<Wielded>()
        .insert((Outfit::default(), WearsUnderwear));
}

/// The session end: the player's red health, the mode and the latch decide
/// it. The first reconstruction after a scan strips the player: no hand or
/// worn data, the underwear rule, and yellow and red health at the record's
/// 0x7FFF share of the maximum. (The original also leaves the dressed
/// copy of the record behind as the corpse; Exarust has no record copy
/// yet.)
fn end_sessions(
    mut ends: MessageReader<EndSession>,
    mut latch: ResMut<Latch>,
    mut ended: MessageWriter<SessionEnded>,
    mut commands: Commands,
    identities: Query<&RecordIdentity>,
    mut fighters: Query<&mut exa_combat::Fighter>,
) {
    for end in ends.read() {
        let Ok(mut fighter) = fighters.get_mut(end.user) else {
            continue;
        };
        let identity = identities.get(end.user).map(|i| i.0).unwrap_or(0);
        let outcome = teleporters::session_end(fighter.health.red, end.story, identity, &mut latch);
        if outcome == (SessionEnd::Reconstructed { stripped: true }) {
            strip_body(&mut commands, end.user, &mut fighter);
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
        let mut scan = scan_for_device(teleporters::scan_timing(1_000).effect_at_ms, [0.5, 0.25]);
        assert!(scan.looping);
        // The device starts the scan's colour at three times its own.
        assert_eq!(scan.colour_scale, teleporters::SCAN_SCALE);
        assert_eq!(scan.stamp, 2_600);
        assert_eq!(scan.tex_pos, [0.5, 0.25]);
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

    #[test]
    fn a_let_go_on_the_slot_powers_the_device_down() {
        let mut app = App::new();
        app.init_resource::<crate::item_cursor::ItemCursor>()
            .init_resource::<exa_physics::sim_app::FrameCommands>()
            .init_resource::<Latch>()
            .init_resource::<crate::flow::Flow>()
            .add_message::<SlotReleased>()
            .add_message::<DeviceSound>()
            .add_systems(Update, (release_from_slots, power_down_devices).chain());
        let slot = app
            .world_mut()
            .spawn((
                Name::new("[slot"),
                Transform::from_xyz(1.0, 0.0, 0.0),
                GlobalTransform::from(Transform::from_xyz(1.0, 0.0, 0.0)),
            ))
            .id();
        let device = app
            .world_mut()
            .spawn((
                TeleportDevice {
                    kind: operative::RES_DEVICE,
                    operative: operative(operative::RES_DEVICE),
                },
                Transform::default(),
                GlobalTransform::default(),
            ))
            .id();
        app.world_mut().entity_mut(slot).insert(ChildOf(device));
        app.world_mut().resource_mut::<Latch>().begin(1);
        app.world_mut()
            .resource_mut::<crate::item_cursor::ItemCursor>()
            .released_cm
            .push(Vec3::new(100.0, 0.0, 0.0));
        app.update();
        assert_eq!(
            app.world().resource::<Latch>().phase,
            exa_world::teleporters::LatchPhase::Idle
        );
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
            .init_resource::<exa_physics::sim_app::FrameCommands>()
            .init_resource::<Possession>()
            .init_resource::<Latch>()
            .init_resource::<crate::effects::scan::ScanBuild>()
            .init_resource::<crate::flow::Flow>()
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
                tagged_id: exa_formats::objdb::LOCAL | crystal,
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

    /// The answer the click reads: a reconstruction device with its
    /// crystal on the slot says yes (a scan), yes again while the scan is
    /// on the player (nothing more starts), and no out of reach, without a
    /// crystal or while the player possesses another body.
    #[test]
    fn the_click_reads_the_device_use_answer() {
        use bevy::ecs::system::RunSystemOnce;
        let decide = |app: &mut App, device: Entity, user: Entity| {
            app.world_mut()
                .run_system_once(move |reads: DeviceReads| reads.decide(device, user))
                .unwrap()
        };
        let mut app = app();
        let (res, user) = device(
            &mut app,
            operative::RES_DEVICE,
            teleporters::LARGE_POWER_CRYSTAL,
        );
        assert_eq!(decide(&mut app, res, user), DeviceDecision::Scan);
        assert!(decide(&mut app, res, user).answer());
        app.world_mut()
            .write_message(UseDevice { device: res, user });
        at(&mut app, 100);
        assert_eq!(scans_on(&mut app, user), 1);
        assert_eq!(decide(&mut app, res, user), DeviceDecision::Nothing);
        assert!(decide(&mut app, res, user).answer());
        app.world_mut().resource_mut::<Possession>().active = true;
        assert!(!decide(&mut app, res, user).answer());
        app.world_mut().resource_mut::<Possession>().active = false;
        app.world_mut()
            .entity_mut(user)
            .insert(GlobalTransform::from_translation(Vec3::new(5.0, 0.0, 0.0)));
        assert_eq!(decide(&mut app, res, user), DeviceDecision::Refused);
        // A teleporter whose slot holds the reconstruction crystal refuses.
        let mut app = self::app();
        let (tele, player) = device(
            &mut app,
            operative::TELEPORTER,
            teleporters::LARGE_POWER_CRYSTAL,
        );
        assert!(!decide(&mut app, tele, player).answer());
        // Something that is no device refuses.
        assert!(!decide(&mut app, player, player).answer());
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

    /// The aura and the two draws the scan entity carries.
    fn scan_aura(app: &mut App, user: Entity) -> (scan::ScanAura, [f32; 2]) {
        let world = app.world_mut();
        let mut query = world.query::<(&ScanEffect, &scan::ScanAura, &ChildOf)>();
        let (scan, aura, _) = query
            .iter(world)
            .find(|(_, _, parent)| parent.parent() == user)
            .expect("a scan on the player");
        (aura.clone(), scan.0.tex_pos)
    }

    #[test]
    fn a_reconstruction_device_scans_glows_and_latches() {
        let mut app = app();
        let (device, user) = device(&mut app, operative::RES_DEVICE, 0x1F4);
        app.world_mut().write_message(UseDevice { device, user });
        at(&mut app, 1_000);
        assert_eq!(scans_on(&mut app, user), 1);
        // The scan's node wears the device's aura: the library's material,
        // its 184-byte grey block, the 0x9000 state bits, one sub-object and
        // the 0x20 material flag with the shared noise picture.
        let (aura, tex_pos) = scan_aura(&mut app, user);
        assert_eq!(aura.0.material, Some(scan::AURA_MATERIAL));
        assert_eq!(aura.0.block, scan::AURA_BLOCK);
        assert_eq!(aura.0.state, scan::AURA_STATE);
        assert_eq!(aura.0.flags, 0x20);
        assert_eq!(aura.0.subobjects, 1);
        assert_eq!(aura.0.noise, Some("_ForceNoise"));
        // The node's two creation draws are the stream's first two.
        assert_eq!(tex_pos, scan::ScanBuild::default().draws());
        let latch = *app.world().resource::<Latch>();
        assert_eq!((latch.phase, latch.remembered), (LatchPhase::Charging, 7));
        // The same word the story session reads: a death can now rebuild.
        assert_eq!(
            app.world()
                .resource::<crate::flow::Flow>()
                .story
                .reconstruction,
            crate::flow::REBUILDABLE
        );
        // The identity beside it is the scanned player's record identity.
        assert_eq!(
            app.world().resource::<crate::flow::Flow>().story.checkpoint,
            7
        );
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
        // The crystal taken off powers the device down: the latch and the
        // story session's reconstruction word clear.
        app.world_mut().write_message(SlotReleased { device });
        at(&mut app, 7_813);
        assert_eq!(app.world().resource::<Latch>().phase, LatchPhase::Idle);
        assert_eq!(
            app.world()
                .resource::<crate::flow::Flow>()
                .story
                .reconstruction,
            0
        );
    }

    /// A teleporter whose screen is already showing opens nothing more: the
    /// original finds the destination form by its id and answers "already
    /// open". The crystal stays on the slot.
    #[test]
    fn a_teleporter_opens_nothing_while_its_screen_shows() {
        let mut app = app();
        let (device, user) = device(&mut app, operative::TELEPORTER, 0x1AF);
        let state = crate::destination::PortalScreenState {
            open: Some(crate::destination::OpenScreen {
                screen: exa_ui::selection::DestinationScreen::open_with_markers(
                    vec![7, 10, 12, 13],
                    Vec::new(),
                    4,
                    false,
                    0,
                ),
                device,
                user,
                item: Entity::PLACEHOLDER,
                width: 0,
                height: 0,
                unit: 4,
                random: crate::arena::Mt19937::new(0),
            }),
        };
        app.insert_resource(state);
        app.world_mut().write_message(UseDevice { device, user });
        at(&mut app, 0);
        assert!(
            app.world()
                .resource::<Messages<OpenPortalScreen>>()
                .is_empty()
        );
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

    /// The strip the flow applies when a scanned story player dies: the
    /// hands and worn data go, the underwear rule stays, and yellow and red
    /// health are the record's 0x7FFF share of the maximum.
    #[test]
    fn a_stripped_body_loses_its_gear_and_keeps_just_under_half_health() {
        use bevy::ecs::system::RunSystemOnce;
        let mut app = App::new();
        let user = app
            .world_mut()
            .spawn((
                Fighter::of_stats(
                    None,
                    CharacterStats {
                        max_health: 200.0,
                        yellow: 200.0,
                        red: 0.0,
                        ..Default::default()
                    },
                ),
                Outfit::default(),
                Wielded::model("ub05ssworda"),
            ))
            .id();
        app.world_mut()
            .run_system_once(
                move |mut commands: Commands, mut fighters: Query<(Entity, &mut Fighter)>| {
                    for (entity, mut fighter) in &mut fighters {
                        strip_body(&mut commands, entity, &mut fighter);
                    }
                },
            )
            .expect("the strip runs");
        let health = app.world().get::<Fighter>(user).unwrap().health;
        // 200 × 0.4999924 = 99.9985.
        assert!((health.yellow - 99.998_47).abs() < 1e-3, "{health:?}");
        assert!((health.red - 99.998_47).abs() < 1e-3, "{health:?}");
        assert!(app.world().get::<Wielded>(user).is_none());
        assert!(app.world().get::<WearsUnderwear>(user).is_some());
        assert!(app.world().get::<Outfit>(user).unwrap().0.is_empty());
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
