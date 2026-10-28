//! The scene body generator: an arena fighter, a company entrant or a hub
//! NPC built from a spawn plan.
//!
//! A plan is plain data: the record it looks like (or the shipped actor
//! template its record comes from), where it stands, its team and role
//! words, its condition words and the item its main hand shows. The
//! generator builds the body the game's placed characters use: the body
//! model morphed by the record's physique and the garments the dress pass
//! merges around it, its skinned parts and skin look, the held item's
//! model, the physical body whose `Physical` the limp plugin turns into the
//! simulated ragdoll, its record data, its possessions and its brain — the
//! duellist AI for a match opponent, or the placed-character brain for a
//! company entrant or a hub NPC.
//!
//! A body is removed through [`SceneCommand::Despawn`]: its character
//! capsule leaves the physics world first and the root with its whole
//! subtree is despawned, so no collider is left behind.
//!
//! An arena opponent takes a [`BodyLook::Generated`] plan: the spawn writes
//! its own character record with [`crate::arena_records::generated_record`]
//! (the random NPC pass and the outfit tables built from the object
//! database), then dresses the merged mesh from that record. The actor
//! templates (`opponent01`…`opponent16`, `arenashpkp`) remain the Beast's,
//! a company entrant's and the hub specialists' stand-ins where the original
//! reads a saved record or a creature body item Exarust does not keep yet
//! [guess]; the hub shopkeeper is the exact `arenashpkp` template.
//!
//! Evidence: the arena's generic and hire-list spawn routines, the manager
//! and hub placements and the round setup (`arena_spawn`, `match_layout`),
//! plus the character spawn path the level's placed characters use.

use bevy::prelude::*;
use exa_assets::{Game, GameData, ModelCache, ModelLoader, PackKind, SkinLook, model};
use exa_character::held::{Hands, HeldItem};
use exa_formats::characters::{CharacterRecord, ID_MASK};
use exa_formats::objdb::Item;
use exa_formats::races::{Race, RaceBody};
use exa_physics::body::{BodyStats, RaceMuscles};

use crate::arena_outfit::{ItemFacts, Lcg, OutfitTables, ProfessionCatalog};
use crate::arena_records::{GeneratedFighter, generated_record};
use crate::arena_spawn::{
    BeastPlan, GenericPlan, HirePlan, HubBodyPlan, OPPONENT_TEAM, PLAYER_TEAM, SPAWN_DAMAGE,
    SPAWN_HEALTH,
};
use crate::match_layout::{BehaviourReset, OPPONENT_BEHAVIOUR, Place};

/// How far above the given feet point a body's root stands, metres: the
/// placed-character spawn's lift.
pub const BODY_RISE: f32 = 0.05;
/// The template names a generated fighter's stand-in record comes from.
pub const FIGHTER_TEMPLATES: [&str; 16] = [
    "opponent01",
    "opponent02",
    "opponent03",
    "opponent04",
    "opponent05",
    "opponent06",
    "opponent07",
    "opponent08",
    "opponent09",
    "opponent10",
    "opponent11",
    "opponent12",
    "opponent13",
    "opponent14",
    "opponent15",
    "opponent16",
];
/// The hub shopkeeper's shipped actor template.
pub const SHOPKEEPER_TEMPLATE: &str = "arenashpkp";

/// What a plan's record is.
#[derive(Clone, Debug, PartialEq)]
pub enum BodyLook {
    /// A record the plan carries.
    Record(Box<CharacterRecord>),
    /// A shipped actor template, named as `actors.rpk` stores it.
    Actor(String),
    /// A fighter the generator writes a record for from the arena's random
    /// NPC and outfit passes (an arena opponent).
    Generated(GeneratedFighter),
}

/// Which brain drives a scene body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodyBrain {
    /// A match opponent: the duellist AI, which engages after this many
    /// milliseconds (0: at once).
    Duellist { reaction_ms: u32 },
    /// A placed-character brain: a company entrant or a hub NPC.
    Npc,
}

/// One scene body the generator builds.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneBodyPlan {
    /// Where its record comes from.
    pub look: BodyLook,
    /// Its name, overriding the record's when set.
    pub name: Option<String>,
    /// Its character id, as the AI and the report log know it.
    pub id: u32,
    /// Its feet point, metres.
    pub at: Vec3,
    /// Its facing, radians.
    pub yaw: f32,
    /// Its team word.
    pub team: u32,
    /// The role ids added to its roles.
    pub roles: Vec<u32>,
    /// Its roles are cleared before the plan's are added (the arena start
    /// clears a fighter's and adds its own; a hub body keeps its record's).
    pub clear_roles: bool,
    /// Its red health as a share of its maximum, 0…1.
    pub health: f32,
    /// Its first condition word (a generated opponent's damage word).
    pub damage: f32,
    /// Its second condition word (a generated opponent's health word).
    pub condition_health: f32,
    /// The item its main hand shows, replacing the record's (a drawn weapon
    /// variant's item id).
    pub weapon: Option<u32>,
    /// What drives it.
    pub brain: BodyBrain,
    /// Its first action, on the game clock, ms.
    pub act_at_ms: u32,
    /// The behaviour reset the spawn writes, when the caller has one.
    pub behaviour: Option<BehaviourReset>,
}

