//! The trade window (`TShopWindow`): the character preview with the
//! player's loose items under it, the sell and buy baskets, the shop's
//! stock, and a bottom row with the totals, the balance, the purse and the
//! "Trade" button.
//!
//! The page is 0x36c × 0x244 and titled "Trade". Left to right: the
//! character preview (0xe4 × 0x17c) with the pack panel (0xe4 × 0xb4)
//! under it, the sell basket and the buy basket (0xc4 × 0x174 each), and
//! the stock panel (0xe4 × 0x234) held 0x10 from the page's right edge.
//! Each basket asks for its own total when it changes: the buy basket sums
//! its prices, the sell basket sums them and shifts the sum right by two (a
//! quarter, rounded down). The balance is the buy total less the sell
//! total. Which basket is which is read from the layout: the quartered
//! basket stands next to the player's side and its total is drawn at x 280
//! under it, the full-price basket next to the stock with its total at x
//! 480; a positive balance is paid, a negative one received.
//!
//! The bottom row is a form of eight records: the sell total, the buy
//! total, the balance label ("Pay:" or "Receive:"), the balance amount,
//! the warning "You cannot afford this transaction." (hidden at first),
//! "Owned:", the purse, and the "Trade" button (disabled at first). The two
//! labels sit at rows 0x10 and 0x0a when the window opens with the
//! merchant's category and rank options, else at 0x1a and 0x12.
//!
//! Evidence: the constructor, the two basket callbacks, the balance, the
//! character view's take-off and drop callbacks, the close and the mouse
//! slot (all checked against the disassembly), the bottom row's record
//! template and its strings, and the opener, which passes the merchant flag
//! and adds the merchant's options form only with it. The "Trade" button's
//! own callback lies outside this window.

use exa_ui::form::record_state::{DISABLED, NOT_DRAWN};

use crate::career::{Company, basket_total};

/// The window's title.
pub const TRADE_TITLE: &str = "Trade";
/// The page's size in units.
pub const WINDOW_SIZE: (i32, i32) = (0x36c, 0x244);
/// The page's colour word, as the constructor stores it.
pub const WINDOW_COLOUR: u32 = 0xce1c_2023;
/// The character preview, (x, y, width, height).
pub const CHARACTER_VIEW: [i32; 4] = [0, 0, 0xe4, 0x17c];
/// The player's loose items, under the preview: what is taken off the
/// character or dropped on it lands here.
pub const PACK_PANEL: [i32; 4] = [0, 0x180, 0xe4, 0xb4];
/// The basket valued at a quarter (the player's goods for sale).
pub const SELL_BASKET: [i32; 4] = [0xe8, 8, 0xc4, 0x174];
/// The basket valued at full price (the shop's goods to buy).
pub const BUY_BASKET: [i32; 4] = [0x1b0, 8, 0xc4, 0x174];
/// The stock panel's size and its margin from the page's right edge.
pub const STOCK_SIZE: (i32, i32) = (0xe4, 0x234);
pub const STOCK_RIGHT_MARGIN: i32 = 0x10;
/// The bottom row's texts.
pub const PAY_LABEL: &str = "Pay:";
pub const RECEIVE_LABEL: &str = "Receive:";
pub const SHORT_PURSE_TEXT: &str = "You cannot afford this transaction.";
pub const OWNED_LABEL: &str = "Owned:";
pub const BUTTON_LABEL: &str = "Trade";
/// The rows of the balance label and of "Owned:", with the merchant's
/// options and without.
pub const LABEL_ROWS_MERCHANT: [i32; 2] = [0x10, 0x0a];
pub const LABEL_ROWS_PLAIN: [i32; 2] = [0x1a, 0x12];
/// The item kind the character view's drop callback never moves to the
/// pack.
pub const KEPT_KIND: u32 = 8;

/// The stock panel's place on a page `page_width` wide: its right edge
/// 0x10 inside the page's, at the top.
pub fn stock_panel(page_width: i32) -> [i32; 4] {
    [
        page_width - STOCK_SIZE.0 - STOCK_RIGHT_MARGIN,
        0,
        STOCK_SIZE.0,
        STOCK_SIZE.1,
    ]
}

/// What the balance writes into the bottom row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Balance {
    /// The full-price total, drawn under the buy basket.
    pub buy: u32,
    /// The quartered total, drawn under the sell basket.
    pub sell: u32,
    /// Buy less sell, in 32 bits; negative when the player receives.
    pub balance: i32,
    /// The label's new text; `None` leaves it as it was (a short purse).
    pub label: Option<&'static str>,
    /// The label's record state: 0, or [`NOT_DRAWN`] while the purse is
    /// short.
    pub label_state: u32,
    /// The copper the amount record shows (0 while the purse is short).
    pub amount: u32,
    /// The warning's record state: 0 (shown) while the purse is short,
    /// else [`NOT_DRAWN`].
    pub warning_state: u32,
    /// The button's record state: 0, or [`DISABLED`] while the purse is
    /// short or both baskets are empty.
    pub button_state: u32,
}

