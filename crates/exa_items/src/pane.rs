//! The inventory pane: icons at pixel positions, not cells.
//!
//! A drop from outside adds the icon at the cursor minus the grab, adds its
//! bulk, and clamps the icon inside the pane. A drop from this pane or from
//! the pane linked to it only moves the icon; a linked move is not refused
//! for capacity. A limit of 0 does not refuse for size, and a capacity of 0
//! does not refuse for bulk. An item bigger than a positive limit is refused,
//! and so is dropping a container onto the pane that shows it. A shared reject
//! bit is refused. A required mask must match. An own-only pane takes only an
//! id it already lists with the low bit set. Ending a drag on another
//! pane removes the icon. The hit is the top-most icon whose rectangle holds
//! the cursor. The icon's opaque pixels are not tested.
//!
//! Evidence: the item-panel notes (accept, drop, drag ended, hit test).

/// One icon in a pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Icon {
    pub id: u32,
    pub x: i32,
    pub y: i32,
    /// Bulk counted against the pane's capacity.
    pub bulk: u32,
    /// The item's size, compared with the pane's per-item limit.
    pub item_size: u32,
    /// Icon width and height, in pixels.
    pub extent: (i32, i32),
    /// Item flags. The low bit marks an entry the pane already lists.
    pub flags: u32,
}

/// Where the drag started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Already in this pane.
    This,
    /// The pane linked to this one (a pack and the doll).
    Linked,
    /// Any other pane.
    Other,
}

/// Why a drop did not land.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reject {
    /// Larger than the pane's per-item limit.
    TooBig,
    /// The remaining capacity cannot hold the bulk.
    Full,
    /// A container dropped onto the pane that shows it.
    IntoItself,
    /// A move of an icon this pane is not holding.
    NotHeld,
    /// Shares a bit with the pane's reject mask.
    Marked,
    /// The required mask does not match the required value.
    Unwanted,
    /// The pane only takes an id it already lists with the low bit set.
    NotListed,
}

/// A free-form item pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pane {
    pub width: i32,
    pub height: i32,
    pub capacity: u32,
    pub max_item: u32,
    /// The container this pane is showing, which cannot be dropped into it.
    pub container: Option<u32>,
    /// Any shared bit with an item's flags refuses the drop.
    pub reject_flags: u32,
    /// `(flags & required_mask)` must equal `required_value`.
    pub required_mask: u32,
    pub required_value: u32,
    /// Only an id already listed with its low flag bit set may land.
    pub only_own: bool,
    icons: Vec<Icon>,
}

impl Pane {
    pub fn new(width: i32, height: i32, capacity: u32, max_item: u32) -> Self {
        Self {
            width,
            height,
            capacity,
            max_item,
            container: None,
            reject_flags: 0,
            required_mask: 0,
            required_value: 0,
            only_own: false,
            icons: Vec::new(),
        }
    }

    pub fn icons(&self) -> &[Icon] {
        &self.icons
    }

    /// Bulk of the icons currently in the pane.
    pub fn used(&self) -> u32 {
        self.icons.iter().map(|icon| icon.bulk).sum()
    }

    /// Whether this drop may land.
    pub fn accept(&self, icon: &Icon, source: Source) -> Result<(), Reject> {
        if icon.flags & self.reject_flags != 0 {
            return Err(Reject::Marked);
        }
        if self.container == Some(icon.id) {
            return Err(Reject::IntoItself);
        }
        if self.max_item > 0 && icon.item_size > self.max_item {
            return Err(Reject::TooBig);
        }
        if source == Source::Other
            && self.capacity > 0
            && self.used().saturating_add(icon.bulk) > self.capacity
        {
            return Err(Reject::Full);
        }
        if icon.flags & self.required_mask != self.required_value {
            return Err(Reject::Unwanted);
        }
        if self.only_own
            && !self
                .icons
                .iter()
                .any(|held| held.flags & 1 != 0 && held.id == icon.id)
        {
            return Err(Reject::NotListed);
        }
        Ok(())
    }

    /// Land a drag. The icon is placed at `at` minus `grab`, then clamped.
    pub fn drop_icon(
        &mut self,
        mut icon: Icon,
        at: (i32, i32),
        grab: (i32, i32),
        source: Source,
    ) -> Result<(), Reject> {
        self.accept(&icon, source)?;
        let (x, y) = clamp(
            self.width,
            self.height,
            icon.extent,
            (at.0 - grab.0, at.1 - grab.1),
        );
        icon.x = x;
        icon.y = y;
        if source == Source::This {
            let Some(index) = self.icons.iter().position(|held| held.id == icon.id) else {
                return Err(Reject::NotHeld);
            };
            self.icons.remove(index);
        }
        self.icons.push(icon);
        Ok(())
    }

