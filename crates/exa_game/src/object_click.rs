//! The world-object half of the player's click and double click, as the
//! controls' click hook and double-click hook run them on a placed object.
//!
//! The hand reads the click on its release and sends what the pick hit as
//! an [`ObjectClick`]. Characters stay with the hand; this module takes the
//! placed objects:
//!
//! - The single click (outside combat mode) on an object node clears the
//!   placement's first-touch bit, runs a device's use (a scripted device,
//!   class 2 kind 3, runs its use script with the player as its one actor
//!   and itself as the operative, then pulses and sounds; a teleporter or a
//!   reconstruction device asks [`crate::teleporters`] for its own use with
//!   a [`UseDevice`] for the player's body), and then shows the
//!   object's item card at once, asks for the delayed card, or has the
//!   narrator say the device's description
//!   ([`player_controls::click`]). A point on plain level geometry is
//!   looked up among the level's placed objects, as the original's click
//!   does: the first object whose bounding sphere holds the point and whose
//!   box lies within 0.1 cm² of it ([`player_controls::on_object`]); only a
//!   used device's narration follows from it.
//! - The double click (in reach; with Ctrl in combat mode) runs the
//!   object's own interaction ([`player_controls::double_click_object`]):
//!   a usable kind 0 object leaves the scene with its use sound (a
//!   consumable's doors open 2100 ms later through the deferred list; kind 5
//!   also opens the divination reading), a
//!   usable kind 1 item lifts the use cursor and stays in the scene
//!   (`crate::use_cursor`), and a container section opens the container's
//!   popup.
//!
//! Evidence: REA on the controls' click hook, double-click hook, object
//! interaction and the device use they call (`docs/UI.md`, player
//! controls).

use bevy::ecs::system::SystemParam;
use bevy::math::Affine3A;
use bevy::prelude::*;

use exa_items::catalogue::ItemCatalogue;
use exa_physics::ColliderNodes;
use exa_physics::sector::Bounds;
use exa_world::db_objects::PlaceholderNode;
use exa_world::{ObjectSections, PartBounds, PlacedItem};

use crate::div_card_screen::OpenDivination;
use crate::flow::ReadNamedLevel;
use crate::gui_screens::{ContainerRequest, GameScreens};
use crate::hand::Hit;
use crate::item_card::{CardView, WorldCard};
use crate::player_controls::{
    self, ClickOutcome, ClickTarget, DeviceClock, ObjectInteraction, PendingUse, PendingUses,
    TakeUse, USABLE_DEVICE,
};
use crate::teleporters::UseDevice;

/// Centimetres per metre: the original's rules are in cm.
const CM: f32 = 100.0;

/// A click on the world that did not land on a character: what the pick hit,
/// the picked point (m), the pointer (physical pixels) and whether it was
/// the double click's interaction (the hand has already checked its gates:
/// in reach, Ctrl in combat mode).
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct ObjectClick {
    pub hit: Hit,
    pub at: Vec3,
    pub pointer: IVec2,
    pub double: bool,
}

/// What the clicks read of a placed object: its id, its sections, its
/// frame, its nodes and its placeholder.
type PlacedParts = (
    Entity,
    &'static PlacedItem,
    Option<&'static mut ObjectSections>,
    &'static GlobalTransform,
    Option<&'static Children>,
    Option<&'static PlaceholderNode>,
);

/// The level's placed objects as the clicks read and change them.
#[derive(SystemParam)]
pub struct WorldObjects<'w, 's> {
    placed: Query<'w, 's, PlacedParts>,
    parents: Query<'w, 's, &'static ChildOf>,
    nodes: Query<'w, 's, (&'static PartBounds, &'static GlobalTransform)>,
    children: Query<'w, 's, &'static Children>,
}

/// How a click found its object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Found {
    /// The picked node belongs to the object.
    Node(Entity),
    /// The point on plain geometry lies on the object's box.
    OnGeometry(Entity),
}

/// A frame in centimetres: the node's world frame with its translation
/// scaled to cm.
fn frame_cm(transform: &GlobalTransform) -> Affine3A {
    let affine = transform.affine();
    Affine3A {
        matrix3: affine.matrix3,
        translation: affine.translation * CM,
    }
}

impl WorldObjects<'_, '_> {
    /// The placed object at or above `node`, as the original takes the hit
    /// node's root object.
    fn above(&self, node: Entity) -> Option<Entity> {
        let mut at = Some(node);
        let mut steps = 0;
        while let Some(entity) = at {
            if self.placed.contains(entity) {
                return Some(entity);
            }
            steps += 1;
            if steps > 256 {
                return None;
            }
            at = self.parents.get(entity).ok().map(ChildOf::parent);
        }
        None
    }

    /// The object node's box: a placeholder's 50 cm half extents about its
    /// origin, else the model's root node bounds (cm) and that node's frame
    /// (cm). [guess: the model's root node bounds stand for the object
    /// node's]
    pub fn bounds_of(&self, entity: Entity) -> Option<(Bounds, Affine3A)> {
        let (_, _, _, transform, children, placeholder) = self.placed.get(entity).ok()?;
        if let Some(placeholder) = placeholder {
            let half = Vec3::from_array(placeholder.extents);
            return Some((
                Bounds {
                    centre: Vec3::ZERO,
                    half,
                    radius: placeholder.radius,
                },
                frame_cm(transform),
            ));
        }
        children?.iter().find_map(|child| {
            let (bounds, transform) = self.nodes.get(child).ok()?;
            Some((
                Bounds {
                    centre: bounds.centre * CM,
                    half: bounds.half_extents * CM,
                    radius: bounds.radius * CM,
                },
                frame_cm(transform),
            ))
        })
    }

