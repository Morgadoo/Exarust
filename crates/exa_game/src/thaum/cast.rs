//! The cast dispatcher of the running game: the player's power bar and the
//! active powers it starts, as the original's `TThaumTree` cast entry and the
//! body's power list run them.
//!
//! - **The bar** ([`PowerCaster`], [`BarCommand`]): `C` loads and opens the
//!   bar from the caster's record, a digit picks a slot, the arrows step and
//!   `Q` uses the picked slot. The record's slots are the caster's known
//!   powers in the order they were learnt; its pairs are the known powers.
//!   A use hands the slot's word to the original's bar
//!   cast entry (`TreeScreen::cast_slot`, which carries the item gate, the
//!   "not now" queue, the close, flash and last-instant rules) and runs the
//!   domain's front end by the word's domain byte: Force, Energy or
//!   Displacement. A Mind id is refused: the Mind casts need a target pick
//!   the running game does not make.
//! - **The active list** ([`ActiveEntry`]): a started power becomes the
//!   class the original's creator makes — the shell, kinesis, throw and
//!   strikes; shock and shock weapons; vanish, the long door, constant
//!   displacement and the summon; a dome's remote power. The barricade's
//!   wall is raised by [`super::barricade`] itself. Every body tick walks the
//!   list in the original's order, passes control event 0 (or 2 when a power
//!   was told to end), keeps a power that answers a result with bit 2 clear
//!   and takes it out with the last entry in its place otherwise, releasing
//!   the focus it held. A cast motion the classes start is played through
//!   [`exa_anim::Animator`] by its motion id's action name. The walk sits
//!   between the body's two focus settles: the reserve's change moves into
//!   the capacity before it, the spent focus leaves the pool (or the pool
//!   regenerates) after it.
//! - **The duellists' casts** ([`exa_ai::DuelPowers`]): before the combat
//!   state runs, each of a duellist's record slots is answered by its
//!   domain's AI evaluator over what it sees of its target; the power the
//!   combat state's chooser picks is then started by its domain's front end
//!   as an AI caster, and becomes an active power as the bar's do.
//! - **The hit formula's factors** ([`PowerFactors`]): a caster's shell
//!   damping and strike factor are put on its fighter, where the blows it
//!   takes and deals read them.
//! - **Live effects** ([`super::live`]): a shock's hand arcs, a barricade's
//!   wall and a constant displacement's wisps are spawned with the power and
//!   removed when it ends; a blast or bolt cast makes its blast entity
//!   ([`super::blast_live`]), which plays the cast motion, takes the cast
//!   hook and ticks after the body tick. Every other effect has no entity
//!   yet: the class keeps running and the missing entity is named in the
//!   crate README.
//!
//! Rules: `docs/THAUMATURGY.md` (In the game), `docs/PLAYER.md` (Hotkeys).
//! Evidence: the bar's cast entry and the dispatcher by domain byte, the
//! casting rows' creators, the body's active-power walk and `TActivePower`'s
//! control, read from `Exanima.exe`.

use bevy::prelude::*;
use exa_ai::{DuelPowers, DuelSystems, Duellist};
use exa_anim::{Animator, Motions};
use exa_character::held::HeldMoveset;
use exa_combat::swing::SwingState;
use exa_combat::{Condition, Fighter, PowerFactors};
use exa_physics::Physics;
use exa_physics::body::locomotion::{Locomotion, Locomotor};
use exa_physics::body::{BodySystems, BodyTicks, Ragdoll};

use super::arcs::{ArcEmitter, HAND_ARCS_OFFSET, hand_arcs};
use super::barricade::{Barricade, WallFrame, state as wall_state};
use super::barrier_live::new_barrier;
use super::blast_live::{LiveBlast, blast_view, start_blast};
use super::bubble_live::{LiveBubble, end_bubble, new_bubble};
use super::casting::{self, ActivePower, AiView, CastBody, ID_MASK, code};
use super::displacement::{
    CursorHit, DisplacementAim, DisplacementStart, body_wisps, displacement_front_end,
};
use super::energy::{EnergyStart, energy_front_end, shock_charge};
use super::force::{
    BARRICADE, ForceStart, ForceWorld, GroundHit, KINESIS, WAVE_SETUP, place_barricade, place_dome,
};
use super::live::{EffectDraws, EffectRoll, LiveArcs, LiveBarricade, LiveWisps};
use super::power::displacement::{
    ConstantDisplacement, LONG_DOOR, LongDoor, NavAnswer, Summon, Vanish,
};
use super::power::energy::{Shock, ShockWeapons};
use super::power::force::{BUBBLE, Kinesis, RemotePower, SHELL, Shell, Strikes, THROW, Throw};
use super::power::{
    ENDING, KNOWN_WORD, KnownPower, PowerBase, READY, RUNNING, drain, evaluate_domain,
};
use super::tree::{
    DOMAIN_DISPLACEMENT, DOMAIN_ENERGY, DOMAIN_FORCE, DOMAIN_MIND, PowerBar, PowerRecord, TreeMode,
    TreeScreen, domain_of, word,
};
use super::torrent_live::{new_torrent, torrent_view};
use super::wave_live::new_wave;
use super::wisps::{Strand, Wisps};
use crate::effects::blast::Blast;
use crate::hand::{Hand, Holding};
use crate::player::{CursorGround, PlayerBody};
use crate::power_screen::PowerScreen;

/// Centimetres a metre: the original's effects run in cm.
const CM: f32 = 100.0;
/// The right hand's bone.
const RIGHT_HAND: usize = 17;
/// The body tick the focus and the clip lengths are read at.
const TICK_MS: u32 = 12;
/// The cast clip's length when no motion set says otherwise, in ms.
const DEFAULT_CLIP_MS: u32 = 1200;
/// The status bit of a body that controls another: its bar casts nothing.
const CONTROLLING: u32 = 0x20_0000;

/// What a power-bar key asks of a caster.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarAction {
    /// `C`: open the bar, or store and close it.
    Toggle,
    /// A digit: pick a slot (0-based).
    Pick(usize),
    /// `Q`: use the picked slot.
    Use,
    /// An arrow: step the choice.
    Step(i32),
}

/// A power-bar command for one caster, sent by the hotkey handler.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarCommand {
    pub caster: Entity,
    pub action: BarAction,
}

