//! The trade window's four item panels and the drag between them.
//!
//! The window's constructor makes four item containers besides the
//! character preview, all of the free-form item panel
//! ([`crate::item_container`]): the player's pack, the sell basket, the buy
//! basket and the shop's stock. There are no rows and no names: each item
//! is an icon at its own place, as in every other item panel. Each panel's
//! required-flag pair decides what it takes:
//!
//! | panel | place (client) | mask / value | change callback |
//! |---|---|---|---|
//! | pack | 0, 0x180, 0xe4 × 0xb4 | 0x80000000 / 0 | none |
//! | sell basket | 0xe8, 8, 0xc4 × 0x174 | 0x80000000 / 0 | sell total |
//! | buy basket | 0x1b0, 8, 0xc4 × 0x174 | 0x80000000 / 0x80000000 | buy total |
//! | stock | right edge 0x10 in, 0, 0xe4 × 0x234 | 0x80000000 / 0x80000000 | none |
//!
//! So the shop's items (which carry [`SHOP_ITEM_FLAGS`]) go between the
//! stock and the buy basket and the player's between the pack and the sell
//! basket. The places are in the window's client: the constructor's border
//! word is 8 ([`crate::shop_window::BORDER`]), and the stock panel's 0x234
//! fills the 0x244-tall page less that border at the top and bottom.
//!
//! The opener's layout loads the pack from the player's inventory list
//! (the stored places, [`Container::load_list`]) and the stock from the
//! shelf's list with the shop's flag, laid out by the seeded grid pass
//! ([`crate::item_container::grid_layout`]) with the day counter plus one
//! as its seed. The baskets start empty.
//!
//! A basket's change callback sums the value word of each item it holds
//! (the buy basket whole, the sell basket quartered,
//! [`crate::shop_window::ShopWindow::buy_changed`]) and redoes the bottom
//! row.
//!
//! The drag is the item panel's own: a left press on an entry marks it
//! dragged with the grab at half the icon ([`crate::item_container::drag_grab`]);
//! the release over a panel asks that panel's accept test
//! ([`Container::accept`]), drops there ([`Container::drop_entry`]: the
//! same panel only repositions) and ends the source's drag
//! ([`Container::finish_drag`]). A release anywhere else, or a refusal,
//! cancels and the entry stays where it was.
//!
//! Evidence: the trade window's constructor, its opener and layout, the two
//! basket callbacks, and the item panel's accept, drop, drag-ended and
//! mouse slots (`context/re/component-map/sections/items.md`).

use crate::hub_windows::SHOP_ITEM_FLAGS;
use crate::item_container::{Cell, Container, Drag, DropDrag, ListChild, grid_layout};
use crate::shop_window::{BUY_BASKET, PACK_PANEL, SELL_BASKET, WINDOW_SIZE, stock_panel};

/// The inset a panel's entries keep from its edges, as the item panels of
/// the inventory draw them ([`crate::gui_screens::panel_client`]).
pub const PANEL_INSET: i32 = 4;

/// One of the trade window's item panels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShopPanel {
    Pack,
    Sell,
    Buy,
    Stock,
}

/// The four panels in drawing order.
pub const SHOP_PANELS: [ShopPanel; 4] = [
    ShopPanel::Pack,
    ShopPanel::Sell,
    ShopPanel::Buy,
    ShopPanel::Stock,
];

impl ShopPanel {
    /// The panel's number in a drag's source and destination fields. The
    /// original carries pointers; any distinct number serves.
    pub fn number(self) -> u32 {
        match self {
            ShopPanel::Pack => 1,
            ShopPanel::Sell => 2,
            ShopPanel::Buy => 3,
            ShopPanel::Stock => 4,
        }
    }

    /// The panel's rectangle in the window's client: x, y, width, height.
    pub fn rect(self) -> [i32; 4] {
        match self {
            ShopPanel::Pack => PACK_PANEL,
            ShopPanel::Sell => SELL_BASKET,
            ShopPanel::Buy => BUY_BASKET,
            ShopPanel::Stock => stock_panel(WINDOW_SIZE.0),
        }
    }

