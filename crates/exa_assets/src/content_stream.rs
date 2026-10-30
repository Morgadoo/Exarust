//! Background content streaming: the scheduler the original's stream thread
//! runs.
//!
//! The original keeps seven rings between the main thread and its stream
//! thread: generic jobs, their completions, object requests and results,
//! character requests and results, and characters to attach to an existing
//! scene node. It also keeps a list of streamed characters and a list of
//! placed level objects. One pass of the thread:
//!
//! 1. while the level's list is unfinished, its next placed object and
//!    nothing else;
//! 2. otherwise one generic job (its completion callback is queued for the
//!    main thread), then one requested database object, then one requested
//!    character, each when one is queued, all in the same pass;
//! 3. when none of those three ran, the pending streamed character nearest
//!    the focus point;
//! 4. when that did not run either, sleep until woken.
//!
//! The main-thread pump runs the completion callbacks, inserts the streamed
//! characters into the scene and attaches the characters requested for an
//! existing entity. Requested database objects and the other requested
//! characters wait in their result rings until the caller fetches them by
//! the key it asked with: a fetch always looks through the results, and when
//! nothing matches it answers with the null node while any request is still
//! queued, and with the scene's nil node otherwise.
//!
//! A placed level object may be the player: the session's arrival word names
//! the operative kind the player's object has, and a marker record names it
//! when its first word equals the arrival word. The player's position
//! becomes the focus point the streamed characters are ordered by. Exarust drives [`ContentStream::step`] from its own executor; the
//! original runs the same loop on its stream thread.

use std::collections::VecDeque;

use crate::stream::RingQueue;
use exa_formats::data_stream::DataStream;

/// The stream thread's two data streams, which the original's constructor
/// creates (two different stream classes). Which one reads what is a guess:
/// the names follow how the thread's loaders use them. Nothing uses the
/// type yet; the scheduler above runs on whichever executor drives it
/// (Bevy's task pool here).
#[derive(Debug, Default)]
pub struct StreamThread {
    chunk_stream: DataStream,
    parser: DataStream,
}

impl StreamThread {
    /// A thread with empty buffers, as the constructor leaves them.
    pub fn new() -> Self {
        Self::default()
    }

    /// The chunk stream.
    pub fn chunk_stream(&mut self) -> &mut DataStream {
        &mut self.chunk_stream
    }

    /// The record parser.
    pub fn parser(&mut self) -> &mut DataStream {
        &mut self.parser
    }
}

/// A completion callback, run on the main thread after its job.
pub type Completion = Box<dyn FnOnce() + Send>;

/// A named unit of work for the stream thread, with an optional callback the
/// main thread runs once it is done.
pub struct Job {
    pub work: Box<dyn FnOnce() + Send>,
    pub done: Option<Completion>,
}

/// One requested database object: the record id, its flags and the scene
/// node it belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectRequest {
    pub id: u32,
    pub flags: u32,
    pub parent: u64,
}

/// One requested character: the record that owns it (its class and flags)
/// and the scene it goes into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterRequest {
    pub owner: u64,
    pub scene: u64,
    pub class: u32,
    pub flags: u32,
}

/// Where a finished character goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterSink {
    /// A new character for the scene.
    Scene,
    /// A character attached to an existing entity (its request had flag
    /// `0x400`).
    Attach,
    /// A streamed character built in place, which the pump inserts.
    Streamed,
}

/// A finished database object with the request that asked for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectResult {
    pub request: ObjectRequest,
    pub node: u64,
}

/// A finished character, waiting for the main thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterResult {
    /// The key a requested character was fetched by (its record owner); a
    /// streamed character's stage node.
    pub owner: u64,
    pub scene: u64,
    /// The built scene node.
    pub node: u64,
    pub sink: CharacterSink,
    /// A streamed character's stage node, the stand-in the level placed:
    /// the pump frees it and registers `node` in its place. `None` for a
    /// character requested for the scene or attached to an entity.
    pub placeholder: Option<u64>,
}

/// A character the level streams in place: its scene node and position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StreamedCharacter {
    pub node: u64,
    pub scene: u64,
    pub position: [f32; 3],
}

/// What one loop pass did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Built (or skipped) the next placed level object.
    LevelObject { built: bool },
    /// Served the request rings: whether a generic job ran, and whether a
    /// requested database object and a requested character were taken
    /// (`Some`) and built (`Some(true)`).
    Requests {
        job: bool,
        object: Option<bool>,
        character: Option<bool>,
    },
    /// Built the nearest pending streamed character.
    Streamed { built: bool },
    /// Nothing to do; the original sleeps until woken.
    Idle,
}

/// A built placed level object and what the stream needs from it. The
/// original reads these off the node it just built, inside the object
/// builder; Exarust's builders report them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LevelNode {
    /// The built scene node.
    pub node: u64,
    /// The kind word the database builder left on the node, which the
    /// original reads before it decides what the object is:
    /// [`OBJECT_OPERATIVE`], [`OBJECT_DOOR`] or [`OBJECT_VOLUME`], else 0
    /// (`exa_world::db_objects::node_kind`).
    pub class: u32,
    /// For an object of the operative class (the actors), the kind its
    /// operative record carries: the placed player is the object whose kind
    /// is the one the arrival word names. `None` for every other class.
    pub operative_kind: Option<u32>,
    /// The node's position (centimetres, the original's world units), which
    /// becomes the focus point when the object is the player.
    pub position: [f32; 3],
    /// For a placed volume object whose marker record exists with type byte
    /// `0x40`: the two words the record stores, in file order,
    /// `[entrance, previous level]` (`exa_formats::objdb::Volume::marker`).
    /// The marker names the player when the entrance is the session's
    /// arrival variable and the previous level is the level the session
    /// came from.
    pub marker: Option<[u32; 2]>,
}

/// The node kind of an operative object's tree (and of the placeholder):
/// the level builder registers it with the dynamics system and reads the
/// operative kind back, which can name the player.
pub const OBJECT_OPERATIVE: u32 = 0x0001_0000;
/// The node kind of a door's tree, checked first: the level builder
/// registers it with the scene's grid environment.
pub const OBJECT_DOOR: u32 = 0x0004_0000;
/// The node kind of a model-less volume record's container volume (the
/// volume section's tag): the level builder runs the transition-marker test
/// on it.
pub const OBJECT_VOLUME: u32 = 0x8000_AC07;

impl LevelNode {
    /// A node with nothing the stream can read as the player.
    pub fn new(node: u64) -> Self {
        Self {
            node,
            class: 0,
            operative_kind: None,
            position: [0.0; 3],
            marker: None,
        }
    }
}

/// The arrival word a journey between levels leaves in the session
/// (`exa_ui::selection::ARRIVAL_WORD`): the player's operative kind is 15.
pub const ARRIVAL_WORD: u32 = 0xEEEE_00DA;
/// The arrival word a game started from the menu or loaded from a save
/// leaves in the session: the player's operative kind is 18.
pub const SESSION_START_WORD: u32 = 0xEEEE_00DB;
/// The session's arrival word when nothing set one.
pub const NO_ARRIVAL_WORD: u32 = u32::MAX;
/// The previous level id a session that did not come from a level has (the
/// original resets the variable to all ones); a marker's own word must be
/// this too to name the player on a fresh start.
pub const NO_PREVIOUS_LEVEL: u32 = u32::MAX;
/// The player's operative kind under [`ARRIVAL_WORD`].
pub const ARRIVAL_PLAYER_KIND: u32 = 15;
/// The player's operative kind under [`SESSION_START_WORD`].
pub const SESSION_START_PLAYER_KIND: u32 = 18;

/// The flags word bits the level object builder copies from the placed
/// object's own flags word onto its root node (the rest of the node's word
/// stays): the static bit `0x80000` and bits `0x100000`, `0x200000` and
/// `0x800000`, whose meanings are not traced. The original clears exactly
/// these bits first (`node & 0xFF47FFFF`).
pub const OBJECT_NODE_FLAGS: u32 = 0x00B8_0000;

/// The placement flag that makes the root node level geometry
/// (`PlacedObject::STATIC`): the builder adds the node's own static bit.
pub const PLACEMENT_STATIC: u32 = 0x10;

/// The root node's static bit.
pub const NODE_STATIC: u32 = 0x0008_0000;

