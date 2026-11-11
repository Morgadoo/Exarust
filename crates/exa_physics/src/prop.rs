//! Props and items as bodies in the solver: a model node's rigid body (and
//! its articulated children) becomes a [`Ragdoll`] built by
//! [`Ragdoll::rigid`], registered asleep where the level placed it and
//! woken by a blow, a drag or a contact with another body. Its meshes stay
//! in the physics world as stand-ins moved with it, for the rays and
//! sweeps; bodies (characters' parts, corpses, other props) skip them and
//! meet the body itself through their pairs (`crate::body::Pairs`), and
//! the body meets the level. A body that falls below
//! [`crate::body::FALL_OUT_HEIGHT`] is removed with its node, as the
//! original removes it.
//!
//! The level loader marks each rigid node ([`BodyNode`]) and gives the
//! root node what registration needs ([`PendingProp`]); registration places
//! the particles at the nodes' world transforms once they are known.

use std::collections::HashMap;

use bevy::math::Affine3A;
use bevy::prelude::*;
use rapier3d::prelude::{ColliderHandle, Pose};

use crate::Physics;
use crate::body::{BodySystems, Ragdoll, RigidPart};

/// `user_data` of the colliders that stand in for a rigid body's meshes
/// ([`crate::contact::Level::movable`]).
pub const STAND_IN: u128 = 0x0053_5441_4E44_494E;

/// Registers props' bodies, moves their stand-ins and removes the ones
/// that fall out of the world. Added by [`crate::PhysicsPlugin`].
pub struct PropPlugin;

impl Plugin for PropPlugin {
    fn build(&self, app: &mut App) {
        crate::sim_app::add_frame_commands(app);
        app.init_resource::<StandIns>()
            .init_resource::<crate::collision_share::CollisionChanges>()
            .add_systems(
                PostUpdate,
                register_props.after(TransformSystems::Propagate),
            )
            .add_systems(
                FixedUpdate,
                (move_stand_ins, remove_fallen_props)
                    .chain()
                    .after(BodySystems),
            );
        app.add_message::<crate::mechanism::MechanismSignal>();
        crate::body::door_body::plugin(app);
        crate::body::lever_body::plugin(app);
        crate::body::plate_body::plugin(app);
        crate::body::slide_body::plugin(app);
    }
}

/// On a rigid body's root node: the body is a prop's (or an item's).
/// Holds each part's transform where it was registered, which its
/// stand-ins were built at.
#[derive(Component, Debug, Clone)]
pub struct PropBody {
    placed: Vec<Affine3A>,
}

impl PropBody {
    /// Each part's transform where the body was registered.
    pub fn placed(&self) -> &[Affine3A] {
        &self.placed
    }
}

/// On a node a rigid body moves: the root node holding its body and its
/// part's index. Its meshes (and those under it) are the body's stand-ins
/// in the physics world, not static level.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BodyNode {
    pub body: Entity,
    pub part: usize,
}

/// On a rigid body's root node until it is registered: its parts, parents
/// first. Each part's `world` is set at registration from its node's
/// world transform.
#[derive(Component, Debug, Clone)]
pub struct PendingProp(pub Vec<RigidPart>);

/// A stored scrape or roll sound slot, in its body part's frame (metres):
/// the point whose motion drives the loop, and the volume and pitch it
/// asks for per unit of speed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodyScrape {
    pub part: usize,
    pub point: Vec3,
    pub volume: f32,
    pub pitch: f32,
}

/// On a rigid body's root node: the stored sound slots to attach when the
/// body is registered. The original keeps one source per slot and drops
/// the slots whose sound the audio environment cannot find; the level
/// loader keeps the named ones and this component carries them to the body.
#[derive(Component, Debug, Clone, Default)]
pub struct PendingScrapes(pub Vec<BodyScrape>);

/// On a rigid body's root node: the names of the sound sets its parts and
/// sources play, which the audio side resolves (the original resolves them
/// when the body is set up and registered).
///
/// `impact` holds each part's collision sound set (its node's first mesh
/// material's, [`crate::body::impact_sound`]), none for a part without a
/// material; `scrapes` the sound of each scrape source, in the order the
/// sources were added ([`crate::body::Ragdoll::scrapes`]).
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
pub struct BodySounds {
    pub impact: Vec<Option<String>>,
    pub scrapes: Vec<String>,
}

