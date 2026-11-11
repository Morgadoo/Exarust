//! Item abilities: a held item whose weapon section names an ability, and
//! what each ability does while its crystal sits in the item's first
//! socket.
//!
//! A weapon section's body carries an ability type (the word at byte
//! [`ABILITY_AT`]) and, from byte [`SOCKET_AT`], its crystal sockets: the
//! first socket's kind in the low byte and the crystal's charge in the high
//! one ([`exa_items::item_gui::Sockets`]). In the shipped object database
//! four items have a type: 1 "UB5 FlangedMace", 2 "Swrd Self Sharp", 3
//! "Sword GSF 01" and 4 "Sceptre A" (type 4 installs nothing). The sockets
//! are filled through the item card's socket row ([`crate::socket_drag`]).
//!
//! **The list.** When the hands are dressed the original registers one
//! check per held item of type 1, 2 or 3 on the character's item-effect
//! list; every game second the body tick runs the list, whose counter
//! advances on each run. A check whose item is no longer in a hand is
//! dropped. Exarust scans every character's two hand slots once a second
//! instead ([`ability_tick`]), from the live object database
//! ([`LiveObjects`]), and counts the runs per character ([`AbilityClock`]).
//!
//! **Type 1, the lightning mace** ([`lightning_charged`]): a first socket
//! of kind 1 (Power Crystal A) with a charge below 0xfe, or of kind 0x11
//! (Power Crystal AR) with a charge below 0x7f, raises the mace's hum arcs
//! ([`crate::thaum::arcs::hum_arcs`]) on a node 53 cm along the item's +Y
//! when none are live; a spent socket ends them. While they are up, a hit
//! with the mace shocks the struck character with zero strength
//! ([`crate::thaum::live::ArcShocked`]), then charges the socket by 10 (by
//! 0x80 for a kind with bit 0x10) until it would reach 0xff, where it is
//! set to 0xff and the arcs end, and leaves a spark burst at the contact.
//!
//! **Type 2, the self-sharpening blade** ([`item_effects::sharpen_tick`]):
//! on every run whose counter's low three bits are all set, a first socket
//! of kind 2 (Power Crystal B) with a charge below 0xff and a slash byte
//! (weapon byte 0x37) below 0x8c gains one in the charge and two in the
//! slash byte, and the weapon record's slash share (W11) is rewritten from
//! the slash byte. Every hit with the blade blunts it
//! ([`item_effects::blunt_on_hit`]): a slash byte above 0x40 loses four,
//! otherwise it becomes 0x3c, and the share follows.
//!
//! **Type 3, the burning blade** ([`fire_row::held_tick`]): lights the
//! blade when its first socket holds a Power Crystal C (kind 4) whose
//! charge byte is below 254 and no live flame row is on it, and puts a
//! burning blade out once the crystal is gone. Nothing drains a kind-4
//! crystal, so a socketed blade burns until the crystal is taken out.
//!
//! - **Lighting** raises the item's lit flag ([`LitBlade`]), puts a flame
//!   row on the item along its own +Y from 30 to 115 cm, fading in
//!   ([`fire_row::ignition`]), chooses the moveset again with the
//!   "Flaming" overlay ([`HeldModifier`]), raises the body's lit-blade
//!   byte ([`BurningGrip`]), sets the grip mode again (a burning two-hander
//!   at rest keeps both hands on its grip) and re-runs the player light's
//!   switch, which a lit blade in the first hand turns off.
//! - **Putting out** sets the row's extinguish flag (it fades out and leaves
//!   the scene), clears the lit flag and re-runs the switch. It does not
//!   choose the moveset again: the overlay and the byte stay until the
//!   hands are dressed again.
//! - **A hit** with a burning blade plays `FireSizzleA` at the contact,
//!   volume 0.3 and pitch `0.8 + 0.2·u` ([`fire_row::sizzle`]).
//!
//! The row's light (warm white, 800 cm, softness 4·j, five at full fade)
//! sits at the middle of the line, its loop `FireS02` follows the row's
//! speed and fade, and its ribbon is drawn with `_FlameMat01`
//! ([`exa_render::flame_row`]) over the `flameletsa3` picture the material's
//! built-in record names.
//!
//! Debug tool: `--ability-item N[,cold][,npc]` (the binary's flag; `--flame-sword`
//! is `--ability-item 3`) gives the player the type-N item and drops its
//! crystal (A, B or C) into the first socket through the item GUI's socket
//! rule; `cold` leaves the socket empty, `npc` also puts the item in the
//! hands of every other character that holds its model, `npc-only` only
//! there.
//!
//! Evidence: the held-item ability install, the list's run, the three
//! checks, the arcs' raise and end, the lightning hit hook and its
//! deferred charge, the self-sharpening check and hit hook, the burning
//! blade's lighting and putting out, the hit callback, the flame row's
//! node, the moveset choice's lit-blade byte and the two-handed grip
//! callback that reads it, the item GUI's socket drop and the player
//! light's switch, read from `Exanima.exe` and checked against the
//! disassembly.

use std::collections::HashMap;

use bevy::camera::visibility::NoFrustumCulling;
use bevy::prelude::*;
use exa_assets::live_objects::LiveObjects;
use exa_assets::{ColorSpace, Game, ModelCache, ModelLoader};
use exa_audio::{AudioEnvironment, SoundBank, VoiceHandle, voice_flags};
use exa_character::grip_runtime::{HeldAbility, HeldRecord};
use exa_character::held::{BurningGrip, HeldModifier};
use exa_combat::attack::Slot;
use exa_combat::{Blade, Hit};
use exa_formats::rsq::MovesetConfig;
use exa_formats::weapon_stats::at;
use exa_items::possession::Possessions;
use exa_physics::grip::HeldGrip;
use exa_render::flame_row::{FlameRowMaterial, gradient_image};

