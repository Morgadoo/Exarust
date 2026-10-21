//! The renderer's deferred draw queue.
//!
//! The original queues up to 1023 draws a frame, grouping them by the
//! number of lights that reach the object's sector. A draw carries its
//! sector, the object and part it draws and a word; the queue keeps one
//! chain per light count in first-seen order and the flush walks those
//! chains, binding the sector and calling the part's draw for each entry.
//!
//! Bevy's binned phases already replay a per-frame queue and bind each
//! item's data before drawing it, so Exarust has no caller for this queue
//! yet; the module keeps the original's capacity, grouping and light count
//! rules as tested functions. Rules: `docs/RENDERING.md` (Pass order).

/// The largest number of queued draws before the next append flushes.
pub const QUEUE_CAPACITY: usize = 1023;
/// The smallest light weight that counts towards a draw's bucket.
pub const LIGHT_WEIGHT: f32 = 0.004;
/// The most lights one draw's bucket counts.
pub const MAX_BUCKET_LIGHTS: u8 = 6;

/// One link of a sector's light chain: its weight, whether it is on and the
/// next link.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightLink {
    pub weight: f32,
    pub on: bool,
    pub next: Option<usize>,
}

/// A sector's lights.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SectorLights {
    pub head: Option<usize>,
    pub links: Vec<LightLink>,
}

impl SectorLights {
    /// The number of lights that reach a draw starting at `start`: it walks
    /// the chain while each link is on and weighs at least
    /// [`LIGHT_WEIGHT`], stops after [`MAX_BUCKET_LIGHTS`] links or at the
    /// chain's end, and never follows a negative index. The sector's own
    /// light count caps the walk (`light_capacity`, the renderer's field at
    /// its sector light count, itself capped at six).
    pub fn bucket_lights(&self, start: Option<usize>, light_capacity: usize) -> u8 {
        let cap = light_capacity.min(MAX_BUCKET_LIGHTS as usize);
        let mut count = 0u8;
        let mut index = start;
        while (count as usize) < cap {
            let Some(link) = index.and_then(|i| self.links.get(i)) else {
                break;
            };
            if !link.on || link.weight < LIGHT_WEIGHT || link.weight.is_nan() {
                break;
            }
            count += 1;
            index = link.next;
        }
        count
    }
}

/// One queued draw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QueuedDraw {
    /// The sector whose lights were counted.
    pub sector: u32,
    /// The object and the part within it.
    pub object: u32,
    pub part: u32,
    /// The part's word.
    pub word: u32,
    /// The bucket: the light count, 0 to 6.
    pub lights: u8,
}

/// The deferred draw queue: at most [`QUEUE_CAPACITY`] entries, one bucket
/// per light count in first-seen order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DrawQueue {
    entries: Vec<QueuedDraw>,
    sector: Option<u32>,
    /// The last bucket's light count, kept for the renderer's state after a
    /// flush.
    last_lights: Option<u8>,
}

impl DrawQueue {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the sector the next entries are queued under.
    pub fn set_sector(&mut self, sector: u32) {
        self.sector = Some(sector);
    }

