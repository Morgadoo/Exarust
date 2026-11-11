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
//!   Displacement; a Mind word runs the Mind front end and its cast at the
//!   body the mind pick found ([`crate::mind::runtime::cast_mind`]), and a
//!   Spirit Form asks for its spirit body. The bar's Use on a body the
//!   player was handed by Control Mind ends the control.
//! - **The active list** ([`ActiveEntry`]): a started power becomes the
//!   class the original's creator makes — the shell, kinesis, throw and
//!   strikes; shock and shock weapons; vanish, the long door, constant
//!   displacement and the summon; a dome's remote power. The barricade's
//!   wall is raised by [`super::barricade`] itself. Every body tick walks the
//!   list in the original's order, passes control event 0 (or 2 when a power
//!   was told to end), keeps a power that answers a result with bit 2 clear
//!   and takes it out with the last entry in its place otherwise, releasing
//!   the focus it held. Before the walk the player's buttons ask the cast
//!   hook ([`casting::player_hooks`]) and, with a cast motion playing, the
//!   release flag ends the cast through it ([`casting::release_cast`]); a
//!   cast that ends while the body casts returns it to its stance. A cast
//!   motion the classes start is played through
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
//!   wall, a shock weapon's arcs (one on each held blade) and a constant
//!   displacement's wisps are spawned with the power and
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
use exa_combat::condition::WEAK_BELOW;
use exa_combat::swing::SwingState;
use exa_combat::{AttackAskedHook, AttackHook, Condition, Fighter, PowerFactors};
use exa_physics::Physics;
use exa_physics::body::locomotion::{Locomotion, Locomotor};
use exa_physics::body::{BodySystems, BodyTicks, OutOfScene, Ragdoll};

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
use super::live::{
    EffectDraws, EffectRoll, FrameEffectDraws, FrameEffectRoll, LiveArcs, LiveBarricade, LiveWisps,
};
use super::power::displacement::{
    ConstantDisplacement, LONG_DOOR, LongDoor, NavAnswer, Summon, Vanish,
};
use super::power::energy::{Shock, ShockWeapons};
use super::power::force::{BUBBLE, Kinesis, RemotePower, SHELL, Shell, Strikes, THROW, Throw};
use super::power::{
    ENDING, KNOWN_WORD, KnownPower, PowerBase, READY, RUNNING, drain, evaluate_domain,
};
use super::torrent_live::{new_torrent, torrent_view};
use super::tree::{
    DOMAIN_DISPLACEMENT, DOMAIN_ENERGY, DOMAIN_FORCE, DOMAIN_MIND, PowerBar, PowerRecord,
    RunningPower, SlotMark, TreeMode, TreeScreen, domain_of, word,
};
use super::visuals::{CreatorInput, EffectKind, active_object, creator, spawn_visual};
use super::wave_live::new_wave;
use super::wisps::{Strand, Wisps};
use crate::effects::blast::Blast;
use crate::effects::danger::Hazard;
use crate::hand::{Hand, Holding};
use crate::player::{CursorGround, PlayerBody};
use crate::power_screen::PowerScreen;

/// Centimetres a metre: the original's effects run in cm.
const CM: f32 = 100.0;
/// The right hand's bone.
const RIGHT_HAND: usize = 17;
/// The chest's bone.
const CHEST: usize = 9;
/// The body tick the focus and the clip lengths are read at.
pub(super) const TICK_MS: u32 = 12;
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
    /// Further live entities that end with this power (a shock weapon's arcs,
    /// one per armed blade). They are told to finish rather than dropped at
    /// once, so a tendril already in flight can leave.
    pub companions: Vec<Entity>,
    /// What a throw's body is doing, read before each tick. None leaves the
    /// throw waiting.
    pub throw_view: Option<super::power::force::ThrowView>,
}

impl ActiveEntry {
    /// The entry as the bar's marks read it.
    pub fn running(&self) -> RunningPower {
        RunningPower {
            id: self.id,
            state: self.state,
            mark: self.class.slot_mark(self.id),
        }
    }
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

    /// How the class marks its bar slot: the shell, kinesis, strikes,
    /// shock weapons, constant displacement and bubble light it whatever
    /// their state; the barricade, throw, shock, vanish, long door and
    /// summon keep the base power's rule.
    fn slot_mark(&self, id: u32) -> SlotMark {
        match self {
            ActiveClass::Shell(_)
            | ActiveClass::Kinesis(_)
            | ActiveClass::Strikes(_)
            | ActiveClass::ShockWeapons(_)
            | ActiveClass::Constant(_) => SlotMark::Lit,
            ActiveClass::Remote(_) if id & ID_MASK == BUBBLE & ID_MASK => SlotMark::Lit,
            _ => SlotMark::Base,
        }
    }

    /// Whether the class's end leaves the cast motion running: a kinesis
    /// whose body went to a throw.
    fn keeps_cast(&self) -> bool {
        matches!(self, ActiveClass::Kinesis(p) if p.handed)
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
    /// The body's power id's domain evaluator answered nothing when the
    /// duellist's answers were last written.
    pub evaluator_idle: bool,
    /// An event the original's status-effect broadcast passes every entry
    /// of the list, outside the focus tick's walk: 2 when the body starts
    /// to collapse or dies, 8 at a level change ([`status_event`]). Taken
    /// by the next walk, which runs it before its own.
    pub broadcast: Option<i32>,
    /// The body was dying (collapsing or dead) at the last walk: the
    /// broadcast's 2 goes once, when it starts.
    pub was_dying: bool,
    /// The power memory's creature list for the summoning: the character
    /// id of the demon it brought in, kept once the demon has arrived and
    /// never cleared (the AI's summoning advice refuses while it is set).
    pub summoned: Option<u32>,
    /// The body's two weapon hook slots, by the power id that set them
    /// last ([`HookSlots`]).
    pub hooks: HookSlots,
}

/// The body's weapon hook slots Force Strikes and Shock Weapons share: the
/// item hook (a weapon taken or put away) and the strike hook (a landed
/// blow). Strikes' activation takes both; Shock Weapons' takes the item
/// hook; either's end clears both, whoever set them. A power acts on a
/// weapon change or a blow only while the slot is its own, so a second of
/// the two cast over the first takes its weapon changes, and ending either
/// leaves the other deaf to them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HookSlots {
    pub item: Option<u32>,
    pub strike: Option<u32>,
}

impl HookSlots {
    /// Force Strikes' activation.
    pub fn take_strikes(&mut self) {
        self.item = Some(super::force::STRIKES);
        self.strike = Some(super::force::STRIKES);
    }

    /// Shock Weapons' activation.
    pub fn take_shock_weapons(&mut self) {
        self.item = Some(super::energy::SHOCK_WEAPONS);
    }

    /// Either power's end.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Whether `power` holds the item hook.
    pub fn item_is(&self, power: u32) -> bool {
        self.item == Some(power)
    }

    /// Whether `power` holds the strike hook.
    pub fn strike_is(&self, power: u32) -> bool {
        self.strike == Some(power)
    }
}

/// The status-effect broadcast's events: a body that starts to collapse or
/// dies (the collapse, the kill plane and the fall's death) passes 2, a
/// level change passes 8 to the player's list; the focus tick's own walk
/// passes 0 to the body's own entries.
pub mod status_event {
    pub const TICK: i32 = 0;
    pub const DOWN: i32 = 2;
    pub const LEVEL_CHANGE: i32 = 8;
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
            evaluator_idle: false,
            broadcast: None,
            was_dying: false,
            summoned: None,
            hooks: HookSlots::default(),
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

    /// What the armour pass's worn specials do to the caster's body: the
    /// item word's power-refusing bit is cleared and set again by a mind
    /// ward (special kind 5), and the focus regeneration bonus becomes the
    /// focus charm's (kind 7) or 0. The ward also tells every power running
    /// on the body to end; a body is given its caster at spawn, when none
    /// runs yet.
    pub fn apply_worn(&mut self, specials: &exa_formats::encumbrance::WornSpecials) {
        self.body.item_flags &= !super::tree::ITEM_REFUSES_POWERS;
        if specials.mind_ward {
            self.body.item_flags |= super::tree::ITEM_REFUSES_POWERS;
        }
        self.body.regen_bonus = specials.focus_regen;
    }

