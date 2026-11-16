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
use exa_formats::characters::ID_MASK;
use exa_formats::encumbrance::{Skills, encumbrance, head_extra};
use exa_formats::objdb::Armour;
use exa_items::catalogue::{ItemCatalogue, PendingHands};
use exa_items::possession::{ItemKind, Possessions};

use crate::torch::CarriesTorch;

/// A character's encumbrance from what its record currently wears and
/// holds: the worn items' burdens, the shield of the first two hands (the
/// second wins, as [`exa_formats::encumbrance::held_shield`] picks it), and
/// the character's armour and shield techniques. An item the catalogue
/// does not know weighs nothing, as a missing record does in the original's
/// armour pass. Ids are looked up by their low 30 bits, as the spawn's sum
/// looks them up in the base database, so the two agree.
pub fn encumbrance_of(p: &Possessions, catalogue: &ItemCatalogue, skills: Skills) -> f32 {
    let worn: Vec<_> = p
        .worn
        .iter()
        .filter_map(|worn| catalogue.burden(worn.id & ID_MASK))
        .collect();
    let shield = [p.out[1], p.out[0]]
        .into_iter()
        .filter(|hand| hand.id != 0)
        .map(|hand| hand.id & ID_MASK)
        .filter(|&id| {
            catalogue
                .kind(id)
                .is_some_and(|kind| kind.shield && kind.weapon.is_none())
        })
        .find_map(|id| catalogue.shield_record(id));
    encumbrance(&worn, shield.as_ref(), skills)
}

/// Refreshes a body's encumbrance when its possessions change, as the
/// armour pass does after an equip, unequip or swap. Both copies are
/// written: the body's [`Physical::stats`](exa_physics::body::Physical)
/// (gait, turn and step reach) and its fighter's stats (footwork and the
/// movement stage). The body's live record supplies the techniques. A
/// body's first frame keeps the value its spawn summed from the level's
/// own item list.
#[allow(clippy::type_complexity)]
pub fn refresh_encumbrance(
    catalogue: Option<Res<ItemCatalogue>>,
    mut bodies: Query<
        (
            Ref<Possessions>,
            Option<&exa_character::record::CharacterData>,
            &mut exa_physics::body::Physical,
            Option<&mut exa_combat::Fighter>,
        ),
        Changed<Possessions>,
    >,
) {
    let Some(catalogue) = catalogue.as_deref() else {
        return;
    };
    for (possessions, data, mut physical, fighter) in &mut bodies {
        if possessions.is_added() {
            continue;
        }
        let skills = data.map_or_else(Skills::default, |data| Skills::of_record(&data.record));
        let value = encumbrance_of(&possessions, catalogue, skills);
        physical.stats.encumbrance = value;
        if let Some(mut fighter) = fighter {
            fighter.stats.encumbrance = value;
        }
    }
}

/// The armour pass's worn-item rules, run again when a body's possessions
/// change (an equip, an unequip or a swap) as the original runs the whole
/// pass on every equipment refresh; the spawn ran them first:
/// - the special kinds ([`exa_formats::encumbrance::WornSpecials`]): kind 2
///   grants the second wind, 3/4/6 register their item effects, 4 marks
///   the golem suit, 5 wards the mind, 7 quickens the focus;
/// - the item-effect list: the pass first drops every worn item's entry,
///   then registers the worn ones in wearing order; an item still worn
///   keeps the bytes its entry carries;
/// - the golem fix: a golem race's red regeneration is blocked unless it
///   wears its core item, an item without an apparel section
///   ([`exa_formats::character_stats::golem_rules`]).
///
/// A body's first frame keeps what its spawn set. A change that leaves the
/// worn list and the out set's items as the last pass saw them (an item
/// taken from or given to the carried list) runs no pass ([`ArmourPassSeen`]).
#[allow(clippy::type_complexity)]
pub fn refresh_worn_specials(
    mut commands: Commands,
    catalogue: Option<Res<ItemCatalogue>>,
    mut bodies: Query<
        (
            Entity,
            Ref<Possessions>,
            &mut exa_combat::Fighter,
            Option<&mut crate::item_effects::WornEffects>,
            Option<&mut crate::thaum::cast::PowerCaster>,
            Option<&mut ArmourPassSeen>,
        ),
        Changed<Possessions>,
    >,
) {
    let Some(catalogue) = catalogue.as_deref() else {
        return;
    };
    for (entity, possessions, mut fighter, effects, caster, seen) in &mut bodies {
        let now = ArmourPassSeen::of(&possessions);
        match seen {
            Some(mut seen) => {
                if *seen == now {
                    continue;
                }
                *seen = now;
            }
            None => {
                commands.entity(entity).insert(now);
            }
        }
        if possessions.is_added() {
            continue;
        }
        let worn: Vec<(u32, Option<exa_formats::objdb::Burden>)> = possessions
            .worn
            .iter()
            .map(|w| (w.id, catalogue.burden(w.id & ID_MASK)))
            .collect();
        let specials = exa_formats::encumbrance::WornSpecials::of_worn(
            worn.iter().filter_map(|&(id, b)| Some((id, b?))),
        );
        let rules = exa_formats::character_stats::golem_rules(
            fighter.stats.rules,
            fighter.stats.golem_core,
            worn.iter().filter(|(_, b)| b.is_none()).map(|&(id, _)| id),
        );
        if fighter.stats.rules != rules
            || fighter.second_wind.granted != specials.second_wind
            || fighter.stats.weightless != specials.weightless
        {
            fighter.stats.rules = rules;
            fighter.second_wind.granted = specials.second_wind;
            // The gauntlets' weightless share, which the holding pass of
            // the next equip refresh reads.
            fighter.stats.weightless = specials.weightless;
        }
        let fresh = crate::item_effects::WornEffects::of_specials(&specials, rules.0);
        match (effects, fresh) {
            (Some(mut old), Some(mut new)) => {
                for item in &mut new.items {
                    if let Some(kept) = old.items.iter().find(|o| o.id == item.id) {
                        *item = *kept;
                    }
                }
                new.list.impacts = old.list.impacts;
                new.list.ticks = old.list.ticks;
                new.next_run_ms = old.next_run_ms;
                if *old != new {
                    *old = new;
                }
            }
            (Some(mut old), None) => {
                // Every worn item's entry is dropped; the list stays, empty.
                if !old.list.entries.is_empty() || !old.items.is_empty() {
                    old.list.entries.clear();
                    old.items.clear();
                }
            }
            (None, Some(new)) => {
                commands.entity(entity).insert(new);
            }
            (None, None) => {}
        }
        let suit = specials
            .effects
            .iter()
            .any(|item| item.special == exa_formats::objdb::Burden::GOLEM_SUIT);
        if suit {
            commands.entity(entity).insert(exa_combat::GolemSuit);
        } else {
            commands.entity(entity).remove::<exa_combat::GolemSuit>();
        }
        if specials.mind_ward {
            commands.entity(entity).insert(exa_ai::brain::MindWarded);
        } else {
            commands
                .entity(entity)
                .remove::<exa_ai::brain::MindWarded>();
        }
        if let Some(mut caster) = caster {
            let before = (caster.body.item_flags, caster.body.regen_bonus.to_bits());
            let mut next = caster.clone();
            next.apply_worn(&specials);
            if (next.body.item_flags, next.body.regen_bonus.to_bits()) != before {
                *caster = next;
            }
        }
        debug!(
            "armour pass on {entity:?}: rules {:#x}, second wind {}, ward {}, suit {suit}, \
             focus bonus {}",
            rules.0, specials.second_wind, specials.mind_ward, specials.focus_regen
        );
    }
}