impl SceneBodyPlan {
    /// A plan from a place (centimetres) with the arena's defaults.
    fn base(place: &Place) -> Self {
        Self {
            look: BodyLook::Record(Box::default()),
            name: None,
            id: 0,
            at: Vec3::from_array(place.position) / 100.0,
            yaw: place.forward[0].atan2(place.forward[2]),
            team: OPPONENT_TEAM,
            roles: Vec::new(),
            clear_roles: true,
            health: 1.0,
            damage: SPAWN_DAMAGE,
            condition_health: SPAWN_HEALTH,
            weapon: None,
            brain: BodyBrain::Duellist { reaction_ms: 0 },
            act_at_ms: 0,
            behaviour: None,
        }
    }

    /// The plan's template name, when its record is an actor.
    #[must_use]
    pub fn actor(&self) -> Option<&str> {
        match &self.look {
            BodyLook::Actor(name) => Some(name),
            BodyLook::Record(_) | BodyLook::Generated(_) => None,
        }
    }
}

/// The template a body without a generated record stands in with: the
/// Beast, a company entrant and a hub specialist, picked from the level and
/// the gear tier so a card of bodies is not one face [guess]. Generated
/// arena opponents no longer use a template; they write their own record.
#[must_use]
pub fn fighter_template(level: i32, tier: i32) -> &'static str {
    let level = level.clamp(0, 5) as usize;
    let tier = tier.clamp(0, 5) as usize;
    FIGHTER_TEMPLATES[(level * 2 + tier) % FIGHTER_TEMPLATES.len()]
}

/// A generated opponent's plan: team 2, the hostile role, the spawn's health
/// and damage words, the duellist brain and the match's action grace.
///
/// `plan`'s level and outfit level select the stand-in template.
#[must_use]
pub fn opponent_plan(
    plan: GenericPlan,
    place: &Place,
    role: u32,
    now_ms: u32,
    act_grace_ms: u32,
) -> SceneBodyPlan {
    let mut body = SceneBodyPlan::base(place);
    // The original writes the fighter's own record: the random NPC pass at
    // the match's level, dressed by the level-derived outfit.
    body.look = BodyLook::Generated(GeneratedFighter::generic(plan));
    body.team = OPPONENT_TEAM;
    body.roles = vec![role];
    // The original's body takes the id its freshly written record gets; with
    // the stand-in template the id is only a non-zero tag the match tick
    // reads for a non-company body, whose falls are not resolved [guess].
    body.id = u32::try_from(plan.level).unwrap_or(0).wrapping_add(1);
    body.damage = plan.damage;
    body.condition_health = plan.health;
    body.act_at_ms = now_ms.wrapping_add(act_grace_ms);
    body.behaviour = Some(OPPONENT_BEHAVIOUR);
    body
}

/// A hire-list opponent's plan: the profile's level byte is the event's gear
/// tier, so it selects the stand-in template; `weapon` is the event row's
/// drawn variant, shown in the main hand.
#[must_use]
pub fn hire_opponent_plan(
    plan: HirePlan,
    place: &Place,
    role: u32,
    weapon: Option<u32>,
    now_ms: u32,
    act_grace_ms: u32,
) -> SceneBodyPlan {
    let mut body = opponent_plan(
        GenericPlan {
            level: plan.stat_level,
            outfit_level: i32::from(plan.outfit.level()),
            warmth: 0.0,
            health: SPAWN_HEALTH,
            damage: SPAWN_DAMAGE,
        },
        place,
        role,
        now_ms,
        act_grace_ms,
    );
    body.look = BodyLook::Generated(GeneratedFighter::hire(plan, weapon));
    body.id = u32::try_from(plan.stat_level).unwrap_or(0).wrapping_add(1);
    body.weapon = weapon;
    body
}

/// The Beast's plan: its team and hostile role at the opponent's place.
///
/// The body item and tier equipment are the `arena_spawn::BeastPlan`'s. The
/// character generator's body-item model is not modelled yet, so the body
/// is the stand-in template until the beast's race and body item are
/// established [guess].
#[must_use]
pub fn beast_scene_plan(
    plan: &BeastPlan,
    place: &Place,
    now_ms: u32,
    act_grace_ms: u32,
) -> SceneBodyPlan {
    let mut body = SceneBodyPlan::base(place);
    body.look = BodyLook::Actor(fighter_template(0, 0).to_string());
    body.team = plan.team;
    body.roles = vec![plan.role];
    body.id = plan.body;
    body.damage = plan.damage;
    body.condition_health = plan.health;
    body.act_at_ms = now_ms.wrapping_add(act_grace_ms);
    body.behaviour = Some(OPPONENT_BEHAVIOUR);
    body
}

/// A company entrant's plan: the player's side, the manager's role word and
/// the match's action grace.
///
/// The entrant's record is the employee's world-database character, which
/// the career does not keep, so the body takes the stand-in template named
/// after the member [guess].
#[must_use]
pub fn company_plan(
    member: Option<(&str, u16)>,
    place: &Place,
    role: u32,
    id: u32,
    now_ms: u32,
    act_grace_ms: u32,
) -> SceneBodyPlan {
    let mut body = SceneBodyPlan::base(place);
    body.look = BodyLook::Actor(fighter_template(0, 0).to_string());
    body.name = member.map(|(name, _)| name.to_string());
    body.team = PLAYER_TEAM;
    body.roles = vec![role];
    body.id = id;
    body.health = member.map_or(1.0, |(_, health)| f32::from(health) / f32::from(u16::MAX));
    body.damage = SPAWN_HEALTH;
    body.condition_health = SPAWN_HEALTH;
    body.brain = BodyBrain::Npc;
    body.act_at_ms = now_ms.wrapping_add(act_grace_ms);
    body
}