/// The root node word a placed object gives its model: the object's own
/// flags under [`OBJECT_NODE_FLAGS`] replace that field of the node's word,
/// and the placement's static bit adds the node's static flag. The original
/// copies no other bit.
pub fn root_node_flags(node: u32, object: u32, placement: u32) -> u32 {
    let mut word = (node & !OBJECT_NODE_FLAGS) | (object & OBJECT_NODE_FLAGS);
    if placement & PLACEMENT_STATIC != 0 {
        word |= NODE_STATIC;
    }
    word
}

/// The tags of the saved sub-chunks a streamed character carries: the flat
/// pose, the motion state, the pose tree and the AI home. The character
/// builder copies the whole trailing chunk list onto the stage node, then
/// scans it for these four; other tags stay in the copied bytes unread.
pub const SAVED_FLAT_POSE: u32 = 0xCAC0_E100;
pub const SAVED_MOTION: u32 = 0xCAC0_EA00;
pub const SAVED_POSE_TREE: u32 = 0xCAC0_EB00;
pub const SAVED_AI_HOME: u32 = 0xCACE_0F77;

/// The saved chunks of one streamed character, by tag. A tag that appears
/// more than once keeps its last occurrence, as the builder overwrites its
/// pointer each time it matches.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SavedChunks<'a> {
    pub flat_pose: Option<&'a [u8]>,
    pub motion: Option<&'a [u8]>,
    pub pose_tree: Option<&'a [u8]>,
    pub ai_home: Option<&'a [u8]>,
}

/// Reads a streamed character's saved chunk blob: a 32-bit length of the
/// bytes that follow it, then `{u32 tag, u32 size, bytes}` records. The
/// builder's scan starts at byte 4 and runs while the next record's header
/// lies within the stored length; a record whose declared size runs past the
/// slice ends the scan, and a size that would not move the offset ends it
/// too, so malformed input reads nothing more rather than looping.
pub fn saved_chunks(blob: &[u8]) -> SavedChunks<'_> {
    let mut chunks = SavedChunks::default();
    if blob.len() < 4 {
        return chunks;
    }
    let declared = u32::from_le_bytes([blob[0], blob[1], blob[2], blob[3]]) as usize;
    // The length counts the bytes after its own four, so the record area
    // ends at 4 + length (the builder's loop bound, made safe).
    let limit = declared.saturating_add(4).min(blob.len());
    let mut at = 4usize;
    while at + 8 <= limit {
        let tag = u32::from_le_bytes([blob[at], blob[at + 1], blob[at + 2], blob[at + 3]]);
        let size =
            u32::from_le_bytes([blob[at + 4], blob[at + 5], blob[at + 6], blob[at + 7]]) as usize;
        let start = at + 8;
        let Some(end) = start.checked_add(size).filter(|end| *end <= blob.len()) else {
            break;
        };
        let body = &blob[start..end];
        match tag {
            SAVED_FLAT_POSE => chunks.flat_pose = Some(body),
            SAVED_MOTION => chunks.motion = Some(body),
            SAVED_POSE_TREE => chunks.pose_tree = Some(body),
            SAVED_AI_HOME => chunks.ai_home = Some(body),
            _ => {}
        }
        if end <= at {
            break;
        }
        at = end;
    }
    chunks
}

// The level's character-section handoff (count, the optional local
// character database chunk, then one placement chunk per character; a
// missing record, or a world-database character of another locale, builds
// nothing and leaves no slot) is read by
// `exa_formats::characters::LevelCharacters::parse` and filtered by
// `exa_world::placed_characters`; the stream does not read it again.

/// The constant the character builders multiply a stored 16-bit size word
/// by: `1 / 65535` as a double, so a full word `0xFFFF` is exactly one.
pub const SIZE_WORD_SCALE: f64 = 1.0 / 65535.0;

/// The two size words a streamed character's record holds, defaulted as its
/// builder does: when the second word is zero both become `0xFFFF`, which
/// [`SIZE_WORD_SCALE`] scales to one, so an unset pair is the body's own
/// size. Only the streamed builder defaults, and only for a body that is not
/// dead (see [`streamed_size_words`]); the request builder scales the words
/// as stored.
pub fn normalised_size_words(words: [u16; 2]) -> [u16; 2] {
    if words[1] == 0 { [u16::MAX; 2] } else { words }
}

/// The size words the streamed character builder scales: a dead body
/// (condition [`exa_formats::characters::DEAD`]) goes to the corpse builder
/// with its words as stored, any other body takes
/// [`normalised_size_words`] first.
pub fn streamed_size_words(words: [u16; 2], condition: i32) -> [u16; 2] {
    if condition == exa_formats::characters::DEAD {
        words
    } else {
        normalised_size_words(words)
    }
}

/// One of the character's two stored size words, scaled by the body's base
/// scale for its scale field: the stored word widened to double, times
/// [`SIZE_WORD_SCALE`], times the base scale, narrowed to single precision
/// in that order. The builder writes the second word first.
///
/// The two words the builder reads from the character definition are the
/// values that size the body; the record layout's field names are not
/// traced here. **[guess: the word pair is the body's two size words]**
pub fn stored_size_scale(base: f32, stored: u16) -> f32 {
    (f64::from(base) * (f64::from(stored) * SIZE_WORD_SCALE)) as f32
}

/// The character's two scale fields in the order the builder writes them:
/// the second word's scale first, then the first word's. The words are
/// scaled as given: the streamed builder passes them through
/// [`streamed_size_words`] first, the request builder does not.
pub fn character_scales(base: f32, words: [u16; 2]) -> (f32, f32) {
    let [first, second] = words;
    (
        stored_size_scale(base, second),
        stored_size_scale(base, first),
    )
}

/// The stream request flags the character builder reads: bit `0x2000` marks
/// the built character's node with its own bit `0x1000`; bit `0x1000` skips
/// both the AI start and the saved AI pose restore.
pub const MARK_NODE: u32 = 0x2000;
pub const NO_AI_START: u32 = 0x1000;
/// The node flag [`MARK_NODE`] sets.
pub const NODE_MARK: u32 = 0x1000;

/// The character node word for a request: the request's [`MARK_NODE`] adds
/// [`NODE_MARK`] to the node's own word.
pub fn character_node_flags(node: u32, request: u32) -> u32 {
    if request & MARK_NODE != 0 {
        node | NODE_MARK
    } else {
        node
    }
}

/// A character's saved AI state, which the builders restore into the
/// controller right after starting its AI: the request builder from the
/// request's saved state unless the request sets [`NO_AI_START`], the
/// streamed builder from the saved AI home chunk. A position and a facing. The fourth word is copied into the controller's next field;
/// it is read as a facing angle. **[guess: the fourth word is a heading]**
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SavedAiPose {
    pub position: [f32; 3],
    pub heading: f32,
}

/// Reads the 16 bytes of a saved AI state (a request's, or the saved AI home
/// chunk's): the position as three floats, then a fourth word. Any shorter input has no
/// pose, and the builder starts the AI from its own state instead.
pub fn saved_ai_pose(bytes: &[u8]) -> Option<SavedAiPose> {
    let bytes: &[u8; 16] = bytes.get(..16)?.try_into().ok()?;
    let float = |at: usize| f32::from_le_bytes(bytes[at..at + 4].try_into().expect("4 bytes"));
    Some(SavedAiPose {
        position: [float(0), float(4), float(8)],
        heading: float(12),
    })
}

/// The builders the scheduler calls. The original's stream thread calls into
/// the database, character and item factories; Exarust supplies closures
/// over its own loaders.
pub trait ContentBuilders {
    /// Builds placed level object `index` of the current level.
    fn level_object(&mut self, index: usize) -> Option<LevelNode>;
    /// Builds one requested database object.
    fn object(&mut self, request: &ObjectRequest) -> Option<u64>;
    /// Builds one requested character and returns its scene node.
    fn character(&mut self, request: &CharacterRequest) -> Option<u64>;
    /// Builds one streamed character in place.
    fn streamed_character(&mut self, character: &StreamedCharacter) -> Option<u64>;
}

/// What the main-thread pump drained.
#[derive(Debug, Default)]
pub struct Pumped {
    /// Completion callbacks run.
    pub completions: usize,
    /// Streamed characters to insert into the scene, then characters to
    /// attach to existing entities, each in build order.
    pub added: Vec<CharacterResult>,
    pub attached: Vec<CharacterResult>,
}

