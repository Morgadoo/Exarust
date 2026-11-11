//! The content stream's thread: the original's stream thread runs the
//! scheduler of [`crate::content_stream`] on its own, sleeping on an event
//! the posts signal; here the same loop runs on Bevy's async compute pool.
//!
//! The original's thread loops until its terminate flag: one pass
//! ([`ContentStream::step`]'s order), and when the pass did nothing it waits
//! on the thread's event. Every post (a job, an object request, a character
//! request) pushes onto its ring under that ring's lock and then signals the
//! event. The rings are locked only around their own pushes, peeks and pops,
//! never across a build: a requested object is peeked, built with no lock
//! held, its result pushed and only then its request popped, so a fetch made
//! during the build answers "pending".
//!
//! Exarust keeps the rings in one [`ContentStream`] behind one lock, taken
//! for each ring step and released for the builds. The thread is a task on
//! the async compute pool that runs passes until one finds nothing and no
//! post came in meanwhile, then ends; the next post starts another (the
//! event's wait, without keeping a pool thread parked). With no pool (tools,
//! tests) the loop runs on a plain thread instead. One pass runs at a time,
//! so requests are built in the order they were posted, as one thread
//! builds them.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::content_stream::{
    CharacterRequest, CharacterResult, ContentBuilders, ContentStream, Fetch, Job, ObjectRequest,
    Pumped, Step, StreamedCharacter,
};

/// The thread's event and its state: whether a loop is running, and whether
/// a post signalled since the current pass began.
#[derive(Debug, Default)]
struct Wake {
    running: bool,
    signalled: bool,
    /// Passes run, for the logs and the tests.
    passes: u64,
}

struct Shared {
    stream: Mutex<ContentStream>,
    wake: Mutex<Wake>,
    builders: Mutex<Box<dyn ContentBuilders + Send>>,
}

fn lock<T: ?Sized>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The content stream with its thread (see the module notes).
#[derive(Clone)]
pub struct StreamWorker {
    shared: Arc<Shared>,
}

impl StreamWorker {
    /// A stream with empty rings whose thread builds through `builders`.
    pub fn new(builders: impl ContentBuilders + Send + 'static) -> Self {
        Self {
            shared: Arc::new(Shared {
                stream: Mutex::new(ContentStream::new()),
                wake: Mutex::new(Wake::default()),
                builders: Mutex::new(Box::new(builders)),
            }),
        }
    }

    /// Queues a generic job and signals the thread.
    pub fn submit_job(&self, job: Job) -> bool {
        let posted = lock(&self.shared.stream).submit_job(job);
        self.signal();
        posted
    }

    /// Queues an object request and signals the thread (the original's
    /// object post always succeeds).
    pub fn request_object(&self, request: ObjectRequest) -> bool {
        let posted = lock(&self.shared.stream).request_object(request);
        self.signal();
        posted
    }

    /// Queues a character request and signals the thread.
    pub fn request_character(&self, request: CharacterRequest) -> bool {
        let posted = lock(&self.shared.stream).request_character(request);
        self.signal();
        posted
    }

    /// Hands the thread a level's streamed characters, built nearest the
    /// focus point first once the request rings are empty
    /// ([`ContentStream::begin_characters`], the original's character-chunk
    /// handoff), and signals it. An empty list drops the ones not yet
    /// started.
    pub fn begin_characters(
        &self,
        scene: u64,
        focus: [f32; 3],
        characters: impl IntoIterator<Item = StreamedCharacter>,
    ) {
        {
            let mut stream = lock(&self.shared.stream);
            stream.set_focus(focus);
            stream.begin_characters(scene, characters);
        }
        self.signal();
    }

    /// Streamed characters not yet built.
    pub fn pending_streamed(&self) -> usize {
        lock(&self.shared.stream).pending_streamed()
    }

    /// The object fetch ([`ContentStream::fetch_object`]).
    pub fn fetch_object(&self, id: u32, flags: u32, parent: u64) -> Fetch {
        lock(&self.shared.stream).fetch_object(id, flags, parent)
    }

    /// The character fetch ([`ContentStream::fetch_character`]): `None`
    /// while the character is not built (the original's callers spin on
    /// that answer).
    pub fn fetch_character(&self, owner: u64, scene: u64) -> Option<CharacterResult> {
        lock(&self.shared.stream).fetch_character(owner, scene)
    }

    /// The main-thread pump ([`ContentStream::pump`]).
    pub fn pump(&self) -> Pumped {
        // The completions run outside the lock: they may post again.
        let completions = lock(&self.shared.stream).take_completions();
        let mut pumped = Pumped {
            completions: completions.len(),
            ..Pumped::default()
        };
        for completion in completions {
            completion();
        }
        lock(&self.shared.stream).drain_results(&mut pumped);
        pumped
    }

