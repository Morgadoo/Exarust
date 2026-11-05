//! The Ctrl/double-click interaction on a living character other than the
//! player: its roles' interaction condition, as the controls' interaction
//! hook runs it.
//!
//! The hand sends the click on a living character as a [`CharacterUse`]
//! (the player's own body opens the inventory, a dead one its loot window,
//! both in the hand). The original then goes on only while the character
//! is not collapsing (condition bit `0x800`) and the player possesses no
//! other body. It asks the roles' entry finder for the interaction
//! condition the character holds toward the player (section `0x40`,
//! [`exa_ai::roles::interaction_condition`]):
//!
//! - no such section: the condition fails;
//! - an empty one passes;
//! - otherwise the text runs on its own script context with the character
//!   as `Actor[0]` and the player as `Actor[1]` (no running role), and a
//!   `result` above 0 passes (the shipped Golem Boss asks
//!   `result = Actor[1].HasItem($1DE);`).
//!
//! A failed condition still lets a character through whose power list
//! holds a Loyalty marker (`0x10010A`) or a Command Dead marker
//! (`0x10010D`) the player cast. Either way the character's inventory
//! window opens: the same call the player's own body makes, with the
//! toggle, bound to that character ([`crate::gui_screens::GameScreens::show_inventory`]):
//! its pack is the character's carried list, its character view the
//! character's figure, and it is placed at the screen's right edge.
//!
//! Evidence: REA on the controls' interaction hook and its condition test
//! (`Exanima.exe` 0.9.5, checked against the assembly).

use bevy::prelude::*;
use exa_ai::brain::CharacterId;
use exa_ai::roles::{self, Partner, Relations, Roles};
use exa_combat::Condition;

use crate::mind::casts::{COMMAND_DEAD_MARK, LOYALTY_MARK, MindBody};
use crate::player::PlayerBody;

/// The player's interaction on a living character's body.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterUse {
    pub body: Entity,
}

/// What the interaction did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UseOutcome {
    /// The character is collapsing, or the player possesses another body.
    Refused,
    /// Neither the condition nor a marker of the player's lets it through.
    Nothing,
    /// The character's inventory window opens.
    Inventory { by_condition: bool },
}

/// The character's condition word bit the interaction refuses (a lethal
/// wound's collapse; with the dead mode it is a loot instead).
pub const REFUSED_CONDITION: u32 = 0x800;

/// The markers that let the player open a character's inventory when the
/// condition fails, if the player cast them.
pub const OPENING_MARKS: [u32; 2] = [LOYALTY_MARK, COMMAND_DEAD_MARK];

pub struct CharacterUsePlugin;

impl Plugin for CharacterUsePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<CharacterUse>()
            .add_message::<exa_script::ScriptEffect>()
            .add_message::<crate::gui_screens::ScreenRequest>()
            .init_resource::<CharacterUseScript>()
            .add_systems(
                Update,
                (scripted_character_uses, run_character_uses)
                    .chain()
                    .after(crate::hand::HandSystems),
            );
    }
}

/// One scripted interaction of `--character-use`: the character's record
/// id, the frame after the load, and whether the player's Loyalty marker is
/// put on the character first (`+`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScriptedUse {
    pub id: u32,
    pub frame: u32,
    pub loyal: bool,
}

/// `--character-use ID@FRAME[+],...`: the player's interaction on the
/// living character with that record id at that frame after the load, as
/// the Ctrl/double click on its body sends it (a debug tool). `+` first
/// records a Loyalty marker the player cast on the character, so a failed
/// condition still opens its inventory.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct CharacterUseScript(pub Vec<ScriptedUse>);

impl CharacterUseScript {
    pub fn parse(text: &str) -> Option<Self> {
        text.split(',')
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(|entry| {
                let (entry, loyal) = match entry.strip_suffix('+') {
                    Some(rest) => (rest, true),
                    None => (entry, false),
                };
                let (id, frame) = entry.split_once('@')?;
                Some(ScriptedUse {
                    id: crate::gui_drop::parse_id(id)?,
                    frame: frame.trim().parse().ok()?,
                    loyal,
                })
            })
            .collect::<Option<Vec<_>>>()
            .filter(|uses| !uses.is_empty())
            .map(Self)
    }
}

/// Sends the scripted interactions ([`CharacterUseScript`]).
fn scripted_character_uses(
    script: Res<CharacterUseScript>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    mut frames: Local<u32>,
    mut characters: Query<(Entity, &CharacterId, Option<&mut MindBody>), Without<PlayerBody>>,
    players: Query<Entity, With<PlayerBody>>,
    mut uses: MessageWriter<CharacterUse>,
) {
    let Some(last) = script.0.iter().map(|u| u.frame).max() else {
        return;
    };
    if loading.is_some() || *frames > last {
        return;
    }
    *frames += 1;
    let player = players.iter().next().map(crate::mind::runtime::body_id);
    for entry in script.0.iter().filter(|u| u.frame == *frames) {
        let Some((body, _, mind)) = characters.iter_mut().find(|(_, id, _)| id.0 == entry.id)
        else {
            info!("character use script: no living character {}", entry.id);
            continue;
        };
        if entry.loyal
            && let Some(mut mind) = mind
        {
            mind.powers.push(crate::mind::casts::MindPower::recorded(
                LOYALTY_MARK,
                player,
                None,
            ));
        }
        info!(
            "character use script: the player's interaction on {} at frame {}{}",
            entry.id,
            entry.frame,
            if entry.loyal {
                ", with the player's Loyalty marker"
            } else {
                ""
            }
        );
        uses.write(CharacterUse { body });
    }
}