use crate::effects::fire_row::{self, AttachedRow, Flame, HeldFire, ItemAbility};
use crate::item_effects;
use crate::player::PlayerBody;
use crate::thaum::arcs::{HUM_ARCS_OFFSET, hum_arcs, spark_burst};
use crate::thaum::live::{ArcShocked, FrameEffectDraws, LiveArcs};

/// The weapon section's tag.
pub const WEAPON_SECTION: u32 = 0x4000;
/// The ability type's word in the weapon section's body.
pub const ABILITY_AT: usize = 0x44;
/// The first socket's word (kind, then charge) in the weapon section's body.
pub const SOCKET_AT: usize = 0x48;
/// The ability types.
pub const LIGHTNING: u32 = 1;
pub const SELF_SHARPENING: u32 = 2;
pub const BURNING_BLADE: u32 = fire_row::BURNING_ABILITY;
/// How often the held items' abilities are checked (ms of game time).
pub const TICK_MS: u64 = 1000;
/// The three ability items of the shipped database and their models:
/// "UB5 FlangedMace" (a factory mace), "Swrd Self Sharp" and "Sword GSF
/// 01", the one burning blade.
pub const LIGHTNING_MACE: u32 = 0x1D3;
pub const SELF_SHARPENING_SWORD: u32 = 0x433;
pub const FLAME_SWORD: u32 = 0x487;
pub const LIGHTNING_MACE_MODEL: &str = "@UBM Mace1H S1";
pub const SELF_SHARPENING_MODEL: &str = "SwrdOrgSlfShA";
pub const FLAME_SWORD_MODEL: &str = "Sword GSF 01";
/// The flame row's material picture (`_FlameMat01`'s built-in record).
pub const FLAME_TEXTURE: &str = "flameletsa3";
/// Where along the item's +Y the row's light sits (cm): the line's middle.
pub const LIGHT_AT_CM: f32 = (fire_row::START_CM + fire_row::END_CM) * 0.5;
/// The lightning hit's shock strength.
pub const LIGHTNING_SHOCK: f32 = 0.0;
/// What a lightning hit adds to the socket's charge: 10, or 0x80 for a
/// kind with bit 0x10 (Power Crystal AR).
pub const LIGHTNING_DRAIN: u8 = 10;
pub const LIGHTNING_DRAIN_AR: u8 = 0x80;

pub struct ItemAbilityPlugin;

impl Plugin for ItemAbilityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AbilityClock>()
            .init_resource::<FlameRowVoices>()
            .add_systems(
                Update,
                (
                    debug_ability_item,
                    ability_tick,
                    check_focus_items,
                    hit_hooks,
                    (flame_row_lights, flame_row_draws, flame_row_loops)
                        .after(fire_row::frame_rows),
                )
                    .chain(),
            )
            .add_systems(
                FixedUpdate,
                follow_items
                    .after(exa_physics::body::BodySystems)
                    .before(crate::thaum::live::tick_arcs),
            );
    }
}

/// The focus item's ability type: an item whose weapon section names it
/// gives its holder the focus-item slot bit.
pub const FOCUS_ABILITY: u32 = item_effects::FOCUS_ITEM_KIND as u32;

/// The focus-item check, run when what a caster holds in its hands
/// changes: either hand's weapon section naming ability type 4 sets the
/// body's slot bit, and the bit going tells the running powers past the
/// slots to end ([`crate::thaum::cast::PowerCaster::take_focus_slot`]).
/// [inf: the original's check is called from the hands' item changes; it
/// runs here when the held ids differ from the last frame's, and once for
/// a caster first seen.]
pub(crate) fn check_focus_items(
    mut casters: Query<(Entity, &Possessions, &mut crate::thaum::cast::PowerCaster)>,
    objects: Option<Res<LiveObjects>>,
    mut held: Local<HashMap<Entity, [u32; 2]>>,
) {
    let Some(objects) = objects else {
        return;
    };
    held.retain(|entity, _| casters.contains(*entity));
    for (entity, possessions, mut caster) in &mut casters {
        let ids = [possessions.out[0].id, possessions.out[1].id];
        if held.get(&entity) == Some(&ids) {
            continue;
        }
        held.insert(entity, ids);
        let kinds = ids.map(|id| {
            (id != 0)
                .then(|| objects.db().section(id, WEAPON_SECTION))
                .flatten()
                .and_then(ability_of)
                .map(|a| a.kind as i32)
        });
        let slot = item_effects::focus_slot(caster.body.item_flags, kinds);
        let ended = caster.take_focus_slot(slot);
        debug!(
            "focus item check: {entity:?} holds {ids:?}, slot word {:#x}, {} slots{}",
            slot.word,
            caster.body.slots,
            if ended {
                ", running powers past them told to end"
            } else {
                ""
            }
        );
    }
}

/// An item's ability: its type word and its first socket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ability {
    pub kind: u32,
    pub socket: ItemAbility,
}

/// The ability a weapon section's body names, if it is long enough to hold
/// one.
pub fn ability_of(body: &[u8]) -> Option<Ability> {
    let kind = u32::from_le_bytes(body.get(ABILITY_AT..ABILITY_AT + 4)?.try_into().ok()?);
    let word = u16::from_le_bytes(body.get(SOCKET_AT..SOCKET_AT + 2)?.try_into().ok()?);
    Some(Ability {
        kind,
        socket: ItemAbility {
            kind: u32::from(word),
            fuel: (word >> 8) as u8,
        },
    })
}

