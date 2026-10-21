//! The trade window (`TShopWindow`): the shop's stock, the player's pack,
//! the buy and sell baskets, the running balance and the confirm button.
//!
//! The original's window is titled "Trade" over a 0x36c × 0x244 page. Two
//! 0xe4 × 0xb4 panels stand at the top (the shop's stock on the left, the
//! player's pack on the right, both accepting only items flagged
//! `0x80000000`), and the two baskets (0xc4 × 0x174) sit between them. A
//! changed basket asks for the two totals: the buy basket's prices at full
//! value, the sell basket's quartered. The difference is the balance: a
//! negative balance shows "Receive:" with its amount, a positive one
//! "Pay:" — with the confirm button disabled when the purse cannot cover
//! it or both baskets are empty. A right-button release closes the window.
//!
//! Evidence: the constructor and its four panels, the two basket change
//! callbacks, the balance, the stock drop, the drop test, the close and the
//! mouse slot.

use crate::career::{Company, basket_total};
use crate::trade::Transaction;

/// The window's title.
pub const TRADE_TITLE: &str = "Trade";
/// The window page's size in units.
pub const WINDOW_SIZE: (i32, i32) = (0x36c, 0x244);
/// The two side panels and the two baskets, as (x, y, width, height).
pub const STOCK_PANEL: [i32; 4] = [0, 0x180, 0xe4, 0xb4];
pub const PACK_PANEL: [i32; 4] = [0, 0, 0xe4, 0xb4];
pub const SELL_BASKET: [i32; 4] = [0xe8, 8, 0xc4, 0x174];
pub const BUY_BASKET: [i32; 4] = [0x1b0, 8, 0xc4, 0x174];
/// The two basket gaps past the pack's left edge: 300 − 0xe4 and 400 −
/// 0x18a in the large page, or 0x10/10 in the compact one.
pub const BASKET_GAP_LARGE: [i32; 2] = [0x1a, 0x12];
pub const BASKET_GAP_COMPACT: [i32; 2] = [0x10, 0x0a];
/// The labels and the inks the balance sets.
pub const PAY_LABEL: &str = "Pay:";
pub const RECEIVE_LABEL: &str = "Receive:";
/// The button's label ink word and its enabled flag's bit.
pub const INK_HIGHLIGHT: u32 = 0x110;
pub const DISABLED_BIT: u32 = 0x10;
/// The drop kind the shop's panel always listens for (the original's 8 is
/// not accepted by the stock panel).
pub const REFUSED_KIND: u32 = 8;
/// The item flag a stock panel requires.
pub const STOCK_REQUIRED: u32 = 0x8000_0000;

/// What the balance reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Balance {
    /// Full value of the buy basket.
    pub buy: u32,
    /// Quartered value of the sell basket.
    pub sell: u32,
    /// Buy less sell; negative when the player receives.
    pub balance: i32,
    pub label: &'static str,
    /// The amount shown under the label.
    pub amount: i32,
    /// The label's ink word (0x110 when the player receives or pays, 0
    /// when the purse is short).
    pub ink: u32,
    /// Whether the confirm button takes a press.
    pub enabled: bool,
    /// The button's state word: 0 or [`DISABLED_BIT`].
    pub button_state: u32,
}

/// What a press did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShopReply {
    /// The press was the window's business.
    Handled,
    /// The right button was released: the window closes.
    Close,
}

/// Where an item enters the stock panel from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropSource {
    /// The player's pack (the linked panel): no change to capacity.
    Linked,
    /// Anywhere else.
    Elsewhere,
}