/// A hub body's plan from its placement: `arenashpkp` for the shopkeeper,
/// the stand-in template for a specialist.
///
/// The placement's yaw is already in radians (`arena_spawn::yaw_radians`).
#[must_use]
pub fn hub_body_plan(placement: &HubBodyPlan, shopkeeper: bool) -> SceneBodyPlan {
    let forward = [placement.yaw.sin(), 0.0, placement.yaw.cos()];
    let mut body = SceneBodyPlan::base(&Place {
        position: placement.position,
        forward,
    });
    body.look = BodyLook::Actor(if shopkeeper {
        SHOPKEEPER_TEMPLATE.to_string()
    } else {
        fighter_template(0, 0).to_string()
    });
    body.yaw = placement.yaw;
    body.team = PLAYER_TEAM;
    body.roles = placement.roles.to_vec();
    body.clear_roles = false;
    body.damage = placement.damage;
    body.condition_health = placement.health;
    body.brain = BodyBrain::Npc;
    body
}

/// A body the generator placed, as the match and the hub read it.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct SceneFighter {
    /// Its team word.
    pub team: u32,
    /// The company's side (the plan's team is the player's).
    pub company: bool,
    /// Its character id.
    pub id: u32,
    /// The plan's first condition word.
    pub damage: f32,
    /// The plan's second condition word.
    pub condition_health: f32,
    /// Its first action, on the game clock.
    pub act_at_ms: u32,
    /// The behaviour reset the spawn wrote, kept for the systems that will
    /// apply it to a live behaviour state.
    pub behaviour: Option<BehaviourReset>,
}

/// What a spawned body came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpawnedBody {
    /// The character root (the body model is its child).
    pub root: Entity,
    /// The body model the parts hang from.
    pub model: Entity,
}

/// What the generator needs from the running game.
pub struct SpawnContext<'a> {
    /// The game the body model, the skin and the actor templates read.
    pub game: &'a Game,
    pub loader: &'a mut ModelLoader<'a>,
    /// The game's object database, as the spawn resolves items by id.
    pub items: &'a [Item],
    /// The arena's outfit tables, built from the same item facts.
    pub outfits: &'a OutfitTables,
    /// The profession grid the random NPC pass spends experience on.
    pub professions: &'a mut ProfessionCatalog,
    /// The variant index the next random NPC avoids, shared across spawns.
    pub previous_variant: &'a mut u32,
    /// The shipped race table.
    pub races: &'a [Race],
    /// The actor templates, read once and kept across spawns.
    pub actors: &'a mut crate::prelude::ActorCache,
    /// The role database the roles read.
    pub book: &'a exa_formats::roles::RoleDb,
    /// The player the duellists rate.
    pub player: Entity,
    /// The game clock the plan's action time is read against, ms.
    pub now_ms: u32,
    /// The jitter generator the brains draw from, kept across spawns.
    pub roll: &'a mut u32,
}

/// The item a record's reference names in the game's object database.
fn item_of(items: &[Item], id: u32) -> Option<&Item> {
    items.iter().find(|item| item.id == id & ID_MASK)
}