    /// The rectangle its entries' cells are relative to, in the window's
    /// client: the panel less [`PANEL_INSET`] all round.
    pub fn client(self) -> [i32; 4] {
        let [x, y, w, h] = self.rect();
        [
            x + PANEL_INSET,
            y + PANEL_INSET,
            w - 2 * PANEL_INSET,
            h - 2 * PANEL_INSET,
        ]
    }

    /// The required flag mask and value the constructor gives the panel.
    pub fn required(self) -> (u32, u32) {
        match self {
            ShopPanel::Pack | ShopPanel::Sell => (SHOP_ITEM_FLAGS, 0),
            ShopPanel::Buy | ShopPanel::Stock => (SHOP_ITEM_FLAGS, SHOP_ITEM_FLAGS),
        }
    }

    /// Whether the panel has a change callback: the two baskets.
    pub fn is_basket(self) -> bool {
        matches!(self, ShopPanel::Sell | ShopPanel::Buy)
    }
}

/// The drag in flight: the panel it started in and the entry it took.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShopDrag {
    pub source: ShopPanel,
    pub icon: usize,
    pub grab: Cell,
    pub id: u32,
    pub flags: u32,
    pub bulk: i32,
}

/// What a release did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShopDrop {
    /// The item moved from one panel to another.
    Moved {
        from: ShopPanel,
        to: ShopPanel,
        id: u32,
    },
    /// The entry took a new place in its own panel.
    Repositioned(ShopPanel),
    /// The release was outside every panel, or the panel refused it; the
    /// entry stays where it was.
    Cancelled,
}

/// The four item panels.
#[derive(Clone, Debug, PartialEq)]
pub struct ShopPanels {
    pub pack: Container,
    pub sell: Container,
    pub buy: Container,
    pub stock: Container,
    pub drag: Option<ShopDrag>,
}

impl Default for ShopPanels {
    fn default() -> Self {
        Self::new(4)
    }
}

impl ShopPanels {
    /// The four empty panels at GUI `unit`, with their required flags and
    /// client sizes.
    pub fn new(unit: i32) -> Self {
        let make = |panel: ShopPanel| {
            let [_, _, w, h] = panel.client();
            let (mask, value) = panel.required();
            Container {
                required_mask: mask,
                required_value: value,
                client: (w, h),
                ..Container::form(unit)
            }
        };
        Self {
            pack: make(ShopPanel::Pack),
            sell: make(ShopPanel::Sell),
            buy: make(ShopPanel::Buy),
            stock: make(ShopPanel::Stock),
            drag: None,
        }
    }

    pub fn get(&self, panel: ShopPanel) -> &Container {
        match panel {
            ShopPanel::Pack => &self.pack,
            ShopPanel::Sell => &self.sell,
            ShopPanel::Buy => &self.buy,
            ShopPanel::Stock => &self.stock,
        }
    }

    pub fn get_mut(&mut self, panel: ShopPanel) -> &mut Container {
        match panel {
            ShopPanel::Pack => &mut self.pack,
            ShopPanel::Sell => &mut self.sell,
            ShopPanel::Buy => &mut self.buy,
            ShopPanel::Stock => &mut self.stock,
        }
    }

    /// The stock as the layout loads it: each shelf item with the shop's
    /// flag, laid out by the grid pass seeded with `seed`. `slot` gives
    /// each item's icon slot and half extents.
    pub fn load_stock(
        &mut self,
        items: &[u32],
        seed: u32,
        mut slot: impl FnMut(u32, u32) -> (usize, (i32, i32)),
    ) {
        let stock = &mut self.stock;
        let slots: Vec<(usize, (i32, i32))> =
            items.iter().map(|&id| slot(id, SHOP_ITEM_FLAGS)).collect();
        let halves: Vec<[i32; 2]> = slots.iter().map(|(_, h)| [h.0, h.1]).collect();
        let cells = grid_layout(
            items.len(),
            stock.client.0,
            stock.client.1,
            stock.icon_size,
            &halves,
            seed,
        );
        stock.entries = items
            .iter()
            .zip(&slots)
            .zip(cells.into_iter().chain(std::iter::repeat(Cell::default())))
            .map(|((&id, &(icon, _)), cell)| crate::item_container::Entry {
                cell,
                id,
                icon,
                flags: SHOP_ITEM_FLAGS,
                drag: 0,
            })
            .collect();
        stock.loaded = true;
    }

