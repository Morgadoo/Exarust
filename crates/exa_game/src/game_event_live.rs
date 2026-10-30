//! The story scripts' `GameEvent` in the running game.
//!
//! [`crate::game_event::handle`] is the original's handler as a pure rule.
//! This module feeds it the live player and applies what it returns, in the
//! handler's order:
//!
//! - the player read: the memory role from the body's roles, the class word,
//!   sex and potential from its record, the voice volume and pitch from its
//!   voice, the fragments seen and the background from the flow's story
//!   state;
//! - the raise-dead narration asks the power grant for Raise Dead on the
//!   record's known powers, with the mind tree (`pwr_mind.pwr`), before the
//!   handler picks its line (the grant writes the power when it succeeds);
//! - the requested action goes through the flow's unforced action switch
//!   ([`crate::flow::Step::RequestAction`]);
//! - the voice line goes through the body's voice request (the pain voice's
//!   path: the line's set, its waits and the character's gains);
//! - the narrator topics start through the dialogue engine
//!   ([`exa_script::Narrate`], with their delay);
//! - the potential and the class word are written to the player's record;
//! - the sounds play as named one-shots at the player or at the origin,
//!   with the handler's flags;
//! - the flow keeps the fragments seen and the background unlock itself
//!   ([`crate::flow::Flow::game_event`]).
//!
//! Not applied yet (logged): the post-process flash (no renderer for it),
//! the fragment's witness event to the characters who hold its role (the
//! role-event broadcast is not ported), and the mind and force awakenings'
//! visual effects on the body.
//!
//! The random draws come from the process generator the sounds use (the
//! sound bank's), as the original takes them from its one generator.
//!
//! A debug aid, `--game-event CODE@FRAME[,...]` ([`GameEventScript`]), raises
//! a script `GameEvent` at a frame after the load, so the effects can be
//! checked in a level without walking to its script.

use bevy::prelude::*;
use exa_character::record::CharacterData;
use exa_combat::pain::{Loudness, PainSounded, PainSounds};
use exa_combat::voice::CharacterVoice;
use exa_formats::characters::{CharacterRecord, INSIGHT, Power};

use crate::flow::{Background, Flow, FlowSteps, Step};
use crate::game_event::{
    EventPlayer, MEMORY_ROLE, RAISE_DEAD_POWER, SoundAt, StoryContext, apply_to_record, asks_grant,
};
use crate::player::PlayerBody;
use crate::thaum::cast::PowerCaster;

/// `--game-event CODE@FRAME[,...]`: script `GameEvent` codes raised at
/// frames after the load (a debug aid).
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventScript(pub Vec<(u32, u32)>);

impl GameEventScript {
    /// Reads `code@frame,...` (codes in decimal or `0x` hex).
    pub fn parse(text: &str) -> Option<Self> {
        text.split(',')
            .map(|entry| {
                let (code, frame) = entry.split_once('@')?;
                Some((crate::gui_drop::parse_id(code)?, frame.trim().parse().ok()?))
            })
            .collect::<Option<Vec<_>>>()
            .map(Self)
    }
}

/// The mind tree's nodes, read once from the resource pack (the power grant
/// reads the domain's tree file each time; Exarust keeps the parsed copy).
#[derive(Default)]
pub struct MindTree(Option<Option<Vec<exa_formats::power_tree::PowerNode>>>);

impl MindTree {
    fn nodes(
        &mut self,
        game: Option<&exa_assets::Game>,
    ) -> Option<&[exa_formats::power_tree::PowerNode]> {
        if self.0.is_none() {
            let game = game?;
            let file = crate::thaum::tree::DOMAIN_FILES
                [usize::from(crate::thaum::tree::domain_of(RAISE_DEAD_POWER))];
            self.0 = Some(
                game.0
                    .resource_file(file)
                    .and_then(|bytes| exa_formats::power_tree::read_power_tree(bytes).ok())
                    .map(|tree| tree.nodes),
            );
        }
        self.0.as_ref().and_then(|nodes| nodes.as_deref())
    }
}