    /// Grants a power as the game's grant routine does: the record must
    /// hold Insight's seventh technique and the power's node must have a
    /// learnt prerequisite in `nodes`, after which the pair carries
    /// `0x9fff` and the known list is rebuilt. The record's power tree
    /// screen is the caller's. Returns whether the power was granted.
    pub fn grant(
        &mut self,
        nodes: &[exa_formats::power_tree::PowerNode],
        insight: &exa_formats::characters::Skill,
        power: u32,
    ) -> bool {
        if !super::tree::grant_power(nodes, insight, &mut self.record.pairs, power) {
            return false;
        }
        self.known = known_from(&self.record);
        true
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

/// The live force-shell aura on its effect node.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct LiveShellAura(pub crate::thaum::ForceShell);

/// A vanish effect asking the body it hides to be placed somewhere else,
/// after the caster tick that asked for it.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct Otherwhere {
    pub entity: Entity,
    /// The destination (cm).
    pub to: Vec3,
}

/// The plugin that runs the bar and the active powers.
pub struct CastPlugin;

impl Plugin for CastPlugin {
    fn build(&self, app: &mut App) {
        exa_physics::sim_app::add_frame_commands(app);
        super::veil_live::bridge_veil_types(app);
        super::summon_live::bridge_summon_types(app);
        app.add_message::<BarCommand>()
            .add_message::<exa_combat::veto::BlowPhased>()
            .add_message::<super::veil_live::VeilSounded>()
            .add_systems(PostUpdate, super::veil_live::sound_veils)
            // The constant displacement's veil: the start's collision bit,
            // then the restore hook and the phased blows of this step.
            .add_systems(
                FixedUpdate,
                (
                    super::veil_live::displace_bodies,
                    super::veil_live::restore_phased,
                    super::veil_live::phase_blows,
                )
                    .chain()
                    .after(tick_casters)
                    .after(exa_combat::CombatSystems),
            )
            // An AI's Long Door and Summon ask the grid before the walk
            // runs them; a summon whose place is known asks for its demon,
            // and one whose demon's body exists takes it.
            .add_systems(
                FixedUpdate,
                (
                    super::long_door_live::pick_long_doors,
                    super::summon_live::ask_demons,
                    super::summon_live::arrive_demons,
                )
                    .chain()
                    .before(tick_casters)
                    .after(BodySystems),
            )
            .add_systems(
                FixedUpdate,
                (
                    super::summon_live::release_demons,
                    super::summon_live::end_demon_wisps,
                )
                    .after(tick_casters)
                    .before(super::vanish_live::step_vanishes),
            )
            .add_message::<super::summon_live::DemonAsked>()
            .add_message::<super::live::ArcShocked>()
            .add_message::<Otherwhere>()
            .add_message::<exa_ai::presence::Entered>()
            .add_message::<AttackAskedHook>()
            .add_message::<super::kinesis_live::KinesisPicked>()
            .add_message::<super::throw_live::ThrowFade>()
            .add_message::<super::hold_live::HoldEnd>()
            .add_message::<exa_combat::impact::LimbImpact>()
            .add_message::<exa_combat::impact::ShellHit>()
            .add_message::<exa_combat::StrikeHooked>()
            .init_resource::<super::kinesis_live::KinesisPickHook>()
            .init_resource::<super::kinesis_live::KinesisCursor>()
            .init_resource::<super::kinesis_live::KinesisSight>()
            .init_resource::<super::kinesis_live::KinesisKeys>()
            .init_resource::<super::throw_live::ThrowPick>()
            .init_resource::<EffectDraws>()
            .init_resource::<EffectRoll>()
            .init_resource::<FrameEffectDraws>()
            .init_resource::<FrameEffectRoll>()
            .init_resource::<super::debug_cast::DebugCasts>()
            // What the bar's Mind casts read and write (the mind runtime's
            // plugin runs them).
            .init_resource::<crate::mind::runtime::MindPick>()
            .init_resource::<crate::mind::runtime::MindDeadPick>()
            .add_message::<exa_audio::PointSounded>()
            .add_message::<exa_audio::MusicOverlay>()
            .init_resource::<crate::mind::runtime::MindAim>()
            .init_resource::<crate::mind::runtime::MindControl>()
            .add_message::<crate::mind::runtime::SpiritBodyAsked>()
            .add_systems(Update, (run_bar_commands, sync_screen_bar).chain())
            .add_message::<exa_script::Narrate>()
            .add_systems(
                Update,
                (
                    super::effect_draw::ripple_shell_hits,
                    super::effect_draw::ripple_strikes,
                ),
            )
            .add_systems(Update, super::power::teach_on_death)
            .add_systems(
                Update,
                super::debug_cast::run_debug_casts
                    .after(crate::player::cursor_ground)
                    .before(run_bar_commands),
            )
            .add_systems(
                Update,
                (
                    super::kinesis_live::read_kinesis_input,
                    super::throw_live::pick_throw,
                )
                    .chain()
                    .before(run_bar_commands),
            )
            .add_systems(
                Update,
                super::kinesis_live::see_kinesis_reach
                    .after(super::kinesis_live::read_kinesis_input)
                    .before(crate::hand::HandSystems),
            )
            // The duellists' power system answers their combat state and
            // casts what it chose, in the brains' tick, every body tick.
            .add_systems(
                exa_ai::BrainChain,
                (
                    advise_duellists
                        .after(exa_ai::brain::ai_tick)
                        .after(crate::role_scripts::run_encounter_scripts)
                        .before(DuelSystems),
                    cast_for_duellists
                        .after(DuelSystems)
                        .before(exa_ai::brain::flee_walk),
                ),
            )
            .add_systems(Update, publish_power_factors.after(run_bar_commands))
            .add_systems(
                Update,
                broadcast_level_change.run_if(resource_exists::<
                    Messages<exa_world::level_lifecycle::LevelChangeRequest>,
                >),
            )
            .add_systems(
                FixedUpdate,
                (
                    reset_entered_casts
                        .after(exa_ai::presence::check_presence)
                        .before(exa_ai::BrainTick),
                    answer_attack_asks
                        .after(exa_ai::BrainTick)
                        .before(tick_casters),
                    super::kinesis_live::take_kinesis_picks
                        .after(exa_physics::body::TickBehaviour)
                        .before(BodySystems)
                        .before(tick_casters),
                    super::throw_live::sense_throws
                        .before(BodySystems)
                        .before(tick_casters)
                        .after(super::kinesis_live::take_kinesis_picks),
                    super::hold_live::grab_thrown
                        .before(BodySystems)
                        .before(tick_casters)
                        .after(super::throw_live::sense_throws),
                    super::user_stamp::blame_impacts.after(exa_combat::impact::strike_limbs),
                    shock_weapon_hits
                        .after(exa_combat::impact::strike_limbs)
                        .before(super::live::shock_fighters),
                ),
            )
            .add_systems(
                FixedUpdate,
                (
                    tick_casters,
                    follow_weapon_changes.after(tick_casters),
                    apply_otherwhere.after(tick_casters),
                    super::hold_live::end_holds,
                    super::hold_live::dress_strikes,
                    tick_shell_auras.after(tick_casters),
                    super::vanish_live::step_vanishes.after(tick_casters),
                    super::kinesis_live::steer_kinesis,
                    super::throw_live::push_throws,
                    super::throw_live::sound_throws,
                    super::kinesis_live::arm_kinesis_pick,
                    super::blast_live::tick_blasts,
                    super::barrier_live::tick_barriers,
                    super::bubble_live::tick_bubbles,
                    super::wave_live::tick_waves,
                    crate::effects::danger::sync_hazards,
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
    /// The throwable body and part under the player's cursor.
    throw_pick: Option<Entity>,
    /// The barricade's ground pick under the player's cursor
    /// ([`CastPlace::barricade_ground`]).
    barricade_ground: Option<Option<GroundHit>>,
}

/// One caster's bar command.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn run_bar_commands(
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
    throw_pick: Option<Res<super::throw_live::ThrowPick>>,
    physics: Option<Res<exa_physics::collision_share::FramePhysics>>,
    ticks: Option<Res<BodyTicks>>,
    motions: Option<Res<Motions>>,
    mut draws: ResMut<FrameEffectDraws>,
    mut roll: ResMut<FrameEffectRoll>,
    mut screen: Option<ResMut<PowerScreen>>,
    weapons: Query<(
        Entity,
        &exa_combat::Blade,
        Option<&Ragdoll>,
        Option<&GlobalTransform>,
    )>,
    mut mind: MindBar,
    picker: super::cast_pick::CastPicker,
    walls: Query<&super::live::LiveBarricade>,
) {
    for command in commands_in.read().copied() {
        // The bar on a body the player was handed: its cast entry asks the
        // caller to end the control, which ends the held body's Control Mind.
        if !casters.contains(command.caster)
            && command.action == BarAction::Use
            && mind
                .minds
                .get(command.caster)
                .is_ok_and(|m| m.cast.status_bits & CONTROLLING != 0)
        {
            info!("power cast while controlling another body: the control ends");
            commands.queue(|world: &mut World| {
                crate::mind::runtime::end_control(world);
            });
            continue;
        }
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
        // While the tree form is open, its own key handler owns the bar's
        // toggle and step keys.
        if screen.as_deref().is_some_and(|s| s.open)
            && matches!(command.action, BarAction::Toggle | BarAction::Step(_))
        {
            continue;
        }
        match command.action {
            BarAction::Toggle => {
                if caster.bar_open {
                    caster.bar.store(&mut caster.record, false);
                    caster.bar_open = false;
                } else if !caster.known.is_empty() {
                    let running: Vec<RunningPower> =
                        caster.active.iter().map(ActiveEntry::running).collect();
                    let now = ticks.as_deref().map_or(0, |t| t.millis() as u32);
                    caster.bar.load_running(&caster.record, &running, now);
                    caster.bar_open = true;
                }
            }
            BarAction::Pick(slot) => {
                if caster.bar_open && caster.bar.select(slot) {
                    caster.bar.store(&mut caster.record, false);
                    if let Some(screen) = screen.as_deref_mut() {
                        screen.sound = Some(crate::thaum::tree::SLOT_SELECT_SOUND);
                    }
                }
            }
            BarAction::Step(direction) => {
                if caster.bar.cycle(direction, Some(&mut caster.record)) {
                    caster.bar.store(&mut caster.record, false);
                    if let Some(screen) = screen.as_deref_mut() {
                        screen.sound = Some(crate::thaum::tree::SLOT_SELECT_SOUND);
                    }
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
                let picked = throw_pick.as_deref().and_then(|p| p.0);
                let kinesis_holds = caster.active.iter().any(|entry| {
                    matches!(&entry.class, ActiveClass::Kinesis(k)
                        if entry.state != ENDING && k.target.is_some())
                });
                // The barricade's cast asks the nearest-mesh pick: the
                // segment from the eye to its point meeting a running
                // barricade's node bounds ends that barricade.
                let barricades: Vec<Entity> = caster
                    .active
                    .iter()
                    .filter(|entry| entry.id == BARRICADE)
                    .filter_map(|entry| entry.entity)
                    .collect();
                let cursor_on_barricade = !barricades.is_empty()
                    && picker
                        .eye()
                        .zip(picker.mesh_point())
                        .is_some_and(|(eye, point)| {
                            barricades.iter().any(|&wall| {
                                walls.get(wall).is_ok_and(|live| {
                                    super::cast_pick::segment_meets_wall(eye, point, &live.wall)
                                })
                            })
                        });
                let world = ForceWorld {
                    controls_busy: busy,
                    possessing,
                    cursor_on_barricade,
                    throw_pick: picked.is_some(),
                    kinesis_holds,
                };
                // The long door and the summoning take the ground pick
                // under the cursor, every node accepted. Without a cursor
                // ray (a scripted `--debug-cast` without `--cursor`) the
                // aim plane's point stands in, taken as it is.
                let cursor = if picker.ray().is_some() {
                    picker.ground(|_| true).map(|hit| CursorHit {
                        point: hit.point * CM,
                        direction: hit.direction,
                        exact: hit.exact(),
                    })
                } else {
                    ground.as_deref().and_then(|g| g.0).map(|point| CursorHit {
                        point: point * CM,
                        direction: Vec3::NEG_Y,
                        exact: true,
                    })
                };
                debug!(
                    "cast picks: ground {:?}, on a barricade {cursor_on_barricade}",
                    cursor.map(|c| (c.point(), c.exact))
                );
                let aim = DisplacementAim {
                    player: true,
                    cursor,
                    possessing,
                };
                let slot = chosen_slot(caster);
                let Some(slot) = slot else {
                    // A digit on the already chosen slot unpicks it (the
                    // bar's select toggles), which leaves nothing to cast.
                    info!("power cast: no slot chosen");
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
                let mut mind_outcome = None;
                let mind_scene = crate::mind::casts::MindScene {
                    now_ms: now,
                    real_ms: now,
                    relation: 0.0,
                    spirit_screen: mind.control.spirit_screen.map(|s| s.state),
                    loud_music: false,
                };
                // The dead powers pick with the collapsed allowed.
                let mind_target = match asked & ID_MASK {
                    crate::mind::casts::RAISE_DEAD | crate::mind::casts::COMMAND_DEAD => {
                        mind.dead_pick.0
                    }
                    _ => mind.pick.0,
                };
                let entry = tree.cast_slot(
                    &mut caster.record,
                    &mut caster.bar,
                    Some(slot),
                    &mut caster.body,
                    now,
                    |domain, asked, body| {
                        if domain == DOMAIN_MIND {
                            let answer = crate::mind::runtime::cast_mind(
                                asked,
                                body,
                                entity,
                                mind_target,
                                &mut mind.minds,
                                &mind_scene,
                            );
                            mind_outcome = answer.outcome;
                            result_code = answer.code;
                            return answer.code;
                        }
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
                    info!("power cast while controlling another body: the control ends");
                    commands.queue(|world: &mut World| {
                        crate::mind::runtime::end_control(world);
                    });
                }
                if let Some(queued) = entry.queued_slot {
                    debug!("cast queued on slot {queued}");
                }
                info!(
                    "power cast {asked:#x} from slot {slot}: result {result_code} \
                     (weapon class {:#x}, focus {:.3}, slots {}, condition {:#x})",
                    caster.body.weapon_class,
                    caster.body.focus,
                    caster.body.slots,
                    caster.body.condition
                );
                if let Some(outcome) = mind_outcome {
                    info!(
                        "mind cast {asked:#x} at {mind_target:?}: code {}, sound {:?}, request {:?}",
                        outcome.code, outcome.sound, outcome.request
                    );
                    if let Some(sound) = outcome.sound {
                        crate::mind::runtime::play_mind_sound(&mut mind.sounds, sound);
                    }
                    if let crate::mind::casts::MindRequest::SpiritForm { music_volume } =
                        outcome.request
                    {
                        mind.music.write(exa_audio::MusicOverlay::In {
                            name: crate::mind::casts::SPIRIT_MUSIC.to_string(),
                            volume: music_volume,
                        });
                        // The spirit screen's start plays its own sound.
                        crate::mind::runtime::play_mind_sound(
                            &mut mind.sounds,
                            crate::mind::casts::MindSound {
                                name: crate::mind::sight::SPIRIT_SCREEN_SOUND,
                                volume: crate::mind::sight::SPIRIT_SCREEN_VOLUME,
                            },
                        );
                    }
                    if crate::mind::runtime::spirit_asked(&outcome) {
                        let feet = mind
                            .places
                            .get(entity)
                            .map_or(Vec3::ZERO, GlobalTransform::translation);
                        let yaw = locomotor.map_or(0.0, |l| l.yaw);
                        mind.spirits
                            .write(crate::mind::runtime::spirit_request(feet, yaw, entity));
                    }
                }
                // The barricade's placement takes the ground pick under the
                // cursor, its accept test taking the nodes without an
                // object; without a cursor ray the aim plane's point stands
                // in, as for the long door.
                let barricade_ground = if picker.ray().is_some() {
                    Some(picker.ground(|object| !object).map(|hit| GroundHit {
                        point: hit.point * CM,
                        direction: hit.direction,
                        exact: hit.exact(),
                    }))
                } else {
                    None
                };
                debug!("cast picks: the barricade's ground {barricade_ground:?}");
                let place = CastPlace {
                    now,
                    ragdoll,
                    yaw: locomotor.map_or(0.0, |l| l.yaw),
                    cursor: ground.as_deref().and_then(|g| g.0).map(|p| p * CM),
                    clip_ms: shot_clip_ms(motions.as_deref(), animator.as_deref()),
                    target: None,
                    // The creators place the domes on the body's standing
                    // point, its transform's place (as the duellists' casts
                    // take it), not its first part.
                    standing: mind.places.get(entity).ok().map(|at| at.translation() * CM),
                    throw_pick: picked,
                    barricade_ground,
                };
                let armed = weapon_arcs_of(entity, &weapons);
                begin_cast(
                    &mut commands,
                    entity,
                    caster,
                    entry.result > 0,
                    start,
                    &place,
                    physics.as_deref().map(|p| &p.0),
                    &armed,
                    &mut draws,
                    &mut roll,
                );
                if let Some(screen) = screen.as_deref_mut() {
                    if let Some(sound) = entry.sound {
                        screen.sound = Some(sound);
                    }
                    store_on_screen(screen, caster);
                }
            }
        }
    }
}

/// What the bar's Mind casts read and write: the bodies' minds, the
/// mind pick, the runtime's state, the spirit-body request and the bodies'
/// places.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct MindBar<'w, 's> {
    minds: Query<'w, 's, &'static mut crate::mind::runtime::MindBody>,
    pick: Res<'w, crate::mind::runtime::MindPick>,
    dead_pick: Res<'w, crate::mind::runtime::MindDeadPick>,
    sounds: MessageWriter<'w, exa_audio::PointSounded>,
    music: MessageWriter<'w, exa_audio::MusicOverlay>,
    control: Res<'w, crate::mind::runtime::MindControl>,
    spirits: MessageWriter<'w, crate::mind::runtime::SpiritBodyAsked>,
    places: Query<'w, 's, &'static GlobalTransform>,
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
    throw_pick: Option<Entity>,
    /// The barricade's ground pick under the player's cursor: `Some(None)`
    /// when the pick met nothing (no barricade is placed), `None` when the
    /// caster has no cursor ray (the AI's, or a scripted cast).
    barricade_ground: Option<Option<GroundHit>>,
}

/// After a domain's front end: a started power gets the class its creator makes, and the body's power
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
    weapons: &[WeaponArc],
    draws: &mut EffectDraws,
    roll: &mut EffectRoll,
) {
    caster.take_states();
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
            throw_pick: place.throw_pick,
            barricade_ground: place.barricade_ground,
        };
        let made = create_active(
            commands,
            entity,
            caster,
            start,
            &context,
            place.ragdoll,
            physics,
            weapons,
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
        Option<&exa_ai::presence::Presence>,
    )>,
    places: Query<&GlobalTransform>,
    children: Query<&Children>,
    targets: Query<(Option<&Condition>, Option<&PowerCaster>), Without<DuelPowers>>,
) {
    for (
        mut powers,
        mut caster,
        duellist,
        at,
        condition,
        swing,
        locomotion,
        locomotor,
        fighter,
        presence,
    ) in &mut duellists
    {
        // Exarust's own: a hidden duellist advises nothing, so no power
        // leaves a body that is out of the scene.
        if presence.is_some_and(|p| !p.moves()) {
            continue;
        }
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
            // The power memory holds a creature list for the summoning once
            // a demon has arrived; the original never checks that it lives.
            summoned: caster.summoned.is_some(),
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
        // The combat state's stop asks the body's power id's evaluator, and
        // the hook with 0x10 only when that answers nothing. [inf: a missing
        // hook, which the original would call through, answers −1.]
        caster.evaluator_idle = evaluate_domain(caster.body.casting, &view) == 0;
        let keeps = caster
            .body
            .cast_hook
            .is_some_and(|hook| hook.answer(casting::hook_event::RELEASE) >= 0);
        let advised = DuelPowers {
            slots,
            answers,
            pool: caster.body.focus,
            casting: caster.body.casting != 0,
            running: !caster.evaluator_idle || keeps,
            refuses: caster.body.item_flags & super::tree::ITEM_REFUSES_POWERS != 0,
            probed: powers.probed,
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
        Option<&mut Animator>,
        Option<&HeldMoveset>,
        Option<&Ragdoll>,
        Option<&exa_ai::presence::Presence>,
    )>,
    physics: Option<Res<Physics>>,
    ticks: Option<Res<BodyTicks>>,
    motions: Option<Res<Motions>>,
    target_bodies: Query<&Ragdoll>,
    places: Query<&GlobalTransform>,
    mut draws: ResMut<EffectDraws>,
    mut roll: ResMut<EffectRoll>,
    weapons: Query<(
        Entity,
        &exa_combat::Blade,
        Option<&Ragdoll>,
        Option<&GlobalTransform>,
    )>,
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
        mut animator,
        held,
        ragdoll,
        presence,
    ) in &mut duellists
    {
        // Exarust's own: a hidden duellist starts no cast and lets none go
        // while its body is out of the scene; entering clears its cast.
        if presence.is_some_and(|p| !p.moves()) {
            continue;
        }
        let caster = &mut *caster;
        if std::mem::take(&mut powers.probed)
            && caster.evaluator_idle
            && (casting::ask_hook(&mut caster.body, casting::hook_event::RELEASE)
                .is_some_and(|answer| answer < 0)
                || caster.body.cast_hook.is_none())
        {
            casting::end_cast(&mut caster.body);
            debug!("duellist {entity:?} lets its cast go");
            if casting::take_stance_return(&mut caster.body)
                && let Some(animator) = animator.as_deref_mut()
            {
                casting::play_follow(animator);
            }
        }
        let Some(asked) = powers.chosen.take() else {
            continue;
        };
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
        // The AI aims at its target's chest, as its first blast or torrent
        // view must too.
        let cursor = target_chest(&target_bodies, &places, duellist.target);
        let place = CastPlace {
            now,
            ragdoll,
            yaw: facing.x.atan2(facing.z),
            cursor,
            clip_ms: shot_clip_ms(motions.as_deref(), animator.as_deref()),
            target: Some(duellist.target),
            standing: Some(at.translation() * CM),
            throw_pick: None,
            barricade_ground: None,
        };
        let armed = weapon_arcs_of(entity, &weapons);
        begin_cast(
            &mut commands,
            entity,
            caster,
            result > 0,
            start,
            &place,
            physics.as_deref(),
            &armed,
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
    mut casters: Query<(
        Entity,
        &PowerCaster,
        Option<&mut PowerFactors>,
        Option<&mut AttackHook>,
    )>,
) {
    for (entity, caster, factors, hook) in &mut casters {
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
        let gate = AttackHook(
            caster
                .body
                .cast_hook
                .map(|h| h.answer(AttackHook::EVENT) >= 0),
        );
        match hook {
            Some(mut hook) => {
                hook.set_if_neq(gate);
            }
            None => {
                commands.entity(entity).insert(gate);
            }
        }
    }
}

/// An attack start asked the body's cast hook with event 0x100: a negative
/// answer ends the cast.
fn answer_attack_asks(
    mut asked: MessageReader<AttackAskedHook>,
    mut casters: Query<&mut PowerCaster>,
) {
    for AttackAskedHook(entity) in asked.read() {
        if let Ok(mut caster) = casters.get_mut(*entity)
            && casting::ask_hook(&mut caster.body, AttackHook::EVENT).is_some_and(|a| a < 0)
        {
            casting::end_cast(&mut caster.body);
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

/// The chest (cm) of the body `body` or, without one, the place `place`.
fn chest_from(body: Option<&Ragdoll>, place: Option<&GlobalTransform>) -> Option<Vec3> {
    body.and_then(super::blast_live::chest_point)
        .or_else(|| place.map(|p| p.translation() * CM))
}

/// The chest (cm) of `target`, from its body else its transform.
fn target_chest(
    bodies: &Query<&Ragdoll>,
    places: &Query<&GlobalTransform>,
    target: Entity,
) -> Option<Vec3> {
    chest_from(bodies.get(target).ok(), places.get(target).ok())
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
    weapons: &[WeaponArc],
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
            weapons,
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
            let generation = caster.body.cast_generation;
            let mut blast = Blast::new(setup, cost, TICK_MS, &view, &mut draws.0);
            blast.link();
            // The start's charge loop takes its start from the next draw.
            let charge_roll = draws.0.unit() as f32;
            let hazard = Hazard::Blast {
                at: blast.centre,
                released: blast.state >= crate::effects::blast::THROWN,
            };
            let entity = commands
                .spawn((
                    Name::new("_ForceBlast"),
                    LiveBlast {
                        blast,
                        caster: caster_entity,
                        target: context.target,
                        hooked,
                        generation,
                        contact_scale: 0.0,
                        sounds: super::blast_sound::BlastSounds {
                            charge_roll,
                            release_pitch: None,
                        },
                    },
                    hazard,
                ))
                .id();
            let kind = if setup.power == 0x202 {
                EffectKind::Bolt
            } else {
                EffectKind::Blast
            };
            attach_visual(commands, entity, kind, context, ragdoll);
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
            let hazard = Hazard::Wave {
                at: live.wave.centre,
                released: false,
            };
            let entity = commands.spawn((Name::new("_ForceWave"), live, hazard)).id();
            attach_visual(commands, entity, EffectKind::Wave, context, ragdoll);
            debug!("force wave: motion {hooked}, entity {entity:?}");
            Some(entity)
        }
        ForceStart::Torrent { cost } => {
            let body = ragdoll?;
            let view = torrent_view(body, context.heading, context.cursor)?;
            let mut live = new_torrent(
                &mut caster.body,
                caster_entity,
                context.target,
                cost,
                &view,
                TICK_MS,
                draws,
                roll,
            );
            live.torrent.link();
            let hooked = live.hooked;
            let entity = commands.spawn((Name::new("_ForceTorrent"), live)).id();
            attach_visual(commands, entity, EffectKind::Torrent, context, ragdoll);
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
            attach_visual(commands, entity, EffectKind::Barrier, context, ragdoll);
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
            attach_visual(commands, entity, EffectKind::Bubble, context, ragdoll);
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
            // The player's placement needs the ground pick to meet
            // something; with nothing met the original places nothing.
            if context.barricade_ground == Some(None) {
                debug!("force barricade: the ground pick meets nothing");
                return None;
            }
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
            // The creator constructs the power (and its aura, which draws
            // its two texture positions) before it draws the start sound's
            // pitch.
            let tex_pos = [roll.0.unit(), roll.0.unit()];
            let start_pitch = roll.0.unit();
            let mut aura = crate::thaum::ForceShell::new(context.now, tex_pos);
            aura.voice = true;
            let spec = creator(
                EffectKind::Shell,
                &visual_input(context, ragdoll, start_pitch),
            );
            let entity = spawn_visual(commands, spec);
            commands.entity(entity).insert((
                LiveShellAura(aura),
                super::effect_draw::ShellAuraOf(caster_entity),
            ));
            push_active(
                caster,
                SHELL,
                SHELL,
                READY,
                ActiveClass::Shell(power),
                0.0,
                Some(entity),
            );
            info!("force shell: aura entity {entity:?}");
            Some(entity)
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
            None
        }
        ForceStart::Throw { cost, from_kinesis } => {
            let mut cast_id = THROW;
            let mut power = Throw::new(cost);
            power.handed = from_kinesis;
            power.sound.pick = !from_kinesis;
            if from_kinesis {
                let PowerCaster { body, active, .. } = &mut *caster;
                for entry in active.iter_mut() {
                    if let ActiveClass::Kinesis(kinesis) = &mut entry.class
                        && kinesis.state != ENDING
                    {
                        if let Some(target) = kinesis.hand_off(body) {
                            power.body = Some(target);
                        }
                        // The shell and the held environment go with it.
                        power.shell = kinesis.shell.take();
                        power.material = kinesis.material.take();
                        power.grabbed = true;
                        entry.state = ENDING;
                        cast_id = KINESIS;
                    }
                }
            } else {
                power.body = context.throw_pick.map(Entity::to_bits);
            }
            debug!(
                "force throw (from kinesis: {from_kinesis}) of {:?}",
                power.body
            );
            push_active(
                caster,
                THROW,
                cast_id,
                READY,
                ActiveClass::Throw(power),
                0.0,
                None,
            );
            None
        }
        ForceStart::Strikes { motion: false, .. } => None,
        ForceStart::Strikes { cost, motion: true } => {
            let mut power = Strikes::new();
            let held = {
                let body = &mut caster.body;
                power.apply(body, &[]);
                let held = PowerBase::new(super::force::STRIKES).accrue(body, cost);
                body.cast_hook = Some(super::force::STRIKES_HOOK);
                held
            };
            caster.hooks.take_strikes();
            power.held = held;
            let spec = creator(
                EffectKind::Strikes,
                &visual_input(context, ragdoll, roll.0.unit()),
            );
            let entity = spawn_visual(commands, spec);
            push_active(
                caster,
                super::force::STRIKES,
                super::force::STRIKES,
                READY,
                ActiveClass::Strikes(power),
                0.0,
                Some(entity),
            );
            debug!("force strikes: aura entity {entity:?}");
            Some(entity)
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
    weapons: &[WeaponArc],
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
                "energy shock: charge {:.3} a tick over {:.3}",
                charge.length, charge.cost
            );
            entity
        }
        EnergyStart::ShockWeapons { cost } => {
            let mut power = ShockWeapons::new();
            power.held =
                PowerBase::new(super::energy::SHOCK_WEAPONS).accrue(&mut caster.body, cost);
            let armed = spawn_weapon_arcs(commands, caster_entity, weapons, draws);
            power.count = armed.len();
            caster.hooks.take_shock_weapons();
            push_active(
                caster,
                super::energy::SHOCK_WEAPONS,
                super::energy::SHOCK_WEAPONS,
                READY,
                ActiveClass::ShockWeapons(power),
                0.0,
                None,
            );
            if let Some(entry) = caster.active.last_mut() {
                entry.companions = armed;
            }
            debug!(
                "energy shock weapons: {} arcs on the held blades",
                caster
                    .active
                    .last()
                    .map_or(0, |entry| entry.companions.len())
            );
            None
        }
    }
}

/// Whether the caster's power list holds a running Constant Displacement,
/// whose shell a vanish or a Long Door then takes.
fn constant_running(caster: &PowerCaster) -> bool {
    caster
        .active
        .iter()
        .any(|entry| entry.id == super::displacement::CONSTANT)
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
            // A running constant displacement lends the vanish its shell.
            let shared = constant_running(caster);
            let mut power = Vanish::new(context.now, shared, false, cost);
            power.take_hook(&mut caster.body);
            let input = visual_input(context, ragdoll, 0.0);
            // The creator adds its own node unless its activation made one.
            let entity = active_object(EffectKind::Vanish, &input, cost, TICK_MS as f32, false)
                .filter(|object| object.node.adds(false))
                .map(|_| spawn_visual(commands, creator(EffectKind::Vanish, &input)));
            push_active(
                caster,
                0x601,
                super::displacement::VANISH,
                READY,
                ActiveClass::Vanish(power),
                0.0,
                entity,
            );
            if let Some(entity) = entity {
                // The vanish effect builds the body's displacement shell,
                // whose constructor draws its two texture positions, unless
                // it takes the constant displacement's.
                let tex_pos = if shared {
                    [0.0; 2]
                } else {
                    [roll.0.unit(), roll.0.unit()]
                };
                commands
                    .entity(entity)
                    .insert(super::vanish_live::LiveVanish::waiting(
                        caster_entity,
                        tex_pos,
                    ));
            }
            debug!("displacement vanish: effect node {entity:?}");
            entity
        }
        DisplacementStart::LongDoor {
            cost,
            tick_cost,
            to,
        } => {
            // No destination at the cast: the AI's door, which asks the
            // navigation grid from its first phase (`long_door_live`).
            let ai = to.is_none();
            let mut power = LongDoor::new(context.now, to.map(|p| p.to_array()), ai, tick_cost);
            // A running constant displacement lends the door its shell.
            let shared = constant_running(caster);
            power.base.shared = shared;
            if let Some(head) = ragdoll.and_then(|r| r.parts.first()) {
                let at = head.transform().translation * CM;
                power.follow(Vec3::from_array([at.x, at.y, at.z]));
            }
            caster.body.cast_hook = Some(super::power::displacement::LONG_DOOR_HOOK);
            let mut input = visual_input(context, ragdoll, 0.0);
            if let Some(to) = to {
                input.point = Some(to);
            }
            let entity = active_object(EffectKind::LongDoor, &input, cost, TICK_MS as f32, false)
                .filter(|object| object.node.adds(false))
                .map(|_| spawn_visual(commands, creator(EffectKind::LongDoor, &input)));
            push_active(
                caster,
                LONG_DOOR,
                LONG_DOOR,
                READY,
                ActiveClass::LongDoor(power),
                0.0,
                entity,
            );
            if let Some(entity) = entity {
                // The Long Door runs the same vanish effect and shell.
                let tex_pos = if shared {
                    [0.0; 2]
                } else {
                    [roll.0.unit(), roll.0.unit()]
                };
                commands
                    .entity(entity)
                    .insert(super::vanish_live::LiveVanish::waiting(
                        caster_entity,
                        tex_pos,
                    ));
            }
            debug!("displacement long door: effect node {entity:?}");
            entity
        }
        DisplacementStart::Constant { cost } => {
            let mut power = ConstantDisplacement::new();
            power.held =
                PowerBase::new(super::displacement::CONSTANT).accrue(&mut caster.body, cost);
            // The start makes the body's displacement shell first (its
            // constructor's two draws), then the tendril image.
            let tex_pos = [roll.0.unit(), roll.0.unit()];
            commands.spawn((
                Name::new("_EffectShell01 constant"),
                super::vanish_live::ConstantShell::waiting(caster_entity, tex_pos),
                ChildOf(caster_entity),
            ));
            let entity =
                ragdoll.map(|body| spawn_body_wisps(commands, caster_entity, body, draws, roll));
            if let Some(entity) = entity {
                // The hum's start draw, after the tendril image's.
                let hum_roll = roll.0.unit();
                let node = spawn_visual(
                    commands,
                    creator(
                        EffectKind::Constant,
                        &visual_input(context, ragdoll, hum_roll),
                    ),
                );
                commands.entity(entity).add_child(node);
                // The start puts the phasing veto (a fresh streak) and the
                // restore hook on the running body, and its collision bit
                // ([`super::veil_live`]); none of them is taken away when
                // the power ends.
                commands.entity(caster_entity).insert((
                    exa_combat::veto::PhaseVeto::default(),
                    super::veil_live::LiveVeil {
                        hum_roll,
                        ..Default::default()
                    },
                ));
            }
            push_active(
                caster,
                super::displacement::CONSTANT,
                super::displacement::CONSTANT,
                READY,
                ActiveClass::Constant(power),
                0.0,
                entity,
            );
            debug!("constant displacement: wisps on the body, the veil on {caster_entity:?}");
            entity
        }
        DisplacementStart::Summon { at, .. } => {
            // The player's summon knows its place (the cursor's) and faces
            // the caster from it; the AI's picks its place through the
            // grid first (`summon_live`).
            let power = match at {
                Some(place) => {
                    let forward = context.heading;
                    let yaw = exa_physics::engine_math::heading(forward);
                    Summon::player(context.now, place, context.standing, forward, yaw)
                }
                None => Summon::ai(context.now),
            };
            caster.body.cast_hook = Some(super::power::displacement::SUMMON_HOOK);
            // The summoning's creator adds no node: the class builds its
            // vanish effect over the demon when it arrives.
            push_active(
                caster,
                super::displacement::SUMMON,
                super::displacement::SUMMON,
                READY,
                ActiveClass::Summon(power),
                0.0,
                None,
            );
            info!(
                "displacement summon by {caster_entity:?}: phase {:#x}, place {:?} cm",
                power.phase, power.place
            );
            None
        }
    }
}

/// The creator's view of the caster: its feet, hand, chest, body
/// orientation and heading, and the cursor point. The hand and chest are
/// the body's own parts; a body without them sits at its feet.
fn visual_input(context: &CastContext, ragdoll: Option<&Ragdoll>, roll: f32) -> CreatorInput {
    let body = ragdoll.and_then(root_of).unwrap_or(context.standing);
    let part_at = |bone: usize| {
        let doll = ragdoll?;
        let part = bone_part(doll, bone)?;
        part_point(doll, part, Vec3::ZERO)
    };
    let hand = part_at(RIGHT_HAND).unwrap_or(body);
    let chest = part_at(CHEST).unwrap_or(body + Vec3::Y * 120.0);
    let body_rotation = ragdoll
        .and_then(|doll| doll.parts.first())
        .map(|part| part.transform().to_scale_rotation_translation().1)
        .unwrap_or(Quat::IDENTITY);
    CreatorInput {
        body,
        heading: context.heading,
        hand,
        chest,
        body_rotation,
        facing: context.heading,
        point: context.cursor,
        roll,
    }
}

/// Steps the force-shell auras: the ripples, then the fade machine. A
/// closed aura leaves its node.
fn tick_shell_auras(
    mut commands: Commands,
    ticks: Option<Res<BodyTicks>>,
    mut auras: Query<(
        Entity,
        &mut LiveShellAura,
        Has<super::hold_live::StrikesAura>,
    )>,
) {
    let now = ticks.as_deref().map_or(0, |t| t.millis() as u32);
    for (entity, mut aura, strikes) in &mut auras {
        // A strikes aura's ripples are stepped by its running power.
        if !strikes {
            aura.0.step_ripples();
        }
        if aura.0.frame(now).remove {
            commands.entity(entity).despawn();
        }
    }
}

/// Gives a live effect its material node: a child at the effect's place
/// carrying the creator's material, sub-objects, textures and block, so a
/// renderer finds the node and its pass with the effect.
fn attach_visual(
    commands: &mut Commands,
    parent: Entity,
    kind: EffectKind,
    context: &CastContext,
    ragdoll: Option<&Ragdoll>,
) -> Entity {
    let node = spawn_visual(
        commands,
        creator(kind, &visual_input(context, ragdoll, 0.0)),
    );
    // The live effect carries no transform of its own; the node keeps the
    // creator's place and the parent link is only its lifetime.
    commands.entity(parent).add_child(node);
    node
}

/// Places a vanished body at the destination its Long Door reached. The
/// shell the vanish dropped keeps its own place; only the body moves, as
/// the original's place-at call does, and its held items follow it
/// ([`exa_physics::grip::ReplaceHeld`]).
fn apply_otherwhere(
    mut commands: Commands,
    mut moves: MessageReader<Otherwhere>,
    mut bodies: Query<&mut Ragdoll>,
) {
    for Otherwhere { entity, to } in moves.read().copied() {
        if let Ok(mut doll) = bodies.get_mut(entity) {
            let target = otherwhere_root(&doll, to);
            info!("long door: {entity:?} is placed at {to:?} cm (root {target:?} m)");
            super::visuals::teleport(&mut doll, target, None);
            // The original's Long Door then puts the body's held items
            // back in its hands.
            commands
                .entity(entity)
                .insert(exa_physics::grip::ReplaceHeld);
        }
    }
}

/// Where a Long Door puts the body's root (m): the destination (cm, on the
/// floor) with the root kept as high above it as it stood above the body's
/// lowest part. The original asks the ground for the point with the body's
/// own height offset; the lowest part stands in for that offset (a guess).
fn otherwhere_root(doll: &Ragdoll, to: Vec3) -> Vec3 {
    let heights = doll.parts.iter().map(|part| part.transform().translation.y);
    let lowest = heights.clone().fold(f32::INFINITY, f32::min);
    let root = doll
        .parts
        .first()
        .map_or(lowest, |part| part.transform().translation.y);
    let lift = if lowest.is_finite() {
        root - lowest
    } else {
        0.0
    };
    to / CM + Vec3::Y * lift
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
        companions: Vec::new(),
        throw_view: None,
    });
    caster.sync_body();
}

/// Place a barricade: the player's ground pick under the cursor; without a
/// cursor ray the cast's point (the aim plane's, or the AI's target), else
/// two metres ahead, taken as it stands; the floor probe is the level's.
fn barricade_place(
    context: &CastContext,
    physics: Option<&Physics>,
    roll: &mut EffectRoll,
) -> super::force::BarricadePlace {
    let hit = context
        .barricade_ground
        .flatten()
        .unwrap_or_else(|| GroundHit {
            point: context
                .cursor
                .unwrap_or_else(|| context.caster + context.heading * 200.0 - Vec3::Y * 80.0),
            direction: Vec3::NEG_Y,
            exact: true,
        });
    let floor = |from: Vec3, reach: f32| {
        physics
            .and_then(|p| p.fixed_hit(from / CM, Vec3::NEG_Y, reach / CM))
            .map(|d| d * CM)
    };
    place_barricade(hit, context.caster, floor, roll.0.unit())
}

/// One held blade a shock-weapons cast arms: the weapon entity, where its
/// body sits (cm) and its rigid-body spheres in its own frame (cm).
struct WeaponArc {
    entity: Entity,
    anchor: Vec3,
    spheres: Vec<Vec3>,
}

/// The caster's armed blades, in query order. A fist or a claw is not a
/// weapon, so it is left out. Spheres come from the weapon's own body when
/// it has one.
fn weapon_arcs_of(
    caster: Entity,
    weapons: &Query<(
        Entity,
        &exa_combat::Blade,
        Option<&Ragdoll>,
        Option<&GlobalTransform>,
    )>,
) -> Vec<WeaponArc> {
    let mut armed = Vec::new();
    for (entity, blade, body, place) in weapons {
        if blade.wielder != caster || !blade.record.armed() {
            continue;
        }
        armed.push(weapon_arc(entity, body, place));
    }
    armed
}

/// Where a weapon's arcs sit, from its body when it has spheres and from its
/// place otherwise.
fn weapon_arc(
    entity: Entity,
    body: Option<&Ragdoll>,
    place: Option<&GlobalTransform>,
) -> WeaponArc {
    let mut anchor = place.map(|at| at.translation() * CM).unwrap_or(Vec3::ZERO);
    let mut spheres = Vec::new();
    if let Some(body) = body
        && let Some(part) = body.parts.iter().find(|part| !part.spheres.is_empty())
    {
        anchor = Vec3::from(part.transform().translation) * CM;
        spheres = part
            .spheres
            .iter()
            .map(|(centre, _)| *centre * CM)
            .collect();
    }
    WeaponArc {
        entity,
        anchor,
        spheres,
    }
}

/// A weapon put away or taken while shock weapons is running. Putting one
/// away (its grip released, or the blade gone) stops its arcs respawning and
/// ends them, and drops that arc from the list. A blade that just appeared
/// is armed when fewer than four weapons already carry arcs. A blade that
/// was already in hand and was not armed stays that way.
#[allow(clippy::type_complexity)]
fn follow_weapon_changes(
    mut commands: Commands,
    mut casters: Query<(Entity, &mut PowerCaster)>,
    blades: Query<(
        Entity,
        Ref<exa_combat::Blade>,
        Option<&Ragdoll>,
        Option<&GlobalTransform>,
        Option<&exa_physics::grip::GripRelease>,
    )>,
    mut arcs: Query<&mut LiveArcs>,
    mut draws: ResMut<EffectDraws>,
) {
    for (caster_entity, mut caster) in &mut casters {
        // The weapon changes reach the power holding the item hook.
        if !caster.hooks.item_is(super::energy::SHOCK_WEAPONS) {
            continue;
        }
        let Some(index) = caster
            .active
            .iter()
            .position(|entry| matches!(entry.class, ActiveClass::ShockWeapons(_)))
        else {
            continue;
        };
        let fresh: Vec<WeaponArc> = blades
            .iter()
            .filter(|(_, blade, _, _, release)| {
                blade.is_added()
                    && blade.wielder == caster_entity
                    && blade.record.armed()
                    && release.is_none()
            })
            .map(|(entity, _, body, place, _)| weapon_arc(entity, body, place))
            .collect();
        let entry = &mut caster.active[index];
        let mut kept = 0;
        while kept < entry.companions.len() {
            let arc_entity = entry.companions[kept];
            let Some(weapon) = arcs.get(arc_entity).ok().and_then(|live| live.weapon) else {
                entry.companions.swap_remove(kept);
                continue;
            };
            let put_away = match blades.get(weapon) {
                Ok((_, _, _, _, release)) => release.is_some(),
                Err(_) => true,
            };
            if !put_away {
                kept += 1;
                continue;
            }
            if let Ok(mut live) = arcs.get_mut(arc_entity) {
                live.arcs.respawns = false;
                live.arcs.ending = true;
            }
            entry.companions.swap_remove(kept);
        }
        for weapon in &fresh {
            if entry.companions.len() >= super::energy::MOST_WEAPONS {
                break;
            }
            let already = entry
                .companions
                .iter()
                .any(|arc| arcs.get(*arc).ok().and_then(|live| live.weapon) == Some(weapon.entity));
            if already {
                continue;
            }
            entry.companions.extend(spawn_weapon_arcs(
                &mut commands,
                caster_entity,
                std::slice::from_ref(weapon),
                &mut draws,
            ));
        }
        if let ActiveClass::ShockWeapons(power) = &mut entry.class {
            power.count = entry.companions.len();
        }
    }
}

/// A hit from a blade that still carries shock-weapon arcs: the original's
/// item hit hook. The struck body takes the full shock routine at strength
/// 0.01 along the striker's tracked swing velocity (the slot vector the
/// damage routine hands the hook): its 150 ms gate, the hit sound, the
/// burn, the stagger step, the walk stop and the cry, through
/// [`super::live::ArcShocked`]. A spark burst is queued at the contact.
/// A blade whose arcs were ended (put away) does neither.
#[allow(clippy::too_many_arguments)]
fn shock_weapon_hits(
    mut impacts: MessageReader<exa_combat::impact::LimbImpact>,
    casters: Query<&PowerCaster>,
    blades: Query<&exa_combat::Blade>,
    swings: Query<&exa_combat::swing::SwingState>,
    arcs: Query<&LiveArcs>,
    mut shocked: MessageWriter<super::live::ArcShocked>,
    mut commands: Commands,
    mut draws: ResMut<EffectDraws>,
) {
    for hit in impacts.read() {
        let Ok(blade) = blades.get(hit.other) else {
            continue;
        };
        let Some(caster) = casters.get(blade.wielder).ok() else {
            continue;
        };
        let armed = caster.active.iter().any(|entry| {
            matches!(entry.class, ActiveClass::ShockWeapons(_))
                && entry.companions.iter().any(|arc| {
                    arcs.get(*arc)
                        .is_ok_and(|live| live.weapon == Some(hit.other) && !live.arcs.ending)
                })
        });
        if !armed {
            continue;
        }
        let direction = swings.get(blade.wielder).map_or(Vec3::ZERO, |swing| {
            swing.tracker[blade.slot.index()].velocity
        });
        shocked.write(super::live::ArcShocked {
            body: hit.target,
            direction,
            strength: super::arcs::WEAPON_SHOCK,
        });
        info!(
            "{:?} struck by a shock weapon of {:?}: the shock routine at {} along {direction:?}",
            hit.target,
            blade.wielder,
            super::arcs::WEAPON_SHOCK
        );
        let at = Some(hit.point * CM);
        let mut burst = super::arcs::spark_burst(&mut draws.0);
        burst.anchor = at;
        commands.spawn((
            Name::new("_WeaponSpark"),
            LiveArcs {
                arcs: burst,
                hand: None,
                weapon: None,
                caster: Some(blade.wielder),
                keep: false,
            },
        ));
    }
}

/// One arc entity per armed blade. The arcs ride the weapon and start from
/// its spheres.
fn spawn_weapon_arcs(
    commands: &mut Commands,
    caster: Entity,
    weapons: &[WeaponArc],
    draws: &mut EffectDraws,
) -> Vec<Entity> {
    weapons
        .iter()
        .map(|weapon| {
            let mut arcs = ArcEmitter::new(&super::arcs::weapon_arcs(), &mut draws.0);
            arcs.anchor = Some(weapon.anchor);
            arcs.source_spheres = weapon.spheres.clone();
            commands
                .spawn((
                    Name::new("_WeaponArcs"),
                    LiveArcs {
                        arcs,
                        hand: None,
                        weapon: Some(weapon.entity),
                        caster: Some(caster),
                        keep: false,
                    },
                ))
                .id()
        })
        .collect()
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
                weapon: None,
                caster: Some(caster),
                keep: false,
            },
        ))
        .id()
}

