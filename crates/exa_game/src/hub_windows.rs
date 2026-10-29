//! The hub's windows on a [`GuiTask`]: the match board, Manage Company and
//! the trade window, drawn through the GUI draw path.
//!
//! The screens' own rules are [`exa_ui::match_board`], [`crate::manage`] and
//! [`crate::shop_window`]. This module opens them from [`crate::arena_hub`]'s
//! click hook, keeps them on the window manager's task and paints their
//! frames, tables and stock through [`crate::gui_paint`] and
//! [`exa_ui::gui_draw`], as the in-game screens do. The board's paint list is
//! the board's own ([`Board::paint`]); the manage window's roster rows are
//! [`ManageRefresh::roster`]; the trade window's panels and bottom row are
//! [`ShopWindow`]'s.
//!
//! The play-mode hub has no loaded session yet, so [`ArenaCareer`] begins as
//! a new career would (343 copper, day 0) until a session host replaces it.
//! The game's own career is not fabricated beyond those constructor values.
//!
//! What is not drawn: the board's pictures, prize items and reward slots,
//! the manage window's controls and report column text, the trade window's
//! item icons and character preview, and the two windows' own close
//! controls. Escape and a right-button release close an open window.

use std::collections::HashMap;

use bevy::prelude::*;
use exa_ui::form::{ClientRect, FadeStep, FormFade, UNPLACED, place_unset};
use exa_ui::gui_controls::Control;
use exa_ui::gui_core::{FormHost, FormId, GuiRect, GuiTask, NO_CONTROL};
use exa_ui::gui_draw::{GuiFrames, GuiTextures};
use exa_ui::line_skin::shown_palette;
use exa_ui::match_board::{
    Board, BoardAction, BoardPaint, BoardTab, CellValue, CoinDraw, Rect, TableLayout, TableSlot,
    Widget,
};
use exa_ui::skin::SkinPalette;

use crate::arena;
use crate::arena::Roster;
use crate::arena_board;
use crate::career::{Company, Member, Profession};
use crate::gui_paint::{self, Frame};
use crate::manage::{ManageDay, ManageRecord, ManageRefresh, RosterRowText, manage_refresh};
use crate::report::ReportFighter;
use crate::shop_window::ShopWindow;

/// The board's form title.
pub const BOARD_TITLE: &str = exa_ui::match_board::TITLE;
/// Manage Company's size in pixels, from its constructor.
pub const MANAGE_SIZE: (i32, i32) = (0x2c4, 0x1d4);
/// The roster table's place and size in Manage Company.
pub const MANAGE_ROSTER: (i32, i32, i32, i32) = (0, 0x18, 0x158, 0x128);
/// The report table's place and size in Manage Company.
pub const MANAGE_REPORT: (i32, i32, i32, i32) = (0x15c, 0x18, 0x158, 0x128);
/// The trade window's size in pixels.
pub const SHOP_SIZE: (i32, i32) = (
    crate::shop_window::WINDOW_SIZE.0,
    crate::shop_window::WINDOW_SIZE.1,
);
/// The row height the roster and stock rows use. The original's row pitch is
/// not in the packs, so this is a guess from the table heights.
pub const ROW_HEIGHT: i32 = 26;
/// The centred placement word the hub's forms use. The constructors' own
/// placement words are not traced, so centring is a guess.
pub const CENTRE: u32 = 0x8000 | 0x800;
/// The form's title bit. The three windows carry a title string; whether
/// their constructor words set this bit is a guess.
pub const TITLE_BIT: u32 = 1;
/// The button style the screens' constructors share.
pub const BUTTON_STYLE: u32 = 0x40_3320;

/// The career the hub's windows read. A session host replaces the default
/// with the loaded save's company.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct ArenaCareer {
    pub company: Company,
    pub player: Member,
    pub entries: Roster,
    pub log: crate::arena_log::ArenaLog,
    pub matches_today: u32,
    pub picks: crate::arena_match::ArenaPicks,
    /// The merchant's own stock, apart from the shop's shelf.
    pub merchant_stock: Vec<u32>,
    /// The merchant's stock type: the kinds it sells, 0 (the default kinds)
    /// in a new career. The save's merchant chunk keeps it.
    pub merchant_kinds: u32,
    /// The merchant's specialisation: the rank it sells, −1 (any) in a new
    /// career. The save's merchant chunk keeps it.
    pub merchant_rank: i32,
    /// The hub's items as the physician's weapon query reads them, supplied
    /// by the host that knows the scene.
    pub venue_items: Vec<crate::arena_spawn::PlacedItem>,
    /// The roll the window actions draw from.
    pub roll: u32,
    /// The manager-entry warning is still armed: the first Begin that
    /// enters the manager shows the tip and waits for a second press.
    pub warn_manager: bool,
    /// The week's tournament field: sixteen 1-based rival company ids, all
    /// zero until drawn.
    pub rivals: [i32; crate::career::RIVAL_FIELD],
}