impl Balance {
    /// Whether the "Trade" button takes a press.
    pub fn enabled(&self) -> bool {
        self.button_state & DISABLED == 0
    }
}

/// What a mouse word did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShopReply {
    /// A right-button release: the window hides and the word is taken.
    Close,
    /// Any other word goes to the form's own handler.
    Form,
}

/// The trade window's state.
#[derive(Debug, Clone, PartialEq)]
pub struct ShopWindow {
    /// The buy basket's price words.
    pub buy: Vec<u16>,
    /// The sell basket's price words.
    pub sell: Vec<u16>,
    /// The two totals as the last basket change left them (buy, sell).
    pub totals: (u32, u32),
    /// The player's loose items, as (kind, item) in arrival order.
    pub pack: Vec<(u32, i32)>,
    /// The player's copper (the purse record).
    pub money: u32,
    /// Opened with the merchant's category and rank options.
    pub merchant: bool,
    pub shown: bool,
    /// The trade mode the opener raises and the close lowers.
    pub trading: bool,
    /// The window holds its preview character.
    pub character_view: bool,
    /// The bottom row's two label rows.
    pub label_rows: [i32; 2],
    /// The balance label's text.
    pub label: &'static str,
    /// The bottom row as the last balance wrote it.
    pub shown_balance: Balance,
}

impl ShopWindow {
    /// The window as the opener makes it, with `money` copper in the
    /// purse. The bottom row starts as its template: "Pay:", the warning
    /// hidden and the button disabled.
    pub fn new(money: u32, merchant: bool) -> Self {
        Self {
            buy: Vec::new(),
            sell: Vec::new(),
            totals: (0, 0),
            pack: Vec::new(),
            money,
            merchant,
            shown: true,
            trading: true,
            character_view: true,
            label_rows: if merchant {
                LABEL_ROWS_MERCHANT
            } else {
                LABEL_ROWS_PLAIN
            },
            label: PAY_LABEL,
            shown_balance: Balance {
                buy: 0,
                sell: 0,
                balance: 0,
                label: Some(PAY_LABEL),
                label_state: 0,
                amount: 0,
                warning_state: NOT_DRAWN,
                button_state: DISABLED,
            },
        }
    }

    /// The window's panels, left to right: the preview, the pack, the sell
    /// basket, the buy basket and the stock.
    pub fn panels(&self) -> [[i32; 4]; 5] {
        [
            CHARACTER_VIEW,
            PACK_PANEL,
            SELL_BASKET,
            BUY_BASKET,
            stock_panel(WINDOW_SIZE.0),
        ]
    }

    /// The character view's take-off callback: an item with a negative
    /// number is ignored, any other goes to the pack.
    pub fn take_off(&mut self, kind: u32, item: i32) {
        if item >= 0 {
            self.pack.push((kind, item));
        }
    }

    /// The character view's drop callback: kind 8 is not handled (the
    /// original returns an unset value for it, `None` here); any other
    /// drop is handled, and an item with a non-negative number moves to
    /// the pack. Exarust's pack has no capacity, so the add always lands.
    pub fn drop_on_character(&mut self, kind: u32, item: i32) -> Option<bool> {
        if kind == KEPT_KIND {
            return None;
        }
        self.take_off(kind, item);
        Some(true)
    }

    /// The buy basket changed: its total is summed again and the bottom
    /// row redone.
    pub fn buy_changed(&mut self) -> Balance {
        self.totals.0 = basket_total(&self.buy, false);
        self.balance()
    }

    /// The sell basket changed: its total is summed, quartered, and the
    /// bottom row redone.
    pub fn sell_changed(&mut self) -> Balance {
        self.totals.1 = basket_total(&self.sell, true);
        self.balance()
    }

    /// Adds a price word to the buy basket.
    pub fn add_buy(&mut self, word: u16) -> Balance {
        self.buy.push(word);
        self.buy_changed()
    }

    /// Adds a price word to the sell basket.
    pub fn add_sell(&mut self, word: u16) -> Balance {
        self.sell.push(word);
        self.sell_changed()
    }

    /// Removes an item from the buy basket by place.
    pub fn remove_buy(&mut self, at: usize) -> Option<u16> {
        let word = (at < self.buy.len()).then(|| self.buy.remove(at))?;
        self.buy_changed();
        Some(word)
    }

