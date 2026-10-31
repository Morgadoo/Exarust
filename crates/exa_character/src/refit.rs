//! The refit: a dressed character dressed again when its worn list or its
//! main hand changes, as the original's character view refits the doll and
//! the shown character after a drop or a drag off the figure.
//!
//! The original's refit of a character from its record runs the generator
//! again on the existing body, as a job (on a worker when one runs, else at
//! once), then applies the result on the main thread:
//!
//! 1. the skin job is reset at the pixel format's side and takes the body
//!    mesh's existing skin texture as its target;
//! 2. the physique's controller weights are set again;
//! 3. the worn list is dressed again: the merged skin mesh is built anew
//!    from the record's worn items (the apparel models and their paints come
//!    from the caches);
//! 4. the skin texture is composited again into its existing texture;
//! 5. the equipment refresh: armour layers, encumbrance and the move speed
//!    it gives, and the worn items' special marks;
//! 6. the hair: its code (none, the covered-head `?` pieces, or the open
//!    style) from the new worn set's head coverage is compared with the
//!    built hair's; the same code keeps the hair, code 0 removes it, any
//!    other grows it again;
//! 7. on the main thread: the old merged mesh's texture mark is cleared,
//!    the new mesh registered in place of the old, the hair swapped when it
//!    was rebuilt.
//!
//! The held items are not part of it. A weapon or shield dropped on the
//! figure, or dragged off it, goes through the hand-scene job (the
//! take-in or take-out on the doll first, then on the shown character):
//! only the changed hand's model is detached and attached again.
//!
//! The drop onto the figure refits the shown character first and the doll
//! second; the drag off the figure refits the doll first. Both run the
//! underwear rule on the record before a refit that removed an item.
//!
//! Exarust keeps the same split. A body that is [`crate::Dressed`] keeps
//! what its dress was built from ([`DressedWith`]); [`request_refits`]
//! compares the body's [`Outfit`] and [`Wielded`] with it whenever either
//! changes and asks for a [`Refit`] of the part that differs. The equip
//! pass then rebuilds only that part, once the new outfit's paints and
//! fits are in (until then the old dress keeps drawing, as the original's
//! old mesh does until the apply): the outfit part tears down what the
//! dress spawned for the old outfit ([`DressPart`]), shows the body parts
//! it hid again ([`DressHidden`]), merges the new outfit, recomputes the
//! armour layers and keeps or regrows the hair ([`hair_refit`]); the hands
//! part replaces only the main hand's item and fists ([`DressHeld`]). The
//! skin composite is keyed by its look, which a refit does not change, so
//! the composited skin is reused rather than generated again.

use bevy::prelude::*;

use crate::look::{HairCut, hair_cut, wanted_hair_code};
use crate::{Outfit, Wielded};

/// What a dressed body's dress was built from: the outfit of its merged
/// mesh and the weapon its main hand was given. A refit is asked for only
/// when the body's own [`Outfit`] or [`Wielded`] differs from it.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct DressedWith {
    pub outfit: Outfit,
    pub wielded: Option<Wielded>,
}

/// A dressed body waiting to be dressed again: its outfit (the merged
/// mesh, armour and hair), its main hand, or both.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Refit {
    pub outfit: bool,
    pub hands: bool,
}

/// Marks an entity the dress spawned for the outfit: a run of the merged
/// mesh, or a garment part drawn on its own node.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct DressPart;

/// Marks a body-part mesh the dress hid (merged into the skin mesh, or
/// covered by a garment).
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct DressHidden;

/// Marks what the dress gave the main hand: the held weapon's holder or
/// the two fists.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct DressHeld;

/// The marks a refit finds the old dress by.
#[derive(bevy::ecs::system::SystemParam)]
pub struct DressMarks<'w, 's> {
    pub parts: Query<'w, 's, (), With<DressPart>>,
    pub hidden: Query<'w, 's, (), With<DressHidden>>,
    pub held: Query<'w, 's, (), With<DressHeld>>,
    pub hair: Query<'w, 's, (), With<crate::hair::HairRoot>>,
}

/// The refit a body needs, from what its dress was built with and what it
/// wears and wields now; `None` when both agree.
pub fn refit_of(
    dressed: &DressedWith,
    outfit: &Outfit,
    wielded: Option<&Wielded>,
) -> Option<Refit> {
    let refit = Refit {
        outfit: dressed.outfit != *outfit,
        hands: dressed.wielded.as_ref() != wielded,
    };
    (refit.outfit || refit.hands).then_some(refit)
}

/// Asks for the refit of every dressed body whose outfit or main hand
/// changed since it was dressed, and withdraws a pending one when the
/// change was undone before it ran.
#[allow(clippy::type_complexity)]
pub fn request_refits(
    mut commands: Commands,
    mut removed: RemovedComponents<Wielded>,
    changed: Query<Entity, (With<DressedWith>, Or<(Changed<Outfit>, Changed<Wielded>)>)>,
    bodies: Query<(&Outfit, Option<&Wielded>, &DressedWith, Option<&Refit>)>,
) {
    let mut seen = Vec::new();
    for body in changed.iter().chain(removed.read()) {
        if seen.contains(&body) {
            continue;
        }
        seen.push(body);
        let Ok((outfit, wielded, dressed, pending)) = bodies.get(body) else {
            continue;
        };
        match refit_of(dressed, outfit, wielded) {
            Some(refit) if pending != Some(&refit) => {
                debug!(
                    "refit requested for {body}: outfit {}, hands {}",
                    refit.outfit, refit.hands
                );
                commands.entity(body).insert(refit);
            }
            Some(_) => {}
            None if pending.is_some() => {
                commands.entity(body).remove::<Refit>();
            }
            None => {}
        }
    }
}