impl Default for ArenaCareer {
    fn default() -> Self {
        Self {
            company: Company::new("The Company"),
            player: Member {
                name: "Manager".to_string(),
                profession: Profession::Combatant,
                xp: 0,
                skills: [0; 6],
                skill_xp: [0; 6],
                techniques: [[0; 6]; 6],
                loadouts: Default::default(),
                health: u16::MAX,
                in_entry: false,
            },
            entries: Roster::default(),
            log: crate::arena_log::ArenaLog::default(),
            matches_today: 0,
            picks: crate::arena_match::ArenaPicks::default(),
            merchant_stock: Vec::new(),
            merchant_kinds: 0,
            merchant_rank: -1,
            venue_items: Vec::new(),
            roll: 0,
            warn_manager: true,
            rivals: [0; crate::career::RIVAL_FIELD],
        }
    }
}

impl ArenaCareer {
    /// The week's tournament field. A field whose first company is 0 asks
    /// for a new one, drawn from the career's roll (16 of the 19 rivals), as
    /// the tournament chunk's load does.
    pub fn rival_field(&mut self) -> [i32; crate::career::RIVAL_FIELD] {
        if self.rivals[0] == 0 {
            self.rivals = crate::career::draw_rival_field(&mut self.roll);
        }
        self.rivals
    }

    /// The board from this career's company, player, day and daily rows.
    pub fn board(&self) -> Board {
        let rows = arena::generate_daily_board(0, || 0.5);
        let mut board = arena_board::open(&self.company, &self.player, self.company.day, &rows);
        board.matches_played = self.matches_today as i32;
        board.show();
        board
    }

    /// The manage window's refresh from this career.
    pub fn manage(&self) -> ManageRefresh {
        let manager = record_of(&self.player, 0, false);
        let employees: Vec<ManageRecord> = self
            .company
            .roster
            .iter()
            .enumerate()
            .map(|(index, member)| record_of(member, index.wrapping_add(1) as u8, false))
            .collect();
        let day = ManageDay {
            day: self.company.day,
            money: self.company.copper,
            earned: self.company.earned,
            spent: self.company.spent,
            fighters: employees.len() as u32 + 1,
            end_day: crate::arena_match::day_can_end(
                self.company.day,
                &self.entries,
                self.matches_today,
            ),
            alpha: 255,
        };
        manage_refresh(day, &manager, &employees, &self.log.records, &[], |_| None)
    }
}

fn name16(name: &str) -> [u8; 16] {
    let mut out = [0u8; 16];
    let bytes = name.as_bytes();
    let len = bytes.len().min(15);
    out[..len].copy_from_slice(&bytes[..len]);
    out
}

fn record_of(member: &Member, id: u8, controlled: bool) -> ManageRecord {
    ManageRecord {
        fighter: ReportFighter {
            name: name16(&member.name),
            id,
            skill_xp: member.skill_xp,
            techniques: member.techniques,
        },
        profession: member.profession,
        health: member.health,
        controlled,
    }
}

/// The item names the trade window's stock shows, by object-database id.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct HubItemNames(pub HashMap<u32, String>);

impl HubItemNames {
    /// The name of an id, or its bare number when the database has no name.
    pub fn name_of(&self, id: u32) -> String {
        self.0.get(&id).cloned().unwrap_or_else(|| id.to_string())
    }
}

/// Which hub window is open.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HubWindowKind {
    #[default]
    None,
    Board,
    Manage,
    Shop,
}

/// The shop window and the stock it lists.
#[derive(Clone, Debug, PartialEq)]
pub struct ShopModel {
    pub window: ShopWindow,
    pub merchant: bool,
    /// The stock's ids and names, in shelf order.
    pub stock: Vec<(u32, String)>,
}

/// A hub window's form: its rectangle and the controls the window manager
/// hits. The close mark is the form's own.
pub struct HubWindowForm {
    pub parent: Option<FormId>,
    pub bounds: GuiRect,
    pub doomed: bool,
    pub selected: i32,
    pub kind: HubWindowKind,
    pub controls: Vec<Control>,
}

impl HubWindowForm {
    /// The 1-based control under a form-local point.
    pub fn control_at(&self, x: i32, y: i32) -> i32 {
        for (index, control) in self.controls.iter().enumerate() {
            if GuiRect::new(control.x, control.y, control.w, control.h).contains(x, y) {
                return index as i32 + 1;
            }
        }
        0
    }
}

impl FormHost for HubWindowForm {
    fn parent(&self) -> Option<FormId> {
        self.parent
    }
    fn bounds(&self) -> GuiRect {
        self.bounds
    }
    fn is_visible(&self) -> bool {
        true
    }
    fn hit_test(&self, x: i32, y: i32) -> i32 {
        self.control_at(x, y)
    }
    fn select(&mut self, control: i32) -> u32 {
        if self.selected == control {
            return 0;
        }
        self.selected = control;
        3
    }
    fn selected_control(&self) -> i32 {
        self.selected
    }
    fn close(&mut self, _arg: i32) {
        self.doomed = true;
    }
    fn doomed(&self) -> bool {
        self.doomed
    }
}

