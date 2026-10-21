//! A character's hair on its head: the `CharExt` style its record names,
//! its colour and the covered-head rule for the style's `?` pieces.
//!
//! The body builder adds a hair style under the head part, resets its roots
//! to the head's frame and keeps the `?`-prefixed roots only while a helmet
//! or hat covers the head. Exarust builds the style's model once with the
//! asset loader and parents its nodes to the head part, tinting the hair
//! material with the record's colour.

use bevy::prelude::*;
use exa_assets::{Game, ModelCache, ModelLoader, PackKind};
use exa_render::material::linear_rgb;

use crate::Outfit;
use crate::look::{hair_entry, hair_root_kept};
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
}

/// Marks a body whose hair has been built.
#[derive(Component)]
pub struct HairAttached;

/// The head's bone in the body model.
pub const HEAD: usize = 10;

/// Builds the hair of every body that has not got it yet.
///
/// A style shows nothing when its entry is missing or when the outfit covers
/// the head and only the style's `?` roots remain. The roots are re-framed
/// on the head part, and every part's material takes the record's colour,
/// specular level and gloss.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn attach_hair(
    mut commands: Commands,
    game: Option<Res<Game>>,
    mut cache: ResMut<ModelCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<exa_render::material::ExaMaterial>>,
    mut images: ResMut<Assets<Image>>,
    support: Option<Res<bevy::image::CompressedImageFormatSupport>>,
    bodies: Query<(Entity, &Hair, Option<&Outfit>), Without<HairAttached>>,
    children: Query<&Children>,
    parts: Query<&BodyPart>,
) {
    let Some(game) = game else { return };
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
    for (body, hair, outfit) in &bodies {
        let Some(head) = children
            .iter_descendants(body)
            .find(|&e| parts.get(e).is_ok_and(|p| p.bone == HEAD))
        else {
            continue;
        };
        let covered =
            outfit.is_some_and(|o| o.0.iter().any(|g| g.hides.unwrap_or(0) & (1 << HEAD) != 0));
        let Some(entry) = hair_entry(hair.style, &entries) else {
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
            for (mesh, material) in &node.parts {
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
            }
        }
        commands.entity(body).insert(HairAttached);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The head is bone 10 in the body model, where the hair hangs.
    #[test]
    fn the_hair_hangs_on_the_heads_bone() {
        assert_eq!(HEAD, 10);
    }

    /// The colour and covered-head rules are the pure look module's.
    #[test]
    fn the_hair_record_is_the_looks() {
        let h = Hair {
            style: 3,
            colour: [40, 80, 40],
            feral: 0.0,
            light_variant: false,
        };
        assert_eq!(
            hair_shading(h.colour, h.feral, h.light_variant).diffuse,
            hair_shading([40, 80, 40], 0.0, false).diffuse
        );
        assert!(hair_root_kept("?HULA01_", true));
        assert!(!hair_root_kept("?HULA01_", false));
    }
}