    pub fn sector(&self) -> Option<u32> {
        self.sector
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The light count of the last bucket a flush drew, if any.
    pub fn last_lights(&self) -> Option<u8> {
        self.last_lights
    }

    /// Queues one draw. When the queue already holds [`QUEUE_CAPACITY`]
    /// entries the original flushes first, so the returned list is what that
    /// flush drew; otherwise it is empty.
    pub fn append(&mut self, object: u32, part: u32, word: u32, lights: u8) -> Vec<QueuedDraw> {
        let flushed = if self.entries.len() >= QUEUE_CAPACITY {
            self.flush()
        } else {
            Vec::new()
        };
        self.entries.push(QueuedDraw {
            sector: self.sector.unwrap_or(0),
            object,
            part,
            word,
            lights,
        });
        flushed
    }

    /// Draws the queue in bucket order: the first bucket a draw created
    /// first, its entries in append order, then the next bucket, and so on.
    /// The queue is emptied and its last bucket remembered.
    pub fn flush(&mut self) -> Vec<QueuedDraw> {
        let mut order: Vec<u8> = Vec::new();
        for entry in &self.entries {
            if !order.contains(&entry.lights) {
                order.push(entry.lights);
            }
        }
        let mut out = Vec::with_capacity(self.entries.len());
        for lights in &order {
            out.extend(
                self.entries
                    .iter()
                    .filter(|entry| entry.lights == *lights)
                    .copied(),
            );
        }
        self.last_lights = order.last().copied();
        self.entries.clear();
        out
    }

    /// Empties the queue without drawing and forgets the last bucket.
    pub fn reset(&mut self) {
        self.entries.clear();
        self.last_lights = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chain(links: &[(f32, bool, Option<usize>)]) -> SectorLights {
        SectorLights {
            head: if links.is_empty() { None } else { Some(0) },
            links: links
                .iter()
                .map(|&(weight, on, next)| LightLink { weight, on, next })
                .collect(),
        }
    }

    #[test]
    fn lights_count_to_the_cap_and_stop_at_a_dim_or_off_link() {
        // Four on lights all at or above the threshold.
        let lights = chain(&[
            (1.0, true, Some(1)),
            (0.004, true, Some(2)),
            (0.5, true, Some(3)),
            (0.01, true, None),
        ]);
        assert_eq!(lights.bucket_lights(Some(0), 6), 4);
        // The renderer's own count caps the walk at six.
        let many = chain(&[
            (1.0, true, Some(1)),
            (1.0, true, Some(2)),
            (1.0, true, Some(3)),
            (1.0, true, Some(4)),
            (1.0, true, Some(5)),
            (1.0, true, Some(6)),
            (1.0, true, None),
        ]);
        assert_eq!(many.bucket_lights(Some(0), 99), 6);
        assert_eq!(many.bucket_lights(Some(0), 2), 2);
        // A link below 0.004 breaks before counting; so does an off link.
        assert_eq!(
            chain(&[(0.0039, true, Some(1))]).bucket_lights(Some(0), 6),
            0
        );
        assert_eq!(chain(&[(1.0, false, Some(1))]).bucket_lights(Some(0), 6), 0);
        // A negative/absent link ends the walk.
        assert_eq!(chain(&[(1.0, true, None)]).bucket_lights(Some(0), 6), 1);
        assert_eq!(chain(&[]).bucket_lights(Some(0), 6), 0);
    }

    #[test]
    fn the_queue_groups_by_first_seen_light_count() {
        let mut queue = DrawQueue::new();
        queue.set_sector(7);
        queue.append(1, 0, 0x11, 2);
        queue.append(2, 1, 0x22, 0);
        queue.append(3, 0, 0x33, 2);
        queue.append(4, 0, 0x44, 1);
        let draws = queue.flush();
        assert_eq!(
            draws.iter().map(|d| d.object).collect::<Vec<_>>(),
            [1, 3, 2, 4]
        );
        assert!(draws.iter().all(|d| d.sector == 7));
        assert_eq!(queue.last_lights(), Some(1));
        assert!(queue.is_empty());
    }

    #[test]
    fn the_queue_flushes_before_over_capacity() {
        let mut queue = DrawQueue::new();
        for i in 0..QUEUE_CAPACITY {
            assert!(queue.append(i as u32, 0, 0, (i % 3) as u8).is_empty());
        }
        assert_eq!(queue.len(), QUEUE_CAPACITY);
        let flushed = queue.append(9999, 0, 0, 0);
        assert_eq!(flushed.len(), QUEUE_CAPACITY);
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.flush()[0].object, 9999);
        queue.reset();
        assert!(queue.is_empty() && queue.last_lights().is_none());
    }
}