/// Whether the install registers a check for an ability type: 1, 2 and 3.
pub fn installs(kind: u32) -> bool {
    item_effects::hand_effect(kind as i32).is_some()
}

/// The item's render lit flag: its flame row burns. Kept on the held item
/// (the [`Blade`]) with the row it carries.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LitBlade {
    pub row: Entity,
}

/// The lightning mace's live hum arcs, kept on the held item.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct HumArcs {
    pub arcs: Entity,
}

/// Arcs that ride a held item, `offset` cm along its own frame.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct ItemArcs {
    pub item: Entity,
    pub offset: Vec3,
}

/// The light a flame row shows, at the middle of its line.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlameRowLight {
    pub row: Entity,
}

/// A flame row's ribbon draw.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlameRowDraw {
    pub row: Entity,
}

/// The ability list's clock, as the original keeps one per character: the
/// game time (ms) the next run is due, and how many runs the list made.
#[derive(Resource, Debug, Default)]
pub struct AbilityClock {
    pub next_ms: HashMap<Entity, u64>,
    pub runs: HashMap<Entity, i32>,
}

/// Debug tool (`--ability-item`): the player holds the type-`kind` item,
/// with its crystal in the first socket unless `cold`; with `npcs`, every
/// other character holding its model holds it too (and with `player`
/// false, only they do). Applied once the player
/// and the live database are there.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DebugAbilityItem {
    pub kind: u32,
    pub cold: bool,
    pub npcs: bool,
    /// The player holds it too.
    pub player: bool,
    pub applied: bool,
}

impl DebugAbilityItem {
    pub fn new(kind: u32, cold: bool, npcs: bool) -> Self {
        Self {
            kind,
            cold,
            npcs,
            player: true,
            applied: false,
        }
    }

    /// The item, its model and the crystal its ability takes.
    pub fn item(kind: u32) -> Option<(u32, &'static str, u32)> {
        use exa_items::possession::crystal;
        match kind {
            LIGHTNING => Some((LIGHTNING_MACE, LIGHTNING_MACE_MODEL, crystal::A)),
            SELF_SHARPENING => Some((SELF_SHARPENING_SWORD, SELF_SHARPENING_MODEL, crystal::B)),
            BURNING_BLADE => Some((FLAME_SWORD, FLAME_SWORD_MODEL, crystal::C)),
            _ => None,
        }
    }
}

/// What the burning check does to one held item this tick.
pub fn check(ability: Option<Ability>, row: AttachedRow) -> HeldFire {
    match ability {
        Some(a) if a.kind == BURNING_BLADE => fire_row::held_tick(Some(a.socket), row).action,
        _ => HeldFire::None,
    }
}

/// The lightning check's charge test on the first socket's word: kind 1
/// with a charge below 0xfe, or kind 0x11 with a charge below 0x7f (the
/// kind is the word's low five bits).
pub fn lightning_charged(word: u16) -> bool {
    let charge = (word >> 8) as u8;
    match word & 0x1f {
        1 => charge < 0xfe,
        0x11 => charge < 0x7f,
        _ => false,
    }
}

/// What the lightning check does: raise the arcs when charged and none are
/// live, end live arcs when spent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArcsAction {
    None,
    Raise,
    End,
}

pub fn lightning_check(word: u16, live: bool) -> ArcsAction {
    match (lightning_charged(word), live) {
        (true, false) => ArcsAction::Raise,
        (false, true) => ArcsAction::End,
        _ => ArcsAction::None,
    }
}

/// A lightning hit's charge on the socket word: 10 (0x80 for a kind with
/// bit 0x10) is added while the sum stays below 0xff; otherwise the charge
/// becomes 0xff and the arcs end (`true`).
pub fn lightning_hit_charge(word: u16) -> (u16, bool) {
    let step = if word & 0x10 == 0 {
        LIGHTNING_DRAIN
    } else {
        LIGHTNING_DRAIN_AR
    };
    let charge = (word >> 8) as u8;
    if i32::from(charge) < 0xff - i32::from(step) {
        ((word & 0xff) | u16::from(charge + step) << 8, false)
    } else {
        ((word & 0xff) | 0xff00, true)
    }
}

/// Whether the player light's lit-blade rule puts it out: an item of type 3
/// in either hand while the first held item burns.
pub fn blade_puts_light_out(kinds: [Option<u32>; 2], first_lit: bool) -> bool {
    kinds
        .iter()
        .any(|&k| crate::player_light::weapon_puts_out(k.map(|k| k as u8), first_lit))
}

/// Writes a socket word into an item's weapon section.
fn write_socket(objects: &mut LiveObjects, id: u32, word: u16) {
    let [low, high] = word.to_le_bytes();
    objects.write_section_byte(id, WEAPON_SECTION, SOCKET_AT, low);
    objects.write_section_byte(id, WEAPON_SECTION, SOCKET_AT + 1, high);
}

