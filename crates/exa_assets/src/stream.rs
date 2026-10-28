//! The original's content queues and its texture loading queue.
//!
//! The engine hands work between the main thread and its loader threads
//! through single-producer/single-consumer rings: a head index the consumer
//! reads, a tail index the producer writes, and a buffer that grows when it
//! runs out of room. A request can be promoted to the front of its ring
//! (the "priority item" of the texture loader), and the consumer drains only
//! the entries that were ready when it started, so work arriving while it
//! runs waits for the next pass.
//!
//! Exarust keeps the same order and the same snapshot rule. The ring grows
//! instead of waiting for room, so no request is ever dropped; the original
//! blocks its producer until the consumer catches up.
//!
//! The loader thread decodes its ring one request at a time. When it has
//! decoded a request while earlier uploads are still waiting for the main
//! thread, and the request was not promoted, it suspends itself holding the
//! decoded image until the main thread's upload pass wakes it; then it
//! queues that upload and goes on. With an empty ring it marks itself idle
//! and sleeps until a request wakes it. A paused loader is not woken by new
//! requests.

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use crate::texture::{ColorSpace, TextureSettings, placeholder_image};
use crate::texture_cache::TextureKey;

/// Every ring grows by this many entries, as the texture loader's does.
pub const RING_GROWTH: usize = 128;

/// A single-producer/single-consumer ring, in insertion order.
#[derive(Debug, Clone)]
pub struct RingQueue<T> {
    slots: Vec<Option<T>>,
    head: usize,
    tail: usize,
    growth: usize,
}

impl<T> RingQueue<T> {
    /// A ring with room for `capacity` entries, growing by `growth` when it
    /// is full. A zero capacity ring grows from `growth` on its first push.
    pub fn new(capacity: usize, growth: usize) -> Self {
        Self {
            slots: (0..capacity).map(|_| None).collect(),
            head: 0,
            tail: 0,
            growth: growth.max(1),
        }
    }

    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    pub fn len(&self) -> usize {
        if self.slots.is_empty() {
            return 0;
        }
        if self.tail >= self.head {
            self.tail - self.head
        } else {
            self.slots.len() - self.head + self.tail
        }
    }

    pub fn is_empty(&self) -> bool {
        self.head == self.tail
    }

    /// A ring of `n` slots holds `n - 1` entries: the head and tail indices
    /// meet both when it is empty and when it is full, and the original
    /// waits one slot early so that it never has to tell them apart.
    pub fn is_full(&self) -> bool {
        !self.slots.is_empty() && (self.tail + 1) % self.slots.len() == self.head
    }

    /// Appends an entry, growing the ring when it is full.
    pub fn push(&mut self, value: T) {
        if self.slots.is_empty() || self.is_full() {
            self.grow();
        }
        self.slots[self.tail] = Some(value);
        self.tail = (self.tail + 1) % self.slots.len();
    }

    /// The next entry, without removing it (the original's pop step).
    pub fn front(&self) -> Option<&T> {
        if self.is_empty() {
            return None;
        }
        self.slots[self.head].as_ref()
    }

    pub fn front_mut(&mut self) -> Option<&mut T> {
        if self.is_empty() {
            return None;
        }
        self.slots[self.head].as_mut()
    }