/// A level's list of placed objects being streamed.
#[derive(Debug, Default)]
struct LevelObjects {
    scene: u64,
    total: usize,
    /// Reader entries consumed: the ordinal of the next object to read.
    done: usize,
    built: Vec<u64>,
    missing: usize,
    /// The arrival word the level began with.
    arrival: u32,
    /// The id of the level the session came from.
    previous_level: u32,
    /// The operative kind of the player's placed object, from the arrival
    /// word.
    player_kind: Option<u32>,
    /// The player's node, once built.
    player: Option<u64>,
}

/// The finished level object list.
#[derive(Debug, Default)]
pub struct LevelStream {
    /// The scene the level's objects belong to.
    pub scene: u64,
    pub objects: Vec<u64>,
    /// Objects whose database record or model was not found.
    pub missing: usize,
}

/// The source the level wait's report is logged under, at the original's
/// message level 5.
pub const LOG_SOURCE: &str = "Content management";

/// The report the level wait logs when placed objects were missing, as the
/// original formats it (the count in decimal, then the text); `None` for a
/// count of zero, which logs nothing.
pub fn missing_message(missing: usize) -> Option<String> {
    (missing > 0).then(|| format!("{missing} missing database objects."))
}

/// The scene's room-mode bits: with either set, a scene that has a grid
/// environment sorts the level's placed objects into its rooms.
pub const ROOM_MODE_BITS: u32 = 0x3;

/// The room-mode word every shipped scene is created with (both bits set).
pub const SCENE_ROOM_WORD: u32 = 0x73;

/// Where the level wait hands the finished objects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelHandoff {
    /// To the scene's grid environment, which sorts them into its rooms;
    /// the caller's list is left as it is.
    Rooms,
    /// Appended to the caller's list, after what it already holds.
    Append,
}

impl LevelHandoff {
    /// The level wait's choice: the grid environment when the scene has one
    /// and its room-mode word has either bit of [`ROOM_MODE_BITS`], the
    /// caller's list otherwise.
    pub fn of(has_grid_environment: bool, room_word: u32) -> Self {
        if has_grid_environment && room_word & ROOM_MODE_BITS != 0 {
            Self::Rooms
        } else {
            Self::Append
        }
    }
}

/// What fetching a requested database object found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fetch {
    /// The object was built; its scene node.
    Ready(u64),
    /// No result matched, but object requests are still queued, so the one
    /// asked for may yet be built: try again later. The original returns its
    /// null node here.
    Pending,
    /// No result matched and no request is queued: the object was never
    /// requested, failed to build or was already fetched. The original
    /// returns the scene's nil node.
    Missing,
}

/// The original's stream-thread scheduler.
pub struct ContentStream {
    jobs: RingQueue<Job>,
    completions: RingQueue<Completion>,
    object_requests: RingQueue<ObjectRequest>,
    object_results: RingQueue<ObjectResult>,
    character_requests: RingQueue<CharacterRequest>,
    character_results: RingQueue<CharacterResult>,
    attachments: RingQueue<CharacterResult>,
    /// Streamed characters built in place, waiting for the pump.
    streamed_results: RingQueue<CharacterResult>,
    /// The placed characters of the current level, front = already built.
    streamed: VecDeque<StreamedCharacter>,
    stream_cursor: usize,
    scene: u64,
    focus: [f32; 3],
    level: LevelObjects,
    terminated: bool,
}

impl Default for ContentStream {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentStream {
    /// Creates the queues with the original's capacities: 16 jobs,
    /// completions and attachments, 32 object and character requests and
    /// results, 128 streamed results.
    pub fn new() -> Self {
        Self {
            jobs: RingQueue::new(16, 1),
            completions: RingQueue::new(16, 1),
            object_requests: RingQueue::new(32, 1),
            object_results: RingQueue::new(32, 1),
            character_requests: RingQueue::new(32, 1),
            character_results: RingQueue::new(32, 1),
            attachments: RingQueue::new(16, 1),
            streamed_results: RingQueue::new(128, 1),
            streamed: VecDeque::new(),
            stream_cursor: 0,
            scene: 0,
            focus: [0.0; 3],
            level: LevelObjects::default(),
            terminated: false,
        }
    }

    /// Queues a generic job; its completion runs on the next main-thread
    /// pump.
    pub fn submit_job(&mut self, job: Job) -> bool {
        self.jobs.push(job);
        true
    }

    pub fn request_object(&mut self, request: ObjectRequest) -> bool {
        self.object_requests.push(request);
        true
    }

    pub fn request_character(&mut self, request: CharacterRequest) -> bool {
        self.character_requests.push(request);
        true
    }

    /// Drops a queued object request (the original has no cancel; Exarust
    /// must not keep a request alive when its scene goes away).
    pub fn cancel_object(&mut self, id: u32, flags: u32, parent: u64) -> bool {
        self.object_requests
            .remove(|request| {
                request.id == id && request.flags == flags && request.parent == parent
            })
            .is_some()
    }

    /// Drops a queued character request.
    pub fn cancel_character(&mut self, owner: u64, scene: u64) -> bool {
        self.character_requests
            .remove(|request| request.owner == owner && request.scene == scene)
            .is_some()
    }

    /// Begins a level's object list: the scene, how many objects the chunk
    /// holds, the session's arrival word (the entrance the player takes,
    /// [`NO_ARRIVAL_WORD`] when nothing set one) and the id of the level
    /// the session came from ([`NO_PREVIOUS_LEVEL`] for a fresh start), as
    /// the original's two globals hold them. The arrival word names the
    /// player's operative kind; any other word names no kind, so only a
    /// marker record can then name the player.
    pub fn begin_level(&mut self, scene: u64, total: usize, arrival: u32, previous_level: u32) {
        let player_kind = match arrival {
            ARRIVAL_WORD => Some(ARRIVAL_PLAYER_KIND),
            SESSION_START_WORD => Some(SESSION_START_PLAYER_KIND),
            _ => None,
        };
        self.level = LevelObjects {
            scene,
            total,
            arrival,
            previous_level,
            player_kind,
            ..LevelObjects::default()
        };
        self.scene = scene;
    }

    /// Begins a level's streamed characters: the scene and the characters
    /// with their positions. The original's reader leaves out the entries it
    /// cannot turn into a placeholder (a missing character record, or a
    /// conditional entry whose record does not match the session), so the
    /// list holds only what will be built. The focus point is the scene's, set by
    /// [`ContentStream::set_focus`] or by the placed player ([`ContentStream::begin_level`]).
    pub fn begin_characters(
        &mut self,
        scene: u64,
        characters: impl IntoIterator<Item = StreamedCharacter>,
    ) {
        self.scene = scene;
        self.streamed = characters.into_iter().collect();
        self.stream_cursor = 0;
    }

    /// Moves the focus point the nearest-first stream orders by.
    pub fn set_focus(&mut self, focus: [f32; 3]) {
        self.focus = focus;
    }

    pub fn scene(&self) -> u64 {
        self.scene
    }

    /// The focus point the nearest-first stream orders by: the placed
    /// player's position once it is built, else what the caller set.
    pub fn focus(&self) -> [f32; 3] {
        self.focus
    }

    /// The placed player's scene node, once the level's list built it.
    pub fn player(&self) -> Option<u64> {
        self.level.player
    }

    pub fn is_terminated(&self) -> bool {
        self.terminated
    }

    /// Stops the loop after the current pass, as the thread's terminate flag
    /// does.
    pub fn terminate(&mut self) {
        self.terminated = true;
    }

    pub fn queued_jobs(&self) -> usize {
        self.jobs.len()
    }

    pub fn queued_objects(&self) -> usize {
        self.object_requests.len()
    }

    pub fn queued_characters(&self) -> usize {
        self.character_requests.len()
    }

    pub fn pending_streamed(&self) -> usize {
        self.streamed.len().saturating_sub(self.stream_cursor)
    }

