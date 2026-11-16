//! Running powers saved and restored with a character, as the original's
//! save writer and record build carry them.
//!
//! - **Save** ([`saved_powers`]): the player's placement in a save
//!   (`#playerdata`; the level section's character walk writes none, as
//!   the original's) ends with a running-powers chunk
//!   ([`exa_formats::characters::RUNNING_POWERS`]): the count of the powers
//!   that wrote themselves, then each power's own record. Only six classes
//!   write one ([`crate::thaum::power::saves_running`]): the persistent
//!   Sense Minds and Mask Mind, Shell and Strikes write their id and a size
//!   of 0; Loyalty and Command Dead their id, 4 and the target character's
//!   id. Exarust keeps a body's Mind powers on its [`MindBody`] and the
//!   other domains' on its [`PowerCaster`]; the save writes the Mind list
//!   first, then the caster's [Exarust's: the original has one list, in
//!   cast order].
//! - **Load** ([`restore_running`], [`restore_saved_powers`]): the record
//!   build hands each saved power to its domain
//!   ([`crate::thaum::power::restore_persistent`]); a rebuilt power holds
//!   its row's cost and the body's reserve takes it again (three quarters
//!   with the reserve technique, held and reserved alike). The persistent
//!   Sense, Mask, Shell and Strikes come back running; Loyalty and Command
//!   Dead come back ready with the saved target id, and their first tick
//!   looks the target up by its character id and puts the marker back on it
//!   ([`crate::mind::runtime`]'s walk), or ends. Then the spawn runs each
//!   power's activation handler, as the player's spawn and the content
//!   stream's character fetch do: Sense Minds shows its overlay, Mask Mind
//!   sets the masked bit, Shell takes half a blow's energy, Strikes takes
//!   the two combat factors and the weapon hooks.
//!
//! Evidence: the exporter's placement writer (the power walk after the
//! motion state), the record build's restore call and the domain restore
//! routines, the six classes' save and load slots, Loyalty's and Command
//! Dead's ready ticks, and the player spawn's and the stream fetch's
//! activation walk, read from `Exanima.exe` (instructions for the reserve
//! and held arithmetic).

use bevy::prelude::*;
use exa_formats::characters::RunningPower;

use crate::mind::casts::{
    BodyId, COMMAND_DEAD, LOYALTY, MASK_PERSISTENT, MindBody, MindClass, MindPower,
    SENSE_PERSISTENT,
};
use crate::thaum::cast::{ActiveClass, ActiveEntry, PowerCaster};
use crate::thaum::casting::CastBody;
use crate::thaum::power::force::{SHELL, Shell, Strikes};
use crate::thaum::power::mind::{CommandDeadCaster, LoyaltyCaster, MindMask, MindSense};
use crate::thaum::power::{READY, RUNNING, save_power, saves_running};

/// The Force Strikes id.
const STRIKES: u32 = crate::thaum::force::STRIKES;

/// A spawned body's saved running powers, waiting for its mind (and its
/// caster) before [`restore_saved_powers`] rebuilds them.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct SavedPowers(pub Vec<RunningPower>);

/// A loaded save's player powers, put on the next player body that gains
/// its mind ([`give_player_powers`]).
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct PendingPlayerPowers(pub Vec<RunningPower>);

/// One power's record as its save routine writes it.
fn record(words: Vec<u32>) -> RunningPower {
    let id = words.first().copied().unwrap_or(0);
    let data = words
        .iter()
        .skip(2)
        .flat_map(|word| word.to_le_bytes())
        .collect();
    RunningPower { id, data }
}

