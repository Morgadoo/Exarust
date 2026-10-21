//! Background content streaming: the scheduler the original's stream thread
//! runs.
//!
//! The original keeps seven rings between the main thread and its stream
//! thread: generic jobs, their completions, object requests and results,
//! character requests and results, and characters to attach to an existing
//! scene node. It also keeps a list of streamed characters and a list of
//! placed level objects. One pass of the thread, in this order:
//!
//! 1. a placed level object, while the level's list is unfinished;
//! 2. one generic job, whose completion callback is queued for the main
//!    thread;
//! 3. one requested database object;
//! 4. one requested character;
//! 5. the pending streamed character nearest the focus point;
//! 6. otherwise, sleep until woken.
//!
//! The main-thread pump runs the completion callbacks, attaches finished
//! characters and adds them to the scene. Requests and results are keyed
//! lookups, so the caller fetches what it asked for rather than reading a
//! shared object. Exarust drives [`ContentStream::step`] from its own
//! executor; the original runs the same loop on its stream thread.

use std::collections::VecDeque;

use crate::stream::{RING_GROWTH, RingQueue};

/// The distance below which no character is nearer: the original starts its
/// nearest search with the largest f32.
pub const FARTHEST: f32 = 3.4e38;

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
    pub owner: u64,
    pub scene: u64,
    pub node: u64,
    pub sink: CharacterSink,
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
    /// Ran a generic job.
    Job,
    /// Built a requested database object.
    Object { built: bool },
    /// Built a requested character.
    Character { built: bool },
    /// Built the nearest pending streamed character.
    Streamed { built: bool },
    /// Nothing to do; the original sleeps until woken.
    Idle,
}

/// The builders the scheduler calls. The original's stream thread calls into
/// the database, character and item factories; Exarust supplies closures
/// over its own loaders.
pub trait ContentBuilders {
    /// Builds placed level object `index` of the current level.
    fn level_object(&mut self, index: usize) -> Option<u64>;
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
    /// Characters attached to existing entities, and new characters for the
    /// scene.
    pub attached: Vec<CharacterResult>,
    pub added: Vec<CharacterResult>,
}