#[allow(clippy::type_complexity)]
fn debug_ability_item(
    debug: Option<ResMut<DebugAbilityItem>>,
    game: Option<Res<Game>>,
    mut characters: Query<(
        Entity,
        Option<&mut Possessions>,
        Option<&exa_character::Wielded>,
        Has<PlayerBody>,
        Has<exa_character::record::CharacterData>,
    )>,
    objects: Option<ResMut<LiveObjects>>,
    mut given: Local<Vec<Entity>>,
    mut commands: Commands,
) {
    let (Some(mut debug), Some(game), Some(mut objects)) = (debug, game, objects) else {
        return;
    };
    let Some((id, model, crystal)) = DebugAbilityItem::item(debug.kind) else {
        return;
    };
    if !debug.applied {
        if !characters
            .iter()
            .any(|(_, p, _, player, _)| player && p.is_some())
            || !objects.ensure_base(&game.0)
        {
            return;
        }
        debug.applied = true;
        let Some(body) = objects.db().section(id, WEAPON_SECTION) else {
            warn!("--ability-item: \"{model}\" has no weapon section");
            return;
        };
        let mut sockets = exa_items::item_gui::Sockets::of_section(body, SOCKET_AT);
        let dropped = if debug.cold {
            None
        } else {
            Some(sockets.drop_crystal(0, crystal, 0, false))
        };
        write_socket(&mut objects, id, sockets.0[0]);
        let ability = objects
            .db()
            .section(id, WEAPON_SECTION)
            .and_then(ability_of);
        info!(
            "--ability-item: {id:#x} \"{model}\", crystal {crystal} into its first socket ({dropped:?}): {ability:?}"
        );
    }
    if objects.db().section(id, WEAPON_SECTION).is_none() {
        return;
    }
    for (entity, possessions, wielded, player, recorded) in &mut characters {
        // A character with a record gets its possession list from it; wait
        // for that.
        if given.contains(&entity) || (possessions.is_none() && (player || recorded)) {
            continue;
        }
        let holds_model = wielded.is_some_and(|w| w.model.eq_ignore_ascii_case(model));
        if !((player && debug.player) || (!player && debug.npcs && holds_model)) {
            continue;
        }
        given.push(entity);
        // A debug duellist keeps no possession list: it gets one holding
        // the item.
        match possessions {
            Some(mut possessions) => possessions.out[0].id = id,
            None => {
                let mut possessions = Possessions::default();
                possessions.out[0].id = id;
                commands.entity(entity).insert(possessions);
            }
        }
        info!(
            "--ability-item: {entity:?} holds {id:#x} \"{model}\"{}",
            if player { " (the player)" } else { "" }
        );
    }
}

/// The row a held item carries and whether it is on its way out.
fn attached_row(lit: Option<&LitBlade>, rows: &Query<&mut Flame>) -> AttachedRow {
    match lit.and_then(|l| rows.get(l.row).ok()) {
        Some(flame) => AttachedRow {
            exists: true,
            extinguishing: flame.row().flags & fire_row::EXTINGUISHING != 0,
        },
        None => AttachedRow::NONE,
    }
}

/// A held item's place in the tick: the character's blade in the slot and
/// what it carries.
type BladeItem<'a> = (
    Entity,
    &'a mut Blade,
    Option<&'a LitBlade>,
    Option<&'a HumArcs>,
    Option<&'a mut HeldRecord>,
    Option<&'a mut HeldGrip>,
    Option<&'a HeldAbility>,
);