    /// The first placed object (in spawn order) a point (m) lies on.
    /// [guess: the original walks the hit node's scene list; spawn order
    /// stands in for its order]
    fn on_point(&self, at: Vec3) -> Option<Entity> {
        let point = at * CM;
        let mut objects: Vec<Entity> = self.placed.iter().map(|(e, ..)| e).collect();
        objects.sort_by_key(|e| e.index());
        objects.into_iter().find(|&e| {
            self.bounds_of(e).is_some_and(|(bounds, frame)| {
                let centre = frame.transform_point3(bounds.centre);
                player_controls::on_object(point, centre, bounds.radius, &bounds, &frame)
            })
        })
    }

    /// The object a click found: the picked node's own object, else the
    /// object the point lies on.
    pub fn resolve(&self, hit: Hit, at: Vec3) -> Option<Found> {
        let own = match hit {
            Hit::Prop { body, .. } => self.above(body),
            Hit::SceneNode { node } => self.above(node),
            Hit::Level => None,
            Hit::Character { .. } => return None,
        };
        match own {
            Some(e) => Some(Found::Node(e)),
            None => self.on_point(at).map(Found::OnGeometry),
        }
    }

    /// The entities at and under `node`, parents first (bounded).
    fn subtree(&self, node: Entity) -> Vec<Entity> {
        let mut found = vec![node];
        let mut k = 0;
        while k < found.len() && found.len() < 4096 {
            if let Ok(kids) = self.children.get(found[k]) {
                found.extend(kids.iter());
            }
            k += 1;
        }
        found
    }
}

/// What the clicks write.
#[derive(SystemParam)]
pub struct ClickOutputs<'w> {
    cards: MessageWriter<'w, WorldCard>,
    containers: MessageWriter<'w, ContainerRequest>,
    sounds: MessageWriter<'w, exa_audio::PointSounded>,
    uses: Option<ResMut<'w, PendingUses>>,
    devices: ResMut<'w, DeviceClock>,
    screens: Option<ResMut<'w, GameScreens>>,
    card: Option<ResMut<'w, CardView>>,
    /// The use cursor a hand-held world item lifts
    /// (the controls' use state word and its dispatcher).
    use_cursor: Option<ResMut<'w, crate::player_controls::UseCursor>>,
    /// Story discovery. A node's first touch awards the rank in its
    /// placement word.
    experience: Option<ResMut<'w, crate::experience_live::StoryExperience>>,
    /// A kind-5 take opens the divination reading.
    divination: MessageWriter<'w, OpenDivination>,
    /// A kind-7 take asks to load the level its header names.
    levels: MessageWriter<'w, ReadNamedLevel>,
    /// A scripted device's use pulses the object's glow.
    pulses: MessageWriter<'w, crate::glow_pulse::PulseGlow>,
    /// What a used device's script asked for.
    effects: MessageWriter<'w, exa_script::ScriptEffect>,
    /// A used device's `WitnessRole`, placed at the device.
    witness: MessageWriter<'w, crate::script_cast::WitnessGrant>,
    /// A teleporter's or reconstruction device's own use.
    device_uses: MessageWriter<'w, UseDevice>,
}

/// The dialogue side a narration goes through.
#[derive(SystemParam)]
pub struct Narrator<'w> {
    scripts: Option<Res<'w, exa_script::Scripts>>,
    conversations: Option<ResMut<'w, exa_script::Conversations>>,
}

impl Narrator<'_> {
    /// The narrator says `text` after `delay_ms`, as the original's
    /// narrator line request does.
    fn say(&mut self, text: &str, delay_ms: u32, now: u32) {
        let (Some(scripts), Some(conversations)) =
            (self.scripts.as_deref(), self.conversations.as_deref_mut())
        else {
            info!("object click: narrator line {text:?} (no dialogue loaded)");
            return;
        };
        let exa_script::Conversations {
            engine,
            cast,
            player,
            player2,
            world_base,
            locale,
            ..
        } = conversations;
        let mut scene = exa_script::dialogue::Scene {
            library: exa_script::dialogue::Library {
                roles: &scripts.roles,
                narrator: scripts.narrator.as_ref(),
            },
            cast,
            now,
            world_time: exa_script::topics::world_time(*world_base, now, 0),
            player: *player,
            player2: *player2,
            locale: *locale,
            effects: Vec::new(),
        };
        engine.narrate_text(&mut scene, text.as_bytes(), delay_ms);
    }

    /// A used device's script, as the object use runs it: the player is
    /// the one actor slot, no role runs, and the device is the operative.
    /// Returns the effects it asked for, in order (none without the
    /// dialogue side loaded).
    fn run_use_script(&mut self, script: &[u8], now: u32) -> Vec<exa_script::dialogue::Effect> {
        let (Some(scripts), Some(conversations)) =
            (self.scripts.as_deref(), self.conversations.as_deref_mut())
        else {
            info!("object click: use script not run (no dialogue loaded)");
            return Vec::new();
        };
        let player = conversations.player;
        let (result, effects) = conversations.with_scene(scripts, now, |engine, scene| {
            engine.use_script(scene, player, script)
        });
        info!(
            "object click: use script {:?} ran for the player {player:?}: result {result}, {} effect(s)",
            String::from_utf8_lossy(script),
            effects.len()
        );
        effects
    }
}