    /// Requests still queued (object, character).
    pub fn queued(&self) -> (usize, usize) {
        let stream = lock(&self.shared.stream);
        (stream.queued_objects(), stream.queued_characters())
    }

    /// The requested objects that built nothing.
    pub fn missing_requested(&self) -> usize {
        lock(&self.shared.stream).missing_requested()
    }

    /// Whether the thread has nothing left and is not running.
    pub fn is_idle(&self) -> bool {
        let wake = lock(&self.shared.wake);
        !wake.running && !lock(&self.shared.stream).has_work()
    }

    /// Passes the thread has run.
    pub fn passes(&self) -> u64 {
        lock(&self.shared.wake).passes
    }

    /// Empties every ring (the original's reset between sessions).
    pub fn reset(&self) {
        lock(&self.shared.stream).reset();
    }

    /// Stops the loop after its current pass, as the thread's terminate
    /// flag does.
    pub fn terminate(&self) {
        lock(&self.shared.stream).terminate();
    }

    /// The thread's event: marks a post and starts the loop when it is not
    /// running.
    fn signal(&self) {
        let start = {
            let mut wake = lock(&self.shared.wake);
            wake.signalled = true;
            !std::mem::replace(&mut wake.running, true)
        };
        if !start {
            return;
        }
        let shared = self.shared.clone();
        match bevy::tasks::AsyncComputeTaskPool::try_get() {
            Some(pool) => pool.spawn(async move { run(&shared) }).detach(),
            None => {
                std::thread::Builder::new()
                    .name("content stream".into())
                    .spawn(move || run(&shared))
                    .expect("the content stream's thread starts");
            }
        }
    }
}

/// The thread's loop: passes until one does nothing and no post came in
/// since it began, then the loop ends (the original waits on its event).
fn run(shared: &Shared) {
    loop {
        lock(&shared.wake).signalled = false;
        let step = pass(shared);
        let mut wake = lock(&shared.wake);
        wake.passes += 1;
        if step == Step::Idle && !wake.signalled {
            wake.running = false;
            return;
        }
    }
}