/// The hub's windows: the window manager, the one open, and the models the
/// pages draw.
#[derive(Resource, Default)]
pub struct HubWindows {
    pub task: GuiTask,
    pub kind: HubWindowKind,
    pub form: Option<FormId>,
    pub board: Option<Board>,
    pub manage: Option<ManageRefresh>,
    pub shop: Option<ShopModel>,
}

impl HubWindows {
    pub fn is_open(&self) -> bool {
        self.kind != HubWindowKind::None
    }

    pub fn is_kind(&self, kind: HubWindowKind) -> bool {
        self.kind == kind
    }

    /// The board's action from a form-local click.
    pub fn board_click(&mut self, point: (i32, i32)) -> BoardAction {
        self.board
            .as_mut()
            .map_or(BoardAction::None, |board| board.click(point))
    }

    /// The shop's trade button from a form-local click: the button is inside
    /// the page's bottom row.
    pub fn shop_click(&mut self, point: (i32, i32), career: &mut ArenaCareer) -> bool {
        let Some(shop) = self.shop.as_mut() else {
            return false;
        };
        // The bottom row's own record places are not traced; the guessed
        // rect is where the page's bottom-left "Trade" button draws.
        let trade = shop
            .window
            .panels()
            .last()
            .map(|_| GuiRect::new(0, SHOP_SIZE.1 - 32, 120, 28));
        if trade.is_some_and(|rect| rect.contains(point.0, point.1)) {
            let done = shop.window.confirm(&mut career.company);
            if done {
                shop.window.money = career.company.copper;
            }
            return done;
        }
        false
    }

    /// Opens the board on a career.
    pub fn open_board(&mut self, career: &ArenaCareer) {
        self.close();
        self.board = Some(career.board());
        self.kind = HubWindowKind::Board;
        self.push(
            HubWindowKind::Board,
            board_bounds(self.task.screen),
            board_controls(),
        );
    }

    /// Opens Manage Company on a career.
    pub fn open_manage(&mut self, career: &ArenaCareer) {
        self.close();
        self.manage = Some(career.manage());
        self.kind = HubWindowKind::Manage;
        self.push(
            HubWindowKind::Manage,
            centred(MANAGE_SIZE, self.task.screen),
            Vec::new(),
        );
    }

    /// Opens the trade window: the shop's shelf, or the merchant's own.
    pub fn open_shop(&mut self, merchant: bool, career: &ArenaCareer, names: &HubItemNames) {
        self.close();
        let stock = if merchant {
            &career.merchant_stock
        } else {
            &career.company.stock
        };
        let shop = ShopModel {
            window: ShopWindow::new(career.company.copper, merchant),
            merchant,
            stock: stock.iter().map(|&id| (id, names.name_of(id))).collect(),
        };
        self.shop = Some(shop);
        self.kind = HubWindowKind::Shop;
        self.push(
            HubWindowKind::Shop,
            centred(SHOP_SIZE, self.task.screen),
            Vec::new(),
        );
    }

    /// Closes the open window, running its own close bookkeeping.
    pub fn close_window(&mut self) {
        match self.kind {
            HubWindowKind::Board => {
                if let Some(board) = self.board.as_mut() {
                    board.close();
                }
            }
            HubWindowKind::Manage => {
                let _ = crate::manage::close_manage();
            }
            HubWindowKind::Shop => {
                if let Some(shop) = self.shop.as_mut() {
                    shop.window.close();
                }
            }
            HubWindowKind::None => return,
        }
        self.close();
    }

    /// Removes the form and forgets the window.
    pub fn close(&mut self) {
        if let Some(form) = self.form.take()
            && let Some(host) = self.task.form_mut(form)
        {
            host.close(0);
        }
        self.task.delete_doomed();
        self.task.root = None;
        self.task.active = None;
        self.task.active_control = NO_CONTROL;
        self.kind = HubWindowKind::None;
        self.board = None;
        self.manage = None;
        self.shop = None;
    }

    fn push(&mut self, kind: HubWindowKind, bounds: GuiRect, controls: Vec<Control>) {
        let parent = self.task.root;
        let form = self.task.add(Box::new(HubWindowForm {
            parent,
            bounds,
            doomed: false,
            selected: 0,
            kind,
            controls,
        }));
        self.form = Some(form);
        self.task.focus_set(Some(form), NO_CONTROL);
    }

    /// The open form's rectangle.
    pub fn bounds(&self) -> Option<GuiRect> {
        self.form
            .and_then(|form| self.task.form(form))
            .map(FormHost::bounds)
    }
}