/// A device's use as the click runs it: the shared clock's rules; a
/// teleporter or reconstruction device runs its own use, which
/// [`crate::teleporters`] answers from a [`UseDevice`] request for the
/// player's body (the original's use measures the reach from the player and
/// scans the player, whoever clicked). That use runs on the next fixed step,
/// so the click reads its answer as no [Exarust: the original's answer only
/// decides an unnamed device's narration, and both devices are named, so
/// their delayed card follows either way].
fn use_device(
    out: &mut ClickOutputs,
    kind: u8,
    id: u32,
    now: u64,
    device: Entity,
    user: Option<Entity>,
) -> bool {
    let uses = &mut out.device_uses;
    player_controls::device_use(kind, id, now, &mut out.devices, || {
        match user {
            Some(user) => {
                info!(
                    "object click: device {id:#x} kind {kind:#x} runs its own use for the player {user:?}"
                );
                uses.write(UseDevice { device, user });
            }
            None => info!("object click: device {id:#x} kind {kind:#x}: no player to use it"),
        }
        false
    })
}

/// A scripted device's use (kind 3), once the use said yes: its script
/// runs first (the player the one actor, the device the operative), then
/// the object's glow pulses (fade 1200, hold 0) and the sound its usable
/// section names (at most 15 characters) plays at the object at 0.3. The
/// script's effects go out as [`exa_script::ScriptEffect`]s, but a
/// `WitnessRole` is placed at the device: the original grants the role
/// within 18 m of the operative's own position.
#[allow(clippy::too_many_arguments)]
fn scripted_device_use(
    out: &mut ClickOutputs,
    narrator: &mut Narrator,
    entity: Entity,
    id: u32,
    usable: &exa_formats::objdb::Operative,
    script: Option<&[u8]>,
    position: Vec3,
    now: u32,
) {
    if usable.kind != player_controls::DEVICE_SLOW_LINE {
        return;
    }
    if let Some(script) = script {
        for effect in narrator.run_use_script(script, now) {
            match effect {
                exa_script::dialogue::Effect::WitnessRole(role) => {
                    info!(
                        "object click: {id:#x}'s WitnessRole({role:#x}) from the operative at {position:?}"
                    );
                    out.witness.write(crate::script_cast::WitnessGrant {
                        role,
                        origin: position,
                    });
                }
                effect => {
                    out.effects.write(exa_script::ScriptEffect(effect));
                }
            }
        }
    }
    let (fade, hold) = exa_script::objects::USE_PULSE;
    out.pulses.write(crate::glow_pulse::PulseGlow {
        object: entity,
        fade,
        hold,
    });
    if let Some(name) = player_controls::use_sound_name(usable) {
        let name: String = name.chars().take(15).collect();
        info!("object click: scripted device sound {name:?} at 0.3");
        out.sounds.write(exa_audio::PointSounded {
            name,
            position,
            volume: exa_script::objects::USE_VOLUME,
            pitch: 1.0,
            flags: SCRIPTED_USE_SOUND_FLAGS,
        });
    }
}

/// The sound flags a scripted device's use plays with.
pub const SCRIPTED_USE_SOUND_FLAGS: u32 = 0x400;