/// A level's list of placed objects being streamed.
#[derive(Debug, Default)]
struct LevelObjects {
    scene: u64,
    total: usize,
    done: usize,
    built: Vec<u64>,
    missing: usize,
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

/// The original's stream-thread scheduler.
pub struct ContentStream {
    jobs: RingQueue<Job>,
    completions: RingQueue<Completion>,
    object_requests: RingQueue<ObjectRequest>,
    object_results: RingQueue<ObjectResult>,
    character_requests: RingQueue<CharacterRequest>,
    character_results: RingQueue<CharacterResult>,
    attachments: RingQueue<CharacterResult>,
    /// The placed characters of the current level, front = already built.
    streamed: VecDeque<StreamedCharacter>,
    stream_cursor: usize,
    scene: u64,
    focus: [f32; 3],
    level: LevelObjects,
    /// Object requests that found no record since the last report.
    missing_objects: usize,
    terminated: bool,
}

impl Default for ContentStream {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentStream {
    /// Creates the queues with the original's capacities: 16 jobs and
    /// completions, 32 object and character requests and results, 16
    /// attachments, 128 streamed characters.
    pub fn new() -> Self {
        Self {
            jobs: RingQueue::new(16, 1),
            completions: RingQueue::new(16, 1),
            object_requests: RingQueue::new(32, 1),
            object_results: RingQueue::new(32, 1),
            character_requests: RingQueue::new(32, 1),
            character_results: RingQueue::new(32, 1),
            attachments: RingQueue::new(16, 1),
            streamed: VecDeque::with_capacity(128),
            stream_cursor: 0,
            scene: 0,
            focus: [0.0; 3],
            level: LevelObjects::default(),
            missing_objects: 0,
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

    /// Begins a level's object list: the scene and how many objects the
    /// chunk holds (the original's level handoff).
    pub fn begin_level(&mut self, scene: u64, total: usize) {
        self.level = LevelObjects {
            scene,
            total,
            ..LevelObjects::default()
        };
        self.scene = scene;
    }

    /// Begins a level's streamed characters: the scene, the focus point and
    /// the characters with their positions.
    pub fn begin_characters(
        &mut self,
        scene: u64,
        focus: [f32; 3],
        characters: impl IntoIterator<Item = StreamedCharacter>,
    ) {
        self.scene = scene;
        self.focus = focus;
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

    /// Object requests that found no record, since the last call.
    pub fn take_missing_objects(&mut self) -> usize {
        std::mem::take(&mut self.missing_objects)
    }

    /// One pass of the stream thread: level object, generic job, database
    /// object, character, nearest streamed character, else idle.
    pub fn step(&mut self, builders: &mut dyn ContentBuilders) -> Step {
        if self.terminated {
            return Step::Idle;
        }
        if self.level.done < self.level.total {
            let index = self.level.done;
            let built = builders.level_object(index);
            self.level.done += 1;
            match built {
                Some(node) => self.level.built.push(node),
                None => self.level.missing += 1,
            }
            return Step::LevelObject {
                built: built.is_some(),
            };
        }
        if let Some(job) = self.jobs.pop() {
            (job.work)();
            if let Some(done) = job.done {
                self.completions.push(done);
            }
            return Step::Job;
        }
        if let Some(request) = self.object_requests.pop() {
            let built = builders.object(&request);
            match built {
                Some(node) => self.object_results.push(ObjectResult { request, node }),
                None => self.missing_objects += 1,
            }
            return Step::Object {
                built: built.is_some(),
            };
        }
        if let Some(request) = self.character_requests.pop() {
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
                };
                match sink {
                    CharacterSink::Attach => self.attachments.push(result),
                    CharacterSink::Scene => self.character_results.push(result),
                }
            }
            return Step::Character {
                built: built.is_some(),
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
            self.stream_cursor += 1;
            return Step::Streamed {
                built: built.is_some(),
            };
        }
        Step::Idle
    }

    /// Runs completion callbacks and collects the finished characters for
    /// the main thread to insert (the original's pump).
    pub fn pump(&mut self) -> Pumped {
        let mut pumped = Pumped::default();
        while let Some(completion) = self.completions.pop() {
            completion();
            pumped.completions += 1;
        }
        while let Some(result) = self.attachments.pop() {
            pumped.attached.push(result);
        }
        while let Some(result) = self.character_results.pop() {
            pumped.added.push(result);
        }
        pumped
    }

    /// Fetches a finished database object by the key it was requested with:
    /// the original's object fetch. A missing result is `None`; the result
    /// queue keeps the order of the rest.
    pub fn fetch_object(&mut self, id: u32, flags: u32, parent: u64) -> Option<u64> {
        self.object_results
            .remove(|result| {
                result.request.id == id
                    && result.request.flags == flags
                    && result.request.parent == parent
            })
            .map(|result| result.node)
    }

    /// Fetches a finished character by its owner and scene (the original's
    /// character fetch), removing it from the result ring.
    pub fn fetch_character(&mut self, owner: u64, scene: u64) -> Option<CharacterResult> {
        self.character_results
            .remove(|result| result.owner == owner && result.scene == scene)
    }

    /// Waits for the level's object list: runs the scheduler until every
    /// object is done, then returns the built nodes and the missing count.
    /// The original also reports "N missing database objects.".
    pub fn wait_for_level(
        &mut self,
        builders: &mut dyn ContentBuilders,
        mut idle: impl FnMut(),
    ) -> LevelStream {
        while self.level.done < self.level.total && !self.terminated {
            self.step(builders);
            // The original pumps procedural work and yields between objects.
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
        stream
    }

    /// Waits for the character requests to finish, draining the attachment
    /// queue and letting the caller pump other work (the original drains the
    /// texture uploads and the procedural material step while it waits).
    pub fn wait_for_characters(
        &mut self,
        builders: &mut dyn ContentBuilders,
        mut idle: impl FnMut(),
    ) -> Vec<CharacterResult> {
        let mut finished = Vec::new();
        while !self.character_requests.is_empty() && !self.terminated {
            while let Some(result) = self.attachments.pop() {
                finished.push(result);
            }
            let step = self.step(builders);
            if !matches!(step, Step::Character { .. }) {
                idle();
            }
        }
        while let Some(result) = self.attachments.pop() {
            finished.push(result);
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
        self.streamed.clear();
        self.stream_cursor = 0;
        self.level = LevelObjects::default();
        self.missing_objects = 0;
        self.scene = 0;
    }
}

/// Squared distance between two points; the nearest-first order only needs
/// a monotonic measure.
fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    dx * dx + dy * dy + dz * dz
}

/// The growth the original's request rings use, exposed for callers that
/// size their own.
pub const CONTENT_GROWTH: usize = RING_GROWTH;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// A builder that records what it was asked for and answers with the
    /// encoded request.
    #[derive(Default)]
    struct Recorder {
        calls: Arc<Mutex<Vec<String>>>,
        level: Vec<Option<u64>>,
        objects: Vec<Option<u64>>,
        characters: Vec<Option<u64>>,
        streamed_ok: bool,
    }

    impl ContentBuilders for Recorder {
        fn level_object(&mut self, index: usize) -> Option<u64> {
            self.calls.lock().unwrap().push(format!("level {index}"));
            self.level.get(index).copied().flatten()
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
        };
        stream.begin_level(7, 2);
        stream.begin_characters(7, [0.0; 3], []);
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
        assert_eq!(stream.step(&mut builders), Step::Job);
        assert_eq!(stream.step(&mut builders), Step::Object { built: true });
        assert_eq!(stream.step(&mut builders), Step::Character { built: true });
        assert_eq!(stream.step(&mut builders), Step::Idle);
        assert_eq!(
            *calls.lock().unwrap(),
            ["level 0", "level 1", "object 5", "character 9"]
        );
        assert_eq!(stream.take_missing_objects(), 0);
        assert_eq!(stream.fetch_object(5, 0, 7), Some(20));
        assert_eq!(stream.fetch_object(5, 0, 7), None);
        // The pump drains the finished characters into the scene list.
        let pumped = stream.pump();
        assert_eq!(pumped.added.len(), 1);
        assert_eq!(pumped.added[0].node, 30);
        assert_eq!(pumped.added[0].sink, CharacterSink::Scene);
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
        assert_eq!(stream.step(&mut builders), Step::Character { built: true });
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
        stream.begin_characters(
            7,
            [0.0, 0.0, 0.0],
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
        assert_eq!(stream.step(&mut builders), Step::Job);
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
        assert_eq!(stream.step(&mut builders), Step::Character { built: true });
        let pumped = stream.pump();
        assert_eq!(pumped.added.len(), 0);
        assert_eq!(pumped.attached.len(), 1);
        assert_eq!(pumped.attached[0].node, 40);
        assert_eq!(pumped.attached[0].sink, CharacterSink::Attach);
    }

    #[test]
    fn the_level_wait_reports_built_and_missing_objects() {
        let mut stream = ContentStream::new();
        stream.begin_level(7, 3);
        let mut builders = Recorder {
            level: vec![Some(1), None, Some(3)],
            ..Recorder::default()
        };
        let mut waited = 0;
        let level = stream.wait_for_level(&mut builders, || waited += 1);
        assert_eq!(level.scene, 7);
        assert_eq!(level.objects, [1, 3]);
        assert_eq!(level.missing, 1);
        assert_eq!(waited, 3);
        assert_eq!(stream.pending_streamed(), 0);
        // The list is cleared for the next chunk.
        let empty = stream.wait_for_level(&mut builders, || {});
        assert!(empty.objects.is_empty());
        assert_eq!(empty.missing, 0);
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
        stream.begin_characters(7, [0.0; 3], [character(1, [0.0; 3])]);
        stream.begin_level(7, 2);
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
        stream.begin_level(7, 4);
        assert_eq!(stream.wait_for_level(&mut builders, || {}).objects.len(), 0);
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
}