/// Once a game second, every character's held items' ability checks.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn ability_tick(
    mut commands: Commands,
    mut clock: ResMut<AbilityClock>,
    time: Option<Res<Time<Virtual>>>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    mut characters: Query<(
        Entity,
        &Possessions,
        Option<&mut HeldModifier>,
        Option<&mut BurningGrip>,
    )>,
    objects: Option<ResMut<LiveObjects>>,
    mut blades: Query<BladeItem>,
    mut rows: Query<&mut Flame>,
    mut arcs: Query<&mut LiveArcs>,
    draws: Option<ResMut<FrameEffectDraws>>,
    mut seed: Local<u32>,
) {
    let (Some(time), Some(mut objects), Some(mut draws)) = (time, objects, draws) else {
        return;
    };
    if loading.is_some() {
        return;
    }
    let now = time.elapsed().as_millis() as u64;
    for (character, possessions, modifier, burning_grip) in &mut characters {
        let next = clock.next_ms.entry(character).or_insert(0);
        if now < *next {
            continue;
        }
        *next = now + TICK_MS;
        let held = [0, 1].map(|slot| {
            let id = possessions.out[slot].id;
            (id != 0)
                .then(|| objects.db().section(id, WEAPON_SECTION))
                .flatten()
                .and_then(ability_of)
                .filter(|a| installs(a.kind))
                .map(|a| (id, a))
        });
        // Nothing held has an ability: the list is empty and does not run.
        if held.iter().all(Option::is_none) {
            continue;
        }
        let runs = clock.runs.entry(character).or_insert(0);
        *runs = runs.wrapping_add(1);
        let run = *runs;
        let mut flaming = modifier.as_ref().is_some_and(|m| m.0 != 0);
        let mut lit_byte = burning_grip.as_ref().is_some_and(|b| b.0);
        for (slot, entry) in held.into_iter().enumerate() {
            let Some((id, ability)) = entry else {
                continue;
            };
            let want = if slot == 0 { Slot::First } else { Slot::Second };
            // A check whose item is no longer in a hand drops out.
            let Some((blade, mut record, lit, hum, held_record, grip, known)) = blades
                .iter_mut()
                .find(|(_, b, ..)| b.wielder == character && b.slot == want)
            else {
                continue;
            };
            if known.is_none_or(|k| k.0 != ability.kind) {
                commands.entity(blade).insert(HeldAbility(ability.kind));
            }
            match ability.kind {
                LIGHTNING => {
                    let live = hum.is_some_and(|h| arcs.get(h.arcs).is_ok_and(|a| !a.arcs.ending));
                    match lightning_check(ability.socket.kind as u16, live) {
                        ArcsAction::None => {}
                        ArcsAction::Raise => {
                            let arcs_entity = commands
                                .spawn((
                                    Name::new("_MaceArcs"),
                                    LiveArcs {
                                        arcs: hum_arcs(&mut draws.0.0),
                                        hand: None,
                                        weapon: None,
                                        caster: Some(character),
                                        keep: false,
                                    },
                                    ItemArcs {
                                        item: blade,
                                        offset: HUM_ARCS_OFFSET,
                                    },
                                ))
                                .id();
                            commands.entity(blade).insert(HumArcs { arcs: arcs_entity });
                            info!(
                                "item ability: {character:?} slot {slot} {id:#x} raises its arcs (socket {:#06x}, arcs {arcs_entity:?})",
                                ability.socket.kind
                            );
                        }
                        ArcsAction::End => {
                            if let Some(hum) = hum {
                                end_arcs(&mut arcs, hum.arcs);
                            }
                            commands.entity(blade).remove::<HumArcs>();
                            info!(
                                "item ability: {character:?} slot {slot} {id:#x} ends its arcs (socket {:#06x})",
                                ability.socket.kind
                            );
                        }
                    }
                }
                SELF_SHARPENING => {
                    let Some(body) = objects.db().section(id, WEAPON_SECTION) else {
                        continue;
                    };
                    let mut slash = body.get(at::SLASH).copied().unwrap_or(0);
                    let word = ability.socket.kind as u16;
                    let mut charge = (word >> 8) as u8;
                    let before = (slash, charge);
                    let tick = item_effects::sharpen_tick(true, word, &mut charge, &mut slash, run);
                    if (slash, charge) != before {
                        objects.write_section_byte(id, WEAPON_SECTION, at::SLASH, slash);
                        objects.write_section_byte(id, WEAPON_SECTION, SOCKET_AT + 1, charge);
                    }
                    if let Some(share) = tick.slash_share {
                        set_slash(&mut record, held_record, slash, share);
                        info!(
                            "item ability: {character:?} slot {slot} {id:#x} sharpens on run {run}: slash {} -> {slash}, charge {} -> {charge}, W11 {share:.4}",
                            before.0, before.1
                        );
                    }
                }
                BURNING_BLADE => {
                    let row = attached_row(lit, &rows);
                    match check(Some(ability), row) {
                        HeldFire::None => {}
                        HeldFire::Ignite => {
                            *seed = seed.wrapping_add(1);
                            let ignition = fire_row::ignition();
                            let row = commands
                                .spawn((
                                    Name::new("FlameRow"),
                                    Flame::new(
                                        ignition.start_cm,
                                        ignition.end_cm,
                                        ignition.flags,
                                        *seed,
                                    ),
                                    Transform::IDENTITY,
                                    ChildOf(blade),
                                ))
                                .id();
                            commands.spawn((
                                Name::new("FlameRowLight"),
                                FlameRowLight { row },
                                exa_world::light_bundle(&fire_row::light_record()),
                                Transform::from_xyz(0.0, LIGHT_AT_CM / 100.0, 0.0),
                                ChildOf(blade),
                            ));
                            commands.entity(blade).insert(LitBlade { row });
                            // The moveset is chosen again with the overlay
                            // (which clears the byte), then the byte is
                            // raised and the grip mode set again.
                            flaming = true;
                            lit_byte = true;
                            if let Some(mut grip) = grip
                                && let Some(links) = grip.links.as_mut()
                            {
                                links.mode = -1;
                            }
                            info!(
                                "item ability: {character:?} slot {slot} {id:#x} lights its blade (socket {:#06x}, flame row {row:?})",
                                ability.socket.kind
                            );
                        }
                        HeldFire::Extinguish => {
                            if let Some(lit) = lit
                                && let Ok(mut flame) = rows.get_mut(lit.row)
                            {
                                flame.extinguish();
                            }
                            commands.entity(blade).remove::<LitBlade>();
                            info!(
                                "item ability: {character:?} slot {slot} {id:#x} puts its blade out"
                            );
                        }
                    }
                }
                _ => {}
            }
        }
        let wanted = if flaming { MovesetConfig::FLAMING.0 } else { 0 };
        match modifier {
            Some(mut m) => {
                m.set_if_neq(HeldModifier(wanted));
            }
            None if wanted != 0 => {
                commands.entity(character).insert(HeldModifier(wanted));
            }
            None => {}
        }
        match burning_grip {
            Some(mut b) => {
                b.set_if_neq(BurningGrip(lit_byte));
            }
            None if lit_byte => {
                commands.entity(character).insert(BurningGrip(true));
            }
            None => {}
        }
    }
}

/// Writes a slash byte and its share into the held item's record (the
/// share is W11, written whichever side strikes) and the copy of the
/// section a flip rebuilds from.
fn set_slash(record: &mut Blade, held: Option<Mut<HeldRecord>>, slash: u8, share: f32) {
    record.record.slash = share;
    record.record.stats.set(at::SLASH, slash);
    if let Some(mut held) = held {
        held.section.stats.set(at::SLASH, slash);
    }
}

/// Ends arcs: they stop respawning and leave once their points are gone.
fn end_arcs(arcs: &mut Query<&mut LiveArcs>, entity: Entity) {
    if let Ok(mut live) = arcs.get_mut(entity) {
        live.arcs.respawns = false;
        live.arcs.ending = true;
    }
}