/// The board's controls: the four tabs and the Enter button.
fn board_controls() -> Vec<Control> {
    let layout = exa_ui::match_board::layout(false);
    let mut records = Vec::new();
    for (index, tab) in layout.tabs.iter().enumerate() {
        let mut control = Control::new(1, tab.x, tab.y, tab.w, tab.h);
        control.style = BUTTON_STYLE;
        control.text = BoardTab::from_index(index)
            .map(BoardTab::label)
            .unwrap_or(BOARD_TITLE)
            .as_bytes()
            .to_vec();
        records.push(control);
    }
    let mut enter = Control::new(
        1,
        layout.enter_button.x,
        layout.enter_button.y,
        layout.enter_button.w,
        layout.enter_button.h,
    );
    enter.style = BUTTON_STYLE;
    enter.text = b"Enter".to_vec();
    records.push(enter);
    records
}

/// The board's form place: centred in the task's screen.
fn board_bounds(screen: (i32, i32)) -> GuiRect {
    let layout = exa_ui::match_board::layout(false);
    centred((layout.form.w, layout.form.h), screen)
}

fn centred(size: (i32, i32), screen: (i32, i32)) -> GuiRect {
    let root = ClientRect {
        left: 0,
        top: 0,
        right: screen.0,
        bottom: screen.1,
    };
    let (x, y) = place_unset(UNPLACED, UNPLACED, size.0, size.1, CENTRE, root);
    GuiRect::new(x, y, size.0, size.1)
}

/// The client an open form's content draws in.
fn client(bounds: GuiRect) -> GuiRect {
    let spacing = gui_paint::FORM_SPACING;
    GuiRect::new(
        bounds.left + spacing,
        bounds.top + spacing,
        bounds.width() - spacing * 2,
        bounds.height() - spacing * 2,
    )
}

fn at(bounds: GuiRect, rect: Rect) -> GuiRect {
    GuiRect::new(bounds.left + rect.x, bounds.top + rect.y, rect.w, rect.h)
}

fn at4(bounds: GuiRect, rect: (i32, i32, i32, i32)) -> GuiRect {
    GuiRect::new(bounds.left + rect.0, bounds.top + rect.1, rect.2, rect.3)
}

/// The font and unit a hub window draws with, as the screens do.
fn screen_font(
    textures: &GuiTextures,
    unit: i32,
) -> (exa_ui::gui_font::GuiFont, Option<(u16, u16)>) {
    let level = exa_ui::gui_font::level_for_unit(unit);
    crate::gui_screens::screen_font(exa_ui::gui_font::gui_level(level).main, textures)
}

/// What a hub window paints with: the palette its fade hands the skin, the
/// fade itself (for the line skin's control inks), the GUI unit and the
/// textures.
struct Look<'a> {
    palette: SkinPalette,
    fade: f32,
    unit: i32,
    textures: &'a GuiTextures,
}

impl<'a> Look<'a> {
    fn new(fade: f32, unit: i32, textures: &'a GuiTextures) -> Self {
        Self {
            palette: shown_palette(fade),
            fade,
            unit,
            textures,
        }
    }
}

/// Whether a window's opener shows it with a fade. The board's and Manage
/// Company's openers clear the new form's visible flag before they show
/// it, so the show starts the base form's 256-tick fade (mode 0, which
/// hands its alpha to the skin); the trade window's opener only adds the
/// form, which the base constructor left visible, so it draws at 1 from
/// the first frame.
pub fn fades_in(kind: HubWindowKind) -> bool {
    matches!(kind, HubWindowKind::Board | HubWindowKind::Manage)
}

/// The open window's show fade: which form it belongs to and the base
/// form's fade state.
#[derive(Debug, Default)]
pub struct HubFade {
    form: Option<FormId>,
    fade: FormFade,
}

impl HubFade {
    /// The fade `form` draws at, at GUI clock `now`: a form seen for the
    /// first time starts its show fade there when its kind fades in; the
    /// draw takes the fade's alpha while it runs and 1 otherwise.
    pub fn alpha(&mut self, form: FormId, kind: HubWindowKind, now: u32) -> f32 {
        if self.form != Some(form) {
            self.form = Some(form);
            self.fade = FormFade::default();
            if fades_in(kind) {
                self.fade.start_show(now, false);
            }
        }
        match self.fade.step(now) {
            FadeStep::Draw(Some(alpha)) => alpha,
            _ => 1.0,
        }
    }
}

/// The window's frame.
fn paint_window(painted: &mut gui_paint::Painted, bounds: GuiRect, title: &str, look: &Look) {
    let (font, size) = screen_font(look.textures, look.unit);
    let mut frame = Frame::new(bounds, look.unit, title);
    frame.attributes |= TITLE_BIT;
    for layer in gui_paint::paint_frame(&frame, &look.palette, &font, size).layers {
        painted.push(layer);
    }
}

fn paint_text(painted: &mut gui_paint::Painted, rect: GuiRect, text: &str, look: &Look) {
    let (font, size) = screen_font(look.textures, look.unit);
    for layer in gui_paint::paint_label(rect, 0, look.unit, &look.palette, &font, size, text).layers
    {
        painted.push(layer);
    }
}