    /// Removes the front entry (the original's advance step).
    pub fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }
        let value = self.slots[self.head].take();
        self.head = (self.head + 1) % self.slots.len();
        value
    }

    /// Removes the front entry without returning it.
    pub fn advance(&mut self) {
        let _ = self.pop();
    }

    /// Moves the first entry matching `wanted` to the front, keeping every
    /// other entry: the original's request promotion, which shifts the head
    /// back one slot and refills the hole with the last entry.
    pub fn promote<F: Fn(&T) -> bool>(&mut self, wanted: F) -> bool {
        if self.slots.is_empty() {
            return false;
        }
        let len = self.len();
        let mut index = self.head;
        let mut found = None;
        for _ in 0..len {
            if self.slots[index].as_ref().is_some_and(&wanted) {
                found = Some(index);
                break;
            }
            index = (index + 1) % self.slots.len();
        }
        let Some(index) = found else {
            return false;
        };
        if index != self.head {
            let capacity = self.slots.len();
            let new_head = if self.head == 0 {
                capacity - 1
            } else {
                self.head - 1
            };
            let new_tail = if self.tail == 0 {
                capacity - 1
            } else {
                self.tail - 1
            };
            self.head = new_head;
            self.tail = new_tail;
            self.slots[new_head] = self.slots[index].take();
            self.slots[index] = self.slots[new_tail].take();
        }
        true
    }

    /// Removes the first entry matching `wanted`, keeping the order of the
    /// rest. The original has no request removal; a released texture simply
    /// never uploads, and this is Exarust's equivalent.
    pub fn remove<F: Fn(&T) -> bool>(&mut self, wanted: F) -> Option<T> {
        if self.slots.is_empty() {
            return None;
        }
        let len = self.len();
        let capacity = self.slots.len();
        let mut index = self.head;
        for _ in 0..len {
            if self.slots[index].as_ref().is_some_and(&wanted) {
                let value = self.slots[index].take()?;
                // Shift the entries after the hole one slot back.
                let mut at = index;
                loop {
                    let next = (at + 1) % capacity;
                    if next == self.tail {
                        break;
                    }
                    self.slots[at] = self.slots[next].take();
                    at = next;
                }
                self.tail = if self.tail == 0 {
                    capacity - 1
                } else {
                    self.tail - 1
                };
                return Some(value);
            }
            index = (index + 1) % capacity;
        }
        None
    }

    pub fn iter(&self) -> RingIter<'_, T> {
        RingIter {
            queue: self,
            index: self.head,
            left: self.len(),
        }
    }

    pub fn reset(&mut self) {
        for slot in &mut self.slots {
            *slot = None;
        }
        self.head = 0;
        self.tail = 0;
    }

    fn grow(&mut self) {
        let len = self.len();
        let new_capacity = if self.slots.is_empty() {
            self.growth
        } else {
            self.slots.len() + self.growth
        };
        let mut slots: Vec<Option<T>> = (0..new_capacity).map(|_| None).collect();
        for (at, slot) in slots.iter_mut().enumerate().take(len) {
            let source = (self.head + at) % self.slots.len();
            *slot = self.slots[source].take();
        }
        self.slots = slots;
        self.head = 0;
        self.tail = len;
    }
}

/// Iterates a [`RingQueue`] from front to back.
pub struct RingIter<'a, T> {
    queue: &'a RingQueue<T>,
    index: usize,
    left: usize,
}

impl<'a, T> Iterator for RingIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.left == 0 || self.queue.slots.is_empty() {
            return None;
        }
        let value = self.queue.slots[self.index].as_ref();
        self.index = (self.index + 1) % self.queue.slots.len();
        self.left -= 1;
        value
    }
}

/// Run after each upload: the original's wake-up step. `Send + Sync` so a
/// stream can live in a Bevy resource.
pub type UploadCallback = Box<dyn FnMut(&TextureKey) + Send + Sync>;

/// One texture load waiting for the loader thread.
#[derive(Debug, Clone)]
pub struct TextureRequest {
    pub key: TextureKey,
    pub space: ColorSpace,
    /// The creation/sampler word the requesting material wants.
    pub flags: u32,
    /// The image entry to decode.
    pub source: String,
    /// Promoted to the front (the original's "priority" mark, status bit 2):
    /// its decode does not yield to waiting uploads.
    pub promoted: bool,
    /// The loader assigns this when the request is queued, so a release can
    /// cancel this load alone while the same name is requested again (the
    /// original's per-record status bits).
    pub id: u64,
}

impl TextureRequest {
    /// A request for `name` in `source`, not yet promoted.
    pub fn new(name: &str, space: ColorSpace, flags: u32, source: impl Into<String>) -> Self {
        Self {
            key: TextureKey::new(name, space),
            space,
            flags,
            source: source.into(),
            promoted: false,
            id: 0,
        }
    }
}

/// A decoded image waiting for the main thread to add it to Bevy.
pub struct TextureUpload {
    pub key: TextureKey,
    pub image: Image,
    pub alpha: bool,
}

/// What queueing a request did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestOutcome {
    /// The request was appended.
    Queued,
    /// The same texture was already queued; the first request is kept.
    AlreadyQueued,
}

