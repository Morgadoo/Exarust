//! A character's hair on its head: the `CharExt` style its record names,
//! its colour and the covered-head rule for the style's `?` pieces.
//!
//! The body builder adds a hair style under the head part, resets its roots
//! to the head's frame and keeps the `?`-prefixed roots only while the worn
//! set's head coverage has bit 0 (a hood or helmet); bit 1 grows no hair.
//! Exarust builds the style's model once with the asset loader and parents
//! its nodes to the head part, tinting the hair material with the record's
//! colour. The original swaps in its hair material only on parts whose
//! material word (payload offset `0xC`) is over 0.5; every shipped hair
//! material (`harhps1`) holds 1.0 there, so every part is tinted.
//!
//! Long hair hangs as cloth: each style node carrying the cloth marker (the
//! two meshes of `hair hula01…05` and `hair husa02`) becomes a simulated
//! cloth ([`hang_as_cloth`]), its roots with the hair's cloth record. The
//! cloth is pinned by its faces' flags, follows the head part it hangs from
//! and never sleeps below that body's countdown. Not done: the record's bit
//! `0x10000` also swaps the hair textures to the `HarHPC1` set, and the
//! creation scene's smoothing of the hair.

use bevy::prelude::*;
use exa_assets::{Game, ModelCache, ModelLoader, PackKind};
use exa_render::material::linear_rgb;

use crate::look::{hair_entry, hair_root_kept, head_covered};
use exa_anim::BodyPart;
use exa_assets::hair::hair_shading;

/// A character's hair, as its record stores it.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Hair {
    /// The record's style: 0 is bald, 1 is the first `CharExt` style.
    pub style: u32,
    /// The record's hue, saturation and value bytes.
    pub colour: [u8; 3],
    /// The record's feral level: it lightens and thins the hair.
    pub feral: f32,
    /// The record's bit `0x10000` leaves the specular level alone.
    pub light_variant: bool,
    /// The worn set's union of coverage words on the head slot.
    pub head_word: u64,
}

/// Marks a body whose hair has been built.
#[derive(Component)]
pub struct HairAttached;

/// The head's bone in the body model.
pub const HEAD: usize = 10;

/// How long one frame spends building hair, so a crowd's styles load over
/// several frames.
const HAIR_BUDGET: std::time::Duration = std::time::Duration::from_millis(2);