/// One active power: the class the cast's creator made, its state, the
/// focus it spends a tick and the live effect entity it spawned.
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveEntry {
    /// The id the body's power list holds (a vanish holds 0x601, as its
    /// creator registers it).
    pub id: u32,
    /// The id its cast was asked for, which the cast motion's end compares.
    pub cast_id: u32,
    pub state: i32,
    pub class: ActiveClass,
    /// What a sustained power spends a tick.
    pub tick_cost: f32,
    /// The live entity (arcs, wall, wisps), when one exists.
    pub entity: Option<Entity>,
    /// A throw's target: its centre (cm) and mass. None leaves the throw
    /// waiting: the running game has no cursor body pick yet.
    pub throw_target: Option<([f32; 3], f32)>,
}

/// The active power classes the casts create.
#[derive(Debug, Clone, PartialEq)]
pub enum ActiveClass {
    Shell(Shell),
    Kinesis(Kinesis),
    Throw(Throw),
    Strikes(Strikes),
    /// A bubble: the effect is elsewhere, this is its power object.
    Remote(RemotePower),
    Shock(Shock),
    ShockWeapons(ShockWeapons),
    Vanish(Vanish),
    LongDoor(LongDoor),
    Constant(ConstantDisplacement),
    Summon(Summon),
}

impl ActiveClass {
    /// The class' state word, where it keeps one.
    fn state(&self) -> Option<i32> {
        match self {
            ActiveClass::Shell(p) => Some(p.state),
            ActiveClass::Kinesis(p) => Some(p.state),
            ActiveClass::Throw(p) => Some(p.state),
            ActiveClass::Strikes(p) => Some(p.state),
            ActiveClass::Remote(p) => Some(p.state),
            ActiveClass::Shock(p) => Some(p.state),
            ActiveClass::ShockWeapons(p) => Some(p.state),
            ActiveClass::Constant(p) => Some(p.state),
            ActiveClass::Vanish(_) | ActiveClass::LongDoor(_) | ActiveClass::Summon(_) => None,
        }
    }
}

/// A caster's powers: the body the front ends read, the powers the player
/// knows, the bar and the active list.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct PowerCaster {
    pub body: CastBody,
    pub known: Vec<KnownPower>,
    pub active: Vec<ActiveEntry>,
    /// The power record the bar, tree and dispatcher read.
    pub record: PowerRecord,
    pub bar: PowerBar,
    pub bar_open: bool,
    /// The bar cast entry's answer (1 started, 0 released, 2 ended, negative
    /// refused/failed/not now; 0 before the first use).
    pub last_result: i32,
    /// The animator's action when the cast hooks last looked.
    pub seen_action: Option<String>,
}

impl PowerCaster {
    /// A caster who knows `ids` in that order: the record's slots are those
    /// powers, the first chosen as a new character's bar slot is, and the
    /// body has one persistent slot (the debug grant's techniques).
    pub fn new(ids: &[u32]) -> Self {
        let mut record = PowerRecord::empty(4.0);
        record.focus_item = true;
        record.pairs = ids
            .iter()
            .map(|&power| super::tree::KnownPower {
                power,
                flags: super::tree::LEARNED,
            })
            .collect();
        let known = known_from(&record);
        record.slots = [0; 10];
        for (slot, power) in record.slots.iter_mut().zip(&known) {
            *slot = power.id;
        }
        if let Some(first) = record.slots.first_mut()
            && *first != 0
        {
            *first |= word::CHOSEN;
        }
        let mut bar = PowerBar::default();
        bar.load(&record);
        Self {
            body: CastBody {
                slots: 1,
                tick_ms: TICK_MS,
                ..CastBody::default()
            },
            known,
            active: Vec::new(),
            record,
            bar,
            bar_open: false,
            last_result: 0,
            seen_action: None,
        }
    }

    /// A character record's caster: its known powers and its ten slots, on
    /// the body [`PowerCaster::new`] gives.
    pub fn of_record(powers: &exa_formats::characters::Powers) -> Self {
        let mut caster = Self::new(&[]);
        caster.record.pairs = powers
            .known
            .iter()
            .map(|power| super::tree::KnownPower {
                power: power.id,
                flags: power.flags,
            })
            .collect();
        caster.record.slots = powers.slots;
        caster.known = known_from(&caster.record);
        caster.bar.load(&caster.record);
        caster
    }

    /// A character's caster: its record's powers, with the body's technique
    /// word built from the learnt techniques and its persistent slots from
    /// Concentration's ([`casting::slot_count`]).
    pub fn of_character(record: &exa_formats::characters::CharacterRecord) -> Option<Self> {
        let powers = record.powers.as_ref()?;
        let mut caster = Self::of_record(powers);
        caster.body.techniques = record.perks().body_word();
        let concentration = &record.skills[exa_formats::characters::CONCENTRATION];
        caster.body.slots = casting::slot_count(
            concentration.points,
            &concentration.techniques,
            caster.record.focus_item,
        );
        Some(caster)
    }

    /// Rebuild the body's power view from the active list, in its order.
    pub(crate) fn sync_body(&mut self) {
        self.body.powers = self
            .active
            .iter()
            .map(|entry| ActivePower {
                id: entry.id,
                state: entry.state,
            })
            .collect();
    }

    /// Take the state words a front end wrote on the body's power view back
    /// onto the active classes.
    fn take_states(&mut self) {
        for (entry, power) in self.active.iter_mut().zip(&self.body.powers) {
            entry.state = power.state;
        }
    }
}

/// The plugin that runs the bar and the active powers.
pub struct CastPlugin;

impl Plugin for CastPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<BarCommand>()
            .init_resource::<EffectDraws>()
            .init_resource::<EffectRoll>()
            .add_systems(Update, (run_bar_commands, sync_screen_bar).chain())
            .add_systems(
                Update,
                (
                    advise_duellists.before(DuelSystems),
                    cast_for_duellists.after(DuelSystems),
                    publish_power_factors
                        .after(cast_for_duellists)
                        .after(run_bar_commands),
                ),
            )
            .add_systems(
                FixedUpdate,
                (
                    tick_casters,
                    super::blast_live::tick_blasts,
                    super::barrier_live::tick_barriers,
                    super::bubble_live::tick_bubbles,
                    super::wave_live::tick_waves,
                    super::torrent_live::tick_torrents,
                    play_cast_motions,
                    call_motion_hooks,
                )
                    .chain()
                    .after(BodySystems),
            );
    }
}

/// What a domain's front end answered: its code and the class to make.
#[derive(Debug, Clone, Copy, PartialEq)]
enum CastStart {
    Force(ForceStart),
    Energy(EnergyStart),
    Displacement(DisplacementStart),
}