    /// The pack as the layout loads it: the player's inventory list with
    /// its stored places and flags, clamped inside the panel by each icon's
    /// half extents. `slot` gives each item's icon slot and half extents.
    pub fn load_pack(
        &mut self,
        children: &[ListChild],
        mut slot: impl FnMut(u32, u32) -> (usize, (i32, i32)),
    ) {
        let slots: Vec<(usize, (i32, i32))> =
            children.iter().map(|c| slot(c.id, c.flags)).collect();
        let halves: std::collections::HashMap<usize, (i32, i32)> = slots.iter().copied().collect();
        let mut resolved = slots.iter();
        self.pack.load_list(
            children,
            |_, _| resolved.next().map(|&(icon, _)| (icon, 0)),
            |icon| halves.get(&icon).copied(),
        );
    }

    /// The panel under a client-local point and the point relative to the
    /// panel's cells.
    pub fn panel_at(&self, point: (i32, i32)) -> Option<(ShopPanel, Cell)> {
        SHOP_PANELS.into_iter().find_map(|panel| {
            let [x, y, w, h] = panel.client();
            let (px, py) = (point.0 - x, point.1 - y);
            (px >= 0 && py >= 0 && px < w && py < h).then_some((panel, Cell::new(px, py)))
        })
    }

    /// The top-most entry under a client-local point: its panel and index.
    pub fn entry_at(&self, point: (i32, i32)) -> Option<(ShopPanel, usize)> {
        let (panel, local) = self.panel_at(point)?;
        let container = self.get(panel);
        crate::item_container::hit_test(
            container.entries.len(),
            container.icon_size,
            local.x,
            local.y,
            |i| {
                let entry = &container.entries[i];
                (entry.cell, entry.flags, true)
            },
        )
        .map(|index| (panel, index))
    }

    /// The item under a client-local point and the flags its panel holds
    /// it with (a card request's item).
    pub fn item_at(&self, point: (i32, i32)) -> Option<(u32, u32)> {
        let (panel, index) = self.entry_at(point)?;
        let entry = self.get(panel).entries.get(index)?;
        (entry.id != 0).then_some((entry.id, entry.flags))
    }

    /// A left press at a client-local point: over an entry it starts that
    /// entry's drag (the entry is marked dragged, the grab at half the
    /// icon). `bulk` is the item's bulk byte.
    pub fn press(&mut self, point: (i32, i32), bulk: impl FnOnce(u32) -> i32) -> Option<ShopDrag> {
        if self.drag.is_some() {
            return None;
        }
        let (panel, index) = self.entry_at(point)?;
        self.press_entry(panel, index, bulk)
    }

    /// Starts the drag of entry `index` of `panel`, as a press on it does.
    pub fn press_entry(
        &mut self,
        panel: ShopPanel,
        index: usize,
        bulk: impl FnOnce(u32) -> i32,
    ) -> Option<ShopDrag> {
        if self.drag.is_some() {
            return None;
        }
        let container = self.get_mut(panel);
        let grab = match crate::item_container::panel_event(
            0x4001,
            Some(index),
            true,
            false,
            0,
            None,
            container.icon_size,
            false,
        ) {
            crate::item_container::PanelEvent::StartDrag { grab } => grab,
            _ => return None,
        };
        let entry = container.entries.get_mut(index)?;
        entry.drag |= 1;
        let drag = ShopDrag {
            source: panel,
            icon: entry.icon,
            grab,
            id: entry.id,
            flags: entry.flags,
            bulk: bulk(entry.id),
        };
        self.drag = Some(drag);
        Some(drag)
    }

    /// Clears the drag: the source keeps its entry where it was.
    pub fn cancel(&mut self) {
        if let Some(drag) = self.drag.take() {
            let source = self.get_mut(drag.source);
            source.finish_drag(drag.source.number(), true, true, 0, drag.bulk);
        }
    }