/// Builds a scene body from its plan. `None` when the record's body model
/// cannot be loaded.
///
/// The model carries everything the level's placed characters carry; the
/// root carries the capsule the other characters collide with.
pub fn spawn_scene_body(
    commands: &mut Commands,
    context: &mut SpawnContext<'_>,
    plan: &SceneBodyPlan,
) -> Option<SpawnedBody> {
    let mut record = match &plan.look {
        BodyLook::Record(record) => record.as_ref().clone(),
        BodyLook::Actor(name) => context.actors.template(context.game, name, false)?,
        BodyLook::Generated(fighter) => {
            // The spawned bodies share the arena's sequence: one body's
            // look and outfit draws continue into the next body's.
            let mut lcg = Lcg::new(*context.roll);
            let record = generated_record(
                fighter,
                &mut lcg,
                context.outfits,
                context.professions,
                context.previous_variant,
            );
            *context.roll = lcg.state();
            record
        }
    };
    if let Some(name) = &plan.name {
        record.name = name.clone();
    }
    if plan.id != 0 {
        record.id = plan.id;
    }
    let race = exa_formats::races::of_character(context.races, record.race);
    let model_name = race.map_or_else(|| model::BODY_MODEL.to_string(), |r| r.model());
    let moveset = race.map_or("Default", |r| r.moveset()).to_string();
    let morph = exa_assets::BodyMorph::of_model(
        &model_name,
        exa_formats::morph::Physique::of_record(&record),
    );
    let body = context
        .loader
        .morphed_model(PackKind::Resource, &model_name, Some(&morph))?;
    let (skin_look, skin_composite) = character_skin(
        &context.game.0,
        &model_name,
        &record,
        race.map_or(0, |r| r.look),
        &morph.physique,
        record.id ^ 0x51ED_270B,
    );
    let physical = human_frame(record.race);
    let stats = exa_formats::character_stats::CharacterStats::derive(
        &record,
        race,
        exa_formats::character_stats::Spawn {
            story: false,
            dead: false,
        },
    )
    .with_encumbrance(exa_formats::encumbrance::of_record(&record, |id| {
        item_of(context.items, id)
    }));
    let look = exa_world::record_look(&record, |id| item_of(context.items, id));
    let root = commands
        .spawn((
            Transform::from_translation(plan.at + Vec3::Y * BODY_RISE),
            Visibility::default(),
            Name::new(record.name.clone()),
            exa_world::room_view::RoomTracked,
            exa_world::level_lifecycle::LevelEntity,
            exa_physics::CharacterBody::default(),
        ))
        .id();
    let (model_entity, nodes) = exa_world::spawn_model_nodes(commands, &body, None);
    for (index, &node) in nodes.iter().enumerate() {
        if let Some(bone) =
            exa_formats::rsq::bone_index(&body.nodes[index].name).filter(|&bone| bone >= 1)
        {
            commands.entity(node).insert(exa_anim::BodyPart {
                bone,
                rest: body.nodes[index].transform,
            });
        }
    }
    let mut hands = Hands::default();
    let equip = exa_character::spawn::equip_hands(&record, |hand, r| {
        let item = look.hands.get(hand)?.as_ref()?;
        Some(exa_character::spawn::HandEntry {
            model: item.model.clone(),
            weapon: item.weapon,
            shield: item.shield,
            shield_record: item_of(context.items, r.id).and_then(|i| i.shield),
            torch: item.torch,
        })
    });
    if let Some(item) = &equip[0]
        && item.kind == exa_character::spawn::EquipKind::Weapon
        && let Some(weapon) = item.weapon
    {
        hands.main = Some(HeldItem::weapon(weapon.flags));
    }
    if let Some(item) = &equip[1] {
        match item.kind {
            exa_character::spawn::EquipKind::Shield => {
                hands.shield = Some(exa_character::held::ShieldStyle::Plain);
            }
            exa_character::spawn::EquipKind::Weapon if hands.main.is_some() => {
                if let Some(weapon) = item.weapon {
                    hands.second = Some(HeldItem::weapon(weapon.flags));
                }
            }
            _ => {}
        }
    }
    // The record's hand flag that holds the last weapon flipped: the equip
    // pass builds the grip at the far end of its flip animation.
    if exa_character::spawn::holds_flipped(&equip).unwrap_or(false) {
        commands
            .entity(model_entity)
            .insert(exa_character::HeldFlipped);
    }
    // The off hand's item: a shield from either hand (hand 1's wins), else
    // hand 1's torch (the light path), else a second weapon.
    if let Some(plan) = exa_character::spawn::attach_plan(&equip)
        .into_iter()
        .find(|p| p.kind != exa_character::spawn::Attach::Weapon)
        && let Some(item) = equip[plan.hand].as_ref()
    {
        let kind = match plan.kind {
            exa_character::spawn::Attach::Shield => exa_character::offhand::OffKind::Shield,
            exa_character::spawn::Attach::SecondWeapon => exa_character::offhand::OffKind::Weapon,
            _ => exa_character::offhand::OffKind::Weapon,
        };
        if plan.kind != exa_character::spawn::Attach::Torch {
            commands
                .entity(model_entity)
                .insert(exa_character::offhand::OffHeld {
                    model: item.model.clone(),
                    weapon: item.weapon,
                    shield: item.shield_record,
                    kind,
                });
        }
    }
    let mut fighter = exa_combat::Fighter::of_stats(race, stats).with_perks(record.perks());
    fighter.health.yellow = fighter.health.max * plan.health;
    fighter.health.red = fighter.health.yellow;
    let worn = look
        .worn
        .iter()
        .filter(|_| physical)
        .map(|worn| exa_character::Garment {
            model: worn.model.clone(),
            appearance: worn.appearance,
            hides: Some(worn.hides),
            armour: worn.armour,
        })
        .collect();
    let race_body = exa_formats::races::body_of_character(context.races, record.race);
    commands.entity(model_entity).insert((
        Transform::from_rotation(Quat::from_rotation_y(plan.yaw)),
        exa_anim::Animator::new(&moveset, "Idle"),
        hands,
        exa_character::record::CharacterData::of_record(record.clone()),
        exa_character::Outfit(worn),
        fighter,
        exa_items::possession::Possessions::of_record(&record),
        race.map_or_else(
            exa_character::RaceWeaponry::default,
            exa_character::RaceWeaponry::of_race,
        ),
        morph,
        skin_look,
        exa_assets::BodyModel(model_name.clone()),
        exa_character::hair::Hair {
            style: if race.is_some_and(|r| r.look & exa_formats::races::LOOK_HAIR != 0) {
                record.hair_style
            } else {
                0
            },
            colour: [
                record.hair_colour[0],
                record.hair_colour[1],
                record.hair_colour[2],
            ],
            feral: record.feral,
            light_variant: record.profession_rest & 0x1_0000 != 0,
            head_word: look.worn.iter().fold(0, |word, worn| word | worn.head_word),
        },
        exa_physics::body::Physical {
            race: race_muscles(&race_body),
            stats: body_stats(&stats),
            ..default()
        },
        ChildOf(root),
    ));
    if let Some(job) = skin_composite {
        commands.entity(model_entity).insert(job);
    }
    // The main hand's weapon: the plan's drawn variant replaces the record's
    // item and keeps its own weapon data.
    let weapon = plan
        .weapon
        .and_then(|id| item_of(context.items, id))
        .map(|item| (item.model.clone(), item.weapon))
        .or(look.held);
    if let Some((model, weapon)) = weapon {
        commands
            .entity(model_entity)
            .insert(exa_character::Wielded { model, weapon });
    }
    let mut roles = if plan.clear_roles {
        exa_ai::roles::Roles::default()
    } else {
        exa_ai::roles::Roles::of_record(&record, context.book)
    };
    for role in &plan.roles {
        roles.add(context.book, *role);
    }
    commands.entity(model_entity).insert((
        roles,
        exa_ai::brain::CharacterId(record.id),
        exa_ai::combatant::Combatant::default(),
    ));
    match plan.brain {
        BodyBrain::Duellist { reaction_ms } => {
            let mut duellist = exa_ai::Duellist::new(context.player);
            // The plan's first action is the original's AI clock: an
            // opponent it puts in the future engages then, not on the frame
            // it is built.
            let delay = plan.act_at_ms.saturating_sub(context.now_ms);
            duellist.reaction_ms = reaction_ms.max(delay);
            if duellist.reaction_ms == 0 {
                duellist.engaged = true;
            } else {
                duellist.engage_at_ms = Some(plan.act_at_ms.into());
            }
            commands.entity(model_entity).insert(duellist);
        }
        BodyBrain::Npc => {
            commands.entity(model_entity).insert((
                exa_ai::brain::Npc::of_record(&record, context.roll),
                exa_ai::Mind::new(0.0),
                exa_ai::Duellist::new(context.player),
            ));
            insert_powers(commands, model_entity, &record);
        }
    }
    commands.entity(model_entity).insert(SceneFighter {
        team: plan.team,
        company: plan.team == PLAYER_TEAM,
        id: record.id,
        damage: plan.damage,
        condition_health: plan.condition_health,
        act_at_ms: plan.act_at_ms,
        behaviour: plan.behaviour,
    });
    Some(SpawnedBody {
        root,
        model: model_entity,
    })
}

