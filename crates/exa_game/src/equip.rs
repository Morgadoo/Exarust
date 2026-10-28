//! What a character's record says it holds becomes the hands the moveset
//! and the player light read.
//!
//! The record keeps two weapon sets of two hands each and the view's apply
//! step trades them when a swap re-dresses the hands. Exarust carries the
//! plan of that step as [`PendingHands`]; this module turns the record's
//! out set into the [`Hands`] component once the swap has landed: the main
//! hand's weapon, a second weapon in the off hand, the off hand's shield,
//! and a torch in the off hand (an off-hand weapon that also has a light
//! section is the light, as the attach rule has it). A hand holding an item
//! with none of those sections is empty for the moveset.
//!
//! Evidence: the character view's attachment plan and apply step, read in
//! the instructions (`context/re/item-gui-lane.md`).

use bevy::prelude::*;
use exa_character::held::{Hands, HeldItem, ShieldStyle};
use exa_items::catalogue::{ItemCatalogue, PendingHands};
use exa_items::possession::{ItemKind, Possessions};

/// The hands a character's out set makes. `kind` gives an item's sections.
pub fn hands_of(p: &Possessions, kind: impl Fn(u32) -> Option<ItemKind>) -> Hands {
    let mut hands = Hands::default();
    let out = &p.out;
    if let Some(k) = (out[0].id != 0).then(|| kind(out[0].id)).flatten()
        && let Some(flags) = k.weapon
    {
        hands.main = Some(HeldItem {
            flags: (flags & 0xFFFF) as u16,
            light: k.light,
        });
    }
    if let Some(k) = (out[1].id != 0).then(|| kind(out[1].id)).flatten() {
        if let Some(flags) = k.weapon {
            if k.light {
                hands.off_light = true;
            } else {
                hands.second = Some(HeldItem {
                    flags: (flags & 0xFFFF) as u16,
                    light: false,
                });
            }
        } else if k.shield {
            hands.shield = Some(ShieldStyle::Plain);
        } else if k.light {
            hands.off_light = true;
        }
    }
    hands
}

/// Applies the plan a swap left on the body: the hands follow the record's
/// out set and the plan is consumed. The plan itself is the record side of
/// the work; the models are attached by the renderer's own draw of the
/// hands, so nothing is spawned here.
pub fn apply_pending_hands(
    mut commands: Commands,
    catalogue: Option<Res<ItemCatalogue>>,
    mut bodies: Query<(Entity, &Possessions, &mut Hands), With<PendingHands>>,
) {
    for (body, possessions, mut hands) in &mut bodies {
        let next = hands_of(possessions, |id| {
            catalogue.as_ref().and_then(|c| c.kind(id))
        });
        hands.set_if_neq(next);
        commands.entity(body).remove::<PendingHands>();
    }
}

/// Applies the hand plans of swapped weapon sets.
pub struct EquipPlugin;

impl Plugin for EquipPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            apply_pending_hands.after(exa_items::catalogue::swap_weapon_sets),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_items::possession::Hand;

    fn item(weapon: Option<u32>, shield: bool, light: bool) -> ItemKind {
        ItemKind {
            weapon,
            shield,
            light,
            sight: false,
        }
    }

    fn kind_of(id: u32) -> Option<ItemKind> {
        Some(match id {
            5 => item(Some(0x521), false, false),
            6 => item(Some(0x11), false, false),
            7 => item(None, true, false),
            8 => item(None, false, true),
            9 => item(Some(0x10), false, true),
            _ => return None,
        })
    }

    #[test]
    fn the_out_set_makes_the_hands() {
        let p = Possessions {
            out: [Hand { id: 5, flags: 0 }, Hand { id: 6, flags: 0 }],
            ..Default::default()
        };
        let hands = hands_of(&p, kind_of);
        assert_eq!(hands.main.unwrap().flags, 0x521);
        assert_eq!(hands.second.unwrap().flags, 0x11);
        assert!(hands.shield.is_none() && !hands.off_light);
        // A shield in the off hand.
        let p = Possessions {
            out: [Hand { id: 5, flags: 0 }, Hand { id: 7, flags: 0 }],
            ..Default::default()
        };
        let hands = hands_of(&p, kind_of);
        assert_eq!(hands.shield, Some(ShieldStyle::Plain));
        assert!(hands.second.is_none());
        // A light in the off hand.
        let p = Possessions {
            out: [Hand { id: 0, flags: 0 }, Hand { id: 8, flags: 0 }],
            ..Default::default()
        };
        let hands = hands_of(&p, kind_of);
        assert!(hands.main.is_none() && hands.off_light);
        // A weapon that also has a light section is the off-hand light.
        let p = Possessions {
            out: [Hand { id: 5, flags: 0 }, Hand { id: 9, flags: 0 }],
            ..Default::default()
        };
        let hands = hands_of(&p, kind_of);
        assert!(hands.off_light && hands.second.is_none() && hands.shield.is_none());
        // An item with no section in the main hand leaves it empty.
        let p = Possessions {
            out: [Hand { id: 7, flags: 0 }, Hand { id: 0, flags: 0 }],
            ..Default::default()
        };
        assert!(hands_of(&p, kind_of).main.is_none());
    }

    #[test]
    fn a_swap_moves_the_hands() {
        let mut app = App::new();
        app.init_resource::<ItemCatalogue>()
            .add_plugins(EquipPlugin);
        let body = app
            .world_mut()
            .spawn((
                Possessions {
                    out: [Hand { id: 6, flags: 0 }, Hand::default()],
                    ..Default::default()
                },
                Hands {
                    main: Some(HeldItem::weapon(0x521)),
                    ..Default::default()
                },
                PendingHands(exa_items::possession::HandPlan {
                    changed: [true, false],
                    take: [None, None],
                }),
            ))
            .id();
        app.update();
        // The catalogue is empty, so the record's item resolves to no
        // sections and the main hand is emptied; the plan is consumed.
        let hands = app.world().get::<Hands>(body).unwrap();
        assert!(hands.main.is_none());
        assert!(app.world().get::<PendingHands>(body).is_none());
    }
}
