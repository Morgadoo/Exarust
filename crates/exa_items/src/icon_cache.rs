//! The item icon cache: one rendered icon per item and variant, shared by
//! every panel that shows the item, counted by reference.
//!
//! An icon is asked for by item id and variant flags. An item whose model
//! is lit keeps only the apparel-framing bit and its condition byte (bits
//! 16–23) and gains the lit bit; any other keeps only the framing bit. A
//! match counts another reference and returns the 1-based slot; a miss
//! renders the icon into the first free slot (or a new one) with half
//! extents of half the rendered size plus one GUI unit. A failed render
//! returns 0. Releasing the last reference frees the atlas cell and the
//! slot; releasing past it reports "Item reference underflow.". A level or
//! save load clears the cache. Evidence: the icon cache notes
//! (`context/re/item-gui-lane.md`).

/// Variant bits.
pub mod variant {
    /// Framed as apparel (a closer camera).
    pub const APPAREL: u32 = 0x4000_0000;
    /// A lit model with its condition byte in bits 16–23.
    pub const LIT: u32 = 0x2000_0000;
    /// An icon built from a streamed object.
    pub const STREAMED: u32 = 0x1000_0000;
    /// The model carries an extra effect.
    pub const EFFECT: u32 = 0x8000_0000;
    /// The item is a weapon (its light preset).
    pub const WEAPON: u32 = 0x0400_0000;
    /// The halo is drawn highlighted.
    pub const HIGHLIGHT: u32 = 0x0100_0000;
    /// The condition byte of a lit variant.
    pub const CONDITION: u32 = 0x00FF_0000;
}

/// The message a release past the last reference reports.
pub const UNDERFLOW: &str = "Item reference underflow.";

/// The cache key of an icon: an item lit by its model's flags keeps the
/// framing bit and its condition and gains [`variant::LIT`]; any other
/// keeps only the framing bit.
pub fn key(variant: u32, lit_model: bool) -> u32 {
    if lit_model {
        variant & (variant::APPAREL | variant::CONDITION) | variant::LIT
    } else {
        variant & variant::APPAREL
    }
}

/// What the renderer gives for one icon: its atlas cell and its rendered
/// size in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rendered {
    pub cell: i32,
    pub width: f32,
    pub height: f32,
}

/// One cached icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Entry {
    pub id: u32,
    pub cell: i32,
    pub variant: u32,
    pub half_width: i32,
    pub half_height: i32,
    pub refs: i32,
}

/// What a release did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Released {
    /// The handle was 0 or out of range.
    Nothing,
    /// Other references remain.
    Kept,
    /// The last reference went: the cell and the slot are free.
    Freed,
}

/// The cache.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct IconCache {
    pub entries: Vec<Entry>,
    /// The GUI unit, added to each half extent.
    pub unit: i32,
    /// Messages for the error box.
    pub errors: Vec<String>,
}

impl IconCache {
    pub fn new(unit: i32) -> Self {
        Self {
            entries: Vec::new(),
            unit,
            errors: Vec::new(),
        }
    }

    fn halves(&self, r: Rendered) -> (i32, i32) {
        (
            self.unit + (r.width * 0.5).round_ties_even() as i32,
            (r.height * 0.5).round_ties_even() as i32 + self.unit,
        )
    }

    /// The slot of an icon, 1-based, rendering it on a miss; 0 when it
    /// cannot be rendered or the item has no model. `lit_model` is `None`
    /// for an item without a model.
    pub fn acquire<R>(&mut self, id: u32, variant: u32, lit_model: Option<bool>, render: R) -> usize
    where
        R: FnOnce(u32, u32) -> Option<Rendered>,
    {
        let Some(lit) = lit_model else {
            return 0;
        };
        let k = key(variant, lit);
        let mut free = None;
        for (i, e) in self.entries.iter_mut().enumerate() {
            if e.id == id && e.variant == k {
                e.refs += 1;
                return i + 1;
            }
            if free.is_none() && e.id == 0 {
                free = Some(i);
            }
        }
        let Some(r) = render(id, k) else {
            return 0;
        };
        let at = free.unwrap_or_else(|| {
            self.entries.push(Entry::default());
            self.entries.len() - 1
        });
        let (half_width, half_height) = self.halves(r);
        self.entries[at] = Entry {
            id,
            cell: r.cell,
            variant: k,
            half_width,
            half_height,
            refs: 1,
        };
        at + 1
    }