/// The constant displacement's wisps.
pub(crate) fn spawn_body_wisps(
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
/// A character entering the scene resets its body: the cast hook is asked
/// event −1 and cleared with the power id.
fn reset_entered_casts(
    mut entered: MessageReader<exa_ai::presence::Entered>,
    mut casters: Query<&mut PowerCaster>,
) {
    for exa_ai::presence::Entered(entity) in entered.read() {
        if let Ok(mut caster) = casters.get_mut(*entity) {
            casting::reset_body(&mut caster.body);
        }
    }
}

/// A level change in process passes the player's list the status-effect
/// broadcast's 8 (the original's level load tells the player's effects, and
/// each follower's, before it flushes the scene). The next walk runs it.
pub(crate) fn broadcast_level_change(
    mut requests: MessageReader<exa_world::level_lifecycle::LevelChangeRequest>,
    mut casters: Query<(Entity, &mut PowerCaster), With<PlayerBody>>,
    mut fixed: Option<ResMut<exa_physics::sim_app::FixedCommands>>,
) {
    if !requests.read().any(|request| request.level.is_some()) {
        return;
    }
    for (entity, mut caster) in &mut casters {
        if caster.active.is_empty() {
            continue;
        }
        info!(
            "level change: the player's {} active powers are told event 8",
            caster.active.len()
        );
        // The caster is the simulation's: a posted write.
        match fixed.as_deref_mut() {
            Some(fixed) => fixed.write(entity, &mut caster, |c| {
                c.broadcast = Some(status_event::LEVEL_CHANGE);
            }),
            None => caster.broadcast = Some(status_event::LEVEL_CHANGE),
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn tick_casters(
    mut commands: Commands,
    mut casters: Query<
        (
            Entity,
            &mut PowerCaster,
            Option<&Condition>,
            Option<&SwingState>,
            Option<&Locomotion>,
            Option<&Locomotor>,
            Option<&Ragdoll>,
            Option<&mut Animator>,
            Option<&PlayerBody>,
            Option<&exa_combat::condition::StateFlags>,
        ),
        Without<OutOfScene>,
    >,
    walls: Query<&LiveBarricade>,
    mut bubbles: Query<&mut LiveBubble>,
    mut auras: Query<&mut LiveShellAura>,
    ticks: Option<Res<BodyTicks>>,
    buttons: Option<Res<ButtonInput<MouseButton>>>,
    pad: Option<Res<crate::gamepad::PlayPad>>,
    mut faded: MessageWriter<super::throw_live::ThrowFade>,
    mut moves: MessageWriter<Otherwhere>,
    mut holds: MessageWriter<super::hold_live::HoldEnd>,
    mut draws: ResMut<EffectDraws>,
    (mut arcs, control): (
        Query<&mut LiveArcs>,
        Option<Res<crate::player_controls::PlayerControl>>,
    ),
) {
    let now = ticks.as_deref().map_or(0, |t| t.millis() as u32);
    // The controls off, a form has the mouse: the player's hooks read no
    // button.
    // The block pair suspends the behaviour that reads them.
    let buttons = buttons
        .filter(|_| !crate::player_controls::PlayerControl::input_suspended(control.as_deref()));
    for (
        caster_entity,
        mut caster,
        condition,
        swing,
        locomotion,
        locomotor,
        ragdoll,
        mut animator,
        player,
        state_flags,
    ) in &mut casters
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
        let tone = ragdoll
            .as_ref()
            .and_then(|r| r.drive.as_ref())
            .map_or(1.0, |d| d.tone);
        let at = ragdoll.and_then(root_of).unwrap_or(Vec3::ZERO).to_array();
        let PowerCaster {
            body,
            active,
            bar,
            broadcast,
            was_dying,
            hooks,
            ..
        } = caster;
        let was_casting = body.condition == casting::CASTING;
        // [inf: the original's controller runs before the body tick and
        // reads the buttons only while the game view has the pointer; here
        // it runs with the casters, on the buttons as the window has them.]
        // The behaviour tick returns before its asks while the body is
        // shock-staggered or the behaviour is suspended (its disable byte),
        // and its walk stops while the body is dying.
        let suspended =
            crate::player_controls::PlayerControl::behaviour_suspended(control.as_deref());
        let staggered =
            state_flags.is_some_and(|f| f.has(exa_combat::condition::StateFlags::SHOCK_STAGGER));
        if let Some(player) = player
            && casting::player_asks_hooks(
                condition.is_some_and(|c| c.dying()),
                staggered,
                suspended,
            )
        {
            // [inf: the original's global flag beside the stick vector is
            // the gamepad action Exarust reads as Sprint.]
            let sprint_move = pad.as_deref().is_some_and(|pad| {
                let frame = pad.frame();
                f64::from(frame.move_raw.length_squared()) > 0.1 && frame.sprint
            });
            let input = casting::PlayerHookInput {
                primary: buttons
                    .as_deref()
                    .is_some_and(|b| b.pressed(MouseButton::Left)),
                secondary: buttons
                    .as_deref()
                    .is_some_and(|b| b.pressed(MouseButton::Right)),
                combat: player.combat,
                weak: tone < WEAK_BELOW,
                sprint_move,
            };
            if casting::player_hooks(body, input) {
                debug!("cast ended by the player's buttons");
            }
        }
        if casting::release_cast(body) {
            debug!("cast released");
        }
        casting::settle_capacity(body);
        // The status-effect broadcast: a body that starts to collapse or
        // dies passes every entry 2 (the collapse's call: once, as the
        // dying bit is first set); a level change's 8 was left by
        // [`broadcast_level_change`]. It runs before the tick's own walk.
        let dying = condition.is_some_and(|c| c.dying());
        if dying && !*was_dying && broadcast.is_none() {
            *broadcast = Some(status_event::DOWN);
        }
        *was_dying = dying;
        let told = broadcast.take();
        if let Some(event) = told {
            info!(
                "status broadcast: event {event} to {} active powers",
                active.len()
            );
        }
        for pass in told.into_iter().map(Some).chain([None]) {
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
                if let Some(told) = pass {
                    event = told;
                }
                let mut result = 0;
                match &mut entry.class {
                    ActiveClass::Shell(power) => {
                        let step = power.control(event, body);
                        result = step.result;
                    }
                    ActiveClass::Kinesis(power) => {
                        let was = (power.state, power.hooked, body.cast_hook, body.casting);
                        let step = power.control(event, body, entry.tick_cost);
                        result = step.result;
                        if result & 2 != 0 {
                            info!(
                                "kinesis ends: event {event}, handed {}, before (state {}, hooked {}, \
                             hook {:?}, casting {:#x}), focus {:.4} spent {:.4}",
                                power.handed, was.0, was.1, was.2, was.3, body.focus, body.spent
                            );
                        }
                        // Its end closes the pick's shell and gives the body
                        // its environment back, unless a throw took them.
                        if result & 2 != 0
                            && let Some(end) = super::hold_live::HoldEnd::of(
                                power.material.take(),
                                power.shell.take(),
                            )
                        {
                            holds.write(end);
                        }
                    }
                    ActiveClass::Throw(power) => {
                        let ending = event & 0xa != 0 || power.state == ENDING;
                        if entry.throw_view.is_some() || ending {
                            let view = entry.throw_view.unwrap_or(super::power::force::ThrowView {
                                target_at: at,
                                mass: 0.0,
                                caster_point: at,
                                tick_ms: TICK_MS,
                            });
                            let steered = power.phase == super::power::force::THROW_FLY;
                            let step = power.control(event, now, &view, body);
                            power.push = step.velocity.map(|v| (v, steered));
                            power.sound.lift |= step.lift;
                            power.sound.fade |= step.fade;
                            if step.hold {
                                power.held_until = Some(super::power::force::hold_until(now));
                            }
                            // The drop closes the shell; the restore (or the
                            // end) gives the body its environment back and
                            // closes a shell still open.
                            if step.drop_shell
                                && let Some(mut aura) = power
                                    .shell
                                    .take()
                                    .and_then(Entity::try_from_bits)
                                    .and_then(|e| auras.get_mut(e).ok())
                            {
                                aura.0.start_fade();
                                debug!("force throw: the shell drops");
                            }
                            if step.restore
                                && let Some(end) = super::hold_live::HoldEnd::of(
                                    power.material.take(),
                                    power.shell.take(),
                                )
                            {
                                holds.write(end);
                            }
                            result = step.result;
                        }
                    }
                    ActiveClass::Strikes(power) => {
                        let shells: Vec<u64> = power.auras.iter().filter_map(|a| a.shell).collect();
                        let step = power.control(event, body);
                        result = step.result;
                        // Its end closes every aura and clears the
                        // shared weapon hooks.
                        if result & 2 != 0 {
                            hooks.clear();
                            for shell in shells.into_iter().filter_map(Entity::try_from_bits) {
                                if let Ok(mut aura) = auras.get_mut(shell) {
                                    aura.0.start_fade();
                                }
                            }
                            debug!("force strikes ends: its auras close");
                        }
                    }
                    ActiveClass::Remote(power) => {
                        let step = power.control(event, at, body);
                        result = step.result;
                    }
                    ActiveClass::Shock(power) => {
                        let step = power.control(event, body);
                        result = step.result;
                        // The release makes the burst at the hand: a fresh
                        // emitter of the shock's own shape (strength 0.05, so
                        // its tendrils shock what they reach), before the
                        // hand's arcs leave. [inf: the original aims its first
                        // tendril at the controller's aim point within 45° of
                        // the heading (`effects::blast::blast_aim`); the tick
                        // has no aim point here, so the burst starts undirected.]
                        if step.fired
                            && let Some(body) = ragdoll
                        {
                            let mut arcs =
                                ArcEmitter::new(&crate::thaum::arcs::shock_burst(), &mut draws.0);
                            arcs.anchor = bone_part(body, RIGHT_HAND)
                                .and_then(|k| part_point(body, k, HAND_ARCS_OFFSET));
                            commands.spawn((
                                Name::new("_ShockBurst"),
                                LiveArcs {
                                    arcs,
                                    hand: Some((caster_entity, RIGHT_HAND)),
                                    weapon: None,
                                    caster: Some(caster_entity),
                                    keep: false,
                                },
                            ));
                            debug!("energy shock: the burst fires at the hand");
                        }
                        if (step.fired || step.cleared)
                            && let Some(entity) = entry.entity.take()
                        {
                            commands.entity(entity).despawn();
                        }
                    }
                    ActiveClass::ShockWeapons(power) => {
                        let step = power.control(event, body);
                        result = step.result;
                        // Its end clears the shared weapon hooks.
                        if result & 2 != 0 {
                            hooks.clear();
                        }
                    }
                    ActiveClass::Vanish(power) => {
                        power.answer_hook(body);
                        let step = power.control(event, now);
                        if 0.0 < step.spend {
                            drain(body, step.spend);
                        }
                        if step.clear_hook && power.owns_hook(body) {
                            casting::end_cast(body);
                        }
                        power.keep_hook(body);
                        result = step.result;
                    }
                    ActiveClass::LongDoor(power) => {
                        let nav = if power.destination.is_some() {
                            NavAnswer::Found
                        } else {
                            power.nav
                        };
                        let step = power.control(now, nav);
                        if step.moved
                            && let Some(destination) = power.destination
                        {
                            moves.write(Otherwhere {
                                entity: caster_entity,
                                to: Vec3::from_array(destination),
                            });
                        }
                        if 0.0 < step.spend {
                            drain(body, step.spend);
                        }
                        if step.clear_hook && body.casting == LONG_DOOR {
                            casting::end_cast(body);
                        }
                        result = step.result;
                    }
                    ActiveClass::Constant(power) => {
                        let step = power.control(event, body);
                        result = step.result;
                    }
                    ActiveClass::Summon(power) => {
                        let step = power.control(event, now);
                        if step.open_shell || step.release {
                            info!(
                                "summon by {caster_entity:?}: phase {:#x} at {now} ms{}",
                                step.phase,
                                if step.release {
                                    ", the demon is let go"
                                } else {
                                    ", the shell opens"
                                }
                            );
                        }
                        result = step.result;
                    }
                }
                if let Some(state) = entry.class.state() {
                    entry.state = state;
                }
                // The player's bar hook: a power that ends, or that answers
                // bit 4, marks the slot holding its id.
                if player.is_some() && result & 6 != 0 {
                    bar.mark_running(&entry.running(), now);
                }
                if result & 2 != 0 {
                    // The cast motion ends with the power: its id and hook go.
                    if body.casting == entry.cast_id && !entry.class.keeps_cast() {
                        body.casting = 0;
                        body.cast_hook = None;
                        body.condition &= !casting::CASTING;
                    }
                    let entry = active.swap_remove(index);
                    for companion in entry.companions {
                        if let Ok(mut live) = arcs.get_mut(companion) {
                            live.arcs.respawns = false;
                            live.arcs.ending = true;
                        }
                    }
                    if let ActiveClass::Throw(throw) = &entry.class
                        && throw.sound.fade
                        && let Some(voice) = throw.sound.voice
                    {
                        faded.write(super::throw_live::ThrowFade(voice));
                    }
                    if let Some(entity) = entry.entity
                        && let Ok(mut bubble) = bubbles.get_mut(entity)
                    {
                        end_bubble(&mut bubble);
                    }
                    // Constant displacement's end ends its tendril image:
                    // the wisps trail away and leave once faded.
                    if let Some(entity) = entry.entity
                        && matches!(entry.class, ActiveClass::Constant(_))
                    {
                        commands
                            .entity(entity)
                            .entry::<super::live::LiveWisps>()
                            .and_modify(|mut live| live.wisps.end());
                    } else if let Some(entity) = entry.entity
                        && !matches!(entry.class, ActiveClass::Remote(_))
                        && !matches!(entry.class, ActiveClass::Shell(_))
                    {
                        commands.entity(entity).despawn();
                    }
                    // A shell's power ends before its aura: the aura closes and
                    // takes its own node away once it has faded out.
                    if let Some(entity) = entry.entity
                        && matches!(entry.class, ActiveClass::Shell(_))
                        && let Ok(mut aura) = auras.get_mut(entity)
                    {
                        aura.0.start_fade();
                    }
                    continue;
                }
                index += 1;
            }
        }
        // The cast end returns a casting body to its stance: one this tick
        // ended, or one ended since that no animator has played.
        let stance_return = casting::take_stance_return(body);
        if (stance_return || was_casting && body.condition != casting::CASTING)
            && let Some(animator) = animator.as_deref_mut()
        {
            casting::play_follow(animator);
        }
        casting::settle_focus(body);
        caster.sync_body();
    }
}

/// Play the stance return a live effect's cast end asked for, then the cast
/// motions the body recorded, by the motion id's action name.
fn play_cast_motions(
    mut casters: Query<(&mut PowerCaster, &mut Animator), Changed<PowerCaster>>,
    motions: Option<Res<Motions>>,
) {
    for (mut caster, mut animator) in &mut casters {
        // A live effect's cast end this tick returns to the stance now,
        // before any new cast motion, as the original's cast end does.
        if casting::take_stance_return(&mut caster.body) {
            casting::play_follow(&mut animator);
        }
        let Some(motion) = caster.body.cast_motion.take() else {
            continue;
        };
        let Some(motions) = motions.as_deref() else {
            continue;
        };
        match motions.start_requested(&mut animator, motion) {
            Some(name) => {
                info!(
                    "cast clip {name} (action {motion:#x}) starts; the posture switch leaves a \
                     power's clip alone"
                );
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
            info!(
                "cast hook dropped: the cast clip was replaced by {}",
                animator.action
            );
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