/// The trade window's state.
#[derive(Debug, Clone, PartialEq)]
pub struct ShopWindow {
    /// The shelf's price words, oldest first.
    pub stock: Vec<u16>,
    /// The buy basket's price words.
    pub buy: Vec<u16>,
    /// The sell basket's price words.
    pub sell: Vec<u16>,
    /// The player's copper.
    pub money: u32,
    pub compact: bool,
    /// The mode holds no window until it is shown.
    pub shown: bool,
    /// The window's title and page.
    pub title: &'static str,
    pub size: (i32, i32),
    /// The two basket gaps.
    pub gaps: [i32; 2],
    /// The mode holds a character preview.
    pub character_view: bool,
    /// Items the stock panel was given this frame (the original's add).
    pub dropped: Vec<(u16, DropSource)>,
    /// The balance label the button shows; the original updates only the
    /// amount and the ink when the purse is short.
    pub label: &'static str,
}

impl ShopWindow {
    /// The constructor: a large page by default, "Trade", the small-page
    /// gaps for the compact variant. `money` is the player's copper.
    pub fn new(money: u32, compact: bool) -> Self {
        Self {
            stock: Vec::new(),
            buy: Vec::new(),
            sell: Vec::new(),
            money,
            compact,
            shown: true,
            title: TRADE_TITLE,
            size: WINDOW_SIZE,
            gaps: if compact {
                BASKET_GAP_COMPACT
            } else {
                BASKET_GAP_LARGE
            },
            character_view: true,
            dropped: Vec::new(),
            label: PAY_LABEL,
        }
    }

    /// The window's panels, in the original's order.
    pub fn panels(&self) -> [[i32; 4]; 4] {
        [STOCK_PANEL, PACK_PANEL, SELL_BASKET, BUY_BASKET]
    }

    /// The shelf takes an item: an item whose price word is 0 and which
    /// comes from the player's pack is simply listed; anything else is
    /// added at the panel's centre and scattered to the last place.
    pub fn drop_on_stock(&mut self, word: u16, source: DropSource) {
        self.dropped.push((word, source));
        self.stock.push(word);
    }

    /// Adds a price word to the buy basket.
    pub fn add_buy(&mut self, word: u16) {
        self.buy.push(word);
    }

    /// Adds a price word to the sell basket.
    pub fn add_sell(&mut self, word: u16) {
        self.sell.push(word);
    }

    /// Removes an item from a basket by place.
    pub fn remove_buy(&mut self, at: usize) -> Option<u16> {
        (at < self.buy.len()).then(|| self.buy.remove(at))
    }

    pub fn remove_sell(&mut self, at: usize) -> Option<u16> {
        (at < self.sell.len()).then(|| self.sell.remove(at))
    }

    /// The balance: the buy basket at full price less the sell basket at a
    /// quarter. A negative result shows "Receive:" with its amount in
    /// highlight ink and the button enabled; a non-negative one shows
    /// "Pay:" with the amount when the purse covers it, else a disabled
    /// button with no amount. Either way the button is disabled while both
    /// baskets are empty.
    pub fn balance(&mut self) -> Balance {
        let buy = basket_total(&self.buy, false);
        let sell = basket_total(&self.sell, true);
        let balance = buy as i32 - sell as i32;
        if balance < 0 {
            self.label = RECEIVE_LABEL;
            return Balance {
                buy,
                sell,
                balance,
                label: RECEIVE_LABEL,
                amount: balance.wrapping_neg(),
                ink: INK_HIGHLIGHT,
                enabled: true,
                button_state: 0,
            };
        }
        if self.money < balance as u32 {
            return Balance {
                buy,
                sell,
                balance,
                label: self.label,
                amount: 0,
                ink: 0,
                enabled: false,
                button_state: DISABLED_BIT,
            };
        }
        let empty = self.buy.is_empty() && self.sell.is_empty();
        self.label = PAY_LABEL;
        Balance {
            buy,
            sell,
            balance,
            label: PAY_LABEL,
            amount: balance,
            ink: INK_HIGHLIGHT,
            enabled: !empty,
            button_state: if empty { DISABLED_BIT } else { 0 },
        }
    }

    /// The money changed (the purse is read when the balance is asked).
    pub fn set_money(&mut self, money: u32) {
        self.money = money;
    }

