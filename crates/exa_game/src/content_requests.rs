//! The content stream's requested objects and characters in the running
//! game: the stream thread ([`exa_assets::stream_worker`]) builds them off
//! the main thread and the callers fetch them back by the key they posted.
//!
//! **What the original posts.** Its object ring has one poster, the item
//! icon cache: an icon it has not rendered posts the item's id, its variant
//! flags with the streamed bit (`0x10000000`) and the item renderer's
//! scene, and its draw then spins on the object fetch (a zero sleep and a
//! texture upload pass between tries) until the node arrives. The thread's
//! object builder finds the record by id and its model section; a model
//! named with `@` goes to the item factory, any other is loaded from the
//! object libraries when the request has the streamed bit; a record or
//! model that is not found counts a missing object. Its character ring has
//! several posters: the story load's identity reset (the loaded player,
//! request word `0x1000`: neither AI start nor saved AI pose), the
//! character creator, the game screen, the arena's fighters, the traders
//! and the summoned demon; each fetches by its record and the world scene,
//! the load spinning with a zero sleep until the character is built.
//!
//! **What Exarust builds there.** The stream thread builds what a model
//! takes on the CPU ([`exa_assets::model::ModelJob::build`]: the parse, the
//! skin weights, the meshes and their tangents); the main thread adds the
//! built meshes and materials as assets ([`ModelLoader::finish_model`]) and
//! spawns the entities, as Bevy's world lives there. Wired posters:
//!
//! - the item icons ([`crate::item_icon_render`]): an icon whose model is
//!   not loaded posts its object request and waits on the fetch over the
//!   frames, the original's spin; a factory weapon (`@`) is still
//!   assembled on the main thread;
//! - the story load's loaded player ([`crate::save_live`]): the reset's
//!   request is posted with the player's body, morphed by the record's
//!   physique, and the level's player spawn fetches it, waiting while it
//!   is built;
//! - the level's characters ([`ContentRequests::stream_characters`]): while
//!   the collision builds, each body variant is read, parsed, morphed and
//!   meshed on the thread, nearest the player's start first, and the pump
//!   hands it over for the model cache; the spawn finds it built and does
//!   not wait for the rest.
//!
//! The record lookup is the poster's: Exarust's object database lives in
//! the main thread's catalogue, so a post hands the thread the record's
//! model entry with the request.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use bevy::prelude::*;
use exa_assets::content_stream::{
    CharacterRequest, ContentBuilders, Fetch, LevelNode, ObjectRequest, StreamedCharacter,
};
use exa_assets::model::{BuiltModel, ModelJob};
use exa_assets::morph::PendingJob;
use exa_assets::stream_worker::StreamWorker;
use exa_assets::{Game, GameData, PackKind};

/// The object request flag the icon cache always sets: the builder loads
/// the record's model from the object libraries only for a request with it
/// (`exa_items::icon_cache::variant::STREAMED`).
pub const STREAMED: u32 = 0x1000_0000;

/// The scene key the item icons' requests carry: the item renderer's own
/// scene, apart from the world's [Exarust's key: the original passes the
/// renderer scene's address].
pub const ICON_SCENE: u64 = 0x1C0_0001;

/// The world scene's key, which the loaded player's request and fetch carry
/// [Exarust's key: the original passes the world scene's address].
pub const WORLD_SCENE: u64 = 0x5CE_0001;

/// The scene key a level's streamed characters carry [Exarust's key: the
/// original passes the level scene's address].
pub const LEVEL_SCENE: u64 = 0x1E7_0001;

/// A level character's body for the stream thread to build: its model's
/// pending job and where it stands.
pub struct StreamedBody {
    pub job: PendingJob,
    pub position: Vec3,
}

/// A model the stream thread built, waiting for the main thread.
pub struct Built {
    pub model: BuiltModel,
    /// The model cache key the built model belongs under.
    pub key: String,
    /// How long the thread took to build it.
    pub took: Duration,
}