/// The stand-in colliders of each prop body (by its root node) and the
/// part each follows.
#[derive(Resource, Debug, Default, Clone)]
pub struct StandIns(pub HashMap<Entity, Vec<(ColliderHandle, usize)>>);

impl StandIns {
    /// The body (its root node) and the part a stand-in collider follows.
    pub fn owner_of(&self, handle: ColliderHandle) -> Option<(Entity, usize)> {
        self.0.iter().find_map(|(&body, list)| {
            list.iter()
                .find(|&&(h, _)| h == handle)
                .map(|&(_, part)| (body, part))
        })
    }
}

/// Whether a held item's body may be built: its holder's body exists. An
/// item equipped before its holder's body was built (the load builds a body
/// once it is dressed) waits for it, as the original attaches an item to a
/// character already built: until then the hand's node is not where the
/// body will put it, and the item's body, built there, would be tied to the
/// hand at that offset. The frame the holder's body is built its parts are
/// drawn onto the nodes before this runs, so the item is built at its grip
/// under the hand. A grip with no hand or arm to find its holder by does
/// not wait.
fn holder_built(grip: &crate::grip::HeldGrip, bodies: &Query<&Ragdoll>) -> bool {
    if grip.holder_key != Entity::PLACEHOLDER {
        return true;
    }
    let Some(probe) = grip.hands.iter().chain(grip.arms.iter()).flatten().next() else {
        return true;
    };
    bodies
        .iter()
        .any(|b| b.parts.iter().any(|p| p.entity == *probe))
}

/// Builds the bodies of props spawned this frame, at their nodes' world
/// transforms, asleep. A held item's body waits for its holder's
/// ([`holder_built`]).
fn register_props(
    mut commands: Commands,
    pending: Query<(Entity, &PendingProp)>,
    scrape_slots: Query<&PendingScrapes>,
    globals: Query<&GlobalTransform>,
    parents: Query<&ChildOf>,
    (held, bodies, scales): (
        Query<&crate::grip::HeldGrip>,
        Query<&Ragdoll>,
        Query<&crate::weapon_mass::WeaponMassScale>,
    ),
) {
    // In spawn order (the entity index), not the storage's: a body takes
    // its place in the sector's list when it is inserted
    // ([`Ragdoll::list_place`]), and the original adds a level's props in
    // its node order.
    let mut pending: Vec<_> = pending.iter().collect();
    pending.sort_unstable_by_key(|(root, _)| root.index());
    for (root, PendingProp(parts)) in pending {
        if let Ok(grip) = held.get(root)
            && !holder_built(grip, &bodies)
        {
            continue;
        }
        let mut parts = parts.clone();
        for part in &mut parts {
            if let Ok(g) = globals.get(part.entity) {
                part.world = g.affine();
            }
            // A root body's world links anchor in its parent node's frame.
            if part.parent.is_none()
                && let Ok(parent) = parents.get(part.entity)
                && let Ok(g) = globals.get(parent.parent())
            {
                part.parent_world = g.affine();
            }
        }
        let mut body = Ragdoll::rigid(&parts);
        // A held weapon's holding pass redistributes its table mass over
        // the registered parts (their link and contact mobilities; the
        // links its parts were registered with keep their shares) before
        // its grip links are built from them.
        let mass = scales.get(root).ok().map(|&scale| {
            let mass = crate::weapon_mass::WeaponMass::of_body(&body, scale);
            let registered: Vec<f32> = body.parts.iter().map(|p| p.inverse_mass).collect();
            mass.apply(&mut body);
            debug!(
                "{root:?}: weapon mass {:.3} over {} parts (size {:.3}): link mobilities {:?} \
                 (registered {registered:?}), held contact base {:?}",
                scale.total,
                mass.parts.len(),
                scale.size,
                mass.parts.iter().map(|p| p.inverse).collect::<Vec<_>>(),
                mass.held_contact_base()
            );
            mass
        });
        if let Ok(PendingScrapes(slots)) = scrape_slots.get(root) {
            for slot in slots {
                body.add_scrape_on(slot.part, slot.point, slot.volume, slot.pitch);
            }
        }
        let placed = body.parts.iter().map(|p| p.transform()).collect();
        let mut root_commands = commands.entity(root);
        root_commands
            .remove::<PendingProp>()
            .remove::<PendingScrapes>()
            .insert((body, PropBody { placed }));
        if let Some(mass) = mass {
            root_commands.insert(mass);
        }
    }
}