    /// The drag ended on another pane: the icon leaves, unless it is not here.
    pub fn end_drag(&mut self, id: u32) -> bool {
        let Some(index) = self.icons.iter().position(|icon| icon.id == id) else {
            return false;
        };
        self.icons.remove(index);
        true
    }

    /// The top-most icon whose rectangle holds the point.
    pub fn hit(&self, x: i32, y: i32) -> Option<u32> {
        self.icons.iter().rev().find_map(|icon| {
            let inside = x >= icon.x
                && y >= icon.y
                && x < icon.x + icon.extent.0
                && y < icon.y + icon.extent.1;
            inside.then_some(icon.id)
        })
    }
}

fn clamp(width: i32, height: i32, extent: (i32, i32), at: (i32, i32)) -> (i32, i32) {
    let max_x = (width - extent.0).max(0);
    let max_y = (height - extent.1).max(0);
    (at.0.clamp(0, max_x), at.1.clamp(0, max_y))
}

/// The release sweep's cleared flags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemRelease {
    pub flags: Vec<u32>,
    pub base_calls: u32,
}

/// Clears bit 0 of every entry's flag word.
pub fn release_items(flags: &[u32]) -> ItemRelease {
    ItemRelease {
        flags: flags.iter().map(|flag| flag & !1).collect(),
        base_calls: 1,
    }
}

/// The scale's new positions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemScale {
    pub positions: Vec<[i32; 2]>,
    pub base_calls: u32,
}

/// Divides each position by the divisor and multiplies by the unit.
pub fn scale_items(positions: &[[i32; 2]], divisor: i32, unit: i32) -> ItemScale {
    ItemScale {
        positions: positions
            .iter()
            .map(|position| {
                [
                    (f64::from(position[0]) / f64::from(divisor) * f64::from(unit))
                        .round_ties_even() as i32,
                    (f64::from(position[1]) / f64::from(divisor) * f64::from(unit))
                        .round_ties_even() as i32,
                ]
            })
            .collect(),
        base_calls: 1,
    }
}

/// The container destroy's inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContainerDestroyInput {
    pub free_flag: bool,
    pub detached: bool,
    pub count: i32,
}

/// The container destroy's calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContainerDestroy {
    pub release_calls: u32,
    pub entry_calls: i32,
    pub entry_routine: u32,
    pub array_calls: u32,
    pub base_calls: u32,
    pub final_calls: u32,
}