/// What a body's save writes for its running powers, in list order: its
/// Mind list's persistent Sense and Mask, Loyalty and Command Dead, then
/// its caster's Shell and Strikes. A running Loyalty or Command Dead names
/// its target by the character id `record_of` gives for the target's body
/// (0 when the body is gone); one still waiting for its target after a
/// load writes the id it was loaded with.
pub fn saved_powers(
    mind: Option<&MindBody>,
    caster: Option<&PowerCaster>,
    record_of: impl Fn(BodyId) -> Option<u32>,
) -> Vec<RunningPower> {
    let mut out = Vec::new();
    for power in mind.map_or(&[][..], |mind| &mind.powers) {
        if !saves_running(power.id) {
            continue;
        }
        let words = match &power.class {
            MindClass::Sense(_) | MindClass::Mask(_) => save_power(power.id, None),
            MindClass::Loyalty(LoyaltyCaster { state, target, .. })
            | MindClass::CommandDead(CommandDeadCaster { state, target, .. }) => {
                let character = if *state == READY {
                    *target
                } else {
                    target.or(power.target).and_then(&record_of)
                };
                save_power(power.id, Some(character.unwrap_or(0)))
            }
            _ => continue,
        };
        out.push(record(words));
    }
    for entry in caster.map_or(&[][..], |caster| &caster.active) {
        if matches!(entry.class, ActiveClass::Shell(_) | ActiveClass::Strikes(_))
            && saves_running(entry.id)
        {
            out.push(record(save_power(entry.id, None)));
        }
    }
    out
}

/// What a load rebuilt on one body.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Restored {
    /// Mind powers added to the mind's list.
    pub mind: usize,
    /// Force powers added to the caster's list.
    pub force: usize,
    /// Saved powers that rebuilt nothing (no restore row, or a Force power
    /// on a body without a caster).
    pub skipped: usize,
    /// A rebuilt persistent Sense Minds ran its activation: the sight
    /// overlay starts with no end, as on a cast.
    pub sense_overlay: bool,
}

/// Rebuilds a loaded body's saved powers onto its mind and caster
/// ([`crate::thaum::power::restore_persistent`] for the reserve and the
/// held focus), then runs each rebuilt power's activation handler. The
/// casts' body is the caster's when the body has one, else the mind's own.
pub fn restore_running(
    saved: &[RunningPower],
    mind: &mut MindBody,
    mut caster: Option<&mut PowerCaster>,
) -> Restored {
    let mut restored = Restored::default();
    // A Force power needs a caster's list to live on.
    let usable: Vec<RunningPower> = saved
        .iter()
        .filter(|power| {
            let force =
                power.id & crate::thaum::power::DOMAIN_MASK == crate::thaum::power::DOMAIN_FORCE;
            let keep = !force || caster.is_some();
            if !keep {
                restored.skipped += 1;
            }
            keep
        })
        .cloned()
        .collect();
    let rebuilt = {
        let body: &mut CastBody = match caster.as_deref_mut() {
            Some(caster) => &mut caster.body,
            None => &mut mind.cast,
        };
        crate::thaum::power::restore_persistent(body, &usable)
    };
    restored.skipped += usable.len() - rebuilt.len();
    for power in rebuilt {
        let (held, target) = (power.held, power.target);
        let mind_class = match power.id {
            SENSE_PERSISTENT => Some(MindClass::Sense(MindSense {
                state: RUNNING,
                held,
                overlay: false,
            })),
            MASK_PERSISTENT => Some(MindClass::Mask(MindMask {
                state: RUNNING,
                held,
            })),
            LOYALTY => Some(MindClass::Loyalty(LoyaltyCaster {
                state: READY,
                held,
                target,
            })),
            COMMAND_DEAD => Some(MindClass::CommandDead(CommandDeadCaster {
                state: READY,
                held,
                target,
            })),
            _ => None,
        };
        if let Some(class) = mind_class {
            mind.powers.push(MindPower::new(power.id, class));
            restored.mind += 1;
            continue;
        }
        let class = match power.id {
            SHELL => ActiveClass::Shell(Shell {
                state: RUNNING,
                held,
            }),
            STRIKES => {
                let mut strikes = Strikes::new();
                strikes.state = RUNNING;
                strikes.held = held;
                ActiveClass::Strikes(strikes)
            }
            _ => {
                restored.skipped += 1;
                continue;
            }
        };
        if let Some(caster) = caster.as_deref_mut() {
            caster.active.push(ActiveEntry {
                id: power.id,
                cast_id: power.id,
                state: RUNNING,
                class,
                tick_cost: 0.0,
                entity: None,
                companions: Vec::new(),
                throw_view: None,
            });
            restored.force += 1;
        }
    }
    restored.sense_overlay = activate(mind, caster);
    restored
}