    /// One pass of the stream thread: the next level object alone while the
    /// level's list is unfinished; otherwise a generic job, a database
    /// object and a character, one of each that is queued, and only when
    /// none was the nearest streamed character; else idle.
    pub fn step(&mut self, builders: &mut dyn ContentBuilders) -> Step {
        if self.terminated {
            return Step::Idle;
        }
        if self.level.done < self.level.total {
            // The entry being read; a build that fails leaves no slot for
            // it (the original's missing count rises and the list closes
            // up).
            let index = self.level.done;
            let built = builders.level_object(index);
            self.level.done += 1;
            match built {
                Some(object) => {
                    self.level.built.push(object.node);
                    self.adopt(&object);
                }
                None => self.level.missing += 1,
            }
            return Step::LevelObject {
                built: built.is_some(),
            };
        }
        let mut job = false;
        if let Some(next) = self.jobs.pop() {
            (next.work)();
            if let Some(done) = next.done {
                self.completions.push(done);
            }
            job = true;
        }
        // A request that builds nothing is dropped.
        let object = self.object_requests.pop().map(|request| {
            let built = builders.object(&request);
            if let Some(node) = built {
                self.object_results.push(ObjectResult { request, node });
            }
            built.is_some()
        });
        let character = self.character_requests.pop().map(|request| {
            let built = builders.character(&request);
            if let Some(node) = built {
                let sink = if request.flags & 0x400 != 0 {
                    CharacterSink::Attach
                } else {
                    CharacterSink::Scene
                };
                let result = CharacterResult {
                    owner: request.owner,
                    scene: request.scene,
                    node,
                    sink,
                    placeholder: None,
                };
                match sink {
                    CharacterSink::Attach => self.attachments.push(result),
                    CharacterSink::Scene | CharacterSink::Streamed => {
                        self.character_results.push(result)
                    }
                }
            }
            built.is_some()
        });
        if job || object.is_some() || character.is_some() {
            return Step::Requests {
                job,
                object,
                character,
            };
        }
        if self.stream_cursor < self.streamed.len() {
            let mut best = self.stream_cursor;
            let mut best_distance =
                distance(self.streamed[self.stream_cursor].position, self.focus);
            for index in self.stream_cursor + 1..self.streamed.len() {
                let candidate = distance(self.streamed[index].position, self.focus);
                // Strictly nearer: the first of equal distances wins.
                if candidate < best_distance {
                    best = index;
                    best_distance = candidate;
                }
            }
            self.streamed.swap(self.stream_cursor, best);
            let character = self.streamed[self.stream_cursor];
            let built = builders.streamed_character(&character);
            if let Some(node) = built {
                self.streamed_results.push(CharacterResult {
                    owner: character.node,
                    scene: character.scene,
                    node,
                    sink: CharacterSink::Streamed,
                    placeholder: Some(character.node),
                });
            }
            self.stream_cursor += 1;
            return Step::Streamed {
                built: built.is_some(),
            };
        }
        Step::Idle
    }

    /// Adopts a built placed object as the player when it is an operative
    /// object whose kind is the arrival word's, or a volume object whose
    /// marker record matches the arrival word and the previous level: its
    /// position becomes the focus point the streamed characters are ordered
    /// by. A later match replaces an earlier one, as the original's globals
    /// are overwritten.
    fn adopt(&mut self, object: &LevelNode) {
        let by_kind = object.class == OBJECT_OPERATIVE
            && object.operative_kind.is_some()
            && self.level.player_kind == object.operative_kind;
        // The marker's volume section stores the entrance and the previous
        // level's id; both are compared with the session's two variables.
        let by_marker = object.class == OBJECT_VOLUME
            && object.marker == Some([self.level.arrival, self.level.previous_level]);
        let player = by_kind || by_marker;
        if player {
            self.level.player = Some(object.node);
            self.focus = object.position;
        }
    }

    /// Runs the completion callbacks, then collects the streamed characters
    /// to insert into the scene and the characters to attach (the
    /// original's pump, in its order). A streamed character's entry names
    /// the stage node it was built into and the built node: the caller
    /// frees the stage (it holds the object's transform and saved chunks)
    /// and registers the built character in the scene and its sector. An
    /// attach entry is the built character, registered into the entity it
    /// was requested for. Requested characters without the attach flag stay
    /// for [`ContentStream::fetch_character`].
    pub fn pump(&mut self) -> Pumped {
        let mut pumped = Pumped::default();
        while let Some(completion) = self.completions.pop() {
            completion();
            pumped.completions += 1;
        }
        while let Some(result) = self.streamed_results.pop() {
            pumped.added.push(result);
        }
        while let Some(result) = self.attachments.pop() {
            pumped.attached.push(result);
        }
        pumped
    }

    /// Fetches a finished database object by the key it was requested with:
    /// the original's object fetch. The results are searched whatever is
    /// queued, and a found one leaves the order of the rest; when none
    /// matches, the original answers with its null node while any object
    /// request is queued and with the scene's nil node otherwise, which
    /// [`Fetch`] keeps apart.
    pub fn fetch_object(&mut self, id: u32, flags: u32, parent: u64) -> Fetch {
        match self.object_results.remove(|result| {
            result.request.id == id
                && result.request.flags == flags
                && result.request.parent == parent
        }) {
            Some(result) => Fetch::Ready(result.node),
            None if self.object_requests.is_empty() => Fetch::Missing,
            None => Fetch::Pending,
        }
    }

    /// Fetches a finished character by its owner and scene (the original's
    /// character fetch), removing it from the result ring.
    pub fn fetch_character(&mut self, owner: u64, scene: u64) -> Option<CharacterResult> {
        self.character_results
            .remove(|result| result.owner == owner && result.scene == scene)
    }

    /// Waits for the level's object list: runs the scheduler until every
    /// entry is done, then returns the built nodes and the missing count, or
    /// `None` when the level had no list at all (the original's return
    /// value). The original reports "N missing database objects." and clears
    /// the count; the caller logs it from [`LevelStream::missing`].
    pub fn wait_for_level(
        &mut self,
        builders: &mut dyn ContentBuilders,
        mut idle: impl FnMut(),
    ) -> Option<LevelStream> {
        if self.level.total == 0 {
            return None;
        }
        while self.level.done < self.level.total && !self.terminated {
            self.step(builders);
            // The original steps its procedural materials and yields between
            // objects.
            idle();
        }
        let stream = LevelStream {
            scene: self.level.scene,
            objects: std::mem::take(&mut self.level.built),
            missing: self.level.missing,
        };
        self.level.total = 0;
        self.level.done = 0;
        self.level.missing = 0;
        Some(stream)
    }

    /// Waits for the character requests to finish: each round notes whether
    /// the request ring is empty, drains the attachment ring, and stops if
    /// it was; otherwise the caller pumps other work (the original drains
    /// the texture uploads and the procedural material step) and the
    /// scheduler runs a pass.
    pub fn wait_for_characters(
        &mut self,
        builders: &mut dyn ContentBuilders,
        mut idle: impl FnMut(),
    ) -> Vec<CharacterResult> {
        let mut finished = Vec::new();
        loop {
            let empty = self.character_requests.is_empty();
            while let Some(result) = self.attachments.pop() {
                finished.push(result);
            }
            if empty || self.terminated {
                break;
            }
            idle();
            self.step(builders);
        }
        finished
    }

    /// Empties every queue and the streamed list (the original's reset
    /// between levels).
    pub fn reset(&mut self) {
        self.jobs.reset();
        self.completions.reset();
        self.object_requests.reset();
        self.object_results.reset();
        self.character_requests.reset();
        self.character_results.reset();
        self.attachments.reset();
        self.streamed_results.reset();
        self.streamed.clear();
        self.stream_cursor = 0;
        self.level = LevelObjects::default();
        self.scene = 0;
    }
}

/// Squared distance between two points, summed x, y, then z in single
/// precision, as the original's helper does.
fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    dx * dx + dy * dy + dz * dz
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// A builder that records what it was asked for and answers with the
    /// encoded request.
    struct Recorder {
        calls: Arc<Mutex<Vec<String>>>,
        level: Vec<Option<u64>>,
        objects: Vec<Option<u64>>,
        characters: Vec<Option<u64>>,
        streamed_ok: bool,
        /// A placed actor: its index, operative kind and position.
        player: Option<(usize, u32, [f32; 3])>,
        /// A placed object with a marker record: index, the record's words
        /// and the position.
        marker: Option<(usize, [u32; 2], [f32; 3])>,
        /// The class word the placed actor's node carries; the operative
        /// class unless a test wants another.
        player_class: u32,
        /// The class word the marker object's node carries; the volume class
        /// unless a test wants another.
        marker_class: u32,
    }