/// Gives a body its record's powers, as the level's placed characters do.
fn insert_powers(
    commands: &mut Commands,
    fighter: Entity,
    record: &exa_formats::characters::CharacterRecord,
) {
    let Some(caster) = crate::thaum::cast::PowerCaster::of_character(record) else {
        return;
    };
    if caster.known.is_empty() {
        return;
    }
    commands
        .entity(fighter)
        .insert((caster, exa_ai::DuelPowers::default()));
}

/// A character's skin look and the composite job its generated maps come
/// from: the record's tone, look seed, beard and physique layers with the
/// race's look word. `seed` starts the generator that replaces a zero look
/// seed, as the original draws one.
pub fn character_skin(
    game: &GameData,
    model_name: &str,
    record: &CharacterRecord,
    look_word: u32,
    physique: &exa_formats::morph::Physique,
    seed: u32,
) -> (SkinLook, Option<exa_assets::SkinCompositeJob>) {
    let lower = model_name.to_ascii_lowercase();
    let stem = lower.strip_suffix("base.rfc").unwrap_or(&lower).to_string();
    let weights = physique.weights(12);
    let mut state = seed;
    let mut fresh = || {
        state = exa_character::look::random_step(state);
        state
    };
    let exists = |name: &str| game.texture(name).is_some();
    let job = exa_character::texture_job::SkinJob::of_record(
        exa_character::factory::PIXEL_FORMAT,
        &stem,
        record,
        look_word,
        &weights,
        &mut fresh,
        &exists,
    );
    let female = exa_character::look::skin_set(look_word, record.female)
        == exa_character::look::SkinSet::Female;
    (
        SkinLook {
            stem,
            tone: record.skin,
            female,
        },
        job.composite_job(),
    )
}

/// Races with the human frame (human, skeleton, ancient): their bodies take
/// the skeleton's physical links and fit human apparel.
pub fn human_frame(race: u8) -> bool {
    race <= 2
}

/// The factors a race gives its bodies' muscles.
pub fn race_muscles(body: &RaceBody) -> RaceMuscles {
    RaceMuscles {
        stance: body.stance,
        arm_relaxation: body.arm_relaxation,
        balance_bonus: body.balance_bonus,
        damping_ratio: body.damping_ratio,
        clip_rate: body.clip_rate,
        gaits: exa_physics::race_runtime::rows_of(body),
    }
}

/// What a character's stats give its body.
pub fn body_stats(stats: &exa_formats::character_stats::CharacterStats) -> BodyStats {
    BodyStats {
        max_energy: stats.max_energy,
        scale: stats.scale,
        head_gain: stats.head_gain,
        head_factor: stats.head_factor,
        walking_arms: stats.walking_arms,
        pin_factor: stats.pin_factor,
        size: stats.size,
        feral: stats.feral,
        encumbrance: stats.encumbrance,
    }
}

/// A scene body command: what the game asks the generator to do.
#[derive(Message, Clone, Debug, PartialEq)]
pub enum SceneCommand {
    /// Build a body from the plan.
    Spawn(Box<SceneBodyPlan>),
    /// Remove a body, capsule and subtree.
    Despawn(Entity),
    /// Stand a body at a place.
    Place { body: Entity, at: Vec3, yaw: f32 },
}

/// A body was built, with the plan that built it.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct SceneBodyBuilt {
    pub plan: Box<SceneBodyPlan>,
    pub body: SpawnedBody,
}

/// The actor templates and jitter, kept across spawns.
#[derive(Resource, Default)]
pub struct SceneActors {
    pub cache: crate::prelude::ActorCache,
    pub roll: u32,
    /// The profession grid's drift, shared across the process's spawns.
    pub professions: ProfessionCatalog,
    /// The last variant index a random NPC drew.
    pub previous_variant: u32,
}

/// Runs the scene body generator.
///
/// It sits in `Update`, after the physics plugin's character registration, so
/// its commands apply before the frame's body draw. Without the game's packs
/// (`Game`) it drops its commands, which lets a test app schedule the
/// generator without an installation.
pub struct SceneBodiesPlugin;