/// What the armour pass last saw of a body's equipment: its worn items
/// (ids and flags) and the out set's two item ids. The original runs the
/// pass from the equipment jobs' finalisers and the re-dress, not on every
/// change of the record's lists, so a change of the carried list alone runs
/// none, and a mark written straight onto the body between two passes (the
/// core crystal dropped on a golem's figure clears its coreless mark
/// without wearing it, [`core_shown`]) stands until the body's next
/// equipment change.
#[derive(Component, Debug, Clone, PartialEq, Eq, Default)]
pub struct ArmourPassSeen {
    worn: Vec<(u32, u32)>,
    out: [u32; 2],
}

impl ArmourPassSeen {
    pub fn of(p: &Possessions) -> Self {
        Self {
            worn: p.worn.iter().map(|w| (w.id, w.flags)).collect(),
            out: p.out.map(|h| h.id),
        }
    }
}

/// The coreless mark: the body rule bit a golem race carries while it wears
/// no core crystal ([`exa_formats::character_stats::BodyRules::NO_RED_REGENERATION`]).
/// The armour pass sets it on a golem race and clears it for a worn core
/// ([`exa_formats::character_stats::golem_rules`]); it blocks the golem's
/// wake ([`exa_ai::golem::WAKE_EXEMPT`], the same bit), its rise and its
/// red regeneration.
pub const CORELESS: u32 = exa_formats::character_stats::BodyRules::NO_RED_REGENERATION;

/// The glow parameter of a golem's skin material the take-off fades out.
pub const GLOW_PARAMETER: &str = "go";

/// A core crystal dropped on a golem's figure and shown over it: the body's
/// coreless mark is cleared at once. The record's lists do not take the
/// crystal, so no armour pass runs; the mark stays clear until the body's
/// next equipment change runs one ([`ArmourPassSeen`]). Run on the frame's
/// world (a queued command of the view's drop).
pub fn core_shown(world: &mut World, body: Entity) {
    let Some(mut fighter) = world.get_mut::<exa_combat::Fighter>(body) else {
        return;
    };
    let had = fighter.stats.rules.0 & CORELESS != 0;
    fighter.stats.rules.0 &= !CORELESS;
    info!("core shown over {body:?}: coreless mark cleared (it was {had})");
}

/// The core crystal shown over a golem's figure dragged off it: the body's
/// coreless mark is set, the glow of its skin material fades to 0, and a
/// body that does not rest as a statue (its brain's statue byte) collapses
/// (as a lethal wound starts a collapse, [`crate::mind::runtime::lay_down`]),
/// posted for the next simulation step as the frame's other writes to
/// simulation data are. Run on the frame's world (a queued command of the
/// view's release). [Exarust gap: no live skin material takes the glow
/// fade, which is logged.]
pub fn core_taken_off(world: &mut World, body: Entity) {
    if let Some(mut fighter) = world.get_mut::<exa_combat::Fighter>(body) {
        fighter.stats.rules.0 |= CORELESS;
    }
    let statue = world
        .get::<exa_ai::brain::Npc>(body)
        .is_some_and(|npc| npc.statue);
    info!(
        "core taken off {body:?}: coreless mark set, skin glow `{GLOW_PARAMETER}` to 0 \
         (not faded live){}",
        if statue {
            "; a resting statue does not collapse"
        } else {
            "; it collapses"
        }
    );
    if statue {
        return;
    }
    const WHY: &str = "core taken off";
    match world.get_resource_mut::<exa_physics::sim_app::FixedCommands>() {
        Some(mut fixed) => fixed.run(crate::mind::runtime::lay_down, (body, WHY)),
        None => {
            if let Err(e) =
                world.run_system_cached_with(crate::mind::runtime::lay_down, (body, WHY))
            {
                warn!("{WHY}: {e}");
            }
        }
    }
}