/// Builds the hair of every body that has not got it yet.
///
/// A style shows nothing when its entry is missing or the head word has
/// bit 1; under bit 0 only its `?` roots show, otherwise only the others.
/// The roots are re-framed on the head part, and every part's material
/// takes the record's colour, specular level and gloss.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn attach_hair(
    mut commands: Commands,
    game: Option<Res<Game>>,
    cache: Option<ResMut<ModelCache>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<exa_render::material::ExaMaterial>>,
    mut images: ResMut<Assets<Image>>,
    support: Option<Res<bevy::image::CompressedImageFormatSupport>>,
    bodies: Query<(Entity, &Hair), Without<HairAttached>>,
    children: Query<&Children>,
    parts: Query<&BodyPart>,
    hair_cloth: Option<Res<HairCloth>>,
) {
    let (Some(game), Some(mut cache)) = (game, cache) else {
        return;
    };
    if bodies.is_empty() {
        return;
    }
    let mut loader = ModelLoader {
        game: &game.0,
        cache: &mut cache,
        meshes: &mut meshes,
        materials: &mut materials,
        images: &mut images,
        bc_supported: exa_assets::model::bc_supported(support.as_deref()),
    };
    let entries: Vec<String> = game
        .0
        .pack(PackKind::CharExt)
        .map(|p| {
            p.entries()
                .iter()
                .filter(|e| e.name.starts_with("hair "))
                .map(|e| e.name.clone())
                .collect()
        })
        .unwrap_or_default();
    let cloth_on = hair_cloth.map_or_else(hair_cloth_on, |h| h.0);
    let started = std::time::Instant::now();
    for (body, hair) in &bodies {
        if started.elapsed() >= HAIR_BUDGET {
            break;
        }
        let Some(head) = children
            .iter_descendants(body)
            .find(|&e| parts.get(e).is_ok_and(|p| p.bone == HEAD))
        else {
            continue;
        };
        let covered = head_covered(hair.head_word);
        let Some(entry) = hair_entry(hair.style, &entries).filter(|_| hair.head_word & 2 == 0)
        else {
            commands.entity(body).insert(HairAttached);
            continue;
        };
        let Some(model) = loader.model(PackKind::CharExt, entry) else {
            commands.entity(body).insert(HairAttached);
            continue;
        };
        let shading = hair_shading(hair.colour, hair.feral, hair.light_variant);
        let root = commands
            .spawn((Transform::IDENTITY, Visibility::default(), ChildOf(head)))
            .id();
        let mut entities = vec![Entity::PLACEHOLDER; model.nodes.len()];
        let mut hidden = vec![false; model.nodes.len()];
        for (i, node) in model.nodes.iter().enumerate() {
            let parent_hidden = node.parent.is_some_and(|p| hidden[p]);
            if parent_hidden || (node.parent.is_none() && !hair_root_kept(&node.name, covered)) {
                hidden[i] = true;
                continue;
            }
            let local = if node.parent.is_some() {
                node.transform
            } else {
                Transform::IDENTITY
            };
            let parent = node.parent.map_or(root, |p| entities[p]);
            let e = commands
                .spawn((
                    Name::new(node.name.clone()),
                    local,
                    Visibility::default(),
                    ChildOf(parent),
                ))
                .id();
            entities[i] = e;
            for (part_index, (mesh, material)) in node.parts.iter().enumerate() {
                // The hair material takes the record's colour; a type-3
                // material is linearised like the loader's own.
                let tinted = match loader.materials.get(material).cloned() {
                    Some(mut m) => {
                        m.params.diffuse = linear_rgb(shading.diffuse)
                            .extend(m.params.diffuse.w * shading.opacity);
                        m.params.specular = linear_rgb(shading.specular).extend(shading.gloss);
                        loader.materials.add(m)
                    }
                    None => material.clone(),
                };
                let part = commands.spawn((MeshMaterial3d(tinted), ChildOf(e))).id();
                match node.skin_bone {
                    Some(bone) => commands.entity(part).insert(exa_assets::SkinnedPart {
                        bone,
                        mesh: mesh.clone(),
                    }),
                    None => commands.entity(part).insert(Mesh3d(mesh.clone())),
                };
                if node.cloth && cloth_on {
                    hang_as_cloth(
                        &mut commands,
                        part,
                        node.parent.is_none(),
                        node.cloth_mesh.as_ref(),
                        part_index,
                    );
                }
            }
        }
        commands.entity(body).insert(HairAttached);
    }
}

/// Whether long hair hangs as cloth: only with `EXA_HAIR_CLOTH=1`. A
/// living character's hair never sleeps (a hair cloth stays awake on its
/// head's countdown, as the original's), so its cost is bounded by what
/// the sector offers it: only the body parts bucketed in the nine 6 m
/// cells around it, as the original's grid sector walks them
/// ([`exa_physics::sector::CellGrid::around`]), not every part of the
/// level (which held exanima01 at 16 FPS before the stage culled them).
/// Off by default: even culled, exanima01 stayed at 16 FPS on the RTX 3090
/// with hair on (the cloth stage 3.8 ms a tick, the fixed schedule 12.5 ms
/// a tick). The stage now steps its cloths at once and writes their meshes
/// once a frame instead of every tick; the switch stays off until a GPU run
/// with `EXA_DEBUG_CLOTH=1` (whose log sets the stage against the whole
/// fixed tick) shows the level back near 60 FPS.
pub fn hair_cloth_on() -> bool {
    std::env::var_os("EXA_HAIR_CLOTH").is_some_and(|v| v != "0")
}

/// The hair cloth switch as a resource, for a host or a test to set; the
/// attach falls back to [`hair_cloth_on`] when it is absent.
#[derive(Resource, Debug, Clone, Copy)]
pub struct HairCloth(pub bool);

