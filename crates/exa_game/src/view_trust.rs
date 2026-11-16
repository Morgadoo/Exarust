//! The character view's mind cost: the trust change an equipment change on
//! a bound body makes.
//!
//! The inventory's fill sets its view's mind window when the shown body
//! carries a Loyalty or possession marker
//! ([`crate::gui_screens::GameScreens::inventory_mind_window`]). With it
//! set, the view weighs every change of the body's equipment:
//! - a drag off the figure lowers the trust by the item's cost
//!   ([`exa_ui::character_view::drag_over_cost`]: twice a weapon's
//!   desirability, else an apparel's protection value) and keeps the cost;
//! - a cancelled drag, or a drop back on the view, raises it by the kept
//!   cost again;
//! - a drop of apparel raises it by the piece's wear room
//!   ([`exa_ai::search::wear_room`]); a drop of a weapon by twice its
//!   desirability less what it displaces from the hands
//!   ([`exa_items::possession::displaced_value`], each displaced item
//!   weighed by [`held_value`]).
//!
//! Each marker takes the change into its trust and runs its limit test
//! ([`crate::mind::casts::change_binding_trust`]); the first that fails
//! ends, and the view closes its window (the drag or drop then does
//! nothing). The change is a write to the body's mind, which the
//! simulation owns: it goes through [`FixedCommands::write`].
//!
//! Evidence: the character view's drag start, drop and release, the mind
//! unit's trust change and the Loyalty marker's limit test, read against
//! the assembly (`docs/UI.md`, Character view).

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use exa_ai::search::{Apparel, Arm};
use exa_assets::live_objects::LiveObjects;
use exa_items::inventory_window::MIND_CONTROL_BIT;
use exa_physics::sim_app::FixedCommands;

use crate::mind::casts::{BodyId, MindBody, MindClass, change_binding_trust};

/// What the view weighs an item by, read from its record's payload: the
/// weapon section, the apparel body and the shield body's sixth byte.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ItemValues {
    pub weapon: Option<Arm>,
    pub apparel: Option<Apparel>,
    pub shield: Option<u8>,
}

impl ItemValues {
    /// The values of a record payload.
    pub fn of_payload(payload: &[u8]) -> Self {
        Self {
            weapon: Arm::of_payload(payload),
            apparel: Apparel::of_payload(payload),
            shield: exa_formats::containers::section(payload, exa_ai::search::section::SHIELD)
                .map(|body| body.get(6).copied().unwrap_or(0)),
        }
    }

    /// The values of the record `id` names in the live database (any
    /// tier); an id it does not hold has none.
    pub fn of(live: Option<&LiveObjects>, id: u32) -> Self {
        live.and_then(|live| live.db().find(id))
            .map_or_else(Self::default, |record| Self::of_payload(record.payload()))
    }

    /// The weapon's desirability, 0 without a weapon section.
    pub fn desirability(&self) -> f32 {
        self.weapon.map_or(0.0, |arm| arm.value())
    }

    /// The drag-off cost's inputs: the apparel's protection value.
    pub fn protection(&self) -> f32 {
        self.apparel.as_ref().map_or(0.0, Apparel::bulk)
    }
}

/// A held item's value as the take-in's displacement weighs it: the weapon
/// section's desirability, else the shield's. The shield's routine answers
/// a double that its caller reads as a single: the low half of the double's
/// bits, so the weight of a held shield is that word's float (0 for every
/// byte whose product is exact) [the original's quirk, kept].
pub fn held_value(values: &ItemValues) -> f32 {
    if let Some(arm) = values.weapon {
        return arm.value();
    }
    values.shield.map_or(0.0, |byte| {
        let double = exa_formats::item_value::shield_desirability(byte);
        f32::from_bits(double.to_bits() as u32)
    })
}