/// An item dragged off a body's equipment panel (the loot window's): when
/// the body, without it, passes the core test for it
/// ([`exa_items::possession::takes_core`]: its race takes the item as its
/// core and it wears no crystal), the core crystal has left it and it reacts
/// as to the figure's take-off ([`core_taken_off`]). Run on the frame's
/// world after the item left the worn list.
pub fn core_left_equipment(world: &mut World, body: Entity, id: u32) {
    let Some(race) = world
        .get::<exa_character::record::CharacterData>(body)
        .map(|data| data.record.race)
    else {
        return;
    };
    if !world.contains_resource::<crate::doll_view::DollItems>() {
        return;
    }
    let worn: Vec<u32> = world
        .get::<Possessions>(body)
        .map(|p| p.worn.iter().map(|w| w.id).collect())
        .unwrap_or_default();
    let flags = world.resource_scope(|world, mut items: Mut<crate::doll_view::DollItems>| {
        items.ensure(world.get_resource::<exa_assets::Game>());
        items.race(race).map(|r| r.flags)
    });
    if flags.is_some_and(|flags| exa_items::possession::takes_core(flags, id & ID_MASK, &worn)) {
        core_taken_off(world, body);
    }
}

/// A body to re-dress from its record, as the loadout form's close and
/// the trade system ask the character view's re-dress for.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Redress(pub Entity);

/// The record word's bits naming a company role (the profession's high
/// nibble, [`crate::hirelings::role_word`]): the loadout form's close
/// re-dresses every level body that carries one.
pub const ROLE_BITS: u32 = 0xF000_0000;

/// The loadout form's close re-dresses the bodies on the roster and
/// specialist pages (not on the hire or event pages): the player first,
/// then every level body whose record word carries a company role
/// ([`ROLE_BITS`]), in the level's order. The close's page is the one the
/// form showed ([`crate::hub_forms::HubForms::closed_loadout`]).
pub fn redress_at_loadout_close(
    windows: Option<ResMut<crate::hub_windows::HubWindows>>,
    players: Query<Entity, With<crate::player::PlayerBody>>,
    bodies: Query<(Entity, &exa_character::record::CharacterData)>,
    mut redress: MessageWriter<Redress>,
) {
    let Some(page) = windows.and_then(|mut w| w.forms.closed_loadout.take()) else {
        return;
    };
    if page != crate::manage::PAGE_ROSTER && page != crate::manage::PAGE_SPECIALIST {
        return;
    }
    let mut count = 0;
    for player in &players {
        redress.write(Redress(player));
        count += 1;
    }
    let mut level: Vec<(Entity, u32)> = bodies
        .iter()
        .filter(|(_, data)| data.record.profession() & ROLE_BITS != 0)
        .map(|(entity, _)| (entity, entity.index().index()))
        .collect();
    level.sort_by_key(|&(_, index)| index);
    for (entity, _) in level {
        redress.write(Redress(entity));
        count += 1;
    }
    info!("loadout form closed on page {page}: {count} bodies re-dressed");
}

/// The character view's re-dress of a world body from its record: the
/// underwear rule on the worn list, the apparel re-dressed (Exarust's
/// outfit follows the worn list through its refit), then the hand models
/// made to show the record's out set: the original swaps the record's
/// other set with its out set, writes into the out set what the hand models
/// hold (a two-hander once) and builds the hand-scene job, which attaches
/// each hand that differs, then swaps back. Exarust's hand models are the
/// body's [`Hands`] and its main hand's [`exa_character::Wielded`], made
/// from the out set ([`hands_of`], Exarust's carried torch kept); a body
/// whose hands already show it changes nothing.
#[allow(clippy::type_complexity)]
pub fn redress_bodies(
    mut commands: Commands,
    mut asks: MessageReader<Redress>,
    catalogue: Option<Res<ItemCatalogue>>,
    (mut items, game): (
        Option<ResMut<crate::doll_view::DollItems>>,
        Option<Res<exa_assets::Game>>,
    ),
    mut bodies: Query<(
        &mut Possessions,
        Option<&exa_character::record::CharacterData>,
        Option<&mut Hands>,
        Has<CarriesTorch>,
        Option<&exa_character::Wielded>,
    )>,
) {
    let catalogue = catalogue.as_deref();
    for &Redress(body) in asks.read() {
        let Ok((mut p, data, hands, torch, wielded)) = bodies.get_mut(body) else {
            continue;
        };
        let mut underwear = false;
        if let Some(items) = items.as_deref_mut() {
            items.ensure(game.as_deref());
            let mut record = exa_character::record::CharacterData::of_record(
                exa_formats::characters::CharacterRecord {
                    worn: p.worn.clone(),
                    race: data.map_or(0, |d| d.record.race),
                    ..Default::default()
                },
            );
            if record.ensure_underwear(|id| items.get(id))
                && let Some(braies) = record.record.worn.last().copied()
            {
                p.worn.push(exa_formats::characters::ItemRef::new(
                    braies.id,
                    braies.flags,
                ));
                underwear = true;
            }
        }
        let mut next = hands_of(&p, |id| catalogue.and_then(|c| c.kind(id)));
        let mut hands_changed = false;
        if let Some(mut hands) = hands {
            if torch {
                next = keep_torch(next, &hands);
            }
            hands_changed = hands.set_if_neq(next);
        }
        let main = p.out[0].id;
        let model = catalogue
            .filter(|_| main != 0)
            .and_then(|c| c.model(main))
            .map(|model| exa_character::Wielded {
                model: model.model.clone(),
                weapon: model.weapon,
            });
        let wielded_changed = model.as_ref() != wielded;
        if wielded_changed {
            match model {
                Some(model) => {
                    commands.entity(body).insert(model);
                }
                None => {
                    commands.entity(body).remove::<exa_character::Wielded>();
                }
            }
        }
        info!(
            "re-dress {body:?}: out {:#x}/{:#x}; hands {}, main model {}{}",
            p.out[0].id,
            p.out[1].id,
            if hands_changed { "changed" } else { "kept" },
            if wielded_changed { "changed" } else { "kept" },
            if underwear { ", the braies put on" } else { "" }
        );
    }
}

