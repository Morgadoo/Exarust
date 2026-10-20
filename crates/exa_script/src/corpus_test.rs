//! Checks against the installed role database and narrator graph (skipped
//! without the game), and the plugin on synthetic data.

use bevy::prelude::*;
use exa_formats::roles::{Branch, Dialogue, DialogueActor, Line, NODE_ENTRY, Name, Node, RoleDb};

use crate::bubbles::Speaker;
use crate::cast::{Cast, Character};
use crate::dialogue::{Engine, Library, Scene};
use crate::{Conversations, Narrate, ScriptPlugin, Scripts};

const PLAYER: u32 = 2;
const DERRIN: u32 = 0x8000_0008;

fn scripts() -> Option<Scripts> {
    let dir = exa_assets::locate_install(None)?;
    let game = exa_assets::GameData::open(&dir).ok()?;
    let (s, errors) = Scripts::load(&game);
    assert!(errors.is_empty(), "{errors:?}");
    Some(s)
}

fn scene<'a>(s: &'a Scripts, cast: &'a mut Cast, now: u32) -> Scene<'a> {
    Scene {
        library: Library {
            roles: &s.roles,
            narrator: s.narrator.as_ref(),
        },
        cast,
        now,
        world_time: 5000 + now / 1000,
        player: Some(PLAYER),
        locale: 2,
        effects: Vec::new(),
    }
}

fn each_branch(b: &Branch, f: &mut impl FnMut(&Branch)) {
    f(b);
    for r in &b.replies {
        each_branch(r, f);
    }
}

/// Every shipped script and line runs through the interpreter (roles'
/// init, encounter, interaction and event scripts, every branch's
/// condition, action and lines), with both actors holding the role.
#[test]
fn every_shipped_script_runs() {
    let Some(s) = scripts() else { return };
    let mut count = 0;
    for role in &s.roles.roles {
        let mut cast = Cast::default();
        for id in [DERRIN, PLAYER] {
            let mut c = Character::new(id, "X");
            c.add_role(&s.roles, role.id);
            cast.characters.push(c);
        }
        let mut e = Engine::default();
        let mut sc = scene(&s, &mut cast, 0);
        let actors = vec![Some(DERRIN), Some(PLAYER)];
        let mut texts: Vec<Vec<u8>> = vec![role.init_script.clone()];
        for entry in &role.entries {
            texts.extend(entry.encounter.clone());
            texts.extend(entry.interaction.clone());
            texts.extend(entry.event.clone());
            for n in entry.dialogue.iter().flat_map(|d| d.nodes.iter().flatten()) {
                for b in &n.branches {
                    each_branch(b, &mut |b| {
                        texts.push(b.condition.clone());
                        texts.push(b.action.clone());
                        texts.extend(b.lines.iter().map(|l| l.text.clone()));
                    });
                }
            }
        }
        for t in texts {
            e.run_script(&mut sc, actors.clone(), role.id, &t);
            count += 1;
        }
    }
    assert!(count > 1500, "{count} scripts");
}

/// DERRINTEST's recruitment, then the player talks to Derrin: Explorer's
/// dialogue toward a subordinate offers its topics; "Follow me" is held
/// back while Derrin already follows (Activity 4).
#[test]
fn derrin_offers_the_explorer_topics() {
    let Some(s) = scripts() else { return };
    let mut cast = Cast {
        characters: vec![
            Character::new(DERRIN, "Derrin"),
            Character::new(PLAYER, "Ermus"),
        ],
    };
    let mut e = Engine::default();
    let mut sc = scene(&s, &mut cast, 0);
    let recruit = s.roles.get(57022).unwrap().entries[0]
        .dialogue
        .as_ref()
        .unwrap()
        .node(1)
        .unwrap()
        .branches[0]
        .action
        .clone();
    e.run_script(&mut sc, vec![Some(DERRIN), Some(PLAYER)], 57022, &recruit);
    let derrin = cast.get(DERRIN).unwrap();
    assert!(
        derrin.holds(19) && derrin.holds(31),
        "XA_Derrin and Explorer"
    );
    assert!(derrin.holds_toward(PLAYER, 30), "Leader toward the player");
    assert!(
        cast.get(PLAYER).unwrap().holds_toward(DERRIN, 29),
        "Subordinate"
    );
    let values = &derrin.roles.iter().find(|r| r.id == 31).unwrap().values;
    assert_eq!(values.first(), Some(&4), "Explorer's Activity = 4");
    let mut sc = scene(&s, &mut cast, 0);
    e.talk(&mut sc, DERRIN);
    let texts: Vec<String> = e
        .panel
        .entries
        .iter()
        .map(|c| String::from_utf8_lossy(&c.text).into_owned())
        .collect();
    for t in [
        "Wait for me here.",
        "How are you doing?",
        "Let me see your equipment.",
    ] {
        assert!(texts.iter().any(|x| x == t), "{t} in {texts:?}");
    }
    assert!(!texts.iter().any(|x| x == "Follow me."), "{texts:?}");
}

/// The shipped narrator's Corlian node offers three replies, the first
/// with the player's name.
#[test]
fn the_shipped_narrator_greets_corlian() {
    let Some(s) = scripts() else { return };
    let mut cast = Cast {
        characters: vec![Character::new(PLAYER, "Ermus")],
    };
    let mut e = Engine::default();
    let mut sc = scene(&s, &mut cast, 0);
    e.narrate(&mut sc, Name::from_bytes(b"Corlian"), 0);
    assert_eq!(e.bubbles.list[0].speaker, Speaker::Narrator);
    assert_eq!(e.panel.entries.len(), 3);
    assert_eq!(e.panel.entries[0].text, b"I am Ermus, master of forces.");
}

/// The plugin steps the engine on the virtual clock: a narrator message
/// puts up the narrator's line.
#[test]
fn the_plugin_runs_the_narrator() {
    let narrator = Dialogue {
        target: 0,
        actors: vec![DialogueActor::default(); 2],
        nodes: vec![Some(Node {
            flags: NODE_ENTRY,
            title: Name::from_bytes(b"Hello"),
            branches: vec![Branch {
                lines: vec![Line {
                    speaker: 1,
                    text: b"You wake.".to_vec(),
                }],
                ..Branch::default()
            }],
            ..Node::default()
        })],
    };
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, ScriptPlugin))
        .insert_resource(Scripts {
            roles: RoleDb::default(),
            narrator: Some(narrator),
        });
    {
        let mut c = app.world_mut().resource_mut::<Conversations>();
        c.cast.characters.push(Character::new(PLAYER, "Ermus"));
        c.player = Some(PLAYER);
    }
    app.world_mut().write_message(Narrate {
        topic: "Hello".into(),
        delay: 0,
    });
    app.update();
    let c = app.world().resource::<Conversations>();
    assert_eq!(c.engine.bubbles.list.len(), 1);
    assert_eq!(c.engine.bubbles.list[0].text, b"You wake.");
}