/// The hit hooks of the held items: a burning blade sizzles, a
/// self-sharpening blade blunts, and a lightning mace with live arcs
/// shocks the struck character, charges its socket and leaves a spark
/// burst. The original's lightning hook defers the charge and the burst to
/// the game's event queue [guess: Exarust runs them at once].
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn hit_hooks(
    mut hits: MessageReader<Hit>,
    mut blades: Query<(
        &mut Blade,
        Option<&LitBlade>,
        Option<&HumArcs>,
        Option<&mut HeldRecord>,
    )>,
    possessions: Query<&Possessions>,
    objects: Option<ResMut<LiveObjects>>,
    mut arcs: Query<&mut LiveArcs>,
    draws: Option<ResMut<FrameEffectDraws>>,
    mut bank: ResMut<SoundBank>,
    mut sounds: MessageWriter<exa_audio::PointSounded>,
    mut shocked: MessageWriter<ArcShocked>,
    mut commands: Commands,
) {
    let (Some(mut objects), Some(mut draws)) = (objects, draws) else {
        for _ in hits.read() {}
        return;
    };
    for hit in hits.read() {
        let Ok((mut blade, lit, hum, held_record)) = blades.get_mut(hit.striker) else {
            continue;
        };
        let id = possessions
            .get(blade.wielder)
            .map_or(0, |p| p.out[blade.slot.index()].id);
        let ability = (id != 0)
            .then(|| objects.db().section(id, WEAPON_SECTION))
            .flatten()
            .and_then(ability_of);
        if lit.is_some() {
            let sizzle = fire_row::sizzle(f64::from(bank.random_unit()));
            info!(
                "item ability: a burning blade's hit sizzles at {:?} (volume {}, pitch {:.3})",
                hit.point, sizzle.volume, sizzle.pitch
            );
            sounds.write(exa_audio::PointSounded {
                name: fire_row::SIZZLE_SOUND.into(),
                position: hit.point,
                volume: sizzle.volume,
                pitch: sizzle.pitch,
                flags: 0,
            });
        }
        match ability.map(|a| a.kind) {
            Some(SELF_SHARPENING) => {
                let Some(mut slash) = objects
                    .db()
                    .section(id, WEAPON_SECTION)
                    .and_then(|b| b.get(at::SLASH).copied())
                else {
                    continue;
                };
                let before = slash;
                let share = item_effects::blunt_on_hit(&mut slash);
                objects.write_section_byte(id, WEAPON_SECTION, at::SLASH, slash);
                set_slash(&mut blade, held_record, slash, share);
                info!(
                    "item ability: {id:#x}'s hit blunts it: slash {before} -> {slash}, W11 {share:.4}"
                );
            }
            Some(LIGHTNING) if hum.is_some_and(|h| arcs.contains(h.arcs)) => {
                let direction = hit.velocity.normalize_or_zero();
                shocked.write(ArcShocked {
                    body: hit.target,
                    direction,
                    strength: LIGHTNING_SHOCK,
                });
                let word = ability.map_or(0, |a| a.socket.kind as u16);
                let (charged, spent) = lightning_hit_charge(word);
                write_socket(&mut objects, id, charged);
                if spent && let Some(hum) = hum {
                    end_arcs(&mut arcs, hum.arcs);
                    commands.entity(hit.striker).remove::<HumArcs>();
                }
                let mut burst = spark_burst(&mut draws.0.0);
                burst.anchor = Some(hit.point * 100.0);
                commands.spawn((
                    Name::new("_MaceSpark"),
                    LiveArcs {
                        arcs: burst,
                        hand: None,
                        weapon: None,
                        caster: Some(blade.wielder),
                        keep: false,
                    },
                ));
                info!(
                    "item ability: {id:#x}'s arcs shock {:?} at {:?}: socket {word:#06x} -> {charged:#06x}{}",
                    hit.target,
                    hit.point,
                    if spent { ", spent: the arcs end" } else { "" }
                );
            }
            _ => {}
        }
    }
}

/// Arcs riding a held item follow it: their frame sits `offset` along the
/// item's own axes and turns with it. Arcs whose item is gone end.
fn follow_items(
    mut commands: Commands,
    mut arcs: Query<(Entity, &mut LiveArcs, &ItemArcs)>,
    items: Query<&GlobalTransform>,
) {
    for (entity, mut live, riding) in &mut arcs {
        match items.get(riding.item) {
            Ok(place) => {
                live.arcs.anchor = Some(place.transform_point(riding.offset / 100.0) * 100.0);
                live.arcs.frame = place.rotation();
            }
            Err(_) => {
                live.arcs.respawns = false;
                live.arcs.ending = true;
                commands.entity(entity).remove::<ItemArcs>();
            }
        }
    }
}

/// Each row's light follows its row; it leaves with it.
fn flame_row_lights(
    mut commands: Commands,
    rows: Query<&Flame>,
    mut lights: Query<(Entity, &FlameRowLight, &mut PointLight)>,
) {
    for (entity, light, mut point) in &mut lights {
        let Ok(flame) = rows.get(light.row) else {
            commands.entity(entity).despawn();
            continue;
        };
        let shown = flame.row().light();
        point.intensity = shown.intensity.max(0.0) * exa_world::LUMENS_PER_INTENSITY;
        point.radius = exa_world::shadow_radius(shown.softness);
    }
}

/// The flame material's picture and palette, loaded once.
#[derive(Resource)]
struct FlameRowAssets {
    material: Handle<FlameRowMaterial>,
}