/// Run a power word through its domain's front end, as the bar's dispatcher
/// does: Force, Energy and Displacement by the word's domain byte; a Mind id
/// is refused here.
fn dispatch_domain(
    domain: u8,
    asked: u32,
    body: &mut CastBody,
    world: &ForceWorld,
    aim: &DisplacementAim,
) -> (i32, Option<CastStart>) {
    match domain {
        DOMAIN_FORCE => {
            let (result, start) = super::force::force_front_end(asked, body, world);
            (result, start.map(CastStart::Force))
        }
        DOMAIN_ENERGY => {
            let (result, start) = energy_front_end(asked, body);
            (result, start.map(CastStart::Energy))
        }
        DOMAIN_DISPLACEMENT => {
            let (result, start) = displacement_front_end(asked, body, aim);
            (result, start.map(CastStart::Displacement))
        }
        DOMAIN_MIND => (code::REFUSED, None),
        _ => (code::REFUSED, None),
    }
}

/// The context a cast's creator reads besides the body.
struct CastContext {
    now: u32,
    caster: Vec3,
    heading: Vec3,
    cursor: Option<Vec3>,
    clip_ms: Option<u32>,
    /// The body the AI casts at.
    target: Option<Entity>,
    /// The caster's own place (cm), at its feet: its transform's, else the
    /// body's root.
    standing: Vec3,
}

/// One caster's bar command.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn run_bar_commands(
    mut commands: Commands,
    mut commands_in: MessageReader<BarCommand>,
    mut casters: Query<(
        Entity,
        &mut PowerCaster,
        Option<&Condition>,
        Option<&SwingState>,
        Option<&Locomotion>,
        Option<&Locomotor>,
        Option<&mut Animator>,
        Option<&HeldMoveset>,
        Option<&Ragdoll>,
    )>,
    hand: Option<Res<Hand>>,
    ground: Option<Res<CursorGround>>,
    physics: Option<Res<Physics>>,
    ticks: Option<Res<BodyTicks>>,
    motions: Option<Res<Motions>>,
    mut draws: ResMut<EffectDraws>,
    mut roll: ResMut<EffectRoll>,
    mut screen: Option<ResMut<PowerScreen>>,
) {
    for command in commands_in.read().copied() {
        let Ok((
            entity,
            mut caster,
            condition,
            swing,
            locomotion,
            locomotor,
            animator,
            held,
            ragdoll,
        )) = casters.get_mut(command.caster)
        else {
            continue;
        };
        let caster = &mut *caster;
        match command.action {
            BarAction::Toggle => {
                if caster.bar_open {
                    caster.bar.store(&mut caster.record, false);
                    caster.bar_open = false;
                } else if !caster.known.is_empty() {
                    caster.bar.load(&caster.record);
                    caster.bar_open = true;
                }
            }
            BarAction::Pick(slot) => {
                if caster.bar_open && caster.bar.select(slot) {
                    caster.bar.store(&mut caster.record, false);
                }
            }
            BarAction::Step(direction) => {
                if caster.bar.cycle(direction, Some(&mut caster.record)) {
                    caster.bar.store(&mut caster.record, false);
                }
            }
            BarAction::Use => {
                refresh_body(
                    &mut caster.body,
                    condition,
                    swing,
                    locomotion,
                    locomotor,
                    held,
                );
                let busy = hand
                    .as_deref()
                    .is_some_and(|h| h.holding != Holding::Nothing);
                let possessing = caster.body.status_bits & CONTROLLING != 0;
                let world = ForceWorld {
                    controls_busy: busy,
                    possessing,
                    cursor_on_barricade: false,
                    throw_pick: false,
                    kinesis_holds: false,
                };
                let aim = DisplacementAim {
                    player: true,
                    // [inf: the running game has the cursor's point on the
                    // aim plane, not a surface hit with a normal, so the
                    // point is taken as it stands (exact) and the original's
                    // 15 cm back-off along a hit normal does not apply.]
                    cursor: ground.as_deref().and_then(|g| g.0).map(|point| CursorHit {
                        point: point * CM,
                        normal: Vec3::Y,
                        exact: true,
                    }),
                    possessing,
                };
                let slot = chosen_slot(caster);
                let Some(slot) = slot else {
                    continue;
                };
                let word_value = caster.record.slots[slot];
                if word_value & word::INDEX == 0 {
                    continue;
                }
                let asked = word_value & ID_MASK;
                let now = ticks.as_deref().map_or(0, |t| t.millis() as u32);
                let mut tree = TreeScreen::empty(DOMAIN_FORCE, TreeMode::Play);
                tree.bar_open = caster.bar_open;
                let mut start = None;
                let mut result_code = 0;
                let entry = tree.cast_slot(
                    &mut caster.record,
                    &mut caster.bar,
                    Some(slot),
                    &mut caster.body,
                    now,
                    |domain, asked, body| {
                        let (result, begun) = dispatch_domain(domain, asked, body, &world, &aim);
                        start = begun;
                        result_code = result;
                        result
                    },
                );
                caster.bar_open = tree.bar_open;
                caster.last_result = result_code;
                if let Some(sound) = entry.sound {
                    debug!("cast failed: {}", sound.name);
                }
                if entry.end_control {
                    debug!("cast while controlling another body: the control would end");
                }
                if let Some(queued) = entry.queued_slot {
                    debug!("cast queued on slot {queued}");
                }
                debug!("bar cast {asked:#x}: {result_code}");
                let place = CastPlace {
                    now,
                    ragdoll,
                    yaw: locomotor.map_or(0.0, |l| l.yaw),
                    cursor: ground.as_deref().and_then(|g| g.0).map(|p| p * CM),
                    clip_ms: shot_clip_ms(motions.as_deref(), animator.as_deref()),
                    target: None,
                    standing: None,
                };
                begin_cast(
                    &mut commands,
                    entity,
                    caster,
                    entry.result > 0,
                    start,
                    &place,
                    physics.as_deref(),
                    &mut draws,
                    &mut roll,
                );
                if let Some(screen) = screen.as_deref_mut() {
                    store_on_screen(screen, caster);
                }
            }
        }
    }
}

/// Where a cast starts: the clock, the caster's body and heading (yaw,
/// +Z at 0), the cursor's point (cm, the player's) and the cast clip's
/// length.
struct CastPlace<'a> {
    now: u32,
    ragdoll: Option<&'a Ragdoll>,
    yaw: f32,
    cursor: Option<Vec3>,
    clip_ms: Option<u32>,
    target: Option<Entity>,
    /// The caster's transform's place (cm), when it has one.
    standing: Option<Vec3>,
}