/// Moves the stand-ins of every prop body the last tick changed with it:
/// each by its part's motion since registration.
fn move_stand_ins(
    mut physics: ResMut<Physics>,
    mut changes: ResMut<crate::collision_share::CollisionChanges>,
    stand_ins: Res<StandIns>,
    bodies: Query<(Entity, &Ragdoll, &PropBody), Changed<Ragdoll>>,
) {
    for (root, body, prop) in &bodies {
        let Some(list) = stand_ins.0.get(&root) else {
            continue;
        };
        for &(handle, part) in list {
            let (Some(p), Some(placed)) = (body.parts.get(part), prop.placed.get(part)) else {
                continue;
            };
            let (_, rotation, translation) =
                (p.transform() * placed.inverse()).to_scale_rotation_translation();
            let pose = Pose::from_parts(translation, rotation);
            if let Some(c) = physics.0.colliders.get_mut(handle)
                && *c.position() != pose
            {
                c.set_position(pose);
                changes.moved(handle);
            }
        }
    }
}

/// Removes the prop bodies whose root fell out of the world, with their
/// nodes and stand-ins.
#[allow(clippy::type_complexity)]
fn remove_fallen_props(
    mut commands: Commands,
    mut physics: ResMut<Physics>,
    (mut stand_ins, mut changes, mut frame): (
        ResMut<StandIns>,
        ResMut<crate::collision_share::CollisionChanges>,
        ResMut<crate::sim_app::FrameCommands>,
    ),
    bodies: Query<(Entity, &Ragdoll), (With<PropBody>, Changed<Ragdoll>)>,
) {
    for (root, body) in &bodies {
        if !body.fell_out() {
            continue;
        }
        // The stand-in list is the frame's (its picks read it): the
        // removal is made there too.
        let gone = frame.write_resource(&mut stand_ins, move |s| s.0.remove(&root));
        for (handle, _) in gone.unwrap_or_default() {
            crate::collision_share::remove_collider(&mut physics, &mut changes, handle);
        }
        info!("a body fell out of the world: removed");
        commands.entity(root).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn crate_part(entity: Entity) -> RigidPart {
        RigidPart {
            entity,
            parent: None,
            world: Affine3A::IDENTITY,
            rest: [
                Vec3::new(-0.2, 0.0, -0.2),
                Vec3::new(0.2, 0.0, -0.2),
                Vec3::new(0.0, 0.0, 0.2),
                Vec3::new(0.0, 0.4, 0.0),
            ],
            point_masses: [4.0; 4],
            parent_world: Affine3A::IDENTITY,
            mobility: 1.0,
            spheres: vec![(Vec3::new(0.0, 0.2, 0.0), 0.2)],
            origin: Vec3::ZERO,
            links: Vec::new(),
            mesh: None,
            triangles: None,
        }
    }

    /// A pending prop is registered at its node's world transform, asleep,
    /// and remembers where.
    #[test]
    fn pending_props_register_where_their_nodes_are() {
        let mut app = App::new();
        app.add_systems(Update, register_props);
        let at = Vec3::new(2.0, 1.0, -3.0);
        let node = app
            .world_mut()
            .spawn(GlobalTransform::from_translation(at))
            .id();
        app.world_mut()
            .entity_mut(node)
            .insert(PendingProp(vec![crate_part(node)]));
        app.update();
        let e = app.world().entity(node);
        assert!(!e.contains::<PendingProp>());
        let body = e.get::<Ragdoll>().unwrap();
        assert!(body.asleep() && body.countdown() == 0);
        assert_eq!(body.parts[0].positions[3], at + Vec3::new(0.0, 0.4, 0.0));
        let placed = e.get::<PropBody>().unwrap().placed()[0];
        assert!(Vec3::from(placed.translation).distance(at) < 1e-5);
    }

    /// A held item equipped before its holder's body was built keeps its
    /// body pending until the holder's body exists, then is built where its
    /// node is that frame (under the hand the body has placed).
    #[test]
    fn a_held_items_body_waits_for_its_holders_body() {
        use crate::grip::{GripSpec, HeldGrip, HeldKind};
        let mut app = App::new();
        app.add_systems(Update, register_props);
        let hand = app.world_mut().spawn_empty().id();
        let node = app
            .world_mut()
            .spawn(GlobalTransform::from_translation(Vec3::new(0.0, 0.5, 0.0)))
            .id();
        let grip = HeldGrip {
            spec: GripSpec {
                hand: 0,
                links: Vec::new(),
                second_hand: None,
                arm: None,
                extra: None,
            },
            scale: 1.0,
            size: 1.0,
            second_hand_factor: 1.0,
            flip: false,
            class: 0x10,
            soft_weld: false,
            kind: HeldKind::Weapon,
            holder_key: Entity::PLACEHOLDER,
            hands: [Some(hand), None],
            arms: [None, None],
            links: None,
            released: false,
            head_extra: 0.0,
        };
        app.world_mut()
            .entity_mut(node)
            .insert((PendingProp(vec![crate_part(node)]), grip));
        app.update();
        assert!(app.world().entity(node).contains::<PendingProp>());
        // The holder's body is built and has put the hand (and the item's
        // node under it) where it holds it.
        let at = Vec3::new(1.0, 1.2, 0.3);
        let mut holder = crate_part(hand);
        holder.world = Affine3A::from_translation(at);
        app.world_mut().spawn(Ragdoll::rigid(&[holder]));
        app.world_mut()
            .entity_mut(node)
            .insert(GlobalTransform::from_translation(at));
        app.update();
        let e = app.world().entity(node);
        assert!(!e.contains::<PendingProp>());
        let placed = e.get::<PropBody>().unwrap().placed()[0];
        assert!(Vec3::from(placed.translation).distance(at) < 1e-5);
    }

    /// In the running solver (83 Hz, 30 frames a second): a registered prop
    /// sleeps through every tick where it was placed, in the air, and is
    /// not drawn; a blow wakes it to 256 and it falls; with nothing under
    /// it, it falls out of the world and is removed.
    #[test]
    fn a_prop_sleeps_until_a_blow_then_falls() {
        use crate::body::{BodyPlugin, Shove};
        use bevy::time::TimeUpdateStrategy;
        use std::time::Duration;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, BodyPlugin, PropPlugin))
            .insert_resource(Time::<Fixed>::from_hz(crate::STEP_HZ))
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
                1.0 / 30.0,
            )))
            .insert_resource(Physics::default());
        let at = Vec3::new(0.0, 2.0, 0.0);
        let node = app
            .world_mut()
            .spawn((
                Transform::from_translation(at),
                GlobalTransform::from_translation(at),
            ))
            .id();
        app.world_mut()
            .entity_mut(node)
            .insert(PendingProp(vec![crate_part(node)]));
        for _ in 0..60 {
            app.update();
        }
        let body = app.world().get::<Ragdoll>(node).unwrap();
        assert!(body.asleep());
        assert_eq!(body.parts[0].positions[3], at + Vec3::new(0.0, 0.4, 0.0));
        assert_eq!(
            *app.world().get::<Transform>(node).unwrap(),
            Transform::from_translation(at)
        );
        app.world_mut().write_message(Shove {
            body: node,
            bone: 0,
            velocity: Vec3::X,
            daze: 0.0,
            step: None,
            sag: None,
        });
        for _ in 0..30 {
            app.update();
        }
        let body = app.world().get::<Ragdoll>(node).unwrap();
        assert!(!body.asleep() && body.countdown() == 256);
        assert!(body.parts[0].positions[3].y < at.y);
        // About 5.5 s of falling to −50 m.
        for _ in 0..200 {
            app.update();
        }
        assert!(app.world().get_entity(node).is_err(), "never removed");
    }

    /// A ray picks the nearest fixed collider, and a stand-in names its body
    /// and part.
    #[test]
    fn a_pick_names_the_stand_ins_body() {
        use rapier3d::prelude::{ColliderBuilder, PhysicsWorld};
        let mut world = PhysicsWorld::new();
        let near = world.insert_collider(
            ColliderBuilder::ball(0.5)
                .translation(Vec3::new(0.0, 0.0, 5.0))
                .user_data(STAND_IN)
                .build(),
            None,
        );
        world.insert_collider(
            ColliderBuilder::ball(0.5)
                .translation(Vec3::new(0.0, 0.0, 9.0))
                .build(),
            None,
        );
        world.step();
        let physics = Physics(world);
        let body = Entity::from_raw_u32(7).unwrap();
        let stand_ins = StandIns(HashMap::from([(body, vec![(near, 2)])]));
        let (handle, t) = physics.fixed_pick(Vec3::ZERO, Vec3::Z, 40.0).unwrap();
        assert_eq!(handle, near);
        assert!((t - 4.5).abs() < 1e-4, "{t}");
        assert_eq!(stand_ins.owner_of(handle), Some((body, 2)));
        assert_eq!(physics.fixed_pick(Vec3::ZERO, Vec3::Z, 4.0), None);
        let (far, _) = physics
            .fixed_pick(Vec3::new(0.0, 0.0, 6.0), Vec3::Z, 40.0)
            .unwrap();
        assert_eq!(stand_ins.owner_of(far), None);
    }

    /// A stand-in follows its body's part from where it was registered.
    #[test]
    fn stand_ins_follow_their_bodies() {
        use rapier3d::prelude::{ColliderBuilder, PhysicsWorld};
        let mut app = App::new();
        app.insert_resource(Physics(PhysicsWorld::new()))
            .init_resource::<StandIns>()
            .init_resource::<crate::collision_share::CollisionChanges>()
            .add_systems(Update, (register_props, move_stand_ins).chain());
        let node = app.world_mut().spawn(GlobalTransform::IDENTITY).id();
        app.world_mut()
            .entity_mut(node)
            .insert(PendingProp(vec![crate_part(node)]));
        app.update();
        let collider = ColliderBuilder::ball(0.2).user_data(STAND_IN).build();
        let handle = app
            .world_mut()
            .resource_mut::<Physics>()
            .0
            .insert_collider(collider, None);
        app.world_mut()
            .resource_mut::<StandIns>()
            .0
            .insert(node, vec![(handle, 0)]);
        let owners = app.world().resource::<StandIns>();
        assert_eq!(owners.owner_of(handle), Some((node, 0)));
        assert_eq!(owners.owner_of(ColliderHandle::invalid()), None);
        // The body moves 1 m along x.
        let mut e = app.world_mut().entity_mut(node);
        let mut body = e.get_mut::<Ragdoll>().unwrap();
        for x in &mut body.parts[0].positions {
            x.x += 1.0;
        }
        app.update();
        let physics = app.world().resource::<Physics>();
        let at = physics
            .0
            .colliders
            .get(handle)
            .unwrap()
            .position()
            .translation;
        assert!((at - Vec3::X).length() < 1e-5, "stand-in at {at}");
    }

    /// A body's stored scrape slots reach its source list when it registers,
    /// on the part that carried them.
    #[test]
    fn stored_scrape_slots_reach_the_registered_body() {
        let mut app = App::new();
        app.add_systems(Update, register_props);
        let node = app.world_mut().spawn(GlobalTransform::IDENTITY).id();
        app.world_mut().entity_mut(node).insert((
            PendingProp(vec![crate_part(node)]),
            PendingScrapes(vec![BodyScrape {
                part: 0,
                point: Vec3::new(0.0, 0.2, 0.0),
                volume: 0.5,
                pitch: 1.2,
            }]),
        ));
        app.update();
        let e = app.world().entity(node);
        assert!(!e.contains::<PendingScrapes>());
        let body = e.get::<Ragdoll>().unwrap();
        assert_eq!(body.scrapes().len(), 1);
        assert_eq!(body.scrapes()[0].part, 0);
        assert_eq!(body.scrapes()[0].point, Vec3::new(0.0, 0.2, 0.0));
        assert_eq!(body.scrapes()[0].volume, 0.5);
        assert_eq!(body.scrapes()[0].pitch, 1.2);
    }
}