/// The single and double clicks on placed objects.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn run_object_clicks(
    mut commands: Commands,
    mut clicks: MessageReader<ObjectClick>,
    mut objects: WorldObjects,
    catalogue: Option<Res<ItemCatalogue>>,
    time: Option<Res<Time<Virtual>>>,
    (mut physics, mut collider_nodes, mut fixed): (
        Option<ResMut<exa_physics::collision_share::FramePhysics>>,
        Option<ResMut<ColliderNodes>>,
        Option<ResMut<exa_physics::sim_app::FixedCommands>>,
    ),
    mut out: ClickOutputs,
    mut narrator: Narrator,
    players: Query<Entity, With<crate::player::PlayerBody>>,
) {
    let now = time.map_or(0, |t| t.elapsed().as_millis() as u64);
    let user = players.iter().next();
    for click in clicks.read() {
        let Some(found) = objects.resolve(click.hit, click.at) else {
            continue;
        };
        let (Found::Node(entity) | Found::OnGeometry(entity)) = found;
        let Ok((_, placed, sections, transform, ..)) = objects.placed.get_mut(entity) else {
            continue;
        };
        let id = placed.tagged_id;
        let facts = catalogue.as_deref().and_then(|c| c.card_facts(id));
        let named = facts.is_some_and(|f| f.name_id != 0);
        // [guess: a non-empty description stands for the model section's
        // description string id being set]
        let description = facts.map(|f| f.description.clone()).unwrap_or_default();
        let position = transform.translation();
        let mut sections = sections;
        if !click.double {
            match found {
                Found::OnGeometry(_) => {
                    // The point search: a device's use and its narration
                    // only.
                    let Some(usable) = sections.as_deref().and_then(|s| s.usable) else {
                        continue;
                    };
                    let used = usable.class == USABLE_DEVICE
                        && use_device(&mut out, usable.kind, id, now, entity, user);
                    if used {
                        let script = sections.as_deref().and_then(|s| s.use_script.clone());
                        scripted_device_use(
                            &mut out,
                            &mut narrator,
                            entity,
                            id,
                            &usable,
                            script.as_deref(),
                            position,
                            now as u32,
                        );
                    }
                    if used && !description.is_empty() {
                        let delay = if usable.kind == player_controls::DEVICE_SLOW_LINE {
                            player_controls::DEVICE_LINE_SLOW_MS
                        } else {
                            player_controls::DEVICE_LINE_MS
                        };
                        narrator.say(&description, delay, now as u32);
                    }
                }
                Found::Node(_) => {
                    let flags = match sections.as_deref_mut() {
                        Some(s) => {
                            let word = s.placement;
                            let (placement, first) = player_controls::first_touch(word);
                            if first {
                                s.placement = placement;
                                info!("object click: first touch of {id:#x}");
                                if let Some(xp) = out.experience.as_deref_mut() {
                                    let points =
                                        xp.state.discover_kill(&mut xp.player, &mut xp.party, word);
                                    if points != 0 {
                                        info!(
                                            "object click: first touch of {id:#x} earns {points}"
                                        );
                                    }
                                }
                            }
                            s.placement
                        }
                        // An object without a usable or container section
                        // keeps no placement word here: its condition byte
                        // stands in.
                        None => u32::from(placed.condition) << 16,
                    };
                    let target = sections
                        .as_deref()
                        .map_or(ClickTarget::plain(), ClickTarget::of_object)
                        .with_strings(named, !description.is_empty());
                    let used = match sections.as_deref().and_then(|s| s.usable) {
                        Some(usable) if usable.class == USABLE_DEVICE => {
                            let used = use_device(&mut out, usable.kind, id, now, entity, user);
                            if used {
                                let script = sections.as_deref().and_then(|s| s.use_script.clone());
                                scripted_device_use(
                                    &mut out,
                                    &mut narrator,
                                    entity,
                                    id,
                                    &usable,
                                    script.as_deref(),
                                    position,
                                    now as u32,
                                );
                            }
                            used
                        }
                        _ => false,
                    };
                    let outcome = player_controls::click(target.with_device_used(used));
                    debug!("object click {id:#x}: {outcome:?}");
                    match outcome {
                        ClickOutcome::CardNow => {
                            out.cards.write(WorldCard {
                                item: id,
                                flags,
                                delayed: false,
                            });
                        }
                        ClickOutcome::DelayedCard => {
                            out.cards.write(WorldCard {
                                item: id,
                                flags,
                                delayed: true,
                            });
                        }
                        ClickOutcome::Narrate { delay_ms } => {
                            narrator.say(&description, delay_ms, now as u32);
                        }
                        _ => {}
                    }
                }
            }
            continue;
        }
        // The double click. The original's pick always returns the object's
        // own node; Exarust's can return the level for an object whose mesh
        // has no node of its own, so the point search stands in for it here.
        // [Exarust stand-in]
        let Some(s) = sections.as_deref() else {
            continue;
        };
        let (use_of, opens) = player_controls::double_click_object(s, now as u32);
        info!("double click object {id:#x}: {use_of:?}, container {opens}");
        // A usable item taken in hand lifts the use cursor: the controls'
        // state word 0x100 and the shared cursor mode 0x40. The next mouse
        // press ends it and runs the use under the pointer
        // ([`crate::use_cursor`]). The object itself stays in the scene.
        if use_of == ObjectInteraction::UseInHand {
            let kind = s.usable.as_ref().map_or(1, |u| u.kind);
            // The item card's close runs first, as in the take.
            close_card(&mut out, now as u32);
            if let Some(cursor) = out.use_cursor.as_deref_mut() {
                // The scene object's id word, tag included (the original
                // stores the node's id): a door's key section names the
                // key that way. Slot and panel are zero.
                cursor.lift(placed.tagged_id, placed.tagged_id, 0, 0, kind);
                info!("double click object {id:#x}: the use cursor is up for it");
            }
            continue;
        }
        if use_of.takes_object() {
            let usable = s.usable;
            if let Some(name) = usable.as_ref().and_then(player_controls::use_sound_name)
                && let Some(sound) = player_controls::point_sound(&name, position)
            {
                out.sounds.write(exa_audio::PointSounded {
                    name: sound.name,
                    position: sound.at,
                    volume: sound.volume,
                    pitch: sound.pitch,
                    flags: 0,
                });
            }
            match use_of {
                ObjectInteraction::Consume { due_ms } => {
                    if let (Some(uses), Some(usable)) = (out.uses.as_deref_mut(), usable) {
                        uses.0.push(PendingUse {
                            due_ms,
                            target: usable.target,
                        });
                    }
                }
                ObjectInteraction::Open => {
                    close_card(&mut out, now as u32);
                    match player_controls::take_use(usable.map_or(0, |u| u.kind)) {
                        TakeUse::Nothing => {}
                        // Kind 5 closes the card (already done) and opens the
                        // reading with the zero argument the use passes.
                        TakeUse::Divination => {
                            out.divination.write(OpenDivination { seed: 0 });
                        }
                        // Kind 7 asks to load the level the header names.
                        TakeUse::NameHook => {
                            out.levels.write(ReadNamedLevel { id });
                        }
                    }
                }
                _ => {}
            }
            // The node leaves the scene, its colliders with it.
            let nodes = objects.subtree(entity);
            if let (Some(physics), Some(collider_nodes)) =
                (physics.as_deref_mut(), collider_nodes.as_deref_mut())
            {
                // The replica now, the step's world at the next step.
                let handles = collider_nodes.take_under(&nodes);
                exa_physics::collision_share::remove_colliders(
                    physics,
                    fixed.as_deref_mut(),
                    handles,
                );
            }
            commands.entity(entity).despawn();
            continue;
        }
        if opens {
            out.containers.write(ContainerRequest {
                object: entity,
                id,
                pointer: click.pointer,
            });
        }
    }
}