/// The eleventh body slot: the armour pass reads a worn piece's display only
/// while walking that node. A body without it still takes the physique offset.
const HEAD_SLOT: usize = 10;

/// The head height the armour pass measured, onto each held weapon, and onto
/// an arm link that is already built. A two-hander or a polearm is the only
/// grip the pass moves. This runs before the grip builder, so a weapon whose
/// links do not exist yet is shifted when they are built. Without a character
/// record the physique is the average (0.5), which adds nothing.
pub fn apply_head_height(
    catalogue: Option<Res<ItemCatalogue>>,
    bodies: Query<(
        &exa_physics::body::Ragdoll,
        Option<&Possessions>,
        Option<&exa_character::record::CharacterData>,
    )>,
    mut held: Query<&mut exa_physics::grip::HeldGrip>,
) {
    let Some(catalogue) = catalogue.as_deref() else {
        return;
    };
    let armed = |grip: &exa_physics::grip::HeldGrip| {
        grip.kind == exa_physics::grip::HeldKind::Weapon
            && grip.class & exa_physics::grip::ARMED_GRIP != 0
    };
    let holders = exa_physics::body::first_by_key(
        held.iter()
            .filter(|grip| armed(grip))
            .map(|grip| grip.holder_key),
        bodies.iter().map(|found| (found, found.0)),
    );
    for mut grip in &mut held {
        if !armed(&grip) {
            continue;
        }
        let Some(&(body, possessions, data)) = holders.get(&grip.holder_key) else {
            continue;
        };
        let physique = data.map_or(0.5, |data| data.record.physique[0]);
        let worn: Vec<Armour> = if body.parts.iter().any(|part| part.bone == HEAD_SLOT) {
            possessions.map_or_else(Vec::new, |possessions| {
                possessions
                    .worn
                    .iter()
                    .filter_map(|worn| catalogue.burden(worn.id & ID_MASK))
                    .map(|burden| Armour {
                        display: burden.display,
                        head_height_mark: burden.head_height_mark,
                        ..Armour::default()
                    })
                    .collect()
            })
        } else {
            Vec::new()
        };
        let extra = head_extra(&worn, physique);
        if grip.head_extra.to_bits() == extra.to_bits() {
            continue;
        }
        let part = grip.spec.arm.map(|pair| pair.part);
        grip.head_extra = extra;
        if let (Some(links), Some(part)) = (grip.links.as_mut(), part) {
            exa_physics::grip::raise_arm(links, body, part, extra);
        }
    }
}

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

/// The hands after a swap, keeping Exarust's carried torch: the torch is
/// outside the record, so the hand it is in keeps it whatever the record's
/// out set holds there.
pub fn keep_torch(mut next: Hands, before: &Hands) -> Hands {
    if let Some(main) = before.main
        && main.light
        && main.flags == crate::torch::TORCH_FLAGS
    {
        next.main = Some(main);
    }
    if before.off_light {
        next.off_light = true;
        next.second = None;
        next.shield = None;
    }
    next
}

/// Applies the plan a swap left on the body: the hands follow the record's
/// out set and the plan is consumed. The plan itself is the record side of
/// the work; the models are attached by the renderer's own draw of the
/// hands, so nothing is spawned here. A body carrying Exarust's torch keeps
/// it in its hand ([`keep_torch`]).
pub fn apply_pending_hands(
    mut commands: Commands,
    catalogue: Option<Res<ItemCatalogue>>,
    mut bodies: Query<(Entity, &Possessions, &mut Hands, Has<CarriesTorch>), With<PendingHands>>,
) {
    for (body, possessions, mut hands, torch) in &mut bodies {
        let mut next = hands_of(possessions, |id| {
            catalogue.as_ref().and_then(|c| c.kind(id))
        });
        if torch {
            next = keep_torch(next, &hands);
        }
        hands.set_if_neq(next);
        commands.entity(body).remove::<PendingHands>();
    }
}