    /// The release at a client-local point. `half` is the dragged icon's
    /// half extents and `model_found` whether the item has a model record
    /// (the accept test refuses one that has none).
    pub fn release(&mut self, point: (i32, i32), half: (i32, i32), model_found: bool) -> ShopDrop {
        let Some(drag) = self.drag else {
            return ShopDrop::Cancelled;
        };
        let Some((destination, local)) = self.panel_at(point) else {
            self.cancel();
            return ShopDrop::Cancelled;
        };
        let source = drag.source;
        if destination == source {
            self.drag = None;
            self.get_mut(source).drop_entry(
                DropDrag {
                    same_panel: true,
                    linked: false,
                    id: drag.id,
                    icon: drag.icon,
                    flags: drag.flags,
                    bulk: drag.bulk,
                },
                drag.grab,
                local,
                half,
            );
            return ShopDrop::Repositioned(source);
        }
        let accepted = self.get(destination).accept(
            &Drag {
                src: Some(source.number()),
                dst: Some(destination.number()),
                objid: drag.id,
                objflags: drag.flags,
                icon: drag.icon as u32,
                bulk: drag.bulk.clamp(0, 255) as u8,
                model_found,
            },
            destination.number(),
        );
        if !accepted {
            self.cancel();
            return ShopDrop::Cancelled;
        }
        let dropped = self.get_mut(destination).drop_entry(
            DropDrag {
                same_panel: false,
                linked: false,
                id: drag.id,
                icon: drag.icon,
                flags: drag.flags,
                bulk: drag.bulk,
            },
            drag.grab,
            local,
            half,
        );
        if dropped.is_none() {
            self.cancel();
            return ShopDrop::Cancelled;
        }
        self.drag = None;
        self.get_mut(source)
            .finish_drag(destination.number(), false, true, drag.flags, drag.bulk);
        ShopDrop::Moved {
            from: source,
            to: destination,
            id: drag.id,
        }
    }

    /// The item ids a panel holds, in its order.
    pub fn ids(&self, panel: ShopPanel) -> Vec<u32> {
        self.get(panel).entries.iter().map(|e| e.id).collect()
    }

    /// The confirmed trade's items change hands: the buy basket's join the
    /// pack without the shop's flag and the sell basket's join the stock
    /// with it, each at its panel's centre [guess: the "Trade" button's
    /// callback lies outside the window and is not read].
    pub fn settle(&mut self) {
        let bought = std::mem::take(&mut self.buy.entries);
        for entry in bought {
            self.pack
                .add_at_centre(entry.id, entry.icon, entry.flags & !SHOP_ITEM_FLAGS, 0);
        }
        self.settle_sold();
    }

    /// The arena's settle: the window's bought hook takes the buy basket's
    /// items to the company's arsenal, so they leave the window (their icon
    /// handles are returned for release); the sell basket's join the stock
    /// as in [`Self::settle`].
    pub fn settle_bought_away(&mut self) -> Vec<usize> {
        let bought = std::mem::take(&mut self.buy.entries);
        self.settle_sold();
        bought
            .into_iter()
            .map(|entry| entry.icon)
            .filter(|&icon| icon != 0)
            .collect()
    }

    fn settle_sold(&mut self) {
        self.buy.used = 0;
        self.sell.used = 0;
        let sold = std::mem::take(&mut self.sell.entries);
        for entry in sold {
            self.stock
                .add_at_centre(entry.id, entry.icon, entry.flags | SHOP_ITEM_FLAGS, 0);
        }
    }