/// The player the handler reads, from the live body.
pub fn event_player(
    record: Option<&CharacterRecord>,
    roles: Option<&exa_ai::roles::Roles>,
    voice: Option<&CharacterVoice>,
    collected: u32,
    raise_dead: bool,
) -> EventPlayer {
    // The original asks the record's role list; the body's roles are
    // Exarust's live copy of it.
    let memory_role = match roles {
        Some(roles) => roles.has(MEMORY_ROLE),
        None => record.is_some_and(|r| r.roles.iter().any(|role| role.id == MEMORY_ROLE)),
    };
    EventPlayer {
        memory_role,
        raise_dead,
        class: record.map_or(0, |r| r.extra_word),
        female: record.is_some_and(|r| r.female),
        potential: record.map_or(0.0, crate::game_event::potential),
        collected,
        voice_volume: voice.map_or(1.0, |v| v.volume),
        voice_pitch: voice.map_or(1.0, |v| v.pitch),
    }
}

/// The power grant for Raise Dead on the record's known powers: Insight's
/// grant technique and a learnt prerequisite in the mind tree. Returns
/// whether the power was written. A power the record already held keeps
/// its stored bytes.
pub fn grant_raise_dead(
    record: &mut CharacterRecord,
    nodes: &[exa_formats::power_tree::PowerNode],
) -> bool {
    let insight = record.skills[INSIGHT];
    let Some(powers) = record.powers.as_mut() else {
        return false;
    };
    let mut known: Vec<crate::thaum::tree::KnownPower> = powers
        .known
        .iter()
        .map(|power| crate::thaum::tree::KnownPower {
            power: power.id,
            flags: power.flags,
        })
        .collect();
    if !crate::thaum::tree::grant_power(nodes, &insight, &mut known, RAISE_DEAD_POWER) {
        return false;
    }
    let old = std::mem::take(&mut powers.known);
    powers.known = known
        .into_iter()
        .map(|pair| Power {
            id: pair.power,
            flags: pair.flags,
            blob: old
                .iter()
                .find(|power| power.id == pair.power)
                .and_then(|power| power.blob.clone()),
        })
        .collect();
    true
}

/// The live writers the effects go through.
#[derive(bevy::ecs::system::SystemParam)]
pub struct EventOutputs<'w> {
    queued: ResMut<'w, FlowSteps>,
    voices: MessageWriter<'w, PainSounded>,
    sounds: MessageWriter<'w, exa_audio::PointSounded>,
    narrator: MessageWriter<'w, exa_script::Narrate>,
}

type EventPlayerQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static GlobalTransform,
        Option<&'static mut CharacterData>,
        Option<&'static exa_ai::roles::Roles>,
        Option<&'static CharacterVoice>,
        Option<&'static mut PowerCaster>,
    ),
    With<PlayerBody>,
>;

/// The consumer of the story scripts' `GameEvent` calls: each code runs the
/// flow's hook ([`Flow::game_event`]) with the live player, and the effects
/// it returns act on the player.
#[allow(clippy::too_many_arguments)]
pub fn run_game_events(
    mut events: MessageReader<exa_script::ScriptEffect>,
    mut flow: ResMut<Flow>,
    mut players: EventPlayerQuery,
    mut out: EventOutputs,
    game: Option<Res<exa_assets::Game>>,
    mut bank: Option<ResMut<exa_audio::SoundBank>>,
    mut mind: Local<MindTree>,
) {
    for event in events.read() {
        let &exa_script::dialogue::Effect::GameEvent(code) = &event.0 else {
            continue;
        };
        let story = StoryContext {
            unknown_background: flow.story.background == Background::Unknown,
        };
        let mut player = players.iter_mut().next();
        // The raise-dead line asks the grant first; it writes the power on
        // success.
        let mut raise_dead = false;
        if asks_grant(code, story)
            && let Some((_, _, Some(data), _, _, caster)) = player.as_mut()
            && let Some(nodes) = mind.nodes(game.as_deref())
        {
            raise_dead = grant_raise_dead(&mut data.record, nodes);
            if raise_dead {
                info!(
                    "game event {code:#x}: the power grant wrote Raise Dead ({RAISE_DEAD_POWER:#x})"
                );
                // The body's caster keeps its own copy of the known powers.
                if let Some(caster) = caster.as_mut() {
                    let insight = data.record.skills[INSIGHT];
                    caster.grant(nodes, &insight, RAISE_DEAD_POWER);
                }
            }
        }
        let reading = match player.as_ref() {
            Some((_, _, data, roles, voice, _)) => event_player(
                data.as_deref().map(|d| &d.record),
                *roles,
                *voice,
                flow.story.memory,
                raise_dead,
            ),
            None => event_player(None, None, None, flow.story.memory, raise_dead),
        };
        let mut draw = || bank.as_mut().map_or(0.5, |bank| bank.rng().unit_f64());
        let (steps, effects) = flow.game_event(code, story, &reading, &mut draw);
        info!(
            "game event {code:#x}: background {:?}, memory role {}, fragments {:#06x} -> {:#06x}, {} settings write(s), effects {effects:?}",
            flow.story.background,
            reading.memory_role,
            reading.collected,
            flow.story.memory,
            steps.len()
        );
        out.queued.steps.extend(steps);
        for effect in effects {
            apply(code, &effect, player.as_mut(), &mut out);
        }
    }
}