/// What a fetch of a requested object found.
pub enum FetchedModel {
    Ready(Box<Built>),
    /// Still queued or being built: try again.
    Pending,
    /// Never requested, failed to build, or already fetched.
    Missing,
}

#[derive(Default)]
struct Payloads {
    /// The model entry of each requested object's record, by id, as the
    /// poster's database holds it.
    records: HashMap<u32, String>,
    /// The prepared job of each requested character, by owner and scene.
    characters: HashMap<(u64, u64), ModelJob>,
    /// The pending body of each streamed character, by its stage node.
    streamed: HashMap<u64, PendingJob>,
    /// Streamed characters' bodies the pump handed over, in build order.
    streamed_ready: Vec<Built>,
    /// Built models by the node key the stream holds.
    built: HashMap<u64, Built>,
    next: u64,
}

impl Payloads {
    fn store(&mut self, built: Built) -> u64 {
        // Node 0 is the null node.
        self.next += 1;
        self.built.insert(self.next, built);
        self.next
    }
}

fn lock(payloads: &Mutex<Payloads>) -> MutexGuard<'_, Payloads> {
    payloads.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The stream thread's builders over the installation.
struct RequestBuilders {
    game: Arc<GameData>,
    payloads: Arc<Mutex<Payloads>>,
}

impl RequestBuilders {
    fn build(&self, job: ModelJob) -> Built {
        let started = Instant::now();
        let key = job.key();
        let model = job.build(None);
        Built {
            model,
            key,
            took: started.elapsed(),
        }
    }
}

impl ContentBuilders for RequestBuilders {
    /// The level's own list runs on the level loader's stream.
    fn level_object(&mut self, _: usize) -> Option<LevelNode> {
        None
    }

    fn object(&mut self, request: &ObjectRequest) -> Option<u64> {
        let entry = lock(&self.payloads).records.get(&request.id).cloned();
        // A record the database does not hold builds nothing.
        let entry = entry?;
        // A factory model is assembled on the main thread [Exarust's: its
        // atlas and stats live in the model cache]; posters never send one.
        if entry.starts_with('@') || request.flags & STREAMED == 0 {
            return None;
        }
        let pack = [PackKind::Objects, PackKind::Objlib]
            .into_iter()
            .find(|&pack| {
                self.game
                    .pack(pack)
                    .is_some_and(|archive| archive.get(&entry).is_some())
            })?;
        let job = match ModelJob::from_game(&self.game, pack, &entry) {
            Ok(job) => job,
            Err(message) => {
                warn!("content stream: object {:#x}: {message}", request.id);
                return None;
            }
        };
        let built = self.build(job);
        debug!(
            "content stream: object {:#x} ({entry}) built in {:?}",
            request.id, built.took
        );
        Some(lock(&self.payloads).store(built))
    }

    fn character(&mut self, request: &CharacterRequest) -> Option<u64> {
        let job = lock(&self.payloads)
            .characters
            .remove(&(request.owner, request.scene))?;
        let built = self.build(job);
        info!(
            "content stream: character {:#x} built on the stream thread in {:.1} ms (request word {:#x})",
            request.owner,
            built.took.as_secs_f64() * 1000.0,
            request.flags
        );
        Some(lock(&self.payloads).store(built))
    }

    /// A level character's body, nearest the focus first: the read, the
    /// parse, the morph and the meshes off the main thread. The original
    /// builds the whole character here; Exarust's body is its model, and
    /// the level's spawn makes the entities from it.
    fn streamed_character(&mut self, character: &StreamedCharacter) -> Option<u64> {
        let pending = lock(&self.payloads).streamed.remove(&character.node)?;
        let job = match pending.job(&self.game) {
            Ok(job) => job,
            Err(message) => {
                warn!(
                    "content stream: streamed character {}: {message}",
                    character.node
                );
                return None;
            }
        };
        let built = self.build(job);
        debug!(
            "content stream: streamed character {} ({}) built in {:?}",
            character.node, built.key, built.took
        );
        Some(lock(&self.payloads).store(built))
    }
}