    /// Renders a cached icon again, or acquires it when it is not cached.
    pub fn refresh<R>(&mut self, id: u32, variant: u32, lit_model: Option<bool>, render: R) -> usize
    where
        R: FnOnce(u32, u32) -> Option<Rendered>,
    {
        let Some(lit) = lit_model else {
            return 0;
        };
        let k = key(variant, lit);
        match self
            .entries
            .iter()
            .position(|e| e.id == id && e.variant == k)
        {
            Some(at) => match render(id, k) {
                Some(_) => at + 1,
                None => 0,
            },
            None => self.acquire(id, variant, Some(lit), render),
        }
    }

    /// Renders every cached icon of `id` again; returns how many rendered.
    pub fn refresh_all<R>(&mut self, id: u32, mut render: R) -> usize
    where
        R: FnMut(u32, u32) -> Option<Rendered>,
    {
        self.entries
            .iter()
            .filter(|e| e.id == id)
            .filter(|e| render(e.id, e.variant).is_some())
            .count()
    }

    /// Adds a reference to slot `handle`; returns the new count, 0 for a
    /// bad handle.
    pub fn add_ref(&mut self, handle: usize) -> i32 {
        match handle.checked_sub(1).and_then(|i| self.entries.get_mut(i)) {
            Some(e) => {
                e.refs += 1;
                e.refs
            }
            None => 0,
        }
    }

    /// Releases slot `*handle` and clears the handle.
    pub fn release(&mut self, handle: &mut usize) -> Released {
        let Some(e) = handle.checked_sub(1).and_then(|i| self.entries.get_mut(i)) else {
            return Released::Nothing;
        };
        e.refs -= 1;
        *handle = 0;
        if e.refs >= 1 {
            return Released::Kept;
        }
        let underflow = e.refs < 0;
        *e = Entry {
            cell: e.cell,
            ..Entry::default()
        };
        if underflow {
            self.errors.push(UNDERFLOW.to_string());
        }
        Released::Freed
    }

    /// Releases slot `*handle` only when it is a lit variant. The original
    /// takes the last slot as out of range here.
    pub fn release_lit(&mut self, handle: &mut usize) -> Released {
        if *handle == 0 || *handle >= self.entries.len() {
            return Released::Nothing;
        }
        if self.entries[*handle - 1].variant & variant::LIT == 0 {
            return Released::Nothing;
        }
        let e = &mut self.entries[*handle - 1];
        e.refs -= 1;
        *handle = 0;
        if e.refs >= 1 {
            return Released::Kept;
        }
        *e = Entry {
            cell: e.cell,
            ..Entry::default()
        };
        Released::Freed
    }

    /// The item an icon slot shows; 0 for slot 0.
    pub fn id_of(&self, handle: usize) -> u32 {
        handle
            .checked_sub(1)
            .and_then(|i| self.entries.get(i))
            .map_or(0, |e| e.id)
    }

    /// A level or save load empties the cache.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// A UI-scale change: every icon is rendered again at the new size and
    /// its half extents follow.
    pub fn rescale<R>(&mut self, unit: i32, mut render: R)
    where
        R: FnMut(u32, u32) -> Option<Rendered>,
    {
        self.unit = unit;
        for i in 0..self.entries.len() {
            let e = self.entries[i];
            if let Some(r) = render(e.id, e.variant) {
                let (hw, hh) = self.halves(r);
                let e = &mut self.entries[i];
                e.cell = r.cell;
                e.half_width = hw;
                e.half_height = hh;
            }
        }
    }
}

/// The icon size for GUI unit `unit`: 40 units.
pub fn icon_size(unit: i32) -> i32 {
    40 * unit
}

/// Why an item could not be shown, as the original's error box words it.
/// `found` is the record's size and display name when the id's database
/// (the level's, the world's or the game's, by its tag bits) has it.
pub fn user_item_error(id: u32, found: Option<(u32, String)>) -> Option<String> {
    if id == 0 {
        return None;
    }
    Some(match found {
        None => format!("User item does not exist: {id:08X}"),
        Some((size, _)) if size < 0x19 => format!("Invalid User Item: {id:08X}"),
        Some((_, name)) => format!("Failed to load user item: {name}"),
    })
}