type PlayerItem<'a> = (
    Entity,
    &'a GlobalTransform,
    Option<Mut<'a, CharacterData>>,
    Option<&'a exa_ai::roles::Roles>,
    Option<&'a CharacterVoice>,
    Option<Mut<'a, PowerCaster>>,
);

/// One effect on the live player.
fn apply(
    code: u32,
    effect: &crate::game_event::Effect,
    player: Option<&mut PlayerItem<'_>>,
    out: &mut EventOutputs,
) {
    use crate::game_event::Effect;
    let at = player.as_ref().map_or(Vec3::ZERO, |p| p.1.translation());
    match *effect {
        Effect::RequestAction(motion) => out.queued.steps.push(Step::RequestAction(motion)),
        Effect::Voice {
            line,
            volume,
            pitch,
        } => {
            if let Some(player) = player.as_ref() {
                out.voices.write(PainSounded {
                    fighter: player.0,
                    point: at,
                    sounds: PainSounds {
                        wound: None,
                        voice: Some((line, Loudness { volume, pitch })),
                    },
                });
            }
        }
        Effect::Dialogue { text, delay_ms } => {
            info!("game event {code:#x}: narrator topic {text:?} after {delay_ms} ms");
            out.narrator.write(exa_script::Narrate {
                topic: text.to_string(),
                delay: delay_ms,
            });
        }
        Effect::Potential(_) | Effect::SetPotential(_) | Effect::Class { .. } => {
            if let Some((_, _, Some(data), ..)) = player {
                apply_to_record(effect, &mut data.record);
                info!(
                    "game event {code:#x}: record potential {:.3}, class word {:#010x}",
                    crate::game_event::potential(&data.record),
                    data.record.extra_word
                );
            } else {
                info!("game event {code:#x}: {effect:?} has no player record to write");
            }
        }
        Effect::Sound {
            name,
            at: place,
            volume,
            pitch,
            priority,
        } => {
            let position = match place {
                SoundAt::Player => at,
                SoundAt::Origin => Vec3::ZERO,
            };
            out.sounds.write(exa_audio::PointSounded {
                name: name.to_string(),
                position,
                volume,
                pitch,
                flags: priority,
            });
        }
        Effect::Flash => info!("game event {code:#x}: the post-process flash (not drawn yet)"),
        Effect::AiEvent {
            kind,
            first,
            second,
            name,
        } => info!(
            "game event {code:#x}: witness event {kind:#x} {first} {second} {name:?} (not ported)"
        ),
        Effect::MindAwakening | Effect::ForceAwakening => {
            info!("game event {code:#x}: {effect:?} (its body effect is not ported)")
        }
        // The flow keeps these.
        Effect::Collected(_) | Effect::SetCollected(_) | Effect::Unlock { .. } => {}
    }
}

/// Raises the scripted codes at their frames after the load.
fn scripted_game_events(
    script: Option<Res<GameEventScript>>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    levels: Query<(), With<exa_world::LevelRoot>>,
    mut frames: Local<u32>,
    mut effects: MessageWriter<exa_script::ScriptEffect>,
) {
    let Some(script) = script else {
        return;
    };
    if loading.is_some() || levels.is_empty() {
        return;
    }
    *frames += 1;
    for &(code, frame) in &script.0 {
        if frame == *frames {
            info!("game event: scripted code {code:#x} at frame {frame}");
            effects.write(exa_script::ScriptEffect(
                exa_script::dialogue::Effect::GameEvent(code),
            ));
        }
    }
}

pub struct GameEventLivePlugin;