    /// Removes an item from the sell basket by place.
    pub fn remove_sell(&mut self, at: usize) -> Option<u16> {
        let word = (at < self.sell.len()).then(|| self.sell.remove(at))?;
        self.sell_changed();
        Some(word)
    }

    /// The bottom row from the two held totals. The two totals are shown
    /// first; the balance is buy less sell in 32 bits. A non-negative
    /// balance larger than the purse hides the label, shows 0 and the
    /// warning and disables the button, leaving the label's text alone.
    /// One the purse covers shows "Pay:" and the amount, hides the warning
    /// and disables the button only while both baskets are empty. A
    /// negative balance shows "Receive:" and its magnitude, hides the
    /// warning and enables the button.
    pub fn balance(&mut self) -> Balance {
        let (buy, sell) = self.totals;
        let balance = buy.wrapping_sub(sell) as i32;
        let row = if balance < 0 {
            Balance {
                buy,
                sell,
                balance,
                label: Some(RECEIVE_LABEL),
                label_state: 0,
                amount: balance.wrapping_neg() as u32,
                warning_state: NOT_DRAWN,
                button_state: 0,
            }
        } else if i64::from(balance) > i64::from(self.money) {
            Balance {
                buy,
                sell,
                balance,
                label: None,
                label_state: NOT_DRAWN,
                amount: 0,
                warning_state: 0,
                button_state: DISABLED,
            }
        } else {
            let empty = self.buy.is_empty() && self.sell.is_empty();
            Balance {
                buy,
                sell,
                balance,
                label: Some(PAY_LABEL),
                label_state: 0,
                amount: balance as u32,
                warning_state: NOT_DRAWN,
                button_state: if empty { DISABLED } else { 0 },
            }
        };
        if let Some(text) = row.label {
            self.label = text;
        }
        self.shown_balance = row;
        row
    }

    /// The purse changed; the row is redone at the next basket change.
    pub fn set_money(&mut self, money: u32) {
        self.money = money;
    }

    /// The mouse slot: a word with both 0x2000 and 2 set (a right-button
    /// release) hides the window; any other is the form's.
    pub fn press(&mut self, word: u16) -> ShopReply {
        if word & 0x2002 == 0x2002 {
            self.shown = false;
            ShopReply::Close
        } else {
            ShopReply::Form
        }
    }

    /// The close slot: the preview character is released, the trade mode
    /// lowered, and the form closes.
    pub fn close(&mut self) {
        self.character_view = false;
        self.trading = false;
        self.shown = false;
    }

