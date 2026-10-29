//! The world-object half of the player's click and double click, as the
//! controls' click hook and double-click hook run them on a placed object.
//!
//! The hand reads the click on its release and sends what the pick hit as
//! an [`ObjectClick`]. Characters stay with the hand; this module takes the
//! placed objects:
//!
//! - The single click (outside combat mode) on an object node clears the
//!   placement's first-touch bit, runs a device's use, and then shows the
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
//!   consumable's doors open 2100 ms later through the deferred list), and
//!   a container section opens the container's popup.
//!
//! Evidence: REA on the controls' click hook, double-click hook, object
//! interaction and the device use they call (`docs/UI.md`, player
//! controls).

use bevy::ecs::system::SystemParam;
use bevy::math::Affine3A;
use bevy::prelude::*;

use exa_items::catalogue::ItemCatalogue;
use exa_physics::sector::Bounds;
use exa_physics::{ColliderNodes, Physics};
use exa_world::db_objects::PlaceholderNode;
use exa_world::{ObjectSections, PartBounds, PlacedItem};

use crate::gui_screens::{ContainerRequest, GameScreens};
use crate::hand::Hit;
use crate::item_card::{CardView, WorldCard};
use crate::player_controls::{
    self, ClickOutcome, ClickTarget, DeviceClock, ObjectInteraction, PendingUse, PendingUses,
    TakeUse, USABLE_DEVICE,
};

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
}

/// A device's use as the click runs it: the shared clock's rules, the
/// teleporter and reconstruction device answering no here (their screens
/// are opened by Exarust's own device path; [Exarust stand-in]).
fn use_device(devices: &mut DeviceClock, kind: u8, id: u32, now: u64) -> bool {
    player_controls::device_use(kind, id, now, devices, || {
        info!("object click: device {id:#x} kind {kind:#x} answers through its own use");
        false
    })
}

/// The single and double clicks on placed objects.
#[allow(clippy::too_many_arguments)]
pub fn run_object_clicks(
    mut commands: Commands,
    mut clicks: MessageReader<ObjectClick>,
    mut objects: WorldObjects,
    catalogue: Option<Res<ItemCatalogue>>,
    time: Option<Res<Time<Virtual>>>,
    (mut physics, mut collider_nodes): (Option<ResMut<Physics>>, Option<ResMut<ColliderNodes>>),
    mut out: ClickOutputs,
    mut narrator: Narrator,
) {
    let now = time.map_or(0, |t| t.elapsed().as_millis() as u64);
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
                    if usable.class == USABLE_DEVICE
                        && use_device(&mut out.devices, usable.kind, id, now)
                        && !description.is_empty()
                    {
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
                            let (placement, first) = player_controls::first_touch(s.placement);
                            if first {
                                s.placement = placement;
                                info!("object click: first touch of {id:#x}");
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
                            use_device(&mut out.devices, usable.kind, id, now)
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
                        use_ => info!("object take {id:#x}: the record's use {use_:?}"),
                    }
                }
                _ => {}
            }
            // The node leaves the scene, its colliders with it.
            let nodes = objects.subtree(entity);
            if let (Some(physics), Some(collider_nodes)) =
                (physics.as_deref_mut(), collider_nodes.as_deref_mut())
            {
                for handle in collider_nodes.take_under(&nodes) {
                    physics.0.remove_collider(handle);
                }
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
/// interaction, and whether it is sent as a hit on plain level geometry at
/// the object's box centre (which exercises the point search) rather than
/// on the object's node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScriptedObjectClick {
    pub frame: u32,
    pub double: bool,
    pub level: bool,
}

impl ScriptedObjectClick {
    /// Reads `F[d][l],...`: a frame, `d` for the double click, `l` for a
    /// level hit. `None` on anything else.
    pub fn parse_list(list: &str) -> Option<Vec<Self>> {
        list.split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|item| {
                let digits = item.trim_end_matches(['d', 'l']);
                let flags = &item[digits.len()..];
                Some(Self {
                    frame: digits.parse().ok()?,
                    double: flags.contains('d'),
                    level: flags.contains('l'),
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
        .filter(|(_, _, sections, ..)| sections.as_ref().is_some_and(|s| s.container.is_some()))
        .map(|(e, placed, _, transform, ..)| (e, placed.tagged_id, transform.translation()))
        .min_by(|a, b| {
            a.2.distance_squared(feet)
                .total_cmp(&b.2.distance_squared(feet))
        });
    let Some((entity, id, at)) = nearest else {
        info!("object click script: no placed container");
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
        "object click script: frame {frame}, {} on container {id:#x} at {centre:?} ({:.0} cm away), {hit:?}",
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
            .add_message::<exa_audio::PointSounded>()
            .init_resource::<PendingUses>()
            .init_resource::<DeviceClock>()
            .init_resource::<ColliderNodes>()
            .insert_resource(Physics::default())
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
    }
}
