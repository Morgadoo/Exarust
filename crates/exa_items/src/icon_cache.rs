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

/// The manager's icon-atlas flags.
pub const MANAGER_ICON_FLAGS: u32 = 0x103;
/// The manager's atlas texture flags.
pub const MANAGER_TEXTURE_FLAGS: u32 = 0x100e608;
/// An icon's size in units.
pub const MANAGER_ICON_UNITS: i32 = 40;
/// The apparel texture size at shade quality 3 and above.
pub const APPAREL_SIZE_HIGH: i32 = 0x400;
/// The apparel texture size below it.
pub const APPAREL_SIZE_LOW: i32 = 0x200;
/// The RFC reader's two flags after the manager's constructor.
pub const MANAGER_RFC_FLAGS: [u8; 2] = [1, 1];

/// The manager's construction: the unit, the atlas flags and sizes, the
/// apparel painter's texture size and the RFC reader's flags. The original
/// makes the object database access, the shared apparel painter, the item
/// renderer, an icon atlas with [`MANAGER_ICON_FLAGS`] and an icon size of
/// 40 units, and a reader whose scene is the renderer's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManagerSetup {
    pub unit: i32,
    pub icon_flags: u32,
    pub texture_flags: u32,
    pub icon_size: i32,
    pub apparel_size: i32,
    pub rfc_flags: [u8; 2],
}

impl ManagerSetup {
    /// The manager as its constructor makes it: `shade_quality` 3 and above
    /// paints apparel at 1024, below it at 512.
    pub fn new(unit: i32, shade_quality: i32) -> Self {
        Self {
            unit,
            icon_flags: MANAGER_ICON_FLAGS,
            texture_flags: MANAGER_TEXTURE_FLAGS,
            icon_size: MANAGER_ICON_UNITS * unit,
            apparel_size: if shade_quality >= 3 {
                APPAREL_SIZE_HIGH
            } else {
                APPAREL_SIZE_LOW
            },
            rfc_flags: MANAGER_RFC_FLAGS,
        }
    }
}

/// Where a user item id is looked up: its top two bits tag the database.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserItemSource {
    /// The level's objects (bit 31).
    Level,
    /// The world's objects (bit 30).
    World,
    /// The game's objects (neither tag).
    Game,
}

/// The part of a user item id past its database tag.
pub fn user_item_id(id: u32) -> u32 {
    id & 0x3fff_ffff
}

/// The database a user item's tag names.
pub fn user_item_source(id: u32) -> UserItemSource {
    if id & 0x8000_0000 != 0 {
        UserItemSource::Level
    } else if id & 0x4000_0000 != 0 {
        UserItemSource::World
    } else {
        UserItemSource::Game
    }
}

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