/// The item card's close the take runs first: a pending build is cancelled
/// and a shown card fades.
fn close_card(out: &mut ClickOutputs, now: u32) {
    if let Some(screens) = out.screens.as_deref_mut() {
        let _ = screens.card.close(now);
    }
    if let Some(card) = out.card.as_deref_mut().and_then(|c| c.shown.as_mut()) {
        card.fade.fade_out(now);
    }
}

/// One scripted object click (`exarust --object-click`), a debug tool: the
/// frame after the load it fires on, whether it is the double click's
/// interaction, whether it is sent as a hit on plain level geometry at
/// the object's box centre (which exercises the point search) rather than
/// on the object's node, and the placed id it aims at (the container
/// nearest the player without one).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScriptedObjectClick {
    pub frame: u32,
    pub double: bool,
    pub level: bool,
    pub target: Option<u32>,
}

impl ScriptedObjectClick {
    /// Reads `F[d][l][#id],...`: a frame, `d` for the double click, `l` for
    /// a level hit, and `#` with a hexadecimal placed id (its base id, the
    /// tier tag left off). `None` on anything else.
    pub fn parse_list(list: &str) -> Option<Vec<Self>> {
        list.split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|item| {
                let (item, target) = match item.split_once('#') {
                    Some((head, id)) => (
                        head,
                        Some(u32::from_str_radix(id.trim_start_matches("0x"), 16).ok()?),
                    ),
                    None => (item, None),
                };
                let digits = item.trim_end_matches(['d', 'l']);
                let flags = &item[digits.len()..];
                Some(Self {
                    frame: digits.parse().ok()?,
                    double: flags.contains('d'),
                    level: flags.contains('l'),
                    target,
                })
            })
            .collect()
    }
}

/// The scripted object clicks of a run.
#[derive(Resource, Debug, Clone, Default)]
pub struct ObjectClickScript(pub Vec<ScriptedObjectClick>);