/// One pass of the thread, in [`ContentStream::step`]'s order, with the
/// rings locked only for their own steps.
fn pass(shared: &Shared) -> Step {
    let mut builders = lock(&shared.builders);
    {
        // The level's list (a stream given one): its next entry alone.
        let mut stream = lock(&shared.stream);
        if stream.level_pending() {
            return stream.step(&mut **builders);
        }
    }
    let job = lock(&shared.stream).take_job();
    let ran_job = job.is_some();
    if let Some(job) = job {
        (job.work)();
        lock(&shared.stream).finish_job(job.done);
    }
    let request = lock(&shared.stream).next_object();
    let object = request.map(|request| {
        let built = builders.object(&request);
        lock(&shared.stream).finish_object(&request, built);
        built.is_some()
    });
    let request = lock(&shared.stream).next_character();
    let character = request.map(|request| {
        let built = builders.character(&request);
        lock(&shared.stream).finish_character(&request, built);
        built.is_some()
    });
    if ran_job || object.is_some() || character.is_some() {
        return Step::Requests {
            job: ran_job,
            object,
            character,
        };
    }
    let next = lock(&shared.stream).next_streamed();
    if let Some(streamed) = next {
        let built = builders.streamed_character(&streamed);
        lock(&shared.stream).finish_streamed(&streamed, built);
        return Step::Streamed {
            built: built.is_some(),
        };
    }
    Step::Idle
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content_stream::LevelNode;
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    /// A builder that records the order it built in, and can be held on a
    /// gate so a test sees the rings mid-build.
    struct Gated {
        order: Arc<Mutex<Vec<String>>>,
        gate: Option<mpsc::Receiver<()>>,
    }

    impl ContentBuilders for Gated {
        fn level_object(&mut self, _: usize) -> Option<LevelNode> {
            None
        }
        fn object(&mut self, request: &ObjectRequest) -> Option<u64> {
            if let Some(gate) = &self.gate {
                gate.recv_timeout(Duration::from_secs(5)).ok();
            }
            lock(&self.order).push(format!("object {}", request.id));
            (request.id != 0).then_some(100 + u64::from(request.id))
        }
        fn character(&mut self, request: &CharacterRequest) -> Option<u64> {
            lock(&self.order).push(format!("character {}", request.owner));
            Some(200 + request.owner)
        }
        fn streamed_character(&mut self, character: &StreamedCharacter) -> Option<u64> {
            lock(&self.order).push(format!("streamed {}", character.node));
            (character.node != 0).then_some(300 + character.node)
        }
    }

    fn wait_idle(worker: &StreamWorker) {
        let started = Instant::now();
        while !worker.is_idle() {
            assert!(started.elapsed() < Duration::from_secs(5), "never idle");
            std::thread::yield_now();
        }
    }

    fn object(id: u32) -> ObjectRequest {
        ObjectRequest {
            id,
            flags: 0x1000_0000,
            parent: 7,
        }
    }

    #[test]
    fn requests_are_built_off_the_caller_in_posting_order() {
        let order = Arc::new(Mutex::new(Vec::new()));
        let worker = StreamWorker::new(Gated {
            order: order.clone(),
            gate: None,
        });
        worker.request_object(object(1));
        worker.request_character(CharacterRequest {
            owner: 5,
            scene: 7,
            class: 0,
            flags: 0,
        });
        worker.request_object(object(2));
        wait_idle(&worker);
        assert_eq!(*lock(&order), ["object 1", "character 5", "object 2"]);
        assert_eq!(worker.fetch_object(2, 0x1000_0000, 7), Fetch::Ready(102));
        assert_eq!(worker.fetch_object(1, 0x1000_0000, 7), Fetch::Ready(101));
        assert_eq!(worker.fetch_character(5, 7).map(|r| r.node), Some(205));
        assert_eq!(worker.fetch_object(1, 0x1000_0000, 7), Fetch::Missing);
    }

    #[test]
    fn a_fetch_during_the_build_is_pending_until_the_result_is_in() {
        let (open, gate) = mpsc::channel();
        let worker = StreamWorker::new(Gated {
            order: Arc::default(),
            gate: Some(gate),
        });
        worker.request_object(object(3));
        // The request stays queued while it is built.
        assert_eq!(worker.fetch_object(3, 0x1000_0000, 7), Fetch::Pending);
        open.send(()).unwrap();
        wait_idle(&worker);
        assert_eq!(worker.fetch_object(3, 0x1000_0000, 7), Fetch::Ready(103));
    }

    #[test]
    fn a_failed_object_counts_missing_and_its_fetch_is_missing() {
        let worker = StreamWorker::new(Gated {
            order: Arc::default(),
            gate: None,
        });
        worker.request_object(object(0));
        wait_idle(&worker);
        assert_eq!(worker.missing_requested(), 1);
        assert_eq!(worker.fetch_object(0, 0x1000_0000, 7), Fetch::Missing);
    }

    #[test]
    fn the_loop_ends_when_idle_and_a_later_post_starts_it_again() {
        let order = Arc::new(Mutex::new(Vec::new()));
        let worker = StreamWorker::new(Gated {
            order: order.clone(),
            gate: None,
        });
        worker.request_object(object(1));
        wait_idle(&worker);
        let passes = worker.passes();
        assert!(passes >= 2, "a building pass and the idle one");
        worker.request_object(object(4));
        wait_idle(&worker);
        assert!(worker.passes() > passes);
        assert_eq!(*lock(&order), ["object 1", "object 4"]);
    }

    #[test]
    fn a_terminated_stream_builds_nothing_more() {
        let order = Arc::new(Mutex::new(Vec::new()));
        let worker = StreamWorker::new(Gated {
            order: order.clone(),
            gate: None,
        });
        worker.terminate();
        worker.request_object(object(1));
        wait_idle(&worker);
        assert!(lock(&order).is_empty());
    }

    /// The level's streamed characters wait for the request rings, then
    /// build nearest the focus first; the pump hands each over with its
    /// stage node; a new list drops what was not started.
    #[test]
    fn streamed_characters_follow_the_requests_nearest_first() {
        let order = Arc::new(Mutex::new(Vec::new()));
        let (open, gate) = mpsc::channel();
        let worker = StreamWorker::new(Gated {
            order: order.clone(),
            gate: Some(gate),
        });
        // Held on its gate, the object keeps the thread busy while the
        // characters are handed over.
        worker.request_object(object(1));
        let at = |node, x| StreamedCharacter {
            node,
            scene: 9,
            position: [x, 0.0, 0.0],
        };
        worker.begin_characters(9, [4.0, 0.0, 0.0], [at(1, 0.0), at(2, 5.0), at(0, 3.5)]);
        open.send(()).unwrap();
        wait_idle(&worker);
        assert_eq!(
            *lock(&order),
            ["object 1", "streamed 0", "streamed 2", "streamed 1"]
        );
        let pumped = worker.pump();
        let added: Vec<_> = pumped
            .added
            .iter()
            .map(|r| (r.node, r.placeholder))
            .collect();
        assert_eq!(added, [(302, Some(2)), (301, Some(1))]);
        assert_eq!(worker.pending_streamed(), 0);
    }
}