/// The key an icon resolved through the object database takes: [`key`]
/// plus [`variant::EFFECT`] when the model carries it and
/// [`variant::STREAMED`] always.
pub fn streamed_key(variant: u32, lit_model: bool, effect: bool) -> u32 {
    let key = key(variant, lit_model);
    if effect {
        key | variant::EFFECT | variant::STREAMED
    } else {
        key | variant::STREAMED
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
        e.refs = 0;
        e.id = 0;
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
        e.refs = 0;
        e.id = 0;
        Released::Freed
    }

    /// The item an icon slot shows; 0 for slot 0.
    pub fn id_of(&self, handle: usize) -> u32 {
        handle
            .checked_sub(1)
            .and_then(|i| self.entries.get(i))
            .map_or(0, |e| e.id)
    }

    /// Where an icon centred on `(x, y)` is drawn from: half the icon size
    /// back, then its half width right and its half height up.
    pub fn draw_origin(&self, handle: usize, x: i32, y: i32, icon_size: i32) -> Option<(i32, i32)> {
        let e = self.entries.get(handle.checked_sub(1)?)?;
        let h = ((icon_size as u32) >> 1) as i32;
        Some(((x - h) + e.half_width, (y - h) - e.half_height))
    }

    /// The point an icon's pixel test reads for a cursor at `(x, y)`: the
    /// inverse of [`IconCache::draw_origin`].
    pub fn test_point(&self, handle: usize, x: i32, y: i32, icon_size: i32) -> Option<(i32, i32)> {
        let e = self.entries.get(handle.checked_sub(1)?)?;
        let h = ((icon_size as u32) >> 1) as i32;
        Some((h + x - e.half_width, h + y + e.half_height))
    }

    /// Renders a cached icon again by its handle (the debug editor's
    /// refresh): the stored id and variant go to `render`, and a success
    /// replaces the cell and both extents. False for a bad handle or a
    /// failed render.
    pub fn refresh_handle<R>(&mut self, handle: usize, mut render: R) -> bool
    where
        R: FnMut(u32, u32) -> Option<Rendered>,
    {
        let Some(entry) = handle
            .checked_sub(1)
            .and_then(|index| self.entries.get(index).copied())
        else {
            return false;
        };
        let Some(rendered) = render(entry.id, entry.variant) else {
            return false;
        };
        let (half_width, half_height) = self.halves(rendered);
        let slot = &mut self.entries[handle - 1];
        slot.cell = rendered.cell;
        slot.half_width = half_width;
        slot.half_height = half_height;
        true
    }

    /// One cached icon's draw: its atlas cell and the position the atlas is
    /// asked to draw at. The original only draws a nonzero handle, with
    /// white inks.
    pub fn draw(&self, handle: usize, at: (i32, i32)) -> Option<(i32, (i32, i32))> {
        let entry = handle.checked_sub(1).and_then(|i| self.entries.get(i))?;
        if handle == 0 {
            return None;
        }
        Some((entry.cell, at))
    }

    /// The pointed pixel test: only a nonzero handle, and only when the
    /// atlas's alpha at the point is above zero.
    pub fn pixel_test(
        &self,
        handle: usize,
        point: (i32, i32),
        alpha_at: impl Fn(i32, i32) -> f32,
    ) -> bool {
        handle != 0 && handle <= self.entries.len() && alpha_at(point.0, point.1) > 0.0
    }

    /// The loose pixel test: the atlas's own second test, which decides
    /// from its cell's coverage; a zero handle never passes.
    pub fn loose_pixel_test(
        &self,
        handle: usize,
        point: (i32, i32),
        covered: impl Fn(i32, i32) -> bool,
    ) -> bool {
        handle != 0 && handle <= self.entries.len() && covered(point.0, point.1)
    }

    /// The manager's destructor: the entries and the two renderer objects
    /// go (the original frees the item renderer and the atlas; Exarust's
    /// atlas-free cache just drops its entries).
    pub fn destroy(&mut self) {
        self.clear();
        self.errors.clear();
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
        assert_eq!(streamed_key(0x0037_0000, true, false), 0x3037_0000);
        assert_eq!(streamed_key(0, false, true), 0x9000_0000);
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
        assert_eq!(
            c.draw_origin(1, 100, 100, 160),
            Some((100 - 80 + 108, 100 - 80 - 68))
        );
        assert_eq!(
            c.test_point(1, 148, -48, 160),
            Some((80 + 148 - 108, 80 - 48 + 68))
        );
        assert_eq!(c.draw_origin(0, 0, 0, 160), None);
        c.clear();
        assert!(c.entries.is_empty());
        assert_eq!(icon_size(4), 160);
    }

    #[test]
    fn the_manager_is_built_for_forty_unit_icons() {
        let setup = ManagerSetup::new(5, 3);
        assert_eq!(setup.icon_flags, 0x103);
        assert_eq!(setup.texture_flags, 0x100e608);
        assert_eq!(setup.icon_size, 200);
        assert_eq!(setup.apparel_size, 0x400);
        assert_eq!(setup.rfc_flags, [1, 1]);
        assert_eq!(ManagerSetup::new(4, 2).apparel_size, 0x200);
        assert_eq!(MANAGER_ICON_UNITS, 40);
    }

    #[test]
    fn a_user_item_id_names_its_database() {
        assert_eq!(user_item_id(0x8000_0012), 0x12);
        assert_eq!(user_item_id(0x4000_00ff), 0xff);
        assert_eq!(user_item_id(0x0123_4567), 0x0123_4567);
        assert_eq!(user_item_source(0x8000_0001), UserItemSource::Level);
        assert_eq!(user_item_source(0x4000_0001), UserItemSource::World);
        assert_eq!(user_item_source(1), UserItemSource::Game);
        assert_eq!(user_item_source(0xc000_0001), UserItemSource::Level);
    }

    #[test]
    fn a_handle_refresh_replaces_the_cell_and_extents() {
        let mut c = IconCache::new(4);
        c.acquire(7, 0, Some(false), render(1));
        assert!(c.refresh_handle(1, |id, variant| {
            assert_eq!((id, variant), (7, 0));
            Some(Rendered {
                cell: 8,
                width: 200.0,
                height: 100.0,
            })
        }));
        assert_eq!(
            (
                c.entries[0].cell,
                c.entries[0].half_width,
                c.entries[0].half_height
            ),
            (8, 104, 54)
        );
        assert!(!c.refresh_handle(2, |_, _| Some(Rendered {
            cell: 1,
            width: 100.0,
            height: 61.0
        })));
        assert!(!c.refresh_handle(1, |_, _| None));
    }

    #[test]
    fn a_freed_slot_keeps_its_cell_variant_and_halves() {
        let mut c = IconCache::new(4);
        c.acquire(7, 0x0100_0000, Some(true), render(9));
        let before = c.entries[0];
        let mut handle = 1;
        assert_eq!(c.release(&mut handle), Released::Freed);
        let after = c.entries[0];
        assert_eq!(after.id, 0);
        assert_eq!(after.refs, 0);
        assert_eq!(
            (
                after.cell,
                after.variant,
                after.half_width,
                after.half_height
            ),
            (
                before.cell,
                before.variant,
                before.half_width,
                before.half_height
            )
        );
        // Drawing and the two pixel tests still need a live handle.
        assert_eq!(c.draw(0, (1, 2)), None);
        assert!(!c.pixel_test(0, (0, 0), |_, _| 1.0));
        assert!(!c.loose_pixel_test(0, (0, 0), |_, _| true));
        assert_eq!(c.draw(1, (1, 2)), Some((9, (1, 2))));
        assert!(c.pixel_test(1, (0, 0), |_, _| 0.5));
        assert!(!c.pixel_test(1, (0, 0), |_, _| 0.0));
        assert!(c.loose_pixel_test(1, (0, 0), |_, _| true));
        assert!(!c.loose_pixel_test(9, (0, 0), |_, _| true));
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