/// The weapon switch's swap point: when the posture's switch arm passes
/// 60 % of the clip it posts the body's weapon-set swap (the view's
/// immediate swap) to the main thread's queue, which runs it after the
/// step ([`exa_combat::condition::WeaponSwapDue`]). The swap is asked for
/// on every body that keeps possessions, the AI's draw as the player's
/// switch; [`exa_items::catalogue::swap_weapon_sets`] makes it this frame.
pub fn swap_at_switch_point(
    mut commands: Commands,
    mut due: MessageReader<exa_combat::condition::WeaponSwapDue>,
    bodies: Query<(), With<Possessions>>,
) {
    for swap in due.read() {
        if bodies.contains(swap.body) {
            commands
                .entity(swap.body)
                .insert(exa_items::catalogue::SwapRequested);
        }
    }
}

/// The record's side of a held weapon coming loose ([`CameLoose`], sent by
/// the grip stage as it frees the item): the holder's came-loose hook runs
/// on its possessions ([`Possessions::forget_loose`]): every out hand that
/// does not hold a shield loses its item and that item's held entry leaves
/// the carried list, and a stowed hand whose item is gone is cleared. The
/// hook runs once per dropped weapon, as the original's drop calls it;
/// a collapse drops its weapons this way. An item the catalogue does not
/// know is not a shield.
///
/// [`CameLoose`]: exa_physics::grip::CameLoose
pub fn forget_loose_hands(
    catalogue: Option<Res<ItemCatalogue>>,
    mut loose: MessageReader<exa_physics::grip::CameLoose>,
    mut bodies: Query<&mut Possessions>,
    (players, mut end_drag): (
        Query<(), With<crate::player::PlayerBody>>,
        Option<ResMut<crate::controls_live::EndGuiDrag>>,
    ),
) {
    let is_shield = |id: u32| {
        catalogue
            .as_ref()
            .and_then(|c| c.kind(id & ID_MASK))
            .is_some_and(|k| k.shield && k.weapon.is_none())
    };
    for came in loose.read() {
        // Something coming loose from the player's hand first ends the
        // GUI's held left button (a held drag ends as on a release).
        if players.contains(came.holder)
            && let Some(end) = end_drag.as_deref_mut()
        {
            end.0 = true;
        }
        let Ok(mut possessions) = bodies.get_mut(came.holder) else {
            continue;
        };
        if !possessions
            .out
            .iter()
            .any(|hand| hand.id != 0 && !is_shield(hand.id))
        {
            continue;
        }
        possessions.forget_loose(is_shield);
        info!(
            "a held weapon came loose: the record's out set is {:?}",
            possessions.out.map(|h| h.id)
        );
    }
}

/// Applies the hand plans of swapped weapon sets, and runs the bodies' worn
/// item effects once a fixed tick.
pub struct EquipPlugin;