/// Removes what the dress spawned for the old outfit and shows the body
/// parts it hid, before the new outfit is dressed (the commands run in
/// order, so a part the new dress hides again stays hidden).
pub fn tear_down_outfit(
    commands: &mut Commands,
    body: Entity,
    children: &Query<&Children>,
    parts: &Query<(), With<DressPart>>,
    hidden: &Query<(), With<DressHidden>>,
) -> (usize, usize) {
    let (mut removed, mut shown) = (0, 0);
    for entity in children.iter_descendants(body) {
        if parts.contains(entity) {
            commands.entity(entity).despawn();
            removed += 1;
        } else if hidden.contains(entity) {
            commands
                .entity(entity)
                .insert(Visibility::Inherited)
                .remove::<DressHidden>();
            shown += 1;
        }
    }
    (removed, shown)
}

/// Removes the main hand's old item or fists before the hands are given
/// their new ones.
pub fn tear_down_hands(
    commands: &mut Commands,
    body: Entity,
    children: &Query<&Children>,
    held: &Query<(), With<DressHeld>>,
) -> usize {
    let mut removed = 0;
    for entity in children.iter_descendants(body) {
        if held.contains(entity) {
            commands.entity(entity).despawn();
            removed += 1;
        }
    }
    removed
}

/// The worn set's head coverage the hair reads: the union of the garments'
/// coverage words on the head slot (slot 10).
pub fn head_word<'a>(head_words: impl Iterator<Item = &'a u64>) -> u64 {
    head_words.fold(0, |word, w| word | w)
}

/// What a refit does with a character's hair when the worn set's head
/// coverage goes from `old` to `new`: the built code is the one the old
/// coverage grew, the wanted one the new coverage asks for ([`hair_cut`]).
/// A bald style (0) grows no hair either way.
pub fn hair_refit(style: u32, old: u64, new: u64) -> HairCut {
    let look = if style == 0 {
        0
    } else {
        exa_formats::races::LOOK_HAIR
    };
    hair_cut(wanted_hair_code(look, old), wanted_hair_code(look, new))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outfit(models: &[&str]) -> Outfit {
        Outfit::of_models(&models.iter().map(|m| (*m).to_owned()).collect::<Vec<_>>())
    }

    /// Only the part that changed is refit, and nothing when both agree.
    #[test]
    fn a_refit_rebuilds_only_what_changed() {
        let dressed = DressedWith {
            outfit: outfit(&["braies_a", "helm_a"]),
            wielded: Some(Wielded::model("ub05ssworda")),
        };
        let sword = Wielded::model("ub05ssworda");
        assert_eq!(
            refit_of(&dressed, &outfit(&["braies_a", "helm_a"]), Some(&sword)),
            None
        );
        assert_eq!(
            refit_of(&dressed, &outfit(&["braies_a"]), Some(&sword)),
            Some(Refit {
                outfit: true,
                hands: false
            })
        );
        assert_eq!(
            refit_of(&dressed, &outfit(&["braies_a", "helm_a"]), None),
            Some(Refit {
                outfit: false,
                hands: true
            })
        );
        // A garment painted differently is a different outfit.
        let mut repainted = outfit(&["braies_a", "helm_a"]);
        repainted.0[1].hides = Some(1 << 10);
        assert!(refit_of(&dressed, &repainted, Some(&sword)).unwrap().outfit);
    }

    /// The hair keeps its pieces while the head stays as covered as it was,
    /// swaps to the `?` pieces under a helmet and back without it, and goes
    /// under a garment that leaves no room.
    #[test]
    fn the_hair_follows_the_head_coverage() {
        assert_eq!(hair_refit(3, 0, 0), HairCut::Keep);
        assert_eq!(hair_refit(3, 1, 0), HairCut::Rebuild);
        assert_eq!(hair_refit(3, 0, 1), HairCut::Rebuild);
        assert_eq!(hair_refit(3, 1, 1 | 4), HairCut::Keep);
        assert_eq!(hair_refit(3, 0, 2), HairCut::Remove);
        assert_eq!(hair_refit(3, 2, 0), HairCut::Rebuild);
        // Bald: nothing to grow.
        assert_eq!(hair_refit(0, 1, 0), HairCut::Keep);
        assert_eq!(head_word([1u64, 4, 0].iter()), 5);
    }

    /// A change asks for a refit; undoing it before the refit ran
    /// withdraws it; the outfit and the hand are told apart.
    #[test]
    fn changes_ask_for_refits() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Update, request_refits);
        let dressed = DressedWith {
            outfit: outfit(&["braies_a", "helm_a"]),
            wielded: Some(Wielded::model("ub05ssworda")),
        };
        let body = app
            .world_mut()
            .spawn((
                dressed.outfit.clone(),
                Wielded::model("ub05ssworda"),
                dressed.clone(),
            ))
            .id();
        app.update();
        assert!(app.world().get::<Refit>(body).is_none());
        app.world_mut()
            .entity_mut(body)
            .insert(outfit(&["braies_a"]));
        app.update();
        assert_eq!(
            app.world().get::<Refit>(body),
            Some(&Refit {
                outfit: true,
                hands: false
            })
        );
        app.world_mut()
            .entity_mut(body)
            .insert(dressed.outfit.clone());
        app.update();
        assert!(app.world().get::<Refit>(body).is_none());
        app.world_mut().entity_mut(body).remove::<Wielded>();
        app.update();
        assert_eq!(
            app.world().get::<Refit>(body),
            Some(&Refit {
                outfit: false,
                hands: true
            })
        );
    }
}