fn paint_text_ink(
    painted: &mut gui_paint::Painted,
    rect: GuiRect,
    text: &str,
    ink: Option<[u8; 4]>,
    look: &Look,
) {
    let (font, size) = screen_font(look.textures, look.unit);
    let unit = look.unit;
    for layer in
        gui_paint::paint_label_ink(rect, 0, unit, &look.palette, &font, size, text, ink).layers
    {
        painted.push(layer);
    }
}

fn paint_panel(painted: &mut gui_paint::Painted, rect: GuiRect, look: &Look) {
    let attributes = gui_paint::FORM_ATTRIBUTES;
    for layer in gui_paint::paint_panel(rect, look.unit, &look.palette, attributes, 0).layers {
        painted.push(layer);
    }
}

/// The board's paint list through the GUI path.
fn paint_board(board: &Board, bounds: GuiRect, look: &Look) -> gui_paint::Painted {
    let mut painted = gui_paint::Painted {
        client: client(bounds),
        ..Default::default()
    };
    paint_window(&mut painted, bounds, BOARD_TITLE, look);
    let unit = look.unit;
    let (font, size) = screen_font(look.textures, unit);
    for item in board.paint() {
        match item {
            BoardPaint::Frame {
                widget: Widget::Form,
                ..
            } => {}
            BoardPaint::Frame { rect, .. } => {
                paint_panel(&mut painted, at(bounds, rect), look);
            }
            BoardPaint::Tab {
                rect,
                title,
                selected,
                ..
            } => {
                let button = at(bounds, rect);
                let caption = if selected {
                    format!("[{title}]")
                } else {
                    title.to_string()
                };
                // The tab records' own words are not traced: a lone
                // button whose on and lit bits stand for the selection (a
                // guess).
                let state = if selected {
                    exa_ui::gui_controls::state::ON | exa_ui::gui_controls::state::LIT
                } else {
                    0
                };
                let words = gui_paint::ButtonWords {
                    style: BUTTON_STYLE,
                    state,
                    group: 0,
                };
                for layer in gui_paint::paint_line_button(
                    button,
                    words,
                    unit,
                    &look.palette,
                    look.fade,
                    &font,
                    size,
                    &caption,
                )
                .layers
                {
                    painted.push(layer);
                }
            }
            BoardPaint::Table { rect, .. } => {
                paint_panel(&mut painted, at(bounds, rect), look);
            }
            BoardPaint::Column { rect, title, .. } => {
                paint_text(&mut painted, at(bounds, rect), title, look);
            }
            BoardPaint::Cell { rect, value, .. } => {
                let text = match value {
                    CellValue::Text(text) => text,
                    // A coin column carries a draw callback instead of a
                    // text value: the following Coin item draws its dash or
                    // its coin counts.
                    CellValue::Coin(_) => String::new(),
                    CellValue::Health(word) => {
                        format!("{:.0}%", f64::from(word) / 655.35)
                    }
                    CellValue::Empty => String::new(),
                };
                if !text.is_empty() {
                    paint_text(&mut painted, at(bounds, rect), &text, look);
                }
            }
            BoardPaint::Coin { rect, draw, .. } => {
                let (x, text) = match draw {
                    CoinDraw::Dash { x } => (x, "-".to_string()),
                    CoinDraw::Coins {
                        x,
                        copper,
                        silver,
                        gold,
                    } => (x, coin_text_parts(gold, silver, copper)),
                };
                if !text.is_empty() {
                    let cell = Rect { x, ..rect };
                    paint_text(&mut painted, at(bounds, cell), &text, look);
                }
            }
            BoardPaint::Label { rect, text, .. } => {
                if let Some(rect) = rect {
                    paint_text(&mut painted, at(bounds, rect), &text, look);
                }
            }
        }
    }
    painted
}

/// A coin amount's text as a table cell can show it: "g s c", leaving out
/// the leading zeroes. The board's coin draw gives the counts; a zero cell
/// is the dash and never reaches this.
fn coin_text_parts(gold: u32, silver: u32, copper: u32) -> String {
    match (gold, silver, copper) {
        (0, 0, coins) => format!("{coins}c"),
        (0, silver, coins) => format!("{silver}s {coins}c"),
        (gold, silver, coins) => format!("{gold}g {silver}s {coins}c"),
    }
}