/// The content stream of the running game and what its thread built.
#[derive(Resource, Clone)]
pub struct ContentRequests {
    worker: StreamWorker,
    payloads: Arc<Mutex<Payloads>>,
}

impl ContentRequests {
    pub fn new(game: Arc<GameData>) -> Self {
        let payloads = Arc::new(Mutex::new(Payloads::default()));
        Self {
            worker: StreamWorker::new(RequestBuilders {
                game,
                payloads: payloads.clone(),
            }),
            payloads,
        }
    }

    pub fn worker(&self) -> &StreamWorker {
        &self.worker
    }

    /// Posts an object request: record `id`'s model `entry`, its variant
    /// `flags` and the scene it is for.
    pub fn request_object(&self, id: u32, flags: u32, parent: u64, entry: &str) -> bool {
        lock(&self.payloads).records.insert(id, entry.to_string());
        self.worker
            .request_object(ObjectRequest { id, flags, parent })
    }

    /// Posts a character request with the model job its build takes.
    pub fn request_character(&self, request: CharacterRequest, job: ModelJob) -> bool {
        lock(&self.payloads)
            .characters
            .insert((request.owner, request.scene), job);
        self.worker.request_character(request)
    }

    /// The object fetch, with the built model.
    pub fn fetch_object(&self, id: u32, flags: u32, parent: u64) -> FetchedModel {
        match self.worker.fetch_object(id, flags, parent) {
            Fetch::Ready(node) => match lock(&self.payloads).built.remove(&node) {
                Some(built) => FetchedModel::Ready(Box::new(built)),
                None => FetchedModel::Missing,
            },
            Fetch::Pending => FetchedModel::Pending,
            Fetch::Missing => FetchedModel::Missing,
        }
    }

    /// The character fetch, with the built model: `None` while it is not
    /// built.
    pub fn fetch_character(&self, owner: u64, scene: u64) -> Option<Built> {
        let result = self.worker.fetch_character(owner, scene)?;
        lock(&self.payloads).built.remove(&result.node)
    }

    /// Hands a level's characters' bodies to the stream thread (the
    /// original's character-chunk handoff): they are built once the request
    /// rings are empty, nearest `focus` first (the player's start, the
    /// original's focus point). The bodies of a previous list not yet
    /// started are dropped.
    pub fn stream_characters(&self, focus: Vec3, bodies: Vec<StreamedBody>) {
        let mut characters = Vec::with_capacity(bodies.len());
        {
            let mut payloads = lock(&self.payloads);
            payloads.streamed.clear();
            for (index, body) in bodies.into_iter().enumerate() {
                // Stage node 0 is the null node.
                let node = index as u64 + 1;
                payloads.streamed.insert(node, body.job);
                characters.push(StreamedCharacter {
                    node,
                    scene: LEVEL_SCENE,
                    position: body.position.to_array(),
                });
            }
        }
        self.worker
            .begin_characters(LEVEL_SCENE, focus.to_array(), characters);
    }

    /// Ends the level's streamed list: the bodies not yet started are
    /// dropped (the level's spawn builds them itself).
    pub fn end_streamed_characters(&self) -> usize {
        let left = self.worker.pending_streamed();
        lock(&self.payloads).streamed.clear();
        self.worker
            .begin_characters(LEVEL_SCENE, [0.0; 3], std::iter::empty());
        left
    }

    /// The streamed characters' bodies the pump handed over since the last
    /// call, in build order.
    pub fn take_streamed(&self) -> Vec<Built> {
        std::mem::take(&mut lock(&self.payloads).streamed_ready)
    }

    /// The stream's main-thread pump: runs the completions and keeps the
    /// streamed characters' bodies it hands over for the level's spawn.
    pub fn pump(&self) {
        let pumped = self.worker.pump();
        if pumped.added.is_empty() {
            return;
        }
        let mut payloads = lock(&self.payloads);
        for result in pumped.added {
            if let Some(built) = payloads.built.remove(&result.node) {
                payloads.streamed_ready.push(built);
            }
        }
    }