/// Each row's ribbon, rebuilt every frame from the camera and the tick's
/// fraction, drawn additively with `_FlameMat01`.
#[allow(clippy::too_many_arguments)]
fn flame_row_draws(
    mut commands: Commands,
    assets: Option<Res<FlameRowAssets>>,
    game: Option<Res<Game>>,
    mut cache: ResMut<ModelCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut exa_materials: ResMut<Assets<exa_render::material::ExaMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<FlameRowMaterial>>,
    support: Option<Res<bevy::image::CompressedImageFormatSupport>>,
    fixed: Res<Time<Fixed>>,
    time: Res<Time>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    rows: Query<(Entity, &Flame)>,
    mut draws: Query<(Entity, &FlameRowDraw, &Mesh3d)>,
) {
    if rows.is_empty() && draws.is_empty() {
        return;
    }
    let assets = match assets {
        Some(assets) => assets.material.clone(),
        None => {
            let Some(game) = game else { return };
            let mut loader = ModelLoader {
                game: &game.0,
                cache: &mut cache,
                meshes: &mut meshes,
                materials: &mut exa_materials,
                images: &mut images,
                bc_supported: exa_assets::model::bc_supported(support.as_deref()),
            };
            let Some((picture, _)) = loader.texture(FLAME_TEXTURE, ColorSpace::Srgb) else {
                warn!("flame row: no {FLAME_TEXTURE} picture");
                return;
            };
            let gradient = images.add(gradient_image());
            let material = materials.add(FlameRowMaterial::new(picture, gradient));
            commands.insert_resource(FlameRowAssets {
                material: material.clone(),
            });
            material
        }
    };
    let Some(camera) = cameras
        .iter()
        .filter(|(c, _)| c.is_active)
        .map(|(_, t)| t.translation() * 100.0)
        .next()
    else {
        return;
    };
    let t = fixed.overstep_fraction();
    let now_ms = time.elapsed().as_millis() as u32;
    let mut drawn = HashMap::new();
    for (entity, draw, mesh) in &mut draws {
        match rows.get(draw.row) {
            Ok((_, flame)) => {
                drawn.insert(draw.row, ());
                if let Some(mut m) = meshes.get_mut(&mesh.0) {
                    *m = flame.row().ribbon_mesh(camera, t);
                }
            }
            Err(_) => commands.entity(entity).despawn(),
        }
    }
    // One material per scene: the animation follows the first row's fade.
    if let Some((_, flame)) = rows.iter().next()
        && let Some(mut m) = materials.get_mut(&assets)
    {
        m.set_animation(fire_row::texture_animation(now_ms, flame.row().fade));
    }
    for (row, flame) in &rows {
        if drawn.contains_key(&row) || flame.row().removed() {
            continue;
        }
        commands.spawn((
            Name::new("FlameRowDraw"),
            FlameRowDraw { row },
            Mesh3d(meshes.add(flame.row().ribbon_mesh(camera, t))),
            MeshMaterial3d(assets.clone()),
            Transform::IDENTITY,
            NoFrustumCulling,
            // The ribbon runs through its own light: it neither casts nor
            // takes shadows.
            bevy::light::NotShadowCaster,
            bevy::light::NotShadowReceiver,
        ));
    }
}

/// The looping voice each row holds.
#[derive(Resource, Default)]
struct FlameRowVoices(HashMap<Entity, (VoiceHandle, Option<u32>)>);

