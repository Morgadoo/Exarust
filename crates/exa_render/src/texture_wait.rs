//! The material loader's wait for its maps' texture uploads, before the
//! parallax pass reads the height map's size.
//!
//! The loader's parallax step starts the material's maps uploading, then
//! spins until none of the leading slots is still uploading: a slot's word
//! bit 1 means the driver has not finished. It scans the material's map
//! slots from slot 0 while they exist, but never past the fifth (`0..=4` —
//! the loop's bound is a five-slot window), and its poll rounds cover the
//! slots it found occupied. When no slot is left uploading the round ends
//! the spin and one last poll runs.
//!
//! Exarust loads through Bevy's asset server, which hands the loader only
//! finished images, so the spin has no counterpart. The scan and the poll
//! order are kept here, tested, because they decide which maps the parallax
//! pass guarantees before it reads a size.

/// The map slots the wait looks at, at most.
pub const SCANNED_SLOTS: usize = 5;

/// One of a material record's map slots.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MapSlot {
    /// The slot holds a texture.
    pub present: bool,
    /// The texture's word bit 1: the driver is still uploading it.
    pub uploading: bool,
}

impl MapSlot {
    /// A slot with a finished texture.
    pub fn ready() -> Self {
        Self {
            present: true,
            uploading: false,
        }
    }

    /// A slot whose texture is still uploading.
    pub fn uploading() -> Self {
        Self {
            present: true,
            uploading: true,
        }
    }
}

/// The leading occupied slots the wait scans: from slot 0 while it holds a
/// texture, and never more than [`SCANNED_SLOTS`].
///
/// The engine's loop tests the slot, then stops on the first missing one,
/// with a bound of five iterations (`while index < 4` after incrementing).
pub fn scanned(slots: &[MapSlot]) -> usize {
    let mut count = 0;
    for slot in slots.iter().take(SCANNED_SLOTS) {
        if !slot.present {
            break;
        }
        count += 1;
    }
    count
}

/// The slots the wait starts uploading, in slot order.
pub fn started(slots: &[MapSlot]) -> Vec<usize> {
    (0..scanned(slots))
        .filter(|&i| slots[i].uploading)
        .collect()
}

/// One poll round's slot: the first slot still uploading among the slots
/// the wait found occupied, or `None` when the round finds none — which
/// ends the spin.
///
/// Each round polls one uploading slot, yields to the scheduler, and starts
/// again from slot 0; the engine's `Sleep(0)` is the yield.
pub fn poll(slots: &[MapSlot]) -> Option<usize> {
    let count = scanned(slots);
    (0..count).find(|&i| slots[i].uploading)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scan_stops_at_the_first_missing_slot() {
        assert_eq!(scanned(&[]), 0);
        assert_eq!(scanned(&[MapSlot::default()]), 0);
        assert_eq!(
            scanned(&[
                MapSlot::ready(),
                MapSlot::ready(),
                MapSlot::default(),
                MapSlot::ready()
            ]),
            2
        );
        // More than five ready slots still scan only the first five.
        let many = vec![MapSlot::ready(); 9];
        assert_eq!(scanned(&many), SCANNED_SLOTS);
        assert_eq!(started(&many), Vec::<usize>::new());
    }

    #[test]
    fn only_uploading_slots_are_started_and_polled() {
        let slots = [MapSlot::uploading(), MapSlot::ready(), MapSlot::uploading()];
        assert_eq!(started(&slots), vec![0, 2]);
        assert_eq!(poll(&slots), Some(0));
        // Once the first finishes, the round moves on to the next.
        let slots = [MapSlot::ready(), MapSlot::ready(), MapSlot::uploading()];
        assert_eq!(poll(&slots), Some(2));
        // All finished: the round finds nothing and the spin ends.
        let slots = [MapSlot::ready(), MapSlot::ready()];
        assert_eq!(poll(&slots), None);
        assert_eq!(poll(&[]), None);
        // A slot past the scanned window is never polled.
        let mut slots = vec![MapSlot::ready(); SCANNED_SLOTS];
        slots.push(MapSlot::uploading());
        assert!(started(&slots).is_empty());
        assert_eq!(poll(&slots), None);
    }
}