/// After a domain's front end: a vanish asked again ends the running one,
/// a started power gets the class its creator makes, and the body's power
/// view follows the active list.
#[allow(clippy::too_many_arguments)]
fn begin_cast(
    commands: &mut Commands,
    entity: Entity,
    caster: &mut PowerCaster,
    started: bool,
    start: Option<CastStart>,
    place: &CastPlace,
    physics: Option<&Physics>,
    draws: &mut EffectDraws,
    roll: &mut EffectRoll,
) {
    caster.take_states();
    if matches!(
        start,
        Some(CastStart::Displacement(DisplacementStart::EndVanish))
    ) {
        // Vanish pressed while it runs: the running power's hook is told to
        // end it.
        for running in &mut caster.active {
            if let ActiveClass::Vanish(power) = &mut running.class {
                power.release();
            }
        }
    }
    if started && let Some(start) = start {
        let root = place.ragdoll.and_then(root_of).unwrap_or(Vec3::ZERO);
        let context = CastContext {
            now: place.now,
            caster: root,
            heading: Vec3::new(place.yaw.sin(), 0.0, place.yaw.cos()),
            cursor: place.cursor,
            clip_ms: place.clip_ms,
            target: place.target,
            standing: place.standing.unwrap_or(root),
        };
        let made = create_active(
            commands,
            entity,
            caster,
            start,
            &context,
            place.ragdoll,
            physics,
            draws,
            roll,
        );
        debug!(
            "cast by {entity:?}: {} active, effect entity {:?}",
            caster.active.len(),
            made
        );
    }
    caster.sync_body();
}

/// Answers each duellist's power slots from its caster: every slot's power
/// through its domain's AI evaluator, over what the duellist sees of its
/// target. The pool is the caster's focus.
#[allow(clippy::type_complexity)]
fn advise_duellists(
    mut duellists: Query<(
        &mut DuelPowers,
        &mut PowerCaster,
        &Duellist,
        &GlobalTransform,
        Option<&Condition>,
        Option<&SwingState>,
        Option<&Locomotion>,
        Option<&Locomotor>,
        Option<&Fighter>,
    )>,
    places: Query<&GlobalTransform>,
    children: Query<&Children>,
    targets: Query<(Option<&Condition>, Option<&PowerCaster>), Without<DuelPowers>>,
) {
    for (mut powers, mut caster, duellist, at, condition, swing, locomotion, locomotor, fighter) in
        &mut duellists
    {
        let caster = &mut *caster;
        refresh_body(
            &mut caster.body,
            condition,
            swing,
            locomotion,
            locomotor,
            None,
        );
        let mut view = AiView {
            caster: caster.body.clone(),
            health_ratio: fighter
                .filter(|f| f.health.max > 0.0)
                .map_or(1.0, |f| f.health.yellow / f.health.max),
            summoned: caster
                .active
                .iter()
                .any(|entry| matches!(entry.class, ActiveClass::Summon(_))),
            ..AiView::default()
        };
        if let Ok(target) = places.get(duellist.target) {
            let to = (target.translation() - at.translation()) * CM;
            view.distance_sq = to.length_squared();
            // [Exarust's measure: 1 less the cosine between the facing and
            // the flat line to the target, 0 ahead to 2 behind, as the
            // combat state's own "ahead" is.]
            let flat = Vec3::new(to.x, 0.0, to.z).normalize_or_zero();
            view.facing = 1.0 - (at.rotation() * Vec3::Z).dot(flat);
        }
        let target_body = std::iter::once(duellist.target)
            .chain(children.iter_descendants(duellist.target))
            .filter_map(|e| targets.get(e).ok());
        for (condition, power_caster) in target_body {
            if let Some(condition) = condition {
                view.target_condition |= condition.0;
            }
            if let Some(power_caster) = power_caster {
                view.target_status |= power_caster.body.status_bits;
            }
        }
        let slots = caster.record.slots.map(|slot| {
            if slot & word::INDEX == 0 {
                0
            } else {
                slot & ID_MASK
            }
        });
        let answers = slots.map(|slot| {
            if slot == 0 {
                0
            } else {
                evaluate_domain(slot, &view)
            }
        });
        let casting = caster.body.casting != 0;
        let advised = DuelPowers {
            slots,
            answers,
            pool: caster.body.focus,
            casting,
            running: casting || caster.body.cast_hook.is_some_and(|hook| {
                    hook.answer(super::casting::hook_event::CAST_START) >= 0
                }),
            chosen: powers.chosen,
        };
        powers.set_if_neq(advised);
    }
}

/// Casts the power each duellist's combat state chose, by its domain's
/// front end as an AI caster (no cursor, its hands free).
/// [inf: the AI's cast is taken to start the power as the bar's dispatcher
/// does, without the bar's slot rules.]
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn cast_for_duellists(
    mut commands: Commands,
    mut duellists: Query<(
        Entity,
        &mut DuelPowers,
        &mut PowerCaster,
        &Duellist,
        &GlobalTransform,
        Option<&Condition>,
        Option<&SwingState>,
        Option<&Locomotion>,
        Option<&Locomotor>,
        Option<&Animator>,
        Option<&HeldMoveset>,
        Option<&Ragdoll>,
    )>,
    physics: Option<Res<Physics>>,
    ticks: Option<Res<BodyTicks>>,
    motions: Option<Res<Motions>>,
    mut draws: ResMut<EffectDraws>,
    mut roll: ResMut<EffectRoll>,
) {
    let now = ticks.as_deref().map_or(0, |t| t.millis() as u32);
    for (
        entity,
        mut powers,
        mut caster,
        duellist,
        at,
        condition,
        swing,
        locomotion,
        locomotor,
        animator,
        held,
        ragdoll,
    ) in &mut duellists
    {
        let Some(asked) = powers.chosen.take() else {
            continue;
        };
        let caster = &mut *caster;
        refresh_body(
            &mut caster.body,
            condition,
            swing,
            locomotion,
            locomotor,
            held,
        );
        let possessing = caster.body.status_bits & CONTROLLING != 0;
        let world = ForceWorld {
            controls_busy: false,
            possessing,
            cursor_on_barricade: false,
            throw_pick: false,
            kinesis_holds: false,
        };
        let aim = DisplacementAim {
            player: false,
            cursor: None,
            possessing,
        };
        let (result, start) =
            dispatch_domain(domain_of(asked), asked, &mut caster.body, &world, &aim);
        caster.last_result = result;
        debug!("duellist {entity:?} casts {asked:#x}: {result}");
        let facing = at.rotation() * Vec3::Z;
        let place = CastPlace {
            now,
            ragdoll,
            yaw: facing.x.atan2(facing.z),
            cursor: None,
            clip_ms: shot_clip_ms(motions.as_deref(), animator),
            target: Some(duellist.target),
            standing: Some(at.translation() * CM),
        };
        begin_cast(
            &mut commands,
            entity,
            caster,
            result > 0,
            start,
            &place,
            physics.as_deref(),
            &mut draws,
            &mut roll,
        );
        powers.casting = caster.body.casting != 0;
    }
}