/// The texture loader's request queue and its pending-upload list.
///
/// The original runs `decode_next` on its loader thread and `pump_uploads` on
/// the main thread each frame; the loader yields the main thread whenever
/// uploads are waiting, and a request can be promoted to the front so a
/// material that needs it can wait out the load.
///
/// A request's progress is the original record's low status bits:
///
/// | bit | meaning |
/// |---:|---|
/// | `0x1` | streaming: a load was requested; the priority wait spins while it is set |
/// | `0x2` | promoted (the priority item); a load requested while the manager's fast-load flag is on starts with it |
/// | `0x4` | taken by the loader and still in its pass (Exarust's `in_hand`) |
/// | `0x8` | decoded, waiting for the main thread's upload; also a load made on the spot |
/// | `0x10` | released while loading; its upload is dropped |
///
/// The loader clears the first four bits of the record it just finished when
/// it starts looking for its next request, so a priority load of a texture
/// that is already decoded and queued for upload finds nothing to promote
/// ("Priority item not found."); only the record the loader still holds,
/// decoding it or waiting with it for the upload pass, is found. A release
/// of a record the loader holds waits for that pass to end.
#[derive(Default)]
pub struct TextureStream {
    requests: Option<RingQueue<TextureRequest>>,
    uploads: Vec<TextureUpload>,
    /// The keys uploaded since the last [`TextureStream::take_finished`].
    finished: Vec<TextureKey>,
    /// Released while loading (status bit 16), by request id.
    cancelled: HashSet<u64>,
    next_id: u64,
    /// The requests the loader is decoding now (status bit 1 per record),
    /// with the id that request was queued under.
    in_hand: HashMap<TextureKey, u64>,
    in_flight: usize,
    /// Whether the request decoded last was promoted.
    last_promoted: bool,
    paused: bool,
    /// The loader was woken (the original's wake to the loader thread): by a
    /// queued request or by an upload pass that let it go on.
    loader_wake: bool,
    /// The decoded request the loader holds while it waits for the main
    /// thread's upload pass: it finished it while earlier uploads were
    /// waiting and it was not promoted. Exarust has already queued its
    /// upload (behind the earlier ones, so the order is the same); the
    /// loader starts no other decode until a pass releases it.
    holding: Option<TextureKey>,
    /// The loader's idle flag: its ring was empty when it last looked. The
    /// original sets it before it sleeps, and a queued request clears it.
    loader_idle: bool,
    completion: Option<UploadCallback>,
    settings: TextureSettings,
}

impl TextureStream {
    pub fn new() -> Self {
        Self {
            requests: Some(RingQueue::new(16, RING_GROWTH)),
            ..Self::default()
        }
    }

    /// Queues a load. The original also wakes the loader unless the stream is
    /// paused; [`TextureStream::take_wake`] reports that.
    pub fn request(&mut self, mut request: TextureRequest) -> RequestOutcome {
        let queue = self
            .requests
            .get_or_insert_with(|| RingQueue::new(16, RING_GROWTH));
        // A duplicate request for a texture already queued would upload it
        // twice; the manager never asks for the same record twice, but a
        // stale caller can. Keep the first, as the file loader's registry
        // does.
        if queue.iter().any(|queued| queued.key == request.key) {
            return RequestOutcome::AlreadyQueued;
        }
        request.id = self.next_id;
        self.next_id += 1;
        queue.push(request);
        self.in_flight += 1;
        // The push clears the loader's idle flag before it wakes it.
        self.loader_idle = false;
        self.wake();
        RequestOutcome::Queued
    }

    /// Moves a queued request to the front and marks it promoted: the
    /// original's "priority item". A request the loader holds right now
    /// (status bit 4) counts as found and stays where it is: one it is
    /// decoding, or the decoded one it waits with for the upload pass. One
    /// already handed to the upload list is not (the loader cleared its
    /// bits when it went on), nor one already uploaded; the caller reports
    /// "Priority item not found.".
    pub fn promote(&mut self, key: &TextureKey) -> bool {
        if self.in_hand.contains_key(key) || self.holding.as_ref() == Some(key) {
            return true;
        }
        let Some(queue) = self.requests.as_mut() else {
            return false;
        };
        if !queue.promote(|request| &request.key == key) {
            return false;
        }
        if let Some(front) = queue.front_mut() {
            front.promoted = true;
        }
        true
    }

    /// Drops a load that has not uploaded yet. A queued request is removed
    /// in order; one already decoded and waiting for its upload is dropped
    /// with it; one being decoded now is marked (the original's status bit
    /// 16) so its upload is skipped. The original's record is freed, which
    /// is the same outcome. `false` when nothing was pending (the texture
    /// was already uploaded).
    pub fn cancel(&mut self, key: &TextureKey) -> bool {
        let queued = self
            .requests
            .as_mut()
            .is_some_and(|queue| queue.remove(|request| &request.key == key).is_some());
        let uploading = self
            .uploads
            .iter()
            .position(|upload| &upload.key == key)
            .map(|index| self.uploads.remove(index))
            .is_some();
        let in_hand = self.in_hand.get(key).copied();
        if !queued && !uploading && in_hand.is_none() {
            return false;
        }
        if queued {
            self.in_flight = self.in_flight.saturating_sub(1);
        }
        if uploading {
            self.in_flight = self.in_flight.saturating_sub(1);
        }
        // One still decoding is dropped when the pass finishes.
        if let Some(id) = in_hand {
            self.cancelled.insert(id);
        }
        true
    }