    /// The stock panel's drop test: kind 8 is never the shop's; every other
    /// item is taken.
    pub fn accepts_drop(kind: u32) -> bool {
        kind != REFUSED_KIND
    }

    /// The mouse slot: a right-button release (0x2002) closes the window;
    /// any other word is the form's.
    pub fn press(&mut self, word: u32) -> ShopReply {
        if word & 0x2002 == 0x2002 {
            self.shown = false;
            ShopReply::Close
        } else {
            ShopReply::Handled
        }
    }

    /// The close slot: the character preview and the baskets go, the window
    /// hides.
    pub fn close(&mut self) {
        self.character_view = false;
        self.shown = false;
        self.buy.clear();
        self.sell.clear();
    }

    /// Confirms the baskets through the company's coins: a buy is the full
    /// basket, a sale a quarter of it. An empty pair or a short purse is
    /// refused; on success the baskets empty.
    pub fn confirm(&mut self, company: &mut Company) -> bool {
        if !self.balance().enabled {
            return false;
        }
        let done = company.trade_lines(&self.buy, &self.sell);
        if done {
            self.buy.clear();
            self.sell.clear();
            self.money = company.copper;
        }
        done
    }

    /// Confirms a trade of real item ids: the buy basket's items leave the
    /// shelf and the sold items join it, with the two tallies (spent and
    /// earned) as the original's transaction records them.
    pub fn confirm_items(&mut self, company: &mut Company, bought: &[u32], sold: &[u32]) -> bool {
        let buy = basket_total(&self.buy, false);
        let sell = basket_total(&self.sell, true);
        let due = buy as i32 - sell as i32;
        if (buy == 0 && sell == 0) || company.copper < due.max(0) as u32 {
            return false;
        }
        let trade = Transaction {
            due,
            tallies: [due.max(0), (-due).max(0)],
            sold: sold.to_vec(),
            bought: bought.to_vec(),
        };
        company.settle(&trade);
        self.buy.clear();
        self.sell.clear();
        self.money = company.copper;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_constructor_matches_the_window() {
        let window = ShopWindow::new(300, false);
        assert_eq!(window.title, "Trade");
        assert_eq!(window.size, (0x36c, 0x244));
        assert_eq!(window.gaps, [0x1a, 0x12]);
        assert!(window.shown && window.character_view);
        assert_eq!(
            window.panels(),
            [
                [0, 0x180, 0xe4, 0xb4],
                [0, 0, 0xe4, 0xb4],
                [0xe8, 8, 0xc4, 0x174],
                [0x1b0, 8, 0xc4, 0x174],
            ]
        );
        let compact = ShopWindow::new(0, true);
        assert_eq!(compact.gaps, [0x10, 0x0a]);
        assert_eq!((STOCK_REQUIRED, REFUSED_KIND), (0x8000_0000, 8));
    }

    #[test]
    fn the_balance_shows_pay_receive_or_a_short_purse() {
        let mut window = ShopWindow::new(100, false);
        // An empty pair: "Pay: 0" with the button disabled.
        let balance = window.balance();
        assert_eq!((balance.label, balance.amount), (PAY_LABEL, 0));
        assert!(!balance.enabled);
        assert_eq!(balance.button_state, DISABLED_BIT);
        // Three copper bought, one silver sold (a quarter of 12 is 3):
        // 3 − 3 = 0, but the baskets are not empty and the button works.
        window.add_buy(3);
        window.add_sell(0x4001);
        let balance = window.balance();
        assert_eq!((balance.buy, balance.sell, balance.balance), (3, 3, 0));
        assert_eq!((balance.label, balance.amount), (PAY_LABEL, 0));
        assert!(balance.enabled);
        assert_eq!((balance.ink, balance.button_state), (INK_HIGHLIGHT, 0));
        // One gold bought, a pair of silver sold: 144 − 6 = 138.
        window.buy.clear();
        window.sell.clear();
        window.add_buy(0x8001);
        window.add_sell(0x4002);
        window.set_money(200);
        let balance = window.balance();
        assert_eq!((balance.buy, balance.sell), (144, 6));
        assert_eq!((balance.balance, balance.amount), (138, 138));
        // More sold than bought: ten gold is 1440 copper, quartered to
        // 360, so the player receives.
        window.buy.clear();
        window.sell.clear();
        window.add_sell(0xc001);
        let balance = window.balance();
        assert_eq!(balance.balance, -360);
        assert_eq!((balance.label, balance.amount), (RECEIVE_LABEL, 360));
        assert!(balance.enabled);
        // A purse short of the price: disabled, no amount, ink 0.
        window.sell.clear();
        window.add_buy(0x8001);
        window.set_money(143);
        let balance = window.balance();
        assert_eq!((balance.label, balance.amount), (RECEIVE_LABEL, 0));
        assert_eq!((balance.ink, balance.button_state), (0, DISABLED_BIT));
        assert!(!balance.enabled);
        window.set_money(144);
        assert!(window.balance().enabled);
    }

    #[test]
    fn the_baskets_and_the_stock_take_and_give_items() {
        let mut window = ShopWindow::new(10, false);
        window.drop_on_stock(0x8001, DropSource::Elsewhere);
        window.drop_on_stock(3, DropSource::Linked);
        assert_eq!(window.stock, [0x8001, 3]);
        assert_eq!(window.dropped.len(), 2);
        window.add_buy(5);
        window.add_buy(6);
        window.add_sell(7);
        assert_eq!(window.remove_buy(0), Some(5));
        assert_eq!(window.remove_buy(9), None);
        assert_eq!(window.remove_sell(0), Some(7));
        assert_eq!(window.remove_sell(0), None);
        assert!(ShopWindow::accepts_drop(0));
        assert!(!ShopWindow::accepts_drop(REFUSED_KIND));
    }

    #[test]
    fn a_right_release_closes_and_the_close_frees_the_baskets() {
        let mut window = ShopWindow::new(0, false);
        assert_eq!(window.press(0x1002), ShopReply::Handled);
        assert!(window.shown);
        assert_eq!(window.press(0x2002), ShopReply::Close);
        assert!(!window.shown);
        let mut window = ShopWindow::new(0, false);
        window.add_buy(1);
        window.close();
        assert!(window.buy.is_empty() && !window.character_view);
    }

    #[test]
    fn confirming_spends_and_earns_the_baskets() {
        let mut company = Company::new("Tester");
        company.copper = 200;
        let mut window = ShopWindow::new(company.copper, false);
        window.add_buy(0x8001);
        window.add_sell(0x4002);
        assert!(window.confirm(&mut company));
        // 144 − 6 = 138 copper spent.
        assert_eq!(company.copper, 62);
        assert_eq!(company.spent, 138);
        assert!(window.buy.is_empty() && window.sell.is_empty());
        assert_eq!(window.money, 62);
        // An empty pair is refused.
        assert!(!window.confirm(&mut company));
        // A short purse is refused and leaves the baskets.
        let mut poor = Company::new("Poor");
        poor.copper = 10;
        let mut window = ShopWindow::new(10, false);
        window.add_buy(0x8001);
        assert!(!window.confirm(&mut poor));
        assert_eq!(poor.copper, 10);
        assert_eq!(window.buy, [0x8001]);
    }

    #[test]
    fn confirming_items_moves_them_and_tallies() {
        let mut company = Company::new("Tester");
        company.copper = 300;
        let mut window = ShopWindow::new(300, false);
        company.stock = vec![11, 12];
        window.stock = vec![11, 12];
        window.add_buy(0x4001);
        window.add_sell(2);
        // 12 bought and a 2-copper sale, quartered to nothing: 12 due.
        let due = 12;
        assert!(window.confirm_items(&mut company, &[11], &[13]));
        assert_eq!(company.copper, 300 - due as u32);
        assert_eq!(company.stock, vec![12, 13]);
        assert_eq!(company.spent, due as u32);
    }
}