    /// Settles the two baskets through the company's coins while the
    /// button takes a press. [guess: the "Trade" button's callback lies
    /// outside this window; Exarust settles through
    /// [`Company::trade_lines`] and empties the baskets on success]
    pub fn confirm(&mut self, company: &mut Company) -> bool {
        if !self.shown_balance.enabled() {
            return false;
        }
        let done = company.trade_lines(&self.buy, &self.sell);
        if done {
            self.buy.clear();
            self.sell.clear();
            self.money = company.copper;
            self.buy_changed();
            self.sell_changed();
        }
        done
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::career::{UNIT_GOLD, UNIT_SILVER, UNIT_TEN_GOLD};

    #[test]
    fn the_constructor_lays_out_the_page() {
        let window = ShopWindow::new(300, false);
        assert_eq!(window.label_rows, [0x1a, 0x12]);
        assert!(window.shown && window.trading && window.character_view);
        // The stock's x: 0x36c − 0xe4 − 0x10 = 0x278.
        assert_eq!(
            window.panels(),
            [
                [0, 0, 0xe4, 0x17c],
                [0, 0x180, 0xe4, 0xb4],
                [0xe8, 8, 0xc4, 0x174],
                [0x1b0, 8, 0xc4, 0x174],
                [0x278, 0, 0xe4, 0x234],
            ]
        );
        assert_eq!(ShopWindow::new(0, true).label_rows, [0x10, 0x0a]);
        // The template: "Pay:", the warning hidden, the button disabled.
        let row = window.shown_balance;
        assert_eq!(row.label, Some(PAY_LABEL));
        assert_eq!((row.warning_state, row.button_state), (0x110, 0x10));
        assert!(!row.enabled());
    }

    #[test]
    fn the_balance_shows_pay_receive_or_a_short_purse() {
        let mut window = ShopWindow::new(100, false);
        // Empty baskets: "Pay:" 0, the button disabled.
        let row = window.balance();
        assert_eq!((row.label, row.amount), (Some(PAY_LABEL), 0));
        assert_eq!(row.button_state, DISABLED);
        // Three copper bought, one silver (12) sold and quartered to 3:
        // 3 − 3 = 0, and the baskets are not empty, so the button works.
        window.add_buy(3);
        let row = window.add_sell(UNIT_SILVER | 1);
        assert_eq!((row.buy, row.sell, row.balance), (3, 3, 0));
        assert_eq!((row.label, row.amount), (Some(PAY_LABEL), 0));
        assert_eq!((row.label_state, row.warning_state), (0, NOT_DRAWN));
        assert!(row.enabled());
        // One gold (144) bought, two silver (24) sold: 144 − 6 = 138.
        window.remove_buy(0);
        window.remove_sell(0);
        window.set_money(200);
        window.add_buy(UNIT_GOLD | 1);
        let row = window.add_sell(UNIT_SILVER | 2);
        assert_eq!((row.buy, row.sell), (144, 6));
        assert_eq!((row.balance, row.amount), (138, 138));
        // Ten gold (1440) sold alone is quartered to 360: received.
        window.remove_buy(0);
        window.remove_sell(0);
        let row = window.add_sell(UNIT_TEN_GOLD | 1);
        assert_eq!(row.balance, -360);
        assert_eq!((row.label, row.amount), (Some(RECEIVE_LABEL), 360));
        assert_eq!(row.button_state, 0);
        assert_eq!(window.label, RECEIVE_LABEL);
        // A purse short of 144: the label keeps its text but is hidden,
        // the amount is 0, the warning shows and the button is disabled.
        window.remove_sell(0);
        window.set_money(143);
        let before = window.label;
        let row = window.add_buy(UNIT_GOLD | 1);
        assert_eq!((row.label, row.label_state), (None, NOT_DRAWN));
        assert_eq!(window.label, before);
        assert_eq!((row.amount, row.warning_state), (0, 0));
        assert_eq!(row.button_state, DISABLED);
        window.set_money(144);
        assert!(window.balance().enabled());
    }

    #[test]
    fn each_basket_change_sums_only_its_own_basket() {
        let mut window = ShopWindow::new(1000, false);
        window.buy.push(40);
        window.sell.push(40);
        // Only the sell basket's callback ran: the buy total is still 0.
        let row = window.sell_changed();
        assert_eq!((row.buy, row.sell, row.balance), (0, 10, -10));
        let row = window.buy_changed();
        assert_eq!((row.buy, row.balance), (40, 30));
    }

    #[test]
    fn the_character_view_sends_items_to_the_pack() {
        let mut window = ShopWindow::new(10, false);
        window.take_off(3, 7);
        window.take_off(3, -1);
        assert_eq!(window.drop_on_character(KEPT_KIND, 9), None);
        assert_eq!(window.drop_on_character(2, -5), Some(true));
        assert_eq!(window.drop_on_character(2, 11), Some(true));
        assert_eq!(window.pack, [(3, 7), (2, 11)]);
    }

    #[test]
    fn a_right_release_hides_and_the_close_ends_the_trade() {
        let mut window = ShopWindow::new(0, false);
        assert_eq!(window.press(0x1002), ShopReply::Form);
        assert_eq!(window.press(0x2000), ShopReply::Form);
        assert!(window.shown);
        assert_eq!(window.press(0x2002), ShopReply::Close);
        assert!(!window.shown);
        let mut window = ShopWindow::new(0, false);
        window.add_buy(1);
        window.close();
        assert!(!window.trading && !window.character_view && !window.shown);
        // The close leaves the baskets to the form.
        assert_eq!(window.buy, [1]);
    }

    #[test]
    fn confirming_spends_through_the_company() {
        let mut company = Company::new("Tester");
        company.copper = 200;
        let mut window = ShopWindow::new(company.copper, false);
        window.add_buy(UNIT_GOLD | 1);
        window.add_sell(UNIT_SILVER | 2);
        assert!(window.confirm(&mut company));
        // 144 − 6 = 138 copper spent.
        assert_eq!((company.copper, company.spent), (62, 138));
        assert!(window.buy.is_empty() && window.sell.is_empty());
        assert_eq!(window.money, 62);
        // Empty baskets: the button is disabled.
        assert!(!window.confirm(&mut company));
        // A short purse is refused and the baskets stay.
        let mut poor = Company::new("Poor");
        poor.copper = 10;
        let mut window = ShopWindow::new(10, false);
        window.add_buy(UNIT_GOLD | 1);
        assert!(!window.confirm(&mut poor));
        assert_eq!(poor.copper, 10);
        assert_eq!(window.buy, [UNIT_GOLD | 1]);
    }
}