    pub fn settings(&self) -> &TextureSettings {
        &self.settings
    }

    pub fn set_settings(&mut self, settings: TextureSettings) {
        self.settings = settings;
    }

    /// The next request to decode, taken off the queue (the loader's side).
    /// The record is marked in progress before the decode, as the original's
    /// loader does; [`TextureStream::finish_decode`] completes the pass.
    pub fn take_next(&mut self) -> Option<TextureRequest> {
        let request = self.requests.as_mut().and_then(RingQueue::pop);
        match request {
            Some(request) => {
                self.loader_idle = false;
                self.last_promoted = request.promoted;
                self.in_hand.insert(request.key, request.id);
                Some(request)
            }
            None => {
                // The loader found its ring empty: it marks itself idle and
                // sleeps until a request wakes it.
                self.loader_idle = true;
                None
            }
        }
    }

    /// Completes a pass started by [`TextureStream::take_next`]: queues the
    /// decoded image for upload, or the 1×1 grey placeholder when it could
    /// not be read. A request released while it was decoding is dropped
    /// (status bit 16). Returns whether an upload was queued.
    pub fn finish_decode(
        &mut self,
        request: TextureRequest,
        decoded: Option<(Image, bool)>,
    ) -> bool {
        // A newer load of the same texture may have been taken while this
        // one ran; only this load's own mark goes.
        if self.in_hand.get(&request.key) == Some(&request.id) {
            self.in_hand.remove(&request.key);
        }
        // The loader waits for the main thread when earlier uploads are
        // still queued and this request was not promoted; it holds the
        // record (its in-hand bit stays set) until the upload pass wakes it.
        let waits = !self.uploads.is_empty() && !self.last_promoted;
        if self.cancelled.remove(&request.id) {
            self.in_flight = self.in_flight.saturating_sub(1);
            return false;
        }
        let (image, alpha) = decoded.unwrap_or_else(|| {
            (
                placeholder_image(request.space, request.flags, &self.settings),
                false,
            )
        });
        if waits {
            self.holding = Some(request.key);
        } else if self.requests.as_ref().is_none_or(RingQueue::is_empty) {
            // Its next look finds the ring empty.
            self.loader_idle = true;
        }
        self.uploads.push(TextureUpload {
            key: request.key,
            image,
            alpha,
        });
        true
    }

    /// Decodes at most one request with `decode` and queues the result for
    /// upload. The caller runs this on whatever executor it has (the
    /// original uses its own loader thread). Returns whether a request was
    /// taken.
    pub fn decode_next(
        &mut self,
        mut decode: impl FnMut(&TextureRequest) -> Option<(Image, bool)>,
    ) -> bool {
        let Some(request) = self.take_next() else {
            return false;
        };
        let decoded = decode(&request);
        self.finish_decode(request, decoded);
        true
    }

    /// Whether the loader is waiting for the main thread: it decoded a
    /// request while earlier uploads were queued, the request was not
    /// promoted, and no upload pass has run since. Pausing does not change
    /// it.
    pub fn wants_to_yield(&self) -> bool {
        self.holding.is_some()
    }

    /// Whether the loader may start another decode now: a request is queued
    /// and it is not waiting for an upload pass.
    pub fn can_decode(&self) -> bool {
        self.requests
            .as_ref()
            .is_some_and(|queue| !queue.is_empty())
            && !self.wants_to_yield()
    }

    /// Whether the loader's ring was empty when it last looked (the
    /// original's idle flag at the loader).
    pub fn loader_idle(&self) -> bool {
        self.loader_idle
    }

    /// Uploads the requests that were ready when the pass started, up to
    /// `budget` of them. Entries queued during the pass wait for the next
    /// one, as in the original. The pass wakes the loader afterwards, as the
    /// original's pump does. Returns how many were uploaded.
    pub fn pump_uploads(&mut self, budget: usize, mut upload: impl FnMut(TextureUpload)) -> usize {
        let ready = self.uploads.len().min(budget);
        if ready == 0 {
            // An empty pass still wakes a loader that is not idle.
            if self.uploads.is_empty() && !self.loader_idle {
                self.holding = None;
                self.wake();
            }
            return 0;
        }
        let rest = self.uploads.split_off(ready);
        let ready = std::mem::replace(&mut self.uploads, rest);
        let mut done = 0;
        for item in ready {
            self.in_flight = self.in_flight.saturating_sub(1);
            self.finished.push(item.key);
            if let Some(callback) = self.completion.as_mut() {
                callback(&item.key);
            }
            upload(item);
            done += 1;
        }
        // The original wakes the loader after the pass unless it is asleep
        // with an empty ring; a loader waiting with a decoded record goes
        // on (its upload is already queued here). A new request wakes it
        // too.
        if !self.loader_idle {
            self.holding = None;
            self.wake();
        }
        done
    }