/// The error box message of an item that could not be resolved for a
/// panel.
pub fn unresolved(id: u32) -> Option<String> {
    (id != 0).then(|| format!("Unable to resolve item: {id:08X}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(cell: i32) -> impl FnOnce(u32, u32) -> Option<Rendered> {
        move |_, _| {
            Some(Rendered {
                cell,
                width: 100.0,
                height: 61.0,
            })
        }
    }

    #[test]
    fn the_key_keeps_framing_and_a_lit_condition() {
        assert_eq!(key(0xC4FF_1234, false), variant::APPAREL);
        assert_eq!(key(0x0037_0000, true), 0x2037_0000);
        assert_eq!(key(0x5000_0001, true), 0x6000_0000);
    }

    #[test]
    fn icons_are_shared_and_counted() {
        let mut c = IconCache::new(4);
        assert_eq!(c.acquire(7, 0, None, render(1)), 0);
        let a = c.acquire(7, 0, Some(false), render(1));
        assert_eq!(a, 1);
        assert_eq!(
            (c.entries[0].half_width, c.entries[0].half_height),
            (54, 34)
        );
        // The same item and variant: no render, another reference.
        assert_eq!(c.acquire(7, 0x0100_0000, Some(false), |_, _| None), 1);
        assert_eq!(c.entries[0].refs, 2);
        let b = c.acquire(8, 0, Some(false), render(2));
        assert_eq!(b, 2);
        assert_eq!(c.acquire(9, 0, Some(false), |_, _| None), 0);
        let mut h = a;
        assert_eq!(c.release(&mut h), Released::Kept);
        assert_eq!(h, 0);
        let mut h = a;
        assert_eq!(c.release(&mut h), Released::Freed);
        assert_eq!(c.id_of(1), 0);
        // The freed slot is reused.
        assert_eq!(c.acquire(10, 0, Some(false), render(3)), 1);
        assert_eq!(c.add_ref(2), 2);
        assert_eq!(c.add_ref(9), 0);
        assert_eq!(c.id_of(2), 8);
        let mut bad = 0;
        assert_eq!(c.release(&mut bad), Released::Nothing);
    }

    #[test]
    fn releasing_past_the_last_reference_reports_it() {
        let mut c = IconCache::new(4);
        c.acquire(7, 0, Some(false), render(1));
        c.entries[0].refs = 0;
        let mut h = 1;
        assert_eq!(c.release(&mut h), Released::Freed);
        assert_eq!(c.errors, vec![UNDERFLOW.to_string()]);
    }

    #[test]
    fn only_lit_icons_release_by_condition() {
        let mut c = IconCache::new(4);
        c.acquire(7, 0, Some(false), render(1));
        c.acquire(8, 0x0012_0000, Some(true), render(2));
        c.acquire(9, 0, Some(true), render(3));
        let mut plain = 1;
        assert_eq!(c.release_lit(&mut plain), Released::Nothing);
        let mut lit = 2;
        assert_eq!(c.release_lit(&mut lit), Released::Freed);
        // The last slot is out of range for this release.
        let mut last = 3;
        assert_eq!(c.release_lit(&mut last), Released::Nothing);
        assert_eq!(last, 3);
    }

    #[test]
    fn refreshing_and_rescaling() {
        let mut c = IconCache::new(4);
        c.acquire(7, 0, Some(false), render(1));
        assert_eq!(c.refresh(7, 0, Some(false), |_, _| None), 0);
        assert_eq!(c.refresh(7, 0, Some(false), render(1)), 1);
        assert_eq!(c.refresh(8, 0, Some(false), render(2)), 2);
        assert_eq!(
            c.refresh_all(7, |_, _| Some(Rendered {
                cell: 5,
                width: 0.0,
                height: 0.0
            })),
            1
        );
        c.rescale(8, |_, _| {
            Some(Rendered {
                cell: 9,
                width: 200.0,
                height: 120.0,
            })
        });
        assert_eq!(
            c.entries[0],
            Entry {
                id: 7,
                cell: 9,
                variant: 0,
                half_width: 108,
                half_height: 68,
                refs: 1
            }
        );
        c.clear();
        assert!(c.entries.is_empty());
        assert_eq!(icon_size(4), 160);
    }

    #[test]
    fn the_error_box_messages() {
        assert_eq!(user_item_error(0, None), None);
        assert_eq!(
            user_item_error(0x8000_0012, None).unwrap(),
            "User item does not exist: 80000012"
        );
        assert_eq!(
            user_item_error(0x12, Some((8, String::new()))).unwrap(),
            "Invalid User Item: 00000012"
        );
        assert_eq!(
            user_item_error(0x12, Some((40, "Hatchet A01".into()))).unwrap(),
            "Failed to load user item: Hatchet A01"
        );
        assert_eq!(
            unresolved(0x2A).unwrap(),
            "Unable to resolve item: 0000002A"
        );
        assert_eq!(unresolved(0), None);
    }
}