/// Fires the scripted object clicks on the placed container nearest the
/// player (the hand's reach gate is skipped), counting frames from the end
/// of the load, and logs the chosen object.
pub fn scripted_object_clicks(
    script: Option<Res<ObjectClickScript>>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    mut frames: Local<u32>,
    players: Query<&GlobalTransform, With<crate::player::Player>>,
    objects: WorldObjects,
    mut out: MessageWriter<ObjectClick>,
) {
    let Some(script) = script.filter(|s| !s.0.is_empty()) else {
        return;
    };
    if loading.is_some() {
        return;
    }
    *frames += 1;
    let frame = *frames;
    let Some(click) = script.0.iter().find(|c| c.frame == frame) else {
        return;
    };
    let Some(feet) = players.iter().next().map(GlobalTransform::translation) else {
        info!("object click script: no player at frame {frame}");
        return;
    };
    let nearest = objects
        .placed
        .iter()
        .filter(|(_, placed, sections, ..)| match click.target {
            Some(id) => placed.id & exa_formats::objdb::ID_MASK == id,
            None => sections.as_ref().is_some_and(|s| s.container.is_some()),
        })
        .map(|(e, placed, _, transform, ..)| (e, placed.tagged_id, transform.translation()))
        .min_by(|a, b| {
            a.2.distance_squared(feet)
                .total_cmp(&b.2.distance_squared(feet))
        });
    let Some((entity, id, at)) = nearest else {
        info!(
            "object click script: no placed object for {:?}",
            click.target
        );
        return;
    };
    let centre = objects.bounds_of(entity).map_or(at, |(bounds, frame)| {
        frame.transform_point3(bounds.centre) / CM
    });
    let hit = if click.level {
        Hit::Level
    } else {
        Hit::SceneNode { node: entity }
    };
    info!(
        "object click script: frame {frame}, {} on object {id:#x} at {centre:?} ({:.0} cm away), {hit:?}",
        if click.double {
            "double click"
        } else {
            "click"
        },
        centre.distance(feet) * CM
    );
    out.write(ObjectClick {
        hit,
        at: centre,
        pointer: IVec2::ZERO,
        double: click.double,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sections(usable: Option<(u8, u8)>, container: bool, placement: u32) -> ObjectSections {
        let mut atlas = [0u8; 16];
        atlas[..4].copy_from_slice(b"Gulp");
        ObjectSections {
            placement,
            usable: usable.map(|(class, kind)| exa_formats::objdb::Operative {
                class,
                kind,
                flags: 0,
                value: 0,
                time: 0,
                target: 0x8000_0042,
                second: 0,
                atlas,
                destinations: [0; exa_formats::objdb::DESTINATIONS],
            }),
            use_script: None,
            container: container.then(|| exa_formats::containers::Container {
                width: 50,
                height: 30,
                capacity: 400,
                max_item: 60,
                children: Vec::new(),
            }),
        }
    }

    fn app() -> App {
        let mut app = App::new();
        app.add_message::<ObjectClick>()
            .add_message::<WorldCard>()
            .add_message::<ContainerRequest>()
            .add_message::<OpenDivination>()
            .add_message::<ReadNamedLevel>()
            .add_message::<exa_audio::PointSounded>()
            .add_message::<crate::glow_pulse::PulseGlow>()
            .add_message::<exa_script::ScriptEffect>()
            .add_message::<crate::script_cast::WitnessGrant>()
            .add_message::<UseDevice>()
            .init_resource::<PendingUses>()
            .init_resource::<DeviceClock>()
            .init_resource::<crate::player_controls::UseCursor>()
            .init_resource::<ColliderNodes>()
            .insert_resource(exa_physics::collision_share::FramePhysics::default())
            .add_systems(Update, run_object_clicks);
        app
    }

    fn placed(app: &mut App, id: u32, sections: Option<ObjectSections>, at: Vec3) -> Entity {
        let mut e = app.world_mut().spawn((
            PlacedItem {
                id,
                tagged_id: id,
                condition: 0,
            },
            GlobalTransform::from_translation(at),
            PlaceholderNode::default(),
        ));
        if let Some(sections) = sections {
            e.insert(sections);
        }
        e.id()
    }

    fn click(app: &mut App, hit: Hit, at: Vec3, double: bool) {
        app.world_mut().write_message(ObjectClick {
            hit,
            at,
            pointer: IVec2::new(30, 40),
            double,
        });
        app.update();
    }

    fn drain<M: Message>(app: &mut App) -> Vec<M> {
        app.world_mut()
            .resource_mut::<Messages<M>>()
            .drain()
            .collect()
    }

    /// The double click on a node under a chest opens its popup; the single
    /// click on an object without a usable section asks for its card at
    /// once with the placement word, one with usable class 0 for the
    /// delayed card, and the first touch bit is cleared.
    #[test]
    fn the_clicks_open_the_chest_and_ask_for_cards() {
        let mut app = app();
        let chest = placed(&mut app, 7, Some(sections(None, true, 0x8001)), Vec3::ZERO);
        let lid = app.world_mut().spawn(ChildOf(chest)).id();
        click(&mut app, Hit::SceneNode { node: lid }, Vec3::ZERO, true);
        let requests = drain::<ContainerRequest>(&mut app);
        assert_eq!(requests.len(), 1);
        assert_eq!((requests[0].object, requests[0].id), (chest, 7));
        assert_eq!(requests[0].pointer, IVec2::new(30, 40));
        click(
            &mut app,
            Hit::Prop { body: lid, part: 0 },
            Vec3::ZERO,
            false,
        );
        assert_eq!(
            drain::<WorldCard>(&mut app),
            vec![WorldCard {
                item: 7,
                flags: 1,
                delayed: false
            }]
        );
        assert_eq!(
            app.world().get::<ObjectSections>(chest).unwrap().placement,
            1
        );
        let key = placed(
            &mut app,
            8,
            Some(sections(Some((1, 0)), false, 0)),
            Vec3::X * 10.0,
        );
        click(&mut app, Hit::SceneNode { node: key }, Vec3::ZERO, false);
        assert_eq!(
            drain::<WorldCard>(&mut app),
            vec![WorldCard {
                item: 8,
                flags: 0,
                delayed: true
            }]
        );
    }

    /// The first touch of a node awards the rank in its placement word and
    /// clears the bit, so a second click earns nothing.
    #[test]
    fn a_first_touch_earns_the_nodes_rank() {
        let mut app = app();
        app.insert_resource(crate::experience_live::StoryExperience::default());
        let node = placed(
            &mut app,
            9,
            Some(sections(None, false, player_controls::FIRST_TOUCH | 0x1000)),
            Vec3::ZERO,
        );
        click(&mut app, Hit::SceneNode { node }, Vec3::ZERO, false);
        let xp = app
            .world()
            .resource::<crate::experience_live::StoryExperience>();
        assert_eq!(xp.state.pool, crate::experience::rarity_points(1) as u32);
        assert_eq!(
            app.world().get::<ObjectSections>(node).unwrap().placement,
            0x1000
        );
        click(&mut app, Hit::SceneNode { node }, Vec3::ZERO, false);
        let xp = app
            .world()
            .resource::<crate::experience_live::StoryExperience>();
        assert_eq!(xp.state.pool, crate::experience::rarity_points(1) as u32);
    }

    /// A point on the level inside a placeholder's 50 cm half box finds
    /// its chest for the double click; a point 60 cm off finds nothing, and
    /// the single click's point search shows no card.
    #[test]
    fn a_point_on_plain_geometry_finds_the_chest() {
        let mut app = app();
        let chest = placed(
            &mut app,
            9,
            Some(sections(None, true, 0)),
            Vec3::new(2.0, 0.0, 0.0),
        );
        click(&mut app, Hit::Level, Vec3::new(2.3, 0.4, 0.0), true);
        let requests = drain::<ContainerRequest>(&mut app);
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].object, chest);
        click(&mut app, Hit::Level, Vec3::new(2.0, 0.0, 0.6), true);
        assert!(drain::<ContainerRequest>(&mut app).is_empty());
        click(&mut app, Hit::Level, Vec3::new(2.3, 0.4, 0.0), false);
        assert!(drain::<WorldCard>(&mut app).is_empty());
    }

    /// The double click on an unmarked consumable takes it: its use sound
    /// plays at the node at 0.25 and pitch 1, its target joins the deferred
    /// list 2100 ms on, and the object leaves the scene.
    #[test]
    fn a_consumable_is_taken_at_once() {
        let mut app = app();
        let at = Vec3::new(1.0, 0.5, -1.0);
        let jar = placed(&mut app, 0x11, Some(sections(Some((0, 1)), false, 0)), at);
        click(&mut app, Hit::SceneNode { node: jar }, at, true);
        let sounds = drain::<exa_audio::PointSounded>(&mut app);
        assert_eq!(
            sounds,
            vec![exa_audio::PointSounded {
                name: "Gulp".into(),
                position: at,
                volume: 0.25,
                pitch: 1.0,
                flags: 0,
            }]
        );
        let uses = &app.world().resource::<PendingUses>().0.entries;
        assert_eq!(
            uses.as_slice(),
            &[PendingUse {
                due_ms: 2_100,
                target: 0x8000_0042
            }]
        );
        assert!(app.world().get_entity(jar).is_err());
    }

    /// A kind-5 take opens the divination reading with the zero argument
    /// the use passes, and the object still leaves the scene.
    #[test]
    fn a_divination_item_opens_the_reading() {
        let mut app = app();
        let deck = placed(
            &mut app,
            0x31,
            Some(sections(Some((0, 5)), false, 0)),
            Vec3::ZERO,
        );
        click(&mut app, Hit::SceneNode { node: deck }, Vec3::ZERO, true);
        assert_eq!(
            drain::<OpenDivination>(&mut app),
            vec![OpenDivination { seed: 0 }]
        );
        assert!(app.world().get_entity(deck).is_err());
    }

    /// A kind-7 take asks to load the level its header names, and the object
    /// still leaves the scene.
    #[test]
    fn a_named_level_item_asks_to_load_it() {
        let mut app = app();
        let book = placed(
            &mut app,
            0x41,
            Some(sections(Some((0, 7)), false, 0)),
            Vec3::ZERO,
        );
        click(&mut app, Hit::SceneNode { node: book }, Vec3::ZERO, true);
        assert_eq!(
            drain::<ReadNamedLevel>(&mut app),
            vec![ReadNamedLevel { id: 0x41 }]
        );
        assert!(app.world().get_entity(book).is_err());
    }

    /// A usable kind 1 item taken in hand lifts the use cursor and stays in
    /// the scene; a marked one does not lift it, and a kind 0 consumable
    /// leaves without one.
    #[test]
    fn a_hand_held_item_lifts_the_use_cursor() {
        use crate::player_controls::{USE_CURSOR_MODE, USE_STATE, UseCursor};

        let mut app = app();
        let key = placed(
            &mut app,
            0x21,
            Some(sections(Some((1, 1)), false, 0)),
            Vec3::ZERO,
        );
        // A level-placed key carries its tier tag: the cursor holds the
        // tagged id a door's key section names.
        app.world_mut()
            .get_mut::<PlacedItem>(key)
            .unwrap()
            .tagged_id = 0x8000_0021;
        click(&mut app, Hit::SceneNode { node: key }, Vec3::ZERO, true);
        let cursor = *app.world().resource::<UseCursor>();
        assert_eq!(cursor.state, USE_STATE);
        assert_eq!(cursor.cursor_mode, USE_CURSOR_MODE);
        assert_eq!((cursor.item, cursor.kind), (0x8000_0021, 1));
        assert_eq!((cursor.slot, cursor.panel), (0, 0));
        assert!(cursor.up());
        assert!(app.world().get_entity(key).is_ok());

        // A used mark on the item keeps the interaction from taking it.
        let marked = placed(
            &mut app,
            0x22,
            Some(sections(Some((1, 1)), false, player_controls::USED_MARK)),
            Vec3::X * 10.0,
        );
        click(&mut app, Hit::SceneNode { node: marked }, Vec3::ZERO, true);
        assert_eq!(app.world().resource::<UseCursor>().state, USE_STATE);
        assert!(app.world().get_entity(marked).is_ok());

        // The consumable of kind 0 is taken and leaves no cursor.
        let jar = placed(
            &mut app,
            0x23,
            Some(sections(Some((0, 1)), false, 0)),
            Vec3::X * 20.0,
        );
        click(&mut app, Hit::SceneNode { node: jar }, Vec3::ZERO, true);
        assert_eq!(app.world().resource::<UseCursor>().state, USE_STATE);
    }

    /// The script list reads frames with `d` (double) and `l` (level).
    #[test]
    fn the_object_click_script_reads_its_list() {
        let list = ScriptedObjectClick::parse_list("100, 110d,120l,130ld").unwrap();
        let read: Vec<(u32, bool, bool)> =
            list.iter().map(|c| (c.frame, c.double, c.level)).collect();
        assert_eq!(
            read,
            vec![
                (100, false, false),
                (110, true, false),
                (120, false, true),
                (130, true, true)
            ]
        );
        assert_eq!(ScriptedObjectClick::parse_list("x1"), None);
        assert_eq!(ScriptedObjectClick::parse_list("d"), None);
        // `#` aims at a placed id, in hexadecimal.
        let aimed = ScriptedObjectClick::parse_list("200#587,210l#0x24c").unwrap();
        assert_eq!(aimed[0].target, Some(0x587));
        assert_eq!((aimed[1].frame, aimed[1].level), (210, true));
        assert_eq!(aimed[1].target, Some(0x24c));
        assert_eq!(ScriptedObjectClick::parse_list("1#zz"), None);
    }

    /// A scripted device's use (class 2, kind 3) pulses the object and
    /// plays its section's sound at 0.3.
    #[test]
    fn a_scripted_device_pulses_its_glow_and_plays_its_sound() {
        let mut app = app();
        // The kind's 2400 ms clock starts at zero: the first use waits.
        let mut time = Time::<Virtual>::default();
        time.advance_by(std::time::Duration::from_millis(3000));
        app.insert_resource(time);
        let device = placed(
            &mut app,
            0x587,
            Some(sections(Some((USABLE_DEVICE, 3)), false, 0)),
            Vec3::ZERO,
        );
        app.world_mut().write_message(ObjectClick {
            hit: Hit::SceneNode { node: device },
            at: Vec3::ZERO,
            pointer: IVec2::ZERO,
            double: false,
        });
        app.update();
        let pulses: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<crate::glow_pulse::PulseGlow>>()
            .drain()
            .collect();
        assert_eq!(
            pulses,
            [crate::glow_pulse::PulseGlow {
                object: device,
                fade: 1200,
                hold: 0
            }]
        );
        let sounds: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<exa_audio::PointSounded>>()
            .drain()
            .collect();
        assert_eq!(sounds.len(), 1);
        assert_eq!(sounds[0].name, "Gulp");
        assert_eq!((sounds[0].volume, sounds[0].flags), (0.3, 0x400));
    }

    /// A teleporter's and a reconstruction device's single click runs
    /// their own use: a `UseDevice` for the player's body, whoever clicked;
    /// without a player nothing is asked. The click reads no answer, so the
    /// device's card or narration does not depend on it.
    #[test]
    fn a_teleporter_click_asks_for_its_own_use() {
        use exa_formats::objdb::operative::{RES_DEVICE, TELEPORTER};
        for kind in [TELEPORTER, RES_DEVICE] {
            let mut app = app();
            let device = placed(
                &mut app,
                0x555,
                Some(sections(Some((USABLE_DEVICE, kind)), false, 0)),
                Vec3::ZERO,
            );
            click(&mut app, Hit::SceneNode { node: device }, Vec3::ZERO, false);
            assert!(drain::<UseDevice>(&mut app).is_empty());
            let player = app
                .world_mut()
                .spawn(crate::player::PlayerBody::new(0.0))
                .id();
            click(&mut app, Hit::SceneNode { node: device }, Vec3::ZERO, false);
            assert_eq!(
                drain::<UseDevice>(&mut app),
                [UseDevice {
                    device,
                    user: player
                }]
            );
        }
    }

    /// A scripted device's use runs its use script with the player as its
    /// one actor: a `WitnessRole(Operative, …)` is granted from the device's
    /// own place, the script's other effects go out as they are.
    #[test]
    fn a_scripted_device_runs_its_use_script_as_the_operative() {
        let mut app = app();
        let mut time = Time::<Virtual>::default();
        time.advance_by(std::time::Duration::from_millis(3000));
        app.insert_resource(time);
        app.insert_resource(exa_script::Scripts::default());
        app.insert_resource(exa_script::Conversations {
            player: Some(7),
            ..Default::default()
        });
        let mut s = sections(Some((USABLE_DEVICE, 3)), false, 0);
        s.use_script = Some(b"WitnessRole(Operative, $E1D1);\r\nGameEvent(A1);".to_vec());
        let at = Vec3::new(160.5, 0.58, -6.0);
        let device = placed(&mut app, 0x587, Some(s), at);
        app.world_mut().write_message(ObjectClick {
            hit: Hit::SceneNode { node: device },
            at,
            pointer: IVec2::ZERO,
            double: false,
        });
        app.update();
        let grants = drain::<crate::script_cast::WitnessGrant>(&mut app);
        assert_eq!(grants.len(), 1);
        assert_eq!(grants[0].role, 0xE1D1);
        assert!((grants[0].origin - at).length() < 1e-4);
        let effects = drain::<exa_script::ScriptEffect>(&mut app);
        assert_eq!(
            effects,
            [exa_script::ScriptEffect(
                exa_script::dialogue::Effect::GameEvent(0xA1)
            )]
        );
    }
}
