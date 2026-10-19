//! The inventory pane: icons at pixel positions, not cells.
//!
//! A drop from outside adds the icon at the cursor minus the grab, adds its
//! bulk, and clamps the icon inside the pane. A drop from this pane or from
//! the pane linked to it only moves the icon; a linked move is not refused
//! for capacity. An item bigger than the pane's limit is refused, and so is
//! dropping a container onto the pane that shows it. Ending a drag on another
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
        if self.container == Some(icon.id) {
            return Err(Reject::IntoItself);
        }
        if icon.item_size > self.max_item {
            return Err(Reject::TooBig);
        }
        if source == Source::Other && self.used().saturating_add(icon.bulk) > self.capacity {
            return Err(Reject::Full);
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
        }
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
}