/// Puts each caster's shell damping and strike factor on its fighter for
/// the hit formula.
fn publish_power_factors(
    mut commands: Commands,
    mut casters: Query<(Entity, &PowerCaster, Option<&mut PowerFactors>)>,
) {
    for (entity, caster, factors) in &mut casters {
        let now = PowerFactors {
            shell: caster.body.hit_damping,
            strikes: caster.body.strike_factor,
        };
        match factors {
            Some(mut factors) => {
                factors.set_if_neq(now);
            }
            None => {
                commands.entity(entity).insert(now);
            }
        }
    }
}

/// The slot a use casts: the bar's chosen slot while it is open, else the
/// record's slot with the chosen bit.
fn chosen_slot(caster: &PowerCaster) -> Option<usize> {
    if caster.bar_open && caster.bar.chosen > 0 {
        return Some(caster.bar.chosen - 1);
    }
    caster
        .record
        .slots
        .iter()
        .position(|slot| slot & word::CHOSEN != 0)
}

/// The body's state as the front ends read it from the running game.
#[allow(clippy::too_many_arguments)]
fn refresh_body(
    body: &mut CastBody,
    condition: Option<&Condition>,
    swing: Option<&SwingState>,
    locomotion: Option<&Locomotion>,
    locomotor: Option<&Locomotor>,
    held: Option<&HeldMoveset>,
) {
    if let Some(condition) = condition {
        body.condition = condition.0;
    }
    if let Some(swing) = swing {
        body.swing_bits = swing.flags;
    }
    if let Some(locomotion) = locomotion {
        body.moving = locomotion.moving;
        body.gait_bits = locomotion.flags;
    }
    if let Some(locomotor) = locomotor {
        body.pace = locomotor.speed.abs();
    }
    if let Some(held) = held {
        body.weapon_class = u32::from(held.weapon_class);
    }
}

/// The cast clip's length in milliseconds: the action's last key (frames)
/// at the body tick. This is Exarust's conversion; the original stores the
/// clip's own millisecond length (a guess labelled here).
fn shot_clip_ms(motions: Option<&Motions>, animator: Option<&Animator>) -> Option<u32> {
    let motions = motions?;
    let animator = animator?;
    let name = motions.action_name(super::energy::SHOCK_MOTION)?;
    let clip = motions.clip(animator, name)?;
    let last = clip.keys.last()?.time;
    Some((f64::from(last) / 83.0 * 1000.0).round() as u32)
}

/// The body's root point in cm.
fn root_of(body: &Ragdoll) -> Option<Vec3> {
    body.parts
        .first()
        .map(|part| Vec3::from(part.transform().translation) * CM)
}