/// The activation handler of every power on the body, as the spawn runs
/// it after the record build: the persistent Sense Minds shows its
/// overlay, Mask Mind sets the masked bit, Shell takes half a blow's
/// energy and Strikes its combat factors and both weapon hooks. The other
/// classes' handler does nothing. Answers whether a Sense Minds asked for
/// its overlay.
fn activate(mind: &mut MindBody, caster: Option<&mut PowerCaster>) -> bool {
    let mut powers = std::mem::take(&mut mind.powers);
    let mut overlay = false;
    {
        let body: &mut CastBody = match caster {
            Some(caster) => {
                let PowerCaster {
                    body,
                    active,
                    hooks,
                    ..
                } = &mut *caster;
                for entry in active.iter_mut() {
                    match &mut entry.class {
                        ActiveClass::Shell(shell) => shell.apply(body),
                        ActiveClass::Strikes(strikes) => {
                            strikes.apply(body, &[]);
                            hooks.take_strikes();
                        }
                        _ => {}
                    }
                }
                caster.sync_body();
                &mut caster.body
            }
            None => &mut mind.cast,
        };
        for power in &mut powers {
            match &mut power.class {
                MindClass::Sense(sense) => overlay |= sense.show(),
                MindClass::Mask(mask) => mask.apply(body),
                _ => {}
            }
        }
    }
    powers.append(&mut mind.powers);
    mind.powers = powers;
    overlay
}

/// Rebuilds the saved powers of each spawned body once its mind is in
/// ([`restore_running`]); the saved list is then dropped.
pub fn restore_saved_powers(
    mut commands: Commands,
    mut bodies: Query<(
        Entity,
        &SavedPowers,
        &mut MindBody,
        Option<&mut PowerCaster>,
    )>,
) {
    for (entity, saved, mut mind, caster) in &mut bodies {
        let restored = restore_running(&saved.0, &mut mind, caster.map(Mut::into_inner));
        info!(
            "load: {entity:?}'s saved powers {:x?}: {} Mind and {} Force rebuilt, {} not",
            saved.0.iter().map(|power| power.id).collect::<Vec<_>>(),
            restored.mind,
            restored.force,
            restored.skipped
        );
        if restored.sense_overlay {
            // The activation's sight: the overlay starts with no end
            // ([`crate::mind::draw::start_sense_overlay`]), as a cast's.
            commands.queue(|world: &mut World| crate::mind::draw::start_sense_overlay(world, 0));
        }
        commands.entity(entity).remove::<SavedPowers>();
    }
}

/// A loaded save's player powers go on the player body that has just
/// gained its mind (the load's new body).
pub fn give_player_powers(
    mut commands: Commands,
    pending: Option<Res<PendingPlayerPowers>>,
    players: Query<Entity, (With<crate::player::PlayerBody>, Added<MindBody>)>,
) {
    let Some(pending) = pending else {
        return;
    };
    let Some(player) = players.iter().next() else {
        return;
    };
    if !pending.0.is_empty() {
        commands
            .entity(player)
            .insert(SavedPowers(pending.0.clone()));
    }
    commands.remove_resource::<PendingPlayerPowers>();
}

/// Logs the player's running persistent powers whenever the set changes
/// (a cast, an end, a load), with the reserve they hold: the evidence a
/// run's log gives for a save and its load.
fn log_player_powers(
    players: Query<(&MindBody, Option<&PowerCaster>), With<crate::player::PlayerBody>>,
    mut last: Local<Option<Vec<(u32, i32)>>>,
) {
    let Some((mind, caster)) = players.iter().next() else {
        return;
    };
    let mut running: Vec<(u32, i32)> = mind
        .powers
        .iter()
        .filter(|power| power.id & crate::thaum::casting::PERSISTENT != 0)
        .map(|power| (power.id, power.state()))
        .collect();
    if let Some(caster) = caster {
        running.extend(
            caster
                .active
                .iter()
                .filter(|entry| entry.id & crate::thaum::casting::PERSISTENT != 0)
                .map(|entry| (entry.id, entry.state)),
        );
    }
    if last.as_ref() == Some(&running) {
        return;
    }
    let reserved = caster.map_or(mind.cast.reserved, |caster| caster.body.reserved);
    info!("powers: the player runs {running:x?} (id, state), reserve {reserved:.4}");
    *last = Some(running);
}