    impl Default for Recorder {
        fn default() -> Self {
            Self {
                calls: Arc::default(),
                level: Vec::new(),
                objects: Vec::new(),
                characters: Vec::new(),
                streamed_ok: false,
                player: None,
                marker: None,
                player_class: OBJECT_OPERATIVE,
                marker_class: OBJECT_VOLUME,
            }
        }
    }

    impl ContentBuilders for Recorder {
        fn level_object(&mut self, index: usize) -> Option<LevelNode> {
            self.calls.lock().unwrap().push(format!("level {index}"));
            let node = self.level.get(index).copied().flatten()?;
            let mut object = LevelNode::new(node);
            if let Some((at, kind, position)) = self.player
                && at == index
            {
                object.class = self.player_class;
                object.operative_kind = Some(kind);
                object.position = position;
            }
            if let Some((at, words, position)) = self.marker
                && at == index
            {
                object.class = self.marker_class;
                object.marker = Some(words);
                object.position = position;
            }
            Some(object)
        }

        fn object(&mut self, request: &ObjectRequest) -> Option<u64> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("object {}", request.id));
            self.objects.pop().flatten()
        }

        fn character(&mut self, request: &CharacterRequest) -> Option<u64> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("character {}", request.owner));
            self.characters.pop().flatten()
        }

        fn streamed_character(&mut self, character: &StreamedCharacter) -> Option<u64> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("streamed {}", character.node));
            self.streamed_ok.then_some(character.node)
        }
    }

    fn character(node: u64, position: [f32; 3]) -> StreamedCharacter {
        StreamedCharacter {
            node,
            scene: 7,
            position,
        }
    }

    #[test]
    fn the_loop_takes_level_objects_jobs_objects_characters_then_the_nearest() {
        let mut stream = ContentStream::new();
        let calls = Arc::new(Mutex::new(Vec::new()));
        let mut builders = Recorder {
            calls: calls.clone(),
            level: vec![Some(10), None],
            objects: vec![Some(20)],
            characters: vec![Some(30)],
            streamed_ok: true,
            ..Recorder::default()
        };
        stream.begin_level(7, 2, 0, NO_PREVIOUS_LEVEL);
        stream.begin_characters(7, []);
        stream.submit_job(Job {
            work: Box::new(|| {}),
            done: None,
        });
        stream.request_object(ObjectRequest {
            id: 5,
            flags: 0,
            parent: 7,
        });
        stream.request_character(CharacterRequest {
            owner: 9,
            scene: 7,
            class: 1,
            flags: 0,
        });
        assert_eq!(
            stream.step(&mut builders),
            Step::LevelObject { built: true }
        );
        assert_eq!(
            stream.step(&mut builders),
            Step::LevelObject { built: false }
        );
        // The job, the object and the character share one pass.
        assert_eq!(
            stream.step(&mut builders),
            Step::Requests {
                job: true,
                object: Some(true),
                character: Some(true),
            }
        );
        assert_eq!(stream.step(&mut builders), Step::Idle);
        assert_eq!(
            *calls.lock().unwrap(),
            ["level 0", "level 1", "object 5", "character 9"]
        );
        assert_eq!(stream.fetch_object(5, 0, 7), Fetch::Ready(20));
        assert_eq!(stream.fetch_object(5, 0, 7), Fetch::Missing);
        // The pump leaves a requested character for its fetch.
        let pumped = stream.pump();
        assert!(pumped.added.is_empty());
        assert_eq!(stream.fetch_character(9, 7).unwrap().node, 30);
    }

    #[test]
    fn a_streamed_character_waits_for_a_pass_without_requests() {
        let mut stream = ContentStream::new();
        let calls = Arc::new(Mutex::new(Vec::new()));
        let mut builders = Recorder {
            calls: calls.clone(),
            objects: vec![None],
            streamed_ok: true,
            ..Recorder::default()
        };
        stream.begin_characters(7, [character(3, [1.0, 0.0, 0.0])]);
        stream.request_object(ObjectRequest {
            id: 5,
            flags: 0,
            parent: 7,
        });
        // A request that builds nothing still takes the pass, and is dropped.
        assert_eq!(
            stream.step(&mut builders),
            Step::Requests {
                job: false,
                object: Some(false),
                character: None,
            }
        );
        assert_eq!(stream.fetch_object(5, 0, 7), Fetch::Missing);
        assert_eq!(stream.step(&mut builders), Step::Streamed { built: true });
        assert_eq!(*calls.lock().unwrap(), ["object 5", "streamed 3"]);
        // The pump inserts it into the scene.
        let pumped = stream.pump();
        assert_eq!(pumped.added.len(), 1);
        assert_eq!(pumped.added[0].node, 3);
        assert_eq!(pumped.added[0].sink, CharacterSink::Streamed);
    }

    /// The pump's streamed entry names the stage node the level placed and
    /// the built character: the caller frees the one and registers the
    /// other. A requested character has no stage node.
    #[test]
    fn a_streamed_result_names_its_stage_and_built_nodes() {
        struct Staged;
        impl ContentBuilders for Staged {
            fn level_object(&mut self, _: usize) -> Option<LevelNode> {
                None
            }
            fn object(&mut self, _: &ObjectRequest) -> Option<u64> {
                None
            }
            fn character(&mut self, _: &CharacterRequest) -> Option<u64> {
                Some(40)
            }
            fn streamed_character(&mut self, _: &StreamedCharacter) -> Option<u64> {
                Some(99)
            }
        }
        let mut stream = ContentStream::new();
        stream.begin_characters(7, [character(3, [0.0; 3])]);
        assert_eq!(stream.step(&mut Staged), Step::Streamed { built: true });
        let pumped = stream.pump();
        assert_eq!(pumped.added.len(), 1);
        let result = pumped.added[0];
        assert_eq!(result.owner, 3);
        assert_eq!(result.placeholder, Some(3));
        assert_eq!(result.node, 99);
        assert_eq!(result.sink, CharacterSink::Streamed);
        stream.request_character(CharacterRequest {
            owner: 9,
            scene: 7,
            class: 0,
            flags: 0,
        });
        assert!(matches!(
            stream.step(&mut Staged),
            Step::Requests {
                character: Some(true),
                ..
            }
        ));
        let requested = stream.fetch_character(9, 7).unwrap();
        assert_eq!(requested.node, 40);
        assert_eq!(requested.placeholder, None);
    }

    #[test]
    fn an_unpumped_character_can_be_fetched_by_owner_and_scene() {
        let mut stream = ContentStream::new();
        let mut builders = Recorder {
            characters: vec![Some(31)],
            ..Recorder::default()
        };
        stream.request_character(CharacterRequest {
            owner: 9,
            scene: 7,
            class: 1,
            flags: 0,
        });
        assert_eq!(
            stream.step(&mut builders),
            Step::Requests {
                job: false,
                object: None,
                character: Some(true),
            }
        );
        assert_eq!(
            stream.fetch_character(9, 7).unwrap().sink,
            CharacterSink::Scene
        );
        assert!(stream.fetch_character(9, 7).is_none());
    }

    #[test]
    fn streamed_characters_come_nearest_first_and_keep_ties_in_list_order() {
        let mut stream = ContentStream::new();
        let calls = Arc::new(Mutex::new(Vec::new()));
        let mut builders = Recorder {
            calls: calls.clone(),
            streamed_ok: true,
            ..Recorder::default()
        };
        stream.set_focus([0.0, 0.0, 0.0]);
        stream.begin_characters(
            7,
            [
                character(5, [5.0, 0.0, 0.0]),
                character(1, [1.0, 0.0, 0.0]),
                character(3, [3.0, 0.0, 0.0]),
                character(9, [-1.0, 0.0, 0.0]),
            ],
        );
        assert_eq!(stream.pending_streamed(), 4);
        while stream.step(&mut builders) != Step::Idle {}
        // 1 and 9 are equally near: the earlier list entry wins.
        assert_eq!(
            *calls.lock().unwrap(),
            ["streamed 1", "streamed 9", "streamed 3", "streamed 5"]
        );
        assert_eq!(stream.pending_streamed(), 0);
    }

    #[test]
    fn job_completions_run_on_the_pump_and_attachments_go_to_attach() {
        let mut stream = ContentStream::new();
        let log = Arc::new(Mutex::new(Vec::new()));
        let job_log = log.clone();
        let done_log = log.clone();
        let read_log = log.clone();
        stream.submit_job(Job {
            work: Box::new(move || job_log.lock().unwrap().push("work")),
            done: Some(Box::new(move || done_log.lock().unwrap().push("done"))),
        });
        let mut builders = Recorder::default();
        assert_eq!(
            stream.step(&mut builders),
            Step::Requests {
                job: true,
                object: None,
                character: None,
            }
        );
        // The callback waits for the main thread.
        assert_eq!(*read_log.lock().unwrap(), ["work"]);
        assert_eq!(stream.pump().completions, 1);
        assert_eq!(*read_log.lock().unwrap(), ["work", "done"]);

        stream.request_character(CharacterRequest {
            owner: 4,
            scene: 7,
            class: 2,
            flags: 0x400,
        });
        builders.characters = vec![Some(40)];
        assert_eq!(
            stream.step(&mut builders),
            Step::Requests {
                job: false,
                object: None,
                character: Some(true),
            }
        );
        let pumped = stream.pump();
        assert_eq!(pumped.added.len(), 0);
        assert_eq!(pumped.attached.len(), 1);
        assert_eq!(pumped.attached[0].node, 40);
        assert_eq!(pumped.attached[0].sink, CharacterSink::Attach);
    }

    /// The pump runs every completion, then inserts every streamed
    /// character, then attaches every requested one, whatever order they
    /// finished in.
    #[test]
    fn the_pump_drains_completions_then_streamed_then_attachments() {
        let mut stream = ContentStream::new();
        let log = Arc::new(Mutex::new(Vec::new()));
        let done = log.clone();
        stream.submit_job(Job {
            work: Box::new(|| {}),
            done: Some(Box::new(move || done.lock().unwrap().push("done"))),
        });
        let mut builders = Recorder {
            objects: vec![Some(20)],
            characters: vec![Some(40)],
            streamed_ok: true,
            ..Recorder::default()
        };
        stream.begin_characters(7, [character(3, [0.0; 3])]);
        stream.request_object(ObjectRequest {
            id: 5,
            flags: 0,
            parent: 7,
        });
        stream.request_character(CharacterRequest {
            owner: 9,
            scene: 7,
            class: 1,
            flags: 0x400,
        });
        assert!(matches!(
            stream.step(&mut builders),
            Step::Requests {
                job: true,
                object: Some(true),
                character: Some(true)
            }
        ));
        assert_eq!(stream.step(&mut builders), Step::Streamed { built: true });
        let pumped = stream.pump();
        assert_eq!(pumped.completions, 1);
        assert_eq!(pumped.added.len(), 1);
        assert_eq!(pumped.added[0].node, 3);
        assert_eq!(pumped.attached.len(), 1);
        assert_eq!(pumped.attached[0].node, 40);
        assert_eq!(*log.lock().unwrap(), ["done"]);
        // A second pump finds nothing: all three rings are empty.
        let again = stream.pump();
        assert_eq!(again.completions, 0);
        assert!(again.added.is_empty() && again.attached.is_empty());
    }

    /// The two arrival words name the player's placed-object kind; any other
    /// load has none.
    #[test]
    fn the_arrival_word_names_the_players_object_kind() {
        let mut stream = ContentStream::new();
        let mut builders = Recorder {
            level: vec![Some(41)],
            player: Some((0, 15, [1.0, 2.0, 3.0])),
            ..Recorder::default()
        };
        stream.begin_level(7, 1, ARRIVAL_WORD, NO_PREVIOUS_LEVEL);
        stream.step(&mut builders);
        assert_eq!(stream.player(), Some(41));
        let mut other = Recorder {
            level: vec![Some(42)],
            player: Some((0, 15, [1.0, 2.0, 3.0])),
            ..Recorder::default()
        };
        stream.begin_level(7, 1, 0, NO_PREVIOUS_LEVEL);
        stream.step(&mut other);
        assert_eq!(stream.player(), None);
        let mut respawn = Recorder {
            level: vec![Some(43)],
            player: Some((0, SESSION_START_PLAYER_KIND, [4.0, 5.0, 6.0])),
            ..Recorder::default()
        };
        stream.begin_level(7, 1, SESSION_START_WORD, NO_PREVIOUS_LEVEL);
        stream.step(&mut respawn);
        assert_eq!(stream.player(), Some(43));
        assert_eq!(stream.focus(), [4.0, 5.0, 6.0]);
    }

    #[test]
    fn the_level_wait_reports_built_and_missing_objects() {
        let mut stream = ContentStream::new();
        stream.begin_level(7, 3, 0, NO_PREVIOUS_LEVEL);
        let mut builders = Recorder {
            level: vec![Some(1), None, Some(3)],
            ..Recorder::default()
        };
        let mut waited = 0;
        let level = stream
            .wait_for_level(&mut builders, || waited += 1)
            .unwrap();
        assert_eq!(level.scene, 7);
        // The missing entry leaves no hole: the two built nodes follow each
        // other, in build order.
        assert_eq!(level.objects, [1, 3]);
        assert_eq!(level.missing, 1);
        assert_eq!(waited, 3);
        assert_eq!(stream.pending_streamed(), 0);
        // The list is cleared for the next chunk, and a level with no list
        // reports nothing at all.
        assert!(stream.wait_for_level(&mut builders, || {}).is_none());
        assert_eq!(
            *builders.calls.lock().unwrap(),
            ["level 0", "level 1", "level 2"]
        );
    }

    #[test]
    fn a_placed_player_becomes_the_focus_point() {
        let mut stream = ContentStream::new();
        let mut builders = Recorder {
            level: vec![Some(41), Some(42)],
            player: Some((1, ARRIVAL_PLAYER_KIND, [1200.0, 48.0, -600.0])),
            ..Recorder::default()
        };
        stream.begin_level(7, 2, ARRIVAL_WORD, NO_PREVIOUS_LEVEL);
        stream.begin_characters(7, []);
        stream.step(&mut builders);
        assert_eq!(stream.player(), None);
        assert_eq!(stream.focus(), [0.0; 3]);
        stream.step(&mut builders);
        assert_eq!(stream.player(), Some(42));
        assert_eq!(stream.focus(), [1200.0, 48.0, -600.0]);
        // An object of another kind, or another arrival word, is not the
        // player.
        let mut other = Recorder {
            level: vec![Some(41)],
            player: Some((0, ARRIVAL_PLAYER_KIND, [1.0, 2.0, 3.0])),
            ..Recorder::default()
        };
        stream.begin_level(7, 1, SESSION_START_WORD, NO_PREVIOUS_LEVEL);
        stream.step(&mut other);
        assert_eq!(stream.player(), None);
        // The focus point is the scene's, not the level's: a list that
        // builds no player leaves it where it was.
        assert_eq!(stream.focus(), [1200.0, 48.0, -600.0]);
        // A marker record names the player when its entrance word is the
        // session's arrival variable and its second word is the level the
        // session came from, whatever kind the actors have.
        const CAME_FROM: u32 = 0x0100_0003;
        let mut builders = Recorder {
            level: vec![Some(43)],
            marker: Some((0, [NO_ARRIVAL_WORD, CAME_FROM], [4.0, 5.0, 6.0])),
            ..Recorder::default()
        };
        stream.begin_level(7, 1, NO_ARRIVAL_WORD, CAME_FROM);
        stream.step(&mut builders);
        assert_eq!(stream.player(), Some(43));
        assert_eq!(stream.focus(), [4.0, 5.0, 6.0]);
        // A marker for another entrance, or from another level, is not the
        // player.
        for (words, arrival, previous) in [
            ([0, CAME_FROM], NO_ARRIVAL_WORD, CAME_FROM),
            ([NO_ARRIVAL_WORD, 0], NO_ARRIVAL_WORD, CAME_FROM),
            ([NO_ARRIVAL_WORD, CAME_FROM], NO_ARRIVAL_WORD, 0x0100_0004),
        ] {
            let mut builders = Recorder {
                level: vec![Some(44)],
                marker: Some((0, words, [7.0, 8.0, 9.0])),
                ..Recorder::default()
            };
            stream.begin_level(7, 1, arrival, previous);
            stream.step(&mut builders);
            assert_eq!(stream.player(), None);
            assert_eq!(stream.focus(), [4.0, 5.0, 6.0]);
        }
        // An entrance word that is not one of the two special ones still
        // matches the marker's own entrance.
        let mut builders = Recorder {
            level: vec![Some(45)],
            marker: Some((0, [0, 0], [1.0, 1.0, 1.0])),
            ..Recorder::default()
        };
        stream.begin_level(7, 1, 0, 0);
        stream.step(&mut builders);
        assert_eq!(stream.player(), Some(45));
    }

    /// The class word decides before the kind: an operative kind on a node
    /// the builder registered as a door, and a matching marker on a node it
    /// registered as an operative object, name no player.
    #[test]
    fn the_objects_class_word_gates_the_player_test() {
        let mut stream = ContentStream::new();
        let mut as_door = Recorder {
            level: vec![Some(51)],
            player: Some((0, ARRIVAL_PLAYER_KIND, [1.0, 2.0, 3.0])),
            player_class: OBJECT_DOOR,
            ..Recorder::default()
        };
        stream.begin_level(7, 1, ARRIVAL_WORD, NO_PREVIOUS_LEVEL);
        stream.step(&mut as_door);
        assert_eq!(stream.player(), None);
        assert_eq!(stream.focus(), [0.0; 3]);

        let mut as_operative = Recorder {
            level: vec![Some(52)],
            marker: Some((0, [NO_ARRIVAL_WORD, 5], [4.0, 5.0, 6.0])),
            marker_class: OBJECT_OPERATIVE,
            ..Recorder::default()
        };
        stream.begin_level(7, 1, NO_ARRIVAL_WORD, 5);
        stream.step(&mut as_operative);
        assert_eq!(stream.player(), None);
        // The classes are the section tags the object was built as.
        assert_eq!(
            (OBJECT_OPERATIVE, OBJECT_DOOR, OBJECT_VOLUME),
            (0x10000, 0x40000, 0x8000_AC07)
        );
    }

    /// An object that is not an actor has no operative kind, so no kind can
    /// make it the player.
    #[test]
    fn only_an_actor_can_be_the_player_by_kind() {
        let mut stream = ContentStream::new();
        let mut builders = Recorder {
            level: vec![Some(41)],
            ..Recorder::default()
        };
        stream.begin_level(7, 1, ARRIVAL_WORD, NO_PREVIOUS_LEVEL);
        stream.step(&mut builders);
        assert_eq!(stream.player(), None);
    }

    #[test]
    fn an_object_fetch_searches_the_results_and_waits_only_for_the_rest() {
        let mut stream = ContentStream::new();
        let mut builders = Recorder {
            objects: vec![Some(20)],
            ..Recorder::default()
        };
        stream.request_object(ObjectRequest {
            id: 5,
            flags: 2,
            parent: 7,
        });
        // A queued request makes an unmatched fetch pending.
        assert_eq!(stream.fetch_object(5, 2, 7), Fetch::Pending);
        stream.step(&mut builders);
        stream.request_object(ObjectRequest {
            id: 6,
            flags: 2,
            parent: 7,
        });
        // The built object is found although another request is queued, as
        // the original always searches its results; the queued one is
        // pending, a never-requested key is too while the queue is not empty.
        assert_eq!(stream.fetch_object(5, 2, 7), Fetch::Ready(20));
        assert_eq!(stream.fetch_object(6, 2, 7), Fetch::Pending);
        stream.cancel_object(6, 2, 7);
        assert_eq!(stream.fetch_object(5, 2, 7), Fetch::Missing);
        assert_eq!(stream.fetch_object(6, 2, 7), Fetch::Missing);
    }

    #[test]
    fn cancel_and_reset_keep_the_queues_consistent() {
        let mut stream = ContentStream::new();
        for id in [1, 2, 3] {
            stream.request_object(ObjectRequest {
                id,
                flags: 0,
                parent: 7,
            });
        }
        assert!(stream.cancel_object(2, 0, 7));
        assert!(!stream.cancel_object(2, 0, 7));
        stream.request_character(CharacterRequest {
            owner: 4,
            scene: 7,
            class: 0,
            flags: 0,
        });
        assert!(stream.cancel_character(4, 7));
        stream.begin_characters(7, [character(1, [0.0; 3])]);
        stream.begin_level(7, 2, 0, NO_PREVIOUS_LEVEL);
        stream.reset();
        assert_eq!(stream.queued_objects(), 0);
        assert_eq!(stream.queued_characters(), 0);
        assert_eq!(stream.pending_streamed(), 0);
        assert_eq!(stream.scene(), 0);
        let mut builders = Recorder::default();
        assert_eq!(stream.step(&mut builders), Step::Idle);
    }

    #[test]
    fn a_terminated_stream_stops_working() {
        let mut stream = ContentStream::new();
        stream.submit_job(Job {
            work: Box::new(|| panic!("must not run")),
            done: None,
        });
        stream.terminate();
        let mut builders = Recorder::default();
        assert_eq!(stream.step(&mut builders), Step::Idle);
        assert!(stream.is_terminated());
        // The waiting helpers must not spin on a terminated stream.
        stream.begin_level(7, 4, 0, NO_PREVIOUS_LEVEL);
        // A level that had a list reports the empty one it built.
        let level = stream.wait_for_level(&mut builders, || {}).unwrap();
        assert!(level.objects.is_empty());
        stream.request_character(CharacterRequest {
            owner: 1,
            scene: 7,
            class: 0,
            flags: 0,
        });
        assert!(stream.wait_for_characters(&mut builders, || {}).is_empty());
    }

    #[test]
    fn a_waited_character_drains_its_attachments() {
        let mut stream = ContentStream::new();
        let mut builders = Recorder {
            characters: vec![Some(11)],
            ..Recorder::default()
        };
        stream.request_character(CharacterRequest {
            owner: 1,
            scene: 7,
            class: 0,
            flags: 0x400,
        });
        let finished = stream.wait_for_characters(&mut builders, || {});
        assert_eq!(finished.len(), 1);
        assert_eq!(finished[0].node, 11);
        assert!(stream.wait_for_characters(&mut builders, || {}).is_empty());
    }

    /// A tag, its payload and the eight-byte header before it.
    fn saved(tag: u32, body: &[u8]) -> Vec<u8> {
        let mut out = tag.to_le_bytes().to_vec();
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(body);
        out
    }

    /// A saved blob: its total length, then the records.
    fn saved_blob(records: &[Vec<u8>]) -> Vec<u8> {
        let mut out = vec![0u8; 4];
        for record in records {
            out.extend_from_slice(record);
        }
        // The stored length counts the bytes after its own four, as the
        // character builder writes the remaining chunk bytes.
        let payload = (out.len() - 4) as u32;
        out[..4].copy_from_slice(&payload.to_le_bytes());
        out
    }

    /// The saved-chunk scan starts after the stored length and keeps the four
    /// tags the character builders replay; unknown tags and their payloads
    /// are stepped over. A short last body is read too: the bound covers the
    /// bytes after the length.
    #[test]
    fn a_streamed_character_saved_chunks_are_found_by_tag() {
        let pose = [1u8, 2, 3, 4];
        let ai = [9u8, 8];
        let blob = saved_blob(&[
            saved(0xCAC0_0001, &[7; 5]),
            saved(SAVED_AI_HOME, &ai),
            saved(SAVED_FLAT_POSE, &pose),
        ]);
        let chunks = saved_chunks(&blob);
        assert_eq!(chunks.flat_pose, Some(&pose[..]));
        assert_eq!(chunks.ai_home, Some(&ai[..]));
        assert_eq!(chunks.motion, None);
        assert_eq!(chunks.pose_tree, None);
        // A one-byte body in the last record is still found: the length is
        // the payload's, and the header of that record lies inside it.
        let last = saved_blob(&[saved(SAVED_POSE_TREE, &[5])]);
        assert_eq!(last[..4], 9u32.to_le_bytes());
        assert_eq!(saved_chunks(&last).pose_tree, Some(&[5u8][..]));
        // A record whose header lies past the stored payload is not read,
        // even though the buffer holds its bytes.
        let mut shorter = blob.clone();
        shorter[..4].copy_from_slice(&10u32.to_le_bytes());
        assert_eq!(saved_chunks(&shorter).flat_pose, None);
        assert_eq!(saved_chunks(&shorter).ai_home, None);
        // A repeated tag keeps its last occurrence.
        let blob = saved_blob(&[saved(SAVED_MOTION, &[1]), saved(SAVED_MOTION, &[2, 3])]);
        assert_eq!(saved_chunks(&blob).motion, Some(&[2u8, 3][..]));
    }

    /// Malformed blobs are total: a short header, a record whose bytes the
    /// slice does not hold and a zero-length loop all end the scan.
    #[test]
    fn saved_chunks_are_total_on_malformed_blobs() {
        assert_eq!(saved_chunks(&[]), SavedChunks::default());
        assert_eq!(saved_chunks(&[1, 2, 3]), SavedChunks::default());
        let mut trunc = (40u32).to_le_bytes().to_vec();
        trunc.extend_from_slice(&SAVED_FLAT_POSE.to_le_bytes());
        trunc.extend_from_slice(&100u32.to_le_bytes());
        trunc.extend_from_slice(&[1, 2, 3]);
        assert_eq!(saved_chunks(&trunc), SavedChunks::default());
        // A valid blob cut inside its last body loses that record only.
        let mut cut = saved_blob(&[saved(SAVED_MOTION, &[1]), saved(SAVED_FLAT_POSE, &[2, 3])]);
        cut.truncate(cut.len() - 1);
        assert_eq!(saved_chunks(&cut).motion, Some(&[1u8][..]));
        assert_eq!(saved_chunks(&cut).flat_pose, None);
        // A zero size is an empty payload and the scan goes on: the next
        // record's header is eight bytes further.
        let mut blob = (16u32).to_le_bytes().to_vec();
        blob.extend_from_slice(&SAVED_MOTION.to_le_bytes());
        blob.extend_from_slice(&0u32.to_le_bytes());
        blob.extend_from_slice(&saved(SAVED_POSE_TREE, &[5]));
        assert_eq!(saved_chunks(&blob).motion, Some(&[][..]));
        assert_eq!(saved_chunks(&blob).pose_tree, Some(&[5u8][..]));
    }

    /// The size word is scaled by 1/65535 in double precision, not by
    /// 1/65536: at half scale the two differ in the single-precision result.
    #[test]
    fn a_stored_size_word_scales_by_one_over_65535() {
        assert_eq!(stored_size_scale(1.0, 0), 0.0);
        assert_eq!(stored_size_scale(1.0, 32768), 0.500_007_6);
        assert_ne!(stored_size_scale(1.0, 32768), 0.5);
        // A full word is exactly one.
        assert_eq!(SIZE_WORD_SCALE.to_bits(), 0x3EF0_0010_0010_0010);
        assert_eq!(stored_size_scale(1.0, 65535), 1.0);
        assert_eq!(stored_size_scale(1.0, 21845), 0.333_333_34);
        // The base scale multiplies after the word's factor, in double.
        assert_eq!(stored_size_scale(0.5, 32768), 0.250_003_8);
        // NaN propagates through the double multiply.
        assert!(stored_size_scale(f32::NAN, 1).is_nan());
    }

    /// A zero second size word means "unset" to the streamed builder: both
    /// become 0xFFFF, whose scale is exactly one, unless the body is dead;
    /// the builder writes the second word's field before the first's.
    #[test]
    fn an_unset_size_pair_defaults_to_full_and_scales_to_one() {
        assert_eq!(normalised_size_words([0x8000, 0]), [0xFFFF, 0xFFFF]);
        assert_eq!(
            normalised_size_words([0x8000, 0x4000]),
            [0x8000, 0x4000],
            "a set pair stays"
        );
        let dead = exa_formats::characters::DEAD;
        assert_eq!(streamed_size_words([0x8000, 0], 0), [0xFFFF, 0xFFFF]);
        assert_eq!(streamed_size_words([0x8000, 0], dead), [0x8000, 0]);
        assert_eq!(
            character_scales(1.0, streamed_size_words([0, 0], 0)),
            (1.0, 1.0)
        );
        // The request builder and a dead body scale the stored zeros.
        assert_eq!(character_scales(1.0, [0, 0]), (0.0, 0.0));
        let (second, first) = character_scales(2.0, [32768, 65535]);
        assert_eq!(second, 2.0);
        assert_eq!(first, 1.000_015_3);
    }

    /// The request's 0x2000 marks the node with 0x1000; 0x1000 skips the AI
    /// start and the pose restore; a saved AI state's 16 bytes are three
    /// floats and a heading.
    #[test]
    fn the_character_request_flags_mark_the_node_and_keep_the_saved_ai() {
        assert_eq!(character_node_flags(0x40, 0), 0x40);
        assert_eq!(character_node_flags(0x40, MARK_NODE), 0x1040);
        assert_eq!(character_node_flags(0x1000, MARK_NODE), 0x1000);
        assert_ne!(MARK_NODE, NO_AI_START);
        assert_eq!(NODE_MARK, 0x1000);
        let mut bytes = Vec::new();
        for value in [1.5f32, -2.0, 300.0, 0.25] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        let pose = saved_ai_pose(&bytes).unwrap();
        assert_eq!(pose.position, [1.5, -2.0, 300.0]);
        assert_eq!(pose.heading, 0.25);
        assert_eq!(saved_ai_pose(&bytes[..15]), None);
        assert_eq!(saved_ai_pose(&[]), None);
    }

    /// The placed object's root node takes only the object's own flags
    /// under 0xB80000, and the placement's static bit adds the node's.
    #[test]
    fn a_placed_object_gives_its_root_the_object_flags_and_the_static_bit() {
        assert_eq!(OBJECT_NODE_FLAGS, 0x00B8_0000);
        // The mask clears exactly the copied field.
        assert_eq!(
            root_node_flags(0xFFFF_FFFF, 0, 0),
            0xFF47_FFFF,
            "the other node bits stay"
        );
        assert_eq!(root_node_flags(0, 0x00B8_0000, 0), 0x00B8_0000);
        assert_eq!(root_node_flags(0, 0x0008_0000, 0), NODE_STATIC);
        assert_eq!(root_node_flags(0, 0, PLACEMENT_STATIC), NODE_STATIC);
        // Object bits outside the field are not copied.
        assert_eq!(root_node_flags(0, 0x0004_0000, 0), 0);
        // The dynamic-node bit survives when the object does not carry it.
        assert_eq!(root_node_flags(0x0100_0000, 0, 0), 0x0100_0000);
    }

    #[test]
    fn a_stream_thread_starts_with_its_two_empty_buffers() {
        let mut thread = StreamThread::new();
        assert_eq!(thread.chunk_stream().size(), 0);
        assert_eq!(thread.parser().size(), 0);
        thread.chunk_stream().write_u32(0x3D23);
        thread.parser().write_u32(7);
        assert_eq!(thread.chunk_stream().bytes(), &0x3D23u32.to_le_bytes());
        assert_eq!(thread.parser().bytes(), &7u32.to_le_bytes());
    }

    #[test]
    fn the_level_wait_reports_missing_objects_in_the_original_words() {
        assert_eq!(missing_message(0), None);
        assert_eq!(
            missing_message(1).as_deref(),
            Some("1 missing database objects.")
        );
        assert_eq!(
            missing_message(37).as_deref(),
            Some("37 missing database objects.")
        );
        assert_eq!(LOG_SOURCE, "Content management");
    }

    #[test]
    fn a_grid_scene_in_room_mode_takes_the_objects_into_its_rooms() {
        assert_eq!(LevelHandoff::of(true, SCENE_ROOM_WORD), LevelHandoff::Rooms);
        assert_eq!(LevelHandoff::of(true, 0x1), LevelHandoff::Rooms);
        assert_eq!(LevelHandoff::of(true, 0x2), LevelHandoff::Rooms);
        assert_eq!(LevelHandoff::of(true, 0x70), LevelHandoff::Append);
        assert_eq!(
            LevelHandoff::of(false, SCENE_ROOM_WORD),
            LevelHandoff::Append
        );
    }
}