    /// Every icon handle the panels hold, for their release when the window
    /// closes.
    pub fn icons(&self) -> Vec<usize> {
        SHOP_PANELS
            .into_iter()
            .flat_map(|panel| self.get(panel).entries.iter().map(|e| e.icon))
            .filter(|&icon| icon != 0)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slots() -> impl FnMut(u32, u32) -> (usize, (i32, i32)) {
        let mut next = 0;
        move |_, _| {
            next += 1;
            (next, (80, 80))
        }
    }

    fn centre(panels: &ShopPanels, panel: ShopPanel, index: usize) -> (i32, i32) {
        let [x, y, _, _] = panel.client();
        let container = panels.get(panel);
        let cell = container.entries[index].cell;
        let half = container.icon_size / 2;
        (x + cell.x + half, y + cell.y + half)
    }

    fn middle(panel: ShopPanel) -> (i32, i32) {
        let [x, y, w, h] = panel.client();
        (x + w / 2, y + h / 2)
    }

    #[test]
    fn the_panels_fill_the_client_and_take_their_own_side() {
        let panels = ShopPanels::new(4);
        // The stock fills the client's height: 0x244 less the border twice.
        assert_eq!(
            ShopPanel::Stock.rect()[3],
            WINDOW_SIZE.1 - 2 * crate::shop_window::BORDER
        );
        assert_eq!(
            ShopPanel::Stock.rect()[0] + ShopPanel::Stock.rect()[2],
            WINDOW_SIZE.0 - 2 * crate::shop_window::BORDER
        );
        assert_eq!(panels.buy.required_value, SHOP_ITEM_FLAGS);
        assert_eq!(panels.sell.required_value, 0);
        assert_eq!(panels.stock.client, (0xe4 - 8, 0x234 - 8));
    }

    #[test]
    fn a_stock_item_drags_into_the_buy_basket_and_not_the_sell_basket() {
        let mut panels = ShopPanels::new(4);
        panels.load_stock(&[7, 8], 1, slots());
        assert!(
            panels
                .stock
                .entries
                .iter()
                .all(|e| e.flags == SHOP_ITEM_FLAGS)
        );
        let at = centre(&panels, ShopPanel::Stock, 1);
        assert_eq!(panels.item_at(at), Some((8, SHOP_ITEM_FLAGS)));
        // The sell basket refuses a shop item; the entry stays.
        assert!(panels.press(at, |_| 0).is_some());
        assert_eq!(
            panels.release(middle(ShopPanel::Sell), (80, 80), true),
            ShopDrop::Cancelled
        );
        assert_eq!(panels.ids(ShopPanel::Stock), [7, 8]);
        assert!(panels.stock.entries.iter().all(|e| e.drag == 0));
        // The buy basket takes it.
        assert!(panels.press(at, |_| 0).is_some());
        assert_eq!(
            panels.release(middle(ShopPanel::Buy), (80, 80), true),
            ShopDrop::Moved {
                from: ShopPanel::Stock,
                to: ShopPanel::Buy,
                id: 8
            }
        );
        assert_eq!(panels.ids(ShopPanel::Stock), [7]);
        assert_eq!(panels.ids(ShopPanel::Buy), [8]);
        assert_eq!(
            panels.item_at(centre(&panels, ShopPanel::Buy, 0)),
            Some((8, SHOP_ITEM_FLAGS))
        );
    }

    #[test]
    fn a_pack_item_sells_and_a_release_outside_cancels() {
        let mut panels = ShopPanels::new(4);
        let children = [ListChild {
            id: 9,
            flags: 0,
            x: 0,
            y: 0,
        }];
        panels.load_pack(&children, slots());
        let at = centre(&panels, ShopPanel::Pack, 0);
        assert!(panels.press(at, |_| 0).is_some());
        // Outside every panel: cancelled.
        assert_eq!(
            panels.release((-50, -50), (80, 80), true),
            ShopDrop::Cancelled
        );
        assert_eq!(panels.ids(ShopPanel::Pack), [9]);
        // The stock refuses the player's item; the sell basket takes it.
        assert!(panels.press(at, |_| 0).is_some());
        assert_eq!(
            panels.release(middle(ShopPanel::Stock), (80, 80), true),
            ShopDrop::Cancelled
        );
        assert!(panels.press(at, |_| 0).is_some());
        assert!(matches!(
            panels.release(middle(ShopPanel::Sell), (80, 80), true),
            ShopDrop::Moved { id: 9, .. }
        ));
        // Settled: the sold item joins the stock with the shop's flag.
        panels.settle();
        assert!(panels.sell.entries.is_empty());
        assert_eq!(panels.stock.entries[0].flags, SHOP_ITEM_FLAGS);
    }

    #[test]
    fn a_drop_in_its_own_panel_only_moves_the_entry() {
        let mut panels = ShopPanels::new(4);
        panels.load_stock(&[7], 1, slots());
        let at = centre(&panels, ShopPanel::Stock, 0);
        panels.press(at, |_| 0);
        let [x, y, _, _] = ShopPanel::Stock.client();
        let to = (x + 120, y + 300);
        assert_eq!(
            panels.release(to, (80, 80), true),
            ShopDrop::Repositioned(ShopPanel::Stock)
        );
        assert_eq!(panels.stock.entries[0].cell, Cell::new(120 - 80, 300 - 80));
        assert_eq!(panels.stock.entries[0].drag, 0);
    }
}