impl Plugin for EquipPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<exa_combat::condition::WeaponSwapDue>()
            .add_message::<Redress>()
            .add_systems(
                Update,
                (
                    redress_at_loadout_close.after(crate::thaum::power::teach_on_death),
                    redress_bodies
                        .after(forget_loose_hands)
                        .after(crate::torch::rehand_torches),
                )
                    .chain()
                    .before(refresh_encumbrance)
                    .in_set(exa_physics::FramePhase::Session),
            )
            .add_systems(
                Update,
                swap_at_switch_point
                    .before(exa_items::catalogue::swap_weapon_sets)
                    .in_set(exa_physics::FramePhase::Gui),
            )
            .add_systems(
                Update,
                apply_pending_hands
                    .after(crate::save_live::load_arena_slot)
                    .after(exa_items::catalogue::swap_weapon_sets)
                    .before(refresh_encumbrance)
                    .in_set(exa_physics::FramePhase::Session),
            )
            .add_systems(
                Update,
                refresh_encumbrance
                    .in_set(exa_physics::FramePhase::Session)
                    .after(crate::arena_live::forfeit_match::<false>),
            )
            .add_systems(
                Update,
                refresh_worn_specials
                    .after(refresh_encumbrance)
                    .in_set(exa_physics::FramePhase::Session),
            )
            .add_message::<exa_physics::grip::CameLoose>()
            .add_systems(
                Update,
                forget_loose_hands
                    .after(crate::script_cast::mirror_items)
                    .before(refresh_encumbrance)
                    .in_set(exa_physics::FramePhase::Session),
            )
            .add_systems(
                FixedUpdate,
                // After the bodies' impact class, before the shoves and the
                // behaviour: the first writes of the tick to the bodies.
                apply_head_height
                    .after(exa_audio::ImpactClassSystems)
                    .before(exa_physics::body::deal_shoves)
                    .before(exa_physics::body::TickBehaviour)
                    .before(exa_physics::grip_solve::GripSolveSet::Build),
            )
            .add_systems(
                FixedUpdate,
                crate::item_effects::run_worn_effects
                    .after(exa_combat::regen::HealthTick)
                    .after(exa_physics::grip_solve::GripSolveSet::Build)
                    .after(exa_character::grip_runtime::release_collapsed)
                    .after(crate::thaum::hold_live::grab_thrown)
                    .after(crate::destination::tick_journeys)
                    .before(exa_physics::body::CharacterTicks),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_items::possession::Hand;

    /// A motion set whose action 0x1F is the weapon switch (one key at 10).
    fn switch_motions() -> exa_anim::Motions {
        use exa_formats::rsq::{Action, Clip, Key, MotionSet, Moveset};
        let names: Vec<String> = (0..0x20)
            .map(|i| {
                if i == 0x1F {
                    "Switch Weapons".to_string()
                } else {
                    format!("Action {i}")
                }
            })
            .collect();
        let clip = || Clip {
            mode: 1,
            keys: vec![Key {
                time: 10.0,
                root: [0.0; 3],
                rotations: vec![],
                flags: vec![],
            }],
        };
        let set = MotionSet {
            bone_count: 0,
            actions: names
                .iter()
                .map(|n| Action {
                    name: n.clone(),
                    params: [0, 0],
                })
                .collect(),
            movesets: vec![Moveset {
                name: "Default".into(),
                parent: String::new(),
                clips: names.iter().map(|_| Some(clip())).collect(),
            }],
        };
        exa_anim::Motions::new(
            std::sync::Arc::new(set),
            std::sync::Arc::new(std::collections::HashMap::new()),
            0.0,
        )
    }

    /// The weapon switch's swap point through the live posture stage and the
    /// view's swap: under 60 % of the clip the sets stay; the tick past it
    /// (the switching word lacks bit 4) posts the swap and the frame trades
    /// the sets; the next tick (switched, bit 4 set) posts nothing more.
    #[test]
    fn the_switch_clip_swaps_the_sets_past_sixty_percent() {
        use exa_combat::condition::{WeaponSwapDue, track_condition};
        use exa_physics::body::{Ragdoll, TickedCharacter};
        let mut app = App::new();
        app.init_resource::<ItemCatalogue>()
            .insert_resource(switch_motions())
            .add_message::<WeaponSwapDue>()
            .add_systems(
                Update,
                (
                    track_condition,
                    swap_at_switch_point,
                    exa_items::catalogue::swap_weapon_sets,
                )
                    .chain(),
            );
        let body = app
            .world_mut()
            .spawn((
                exa_combat::Fighter::default(),
                exa_anim::Animator::new("Default", "Switch Weapons"),
                Ragdoll::new(&[]).with_drive(),
                Possessions {
                    out: [Hand::default(), Hand::default()],
                    stowed: [Hand { id: 6, flags: 0 }, Hand::default()],
                    ..Default::default()
                },
            ))
            .id();
        app.insert_resource(TickedCharacter {
            body,
            character: None,
        });
        let at = |app: &mut App, time: f32| {
            app.world_mut()
                .get_mut::<exa_anim::Animator>(body)
                .unwrap()
                .time = time;
            app.update();
            let word = *app.world().get::<exa_combat::Condition>(body).unwrap();
            (
                word,
                app.world().get::<Possessions>(body).unwrap().out[0].id,
            )
        };
        assert_eq!(at(&mut app, 5.9), (exa_combat::Condition::SWITCHING, 0));
        assert_eq!(at(&mut app, 6.1), (exa_combat::Condition::SWITCHED, 6));
        assert_eq!(at(&mut app, 7.0), (exa_combat::Condition::SWITCHED, 6));
    }

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

    /// A catalogue with one worn body (encumbrance byte `worn`) and one
    /// shield whose weight, block and bulk bytes are given.
    fn catalogue(worn: u8, shield: [u8; 3]) -> ItemCatalogue {
        let mut body = vec![0u8; 0x1B0];
        body[0x1A5] = worn;
        let mut shield_body = vec![0u8; 8];
        shield_body[4..7].copy_from_slice(&shield);
        let section = |tag: u32, body: &[u8]| {
            let mut out = Vec::new();
            out.extend_from_slice(&tag.to_le_bytes());
            out.extend_from_slice(&((8 + body.len()) as u32).to_le_bytes());
            out.extend_from_slice(body);
            out
        };
        let first = section(0x1000, &body);
        let second = section(0x8000, &shield_body);
        let mut file = Vec::new();
        file.extend_from_slice(&exa_formats::objdb::MAGIC.to_le_bytes());
        file.extend_from_slice(&48u32.to_le_bytes());
        for (id, offset, len) in [
            (10u32, 0u32, first.len() as u32),
            (20u32, first.len() as u32, second.len() as u32),
        ] {
            file.extend_from_slice(&id.to_le_bytes());
            file.extend_from_slice(&0u32.to_le_bytes());
            file.extend_from_slice(&offset.to_le_bytes());
            file.extend_from_slice(&len.to_le_bytes());
        }
        // The slot after the last entry, which the loader does not count.
        file.extend_from_slice(&[0; 16]);
        file.extend_from_slice(&first);
        file.extend_from_slice(&second);
        let db = exa_formats::rdb::Database::parse(&file).unwrap();
        ItemCatalogue::of_database(&db)
    }

    #[test]
    fn the_encumbrance_follows_the_worn_items_and_the_shield() {
        let catalogue = catalogue(10, [100, 0, 100]);
        let p = Possessions {
            out: [Hand::default(), Hand { id: 20, flags: 0 }],
            worn: vec![exa_formats::characters::ItemRef { id: 10, flags: 0 }],
            ..Default::default()
        };
        // 10 × 0.0025 plus the shield's (2 − 0) × 1 × 0.25.
        assert!((encumbrance_of(&p, &catalogue, Skills::default()) - 0.525).abs() < 1e-6);
        // An unknown worn item weighs nothing, and a weapon in the same
        // hand is not the shield.
        let p = Possessions {
            out: [Hand { id: 10, flags: 0 }, Hand::default()],
            worn: vec![exa_formats::characters::ItemRef { id: 99, flags: 0 }],
            ..Default::default()
        };
        assert_eq!(encumbrance_of(&p, &catalogue, Skills::default()), 0.0);

        // A level-local (tagged) id weighs as its base record, as the
        // spawn's sum looks it up.
        let p = Possessions {
            out: [
                Hand::default(),
                Hand {
                    id: 20 | 0x8000_0000,
                    flags: 0,
                },
            ],
            worn: vec![exa_formats::characters::ItemRef {
                id: 10 | 0x8000_0000,
                flags: 0,
            }],
            ..Default::default()
        };
        assert!((encumbrance_of(&p, &catalogue, Skills::default()) - 0.525).abs() < 1e-6);

        // The system leaves a new body's spawn value and writes a changed
        // body's into its physical stats.
        let mut app = App::new();
        app.insert_resource(catalogue).add_plugins(EquipPlugin);
        let mut spawned = exa_physics::body::Physical::default();
        spawned.stats.encumbrance = 0.25;
        let body = app
            .world_mut()
            .spawn((Possessions::default(), spawned))
            .id();
        app.update();
        let encumbrance = |app: &App| {
            app.world()
                .get::<exa_physics::body::Physical>(body)
                .unwrap()
                .stats
                .encumbrance
        };
        assert_eq!(encumbrance(&app), 0.25);
        {
            let mut entity = app.world_mut().entity_mut(body);
            let mut possessions = entity.get_mut::<Possessions>().unwrap();
            possessions.out[1] = Hand { id: 20, flags: 0 };
            possessions
                .worn
                .push(exa_formats::characters::ItemRef { id: 10, flags: 0 });
        }
        app.update();
        assert!((encumbrance(&app) - 0.525).abs() < 1e-6);
    }

    /// Putting on a worn item of special kind 2 grants the second wind,
    /// taking it off takes it away; a golem's core (an item without an
    /// apparel section) unblocks its red regeneration while it is worn.
    #[test]
    fn the_armour_pass_reruns_its_worn_rules_on_a_change() {
        let mut body = vec![0u8; 0x220];
        // The special kind byte of the item body.
        body[0x214] = exa_formats::objdb::Burden::SECOND_WIND;
        let mut file = Vec::new();
        let mut section = Vec::new();
        section.extend_from_slice(&0x1000u32.to_le_bytes());
        section.extend_from_slice(&((8 + body.len()) as u32).to_le_bytes());
        section.extend_from_slice(&body);
        file.extend_from_slice(&exa_formats::objdb::MAGIC.to_le_bytes());
        file.extend_from_slice(&32u32.to_le_bytes());
        for word in [10u32, 0, 0, section.len() as u32] {
            file.extend_from_slice(&word.to_le_bytes());
        }
        file.extend_from_slice(&[0; 16]);
        file.extend_from_slice(&section);
        let db = exa_formats::rdb::Database::parse(&file).unwrap();
        let catalogue = ItemCatalogue::of_database(&db);
        let mut app = App::new();
        app.insert_resource(catalogue).add_plugins(EquipPlugin);
        let mut fighter = exa_combat::Fighter::default();
        fighter.stats.golem_core = Some(500);
        fighter.stats.rules = exa_formats::character_stats::BodyRules(0x160);
        let e = app
            .world_mut()
            .spawn((Possessions::default(), fighter))
            .id();
        app.update();
        let state = |app: &App| {
            let f = app.world().get::<exa_combat::Fighter>(e).unwrap();
            (f.second_wind.granted, f.stats.rules.0)
        };
        // The spawn's values stand on the first frame.
        assert_eq!(state(&app), (false, 0x160));
        let wear = |app: &mut App, ids: &[u32]| {
            let mut entity = app.world_mut().entity_mut(e);
            let mut possessions = entity.get_mut::<Possessions>().unwrap();
            possessions.worn = ids
                .iter()
                .map(|&id| exa_formats::characters::ItemRef { id, flags: 0 })
                .collect();
        };
        wear(&mut app, &[10, 500]);
        app.update();
        assert_eq!(state(&app), (true, 0x60));
        wear(&mut app, &[]);
        app.update();
        assert_eq!(state(&app), (false, 0x160));
    }

    /// The loadout form closed on the roster or a specialist's page
    /// re-dresses the player and every body with a company role: hands
    /// that do not show the record's out set are made to (here the
    /// catalogue knows no item, so the record's hands are empty); a body
    /// without a role and a close on the hire or event page change nothing.
    #[test]
    fn the_loadout_close_redresses_the_player_and_the_company() {
        let held = || Hands {
            main: Some(HeldItem::weapon(0x521)),
            ..Default::default()
        };
        let run = |page: u32| {
            let mut app = App::new();
            app.init_resource::<ItemCatalogue>()
                .init_resource::<crate::hub_windows::HubWindows>()
                .add_plugins(EquipPlugin);
            let record = |profession: u32| {
                let mut data = exa_character::record::CharacterData::default();
                data.record.set_profession(profession);
                data
            };
            let body = |app: &mut App, extra: Option<u32>| {
                let mut entity = app.world_mut().spawn((
                    Possessions {
                        out: [Hand { id: 6, flags: 0 }, Hand::default()],
                        ..Default::default()
                    },
                    held(),
                ));
                if let Some(profession) = extra {
                    entity.insert(record(profession));
                }
                entity.id()
            };
            let player = body(&mut app, None);
            app.world_mut()
                .entity_mut(player)
                .insert(crate::player::PlayerBody::new(0.0));
            let merchant = body(&mut app, Some(0x4000_0000));
            let stranger = body(&mut app, Some(0x0000_0100));
            app.update();
            {
                let mut windows = app
                    .world_mut()
                    .resource_mut::<crate::hub_windows::HubWindows>();
                windows.forms.arsenal = Some(crate::manage::ArsenalForm::open(page));
                windows.forms.remove(crate::hub_forms::FormWindow::Loadout);
                assert_eq!(windows.forms.closed_loadout, Some(page));
            }
            app.update();
            assert_eq!(
                app.world()
                    .resource::<crate::hub_windows::HubWindows>()
                    .forms
                    .closed_loadout,
                None
            );
            [player, merchant, stranger].map(|b| app.world().get::<Hands>(b).unwrap().is_empty())
        };
        for page in [crate::manage::PAGE_ROSTER, crate::manage::PAGE_SPECIALIST] {
            assert_eq!(run(page), [true, true, false], "page {page}");
        }
        for page in [crate::manage::PAGE_HIRE, crate::manage::PAGE_EVENT] {
            assert_eq!(run(page), [false, false, false], "page {page}");
        }
    }

    /// The AL crystal dragged off a golem's equipment panel (the loot
    /// window's): the golem, which now takes it as its core, is marked
    /// coreless; any other item, or a body still wearing a crystal, is not.
    #[test]
    fn a_core_off_the_loot_equipment_marks_the_golem() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let Ok(data) = exa_assets::GameData::open(&dir) else {
            return;
        };
        let game = exa_assets::Game(std::sync::Arc::new(data));
        let races = exa_formats::races::RaceDb::new(crate::scene_bodies::game_races(&game));
        let golem = (0..=u8::MAX)
            .find(|&b| races.of_character(b).flags & exa_items::possession::TAKES_CORE_AL != 0)
            .expect("the shipped race table has a race that takes the AL crystal");
        let al = exa_items::possession::crystal::AL;
        let mut world = World::new();
        world.insert_resource(game);
        world.init_resource::<crate::doll_view::DollItems>();
        let body = |world: &mut World, worn: &[u32]| {
            let mut fighter = exa_combat::Fighter::default();
            fighter.stats.rules.0 = 0x40;
            world
                .spawn((
                    Possessions {
                        worn: worn
                            .iter()
                            .map(|&id| exa_formats::characters::ItemRef::new(id, 0))
                            .collect(),
                        ..Default::default()
                    },
                    exa_character::record::CharacterData::of_record(
                        exa_formats::characters::CharacterRecord {
                            race: golem,
                            ..Default::default()
                        },
                    ),
                    fighter,
                ))
                .id()
        };
        let rules = |world: &World, e| world.get::<exa_combat::Fighter>(e).unwrap().stats.rules.0;
        let bare = body(&mut world, &[]);
        core_left_equipment(&mut world, bare, 7);
        assert_eq!(rules(&world, bare), 0x40);
        core_left_equipment(&mut world, bare, al);
        assert_eq!(rules(&world, bare), CORELESS | 0x40);
        let still = body(&mut world, &[exa_items::possession::crystal::A]);
        core_left_equipment(&mut world, still, al);
        assert_eq!(rules(&world, still), 0x40);
    }

    #[test]
    fn the_carried_torch_stays_in_its_hand() {
        let torch = HeldItem {
            flags: crate::torch::TORCH_FLAGS,
            light: true,
        };
        let swapped = Hands {
            main: Some(HeldItem::weapon(0x521)),
            shield: Some(ShieldStyle::Plain),
            ..Default::default()
        };
        // In the main hand: the record's weapon does not replace it.
        let before = Hands {
            main: Some(torch),
            ..Default::default()
        };
        let kept = keep_torch(swapped, &before);
        assert_eq!(kept.main, Some(torch));
        assert_eq!(kept.shield, Some(ShieldStyle::Plain));
        // In the off hand: it stays the off hand's light.
        let before = Hands {
            off_light: true,
            ..Default::default()
        };
        let kept = keep_torch(swapped, &before);
        assert!(kept.off_light && kept.shield.is_none());
        assert_eq!(kept.main.unwrap().flags, 0x521);
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

    /// A held weapon coming loose drops the out set's items from its
    /// holder's record and their held entries from the carried list; a
    /// holder with empty hands writes nothing.
    #[test]
    fn a_loose_weapon_leaves_the_record() {
        use exa_items::possession::Carried;
        use exa_physics::grip::CameLoose;
        let mut app = App::new();
        app.init_resource::<ItemCatalogue>()
            .add_plugins(EquipPlugin);
        let held = Carried {
            id: 5,
            flags: 0x100,
            ..Carried::default()
        };
        let armed = app
            .world_mut()
            .spawn(Possessions {
                out: [Hand { id: 5, flags: 0 }, Hand::default()],
                carried: vec![held],
                ..Possessions::default()
            })
            .id();
        let empty = app.world_mut().spawn(Possessions::default()).id();
        app.update();
        for holder in [armed, empty] {
            let item = app.world_mut().spawn_empty().id();
            app.world_mut().write_message(CameLoose { item, holder });
        }
        app.update();
        let p = app.world().get::<Possessions>(armed).unwrap();
        assert_eq!(p.out[0].id, 0);
        assert!(p.carried.is_empty());
        assert_eq!(
            app.world().get::<Possessions>(empty).unwrap(),
            &Possessions::default()
        );
    }
}