/// Make the class a started power's creator makes, and its live effect.
#[allow(clippy::too_many_arguments)]
fn create_active(
    commands: &mut Commands,
    caster_entity: Entity,
    caster: &mut PowerCaster,
    start: CastStart,
    context: &CastContext,
    ragdoll: Option<&Ragdoll>,
    physics: Option<&Physics>,
    draws: &mut EffectDraws,
    roll: &mut EffectRoll,
) -> Option<Entity> {
    match start {
        CastStart::Force(start) => create_force(
            commands,
            caster_entity,
            caster,
            start,
            context,
            ragdoll,
            physics,
            draws,
            roll,
        ),
        CastStart::Energy(start) => create_energy(
            commands,
            caster_entity,
            caster,
            start,
            context,
            ragdoll,
            draws,
        ),
        CastStart::Displacement(start) => create_displacement(
            commands,
            caster_entity,
            caster,
            start,
            context,
            ragdoll,
            draws,
            roll,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn create_force(
    commands: &mut Commands,
    caster_entity: Entity,
    caster: &mut PowerCaster,
    start: ForceStart,
    context: &CastContext,
    ragdoll: Option<&Ragdoll>,
    physics: Option<&Physics>,
    draws: &mut EffectDraws,
    roll: &mut EffectRoll,
) -> Option<Entity> {
    match start {
        ForceStart::Blast { setup, cost } if setup != WAVE_SETUP => {
            let body = ragdoll?;
            let view = blast_view(body, context.heading, context.cursor)?;
            let hooked = start_blast(&mut caster.body, &setup);
            let blast = Blast::new(setup, cost, TICK_MS, &view, &mut draws.0);
            let entity = commands
                .spawn((
                    Name::new("_ForceBlast"),
                    LiveBlast {
                        blast,
                        caster: caster_entity,
                        target: context.target,
                        hooked,
                    },
                ))
                .id();
            debug!(
                "force blast {:#x}: motion {hooked}, entity {entity:?}",
                setup.power
            );
            Some(entity)
        }
        ForceStart::Blast { cost, .. } => {
            let root = ragdoll.and_then(root_of).unwrap_or(context.caster);
            let live = new_wave(
                &mut caster.body,
                caster_entity,
                cost,
                root,
                context.heading,
                TICK_MS,
                draws,
            );
            let hooked = live.hooked;
            let entity = commands.spawn((Name::new("_ForceWave"), live)).id();
            debug!("force wave: motion {hooked}, entity {entity:?}");
            Some(entity)
        }
        ForceStart::Torrent { cost } => {
            let body = ragdoll?;
            let view = torrent_view(body, context.heading, context.cursor)?;
            let live = new_torrent(
                &mut caster.body,
                caster_entity,
                context.target,
                cost,
                &view,
                TICK_MS,
                draws,
                roll,
            );
            let hooked = live.hooked;
            let entity = commands.spawn((Name::new("_ForceTorrent"), live)).id();
            debug!("force torrent: motion {hooked}, entity {entity:?}");
            Some(entity)
        }
        ForceStart::Barrier { cost } => {
            let live = new_barrier(
                &mut caster.body,
                caster_entity,
                cost,
                context.standing,
                context.now,
                draws,
            );
            let hooked = live.hooked;
            let entity = commands.spawn((Name::new("_ForceBarrier"), live)).id();
            debug!("force barrier: motion {hooked}, entity {entity:?}");
            Some(entity)
        }
        ForceStart::Bubble { cost } => {
            let place = place_dome(
                super::force::ForceKind::Bubble,
                context.standing,
                context.heading,
            );
            let at = place.map_or(context.standing, |p| p.centre);
            let (live, motion) = new_bubble(
                &mut caster.body,
                caster_entity,
                at,
                context.standing,
                context.now,
                draws,
            );
            let held = PowerBase::new(BUBBLE).accrue(&mut caster.body, cost);
            let mut power = RemotePower::new(at.to_array());
            power.held = held;
            let entity = commands.spawn((Name::new("_ForceBubble"), live)).id();
            debug!("force bubble at {at:?}: cast motion {motion}, entity {entity:?}");
            push_active(
                caster,
                BUBBLE,
                BUBBLE,
                RUNNING,
                ActiveClass::Remote(power),
                0.0,
                Some(entity),
            );
            Some(entity)
        }
        ForceStart::Barricade { cost } => {
            let place = barricade_place(context, physics, roll);
            let frame = WallFrame::facing(place.centre, place.facing);
            let now = context.now;
            let trim = |start: Vec3, dir: Vec3, len: f32| {
                physics
                    .and_then(|p| p.fixed_hit(start / CM, dir, len / CM))
                    .map(|d| d * CM)
            };
            let mut wall = Barricade::raise(frame, cost, now, trim, None, &mut draws.0);
            let add = wall.add_to_world(&mut caster.body, None, None, f64::from(roll.0.unit()));
            let entity = commands
                .spawn((
                    Name::new("_ForceBarricade"),
                    LiveBarricade {
                        wall: wall.clone(),
                        caster: Some(caster_entity),
                    },
                ))
                .id();
            let mut power = RemotePower::new(place.centre.to_array());
            power.held = add.held;
            debug!(
                "force barricade at {:?}: motion {}, held {:.3}",
                place.centre, add.motion, add.held
            );
            push_active(
                caster,
                BARRICADE,
                BARRICADE,
                RUNNING,
                ActiveClass::Remote(power),
                0.0,
                Some(entity),
            );
            Some(entity)
        }
        ForceStart::Shell { cost } => {
            let mut power = Shell::new();
            let held = {
                let body = &mut caster.body;
                power.apply(body);
                PowerBase::new(SHELL).accrue(body, cost)
            };
            power.held = held;
            push_active(
                caster,
                SHELL,
                SHELL,
                READY,
                ActiveClass::Shell(power),
                0.0,
                None,
            );
            debug!("force shell: the aura entity is not ported");
            None
        }
        ForceStart::Kinesis { cost } => {
            push_active(
                caster,
                KINESIS,
                KINESIS,
                READY,
                ActiveClass::Kinesis(Kinesis::new()),
                cost,
                None,
            );
            debug!("force kinesis: the pick and aim sweep are not wired");
            None
        }
        ForceStart::Throw { cost, from_kinesis } => {
            push_active(
                caster,
                THROW,
                THROW,
                READY,
                ActiveClass::Throw(Throw::new(cost)),
                0.0,
                None,
            );
            debug!("force throw (from kinesis: {from_kinesis}): no target body pick");
            None
        }
        ForceStart::Strikes { cost, .. } => {
            let mut power = Strikes::new();
            let held = {
                let body = &mut caster.body;
                power.apply(body, &[]);
                PowerBase::new(super::force::STRIKES).accrue(body, cost)
            };
            power.held = held;
            push_active(
                caster,
                super::force::STRIKES,
                super::force::STRIKES,
                READY,
                ActiveClass::Strikes(power),
                0.0,
                None,
            );
            debug!("force strikes: the weapon auras and effect are not wired");
            None
        }
        ForceStart::ThrowMissed => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn create_energy(
    commands: &mut Commands,
    caster_entity: Entity,
    caster: &mut PowerCaster,
    start: EnergyStart,
    context: &CastContext,
    ragdoll: Option<&Ragdoll>,
    draws: &mut EffectDraws,
) -> Option<Entity> {
    match start {
        EnergyStart::Shock { cost } => {
            let charge = shock_charge(context.clip_ms.unwrap_or(DEFAULT_CLIP_MS), cost);
            let power = Shock::new(charge.length, charge.cost);
            let entity = ragdoll.map(|body| spawn_hand_arcs(commands, caster_entity, body, draws));
            push_active(
                caster,
                super::energy::SHOCK,
                super::energy::SHOCK,
                READY,
                ActiveClass::Shock(power),
                0.0,
                entity,
            );
            debug!(
                "energy shock: charge {:.3} a tick over {:.3}, burst entity not ported",
                charge.length, charge.cost
            );
            entity
        }
        EnergyStart::ShockWeapons { cost } => {
            let mut power = ShockWeapons::new();
            power.held =
                PowerBase::new(super::energy::SHOCK_WEAPONS).accrue(&mut caster.body, cost);
            push_active(
                caster,
                super::energy::SHOCK_WEAPONS,
                super::energy::SHOCK_WEAPONS,
                READY,
                ActiveClass::ShockWeapons(power),
                0.0,
                None,
            );
            debug!("energy shock weapons: the per-weapon arc entities are not wired");
            None
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn create_displacement(
    commands: &mut Commands,
    caster_entity: Entity,
    caster: &mut PowerCaster,
    start: DisplacementStart,
    context: &CastContext,
    ragdoll: Option<&Ragdoll>,
    draws: &mut EffectDraws,
    roll: &mut EffectRoll,
) -> Option<Entity> {
    match start {
        DisplacementStart::Vanish { cost } => {
            let power = Vanish::new(context.now, false, false, cost);
            push_active(
                caster,
                0x601,
                super::displacement::VANISH,
                READY,
                ActiveClass::Vanish(power),
                0.0,
                None,
            );
            debug!("displacement vanish: the effect node is not ported");
            None
        }
        DisplacementStart::LongDoor { tick_cost, to, .. } => {
            let power = LongDoor::new(context.now, to.map(|p| p.to_array()), false, tick_cost);
            push_active(
                caster,
                LONG_DOOR,
                LONG_DOOR,
                READY,
                ActiveClass::LongDoor(power),
                0.0,
                None,
            );
            debug!("displacement long door: the effect node and the move are not wired");
            None
        }
        DisplacementStart::Constant { cost } => {
            let mut power = ConstantDisplacement::new();
            power.held =
                PowerBase::new(super::displacement::CONSTANT).accrue(&mut caster.body, cost);
            let entity =
                ragdoll.map(|body| spawn_body_wisps(commands, caster_entity, body, draws, roll));
            push_active(
                caster,
                super::displacement::CONSTANT,
                super::displacement::CONSTANT,
                READY,
                ActiveClass::Constant(power),
                0.0,
                entity,
            );
            debug!("constant displacement: wisps on the body, the veil is not ported");
            entity
        }
        DisplacementStart::Summon { .. } => {
            let power = Summon::new(context.now, false);
            push_active(
                caster,
                super::displacement::SUMMON,
                super::displacement::SUMMON,
                READY,
                ActiveClass::Summon(power),
                0.0,
                None,
            );
            debug!("displacement summon: the demon spawn is not wired");
            None
        }
        DisplacementStart::EndVanish => None,
    }
}

fn push_active(
    caster: &mut PowerCaster,
    id: u32,
    cast_id: u32,
    state: i32,
    class: ActiveClass,
    tick_cost: f32,
    entity: Option<Entity>,
) {
    caster.active.push(ActiveEntry {
        id,
        cast_id,
        state,
        class,
        tick_cost,
        entity,
        throw_target: None,
    });
    caster.sync_body();
}

/// Place a barricade: the cursor's aim-plane point when there is one, else
/// two metres ahead; the floor probe is the level's.
fn barricade_place(
    context: &CastContext,
    physics: Option<&Physics>,
    roll: &mut EffectRoll,
) -> super::force::BarricadePlace {
    let point = context
        .cursor
        .unwrap_or_else(|| context.caster + context.heading * 200.0 - Vec3::Y * 80.0);
    let hit = GroundHit {
        point,
        normal: Vec3::Y,
        exact: context.cursor.is_none(),
    };
    let floor = |from: Vec3, reach: f32| {
        physics
            .and_then(|p| p.fixed_hit(from / CM, Vec3::NEG_Y, reach / CM))
            .map(|d| d * CM)
    };
    place_barricade(hit, context.caster, floor, roll.0.unit())
}

/// The shock's hand arcs.
fn spawn_hand_arcs(
    commands: &mut Commands,
    caster: Entity,
    body: &Ragdoll,
    draws: &mut EffectDraws,
) -> Entity {
    let mut arcs = ArcEmitter::new(&hand_arcs(), &mut draws.0);
    arcs.anchor = bone_part(body, RIGHT_HAND).and_then(|k| part_point(body, k, HAND_ARCS_OFFSET));
    commands
        .spawn((
            Name::new("_ShockArcs"),
            LiveArcs {
                arcs,
                hand: Some((caster, RIGHT_HAND)),
                caster: Some(caster),
                keep: false,
            },
        ))
        .id()
}

/// The constant displacement's wisps.
fn spawn_body_wisps(
    commands: &mut Commands,
    caster: Entity,
    body: &Ragdoll,
    draws: &mut EffectDraws,
    roll: &mut EffectRoll,
) -> Entity {
    let bones: Vec<usize> = body.parts.iter().map(|p| p.bone).collect();
    let mut parts = Vec::new();
    let mut strands = Vec::new();
    for (bone, anchor) in body_wisps(&bones) {
        if let Some(k) = bone_part(body, bone) {
            strands.push(Strand::new(anchor, Some(parts.len())));
            parts.push(k);
        }
    }
    let world = |s: &Strand| {
        s.body
            .and_then(|slot| parts.get(slot))
            .and_then(|&k| part_point(body, k, s.local))
            .unwrap_or(s.local)
    };
    let mut wisps = Wisps::new(strands, 12.0, world, || draws.0.unit() as f32);
    wisps.place(TICK_MS, world, &mut roll.0);
    commands
        .spawn((
            Name::new("_DisplacementWisps"),
            LiveWisps {
                wisps,
                body: caster,
                parts,
            },
        ))
        .id()
}

fn part_point(body: &Ragdoll, part: usize, local_cm: Vec3) -> Option<Vec3> {
    let p = body.parts.get(part)?;
    Some(p.transform().transform_point3(local_cm / CM) * CM)
}

fn bone_part(body: &Ragdoll, bone: usize) -> Option<usize> {
    body.parts.iter().position(|p| p.bone == bone)
}

/// Walk every caster's active list as the original's body tick does.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn tick_casters(
    mut commands: Commands,
    mut casters: Query<(
        &mut PowerCaster,
        Option<&Condition>,
        Option<&SwingState>,
        Option<&Locomotion>,
        Option<&Locomotor>,
        Option<&Ragdoll>,
        Option<&mut Animator>,
    )>,
    walls: Query<&LiveBarricade>,
    mut bubbles: Query<&mut LiveBubble>,
    ticks: Option<Res<BodyTicks>>,
) {
    let now = ticks.as_deref().map_or(0, |t| t.millis() as u32);
    for (mut caster, condition, swing, locomotion, locomotor, ragdoll, mut animator) in &mut casters
    {
        let caster = &mut *caster;
        refresh_body(
            &mut caster.body,
            condition,
            swing,
            locomotion,
            locomotor,
            None,
        );
        caster.take_states();
        let at = ragdoll.and_then(root_of).unwrap_or(Vec3::ZERO).to_array();
        let PowerCaster { body, active, .. } = caster;
        casting::settle_capacity(body);
        let mut index = 0;
        while index < active.len() {
            let entry = &mut active[index];
            let mut event = 0;
            if entry.state == ENDING {
                event = 2;
            }
            if let Some(entity) = entry.entity
                && let Ok(live) = walls.get(entity)
                && live.wall.state >= wall_state::TOLD_TO_END
            {
                event = 2;
            }
            let mut result = 0;
            match &mut entry.class {
                ActiveClass::Shell(power) => {
                    let step = power.control(event, body);
                    result = step.result;
                }
                ActiveClass::Kinesis(power) => {
                    let step = power.control(event, body, entry.tick_cost);
                    result = step.result;
                }
                ActiveClass::Throw(power) => {
                    if let Some((target, mass)) = entry.throw_target {
                        let view = super::power::force::ThrowView {
                            target_at: target,
                            mass,
                            caster_point: at,
                            tick_ms: TICK_MS,
                        };
                        let step = power.control(event, now, &view, body);
                        result = step.result;
                    }
                }
                ActiveClass::Strikes(power) => {
                    let step = power.control(event, body);
                    result = step.result;
                }
                ActiveClass::Remote(power) => {
                    let step = power.control(event, at, body);
                    result = step.result;
                }
                ActiveClass::Shock(power) => {
                    let step = power.control(event, body);
                    result = step.result;
                    if (step.fired || step.cleared)
                        && let Some(entity) = entry.entity.take()
                    {
                        commands.entity(entity).despawn();
                    }
                }
                ActiveClass::ShockWeapons(power) => {
                    let step = power.control(event, body);
                    result = step.result;
                }
                ActiveClass::Vanish(power) => {
                    let step = power.control(event, now);
                    if 0.0 < step.spend {
                        drain(body, step.spend);
                    }
                    result = step.result;
                }
                ActiveClass::LongDoor(power) => {
                    let nav = if power.destination.is_some() {
                        NavAnswer::Found
                    } else {
                        NavAnswer::Waiting
                    };
                    let step = power.control(now, nav);
                    if 0.0 < step.spend {
                        drain(body, step.spend);
                    }
                    result = step.result;
                }
                ActiveClass::Constant(power) => {
                    let step = power.control(event, body);
                    result = step.result;
                }
                ActiveClass::Summon(power) => {
                    let step = power.control(event, now, NavAnswer::Found, false);
                    result = step.result;
                }
            }
            if let Some(state) = entry.class.state() {
                entry.state = state;
            }
            if result & 2 != 0 {
                // The cast motion ends with the power: its id and hook go.
                if body.casting == entry.cast_id {
                    body.casting = 0;
                    body.cast_hook = None;
                    body.condition &= !casting::CASTING;
                    if let Some(animator) = animator.as_deref_mut() {
                        play_follow(animator);
                    }
                }
                let entry = active.swap_remove(index);
                if let Some(entity) = entry.entity
                    && let Ok(mut bubble) = bubbles.get_mut(entity)
                {
                    end_bubble(&mut bubble);
                }
                if let Some(entity) = entry.entity
                    && !matches!(entry.class, ActiveClass::Remote(_))
                {
                    commands.entity(entity).despawn();
                }
                continue;
            }
            index += 1;
        }
        casting::settle_focus(body);
        caster.sync_body();
    }
}

/// Return to the stance after a cast motion: the combat idle or the
/// defence stance when one was playing, else Idle. The original's motion
/// table's own follow-up is not in the packs; this is Exarust's, as its
/// attacks return to the combat idle.
fn play_follow(animator: &mut Animator) {
    let follow = if matches!(animator.action.as_str(), "Cmbt - Idle" | "Defence stance") {
        animator.action.clone()
    } else {
        "Idle".to_string()
    };
    animator.play_loop(&follow);
}

/// Play the cast motions the body recorded, by the motion id's action name.
fn play_cast_motions(
    mut casters: Query<(&mut PowerCaster, &mut Animator), Changed<PowerCaster>>,
    motions: Option<Res<Motions>>,
) {
    for (mut caster, mut animator) in &mut casters {
        let Some(motion) = caster.body.cast_motion.take() else {
            continue;
        };
        let follow = if matches!(animator.action.as_str(), "Cmbt - Idle" | "Defence stance") {
            animator.action.clone()
        } else {
            "Idle".to_string()
        };
        let Some(motions) = motions.as_deref() else {
            continue;
        };
        match motions.action_name(motion) {
            Some(name) => {
                let name = name.to_string();
                if motions.clip(&animator, &name).is_some_and(|c| c.loops()) {
                    animator.start(&name, true, None, 0, exa_anim::Ease::Linear);
                    debug!("cast motion {motion:#x}: {name}, looping");
                } else {
                    animator.play_once(&name, &follow);
                    debug!("cast motion {motion:#x}: {name} then {follow}");
                }
                caster.seen_action = Some(name);
            }
            None => debug!("cast motion {motion:#x}: no action of that name"),
        }
    }
}

/// The original's motion start calls the cast hook with event 4 when the
/// motion it replaces is not the walk, the run or an attack, and drops the
/// hook and power id on a negative answer; a one-shot cast clip's end
/// returns to the stance through it. Here, a caster's animator leaving
/// the action the hooks last saw.
fn call_motion_hooks(
    mut casters: Query<(&mut PowerCaster, &Animator)>,
    motions: Option<Res<Motions>>,
) {
    for (mut caster, animator) in &mut casters {
        if caster.seen_action.as_deref() == Some(animator.action.as_str()) {
            continue;
        }
        let left = caster.seen_action.replace(animator.action.clone());
        let index = left.and_then(|name| {
            motions
                .as_deref()
                .and_then(|motions| motions.0.action_index(&name))
        });
        let calls = index.is_some_and(|i| !matches!(i, 0 | 1 | 5..=12));
        if calls && super::casting::motion_replaced(&mut caster.body) {
            debug!("cast hook dropped: {} replaced", animator.action);
            caster.sync_body();
        }
    }
}

/// Keep the drawn bar and record on the caster's while the tree is closed.
/// While the tree is open the screen owns them; when it closes (its key or
/// Escape stored the bar into the record) the caster takes the screen's
/// record and bar back, so a power the tree learnt or a slot it moved
/// reaches the casts. The screen's Escape closing the bar wins over the
/// caster's open bar.
fn sync_screen_bar(
    mut screen: Option<ResMut<PowerScreen>>,
    mut casters: Query<&mut PowerCaster, With<PlayerBody>>,
    mut was_open: Local<bool>,
    mut synced_bar: Local<bool>,
) {
    let Some(screen) = screen.as_deref_mut() else {
        return;
    };
    let Some(mut caster) = casters.iter_mut().next() else {
        return;
    };
    if screen.open {
        *was_open = true;
        return;
    }
    if *was_open {
        *was_open = false;
        caster.record = screen.record.clone();
        caster.bar = screen.bar.clone();
        caster.bar_open = screen.bar_open;
        caster.known = known_from(&caster.record);
    } else if screen.bar_open != *synced_bar && caster.bar_open == *synced_bar {
        // The screen's Escape closed the bar.
        caster.bar_open = screen.bar_open;
    }
    *synced_bar = caster.bar_open;
    if screen.bar != caster.bar
        || screen.bar_open != caster.bar_open
        || screen.record != caster.record
    {
        screen.bar = caster.bar.clone();
        screen.bar_open = caster.bar_open;
        screen.record = caster.record.clone();
        screen.dirty = true;
    }
}

/// The powers a record has learnt, in its order.
fn known_from(record: &PowerRecord) -> Vec<KnownPower> {
    record
        .pairs
        .iter()
        .filter(|pair| pair.flags & super::tree::LEARNED != 0)
        .map(|pair| KnownPower {
            id: pair.power,
            word: KNOWN_WORD,
        })
        .collect()
}

/// Put a caster's bar and record on the screen resource.
fn store_on_screen(screen: &mut PowerScreen, caster: &PowerCaster) {
    if screen.open {
        return;
    }
    screen.bar = caster.bar.clone();
    screen.bar_open = caster.bar_open;
    screen.record = caster.record.clone();
    screen.dirty = true;
}

/// A body a test or a debug flag starts with no powers.
impl Default for PowerCaster {
    fn default() -> Self {
        Self::new(&[])
    }
}

/// Read a comma-separated power list (`0x201,0x401`, decimal too).
pub fn parse_power_list(text: &str) -> Option<Vec<u32>> {
    let mut powers = Vec::new();
    for part in text.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let value = if let Some(hex) = part.strip_prefix("0x").or_else(|| part.strip_prefix("0X")) {
            u32::from_str_radix(hex, 16).ok()?
        } else {
            part.parse::<u32>().ok()?
        };
        powers.push(value);
    }
    Some(powers)
}

#[cfg(test)]
#[path = "cast_test.rs"]
mod tests;
