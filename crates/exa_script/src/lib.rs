//! Story scripting and dialogue: the role and dialogue script interpreter,
//! the script classes and globals, topic memory, the conversation engine,
//! speech bubbles, scripted objects, and the story/journal state.
//!
//! [`ScriptPlugin`] loads the role database and the narrator graph from the
//! installation, keeps the session's conversations, and steps them each
//! frame on the virtual clock. Other systems talk to it through messages:
//! [`Narrate`], [`NarrateText`], [`Talk`], [`PickReply`] in, [`ScriptEffect`] out.
//!
//! See `README.md` next to this crate and `docs/PLAN.md`.

pub mod bubbles;
pub mod builtins;
pub mod cast;
pub mod dialogue;
pub mod interp;
pub mod objects;
pub mod script_class;
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
            .add_message::<NarrateText>()
            .add_message::<PickReply>()
            .add_message::<LeaveDialogue>()
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
    /// The second player slot (its lines are reply choices too).
    pub player2: Option<u32>,
    /// World time (s) the session started at (the save's clock).
    pub world_base: u32,
    /// The current locale id (`locales.rdb`).
    pub locale: u32,
    pub objects: objects::Objects,
}

impl Conversations {
    /// Runs `f` on the engine with the session's scene at game time `now`
    /// (ms): the role library, the cast, the world clock, the players and
    /// the locale. Returns `f`'s answer and the effects the scripts asked
    /// for, in order.
    pub fn with_scene<R>(
        &mut self,
        scripts: &Scripts,
        now: u32,
        f: impl FnOnce(&mut Engine, &mut Scene<'_>) -> R,
    ) -> (R, Vec<Effect>) {
        let Self {
            engine,
            cast,
            player,
            player2,
            world_base,
            locale,
            ..
        } = self;
        let mut scene = Scene {
            library: Library {
                roles: &scripts.roles,
                narrator: scripts.narrator.as_ref(),
            },
            cast,
            now,
            world_time: topics::world_time(*world_base, now, 0),
            player: *player,
            player2: *player2,
            locale: *locale,
            effects: Vec::new(),
        };
        let answer = f(engine, &mut scene);
        (answer, std::mem::take(&mut scene.effects))
    }
}

/// Start the narrator at a node of `narrator.rcd` (after `delay` ms).
#[derive(Message, Debug, Clone)]
pub struct Narrate {
    pub topic: String,
    pub delay: u32,
}

/// Show narrator text without a graph (after `delay` ms; the original uses
/// this for the plain-text narrator lines of the player controls).
#[derive(Message, Debug, Clone)]
pub struct NarrateText {
    pub text: String,
    pub delay: u32,
}

/// The player talks to a character (opens the reply panel).
#[derive(Message, Debug, Clone, Copy)]
pub struct Talk(pub u32);

/// The player picks entry n of the reply panel.
#[derive(Message, Debug, Clone, Copy)]
pub struct PickReply(pub usize);

/// The player ends every conversation it is in (the Talk key while a
/// dialogue is open).
#[derive(Message, Debug, Clone, Copy, Default)]
pub struct LeaveDialogue;

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
    mut narrate_text: MessageReader<NarrateText>,
    mut talk: MessageReader<Talk>,
    mut pick: MessageReader<PickReply>,
    mut leave: MessageReader<LeaveDialogue>,
    mut effects: MessageWriter<ScriptEffect>,
) {
    let now = time.elapsed().as_millis() as u32;
    let player = conversations.player;
    let ((), out) = conversations.with_scene(&scripts, now, |engine, scene| {
        for n in narrate.read() {
            engine.narrate(scene, Name::from_bytes(n.topic.as_bytes()), n.delay);
        }
        for n in narrate_text.read() {
            engine.narrate_text(scene, n.text.as_bytes(), n.delay);
        }
        for t in talk.read() {
            engine.talk(scene, t.0);
        }
        for p in pick.read() {
            engine.pick(scene, p.0);
        }
        for _ in leave.read() {
            if let Some(who) = player {
                engine.leave(scene, who);
            }
        }
        engine.update(scene);
    });
    for e in out {
        effects.write(ScriptEffect(e));
    }
}

#[cfg(test)]
mod corpus_test;
#[cfg(test)]
mod dialogue_test;
#[cfg(test)]
mod interp_test;