/// Releases the container's entries and array.
pub fn destroy_container(input: ContainerDestroyInput) -> ContainerDestroy {
    let free = u32::from(input.free_flag);
    ContainerDestroy {
        release_calls: free,
        entry_calls: input.count.max(0),
        entry_routine: if input.detached { 0x10_7a90 } else { 0x10_79e0 },
        array_calls: 1,
        base_calls: 1,
        final_calls: free,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn icon(id: u32, bulk: u32) -> Icon {
        Icon {
            id,
            x: 0,
            y: 0,
            bulk,
            item_size: 1,
            extent: (32, 32),
            flags: 0,
        }
    }

    #[test]
    fn a_pane_refuses_a_marked_item_and_keeps_a_listed_one() {
        let mut pane = Pane::new(100, 80, 10, 4);
        pane.reject_flags = 0x4;
        let mut marked = icon(1, 1);
        marked.flags = 0x4;
        assert_eq!(
            pane.drop_icon(marked, (4, 4), (0, 0), Source::Other),
            Err(Reject::Marked)
        );
        pane.reject_flags = 0;
        pane.required_mask = 0x2;
        pane.required_value = 0x2;
        assert_eq!(
            pane.drop_icon(icon(2, 1), (4, 4), (0, 0), Source::Other),
            Err(Reject::Unwanted)
        );
        let mut wanted = icon(2, 1);
        wanted.flags = 0x3;
        pane.drop_icon(wanted, (4, 4), (0, 0), Source::Other)
            .unwrap();
        pane.only_own = true;
        let mut stranger = icon(3, 1);
        stranger.flags = 0x2;
        assert_eq!(
            pane.drop_icon(stranger, (4, 4), (0, 0), Source::Other),
            Err(Reject::NotListed)
        );
        let mut listed = icon(2, 1);
        listed.flags = 0x2;
        pane.drop_icon(listed, (8, 8), (0, 0), Source::Other)
            .unwrap();
        assert_eq!(pane.icons().len(), 2);
        let mut open = Pane::new(100, 80, 0, 0);
        let mut big = icon(8, 9);
        big.item_size = 5;
        open.drop_icon(big, (4, 4), (0, 0), Source::Other).unwrap();
        assert_eq!(open.used(), 9);
    }

    #[test]
    fn a_drop_moves_an_icon_and_a_drag_removes_it() {
        let mut pane = Pane::new(100, 80, 10, 4);
        let mut heavy = icon(7, 2);
        heavy.item_size = 5;
        assert_eq!(
            pane.drop_icon(heavy, (10, 10), (0, 0), Source::Other),
            Err(Reject::TooBig)
        );
        assert!(pane.icons().is_empty());
        assert_eq!(
            pane.drop_icon(icon(7, 11), (10, 10), (0, 0), Source::Other),
            Err(Reject::Full)
        );
        pane.drop_icon(icon(7, 2), (1000, 1000), (16, 16), Source::Other)
            .unwrap();
        assert_eq!(pane.icons()[0].x, 68);
        assert_eq!(pane.icons()[0].y, 48);
        assert_eq!(pane.used(), 2);
        pane.drop_icon(icon(7, 2), (10, 12), (0, 0), Source::This)
            .unwrap();
        assert_eq!(pane.icons().len(), 1);
        assert_eq!((pane.icons()[0].x, pane.icons()[0].y), (10, 12));
        assert_eq!(pane.used(), 2);
        let mut tight = Pane::new(100, 80, 1, 4);
        assert!(
            tight
                .drop_icon(icon(3, 2), (4, 4), (0, 0), Source::Linked)
                .is_ok()
        );
        assert_eq!(tight.used(), 2);
        assert_eq!(
            tight.drop_icon(icon(4, 1), (4, 4), (0, 0), Source::Other),
            Err(Reject::Full)
        );
        tight.container = Some(9);
        assert_eq!(
            tight.drop_icon(icon(9, 1), (4, 4), (0, 0), Source::Linked),
            Err(Reject::IntoItself)
        );
        assert!(pane.end_drag(7));
        assert!(pane.icons().is_empty());
        assert!(!pane.end_drag(7));
        pane.drop_icon(icon(1, 1), (0, 0), (0, 0), Source::Other)
            .unwrap();
        pane.drop_icon(icon(2, 1), (8, 8), (0, 0), Source::Other)
            .unwrap();
        assert_eq!(pane.hit(10, 10), Some(2));
        assert_eq!(pane.hit(2, 2), Some(1));
        assert!(pane.end_drag(1));
        assert_eq!(pane.hit(2, 2), None);
        assert_eq!(pane.hit(10, 10), Some(2));
        assert_eq!(pane.hit(90, 90), None);
    }

    #[test]
    fn the_release_and_scale_sweep_the_entries() {
        let released = release_items(&[3, 2, 1]);
        assert_eq!(released.flags, vec![2, 2, 0]);
        assert_eq!(released.base_calls, 1);

        let scaled = scale_items(&[[100, 200], [50, 25]], 4, 10);
        assert_eq!(scaled.positions, vec![[250, 500], [125, 62]]);
        assert_eq!(scaled.base_calls, 1);
    }

    #[test]
    fn the_container_destroys_its_entries() {
        let freeing = destroy_container(ContainerDestroyInput {
            free_flag: true,
            detached: true,
            count: 3,
        });
        assert_eq!(freeing.release_calls, 1);
        assert_eq!(freeing.entry_calls, 3);
        assert_eq!(freeing.entry_routine, 0x10_7a90);
        assert_eq!(freeing.array_calls, 1);
        assert_eq!(freeing.base_calls, 1);
        assert_eq!(freeing.final_calls, 1);

        let quiet = destroy_container(ContainerDestroyInput {
            free_flag: false,
            detached: false,
            count: 0,
        });
        assert_eq!(quiet.release_calls, 0);
        assert_eq!(quiet.entry_calls, 0);
        assert_eq!(quiet.entry_routine, 0x10_79e0);
        assert_eq!(quiet.array_calls, 1);
        assert_eq!(quiet.base_calls, 1);
        assert_eq!(quiet.final_calls, 0);
    }
}