    /// Pauses the stream: requests are still queued but the loader is not
    /// woken (the original's pause before a bulk load).
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Resumes the stream and wakes the loader.
    pub fn resume(&mut self) {
        self.paused = false;
        if !self.requests.as_ref().is_none_or(RingQueue::is_empty) {
            self.wake();
        }
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Whether the loader was woken since the last call, by a queued request
    /// or an upload pass (the original's wake to the loader thread; a
    /// paused loader is not woken).
    pub fn take_wake(&mut self) -> bool {
        std::mem::take(&mut self.loader_wake)
    }

    /// Whether the loader has nothing queued, nothing decoding and nothing
    /// waiting for its upload: the original's idle flag, which its wait loop
    /// spins on.
    pub fn is_idle(&self) -> bool {
        self.in_flight == 0 && self.uploads.is_empty() && self.in_hand.is_empty()
    }

    pub fn queued(&self) -> usize {
        self.requests.as_ref().map_or(0, RingQueue::len)
    }

    /// Whether any request is queued for the loader.
    pub fn has_requests(&self) -> bool {
        self.queued() > 0
    }

    pub fn pending_uploads(&self) -> usize {
        self.uploads.len()
    }

    /// How many requests the loader is decoding right now.
    pub fn in_hand(&self) -> usize {
        self.in_hand.len()
    }

    /// Whether `key` is still queued or waiting to upload.
    pub fn is_pending(&self, key: &TextureKey) -> bool {
        self.requests
            .as_ref()
            .is_some_and(|queue| queue.iter().any(|request| &request.key == key))
            || self.uploads.iter().any(|upload| &upload.key == key)
            || self.in_hand.contains_key(key)
    }

    /// Loads one texture to completion on the main thread: promotes its
    /// request to the front, decodes it if it is still queued, and drains
    /// every ready upload as the original's pump does. The original's
    /// "priority item" path (it spins until the loader clears the record's
    /// pending bit). Returns `false` when the request was not found, which
    /// the original reports as "Priority item not found.".
    pub fn load_now(
        &mut self,
        key: &TextureKey,
        mut decode: impl FnMut(&TextureRequest) -> Option<(Image, bool)>,
        mut upload: impl FnMut(TextureUpload),
    ) -> bool {
        if !self.promote(key) {
            return false;
        }
        // The priority wait clears the loader's pause flag while it spins.
        self.paused = false;
        // The promoted request sits at the front unless it is already being
        // decoded or uploaded.
        let front = self
            .requests
            .as_ref()
            .and_then(RingQueue::front)
            .is_some_and(|request| &request.key == key);
        if front {
            self.decode_next(&mut decode);
        }
        while self.uploads.iter().any(|item| &item.key == key) {
            self.pump_uploads(usize::MAX, &mut upload);
        }
        true
    }

    /// The uploads finished since the last call, in upload order.
    pub fn take_finished(&mut self) -> Vec<TextureKey> {
        std::mem::take(&mut self.finished)
    }

    /// Installs the callback run after each upload: the original's step that
    /// wakes the waiting caller.
    pub fn on_uploaded(&mut self, callback: UploadCallback) {
        self.completion = Some(callback);
    }

    /// Empties the queues (the loader's reset between levels).
    pub fn reset(&mut self) {
        if let Some(queue) = self.requests.as_mut() {
            queue.reset();
        }
        self.uploads.clear();
        self.finished.clear();
        self.cancelled.clear();
        self.in_hand.clear();
        self.in_flight = 0;
        self.last_promoted = false;
        self.holding = None;
        self.loader_idle = false;
    }

    fn wake(&mut self) {
        if !self.paused {
            self.loader_wake = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(name: &str) -> TextureKey {
        TextureKey::new(name, ColorSpace::Srgb)
    }

    fn request(name: &str) -> TextureRequest {
        TextureRequest::new(name, ColorSpace::Srgb, 0, name)
    }

    fn names(queue: &RingQueue<TextureRequest>) -> Vec<String> {
        queue
            .iter()
            .map(|request| request.key.name.as_str().to_string())
            .collect()
    }

    #[test]
    fn a_ring_keeps_insertion_order_across_wraps_and_growth() {
        let mut queue = RingQueue::new(4, 64);
        for name in ["a", "b", "c"] {
            queue.push(request(name));
        }
        assert_eq!(queue.len(), 3);
        assert_eq!(queue.capacity(), 4);
        assert_eq!(queue.pop().unwrap().key, key("a"));
        // Three entries fill a ring of four slots (the original keeps one
        // slot free so that head == tail means empty).
        queue.push(request("d"));
        assert_eq!(names(&queue), ["b", "c", "d"]);
        // Full: the next push grows the ring and preserves the order.
        queue.push(request("e"));
        assert_eq!(queue.capacity(), 68);
        assert_eq!(names(&queue), ["b", "c", "d", "e"]);
        queue.push(request("f"));
        assert_eq!(names(&queue), ["b", "c", "d", "e", "f"]);
        assert_eq!(queue.pop().unwrap().key, key("b"));
        assert_eq!(names(&queue), ["c", "d", "e", "f"]);
    }

    #[test]
    fn promotion_moves_an_entry_to_the_front_and_keeps_the_rest() {
        let mut queue = RingQueue::new(8, 8);
        for name in ["a", "b", "c", "d"] {
            queue.push(request(name));
        }
        assert!(queue.promote(|r| r.key == key("c")));
        assert_eq!(names(&queue), ["c", "a", "b", "d"]);
        // Promoting the front entry changes nothing.
        assert!(queue.promote(|r| r.key == key("c")));
        assert_eq!(names(&queue), ["c", "a", "b", "d"]);
        // Promoting the last entry: it becomes the front.
        assert!(queue.promote(|r| r.key == key("d")));
        assert_eq!(names(&queue), ["d", "c", "a", "b"]);
        assert!(!queue.promote(|r| r.key == key("zz")));
    }

    #[test]
    fn removal_keeps_the_order_of_the_others() {
        let mut queue = RingQueue::new(8, 8);
        for name in ["a", "b", "c", "d"] {
            queue.push(request(name));
        }
        assert_eq!(queue.remove(|r| r.key == key("b")).unwrap().key, key("b"));
        assert_eq!(names(&queue), ["a", "c", "d"]);
        assert!(queue.remove(|r| r.key == key("zz")).is_none());
        queue.push(request("e"));
        assert_eq!(names(&queue), ["a", "c", "d", "e"]);
    }

    #[test]
    fn removal_works_across_the_ring_wrap() {
        let mut queue = RingQueue::new(4, 8);
        for name in ["a", "b", "c"] {
            queue.push(request(name));
        }
        assert_eq!(queue.pop().unwrap().key, key("a"));
        queue.push(request("d"));
        assert_eq!(names(&queue), ["b", "c", "d"]);
        // The last entry is at index 3 (the ring has wrapped).
        assert_eq!(queue.remove(|r| r.key == key("d")).unwrap().key, key("d"));
        assert_eq!(names(&queue), ["b", "c"]);
        assert_eq!(queue.remove(|r| r.key == key("b")).unwrap().key, key("b"));
        assert_eq!(names(&queue), ["c"]);
        queue.push(request("e"));
        queue.push(request("f"));
        assert_eq!(names(&queue), ["c", "e", "f"]);
    }

    #[test]
    fn a_pass_uploads_only_the_entries_ready_when_it_started() {
        let mut stream = TextureStream::new();
        for name in ["a", "b", "c"] {
            assert_eq!(stream.request(request(name)), RequestOutcome::Queued);
        }
        assert_eq!(stream.request(request("b")), RequestOutcome::AlreadyQueued);
        let mut decoded = 0;
        while stream.decode_next(|_| {
            decoded += 1;
            Some((Image::default(), false))
        }) {}
        assert_eq!(decoded, 3);
        assert_eq!(stream.pending_uploads(), 3);
        // A budget takes that many and leaves the rest for the next pass.
        let mut uploaded = Vec::new();
        stream.pump_uploads(1, |item| {
            uploaded.push(item.key.name.as_str().to_string());
        });
        assert_eq!(uploaded, ["a"]);
        assert_eq!(stream.pending_uploads(), 2);
        stream.pump_uploads(usize::MAX, |item| {
            uploaded.push(item.key.name.as_str().to_string());
        });
        assert_eq!(uploaded, ["a", "b", "c"]);
        assert!(stream.is_idle());
        assert_eq!(stream.take_finished(), vec![key("a"), key("b"), key("c")]);
    }

    #[test]
    fn promotion_cancellation_pause_and_wakes_follow_the_loader() {
        let mut stream = TextureStream::new();
        for name in ["a", "b", "c"] {
            assert_eq!(stream.request(request(name)), RequestOutcome::Queued);
        }
        assert_eq!(stream.request(request("b")), RequestOutcome::AlreadyQueued);
        assert!(stream.take_wake());
        assert!(stream.promote(&key("c")));
        assert!(!stream.promote(&key("zz")));
        assert_eq!(stream.take_next().unwrap().key, key("c"));
        assert!(stream.cancel(&key("a")));
        assert!(!stream.cancel(&key("a")));
        assert_eq!(stream.queued(), 1);
        stream.pause();
        let _ = stream.request(request("d"));
        assert!(!stream.take_wake());
        stream.resume();
        assert!(stream.take_wake());
        assert_eq!(stream.queued(), 2);
    }

    #[test]
    fn a_failed_decode_uploads_the_placeholder() {
        let mut stream = TextureStream::new();
        stream.request(request("missing"));
        assert!(stream.decode_next(|_| None));
        assert_eq!(stream.pending_uploads(), 1);
        let mut format = None;
        stream.pump_uploads(1, |item| {
            format = Some(item.image.texture_descriptor.format);
            assert_eq!(
                item.image.data.as_deref(),
                Some(&[0x40, 0x40, 0x40, 0xff][..])
            );
        });
        assert_eq!(
            format,
            Some(bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb)
        );
        assert!(stream.is_idle());
    }

    /// A release cancels the load it names, not every later load of the
    /// same texture: a re-request while the old decode runs still uploads.
    #[test]
    fn a_re_request_survives_the_cancelled_decode() {
        let mut stream = TextureStream::new();
        stream.request(request("a"));
        let old = stream.take_next().unwrap();
        assert!(stream.cancel(&key("a")));
        assert_eq!(stream.request(request("a")), RequestOutcome::Queued);
        // The old pass finishes: its upload is dropped, not the new load's.
        assert!(!stream.finish_decode(old, Some((Image::default(), false))));
        assert_eq!(stream.pending_uploads(), 0);
        let new = stream.take_next().unwrap();
        assert!(stream.promote(&key("a")));
        assert!(stream.finish_decode(new, Some((Image::default(), false))));
        assert_eq!(stream.pending_uploads(), 1);
    }

    #[test]
    fn a_cancelled_loads_end_keeps_the_re_requests_mark() {
        let mut stream = TextureStream::new();
        stream.request(request("a"));
        let old = stream.take_next().unwrap();
        assert!(stream.cancel(&key("a")));
        stream.request(request("a"));
        let new = stream.take_next().unwrap();
        // The old decode ends while the new one runs.
        assert!(!stream.finish_decode(old, None));
        assert_eq!(stream.in_hand(), 1);
        assert!(stream.promote(&key("a")));
        assert!(stream.finish_decode(new, None));
        assert_eq!(stream.in_hand(), 0);
    }

    #[test]
    fn a_priority_load_promotes_and_runs_to_upload() {
        let mut stream = TextureStream::new();
        for name in ["a", "b", "c"] {
            stream.request(request(name));
        }
        let mut uploaded = Vec::new();
        let mut order = Vec::new();
        let done = stream.load_now(
            &key("c"),
            |request| {
                order.push(request.key.name.as_str().to_string());
                Some((Image::default(), false))
            },
            |item| uploaded.push(item.key.name.as_str().to_string()),
        );
        assert!(done);
        // Only the promoted request is decoded; the rest stay queued, as the
        // original's priority path waits for this one texture alone.
        assert_eq!(order, ["c"]);
        assert_eq!(uploaded, ["c"]);
        assert_eq!(stream.queued(), 2);
        assert!(!stream.is_idle());
        // The loader cleared the record's bits when it went on: a later
        // priority load of the same, already uploaded texture finds nothing
        // ("Priority item not found.") and decodes and uploads nothing.
        let mut second_order = Vec::new();
        let mut second_upload = Vec::new();
        assert!(!stream.load_now(
            &key("c"),
            |request| {
                second_order.push(request.key.name.as_str().to_string());
                Some((Image::default(), false))
            },
            |item| second_upload.push(item.key.name.as_str().to_string()),
        ));
        assert!(second_order.is_empty());
        assert!(second_upload.is_empty());
        assert_eq!(stream.queued(), 2);
        assert!(!stream.load_now(&key("zz"), |_| None, |_| {}));
    }

    /// A priority load finds the decoded texture the loader holds while it
    /// waits for the upload pass and drains it; one already handed to the
    /// upload list is not found, and no upload pass runs for it.
    #[test]
    fn a_priority_load_drains_only_the_held_texture() {
        let mut stream = TextureStream::new();
        for name in ["a", "b"] {
            stream.request(request(name));
        }
        let decode = |_: &TextureRequest| Some((Image::default(), false));
        assert!(stream.decode_next(decode));
        assert!(stream.decode_next(decode));
        // b was decoded behind a's waiting upload: the loader holds it.
        assert!(stream.wants_to_yield());
        assert!(!stream.load_now(&key("a"), |_| None, |_| {}));
        assert_eq!(stream.pending_uploads(), 2);
        let mut uploaded = Vec::new();
        let done = stream.load_now(
            &key("b"),
            |_| None,
            |item| uploaded.push(item.key.name.as_str().to_string()),
        );
        assert!(done);
        assert_eq!(uploaded, ["a", "b"]);
        assert!(!stream.wants_to_yield());
        // It is uploaded now: not found again.
        assert!(!stream.load_now(&key("b"), |_| None, |_| {}));
    }

    #[test]
    fn a_released_load_is_dropped_wherever_it_is() {
        let mut stream = TextureStream::new();
        for name in ["uploaded", "queued", "decoding", "decoded"] {
            stream.request(request(name));
        }
        // Uploaded: nothing pending, so nothing to cancel.
        let done = stream.take_next().unwrap();
        stream.finish_decode(done, Some((Image::default(), false)));
        stream.pump_uploads(usize::MAX, |_| {});
        assert!(!stream.cancel(&key("uploaded")));
        // Queued: removed in order.
        assert!(stream.cancel(&key("queued")));
        assert_eq!(stream.queued(), 2);
        assert!(!stream.is_pending(&key("queued")));
        // Decoding: the pass finishes but its upload is dropped.
        let request = stream.take_next().unwrap();
        assert!(stream.cancel(&key("decoding")));
        assert!(!stream.finish_decode(request, Some((Image::default(), false))));
        assert_eq!(stream.pending_uploads(), 0);
        // Decoded and waiting for its upload: dropped with it.
        let request = stream.take_next().unwrap();
        assert!(stream.finish_decode(request, Some((Image::default(), false))));
        assert_eq!(stream.pending_uploads(), 1);
        assert!(stream.cancel(&key("decoded")));
        assert_eq!(stream.pending_uploads(), 0);
        assert!(stream.is_idle());
        assert!(!stream.cancel(&key("decoded")));
    }

    /// The loader waits for the main thread when it has decoded a request
    /// while earlier uploads are queued, unless the request was promoted;
    /// the upload pass lets it go on. An empty ring makes it idle, and an
    /// idle loader is not woken by the pass.
    #[test]
    fn the_loader_waits_behind_queued_uploads() {
        let mut stream = TextureStream::new();
        for name in ["a", "b", "c", "d"] {
            stream.request(request(name));
        }
        assert!(stream.take_wake());
        let decode = |_: &TextureRequest| Some((Image::default(), false));
        // a: no earlier upload, so the loader goes straight on.
        assert!(stream.decode_next(decode));
        assert!(!stream.wants_to_yield());
        assert!(stream.can_decode());
        // b: a's upload is waiting, so the loader holds b.
        assert!(stream.decode_next(decode));
        assert!(stream.wants_to_yield());
        assert!(!stream.can_decode());
        // Pausing does not change it.
        stream.pause();
        assert!(stream.wants_to_yield());
        stream.resume();
        assert!(stream.take_wake());
        // The pass uploads a and b in order and wakes the loader.
        let mut order = Vec::new();
        stream.pump_uploads(usize::MAX, |item| {
            order.push(item.key.name.as_str().to_string())
        });
        assert_eq!(order, ["a", "b"]);
        assert!(!stream.wants_to_yield());
        assert!(stream.take_wake());
        // c: nothing waits; d is promoted and goes on past c's upload.
        assert!(stream.decode_next(decode));
        assert!(stream.promote(&key("d")));
        assert!(stream.decode_next(decode));
        assert!(!stream.wants_to_yield());
        // The ring is empty: the loader is idle and the pass does not wake
        // it; a new request clears the flag and wakes it.
        assert!(stream.loader_idle());
        assert!(stream.take_next().is_none());
        stream.pump_uploads(usize::MAX, |_| {});
        assert!(!stream.take_wake());
        stream.request(request("e"));
        assert!(!stream.loader_idle());
        assert!(stream.take_wake());
    }
}