    /// Whether a character request for `owner` and `scene` is still queued.
    pub fn character_queued(&self, owner: u64, scene: u64) -> bool {
        lock(&self.payloads)
            .characters
            .contains_key(&(owner, scene))
            || self.worker.queued().1 > 0
    }
}

/// The debug switch `--sync-content`: no content stream, so the requested
/// objects and characters are built on the main thread where they are
/// needed (the state before the stream thread, for timing comparisons).
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct ContentStreamOff;

/// Starts the content stream once the installation is open, and runs the
/// stream's main-thread pump every frame.
pub struct ContentRequestsPlugin;

impl Plugin for ContentRequestsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            (
                start_stream
                    .run_if(not(resource_exists::<ContentRequests>))
                    .run_if(not(resource_exists::<ContentStreamOff>)),
                pump,
            )
                .chain(),
        );
    }
}

fn start_stream(mut commands: Commands, game: Option<Res<Game>>) {
    if let Some(game) = game {
        info!("content stream: the stream thread runs on the async compute pool");
        commands.insert_resource(ContentRequests::new(game.0.clone()));
    }
}

/// The stream's pump: the completions of finished jobs run here and the
/// streamed characters' bodies are handed over. The requested objects and
/// characters wait for their fetches.
fn pump(requests: Option<Res<ContentRequests>>) {
    if let Some(requests) = requests {
        requests.pump();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The stream over the installed game (with `EXANIMA_DIR`).
    fn requests() -> Option<ContentRequests> {
        let dir = exa_assets::locate_install(None)?;
        let game = GameData::open(&dir).ok()?;
        Some(ContentRequests::new(Arc::new(game)))
    }

    fn settle(requests: &ContentRequests) {
        let started = Instant::now();
        while !requests.worker.is_idle() {
            assert!(started.elapsed() < Duration::from_secs(30), "never idle");
            std::thread::yield_now();
        }
    }

    #[test]
    fn a_record_the_poster_did_not_name_is_missing() {
        let Some(requests) = requests() else {
            return;
        };
        requests.worker.request_object(ObjectRequest {
            id: 7,
            flags: STREAMED,
            parent: ICON_SCENE,
        });
        settle(&requests);
        assert!(matches!(
            requests.fetch_object(7, STREAMED, ICON_SCENE),
            FetchedModel::Missing
        ));
        assert_eq!(requests.worker.missing_requested(), 1);
    }

    #[test]
    fn a_model_the_installation_lacks_or_an_unstreamed_request_builds_nothing() {
        let Some(requests) = requests() else {
            return;
        };
        requests.request_object(9, STREAMED, ICON_SCENE, "nothing.rfc");
        requests.request_object(10, 0, ICON_SCENE, "nothing.rfc");
        settle(&requests);
        assert!(matches!(
            requests.fetch_object(9, STREAMED, ICON_SCENE),
            FetchedModel::Missing
        ));
        assert_eq!(requests.worker.missing_requested(), 2);
    }

    /// The installed game's object library: a shipped item model is built
    /// on the stream thread and fetched by its key, once.
    #[test]
    fn a_shipped_object_model_is_built_on_the_stream_thread() {
        let Some(requests) = requests() else {
            return;
        };
        // The carried torch's model.
        let entry = "torcha02";
        requests.request_object(0x22, STREAMED, ICON_SCENE, entry);
        settle(&requests);
        match requests.fetch_object(0x22, STREAMED, ICON_SCENE) {
            FetchedModel::Ready(built) => {
                assert!(built.key.ends_with("torcha02"), "{}", built.key)
            }
            FetchedModel::Pending => panic!("still pending"),
            FetchedModel::Missing => panic!("not built"),
        }
        assert!(matches!(
            requests.fetch_object(0x22, STREAMED, ICON_SCENE),
            FetchedModel::Missing
        ));
    }
}
