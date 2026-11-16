//! `--new-game story[:N]`, a debug aid: once the first load has ended and
//! a player stands in it, a new story starts through the flow's own session
//! begin ([`Flow::begin_session`] around [`Flow::start_story`]), the two
//! calls the character creator's BEGIN makes, without the creator's form.
//! So the session begin's new slot ([`crate::flow::Step::NewSlot`]), the
//! starter's level load and the story's later saves run as after a menu
//! start; with `--save-dir` the new slot and its saves land in that folder.
//!
//! The play level's player stands in for the created character: its record
//! becomes [`CreatedCharacter`] (unless one was committed), and it is
//! despawned as the creator's commit despawns the survivor, so the story's
//! level load spawns the player from that record.
//!
//! The arena career has no starter in the flow: its start is the
//! creator's commit, so `--new-game` offers the story only.

use bevy::prelude::*;

use crate::creator_screen::CreatedCharacter;
use crate::flow::{Background, Flow, FlowSteps, FlowSystems};

/// The new game `--new-game` starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewGame {
    /// A new story from this background.
    Story(Background),
}

impl NewGame {
    /// `story` (the unknown background) or `story:N` (background N, 0 to
    /// 3); `None` for any other word.
    pub fn parse(word: &str) -> Option<Self> {
        let rest = word.strip_prefix("story")?;
        let index = match rest.strip_prefix(':') {
            Some(index) => index.parse::<u32>().ok().filter(|i| *i < 4)?,
            None if rest.is_empty() => 0,
            None => return None,
        };
        Some(NewGame::Story(Background::from_index(index)))
    }
}

/// `--new-game`: the game to start once the first load has ended, and
/// whether it has started.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct NewGameOnLoad {
    pub game: Option<NewGame>,
    pub done: bool,
}

/// Runs [`start_on_load`] before the flow applies its steps.
pub struct NewGamePlugin;

impl Plugin for NewGamePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NewGameOnLoad>().add_systems(
            Update,
            start_on_load
                .after(crate::thaum::power::teach_on_death)
                .before(FlowSystems::ApplySteps)
                .in_set(exa_physics::FramePhase::Session),
        );
    }
}

/// Starts [`NewGameOnLoad`]'s game once, the first frame no load is
/// showing and a player exists.
#[allow(clippy::too_many_arguments)]
pub fn start_on_load(
    mut on_load: ResMut<NewGameOnLoad>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    flow: Option<ResMut<Flow>>,
    steps: Option<ResMut<FlowSteps>>,
    created: Option<ResMut<CreatedCharacter>>,
    players: Query<
        (Entity, Option<&exa_character::record::CharacterData>),
        With<crate::player::PlayerBody>,
    >,
    ticks: Option<Res<exa_physics::body::BodyTicks>>,
    mut commands: Commands,
) {
    if on_load.done || loading.is_some() || players.is_empty() {
        return;
    }
    let Some(game) = on_load.game else {
        return;
    };
    on_load.done = true;
    let (Some(mut flow), Some(mut steps)) = (flow, steps) else {
        warn!("--new-game: no game flow; nothing starts");
        return;
    };
    let NewGame::Story(background) = game;
    if let Some(mut created) = created
        && created.0.is_none()
    {
        created.0 = players
            .iter()
            .find_map(|(_, data)| data.map(|data| data.record.clone()));
    }
    for (player, _) in &players {
        commands.entity(player).despawn();
    }
    let now = ticks.as_ref().map_or(0, |t| t.millis() as u32);
    let viewport = flow.viewport;
    // [guess] the measured body height is the average the flow's spawn
    // arithmetic expects, as the creator's BEGIN passes.
    let start = flow.start_story(background, 1.0, viewport, now);
    let begun = flow.begin_session(start);
    info!(
        "new game: a story from the {background:?} background begins through the session begin ({} steps, first {:?})",
        begun.len(),
        begun.first()
    );
    steps.steps.extend(begun);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow::{Mode, Step};

    #[test]
    fn the_new_game_word_names_a_story_background() {
        assert_eq!(
            NewGame::parse("story"),
            Some(NewGame::Story(Background::Unknown))
        );
        assert_eq!(
            NewGame::parse("story:3"),
            Some(NewGame::Story(Background::Villager))
        );
        assert_eq!(NewGame::parse("story:4"), None);
        assert_eq!(NewGame::parse("arena"), None);
        assert_eq!(NewGame::parse("storyx"), None);
    }

    /// Once the load has ended the story starts through the session begin:
    /// its first step is the new slot, the story's level load follows, the
    /// flow is in story mode, and the play level's player is the created
    /// character and is despawned.
    #[test]
    fn the_new_story_starts_through_the_session_begin_with_its_new_slot() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .configure_sets(Update, FlowSystems::ApplySteps)
            .init_resource::<Flow>()
            .init_resource::<FlowSteps>()
            .init_resource::<CreatedCharacter>()
            .insert_resource(NewGameOnLoad {
                game: Some(NewGame::Story(Background::Unknown)),
                done: false,
            })
            .add_plugins(NewGamePlugin);
        app.insert_resource(exa_ui::LoadingStatus::default());
        app.update();
        assert!(app.world().resource::<FlowSteps>().steps.is_empty());
        app.world_mut().remove_resource::<exa_ui::LoadingStatus>();
        app.update();
        // No player yet: nothing starts.
        assert!(!app.world().resource::<NewGameOnLoad>().done);
        let mut record = exa_formats::characters::CharacterRecord::reset();
        record.name = "Wanderer".into();
        let mut data = exa_character::record::CharacterData::new();
        data.record = record;
        let player = app
            .world_mut()
            .spawn((crate::player::PlayerBody::new(0.0), data))
            .id();
        app.update();
        let steps = &app.world().resource::<FlowSteps>().steps;
        assert_eq!(steps.first(), Some(&Step::NewSlot));
        assert!(steps.iter().any(|step| matches!(
            step,
            Step::LoadLevel {
                locale: "exanima01",
                ..
            }
        )));
        assert_eq!(app.world().resource::<Flow>().mode, Mode::Story);
        let created = app.world().resource::<CreatedCharacter>();
        assert_eq!(
            created.0.as_ref().map(|r| r.name.as_str()),
            Some("Wanderer")
        );
        assert!(app.world().get_entity(player).is_err());
        // Once only.
        let count = app.world().resource::<FlowSteps>().steps.len();
        app.world_mut().spawn(crate::player::PlayerBody::new(0.0));
        app.update();
        assert_eq!(app.world().resource::<FlowSteps>().steps.len(), count);
    }
}