/// The interaction condition's verdict on its section: none fails, an
/// empty text passes, a text passes when its `result` is above 0.
pub fn condition_passes(section: Option<&[u8]>, run: impl FnOnce(&[u8]) -> i32) -> bool {
    match section {
        None => false,
        Some([]) => true,
        Some(text) => run(text) > 0,
    }
}

/// Whether a failed condition is overridden: a Loyalty or Command Dead
/// marker on the character that the player cast.
pub fn marked_by(mind: Option<&MindBody>, player: Option<u32>) -> bool {
    let (Some(mind), Some(player)) = (mind, player) else {
        return false;
    };
    mind.powers
        .iter()
        .any(|p| OPENING_MARKS.contains(&p.id) && p.caster == Some(player))
}

type Character<'a> = (
    &'a CharacterId,
    &'a Roles,
    Option<&'a Relations>,
    Option<&'a Condition>,
    Option<&'a MindBody>,
);

/// Runs the interactions the hand sent.
#[allow(clippy::too_many_arguments)]
pub fn run_character_uses(
    mut uses: MessageReader<CharacterUse>,
    scripts: Option<Res<exa_script::Scripts>>,
    conversations: Option<ResMut<exa_script::Conversations>>,
    possession: Option<Res<crate::teleporters::Possession>>,
    time: Option<Res<Time<Virtual>>>,
    characters: Query<Character<'_>, Without<PlayerBody>>,
    players: Query<(Entity, &CharacterId, &Roles, Option<&Condition>), With<PlayerBody>>,
    mut effects: MessageWriter<exa_script::ScriptEffect>,
    mut screens: MessageWriter<crate::gui_screens::ScreenRequest>,
) {
    let mut conversations = conversations;
    let now = time.map_or(0, |t| t.elapsed().as_millis() as u32);
    for CharacterUse { body } in uses.read().copied() {
        let Ok((id, held, relations, condition, mind)) = characters.get(body) else {
            continue;
        };
        let Some((player_body, player, player_roles, player_condition)) = players.iter().next()
        else {
            continue;
        };
        let word = condition.map_or(0, |c| c.0);
        if word & REFUSED_CONDITION != 0 || possession.as_deref().is_some_and(|p| p.active) {
            info!("character use of {}: {:?}", id.0, UseOutcome::Refused);
            continue;
        }
        let partner = Partner {
            id: player.0,
            roles: player_roles,
            former: None,
        };
        let empty = Relations::default();
        let section = scripts.as_deref().and_then(|s| {
            roles::interaction_condition(&s.roles, held, relations.unwrap_or(&empty), &partner)
        });
        let mut result = None;
        let passed = condition_passes(section, |text| {
            let (Some(scripts), Some(conversations)) =
                (scripts.as_deref(), conversations.as_deref_mut())
            else {
                return 0;
            };
            let (r, out) = conversations.with_scene(scripts, now, |engine, scene| {
                engine.run_script(scene, vec![Some(id.0), Some(player.0)], 0, text)
            });
            for effect in out {
                effects.write(exa_script::ScriptEffect(effect));
            }
            result = Some(r);
            r
        });
        let player_mind = Some(crate::mind::runtime::body_id(player_body));
        let outcome = if passed {
            UseOutcome::Inventory { by_condition: true }
        } else if marked_by(mind, player_mind) {
            UseOutcome::Inventory {
                by_condition: false,
            }
        } else {
            UseOutcome::Nothing
        };
        info!(
            "character use of {}: interaction condition {:?} -> result {result:?}, {outcome:?}",
            id.0,
            section.map(String::from_utf8_lossy),
        );
        if matches!(outcome, UseOutcome::Inventory { .. }) {
            let word = player_condition.copied().unwrap_or_default();
            screens.write(crate::gui_screens::ScreenRequest {
                screen: exa_ui::game_gui::GameScreen::Inventory,
                player_ready: !word.dying(),
                player_alive: !word.is_dead(),
                target: Some(body.index().index() as i32),
                flags: 0,
                toggle: true,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mind::casts::MindPower;

    /// No section fails, an empty one passes, a script passes on a result
    /// above 0 only.
    #[test]
    fn the_condition_reads_its_section_and_result() {
        assert!(!condition_passes(None, |_| 1));
        assert!(condition_passes(Some(b""), |_| 0));
        assert!(condition_passes(Some(b"result = 1;"), |_| 1));
        assert!(!condition_passes(Some(b"result = 0;"), |_| 0));
        assert!(!condition_passes(Some(b"result = -1;"), |_| -1));
    }

    /// A Loyalty or Command Dead marker counts only when the player cast
    /// it; another power or another caster does not.
    #[test]
    fn the_use_script_reads_ids_frames_and_the_marker() {
        let script = CharacterUseScript::parse("0x2b@40+, 7@3").unwrap();
        assert_eq!(
            script.0,
            vec![
                ScriptedUse {
                    id: 0x2b,
                    frame: 40,
                    loyal: true
                },
                ScriptedUse {
                    id: 7,
                    frame: 3,
                    loyal: false
                },
            ]
        );
        assert!(CharacterUseScript::parse("").is_none());
        assert!(CharacterUseScript::parse("43").is_none());
    }

    #[test]
    fn a_players_marker_opens_the_inventory() {
        let mut mind = MindBody::new(9);
        assert!(!marked_by(Some(&mind), Some(1)));
        mind.powers
            .push(MindPower::recorded(LOYALTY_MARK, Some(2), None));
        assert!(!marked_by(Some(&mind), Some(1)));
        mind.powers
            .push(MindPower::recorded(COMMAND_DEAD_MARK, Some(1), None));
        assert!(marked_by(Some(&mind), Some(1)));
        assert!(!marked_by(None, Some(1)));
    }
}
