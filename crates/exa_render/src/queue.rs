//! The renderer's deferred draw queue.
//!
//! The original queues up to 1023 draws a frame, each tagged with the number
//! of lights that reach its sector (its bucket). The queue is a linked list
//! kept grouped by bucket as draws arrive, and the flush walks it from the
//! head, binding each entry's sector and drawing the sector's item.
//!
//! Bevy's binned phases already replay a per-frame queue and bind each
//! item's data before drawing it, so Exarust has no caller for this queue
//! yet; the module keeps the original's capacity, insertion and light count
//! rules as tested functions. Rules: `docs/RENDERING.md` (Pass order).

/// The largest number of queued draws before the next append flushes.
pub const QUEUE_CAPACITY: usize = 1023;
/// The smallest light weight that counts towards a draw's bucket.
pub const LIGHT_WEIGHT: f64 = 0.004;
/// The most lights one draw's bucket counts.
pub const MAX_BUCKET_LIGHTS: u8 = 6;

/// One link of a sector's light chain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightLink {
    pub weight: f32,
    /// The link's enabled field is non-zero. A link without it ends the
    /// count.
    pub enabled: bool,
    /// The next link's index, or `None` when the original stores a negative
    /// one.
    pub next: Option<usize>,
}

/// A sector's lights.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SectorLights {
    pub head: Option<usize>,
    pub links: Vec<LightLink>,
}

impl SectorLights {
    /// The number of lights that reach a draw: it walks the chain from the
    /// sector's head while each link is enabled and its weight (widened to
    /// double) is not below [`LIGHT_WEIGHT`] (a NaN weight is not below it,
    /// so it counts), and stops at the chain's end or when the count
    /// reaches the renderer's light capacity capped at [`MAX_BUCKET_LIGHTS`].
    pub fn bucket_lights(&self, light_capacity: i32) -> u8 {
        let cap = light_capacity.min(i32::from(MAX_BUCKET_LIGHTS));
        let mut count = 0u8;
        let mut index = self.head;
        while i32::from(count) < cap {
            let Some(link) = index.and_then(|i| self.links.get(i)) else {
                break;
            };
            if !link.enabled || f64::from(link.weight) < LIGHT_WEIGHT {
                break;
            }
            count += 1;
            index = link.next;
        }
        count
    }
}

/// One queued draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueuedDraw {
    /// The sector the draw belongs to.
    pub sector: u32,
    /// The item within the sector that is drawn.
    pub item: i32,
    /// The bucket: the sector's light count, 0 to 6.
    pub lights: u8,
    /// The draw's 16-bit word.
    pub word: u16,
}

/// The deferred draw queue: at most [`QUEUE_CAPACITY`] entries in list order.
///
/// A new entry goes to the head when its bucket is the one the renderer's
/// lights were last set up for (the bucket of the last entry of the previous
/// flush; the setting is then forgotten until the next flush). Otherwise it
/// goes right after the first entry that has a successor and the same
/// bucket, or at the end when none does. The first entry of an empty queue
/// is simply queued.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DrawQueue {
    entries: Vec<QueuedDraw>,
    lit_for: Option<u8>,
}