/// The manage window's two tables, headers and roster rows.
fn paint_manage(refresh: &ManageRefresh, bounds: GuiRect, look: &Look) -> gui_paint::Painted {
    let mut painted = gui_paint::Painted {
        client: client(bounds),
        ..Default::default()
    };
    paint_window(&mut painted, bounds, crate::manage::MANAGE_TITLE, look);
    let roster = at4(bounds, MANAGE_ROSTER);
    let report_table = at4(bounds, MANAGE_REPORT);
    paint_panel(&mut painted, roster, look);
    paint_panel(&mut painted, report_table, look);
    let columns = [("Name", 0), ("Rank", 0x84), ("Health", 0x84 + 0x50)];
    for (title, x) in columns {
        paint_text(
            &mut painted,
            GuiRect::new(roster.left + x, roster.top, 84, ROW_HEIGHT),
            title,
            look,
        );
    }
    for (row, text) in refresh.roster.iter().enumerate() {
        let y = roster.top + ROW_HEIGHT * (row as i32 + 1);
        paint_row(&mut painted, roster.left, y, text, look);
    }
    // The report's roster-id lines are not drawn: the report table's own
    // rows carry no column labels in the constructor.
    painted
}

fn paint_row(
    painted: &mut gui_paint::Painted,
    left: i32,
    y: i32,
    text: &RosterRowText,
    look: &Look,
) {
    paint_text_ink(
        painted,
        GuiRect::new(left, y, 0x84, ROW_HEIGHT),
        &text.name.text,
        text.name.ink,
        look,
    );
    paint_text(
        painted,
        GuiRect::new(left + 0x84, y, 0x50, ROW_HEIGHT),
        &text.rank.text,
        look,
    );
    let bar = GuiRect::new(left + 0x84 + 0x50, y + 8, 0x84, ROW_HEIGHT - 16);
    paint_panel(painted, bar, look);
    let filled = (f64::from(text.health) * f64::from(bar.width())) as i32;
    if filled > 0 {
        let fill = GuiRect::new(bar.left, bar.top, filled.min(bar.width()), bar.height());
        for layer in gui_paint::paint_client_fill(fill, &look.palette).layers {
            painted.push(layer);
        }
    }
}

/// The trade window's panels, stock rows and bottom row.
fn paint_shop(shop: &ShopModel, bounds: GuiRect, look: &Look) -> gui_paint::Painted {
    let mut painted = gui_paint::Painted {
        client: client(bounds),
        ..Default::default()
    };
    paint_window(&mut painted, bounds, crate::shop_window::TRADE_TITLE, look);
    for panel in shop.window.panels() {
        paint_panel(
            &mut painted,
            at4(bounds, (panel[0], panel[1], panel[2], panel[3])),
            look,
        );
    }
    let stock = shop.window.panels()[4];
    let stock_rect = at4(bounds, (stock[0], stock[1], stock[2], stock[3]));
    for (row, (_, name)) in shop.stock.iter().enumerate() {
        let y = stock_rect.top + 4 + ROW_HEIGHT * row as i32;
        if y + ROW_HEIGHT > stock_rect.bottom {
            break;
        }
        paint_text(
            &mut painted,
            GuiRect::new(stock_rect.left + 4, y, stock_rect.width() - 8, ROW_HEIGHT),
            name,
            look,
        );
    }
    let bottom = [
        (30, 0, format!("{}", shop.window.totals.1)),
        (260, 0, format!("{}", shop.window.totals.0)),
        (430, 0, shop.window.label.to_string()),
        (520, 0, format!("{}", shop.window.shown_balance.amount)),
        (
            520,
            14,
            format!("{}{}", crate::shop_window::OWNED_LABEL, shop.window.money),
        ),
    ];
    for (x, y, text) in bottom {
        paint_text(
            &mut painted,
            GuiRect::new(bounds.left + x, bounds.bottom - 28 + y, 120, 20),
            &text,
            look,
        );
    }
    painted
}

/// Keeps the hub task's screen on the window's physical size, as the
/// in-game screens do.
pub fn sync_hub_screen(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut hub: ResMut<HubWindows>,
) {
    if let Ok(window) = windows.single() {
        let size = (
            window.physical_width() as i32,
            window.physical_height() as i32,
        );
        if size != hub.task.screen {
            hub.task.screen = size;
        }
    }
}

/// Draws the open hub window into the GUI's queue.
pub fn draw_hub_windows(
    windows: Option<Res<HubWindows>>,
    textures: Option<Res<GuiTextures>>,
    screen: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut frames: ResMut<GuiFrames>,
    time: Option<Res<Time>>,
    mut fade: Local<HubFade>,
) {
    let Some(windows) = windows else {
        return;
    };
    if !windows.is_open() {
        // A reopened window fades in again even if its form id comes back.
        fade.form = None;
        return;
    }
    frames.clear();
    let Some(bounds) = windows.bounds() else {
        return;
    };
    let empty = GuiTextures::default();
    let textures = textures.as_deref().unwrap_or(&empty);
    let unit = screen
        .single()
        .ok()
        .map(|window| exa_ui::scale::gui_scale(window.physical_height(), 0) as i32)
        .unwrap_or(4);
    let now = time.map_or(0, |time| time.elapsed().as_millis() as u32);
    let alpha = windows
        .form
        .map_or(1.0, |form| fade.alpha(form, windows.kind, now));
    let look = Look::new(alpha, unit, textures);
    let painted = match windows.kind {
        HubWindowKind::Board => windows
            .board
            .as_ref()
            .map(|board| paint_board(board, bounds, &look)),
        HubWindowKind::Manage => windows
            .manage
            .as_ref()
            .map(|manage| paint_manage(manage, bounds, &look)),
        HubWindowKind::Shop => windows
            .shop
            .as_ref()
            .map(|shop| paint_shop(shop, bounds, &look)),
        HubWindowKind::None => None,
    };
    if let Some(painted) = painted {
        painted.submit(&mut frames);
    }
}