/// Gives a cloth-marked part of a hair style its cloth: the level's cloth
/// stage builds it from the node's faces and face flags
/// ([`exa_world::cloth`]). A style's root takes the hair's cloth record
/// ([`ClothRecord::HAIR`]: the particle mode, 0.2 drag, no padding, two
/// passes), as the character build hands each root whose controller is a
/// cloth; a marked node below a root keeps the built-in record its loader
/// gave it. The build also writes 0.1 to a cloth word (1.0 by default)
/// whose reader is not traced [not reproduced].
///
/// [`ClothRecord::HAIR`]: exa_physics::cloth_record::ClothRecord::HAIR
pub fn hang_as_cloth(
    commands: &mut Commands,
    part: Entity,
    root: bool,
    source: Option<&std::sync::Arc<exa_assets::ClothMesh>>,
    part_index: usize,
) {
    use exa_world::cloth::{ClothNode, ClothParameters, ClothSource};
    let mut entity = commands.entity(part);
    entity.insert(ClothNode);
    if let Some(mesh) = source {
        entity.insert(ClothSource {
            mesh: mesh.clone(),
            part: part_index,
        });
    }
    if root {
        entity.insert(ClothParameters(
            exa_physics::cloth_record::ClothRecord::HAIR,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A style's model hangs under the head part, with the head's bone.
    #[test]
    fn a_style_hangs_under_the_head() {
        use std::sync::Arc;
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<exa_render::material::ExaMaterial>>()
            .init_resource::<Assets<Image>>()
            .init_resource::<ModelCache>()
            .insert_resource(Game(Arc::new(game)))
            .add_systems(Update, attach_hair);
        let body = app
            .world_mut()
            .spawn((
                Hair {
                    style: 1,
                    colour: [40, 80, 40],
                    feral: 0.0,
                    light_variant: false,
                    head_word: 0,
                },
                crate::Outfit::default(),
            ))
            .id();
        let head = app
            .world_mut()
            .spawn((
                BodyPart {
                    bone: HEAD,
                    rest: Transform::IDENTITY,
                },
                ChildOf(body),
            ))
            .id();
        app.update();
        let world = app.world();
        assert!(world.get::<HairAttached>(body).is_some());
        let children = world.get::<Children>(head).unwrap();
        assert!(!children.is_empty(), "the hair style spawned no root");
    }

    /// The first long style (`hair hula01`) hangs as cloth: its root part
    /// takes the hair's record, and its faces' flags pin some of its
    /// vertices (the scalp) and leave the rest free.
    #[test]
    fn a_long_style_hangs_as_pinned_cloth() {
        use exa_world::cloth::{ClothNode, ClothParameters, ClothSource};
        use std::sync::Arc;
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<exa_render::material::ExaMaterial>>()
            .init_resource::<Assets<Image>>()
            .init_resource::<ModelCache>()
            .insert_resource(Game(Arc::new(game)))
            .add_plugins(exa_world::cloth::ClothPlugin)
            .insert_resource(HairCloth(true))
            .add_systems(Update, attach_hair);
        let body = app
            .world_mut()
            .spawn(Hair {
                style: 1,
                colour: [40, 80, 40],
                feral: 0.0,
                light_variant: false,
                head_word: 0,
            })
            .id();
        app.world_mut().spawn((
            BodyPart {
                bone: HEAD,
                rest: Transform::IDENTITY,
            },
            ChildOf(body),
        ));
        app.update();
        app.update();
        let world = app.world_mut();
        // The cloth stage built each cloth, pinned, in the particle mode.
        let mut built = world.query::<&exa_world::cloth::ClothInstance>();
        let instances: Vec<_> = built.iter(world).collect();
        assert!(!instances.is_empty(), "the hair cloth was never built");
        for instance in instances {
            assert!(instance.cloth.record.particle_mode());
            assert!(!instance.cloth.pins.is_empty());
        }
        let mut cloths = world.query::<(&ClothNode, Option<&ClothParameters>, &ClothSource)>();
        let found: Vec<_> = cloths
            .iter(world)
            .map(|(_, p, s)| (p.copied(), s.mesh.clone()))
            .collect();
        assert!(!found.is_empty(), "the long style hung no cloth");
        for (parameters, mesh) in &found {
            assert_eq!(
                parameters.map(|p| p.0),
                Some(exa_physics::cloth_record::ClothRecord::HAIR)
            );
            let pins =
                exa_physics::cloth::face_flag_pins(mesh.elements, &mesh.faces, &mesh.face_flags);
            assert!(!pins.is_empty(), "the hair cloth has no pinned vertex");
            assert!(pins.len() < mesh.elements, "every hair vertex is pinned");
        }
    }
}