/// The view's trust change on a bound body.
#[derive(SystemParam)]
pub struct ViewTrust<'w, 's> {
    minds: Query<'w, 's, (Entity, &'static mut MindBody)>,
    fighters: Query<'w, 's, &'static exa_combat::Fighter>,
    relations: crate::thaum::live::MindRelations<'w, 's>,
    fixed: Option<ResMut<'w, FixedCommands>>,
}

impl ViewTrust<'_, '_> {
    /// Changes the trust of `body`'s Loyalty and possession markers by
    /// `amount` ([`change_binding_trust`]); answers whether one ended,
    /// which closes the view's window. A body without a mind changes
    /// nothing. The relations are read now, on the frame: the body's last
    /// attitude toward each marker's caster (a missing relation answers 0,
    /// as the original's read answers a relation it makes).
    pub fn change(&mut self, body: Entity, amount: f32) -> bool {
        let Ok((_, mind)) = self.minds.get(body) else {
            return false;
        };
        let casters: Vec<BodyId> = mind
            .powers
            .iter()
            .filter_map(|power| match &power.class {
                MindClass::LoyaltyMark(mark) => mark.caster.or(power.caster),
                _ => power.caster,
            })
            .collect();
        let relations: Vec<(BodyId, f32)> = casters
            .into_iter()
            .map(|caster| {
                let toward = self
                    .minds
                    .iter()
                    .find(|(_, mind)| mind.id == caster)
                    .map(|(entity, _)| entity);
                let relation = toward.map_or(0.0, |toward| self.relations.peek(body, toward));
                (caster, relation)
            })
            .collect();
        let own = self
            .fighters
            .get(body)
            .is_ok_and(|fighter| fighter.perks.body_word() & MIND_CONTROL_BIT != 0);
        let write = move |mind: &mut MindBody| {
            change_binding_trust(
                mind,
                amount,
                |caster| {
                    caster
                        .and_then(|c| relations.iter().find(|(id, _)| *id == c))
                        .map_or(0.0, |(_, relation)| *relation)
                },
                own,
            )
        };
        let Ok((_, mut mind)) = self.minds.get_mut(body) else {
            return false;
        };
        let ended = match self.fixed.as_deref_mut() {
            Some(fixed) => fixed.write(body, &mut mind, write),
            None => write(&mut mind),
        };
        info!(
            "character view: the bound body's trust changes by {amount}{}",
            if ended { "; a binding ends" } else { "" }
        );
        ended
    }
}

/// Test support: a player and a body bound to it by a Loyalty marker, the
/// body's last attitude toward the player `relation`.
#[cfg(test)]
pub(crate) fn bound_pair(world: &mut World, relation: f32) -> (Entity, Entity) {
    use crate::mind::casts::{LOYALTY_MARK, MindPower};
    use crate::mind::runtime::body_id;
    let player = world
        .spawn((
            crate::player::PlayerBody::new(0.0),
            exa_ai::brain::CharacterId(7),
            exa_ai::roles::Roles::default(),
        ))
        .id();
    world
        .entity_mut(player)
        .insert(MindBody::new(body_id(player)));
    let mut relations = exa_ai::roles::Relations::default();
    relations.create(7, 0.0, 0, 0);
    relations.find_mut(7).unwrap().last = relation;
    let body = world
        .spawn((relations, exa_items::possession::Possessions::default()))
        .id();
    let mut mind = MindBody::new(body_id(body));
    let mut mark = MindPower::new(
        LOYALTY_MARK,
        MindClass::LoyaltyMark(crate::thaum::power::mind::LoyaltyTarget::new(body_id(
            player,
        ))),
    );
    mark.caster = Some(body_id(player));
    mind.powers.push(mark);
    world.entity_mut(body).insert(mind);
    (player, body)
}

/// Test support: the trust of the body's first marker and whether it runs.
#[cfg(test)]
pub(crate) fn marker_trust(world: &World, body: Entity) -> (f32, bool) {
    let power = &world.get::<MindBody>(body).unwrap().powers[0];
    let trust = match &power.class {
        MindClass::LoyaltyMark(mark) => mark.trust,
        _ => f32::NAN,
    };
    (trust, power.state() != crate::thaum::power::ENDING)
}

/// Test support: a one-handed weapon record's payload whose desirability
/// is 2.4 (three skill points, full quality, no wear: 3, × 0.8 for the
/// one-handed grip).
#[cfg(test)]
pub(crate) fn weapon_payload() -> Vec<u8> {
    let mut weapon = vec![0u8; 0x5A];
    weapon[0] = 0x10;
    weapon[0x0C] = 0xFF;
    weapon[0x42] = 3;
    let mut payload = 0x4000u32.to_le_bytes().to_vec();
    payload.extend((weapon.len() as u32 + 8).to_le_bytes());
    payload.extend(weapon);
    payload
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_values_read_the_weapon_and_the_shield_quirk() {
        let values = ItemValues::of_payload(&weapon_payload());
        assert!((values.desirability() - 2.4).abs() < 1e-6);
        assert!((held_value(&values) - 2.4).abs() < 1e-6);
        assert!(values.apparel.is_none());
        // A shield's weight is the low half of its double: 40 × 0.05 = 2
        // is exact, so it weighs 0; 7 × 0.05 is not.
        let shield = |byte: u8| ItemValues {
            shield: Some(byte),
            ..Default::default()
        };
        assert_eq!(held_value(&shield(40)), 0.0);
        let odd = (7.0f64 * 0.05).to_bits() as u32;
        assert_eq!(held_value(&shield(7)).to_bits(), odd);
        assert_eq!(held_value(&ItemValues::default()), 0.0);
    }
}