impl Plugin for GameEventLivePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<exa_script::ScriptEffect>()
            .add_message::<exa_script::Narrate>()
            .add_message::<PainSounded>()
            .add_message::<exa_audio::PointSounded>()
            .add_systems(
                Update,
                scripted_game_events.before(crate::flow::FlowSystems::Transition),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_script_reads_codes_and_frames() {
        assert_eq!(
            GameEventScript::parse("0xc00001@30,4@31"),
            Some(GameEventScript(vec![(0xc0_0001, 30), (4, 31)]))
        );
        assert_eq!(GameEventScript::parse("0xc00001"), None);
        assert_eq!(GameEventScript::parse("x@1"), None);
    }

    #[test]
    fn the_player_reading_takes_the_record_roles_and_voice() {
        let mut record = CharacterRecord::new();
        record.female = true;
        record.extra_word = 0x0051_0000;
        crate::game_event::set_potential(&mut record, 4.0);
        record.roles.push(exa_formats::characters::Role {
            id: MEMORY_ROLE,
            weight: 1.0,
            explicit: true,
            inherited: 0,
            variables: Vec::new(),
        });
        let voice = CharacterVoice {
            volume: 0.8,
            pitch: 1.05,
            ..CharacterVoice::default()
        };
        let reading = event_player(Some(&record), None, Some(&voice), 0x3, false);
        assert!(reading.memory_role);
        assert!(reading.female);
        assert_eq!(reading.class, 0x0051_0000);
        assert_eq!(reading.potential, 4.0);
        assert_eq!(reading.collected, 0x3);
        assert_eq!((reading.voice_volume, reading.voice_pitch), (0.8, 1.05));
        // The body's roles answer over the record's.
        let roles = exa_ai::roles::Roles::default();
        assert!(!event_player(Some(&record), Some(&roles), None, 0, false).memory_role);
    }

    /// A fragment event on a player holding the memory role: the action is
    /// requested, the narrator topic asked for, the fragment joins the
    /// session's word and the record's potential gains one; the same
    /// fragment again does nothing.
    #[test]
    fn a_memory_fragment_acts_on_the_live_player() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Flow>()
            .init_resource::<FlowSteps>()
            .add_message::<exa_script::ScriptEffect>()
            .add_message::<exa_script::Narrate>()
            .add_message::<PainSounded>()
            .add_message::<exa_audio::PointSounded>()
            .add_systems(Update, run_game_events);
        let mut record = CharacterRecord::new();
        record.roles.push(exa_formats::characters::Role {
            id: MEMORY_ROLE,
            weight: 1.0,
            explicit: true,
            inherited: 0,
            variables: Vec::new(),
        });
        crate::game_event::set_potential(&mut record, 4.0);
        let player = app
            .world_mut()
            .spawn((
                PlayerBody::new(0.0),
                GlobalTransform::default(),
                CharacterData::of_record(record),
            ))
            .id();
        let fragment = |app: &mut App| {
            app.world_mut().write_message(exa_script::ScriptEffect(
                exa_script::dialogue::Effect::GameEvent(0xc0_0004),
            ));
            app.update();
        };
        fragment(&mut app);
        let world = app.world();
        assert_eq!(world.resource::<Flow>().story.memory, 4);
        assert_eq!(
            world.resource::<FlowSteps>().steps,
            vec![Step::RequestAction(crate::game_event::MEMORY_ACTION)]
        );
        let data = world.get::<CharacterData>(player).unwrap();
        assert_eq!(crate::game_event::potential(&data.record), 5.0);
        let topics: Vec<exa_script::Narrate> = app
            .world_mut()
            .resource_mut::<Messages<exa_script::Narrate>>()
            .drain()
            .collect();
        assert_eq!(topics.len(), 1);
        assert_eq!(
            (topics[0].topic.as_str(), topics[0].delay),
            ("XU_Memory", 2500)
        );
        let voices: Vec<PainSounded> = app
            .world_mut()
            .resource_mut::<Messages<PainSounded>>()
            .drain()
            .collect();
        assert_eq!(voices.len(), 1);
        assert_eq!(voices[0].sounds.voice.map(|(line, _)| line), Some(2));
        app.world_mut().resource_mut::<FlowSteps>().steps.clear();
        fragment(&mut app);
        assert!(app.world().resource::<FlowSteps>().steps.is_empty());
        let data = app.world().get::<CharacterData>(player).unwrap();
        assert_eq!(crate::game_event::potential(&data.record), 5.0);
    }

    /// The mind awakening on the unknown background: the potential becomes
    /// its old value plus ten less the fragments seen, every fragment counts
    /// as seen, and the class word takes the mind bits. Off the unknown
    /// background nothing happens.
    #[test]
    fn the_mind_awakening_writes_the_record_and_the_session() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Flow>()
            .init_resource::<FlowSteps>()
            .add_message::<exa_script::ScriptEffect>()
            .add_message::<exa_script::Narrate>()
            .add_message::<PainSounded>()
            .add_message::<exa_audio::PointSounded>()
            .add_systems(Update, run_game_events);
        let mut record = CharacterRecord::new();
        record.extra_word = 0x0051_0000;
        crate::game_event::set_potential(&mut record, 4.0);
        let player = app
            .world_mut()
            .spawn((
                PlayerBody::new(0.0),
                GlobalTransform::default(),
                CharacterData::of_record(record),
            ))
            .id();
        app.world_mut().resource_mut::<Flow>().story.memory = 0b111;
        app.world_mut().write_message(exa_script::ScriptEffect(
            exa_script::dialogue::Effect::GameEvent(crate::game_event::AWAKENING_MIND),
        ));
        app.update();
        assert_eq!(app.world().resource::<Flow>().story.memory, 0xffff);
        let data = app.world().get::<CharacterData>(player).unwrap();
        // 4 + 10 − 3.
        assert_eq!(crate::game_event::potential(&data.record), 11.0);
        assert_eq!(data.record.extra_word, 0x00d1_0000);
        let sounds: Vec<exa_audio::PointSounded> = app
            .world_mut()
            .resource_mut::<Messages<exa_audio::PointSounded>>()
            .drain()
            .collect();
        let names: Vec<&str> = sounds.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["VHM_Exert01", "VSM_Wail01"]);
        assert_eq!((sounds[0].flags, sounds[1].flags), (0x400, 0x4460));
        assert_eq!(sounds[1].position, Vec3::ZERO);
        // A knight's story: the awakening does nothing.
        app.world_mut().resource_mut::<Flow>().story.background = Background::Knight;
        app.world_mut().write_message(exa_script::ScriptEffect(
            exa_script::dialogue::Effect::GameEvent(crate::game_event::AWAKENING_FIRST),
        ));
        app.update();
        let data = app.world().get::<CharacterData>(player).unwrap();
        assert_eq!(data.record.extra_word, 0x00d1_0000);
    }

    /// The raise-dead grant: Insight's grant technique and Raise Dead's
    /// prerequisite learnt write the power with the granted flags; without
    /// the technique nothing is written. A held power keeps its bytes.
    #[test]
    fn the_raise_dead_grant_writes_the_power() {
        let node = |power: u32, prerequisite: u32| {
            let mut link = [0u8; 16];
            link[8..12].copy_from_slice(&prerequisite.to_le_bytes());
            exa_formats::power_tree::PowerNode {
                power,
                name_text: 0,
                description_text: 0,
                x: 0.0,
                y: 0.0,
                links: if prerequisite == 0 {
                    Vec::new()
                } else {
                    vec![link]
                },
            }
        };
        let nodes = [node(0x101, 0), node(RAISE_DEAD_POWER, 1)];
        let mut record = CharacterRecord::new();
        record.powers = Some(exa_formats::characters::Powers {
            word: 4.0f32.to_bits(),
            slots: [0; exa_formats::characters::POWER_SLOTS],
            known: vec![Power {
                id: 0x101,
                flags: crate::thaum::tree::LEARNED,
                blob: Some(vec![1, 2]),
            }],
        });
        assert!(!grant_raise_dead(&mut record, &nodes));
        record.skills[INSIGHT].points = 10_000;
        record.skills[INSIGHT].techniques[0] = crate::thaum::tree::GRANT_TECHNIQUE;
        assert!(grant_raise_dead(&mut record, &nodes));
        let known = &record.powers.as_ref().unwrap().known;
        assert_eq!(known.len(), 2);
        assert_eq!(known[0].blob, Some(vec![1, 2]));
        assert_eq!(
            (known[1].id, known[1].flags),
            (RAISE_DEAD_POWER, crate::thaum::tree::GRANTED_FLAGS)
        );
    }
}