impl Plugin for SceneBodiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SceneCommand>()
            .add_message::<SceneBodyBuilt>()
            .init_resource::<SceneActors>()
            .add_systems(Update, apply_scene_commands);
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_scene_commands(
    mut commands: Commands,
    mut physics: ResMut<exa_physics::Physics>,
    game: Option<Res<Game>>,
    clock: Option<Res<crate::effects::EffectClock>>,
    mut actors: ResMut<SceneActors>,
    mut cache: ResMut<ModelCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<exa_render::material::ExaMaterial>>,
    mut images: ResMut<Assets<Image>>,
    support: Option<Res<bevy::image::CompressedImageFormatSupport>>,
    players: Query<Entity, With<crate::player::Player>>,
    bodies: Query<Option<&exa_physics::CharacterHandles>>,
    models: Query<&ChildOf, With<SceneFighter>>,
    mut tables: Local<Option<GameTables>>,
    mut messages: MessageReader<SceneCommand>,
    mut built: MessageWriter<SceneBodyBuilt>,
) {
    // Most frames ask for nothing: the game's tables are only read (once)
    // when a command comes.
    if messages.is_empty() {
        return;
    }
    let Some(game) = game else {
        messages.clear();
        return;
    };
    if tables.is_none() {
        *tables = game_items(&game).map(|items| {
            let facts: std::collections::BTreeMap<u32, ItemFacts> = items
                .iter()
                .map(|item| (item.id, ItemFacts::of_item(item)))
                .collect();
            GameTables {
                outfits: OutfitTables::build(|id| facts.get(&id.0).cloned()),
                items,
                races: game_races(&game),
                book: game_roles(&game),
            }
        });
    }
    let Some(GameTables {
        items,
        races,
        book,
        outfits,
    }) = tables.as_ref()
    else {
        messages.clear();
        return;
    };
    let player = players.iter().next();
    for command in messages.read() {
        match command {
            SceneCommand::Spawn(plan) => {
                let Some(player) = player else {
                    warn!("scene bodies: no player to rate");
                    continue;
                };
                let mut loader = ModelLoader {
                    game: &game.0,
                    cache: &mut cache,
                    meshes: &mut meshes,
                    materials: &mut materials,
                    images: &mut images,
                    bc_supported: model::bc_supported(support.as_deref()),
                };
                let SceneActors {
                    cache,
                    roll,
                    professions,
                    previous_variant,
                } = &mut *actors;
                let mut context = SpawnContext {
                    game: &game,
                    loader: &mut loader,
                    items,
                    outfits,
                    professions,
                    previous_variant,
                    races,
                    actors: cache,
                    book,
                    player,
                    now_ms: clock.as_ref().map_or(0, |clock| clock.now_ms),
                    roll,
                };
                if let Some(body) = spawn_scene_body(&mut commands, &mut context, plan) {
                    built.write(SceneBodyBuilt {
                        plan: plan.clone(),
                        body,
                    });
                }
            }
            SceneCommand::Despawn(entity) => {
                // A body is named by its root or by its body model (the
                // entity the match's queries see); the capsule hangs on the
                // root (until the body goes limp), so a model's root goes
                // with it.
                let root = models.get(*entity).map_or(*entity, ChildOf::parent);
                if let Ok(handles) = bodies.get(root) {
                    if let Some(handles) = handles {
                        physics.0.remove_body_with_colliders(handles.body, true);
                    }
                    commands.entity(root).despawn();
                }
            }
            SceneCommand::Place { body, at, yaw } => {
                if bodies.get(*body).is_ok() {
                    commands.entity(*body).insert(
                        Transform::from_translation(*at).with_rotation(Quat::from_rotation_y(*yaw)),
                    );
                }
            }
        }
    }
}

/// The game's tables a spawn reads, parsed once per process.
struct GameTables {
    items: Vec<Item>,
    races: Vec<Race>,
    book: exa_formats::roles::RoleDb,
    outfits: OutfitTables,
}

/// The game's object database as items.
fn game_items(game: &Game) -> Option<Vec<Item>> {
    let bytes = game.0.pack(PackKind::Resource)?.get("objdb.rdb")?;
    let database = exa_formats::rdb::Database::parse(bytes).ok()?;
    Some(exa_formats::objdb::items(&database))
}

/// The game's race table.
fn game_races(game: &Game) -> Vec<Race> {
    game.0
        .pack(PackKind::Resource)
        .and_then(|pack| pack.get("races.rdb"))
        .and_then(|bytes| exa_formats::races::parse(bytes).ok())
        .unwrap_or_default()
}