impl DrawQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The bucket the lights are currently set up for, if any.
    pub fn lit_for(&self) -> Option<u8> {
        self.lit_for
    }

    /// The queued entries in the order a flush draws them.
    pub fn entries(&self) -> &[QueuedDraw] {
        &self.entries
    }

    /// Queues one draw. When the queue already holds [`QUEUE_CAPACITY`]
    /// entries the original flushes first, so the returned list is what that
    /// flush drew; otherwise it is empty.
    pub fn append(&mut self, draw: QueuedDraw) -> Vec<QueuedDraw> {
        let flushed = if self.entries.len() >= QUEUE_CAPACITY {
            self.flush()
        } else {
            Vec::new()
        };
        if self.entries.is_empty() {
            self.entries.push(draw);
        } else if self.lit_for == Some(draw.lights) {
            self.lit_for = None;
            self.entries.insert(0, draw);
        } else {
            let last = self.entries.len() - 1;
            let mut at = 0;
            while at < last && self.entries[at].lights != draw.lights {
                at += 1;
            }
            self.entries.insert(at + 1, draw);
        }
        flushed
    }

    /// Draws the queue in list order, empties it, and remembers the last
    /// entry's bucket as the one the lights are set up for. Flushing an
    /// empty queue changes nothing.
    pub fn flush(&mut self) -> Vec<QueuedDraw> {
        if let Some(last) = self.entries.last() {
            self.lit_for = Some(last.lights);
        }
        std::mem::take(&mut self.entries)
    }

    /// Empties the queue without drawing; the lights' bucket is kept.
    pub fn reset(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link(weight: f32, enabled: bool, next: Option<usize>) -> LightLink {
        LightLink {
            weight,
            enabled,
            next,
        }
    }

    fn sector(links: Vec<LightLink>) -> SectorLights {
        SectorLights {
            head: if links.is_empty() { None } else { Some(0) },
            links,
        }
    }

    fn draw(item: i32, lights: u8) -> QueuedDraw {
        QueuedDraw {
            sector: 7,
            item,
            lights,
            word: 0,
        }
    }

    fn items(queue: &DrawQueue) -> Vec<i32> {
        queue.entries().iter().map(|d| d.item).collect()
    }

    #[test]
    fn lights_count_to_the_cap_and_stop_at_a_dim_or_disabled_link() {
        // Four enabled lights at or above the threshold (0.004 as an f32 is
        // above the double 0.004, so it counts).
        let lights = sector(vec![
            link(1.0, true, Some(1)),
            link(0.004, true, Some(2)),
            link(0.5, true, Some(3)),
            link(0.01, true, None),
        ]);
        assert_eq!(lights.bucket_lights(6), 4);
        // The renderer's own count caps the walk at six.
        let many = sector(vec![
            link(1.0, true, Some(1)),
            link(1.0, true, Some(2)),
            link(1.0, true, Some(3)),
            link(1.0, true, Some(4)),
            link(1.0, true, Some(5)),
            link(1.0, true, Some(6)),
            link(1.0, true, None),
        ]);
        assert_eq!(many.bucket_lights(99), 6);
        assert_eq!(many.bucket_lights(2), 2);
        assert_eq!(many.bucket_lights(0), 0);
        assert_eq!(many.bucket_lights(-1), 0);
        // A link below 0.004 breaks before counting; so does a disabled one.
        assert_eq!(
            sector(vec![link(0.0039, true, Some(1))]).bucket_lights(6),
            0
        );
        assert_eq!(sector(vec![link(1.0, false, Some(1))]).bucket_lights(6), 0);
        // A NaN weight is not below the threshold and counts.
        assert_eq!(sector(vec![link(f32::NAN, true, None)]).bucket_lights(6), 1);
        // The chain's end, an empty sector and a missing link end the walk.
        assert_eq!(sector(vec![link(1.0, true, None)]).bucket_lights(6), 1);
        assert_eq!(sector(vec![]).bucket_lights(6), 0);
        assert_eq!(sector(vec![link(1.0, true, Some(9))]).bucket_lights(6), 1);
    }

    #[test]
    fn a_new_entry_goes_behind_the_first_of_its_bucket() {
        let mut queue = DrawQueue::new();
        queue.append(draw(1, 2));
        // The first entry is also the tail, so the next one follows it.
        queue.append(draw(2, 0));
        assert_eq!(items(&queue), [1, 2]);
        // Bucket 2 again: right after the first bucket-2 entry.
        queue.append(draw(3, 2));
        assert_eq!(items(&queue), [1, 3, 2]);
        // A new bucket goes to the end.
        queue.append(draw(4, 1));
        assert_eq!(items(&queue), [1, 3, 2, 4]);
        // A third bucket-2 entry lands before the second: the group is
        // newest first behind its first entry.
        queue.append(draw(5, 2));
        assert_eq!(items(&queue), [1, 5, 3, 2, 4]);
        // A match at the tail still inserts after it.
        queue.append(draw(6, 1));
        assert_eq!(items(&queue), [1, 5, 3, 2, 4, 6]);
        assert_eq!(queue.lit_for(), None);
    }

    #[test]
    fn the_bucket_the_lights_are_set_for_goes_first() {
        let mut queue = DrawQueue::new();
        queue.append(draw(1, 2));
        queue.append(draw(2, 1));
        let drawn = queue.flush();
        assert_eq!(drawn.iter().map(|d| d.item).collect::<Vec<_>>(), [1, 2]);
        // The last entry's bucket is what the lights are set for.
        assert_eq!(queue.lit_for(), Some(1));
        // Flushing nothing leaves it alone.
        assert!(queue.flush().is_empty());
        assert_eq!(queue.lit_for(), Some(1));
        queue.append(draw(3, 2));
        queue.append(draw(4, 1));
        // Bucket 1 matches: head of the queue, and the setting is spent.
        assert_eq!(items(&queue), [4, 3]);
        assert_eq!(queue.lit_for(), None);
        queue.append(draw(5, 1));
        assert_eq!(items(&queue), [4, 5, 3]);
        // The first entry of an empty queue does not spend the setting.
        queue.flush();
        assert_eq!(queue.lit_for(), Some(2));
        queue.reset();
        queue.append(draw(6, 0));
        assert_eq!(queue.lit_for(), Some(2));
    }

    #[test]
    fn the_queue_flushes_before_over_capacity() {
        let mut queue = DrawQueue::new();
        for i in 0..QUEUE_CAPACITY {
            // Distinct buckets are not needed: one bucket keeps every entry
            // behind the first, which is the cheapest to build.
            assert!(queue.append(draw(i as i32, 0)).is_empty());
        }
        assert_eq!(queue.len(), QUEUE_CAPACITY);
        let flushed = queue.append(draw(9999, 0));
        assert_eq!(flushed.len(), QUEUE_CAPACITY);
        assert_eq!(items(&queue), [9999]);
    }
}