/// Lays the board's painter rects out in a form-local frame for tests.
pub fn board_cell_rects(board: &Board) -> Vec<(TableSlot, Rect)> {
    board
        .paint()
        .into_iter()
        .filter_map(|item| match item {
            BoardPaint::Cell { slot, rect, .. } => Some((slot, rect)),
            _ => None,
        })
        .collect()
}

/// The table layout the painter uses, for tests.
pub fn table_layout(board: &Board, slot: TableSlot) -> TableLayout {
    board.table_layout(slot)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena_hub::{ARENA_MODE, HubTarget, hub_click};
    use crate::arena_spawn::generic_plan;
    use crate::career::Profession;

    #[test]
    fn the_board_and_manage_fade_in_and_the_trade_window_does_not() {
        let mut fade = HubFade::default();
        // The board opens at clock 1000: alpha 0, half way at 1128, 1 once
        // its 256 ticks pass.
        assert_eq!(fade.alpha(7, HubWindowKind::Board, 1000), 0.0);
        let half = fade.alpha(7, HubWindowKind::Board, 1128);
        assert_eq!(half, 0.5);
        let look = shown_palette(half);
        assert_eq!((look.form[3][3], look.text[6][3]), (85, 62));
        assert_eq!(
            exa_ui::line_skin::line_control_inks(half),
            [
                [255, 250, 230, 40],
                [255, 250, 230, 52],
                [255, 250, 230, 64]
            ]
        );
        assert_eq!(fade.alpha(7, HubWindowKind::Board, 1256), 1.0);
        assert_eq!(fade.alpha(7, HubWindowKind::Board, 5000), 1.0);
        // Manage Company, a new form, fades again; the trade window does
        // not.
        assert_eq!(fade.alpha(8, HubWindowKind::Manage, 6000), 0.0);
        assert_eq!(fade.alpha(8, HubWindowKind::Manage, 6064), 0.25);
        assert_eq!(fade.alpha(9, HubWindowKind::Shop, 7000), 1.0);
        let shown = shown_palette(1.0);
        assert_eq!((shown.form[3][3], shown.control[0][3]), (170, 125));
    }

    fn career() -> ArenaCareer {
        let mut career = ArenaCareer::default();
        career.company.name = "Ours".into();
        career
    }

    #[test]
    fn a_new_career_has_a_company_and_its_board_opens() {
        let career = career();
        let board = career.board();
        assert_eq!(board.company_name, "Ours");
        assert_eq!(board.fighters.len(), 1);
        assert_eq!(board.fighters[0].name, "Manager");
        assert!(!board.events[0].name.is_empty());
        assert_eq!(board.matches_played, 0);
    }

    #[test]
    fn a_coin_draw_shows_its_counts_and_zero_is_the_dash() {
        assert_eq!(coin_text_parts(0, 0, 4), "4c");
        assert_eq!(coin_text_parts(0, 3, 4), "3s 4c");
        assert_eq!(coin_text_parts(2, 3, 4), "2g 3s 4c");
        // A zero amount is never formatted: the board's coin draw is the
        // dash at its offset instead.
        let zero = exa_ui::match_board::Board::coin_draw(0, 10);
        assert_eq!(zero, exa_ui::match_board::CoinDraw::Dash { x: 14 });
    }

    #[test]
    fn the_manage_refresh_lists_the_manager_first() {
        let mut career = career();
        career.company.roster.push(Member {
            name: "Anna".into(),
            profession: Profession::Combatant,
            xp: 0,
            skills: [0; 6],
            skill_xp: [0; 6],
            techniques: [[0; 6]; 6],
            loadouts: Default::default(),
            health: 65_000,
            in_entry: false,
        });
        let refresh = career.manage();
        assert_eq!(refresh.roster.len(), 2);
        assert_eq!(refresh.roster[0].name.text, "Manager");
        assert_eq!(refresh.roster[1].name.text, "Anna");
        assert_eq!(refresh.roster[0].rank.text, "Inept+");
        assert_eq!(refresh.money, 343);
    }

    #[test]
    fn the_board_and_manage_open_on_forms_and_close() {
        let mut windows = HubWindows::default();
        let career = career();
        windows.open_board(&career);
        assert!(windows.is_kind(HubWindowKind::Board));
        assert!(windows.form.is_some());
        assert_eq!(windows.bounds().unwrap().width(), 604);
        windows.close_window();
        assert!(!windows.is_open());

        windows.open_manage(&career);
        assert!(windows.is_kind(HubWindowKind::Manage));
        assert_eq!(
            (
                windows.bounds().unwrap().width(),
                windows.bounds().unwrap().height()
            ),
            MANAGE_SIZE
        );
        windows.close_window();
        assert!(!windows.is_open());
    }

    #[test]
    fn the_shop_lists_its_shelf_and_the_merchant_his_own() {
        let mut career = career();
        career.company.stock = vec![5, 6];
        career.merchant_stock = vec![9];
        let mut names = HubItemNames::default();
        names.0.insert(5, "Sword".into());
        names.0.insert(9, "Potion".into());
        let mut windows = HubWindows::default();
        windows.open_shop(false, &career, &names);
        let shop = windows.shop.as_ref().unwrap();
        assert_eq!(shop.stock, [(5, "Sword".into()), (6, "6".into())]);
        assert!(!shop.merchant);
        windows.close_window();
        windows.open_shop(true, &career, &names);
        let shop = windows.shop.as_ref().unwrap();
        assert_eq!(shop.stock, [(9, "Potion".into())]);
        assert!(shop.merchant);
    }

    #[test]
    fn the_board_click_opens_the_match_and_its_tabs_switch() {
        let mut windows = HubWindows::default();
        let career = career();
        windows.open_board(&career);
        let layout = exa_ui::match_board::layout(false);
        let action = windows.board_click((layout.tabs[2].x + 1, layout.tabs[2].y + 1));
        assert_eq!(action, BoardAction::Changed);
        assert_eq!(windows.board.as_ref().unwrap().tab, BoardTab::Tournament);
    }

    #[test]
    fn the_board_enter_in_a_duel_asks_for_the_match() {
        let mut windows = HubWindows::default();
        let career = career();
        windows.open_board(&career);
        let layout = exa_ui::match_board::layout(false);
        let action = windows.board_click((layout.enter_button.x + 1, layout.enter_button.y + 1));
        assert_eq!(action, BoardAction::EnterMatch);
    }

    #[test]
    fn the_shop_trade_button_settles_through_the_company() {
        let mut career = career();
        career.company.stock = vec![7];
        let names = HubItemNames::default();
        let mut windows = HubWindows::default();
        windows.open_shop(false, &career, &names);
        windows.shop.as_mut().unwrap().window.add_buy(3);
        let point = (10, SHOP_SIZE.1 - 20);
        assert!(windows.shop_click(point, &mut career));
        assert_eq!(career.company.copper, 340);
    }

    #[test]
    fn a_board_cell_rect_is_pinned() {
        let career = career();
        let board = career.board();
        let rects = board_cell_rects(&board);
        let first = rects.first().unwrap();
        assert_eq!(first.0, TableSlot::Match);
        assert_eq!(
            first.1,
            Rect {
                x: 0,
                y: 56,
                w: 196,
                h: exa_ui::match_board::ROW_HEIGHT
            }
        );
        assert_eq!(table_layout(&board, TableSlot::Roster).rect.w, 292);
    }

    #[test]
    fn the_window_controls_hit_the_tabs_and_enter() {
        let career = career();
        let mut windows = HubWindows::default();
        windows.open_board(&career);
        let form = windows.task.form(windows.form.unwrap()).unwrap();
        let layout = exa_ui::match_board::layout(false);
        assert_eq!(form.hit_test(layout.tabs[0].x + 1, layout.tabs[0].y + 1), 1);
        assert_eq!(
            form.hit_test(layout.enter_button.x + 1, layout.enter_button.y + 1),
            5
        );
        assert_eq!(form.hit_test(2, 2), 0);
    }

    #[test]
    fn the_hub_click_hook_and_the_windows_agree_on_the_side() {
        let shop = crate::arena_hub::SHOP_NAME;
        assert_eq!(
            hub_click(
                false,
                ARENA_MODE,
                false,
                HubTarget::Object {
                    name: &shop,
                    side: 0.0
                }
            ),
            crate::arena_hub::HubAction::Shop
        );
        // A generic opponent's plan is the level it was rolled at.
        assert_eq!(generic_plan(3, 1).level, 3);
        assert_eq!(crate::report::name_text(&name16("Ash")), "Ash");
    }

    #[test]
    fn the_draw_path_runs_for_three_frames_with_a_board_open() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<GuiFrames>()
            .init_resource::<HubWindows>();
        app.add_systems(Update, draw_hub_windows);
        for _ in 0..3 {
            app.update();
        }
        // With no hub window the pass leaves another screen's frames alone.
        app.world_mut()
            .resource_mut::<GuiFrames>()
            .push(exa_ui::gui_draw::GuiFrame::new());
        app.update();
        assert_eq!(app.world().resource::<GuiFrames>().frames.len(), 1);
        let mut windows = HubWindows::default();
        windows.open_board(&career());
        app.world_mut().insert_resource(windows);
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(app.world().resource::<GuiFrames>().frames.len(), 1);
    }
}
