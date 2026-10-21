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
//!   [`exa_anim::Animator`] by its motion id's action name.
//! - **Live effects** ([`super::live`]): a shock's hand arcs, a barricade's
//!   wall and a constant displacement's wisps are spawned with the power and
//!   removed when it ends. Every other effect has no entity yet: the class
//!   keeps running and the missing entity is named in the crate README.
//!
//! Rules: `docs/THAUMATURGY.md` (In the game), `docs/PLAYER.md` (Hotkeys).
//! Evidence: the bar's cast entry and the dispatcher by domain byte, the
//! casting rows' creators, the body's active-power walk and `TActivePower`'s
//! control, read from `Exanima.exe`.

use bevy::prelude::*;
use exa_anim::{Animator, Motions};
use exa_character::held::HeldMoveset;
use exa_combat::Condition;
use exa_combat::swing::SwingState;
use exa_physics::Physics;
use exa_physics::body::locomotion::{Locomotion, Locomotor};
use exa_physics::body::{BodySystems, BodyTicks, Ragdoll};

use super::arcs::{ArcEmitter, HAND_ARCS_OFFSET, hand_arcs};
use super::barricade::{Barricade, WallFrame, state as wall_state};
use super::casting::{self, ActivePower, CastBody, ID_MASK, code};
use super::displacement::{
    CursorHit, DisplacementAim, DisplacementStart, body_wisps, displacement_front_end,
};
use super::energy::{EnergyStart, energy_front_end, shock_charge};
use super::force::{
    BARRICADE, ForceStart, ForceWorld, GroundHit, KINESIS, place_barricade, place_dome,
};
use super::live::{EffectDraws, EffectRoll, LiveArcs, LiveBarricade, LiveWisps};
use super::power::displacement::{
    ConstantDisplacement, LONG_DOOR, LongDoor, NavAnswer, Summon, Vanish,
};
use super::power::energy::{Shock, ShockWeapons};
use super::power::force::{
    BUBBLE, BUBBLE_MOTION, Kinesis, RemotePower, SHELL, Shell, Strikes, THROW, Throw,
};
use super::power::{ENDING, KNOWN_WORD, KnownPower, PowerBase, READY, RUNNING, drain};
use super::tree::{
    DOMAIN_DISPLACEMENT, DOMAIN_ENERGY, DOMAIN_FORCE, DOMAIN_MIND, PowerBar, PowerRecord, TreeMode,
    TreeScreen, word,
};
use super::wisps::{Strand, Wisps};
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
}

impl PowerCaster {
    /// A caster who knows `ids` in that order: the record's slots are those
    /// powers, the first chosen as a new character's bar slot is, and the
    /// body has one persistent slot (the debug grant's techniques).
    pub fn new(ids: &[u32]) -> Self {
        let known: Vec<KnownPower> = ids
            .iter()
            .copied()
            .map(|id| KnownPower {
                id,
                word: KNOWN_WORD,
            })
            .collect();
        let mut record = PowerRecord::empty(4.0);
        record.focus_item = true;
        record.pairs = known
            .iter()
            .map(|power| super::tree::KnownPower {
                power: power.id,
                flags: super::tree::LEARNED,
            })
            .collect();
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
        }
    }

    /// Rebuild the body's power view from the active list, in its order.
    fn sync_body(&mut self) {
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
                FixedUpdate,
                (tick_casters, play_cast_motions).chain().after(BodySystems),
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
    ground: Vec3,
    cursor: Option<Vec3>,
    clip_ms: Option<u32>,
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
                    cursor: ground.as_deref().and_then(|g| g.0).map(|point| CursorHit {
                        point: point * CM,
                        normal: Vec3::Y,
                        exact: false,
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
                caster.take_states();
                if matches!(
                    start,
                    Some(CastStart::Displacement(DisplacementStart::EndVanish))
                ) {
                    // Vanish pressed while it runs: the running power's hook
                    // is told to end it.
                    for running in &mut caster.active {
                        if let ActiveClass::Vanish(power) = &mut running.class {
                            power.release();
                        }
                    }
                }
                if entry.result > 0
                    && let Some(start) = start
                {
                    let root = ragdoll.and_then(root_of).unwrap_or(Vec3::ZERO);
                    let yaw = locomotor.map_or(0.0, |l| l.yaw);
                    let heading = Vec3::new(yaw.sin(), 0.0, yaw.cos());
                    let context = CastContext {
                        now,
                        caster: root,
                        heading,
                        ground: root,
                        cursor: ground.as_deref().and_then(|g| g.0).map(|p| p * CM),
                        clip_ms: shot_clip_ms(motions.as_deref(), animator.as_deref()),
                    };
                    let made = create_active(
                        &mut commands,
                        entity,
                        caster,
                        start,
                        &context,
                        ragdoll,
                        physics.as_deref(),
                        &mut draws,
                        &mut roll,
                    );
                    debug!(
                        "cast {asked:#x}: {} active, effect entity {:?}",
                        caster.active.len(),
                        made
                    );
                }
                caster.sync_body();
                if let Some(screen) = screen.as_deref_mut() {
                    store_on_screen(screen, caster);
                }
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
    physics: Option<&Physics>,
    draws: &mut EffectDraws,
    roll: &mut EffectRoll,
) -> Option<Entity> {
    match start {
        ForceStart::Blast { setup, .. } => {
            debug!(
                "force blast/bolt/wave {:?}: the effect entity is not ported",
                setup.power
            );
            None
        }
        ForceStart::Torrent { .. } => {
            debug!("force torrent: the effect entity is not ported");
            None
        }
        ForceStart::Barrier { .. } => {
            debug!("force barrier: the dome entity is not ported");
            None
        }
        ForceStart::Bubble { cost } => {
            let place = place_dome(
                super::force::ForceKind::Bubble,
                context.ground,
                context.heading,
            );
            let at = place.map_or([0.0; 3], |p| p.centre.to_array());
            let (motion, held) = {
                let body = &mut caster.body;
                let motion = casting::start_cast(body, BUBBLE_MOTION, BUBBLE);
                let held = PowerBase::new(BUBBLE).accrue(body, cost);
                (motion, held)
            };
            let mut power = RemotePower::new(at);
            power.held = held;
            debug!("force bubble: cast motion {motion}, the dome entity is not ported");
            push_active(
                caster,
                BUBBLE,
                BUBBLE,
                RUNNING,
                ActiveClass::Remote(power),
                0.0,
                None,
            );
            None
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
                    && !matches!(entry.class, ActiveClass::Remote(_))
                {
                    commands.entity(entity).despawn();
                }
                continue;
            }
            index += 1;
        }
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
        match motions
            .as_deref()
            .and_then(|motions| motions.action_name(motion))
        {
            Some(name) => {
                let name = name.to_string();
                animator.play_once(&name, &follow);
                debug!("cast motion {motion:#x}: {name} then {follow}");
            }
            None => debug!("cast motion {motion:#x}: no action of that name"),
        }
    }
}

/// Keep the drawn bar on the caster's bar while the tree is closed.
fn sync_screen_bar(
    mut screen: Option<ResMut<PowerScreen>>,
    casters: Query<&PowerCaster, With<PlayerBody>>,
) {
    let Some(screen) = screen.as_deref_mut() else {
        return;
    };
    let Some(caster) = casters.iter().next() else {
        return;
    };
    if screen.open {
        return;
    }
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
