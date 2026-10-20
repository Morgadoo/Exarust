//! Story scripting and dialogue: the role and dialogue script interpreter,
//! the script classes and globals, topic memory, the conversation engine,
//! speech bubbles, scripted objects, and the story/journal state.
//!
//! [`ScriptPlugin`] loads the role database and the narrator graph from the
//! installation, keeps the session's conversations, and steps them each
//! frame on the virtual clock. Other systems talk to it through messages:
//! [`Narrate`], [`Talk`], [`PickReply`] in, [`ScriptEffect`] out.
//!
//! See `README.md` next to this crate and `docs/PLAN.md`.

pub mod bubbles;
pub mod builtins;
pub mod cast;
pub mod dialogue;
pub mod interp;
pub mod objects;
pub mod story;
pub mod topics;

use bevy::prelude::*;
use exa_assets::{Game, PackKind};
use exa_formats::roles::{Dialogue, Name, RoleDb};

use crate::cast::Cast;
use crate::dialogue::{Effect, Engine, Library, Scene};

pub struct ScriptPlugin;

impl Plugin for ScriptPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Conversations>()
            .add_message::<Narrate>()
            .add_message::<Talk>()
            .add_message::<PickReply>()
            .add_message::<ScriptEffect>()
            .add_systems(
                Update,
                (
                    load_scripts
                        .run_if(resource_exists::<Game>)
                        .run_if(not(resource_exists::<Scripts>)),
                    run_conversations.run_if(resource_exists::<Scripts>),
                )
                    .chain(),
            );
    }
}

/// The installation's role database and narrator graph.
#[derive(Resource, Debug, Default)]
pub struct Scripts {
    pub roles: RoleDb,
    pub narrator: Option<Dialogue>,
}

impl Scripts {
    /// Read `charroles.rdb` and `narrator.rcd` from the resource pack (a
    /// missing or broken file leaves its part empty).
    pub fn load(game: &exa_assets::GameData) -> (Self, Vec<String>) {
        let mut out = Self::default();
        let mut errors = Vec::new();
        let Some(pack) = game.pack(PackKind::Resource) else {
            return (out, vec!["no resource pack".into()]);
        };
        match pack.get("charroles.rdb").map(RoleDb::parse) {
            Some(Ok(db)) => out.roles = db,
            Some(Err(e)) => errors.push(format!("charroles.rdb: {e}")),
            None => errors.push("charroles.rdb missing".into()),
        }
        match pack.get("narrator.rcd").map(Dialogue::parse) {
            Some(Ok(d)) => out.narrator = Some(d),
            Some(Err(e)) => errors.push(format!("narrator.rcd: {e}")),
            None => errors.push("narrator.rcd missing".into()),
        }
        (out, errors)
    }
}

/// The session's conversations and the characters they act on.
#[derive(Resource, Debug, Default)]
pub struct Conversations {
    pub engine: Engine,
    pub cast: Cast,
    /// The player's character id.
    pub player: Option<u32>,
    /// World time (s) the session started at (the save's clock).
    pub world_base: u32,
    /// The current locale id (`locales.rdb`).
    pub locale: u32,
    pub objects: objects::Objects,
}

/// Start the narrator at a node of `narrator.rcd` (after `delay` ms).
#[derive(Message, Debug, Clone)]
pub struct Narrate {
    pub topic: String,
    pub delay: u32,
}

/// The player talks to a character (opens the reply panel).
#[derive(Message, Debug, Clone, Copy)]
pub struct Talk(pub u32);

/// The player picks entry n of the reply panel.
#[derive(Message, Debug, Clone, Copy)]
pub struct PickReply(pub usize);

/// What a script asked the game to do.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct ScriptEffect(pub Effect);

fn load_scripts(game: Res<Game>, mut commands: Commands) {
    let (scripts, errors) = Scripts::load(&game.0);
    for e in errors {
        warn!("scripts: {e}");
    }
    info!(
        "scripts: {} roles, narrator {}",
        scripts.roles.roles.len(),
        if scripts.narrator.is_some() {
            "loaded"
        } else {
            "missing"
        }
    );
    commands.insert_resource(scripts);
}

#[allow(clippy::too_many_arguments)]
fn run_conversations(
    time: Res<Time<Virtual>>,
    scripts: Res<Scripts>,
    mut conversations: ResMut<Conversations>,
    mut narrate: MessageReader<Narrate>,
    mut talk: MessageReader<Talk>,
    mut pick: MessageReader<PickReply>,
    mut effects: MessageWriter<ScriptEffect>,
) {
    let now = time.elapsed().as_millis() as u32;
    let Conversations {
        engine,
        cast,
        player,
        world_base,
        locale,
        ..
    } = &mut *conversations;
    let mut scene = Scene {
        library: Library {
            roles: &scripts.roles,
            narrator: scripts.narrator.as_ref(),
        },
        cast,
        now,
        world_time: topics::world_time(*world_base, now, 0),
        player: *player,
        locale: *locale,
        effects: Vec::new(),
    };
    for n in narrate.read() {
        engine.narrate(&mut scene, Name::from_bytes(n.topic.as_bytes()), n.delay);
    }
    for t in talk.read() {
        engine.talk(&mut scene, t.0);
    }
    for p in pick.read() {
        engine.pick(&mut scene, p.0);
    }
    engine.update(&mut scene);
    for e in scene.effects.drain(..) {
        effects.write(ScriptEffect(e));
    }
}

#[cfg(test)]
mod corpus_test;
#[cfg(test)]
mod dialogue_test;
#[cfg(test)]
mod interp_test;