/// The game's role database.
fn game_roles(game: &Game) -> exa_formats::roles::RoleDb {
    game.0
        .pack(PackKind::Resource)
        .and_then(|pack| pack.get("charroles.rdb"))
        .and_then(|bytes| exa_formats::roles::RoleDb::parse(bytes).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena_spawn::{
        DEGREE, generic_plan, hire_plan, shopkeeper_plan, specialist_plan, yaw_radians,
    };
    use crate::match_layout::{Place, opponent_place};
    use exa_formats::characters::CharacterRecord;

    fn place() -> Place {
        opponent_place([900.0, 0.0, 100.0])
    }

    #[test]
    fn a_generated_opponent_takes_its_team_role_and_words() {
        let plan = opponent_plan(generic_plan(3, 2), &place(), 0x12, 1_000, 3_000);
        assert_eq!(plan.team, 2);
        assert_eq!(plan.roles, vec![0x12]);
        assert!(plan.clear_roles);
        assert_eq!(plan.damage, -100.0);
        assert_eq!(plan.condition_health, 100.0);
        assert_eq!(plan.act_at_ms, 4_000);
        assert_eq!(plan.behaviour, Some(OPPONENT_BEHAVIOUR));
        assert_eq!(plan.at, Vec3::new(9.0, 0.05, -1.0));
        assert!(plan.yaw.abs() < 1e-6);
        assert_eq!(plan.actor(), None);
        let BodyLook::Generated(fighter) = plan.look else {
            panic!("a generic opponent takes a generated record");
        };
        assert_eq!(fighter.level, 3);
        assert_eq!(
            fighter.outfit,
            crate::arena_records::OutfitSource::Level {
                level: 2,
                warmth: (3.0f64 * 0.05) as f32,
                mode_flags: 0
            }
        );
    }

    #[test]
    fn a_hire_opponent_keeps_its_drawn_weapon_and_the_profile_level() {
        let plan = hire_opponent_plan(hire_plan(4, 0, 5), &place(), 0x12, Some(0x1234), 0, 0);
        assert_eq!(plan.weapon, Some(0x1234));
        assert_eq!(plan.id, 7);
        assert_eq!(plan.act_at_ms, 0);
        let unarmed = hire_opponent_plan(hire_plan(4, 0, 0), &place(), 0x12, None, 0, 0);
        assert_eq!(unarmed.weapon, None);
        let BodyLook::Generated(fighter) = unarmed.look else {
            panic!("a hire opponent takes a generated record");
        };
        assert_eq!(
            fighter.outfit,
            crate::arena_records::OutfitSource::Profile(hire_plan(4, 0, 0).outfit)
        );
    }

    #[test]
    fn the_beast_takes_its_item_id_and_role() {
        let beast = crate::arena_spawn::beast_plan(3, 0.5);
        let plan = beast_scene_plan(&beast, &place(), 100, 3_000);
        assert_eq!(plan.team, 2);
        assert_eq!(plan.roles, vec![0x12]);
        assert_eq!(plan.id, 0x1f6);
        assert_eq!(plan.act_at_ms, 3_100);
    }

    #[test]
    fn a_company_entrant_is_the_players_side_with_its_saved_health() {
        let plan = company_plan(Some(("Ash", 32_768)), &place(), 0x11, 3, 0, 3_000);
        assert_eq!(plan.team, 1);
        assert_eq!(plan.roles, vec![0x11]);
        assert_eq!(plan.name.as_deref(), Some("Ash"));
        assert!((plan.health - 0.5).abs() < 1e-4);
        assert!(matches!(plan.brain, BodyBrain::Npc));
        assert_eq!(plan.damage, 100.0);
        let empty = company_plan(None, &place(), 0x11, 1, 0, 3_000);
        assert_eq!(empty.health, 1.0);
        assert_eq!(empty.name, None);
    }

    #[test]
    fn the_hub_places_the_shopkeeper_and_a_specialist_at_their_spots() {
        let shop = hub_body_plan(&shopkeeper_plan(), true);
        assert_eq!(shop.at, Vec3::new(-3.2, 0.0, -1.1));
        assert_eq!(shop.yaw.to_bits(), (15.0f32 * DEGREE).to_bits());
        assert_eq!(shop.roles, vec![0x11, 0x21]);
        assert!(!shop.clear_roles);
        assert_eq!(shop.actor(), Some(SHOPKEEPER_TEMPLATE));
        let physician = specialist_plan([-2700.0, 0.0, 1250.0], 140.0);
        let plan = hub_body_plan(&physician, false);
        assert_eq!(plan.at, Vec3::new(-27.0, 0.0, 12.5));
        assert_eq!(plan.yaw.to_bits(), yaw_radians(140.0).to_bits());
        assert_eq!(plan.roles, vec![0x11, 0x21]);
        assert_eq!(plan.actor(), Some(fighter_template(0, 0)));
        assert_eq!(plan.damage, -100.0);
    }

    #[test]
    fn the_fighter_template_is_bounded_by_level_and_tier() {
        assert_eq!(fighter_template(0, 0), "opponent01");
        assert_eq!(fighter_template(5, 5), FIGHTER_TEMPLATES[15]);
        assert_eq!(fighter_template(-3, -7), "opponent01");
        assert_eq!(fighter_template(99, 99), FIGHTER_TEMPLATES[15]);
    }

    #[test]
    fn a_record_plan_carries_its_own_record() {
        let record = CharacterRecord {
            name: "Named".into(),
            ..CharacterRecord::default()
        };
        let plan = SceneBodyPlan {
            look: BodyLook::Record(Box::new(record.clone())),
            name: Some("Shown".into()),
            ..SceneBodyPlan::base(&place())
        };
        assert_eq!(plan.actor(), None);
        assert_eq!(plan.name.as_deref(), Some("Shown"));
        assert!(matches!(plan.look, BodyLook::Record(_)));
    }

    /// The installed game's body for an opponent is built with the merged
    /// body model, its ragdoll, its fighter, its duellist and the scene
    /// marker; skipped without `EXANIMA_DIR`.
    #[test]
    fn a_real_opponent_body_is_built_from_its_plan() {
        let Some(mut world) = real_world() else {
            return;
        };
        let game = world.remove_resource::<Game>().unwrap();
        let mut cache = world.remove_resource::<ModelCache>().unwrap();
        let mut meshes = world.remove_resource::<Assets<Mesh>>().unwrap();
        let mut materials = world
            .remove_resource::<Assets<exa_render::material::ExaMaterial>>()
            .unwrap();
        let mut images = world.remove_resource::<Assets<Image>>().unwrap();
        let items = game_items(&game).unwrap();
        let facts: std::collections::BTreeMap<u32, ItemFacts> = items
            .iter()
            .map(|item| (item.id, ItemFacts::of_item(item)))
            .collect();
        let outfits = OutfitTables::build(|id| facts.get(&id.0).cloned());
        let races = game_races(&game);
        let book = game_roles(&game);
        let mut actors = crate::prelude::ActorCache::new();
        let mut professions = ProfessionCatalog::default();
        let mut previous_variant = 0;
        let mut roll = 0u32;
        let player = world.spawn(()).id();
        let mut plan = opponent_plan(generic_plan(3, 2), &place(), 0x12, 0, 3_000);
        plan.at = Vec3::new(0.0, 0.0, 0.0);
        let mut loader = ModelLoader {
            game: &game.0,
            cache: &mut cache,
            meshes: &mut meshes,
            materials: &mut materials,
            images: &mut images,
            bc_supported: false,
        };
        let mut commands = world.commands();
        let spawned = {
            let mut context = SpawnContext {
                game: &game,
                loader: &mut loader,
                items: &items,
                outfits: &outfits,
                professions: &mut professions,
                previous_variant: &mut previous_variant,
                races: &races,
                actors: &mut actors,
                book: &book,
                player,
                now_ms: 0,
                roll: &mut roll,
            };
            spawn_scene_body(&mut commands, &mut context, &plan)
        }
        .expect("the actor template's body");
        world.flush();
        assert!(
            world
                .get::<exa_physics::body::Physical>(spawned.model)
                .is_some()
        );
        assert!(
            world
                .get::<exa_physics::body::Ragdoll>(spawned.model)
                .is_none()
        );
        assert!(world.get::<exa_combat::Fighter>(spawned.model).is_some());
        assert!(world.get::<exa_ai::Duellist>(spawned.model).is_some());
        let marker = world.get::<SceneFighter>(spawned.model).unwrap();
        assert_eq!(marker.team, 2);
        assert!(!marker.company);
        assert_eq!(marker.act_at_ms, 3_000);
        assert!(world.get::<exa_character::Outfit>(spawned.model).is_some());
        assert!(
            world
                .get::<exa_character::record::CharacterData>(spawned.model)
                .is_some()
        );
        // The model is a child of the root and the root carries the capsule.
        assert_eq!(
            world
                .get::<ChildOf>(spawned.model)
                .map(|child| child.parent()),
            Some(spawned.root)
        );
        assert!(
            world
                .get::<exa_physics::CharacterBody>(spawned.root)
                .is_some()
        );
        assert!(world.get::<Name>(spawned.root).is_some());
        // The root's subtree goes with it (the ragdoll parts are children of
        // the model, not the root: the body model keeps its parts).
        assert!(world.get_entity(spawned.model).is_ok());
    }

    /// The generator system builds a body from a command in a running app,
    /// with the game's packs; skipped without `EXANIMA_DIR`.
    #[test]
    fn the_generator_system_stands_a_body_from_a_command() {
        let Some(mut app) = real_app() else {
            return;
        };
        app.add_plugins((exa_physics::PhysicsPlugin, SceneBodiesPlugin));
        app.world_mut()
            .spawn((crate::player::Player, Transform::default()));
        let plan = opponent_plan(generic_plan(2, 1), &place(), 0x12, 0, 3_000);
        app.world_mut()
            .write_message(SceneCommand::Spawn(Box::new(plan)));
        for _ in 0..3 {
            app.update();
        }
        let built: Vec<SceneBodyBuilt> = app
            .world_mut()
            .resource_mut::<Messages<SceneBodyBuilt>>()
            .drain()
            .collect();
        assert_eq!(built.len(), 1);
        assert!(
            app.world()
                .get::<exa_physics::body::Physical>(built[0].body.model)
                .is_some()
        );
        // The capsule was registered on the root.
        let capsules = app
            .world()
            .resource::<exa_physics::Physics>()
            .0
            .bodies
            .len();
        assert!(
            app.world()
                .get::<exa_physics::CharacterHandles>(built[0].body.root)
                .is_some()
        );
        // A despawn by the body model (what the match's queries see)
        // removes the root, its capsule and the body with it.
        app.world_mut()
            .write_message(SceneCommand::Despawn(built[0].body.model));
        app.update();
        assert_eq!(
            app.world()
                .resource::<exa_physics::Physics>()
                .0
                .bodies
                .len(),
            capsules - 1
        );
        assert!(app.world().get_entity(built[0].body.root).is_err());
        assert!(app.world().get_entity(built[0].body.model).is_err());
    }

    /// An app with the game's packs and the asset registries, when
    /// `EXANIMA_DIR` names an installation.
    fn real_app() -> Option<App> {
        let dir = exa_assets::locate_install(None)?;
        let data = exa_assets::GameData::open(&dir).ok()?;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<exa_render::material::ExaMaterial>()
            .init_asset::<Image>()
            .init_resource::<ModelCache>()
            .insert_resource(Game(std::sync::Arc::new(data)));
        Some(app)
    }

    /// A world with the game's packs and the asset registries, when
    /// `EXANIMA_DIR` names an installation.
    fn real_world() -> Option<World> {
        let dir = exa_assets::locate_install(None)?;
        let data = exa_assets::GameData::open(&dir).ok()?;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<exa_render::material::ExaMaterial>()
            .init_asset::<Image>()
            .init_resource::<ModelCache>()
            .insert_resource(Game(std::sync::Arc::new(data)));
        app.update();
        Some(std::mem::take(app.world_mut()))
    }
}