/// Each row's `FireS02` loop: started once, then moved to the line's
/// middle and levelled from the row's sound every frame; freed when the
/// row leaves.
fn flame_row_loops(
    game: Option<Res<Game>>,
    mut bank: ResMut<SoundBank>,
    mut env: ResMut<AudioEnvironment>,
    mut sources: ResMut<Assets<AudioSource>>,
    mut voices: ResMut<FlameRowVoices>,
    rows: Query<(Entity, &Flame)>,
) {
    let gone: Vec<Entity> = voices
        .0
        .keys()
        .copied()
        .filter(|&e| !rows.contains(e))
        .collect();
    for entity in gone {
        if let Some((handle, buffer)) = voices.0.remove(&entity) {
            env.free_handle(&mut bank, handle);
            if let Some(buffer) = buffer {
                bank.release_buffer(buffer);
            }
        }
    }
    let Some(game) = game else { return };
    if !env.is_live() {
        return;
    }
    for (entity, flame) in &rows {
        let row = flame.row();
        if row.removed() || !row.has_sound {
            continue;
        }
        let (handle, buffer) = *voices.0.entry(entity).or_insert_with(|| {
            (
                env.new_handle(),
                bank.load_buffer(&game, &mut sources, fire_row::LOOP_SOUND),
            )
        });
        let Some(buffer) = buffer else { continue };
        let sound = row.sound();
        let position = (row.start + row.end) * 0.5 / 100.0;
        match env.handle_voice(handle) {
            Some(id) => {
                if let Some(v) = env.voice_mut(id) {
                    v.position = position;
                    v.gain = sound.volume.max(0.0);
                    v.pitch = sound.pitch;
                }
            }
            None => {
                env.play_buffer_attached(
                    buffer,
                    handle,
                    position,
                    sound.volume.max(0.0),
                    sound.pitch,
                    voice_flags::LOOPING,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(kind: u32, socket: u16) -> Vec<u8> {
        let mut b = vec![0u8; 0x60];
        b[ABILITY_AT..ABILITY_AT + 4].copy_from_slice(&kind.to_le_bytes());
        b[SOCKET_AT..SOCKET_AT + 2].copy_from_slice(&socket.to_le_bytes());
        b
    }

    /// The shipped sword's section: type 3, one empty socket (0x80).
    #[test]
    fn the_shipped_flame_sword_is_cold_until_a_crystal_c_is_socketed() {
        let cold = ability_of(&body(3, 0x0080)).unwrap();
        assert_eq!(cold.kind, BURNING_BLADE);
        assert_eq!(check(Some(cold), AttachedRow::NONE), HeldFire::None);
        // The item GUI's drop: Power Crystal C is kind 4, its charge the
        // stack's bits 16–23.
        let mut sockets = exa_items::item_gui::Sockets([0x0080, 0, 0, 0]);
        sockets.drop_crystal(0, exa_items::possession::crystal::C, 0x12 << 16, false);
        assert_eq!(sockets.0[0], 0x1284);
        let lit = ability_of(&body(3, sockets.0[0])).unwrap();
        assert_eq!(check(Some(lit), AttachedRow::NONE), HeldFire::Ignite);
        // A live row is left burning; one on its way out is replaced.
        let live = AttachedRow {
            exists: true,
            extinguishing: false,
        };
        assert_eq!(check(Some(lit), live), HeldFire::None);
        let going = AttachedRow {
            exists: true,
            extinguishing: true,
        };
        assert_eq!(check(Some(lit), going), HeldFire::Ignite);
        // The crystal taken out (the socket's bare mark): the row goes out.
        assert_eq!(check(Some(cold), live), HeldFire::Extinguish);
        // A spent charge (254 and over) does not burn.
        let spent = ability_of(&body(3, 0xFE84)).unwrap();
        assert_eq!(check(Some(spent), AttachedRow::NONE), HeldFire::None);
    }

    /// Other crystals and other types light nothing.
    #[test]
    fn only_a_type_three_item_with_crystal_c_burns() {
        for socket in [0x0081, 0x0082, 0x0091] {
            let a = ability_of(&body(3, socket)).unwrap();
            assert_eq!(check(Some(a), AttachedRow::NONE), HeldFire::None);
        }
        for kind in [0, 1, 2, 4] {
            let a = ability_of(&body(kind, 0x0084)).unwrap();
            assert_eq!(check(Some(a), AttachedRow::NONE), HeldFire::None);
        }
        assert_eq!(check(None, AttachedRow::NONE), HeldFire::None);
        assert!(ability_of(&[0u8; 0x40]).is_none());
    }

    #[test]
    fn a_lit_type_three_blade_puts_the_player_light_out() {
        assert!(blade_puts_light_out([Some(3), None], true));
        assert!(!blade_puts_light_out([Some(3), None], false));
        assert!(!blade_puts_light_out([Some(1), None], true));
        assert_eq!(LIGHT_AT_CM, 72.5);
        assert_eq!(FLAME_TEXTURE, "flameletsa3");
    }

    /// Crystal A (kind 1) charges the mace below 0xfe, crystal AR (0x11)
    /// below 0x7f; nothing else does.
    #[test]
    fn the_lightning_mace_runs_on_crystal_a_or_ar() {
        assert!(lightning_charged(0x0081));
        assert!(lightning_charged(0xFD81));
        assert!(!lightning_charged(0xFE81));
        assert!(lightning_charged(0x7E91));
        assert!(!lightning_charged(0x7F91));
        assert!(!lightning_charged(0x0084));
        assert!(!lightning_charged(0x0080));
        assert_eq!(lightning_check(0x0081, false), ArcsAction::Raise);
        assert_eq!(lightning_check(0x0081, true), ArcsAction::None);
        assert_eq!(lightning_check(0x0080, true), ArcsAction::End);
        assert_eq!(lightning_check(0x0080, false), ArcsAction::None);
    }

    /// A lightning hit charges the socket by 10 (0x80 with bit 0x10) while
    /// that stays below 0xff; past it the charge is 0xff and the arcs end.
    #[test]
    fn a_lightning_hit_charges_the_socket() {
        assert_eq!(lightning_hit_charge(0x0081), (0x0A81, false));
        assert_eq!(lightning_hit_charge(0xF481), (0xFE81, false));
        assert_eq!(lightning_hit_charge(0xF581), (0xFF81, true));
        assert_eq!(lightning_hit_charge(0x0091), (0x8091, false));
        assert_eq!(lightning_hit_charge(0x7F91), (0xFF91, true));
        // An AR crystal's second hit spends it: its check stops at 0x7f.
        assert!(!lightning_charged(0x8091));
        // Ten hits from empty: 100, and the check still runs below 0xfe.
        let mut word = 0x0081;
        for _ in 0..10 {
            word = lightning_hit_charge(word).0;
        }
        assert_eq!(word, 0x6481);
        assert!(lightning_charged(word));
    }

    /// Types 1, 2 and 3 install a check; 0 and the sceptre's 4 do not.
    #[test]
    fn the_install_takes_types_one_to_three() {
        assert!(installs(LIGHTNING) && installs(SELF_SHARPENING) && installs(BURNING_BLADE));
        assert!(!installs(0) && !installs(4));
        assert_eq!(
            DebugAbilityItem::item(SELF_SHARPENING),
            Some((
                SELF_SHARPENING_SWORD,
                SELF_SHARPENING_MODEL,
                exa_items::possession::crystal::B
            ))
        );
    }

    /// The self-sharpening blade: two slash a run in eight with crystal B,
    /// blunted by four a hit down to 0x3c.
    #[test]
    fn the_self_sharpening_blade_sharpens_and_blunts() {
        let (mut charge, mut slash) = (0u8, 80u8);
        for run in 1..=16 {
            item_effects::sharpen_tick(true, 0x0082, &mut charge, &mut slash, run);
        }
        assert_eq!((charge, slash), (2, 84));
        assert_eq!(
            item_effects::blunt_on_hit(&mut slash),
            item_effects::slash_share(80)
        );
        let mut low = 0x40;
        item_effects::blunt_on_hit(&mut low);
        assert_eq!(low, 0x3c);
        // The share is the record build's.
        let share = item_effects::slash_share(80);
        assert!((share - 0.5 * 0.8f32.powf(0.7)).abs() < 1e-6);
    }
}