/// The live save and load of running powers.
pub struct PowerSavePlugin;

impl Plugin for PowerSavePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                give_player_powers
                    .in_set(exa_physics::FramePhase::Session)
                    .after(crate::mind::runtime::attach_minds),
                restore_saved_powers
                    .in_set(exa_physics::FramePhase::Session)
                    .after(crate::save_live::run_save_steps::<false>)
                    .after(give_player_powers),
                log_player_powers
                    .in_set(exa_physics::FramePhase::Capture)
                    .after(restore_saved_powers),
            ),
        );
    }
}

/// The record id of each body with a mind, for a save's Loyalty and
/// Command Dead targets.
pub fn record_ids<'a>(
    bodies: impl Iterator<Item = (Entity, &'a exa_character::record::CharacterData)>,
) -> Vec<(BodyId, u32)> {
    bodies
        .map(|(entity, data)| (crate::mind::runtime::body_id(entity), data.record.id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mind::casts::LOYALTY_MARK;
    use crate::save_live::{SaveFolder, SaveLivePlugin, SaveReport, SaveToNewSlot, StorySlotLoad};
    use crate::slots::{SlotList, Slots};
    use crate::thaum::casting::RESERVE_TECHNIQUE;
    use crate::thaum::power::mind::MASKED;

    /// A body running persistent Sense Minds (cost 0.27) and a Shell
    /// (0.35), with a Loyalty bound to body 9.
    fn running() -> (MindBody, PowerCaster) {
        let mut mind = MindBody::new(1);
        let sense = MindSense {
            state: RUNNING,
            held: 0.27,
            overlay: true,
        };
        mind.powers
            .push(MindPower::new(SENSE_PERSISTENT, MindClass::Sense(sense)));
        let loyalty = LoyaltyCaster {
            state: RUNNING,
            held: 0.29,
            target: Some(9),
        };
        let mut entry = MindPower::new(LOYALTY, MindClass::Loyalty(loyalty));
        entry.target = Some(9);
        mind.powers.push(entry);
        // An instant power writes nothing.
        mind.powers
            .push(MindPower::recorded(casts_sense(), None, None));
        let mut caster = PowerCaster::new(&[SHELL]);
        caster.active.push(ActiveEntry {
            id: SHELL,
            cast_id: SHELL,
            state: RUNNING,
            class: ActiveClass::Shell(Shell {
                state: RUNNING,
                held: 0.35,
            }),
            tick_cost: 0.0,
            entity: None,
            companions: Vec::new(),
            throw_view: None,
        });
        (mind, caster)
    }

    fn casts_sense() -> u32 {
        crate::mind::casts::SENSE
    }

    #[test]
    fn a_save_writes_the_six_saving_classes_in_list_order() {
        let (mind, caster) = running();
        let saved = saved_powers(Some(&mind), Some(&caster), |body| {
            (body == 9).then_some(0x4000_0005)
        });
        assert_eq!(
            saved,
            vec![
                RunningPower::bare(SENSE_PERSISTENT),
                RunningPower::with_target(LOYALTY, 0x4000_0005),
                RunningPower::bare(SHELL),
            ]
        );
        // A Loyalty still waiting for its target after a load writes the id
        // it was loaded with.
        let mut waiting = MindBody::new(2);
        waiting.powers.push(MindPower::new(
            LOYALTY,
            MindClass::Loyalty(LoyaltyCaster {
                state: READY,
                held: 0.29,
                target: Some(0x4000_0005),
            }),
        ));
        assert_eq!(
            saved_powers(Some(&waiting), None, |_| None),
            vec![RunningPower::with_target(LOYALTY, 0x4000_0005)]
        );
    }

    #[test]
    fn a_load_rebuilds_reserves_and_activates() {
        let saved = vec![
            RunningPower::bare(SENSE_PERSISTENT),
            RunningPower::bare(MASK_PERSISTENT),
            RunningPower::with_target(LOYALTY, 0x4000_0005),
            RunningPower::bare(SHELL),
            RunningPower::bare(STRIKES),
            // Calm has no restore row.
            RunningPower::bare(crate::mind::casts::CALM),
        ];
        let mut mind = MindBody::new(1);
        let mut caster = PowerCaster::new(&[]);
        caster.body.techniques = RESERVE_TECHNIQUE;
        let restored = restore_running(&saved, &mut mind, Some(&mut caster));
        assert_eq!(
            restored,
            Restored {
                mind: 3,
                force: 2,
                skipped: 1,
                sense_overlay: true,
            }
        );
        // Three quarters of each row's cost goes into the reserve, and each
        // power holds what its reserve took.
        let costs = 0.27f32 * 0.75 + 0.24 * 0.75 + 0.29 * 0.75 + 0.35 * 0.75 + 0.3 * 0.75;
        assert!((caster.body.reserved - costs).abs() < 1e-6);
        let MindClass::Sense(sense) = &mind.powers[0].class else {
            panic!("{:?}", mind.powers[0]);
        };
        assert_eq!(
            (sense.state, sense.held, sense.overlay),
            (RUNNING, 0.27 * 0.75, true)
        );
        assert!(matches!(
            mind.powers[1].class,
            MindClass::Mask(MindMask { state: RUNNING, .. })
        ));
        let MindClass::Loyalty(loyalty) = &mind.powers[2].class else {
            panic!("{:?}", mind.powers[2]);
        };
        assert_eq!((loyalty.state, loyalty.target), (READY, Some(0x4000_0005)));
        // The activation handlers: the masked bit, the shell's damping, the
        // strikes' factors and hooks; the body's power view follows the
        // caster's list.
        assert_ne!(caster.body.status_bits & MASKED, 0);
        assert_eq!(caster.body.hit_damping, 0.5);
        assert_eq!(caster.body.swing_factor, 3.0);
        assert!(caster.hooks.strike_is(STRIKES));
        assert_eq!(
            caster.body.powers.iter().map(|p| p.id).collect::<Vec<_>>(),
            [SHELL, STRIKES]
        );
        // Without a caster a Force power has no list to live on.
        let mut bare = MindBody::new(2);
        let restored = restore_running(&[RunningPower::bare(SHELL)], &mut bare, None);
        assert_eq!(restored.skipped, 1);
        assert_eq!(bare.cast.reserved, 0.0);
    }

    fn folder(name: &str) -> std::path::PathBuf {
        let path =
            std::env::temp_dir().join(format!("exarust-power-save-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    /// The live round trip: the save plugin writes the player's running
    /// persistent Sense Minds and Shell into the placement (the bytes the
    /// original's power walk writes), the story load hands them to the next
    /// player body, and the power plugin rebuilds them running with their
    /// reserve taken again.
    #[test]
    fn a_saved_running_power_runs_again_after_the_load() {
        let dir = folder("round-trip");
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(SaveLivePlugin)
            .add_plugins(PowerSavePlugin)
            .insert_resource(SaveFolder(dir.clone()))
            // The loaded player's post to the content stream reads these.
            .init_resource::<exa_assets::ModelCache>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<exa_render::material::ExaMaterial>>()
            .init_resource::<Assets<Image>>();
        app.world_mut().spawn(exa_world::LevelRoot {
            name: "exanima02.rfc".to_string(),
        });
        let (mut mind, caster) = running();
        // Only the self powers: the Loyalty's target is not in this scene.
        mind.powers.remove(1);
        app.world_mut().spawn((
            GlobalTransform::default(),
            crate::player::PlayerBody::new(0.0),
            mind,
            caster,
        ));
        app.update();
        app.world_mut().write_message(SaveToNewSlot);
        app.update();
        let number = app.world().resource::<SaveReport>().saved.unwrap();
        // The written placement ends with the running powers chunk.
        let mut list = SlotList::new(Slots::story(&dir));
        assert!(list.any());
        list.select(0);
        let loaded = list.load_current(false).unwrap();
        let placement = loaded.data.placement().unwrap().unwrap();
        let chunk = placement
            .chunks
            .iter()
            .find(|c| c.id == exa_formats::characters::RUNNING_POWERS)
            .unwrap();
        let words: Vec<u32> = chunk
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .map(|w| u32::from_le_bytes(*w))
            .collect();
        assert_eq!(words, [2, SENSE_PERSISTENT, 0, SHELL, 0]);

        // The load: the old player goes, the new body gains its mind and
        // its caster, and its saved powers come back.
        app.world_mut().write_message(StorySlotLoad {
            number,
            prefer_checkpoint: false,
        });
        app.update();
        assert!(app.world().contains_resource::<PendingPlayerPowers>());
        let player = app
            .world_mut()
            .spawn((
                crate::player::PlayerBody::new(0.0),
                MindBody::new(7),
                PowerCaster::new(&[]),
            ))
            .id();
        app.update();
        app.update();
        assert!(!app.world().contains_resource::<PendingPlayerPowers>());
        let mind = app.world().get::<MindBody>(player).unwrap();
        assert_eq!(mind.powers.len(), 1);
        assert!(matches!(
            mind.powers[0].class,
            MindClass::Sense(MindSense {
                state: RUNNING,
                overlay: true,
                ..
            })
        ));
        let caster = app.world().get::<PowerCaster>(player).unwrap();
        assert_eq!(caster.active.len(), 1);
        assert!(matches!(caster.active[0].class, ActiveClass::Shell(_)));
        assert!((caster.body.reserved - (0.27 + 0.35)).abs() < 1e-6);
        assert!(app.world().get::<SavedPowers>(player).is_none());
        // The loaded Sense Minds' activation started the sight overlay with
        // no end, as a cast does.
        let sense = app.world().resource::<crate::mind::draw::MindSense>();
        assert!(sense.hooked);
        assert_eq!(sense.overlay.end_ms, u32::MAX);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A retail-format placement (a constructed record: no retail save at
    /// hand holds a running power) with Loyalty bound to a character of
    /// the scene: the load rebuilds it ready, and its first tick finds the
    /// target by character id and puts the loyalty marker back on it.
    #[test]
    fn a_loaded_loyalty_finds_its_target_by_character_id() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(PowerSavePlugin)
            .add_message::<exa_audio::PointSounded>()
            .add_systems(Update, crate::mind::runtime::tick_minds);
        let target_record = exa_formats::characters::CharacterRecord {
            id: 0x4000_0005,
            ..exa_formats::characters::CharacterRecord::reset()
        };
        let target = app
            .world_mut()
            .spawn((
                exa_character::record::CharacterData::of_record(target_record),
                exa_combat::Condition::default(),
            ))
            .id();
        let target_id = crate::mind::runtime::body_id(target);
        app.world_mut()
            .entity_mut(target)
            .insert(MindBody::new(target_id));
        let mut placement = Vec::new();
        for word in [1u32, 0, 0, 0, 0, 0] {
            placement.extend(word.to_le_bytes());
        }
        let powers = exa_formats::characters::write_running_powers(&[RunningPower::with_target(
            LOYALTY,
            0x4000_0005,
        )])
        .unwrap();
        placement.extend(exa_formats::characters::RUNNING_POWERS.to_le_bytes());
        placement.extend((powers.len() as u32).to_le_bytes());
        placement.extend(powers);
        let read = exa_formats::save_package::CharacterPlacement::parse(&placement).unwrap();
        let caster = app
            .world_mut()
            .spawn((
                MindBody::new(0),
                SavedPowers(read.running_powers().unwrap()),
            ))
            .id();
        let caster_id = crate::mind::runtime::body_id(caster);
        app.world_mut().get_mut::<MindBody>(caster).unwrap().id = caster_id;
        app.update();
        app.update();
        let mind = app.world().get::<MindBody>(caster).unwrap();
        let MindClass::Loyalty(loyalty) = &mind.powers[0].class else {
            panic!("{:?}", mind.powers);
        };
        assert_eq!(loyalty.state, RUNNING);
        assert_eq!(loyalty.target, Some(target_id));
        assert!((mind.cast.reserved - 0.29).abs() < 1e-6);
        let marked = app.world().get::<MindBody>(target).unwrap();
        assert_eq!(marked.powers.len(), 1);
        assert_eq!(marked.powers[0].id, LOYALTY_MARK);
        assert_eq!(marked.powers[0].caster, Some(caster_id));
        assert!(marked.ai.rearm);
    }
}
