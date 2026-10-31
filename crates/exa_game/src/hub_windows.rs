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
//! Manage Company and the windows it opens (Company Records, Company
//! Status, the loadout form and its dismiss dialog, and the yes/no dialog)
//! are built from their constructors' record tables ([`crate::hub_forms`]).
//! The board's form is built the same way from its template's records
//! ([`exa_ui::match_board::TEMPLATE_RECORDS`]): the frame, tabs, bottom row,
//! captions and coin controls draw as those windows' records do, and the
//! board's paint list draws its child views in the form's client.
//!
//! What is not drawn: the board's pictures, prize items and reward slots
//! (drawn as panels), the trade window's item icons and character preview,
//! and the trade window's own close control. Escape and a right-button release
//! close the top window.

use std::collections::HashMap;

use bevy::prelude::*;
use exa_ui::form::{ClientRect, FadeStep, FormFade, UNPLACED, place_unset};
use exa_ui::gui_controls::Control;
use exa_ui::gui_core::{FormHost, FormId, GuiRect, GuiTask, NO_CONTROL};
use exa_ui::gui_draw::{GuiFrames, GuiTextures};
use exa_ui::line_skin::shown_palette;
use exa_ui::match_board::{
    Board, BoardAction, BoardPaint, CellValue, CloseState, CoinDraw, Rect, TableLayout, TableSlot,
    Widget,
};
use exa_ui::skin::SkinPalette;
use exa_ui::table_view;

use crate::arena::Roster;
use crate::arena_board;
use crate::career::{Company, Member, Profession};
use crate::gui_paint::{self, Frame};
use crate::manage::{ManageDay, ManageRecord, ManageRefresh, manage_refresh};
use crate::report::ReportFighter;
use crate::shop_window::ShopWindow;

/// The board's form title.
pub const BOARD_TITLE: &str = exa_ui::match_board::TITLE;
/// Manage Company's size in pixels, from its constructor.
pub const MANAGE_SIZE: (i32, i32) = exa_ui::arena_forms::MANAGE_FORM.size;
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
    /// Items the company holds off the shelf: a purchase joins them and a
    /// sale takes them. The shop lists them in the pack.
    pub goods: Vec<u32>,
    /// Set when a confirmed trade changed [`Self::goods`], so the player's
    /// carried list can be written back.
    pub goods_changed: bool,
    /// The merchant's stock type: the kinds it sells, 0 (the default kinds)
    /// in a new career. The save's merchant chunk keeps it.
    pub merchant_kinds: u32,
    /// The merchant's specialisation: the rank it sells, −1 (any) in a new
    /// career. The save's merchant chunk keeps it.
    pub merchant_rank: i32,
    /// The hub's items as the physician's weapon query reads them, supplied
    /// by the host that knows the scene.
    pub venue_items: Vec<crate::arena_spawn::PlacedItem>,
    /// The hire list: ten candidates, newest first, each a generated record
    /// with the role the candidate draw gave it.
    pub hire_pool: Vec<Member>,
    /// The profession grid's drift, shared across the list's generations.
    pub hire_catalog: crate::arena_outfit::ProfessionCatalog,
    /// The appearance variant the next candidate avoids.
    pub hire_previous: u32,
    /// The day's six standard matches, rolled once and kept until the day
    /// ends ([`crate::arena::generate_daily_board`]).
    pub daily_rows: [crate::arena::MatchInstance; 6],
    /// The day ended and its shop and merchant shelves still need their
    /// nightly restock, which the live shop world supplies on the next
    /// frame.
    pub restock_pending: bool,
    /// The week's tournament board: six event rows, its prize and the
    /// shuffled company field.
    pub tournament: Option<crate::arena::TournamentBoard>,
    /// The last eight weekly prizes, so a new draw avoids them.
    pub prize_history: [u32; 8],
    /// The run-time library's unit generator (a Mersenne twister apart from
    /// the engine's LCG). A candidate's level and the boards' rank rolls
    /// read it in turn, as the original's one shared library sequence. The
    /// original seeds it once from the clock at start-up; a default career
    /// seeds it from a fixed word [guess].
    pub random: crate::arena::Mt19937,
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
        let mut career = Self {
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
            goods: Vec::new(),
            goods_changed: false,
            merchant_kinds: 0,
            merchant_rank: -1,
            venue_items: Vec::new(),
            hire_pool: Vec::new(),
            hire_catalog: crate::arena_outfit::ProfessionCatalog::default(),
            hire_previous: 0,
            daily_rows: crate::arena::generate_daily_board(0x1234_5678, || 0.5),
            restock_pending: false,
            tournament: None,
            prize_history: [0; 8],
            random: crate::arena::Mt19937::new(DEFAULT_RANDOM_SEED),
            roll: 0,
            warn_manager: true,
            rivals: [0; crate::career::RIVAL_FIELD],
        };
        // A default career is what the play-mode hub starts with: ten
        // candidates, drawn from a fixed seed until a session host replaces
        // the career.
        career.fill_hire_pool_at(0x1234_5678);
        career.regenerate_tournament_at(0x1234_5678);
        career
    }
}

impl ArenaCareer {

    /// Copy the player's loose items into the goods when the shop has not
    /// traded yet. A later open keeps the goods the trade left.
    pub fn seed_goods(&mut self, carried: impl IntoIterator<Item = u32>) {
        if self.goods.is_empty() {
            self.goods
                .extend(carried.into_iter().filter(|id| *id != 0));
        }
    }

    /// The carried list after a trade: items still held keep their cells,
    /// and a purchase is appended at the pack's origin.
    pub fn carried_after_trade(
        old: &[exa_items::possession::Carried],
        goods: &[u32],
    ) -> Vec<exa_items::possession::Carried> {
        goods
            .iter()
            .map(|&id| {
                old.iter().find(|carried| carried.id == id).copied().unwrap_or(
                    exa_items::possession::Carried {
                        id,
                        flags: 0,
                        x: 0,
                        y: 0,
                    },
                )
            })
            .collect()
    }

    /// The arena's single reset, as a new career and an arena slot load
    /// run it before anything else: the shop's shelf, the merchant's stock
    /// (kinds 0, rank −1, any), the company's record words and the day's
    /// tallies, the day, today's matches, the log, the roster, the purse
    /// and the season's week verdicts are cleared. The company's name and
    /// emblem, the manager's record, the tournament entries, the season
    /// rewards, the draw state and the rival field are kept: the original's
    /// reset does not touch them (the load and the new career write them).
    /// The day's specials it generates last are drawn when the board opens
    /// ([`ArenaCareer::board`]). The stores beside the career are
    /// [`crate::save_live::ArenaStores::reset`]'s.
    pub fn reset(&mut self) {
        self.company.stock.clear();
        self.merchant_stock.clear();
        self.goods.clear();
        self.goods_changed = false;
        self.merchant_kinds = 0;
        self.merchant_rank = -1;
        self.company
            .book_records(&crate::arena_match::CompanyRecords::default());
        self.company.earned = 0;
        self.company.spent = 0;
        self.company.day = 0;
        self.matches_today = 0;
        self.log.records.clear();
        self.restock_pending = false;
        self.company.roster.clear();
        self.company.copper = 0;
        self.company.weeks = [false; 4];
        // The reset leaves the hire list and the week's event rows alone and
        // ends by rolling the day's six matches.
        self.regenerate_daily_board_at(crate::arena_match::counter_now() as u32);
    }

    /// A new career's records, in the original's order: the reset (which
    /// rolls the day's matches), the starting purse of 343 copper, the
    /// week's tournament and then the ten hire-list candidates.
    pub fn start_new_career(&mut self) {
        self.reset();
        self.company.copper = crate::career::STARTING_COPPER;
        self.regenerate_tournament_at(crate::arena_match::counter_now() as u32);
        self.fill_hire_pool_at(crate::arena_match::counter_now());
    }

    /// Roll the week's tournament from `seed`: the event entries are
    /// cleared, then its prize (avoiding the last eight), six event rows at
    /// tier 5 down and the sixteen-company field, which becomes the career's
    /// rival field. The rank rolls read the library generator.
    pub fn regenerate_tournament_at(&mut self, seed: u32) {
        self.entries = Roster::default();
        let random = &mut self.random;
        let board = crate::arena::generate_tournament_board(seed, &mut self.prize_history, || {
            random.unit()
        });
        self.rivals = board.companies.map(i32::from);
        self.tournament = Some(board);
    }

    /// Roll the day's six standard matches from `seed`, the board's LCG
    /// seed; the rank rolls read the library generator.
    pub fn regenerate_daily_board_at(&mut self, seed: u32) {
        let random = &mut self.random;
        self.daily_rows = crate::arena::generate_daily_board(seed, || random.unit());
    }

    /// Fill the hire list with ten candidates. The original reseeds from the
    /// performance counter before each candidate; the live counter is read
    /// once and each later candidate's seed is moved on by a fixed odd step,
    /// so two candidates never share a seed [guess: the real counter's
    /// spacing depends on the machine].
    pub fn fill_hire_pool_at(&mut self, counter: u64) {
        let mut pool = Vec::with_capacity(crate::hirelings::POOL_SIZE);
        for index in 0..crate::hirelings::POOL_SIZE {
            let seed = counter.wrapping_add((index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
            pool.push(draw_hire_member(
                seed,
                &mut self.hire_catalog,
                &mut self.hire_previous,
                &mut self.random,
            ));
        }
        self.hire_pool = pool;
    }

    /// Fill the hire list from the live counter.
    pub fn fill_hire_pool(&mut self) {
        self.fill_hire_pool_at(crate::arena_match::counter_now());
    }

    /// Renew the list for a new day: the oldest candidate leaves, and one or
    /// two come while fewer than eight are listed ([`crate::hirelings::renew_pool`]).
    /// The count draw steps the career's roll; the original steps the
    /// engine's shared LCG wherever the restock left it [guess].
    pub fn renew_hire_pool(&mut self) {
        if self.hire_pool.is_empty() {
            self.fill_hire_pool();
            return;
        }
        let mut pool = std::mem::take(&mut self.hire_pool);
        let mut catalog = std::mem::take(&mut self.hire_catalog);
        let mut previous = self.hire_previous;
        let random = &mut self.random;
        let mut counter = crate::arena_match::counter_now();
        crate::hirelings::renew_pool(&mut pool, &mut self.roll, || {
            let member = draw_hire_member(counter, &mut catalog, &mut previous, random);
            counter = counter.wrapping_add(0x9E37_79B9_7F4A_7C15);
            member
        });
        self.hire_catalog = catalog;
        self.hire_previous = previous;
        self.hire_pool = pool;
    }

    /// What a listed candidate costs: the rank from its six skill levels and
    /// the hire fee. The live list carries no item facts, so the outfit's
    /// gear value is not added (a gap against the original, whose candidate
    /// is dressed from the object database).
    pub fn hire_price(&self, index: usize) -> u32 {
        self.hire_pool.get(index).map_or(0, |member| {
            crate::career::Company::listed_price(&member.skills, &[])
        })
    }

    /// Hire the candidate at `index`: the company's own funds, room and
    /// duplicate checks, then the joining record and the removal from the
    /// list. The candidate's skills and techniques come across. Returns its
    /// role on success.
    pub fn hire_candidate(
        &mut self,
        index: usize,
    ) -> Result<crate::career::Profession, crate::career::HireError> {
        let Some(member) = self.hire_pool.get(index).cloned() else {
            return Err(crate::career::HireError::Funds);
        };
        self.company
            .hire_listed(member.name.clone(), member.profession, &member.skills, &[])?;
        if let Some(joined) = self.company.roster.last_mut() {
            joined.skill_xp = member.skill_xp;
            joined.techniques = member.techniques;
        }
        self.log.append_words(
            crate::staff::LOG_JOINED,
            crate::staff::name_words(&name16(&member.name)),
        );
        self.hire_pool.remove(index);
        Ok(member.profession)
    }

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
        let mut board = arena_board::open(
            &self.company,
            &self.player,
            self.company.day,
            &self.daily_rows,
        );
        board.matches_played = self.matches_today as i32;
        if let Some(tournament) = &self.tournament {
            board.tournament_events = arena_board::daily_events(&tournament.rows);
            // The winners table stores a company id less one.
            board.tournament_companies = tournament
                .companies
                .iter()
                .map(|&company| i32::from(company) - 1)
                .collect();
            board.tournament_prize_item = Some(tournament.prize_id);
        }
        board.show();
        board
    }

    /// The company's thirteen records as the records window and the match
    /// read them.
    pub fn records(&self) -> crate::arena_match::CompanyRecords {
        self.company.records()
    }

    /// Company Status's lines from this career.
    pub fn status(&self) -> crate::manage::CompanyStatus {
        let employees: Vec<(Profession, u32)> = self
            .company
            .roster
            .iter()
            .map(|m| (m.profession, crate::manage::rank_of(&m.skill_xp) as u32))
            .collect();
        crate::manage::company_status(
            &self.company.name,
            crate::manage::rank_of(&self.player.skill_xp) as u32,
            &employees,
        )
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

pub(crate) fn name16(name: &str) -> [u8; 16] {
    let mut out = [0u8; 16];
    let bytes = name.as_bytes();
    let len = bytes.len().min(15);
    out[..len].copy_from_slice(&bytes[..len]);
    out
}

/// Draw one hire-list candidate. The profession, and a combatant's or
/// trainer's level, come from [`crate::hirelings::draw_candidate`]; the
/// candidate's record then runs the random-NPC pass from the draw's roll
/// (the original reseeds its shared sequence from the performance counter
/// before each candidate, then draws the role and the record from it). The
/// item outfit pass is not run: the hub has no item-fact table, so the
/// candidate carries no gear and the price adds no gear value.
fn draw_hire_member(
    counter: u64,
    catalog: &mut crate::arena_outfit::ProfessionCatalog,
    previous: &mut u32,
    random: &mut crate::arena::Mt19937,
) -> Member {
    // The role draws step the engine's LCG; a trainer's or combatant's level
    // reads the library generator, and the record's pass then continues the
    // LCG from where the role draws left it.
    let (candidate, roll) = crate::hirelings::draw_candidate(counter, || random.unit());
    let male: Vec<(&str, &str)> = crate::arena_names::MALE_NAMES
        .iter()
        .map(|name| (*name, ""))
        .collect();
    let female: Vec<(&str, &str)> = crate::arena_names::FEMALE_NAMES
        .iter()
        .map(|name| (*name, ""))
        .collect();
    let names = crate::arena_outfit::NameTable {
        male: &male,
        female: &female,
    };
    let request = crate::arena_outfit::NpcRequest {
        level: candidate.level,
        forced_flags: if candidate.older { 0x2_0000 } else { 0 },
        warmth: 0.0,
        previous_variant: *previous,
        names: &names,
    };
    let mut lcg = crate::arena_outfit::Lcg::new(roll);
    let profile = crate::arena_outfit::random_npc_lcg(&mut lcg, &request, catalog);
    *previous = profile.variant;
    let name = if profile.name.1.is_empty() {
        profile.name.0.to_string()
    } else {
        format!("{} {}", profile.name.0, profile.name.1)
    };
    let mut member = Member {
        name,
        profession: candidate.profession,
        xp: 0,
        skills: [0; 6],
        skill_xp: [0; 6],
        techniques: [[0; 6]; 6],
        loadouts: Default::default(),
        health: u16::MAX,
        in_entry: false,
    };
    for (index, row) in profile
        .professions
        .rows
        .iter()
        .take(member.skill_xp.len())
        .enumerate()
    {
        member.skill_xp[index] = row.experience;
        member.techniques[index].copy_from_slice(&row.levels[..6]);
    }
    if candidate.profession == Profession::Trainer {
        crate::hirelings::snap_trainer(&mut member.skill_xp, &mut member.techniques);
    }
    // The six levels the fee and the upkeep read.
    member.skills = member
        .skill_xp
        .map(|xp| u32::from(xp) / u32::from(crate::career::SKILL_STEP));
    member
}

/// The default career's library-generator seed.
const DEFAULT_RANDOM_SEED: u32 = 0x1234_5678;

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
    /// Each row's price word, parallel to [`Self::stock`]. Zero means the
    /// catalogue has no value for that item, so a click leaves the row.
    pub prices: Vec<u16>,
    /// The player's goods shown in the pack, and each one's price word.
    pub goods: Vec<(u32, String)>,
    pub good_prices: Vec<u16>,
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
    /// The closed board's saved tab and list selections, restored when it
    /// is opened again.
    pub board_state: Option<CloseState>,
    /// The board's form, built from its template's records as the company
    /// windows are ([`exa_ui::match_board::TEMPLATE_RECORDS`]).
    pub board_form: Option<exa_ui::record_form::RecordForm>,
    pub manage: Option<ManageRefresh>,
    pub shop: Option<ShopModel>,
    /// The table-built forms: Manage Company and the windows stacked on it.
    pub forms: crate::hub_forms::HubForms,
    /// The GUI unit the forms are built at (0 until the screen is known).
    pub unit: i32,
    /// A table-built form took this frame's press or release; the world
    /// click it makes is not the hub's.
    pub took_pointer: bool,
}

impl HubWindows {
    pub fn is_open(&self) -> bool {
        self.kind != HubWindowKind::None
    }

    pub fn is_kind(&self, kind: HubWindowKind) -> bool {
        self.kind == kind
    }

    /// The board's action from a form-local click: the board hit-tests in
    /// its client, inside the form's border.
    pub fn board_click(&mut self, point: (i32, i32)) -> BoardAction {
        let border = self.board_form.as_ref().map_or(0, |f| f.geometry.border);
        self.board.as_mut().map_or(BoardAction::None, |board| {
            board.click((point.0 - border, point.1 - border))
        })
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
            let bought: Vec<u32> = shop
                .window
                .buy_items
                .iter()
                .copied()
                .filter(|id| *id != 0)
                .collect();
            let sold: Vec<u32> = shop
                .window
                .sell_items
                .iter()
                .copied()
                .filter(|id| *id != 0)
                .collect();
            let done = shop.window.confirm(&mut career.company);
            if done {
                shop.window.money = career.company.copper;
                career.goods.extend(bought);
                for id in sold {
                    if let Some(at) = career.goods.iter().position(|held| *held == id) {
                        career.goods.remove(at);
                    }
                }
                career.goods_changed = true;
            }
            return done;
        }
        let pack = shop.window.panels()[1];
        let pack_rect = GuiRect::new(pack[0], pack[1], pack[2], pack[3]);
        if pack_rect.contains(point.0, point.1) {
            let row = (point.1 - pack_rect.top - 4) / ROW_HEIGHT;
            if row >= 0 {
                let row = row as usize;
                let price = shop.good_prices.get(row).copied().unwrap_or(0);
                let id = shop.goods.get(row).map(|good| good.0).unwrap_or(0);
                if price != 0 && shop.window.offer_item(id, price).is_some() {
                    shop.goods.remove(row);
                    shop.good_prices.remove(row);
                    info!("shop: item {id:#x} joins the sell basket at price {price}");
                    return true;
                }
            }
        }
        // A click on a priced stock row takes it into the buy basket. The
        // row's place matches the shelf the window bound.
        let panel = shop.window.panels()[4];
        let stock_rect = GuiRect::new(panel[0], panel[1], panel[2], panel[3]);
        if stock_rect.contains(point.0, point.1) {
            let row = (point.1 - stock_rect.top - 4) / ROW_HEIGHT;
            if row >= 0 {
                let row = row as usize;
                let price = shop.prices.get(row).copied().unwrap_or(0);
                if price != 0 && shop.window.take_stock_row(row, price).is_some() {
                    let (id, _) = shop.stock.remove(row);
                    shop.prices.remove(row);
                    info!("shop: item {id:#x} joins the buy basket at price {price}");
                    return true;
                }
            }
        }
        false
    }

    /// Opens the board on a career.
    pub fn open_board(&mut self, career: &ArenaCareer) {
        self.close();
        let mut board = career.board();
        if let Some(state) = &self.board_state {
            board.restore(state);
        }
        // The form, its template's records and the board's child views are
        // the constructor's unit-4 numbers rescaled to the live unit, as
        // the GUI unit's rescale leaves every form.
        let unit = if self.unit > 0 { self.unit } else { 4 };
        board.unit = unit;
        let form = exa_ui::record_form::RecordForm::open(
            &exa_ui::match_board::form_spec(board.compact),
            self.task.screen,
            unit,
        );
        let r = form.rect();
        let controls = board_controls(&form);
        self.board = Some(board);
        self.board_form = Some(form);
        self.kind = HubWindowKind::Board;
        self.push(
            HubWindowKind::Board,
            GuiRect::new(r.x, r.y, r.w, r.h),
            controls,
        );
    }

    /// Opens Manage Company on a career.
    pub fn open_manage(&mut self, career: &ArenaCareer) {
        self.close();
        let unit = if self.unit > 0 { self.unit } else { 4 };
        let mut form = exa_ui::record_form::RecordForm::open(
            &exa_ui::arena_forms::MANAGE_FORM,
            self.task.screen,
            unit,
        );
        let refresh = career.manage();
        crate::hub_forms::fill_manage(&mut form, &refresh);
        let r = form.rect();
        self.manage = Some(refresh);
        self.kind = HubWindowKind::Manage;
        self.forms.push(crate::hub_forms::FormWindow::Manage, form);
        self.push(
            HubWindowKind::Manage,
            GuiRect::new(r.x, r.y, r.w, r.h),
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
        let mut window = ShopWindow::new(career.company.copper, merchant);
        // The layout binds the stock list to its panel. The hub has no list
        // registry, so the list number is left unset [guess: the original's
        // number comes from the arena's own list table].
        window.bind_stock(stock, None);
        let shop = ShopModel {
            window,
            merchant,
            stock: stock.iter().map(|&id| (id, names.name_of(id))).collect(),
            // Filled from the catalogue after the window opens. A zero
            // leaves the row on the shelf.
            prices: vec![0; stock.len()],
            goods: career
                .goods
                .iter()
                .map(|&id| (id, names.name_of(id)))
                .collect(),
            good_prices: vec![0; career.goods.len()],
        };
        self.shop = Some(shop);
        self.kind = HubWindowKind::Shop;
        self.push(
            HubWindowKind::Shop,
            centred(SHOP_SIZE, self.task.screen),
            Vec::new(),
        );
    }

    /// Fill each stock row's price word. `price_of` reads the catalogue
    /// (the item, or the copy's source record). Zero leaves the row.
    pub fn price_shop_rows(&mut self, price_of: impl Fn(u32) -> u16) {
        let Some(shop) = self.shop.as_mut() else {
            return;
        };
        shop.prices = shop
            .stock
            .iter()
            .map(|(id, _)| price_of(*id))
            .collect();
        shop.good_prices = shop
            .goods
            .iter()
            .map(|(id, _)| price_of(*id))
            .collect();
    }

    /// Closes the open window, running its own close bookkeeping.
    pub fn close_window(&mut self) {
        match self.kind {
            HubWindowKind::Board => {
                if let Some(board) = self.board.as_mut() {
                    self.board_state = Some(board.close());
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
        self.board_form = None;
        self.manage = None;
        self.shop = None;
        self.forms.clear();
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

/// The board's buttons for the form host: the template's button records
/// (the four tabs, then End Day, Close, Enter and Forfeit), form-local.
/// The board's own hit test decides what a click does; these records let
/// the form host hit-test and select them.
fn board_controls(form: &exa_ui::record_form::RecordForm) -> Vec<Control> {
    let border = form.geometry.border;
    form.controls()
        .iter()
        .filter(|c| c.class == 1)
        .map(|c| {
            let mut c = c.clone();
            c.x += border;
            c.y += border;
            c
        })
        .collect()
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

fn at_pen(bounds: GuiRect, pen: (i32, i32)) -> (i32, i32) {
    (bounds.left + pen.0, bounds.top + pen.1)
}

fn at4(bounds: GuiRect, rect: (i32, i32, i32, i32)) -> GuiRect {
    GuiRect::new(bounds.left + rect.0, bounds.top + rect.1, rect.2, rect.3)
}

/// The font and unit a hub window draws with, as the screens do.
pub(crate) fn screen_font(
    textures: &GuiTextures,
    unit: i32,
) -> (exa_ui::gui_font::GuiFont, Option<(u16, u16)>) {
    let level = exa_ui::gui_font::level_for_unit(unit);
    crate::gui_screens::screen_font(exa_ui::gui_font::gui_level(level).main, textures)
}

/// What a hub window paints with: the palette its fade hands the skin, the
/// fade itself (for the line skin's control inks), the GUI unit and the
/// textures.
pub(crate) struct Look<'a> {
    pub(crate) palette: SkinPalette,
    pub(crate) fade: f32,
    pub(crate) unit: i32,
    pub(crate) textures: &'a GuiTextures,
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

pub(crate) fn paint_text(painted: &mut gui_paint::Painted, rect: GuiRect, text: &str, look: &Look) {
    let (font, size) = screen_font(look.textures, look.unit);
    for layer in gui_paint::paint_label(rect, 0, look.unit, &look.palette, &font, size, text).layers
    {
        painted.push(layer);
    }
}

/// Text at a pen (the glyphs' left edge and baseline), as a table view
/// draws its titles and cells; `None` is the skin's text colour.
fn paint_pen_text(
    painted: &mut gui_paint::Painted,
    pen: (i32, i32),
    text: &str,
    ink: Option<[u8; 4]>,
    look: &Look,
) {
    let (font, size) = screen_font(look.textures, look.unit);
    if let Some(texture) = size {
        let ink = ink.unwrap_or(look.palette.text[6]);
        painted.push(gui_paint::caption_layer(
            &font,
            texture,
            text.as_bytes(),
            pen,
            ink,
        ));
    }
}

pub(crate) fn paint_panel(painted: &mut gui_paint::Painted, rect: GuiRect, look: &Look) {
    let attributes = gui_paint::FORM_ATTRIBUTES;
    for layer in gui_paint::paint_panel(rect, look.unit, &look.palette, attributes, 0).layers {
        painted.push(layer);
    }
}

/// The board's template records as the board's state has them: each
/// record's state word is the board's control word, the primary button
/// and the two labels carry the board's text, and the two coin controls
/// the standard match's entry fee and prize.
fn sync_board_form(board: &Board, form: &mut exa_ui::record_form::RecordForm) {
    use exa_ui::match_board::control;
    for (i, &state) in board.controls.iter().enumerate() {
        form.set_state(i, state);
    }
    form.set_amount(control::STANDARD_FEE, board.standard_fee);
    form.set_amount(control::STANDARD_PRIZE, board.standard_prize);
    for item in board.paint() {
        if let BoardPaint::Label { widget, text, .. } = item {
            let record = match widget {
                Widget::EnterButton => control::ENTER,
                Widget::EnteredLabel => control::ENTERED,
                Widget::Status => control::STATUS,
                _ => continue,
            };
            form.set_text(record, &text);
        }
    }
}

/// Whether a paint item is one of the template's own records, which the
/// board's form draws.
fn template_drawn(item: &BoardPaint) -> bool {
    match item {
        BoardPaint::Tab { .. }
        | BoardPaint::Frame {
            widget: Widget::Form,
            ..
        } => true,
        BoardPaint::Label { widget, .. } => matches!(
            widget,
            Widget::EnterButton
                | Widget::EnteredLabel
                | Widget::AvailableLabel
                | Widget::CloseButton
                | Widget::EndDayButton
                | Widget::ForfeitButton
                | Widget::Status
                | Widget::EntryCaption
                | Widget::PrizeCaption
                | Widget::TournamentPrizeCaption
                | Widget::SeasonCaption
        ),
        _ => false,
    }
}

/// The board through the GUI path: its form built from the template's
/// records, drawn as the company windows are ([`crate::hub_forms::paint_form`]:
/// the frame, the tabs, the bottom row, the captions and the coin
/// controls), then the board's child views from its paint list in the
/// form's client. The pictures and item slots, which the item GUI fills,
/// draw as panels.
fn paint_board(
    board: &Board,
    form: &exa_ui::record_form::RecordForm,
    look: &Look,
) -> gui_paint::Painted {
    let r = form.rect();
    let bounds = GuiRect::new(r.x, r.y, r.w, r.h);
    let mut painted = gui_paint::Painted {
        client: client(bounds),
        ..Default::default()
    };
    let mut form = form.clone();
    sync_board_form(board, &mut form);
    crate::hub_forms::paint_form(&mut painted, &form, look);
    let (ox, oy) = form.origin();
    let origin = GuiRect::new(ox, oy, r.w, r.h);
    for item in board.paint() {
        if template_drawn(&item) {
            continue;
        }
        match item {
            BoardPaint::Frame { rect, .. } => {
                paint_panel(&mut painted, at(origin, rect), look);
            }
            BoardPaint::Tab { .. } => {}
            BoardPaint::Table { rect, .. } => {
                paint_panel(&mut painted, at(origin, rect), look);
            }
            BoardPaint::Column { pen, title, .. } => {
                paint_pen_text(&mut painted, at_pen(origin, pen), title, None, look);
            }
            BoardPaint::Cell {
                rect, pen, value, ..
            } => {
                let pen = at_pen(origin, pen);
                let text = match value {
                    CellValue::Text(text) => text,
                    // A coin column carries a draw callback instead of a
                    // text value: the following Coin item draws its dash or
                    // its coin counts.
                    CellValue::Coin(_) => String::new(),
                    CellValue::Health(word) => {
                        // The roster's health column is owner-drawn: the
                        // table hands the gauge draw the cell's pen (a unit
                        // in, `11u + 6u·row` down) and, the column being
                        // the last, the rest of the client's width.
                        let cell = at(origin, rect);
                        let marker = crate::manage::health_marker(
                            pen.0,
                            pen.1,
                            cell.right - pen.0,
                            look.unit,
                            word,
                        );
                        paint_gauge(&mut painted, &marker, look);
                        String::new()
                    }
                    CellValue::Empty => String::new(),
                };
                if !text.is_empty() {
                    paint_pen_text(&mut painted, pen, &text, None, look);
                }
            }
            BoardPaint::Coin { pen, draw, .. } => {
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
                    paint_pen_text(&mut painted, at_pen(origin, (x, pen.1)), &text, None, look);
                }
            }
            BoardPaint::Label { rect, text, .. } => {
                if let Some(rect) = rect {
                    paint_text(&mut painted, at(origin, rect), &text, look);
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

/// Manage Company: its table-built form, then its two tables (the roster's
/// headers, rows and health gauges, the report's day title and lines).
fn paint_manage(
    refresh: &ManageRefresh,
    form: &exa_ui::record_form::RecordForm,
    look: &Look,
) -> gui_paint::Painted {
    let r = form.rect();
    let bounds = GuiRect::new(r.x, r.y, r.w, r.h);
    let mut painted = gui_paint::Painted {
        client: client(bounds),
        ..Default::default()
    };
    crate::hub_forms::paint_form(&mut painted, form, look);
    let pane = |i: usize| {
        form.child(i)
            .map(|c| GuiRect::new(c.x, c.y, c.w, c.h))
            .unwrap_or(bounds)
    };
    let roster = pane(0);
    let report_table = pane(1);
    paint_panel(&mut painted, roster, look);
    paint_panel(&mut painted, report_table, look);
    // Both tables are table views at the form's unit: their client is
    // inside the base form's border, titles are a unit in and five units
    // down, rows six units tall under the six-unit header, and the
    // columns keep their share of the unit.
    let unit = form.unit();
    let inner = |r: GuiRect| {
        let b = exa_ui::match_board::TABLE_BORDER;
        GuiRect::new(r.left + b, r.top + b, r.width() - 2 * b, r.height() - 2 * b)
    };
    let roster = inner(roster);
    let report_table = inner(report_table);
    let width = |w: i32| {
        if unit == 4 {
            w
        } else {
            table_view::rescale_width(w, 4, unit)
        }
    };
    let pen_in = |client: GuiRect, pen: (i32, i32)| (client.left + pen.0, client.top + pen.1);
    let mut lefts = [0; 4];
    let mut x = 0;
    for (i, (title, w)) in crate::manage::ROSTER_COLUMNS.into_iter().enumerate() {
        lefts[i] = x;
        if !title.is_empty() {
            let pen = pen_in(roster, table_view::title_pen(unit, x));
            paint_pen_text(&mut painted, pen, title, None, look);
        }
        x += width(w);
    }
    // The health column is owner-drawn: the gauge draw gets the cell's pen
    // and, the column being the last, the rest of the client's width.
    for (row, text) in refresh.roster.iter().enumerate() {
        let row = row as i32;
        let name = pen_in(roster, table_view::cell_pen(unit, lefts[0], row));
        paint_pen_text(&mut painted, name, &text.name.text, text.name.ink, look);
        let rank = pen_in(roster, table_view::cell_pen(unit, lefts[1], row));
        paint_pen_text(&mut painted, rank, &text.rank.text, None, look);
        let health = pen_in(roster, table_view::cell_pen(unit, lefts[3], row));
        let marker = crate::manage::health_marker(
            health.0,
            health.1,
            roster.right - health.0,
            unit,
            text.health_word,
        );
        paint_gauge(&mut painted, &marker, look);
    }
    // The report: one column titled with the day, its lines in 6-unit rows
    // that fit the client.
    let title = pen_in(report_table, table_view::title_pen(unit, 0));
    paint_pen_text(&mut painted, title, &refresh.title, None, look);
    for (row, line) in refresh.report.iter().enumerate() {
        let pen = pen_in(report_table, table_view::cell_pen(unit, 0, row as i32));
        if pen.1 > report_table.bottom {
            break;
        }
        paint_pen_text(&mut painted, pen, &line.text, None, look);
    }
    painted
}

/// The roster's health gauge where [`crate::manage::health_marker`] puts
/// it: its frame, and the client fill up to the record's share. The GUI's
/// gauge shading itself is drawn with the panel and client-fill layers.
fn paint_gauge(
    painted: &mut gui_paint::Painted,
    marker: &crate::manage::HealthMarker,
    look: &Look,
) {
    let bar = GuiRect::new(marker.left, marker.top, marker.length, marker.thickness);
    paint_panel(painted, bar, look);
    let filled = (f64::from(marker.fill) * f64::from(marker.length)) as i32;
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
    let pack = shop.window.panels()[1];
    let pack_rect = at4(bounds, (pack[0], pack[1], pack[2], pack[3]));
    for (row, (_, name)) in shop.goods.iter().enumerate() {
        let y = pack_rect.top + 4 + ROW_HEIGHT * row as i32;
        if y + ROW_HEIGHT > pack_rect.bottom {
            break;
        }
        paint_text(
            &mut painted,
            GuiRect::new(pack_rect.left + 4, y, pack_rect.width() - 8, ROW_HEIGHT),
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
        let unit = exa_ui::scale::gui_scale(window.physical_height(), 0) as i32;
        if size != hub.task.screen || unit != hub.unit {
            hub.task.screen = size;
            hub.unit = unit;
            hub.forms.resize(size, unit);
            let hub = &mut *hub;
            if let (Some(board), Some(form)) = (hub.board.as_mut(), hub.board_form.as_mut()) {
                board.unit = unit;
                form.resize(size, unit);
            }
        }
    }
}

/// Adds the hub windows' draw to `app`, after the in-game screens' draw:
/// that pass clears the GUI's queue every frame, so a hub frame pushed
/// before it would never reach the canvas.
pub fn add_hub_draw(app: &mut App) {
    app.add_systems(
        Update,
        draw_hub_windows
            .after(crate::gui_screens::draw_screens)
            .after(crate::arena_hub_live::HubWindowSystems),
    );
}

/// Draws the open hub window into the GUI's queue. The in-game screens'
/// draw clears the queue each frame; without it (a host that has no game
/// screens) the hub clears it itself.
#[allow(clippy::too_many_arguments)]
pub fn draw_hub_windows(
    windows: Option<Res<HubWindows>>,
    textures: Option<Res<GuiTextures>>,
    screen: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut frames: ResMut<GuiFrames>,
    time: Option<Res<Time>>,
    game_screens: Option<Res<crate::gui_screens::GameScreens>>,
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
    if game_screens.is_none() {
        frames.clear();
    }
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
            .zip(windows.board_form.as_ref())
            .map(|(board, form)| paint_board(board, form, &look)),
        HubWindowKind::Manage => windows.manage.as_ref().and_then(|manage| {
            windows
                .forms
                .get(crate::hub_forms::FormWindow::Manage)
                .map(|form| paint_manage(manage, form, &look))
        }),
        HubWindowKind::Shop => windows
            .shop
            .as_ref()
            .map(|shop| paint_shop(shop, bounds, &look)),
        HubWindowKind::None => None,
    };
    let Some(mut painted) = painted else {
        debug!("hub windows: {:?} open but nothing to paint", windows.kind);
        return;
    };
    // The windows stacked over it, bottom to top, drawn shown.
    let shown = Look::new(1.0, unit, textures);
    for open in &windows.forms.forms {
        if open.window != crate::hub_forms::FormWindow::Manage {
            crate::hub_forms::paint_form(&mut painted, &open.form, &shown);
        }
    }
    debug!(
        "hub windows: {:?} at {:?} paints {} layers ({} stacked forms, fade {alpha})",
        windows.kind,
        bounds,
        painted.layers.len(),
        windows.forms.forms.len()
    );
    painted.submit(&mut frames);
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
    use exa_ui::match_board::BoardTab;

    /// The board form's border: a form-local point is a client point
    /// moved by it.
    const B: i32 = exa_ui::arena_forms::BASE_BORDER;

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
    fn the_reset_clears_the_stores_but_keeps_the_name_and_entries() {
        let mut career = ArenaCareer::default();
        career.company.name = "Ash".to_string();
        career
            .company
            .hire("a", crate::career::Profession::Combatant)
            .unwrap();
        career.company.stock = vec![7, 8];
        career.company.day = 9;
        career.company.won = 3;
        career.company.weeks = [true; 4];
        career.matches_today = 2;
        career.merchant_rank = 2;
        career.merchant_kinds = 5;
        career.merchant_stock = vec![4];
        career.entries[0].team_id = 4;
        career.log.append_value(crate::arena_match::LOG_WON, 1);
        career.reset();
        assert!(career.company.roster.is_empty() && career.company.stock.is_empty());
        assert_eq!(
            (
                career.company.day,
                career.company.won,
                career.company.copper
            ),
            (0, 0, 0)
        );
        assert_eq!(career.company.weeks, [false; 4]);
        assert_eq!(
            (
                career.matches_today,
                career.merchant_kinds,
                career.merchant_rank
            ),
            (0, 0, -1)
        );
        assert!(career.merchant_stock.is_empty() && career.log.records.is_empty());
        assert_eq!(career.company.name, "Ash");
        assert_eq!(career.entries[0].team_id, 4);
        career.start_new_career();
        assert_eq!(career.company.copper, 343);
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
    fn the_board_reopens_on_its_last_tab_and_lists() {
        let mut windows = HubWindows::default();
        let career = career();
        windows.open_board(&career);
        {
            let board = windows.board.as_mut().unwrap();
            assert!(board.switch_tab(BoardTab::Events).is_some());
            board.match_type = 2;
            board.rank = 3;
            board.tier = 1;
        }
        windows.close_window();
        assert!(windows.board_state.is_some());
        windows.open_board(&career);
        let board = windows.board.as_ref().unwrap();
        assert_eq!(board.tab, BoardTab::Events);
        assert_eq!((board.match_type, board.rank, board.tier), (2, 3, 1));
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
        let action = windows.board_click((layout.tabs[2].x + 1 + B, layout.tabs[2].y + 1 + B));
        assert_eq!(action, BoardAction::Changed);
        assert_eq!(windows.board.as_ref().unwrap().tab, BoardTab::Tournament);
    }

    #[test]
    fn the_form_host_holds_the_boards_bottom_row() {
        let form = exa_ui::record_form::RecordForm::open(
            &exa_ui::match_board::form_spec(false),
            (1280, 720),
            4,
        );
        let records = board_controls(&form);
        let layout = exa_ui::match_board::layout(false);
        // Four tabs and the four bottom-row buttons.
        assert_eq!(records.len(), 8);
        let bottom = exa_ui::match_board::layout(false).enter_button.y;
        for (index, rect) in [
            layout.end_day_button,
            layout.close_button,
            layout.enter_button,
            layout.forfeit_button,
        ]
        .iter()
        .enumerate()
        {
            let record = &records[4 + index];
            assert_eq!(
                (record.x, record.y, record.w, record.h),
                (rect.x + B, rect.y + B, rect.w, rect.h)
            );
            assert_eq!(record.y, bottom + B);
        }
        assert_eq!(records[4].text, b"End Day".to_vec());
        assert_eq!(records[5].text, b"Close".to_vec());
        assert_eq!(records[6].text, b"Enter".to_vec());
        assert_eq!(records[7].text, b"Forfeit".to_vec());
        assert_eq!(records[1].text, b"Specials".to_vec());
    }

    #[test]
    fn the_board_form_takes_the_boards_states_and_text() {
        use exa_ui::match_board::control;
        let mut windows = HubWindows::default();
        windows.open_board(&career());
        let board = windows.board.as_ref().unwrap();
        let mut form = windows.board_form.clone().unwrap();
        sync_board_form(board, &mut form);
        let c = form.controls();
        assert_eq!(c[control::TAB_MATCHES].state, board.controls[0]);
        assert_eq!(c[control::ENTER].text, b"Enter".to_vec());
        assert_eq!(c[control::ENTERED].text, board.entered_label().into_bytes());
        assert_eq!(form.amounts[control::STANDARD_FEE], board.standard_fee);
        // The template's records draw the tabs and the bottom row; the
        // board's child views stay with its paint list.
        assert!(
            board
                .paint()
                .iter()
                .filter(|i| !template_drawn(i))
                .all(|i| { !matches!(i, BoardPaint::Tab { .. }) })
        );
    }

    #[test]
    fn the_board_enter_in_a_duel_asks_for_the_match() {
        let mut windows = HubWindows::default();
        let career = career();
        windows.open_board(&career);
        let layout = exa_ui::match_board::layout(false);
        let action =
            windows.board_click((layout.enter_button.x + 1 + B, layout.enter_button.y + 1 + B));
        assert_eq!(action, BoardAction::EnterMatch);
    }

    #[test]
    fn the_carried_list_seeds_the_goods_once() {
        let mut career = career();
        career.seed_goods([4, 0, 8]);
        assert_eq!(career.goods, [4, 8]);
        career.seed_goods([1]);
        assert_eq!(career.goods, [4, 8]);
        let old = [exa_items::possession::Carried {
            id: 4,
            flags: 3,
            x: 10,
            y: 2,
        }];
        let carried = ArenaCareer::carried_after_trade(&old, &[4, 9]);
        assert_eq!(carried[0].flags, 3);
        assert_eq!(carried[1].id, 9);
        assert_eq!((carried[1].x, carried[1].y), (0, 0));
    }

    fn a_sale_leaves_the_goods_and_a_purchase_joins_them() {
        let mut career = career();
        career.company.stock = vec![7];
        career.goods = vec![9];
        let names = HubItemNames::default();
        let mut windows = HubWindows::default();
        windows.open_shop(false, &career, &names);
        windows.price_shop_rows(|id| match id {
            7 => 5,
            9 => 12,
            _ => 0,
        });
        let pack = windows.shop.as_ref().unwrap().window.panels()[1];
        assert!(windows.shop_click((pack[0] + 4, pack[1] + 6), &mut career));
        assert_eq!(windows.shop.as_ref().unwrap().window.sell_items, [9]);
        let point = (10, SHOP_SIZE.1 - 20);
        assert!(windows.shop_click(point, &mut career));
        assert_eq!(career.goods, Vec::<u32>::new());
        assert!(career.company.stock.contains(&9));
        windows.open_shop(false, &career, &names);
        windows.price_shop_rows(|id| if id == 7 { 5 } else { 0 });
        let panel = windows.shop.as_ref().unwrap().window.panels()[4];
        assert!(windows.shop_click((panel[0] + 4, panel[1] + 6), &mut career));
        assert!(windows.shop_click(point, &mut career));
        assert_eq!(career.goods, [7]);
        assert!(!career.company.stock.contains(&7));
    }

    fn the_catalogue_prices_a_stock_row() {
        let mut career = career();
        career.company.stock = vec![7];
        let names = HubItemNames::default();
        let mut windows = HubWindows::default();
        windows.open_shop(false, &career, &names);
        windows.price_shop_rows(|id| if id == 7 { 24 } else { 0 });
        let panel = windows.shop.as_ref().unwrap().window.panels()[4];
        let point = (panel[0] + 4, panel[1] + 6);
        assert!(windows.shop_click(point, &mut career));
        assert_eq!(windows.shop.as_ref().unwrap().window.buy, [24]);
        assert!(windows.shop.as_ref().unwrap().stock.is_empty());
    }

    fn a_priced_stock_row_joins_the_buy_basket() {
        let mut career = career();
        career.company.stock = vec![7, 9];
        let names = HubItemNames::default();
        let mut windows = HubWindows::default();
        windows.open_shop(false, &career, &names);
        windows.shop.as_mut().unwrap().prices = vec![5, 12];
        let panel = windows.shop.as_ref().unwrap().window.panels()[4];
        let point = (panel[0] + 4, panel[1] + 4 + 2);
        assert!(windows.shop_click(point, &mut career));
        let shop = windows.shop.as_ref().unwrap();
        assert_eq!(shop.stock.iter().map(|row| row.0).collect::<Vec<_>>(), [9]);
        assert_eq!(shop.prices, [12]);
        assert_eq!(shop.window.buy_items, [7]);
        assert_eq!(shop.window.buy, [5]);
        // A row with no price stays on the shelf.
        windows.shop.as_mut().unwrap().prices[0] = 0;
        assert!(!windows.shop_click(point, &mut career));
        assert_eq!(windows.shop.as_ref().unwrap().window.buy_items, [7]);
    }

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
    fn the_manage_roster_title_clears_its_table_header() {
        // "Roster" (centred down its 28-pixel record) ends above the
        // roster table's titles, which a table view draws five units into
        // its client, inside the two-pixel border.
        for unit in [4, 5, 6, 8] {
            let font = exa_ui::gui_font::gui_level(exa_ui::gui_font::level_for_unit(unit)).main;
            let form = exa_ui::record_form::RecordForm::open(
                &exa_ui::arena_forms::MANAGE_FORM,
                (1920, 1080),
                unit,
            );
            let label = &form.geometry.controls[0];
            assert_eq!(label.text, b"Roster".to_vec());
            let baseline = label.y + (label.h + font.line_height + 2) / 2;
            let roster = form.geometry.children[0];
            let (_, title_y) = table_view::title_pen(unit, 0);
            let top = roster.y + exa_ui::match_board::TABLE_BORDER + title_y - font.ascent;
            assert!(baseline <= top, "unit {unit}: {baseline} > {top}");
        }
        // The roster's columns keep their share of the unit: Health's
        // title at unit 6 is 0x84 + 0x50 + 0x10 = 228 design pixels in,
        // 342 live.
        assert_eq!(
            crate::manage::ROSTER_COLUMNS[..3]
                .iter()
                .map(|&(_, w)| table_view::rescale_width(w, 4, 6))
                .sum::<i32>(),
            342
        );
    }

    #[test]
    fn a_board_cell_rect_is_pinned() {
        let career = career();
        let board = career.board();
        let rects = board_cell_rects(&board);
        let first = rects.first().unwrap();
        assert_eq!(first.0, TableSlot::Match);
        // The first row is under the table view's 6-unit header, inside
        // its two-pixel border.
        assert_eq!(
            first.1,
            Rect {
                x: 2,
                y: 82,
                w: 192,
                h: 24
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
        assert_eq!(
            form.hit_test(layout.tabs[0].x + 1 + B, layout.tabs[0].y + 1 + B),
            1
        );
        // The Enter button is the seventh record: four tabs, End Day,
        // Close, then Enter.
        assert_eq!(
            form.hit_test(layout.enter_button.x + 1 + B, layout.enter_button.y + 1 + B),
            7
        );
        assert_eq!(
            form.hit_test(layout.close_button.x + 1 + B, layout.close_button.y + 1 + B),
            6
        );
        assert_eq!(
            form.hit_test(
                layout.end_day_button.x + 1 + B,
                layout.end_day_button.y + 1 + B
            ),
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
    fn the_hub_draw_survives_the_game_screens_clear() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<GuiFrames>()
            .init_resource::<crate::gui_screens::GameScreens>()
            .init_resource::<crate::item_panel_draw::PanelIcons>()
            .init_resource::<crate::gui_screens::DebugGuiForm>()
            .init_resource::<HubWindows>();
        app.add_systems(Update, crate::gui_screens::draw_screens);
        add_hub_draw(&mut app);
        let mut windows = HubWindows::default();
        windows.task.screen = (1280, 720);
        windows.unit = 4;
        windows.open_manage(&career());
        windows.open_records(&career());
        app.world_mut().insert_resource(windows);
        for _ in 0..5 {
            app.update();
            let frames = app.world().resource::<GuiFrames>();
            // The game screens' pass cleared the queue, then the hub drew
            // Manage Company and Records over it.
            // Without font textures the captions add no layers: the frames,
            // panels, buttons and table panes give 15.
            assert_eq!(frames.frames.len(), 1);
            assert_eq!(frames.frames[0].layers.len(), 15);
            assert!(frames.primitives() > 0);
        }
    }

    #[test]
    fn the_draw_path_runs_for_three_frames_with_a_board_open() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<GuiFrames>()
            .init_resource::<HubWindows>();
        add_hub_draw(&mut app);
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

    #[test]
    fn a_reset_clears_both_the_daily_and_the_lifetime_tallies() {
        let mut career = ArenaCareer::default();
        career.company.earned = 30;
        career.company.spent = 40;
        career.company.total_earned = 300;
        career.company.total_spent = 400;
        career.company.hires = 3;
        career.reset();
        assert_eq!(
            (
                career.company.earned,
                career.company.spent,
                career.company.total_earned,
                career.company.total_spent,
                career.company.hires
            ),
            (0, 0, 0, 0, 0)
        );
    }

    #[test]
    fn the_board_reads_the_days_kept_rows() {
        let mut career = ArenaCareer::default();
        let first = career.board().events.clone();
        assert_eq!(career.daily_rows.len(), 6);
        // The board's events are the stored rows.
        for (event, row) in career.board().events.iter().zip(career.daily_rows) {
            assert_eq!(event.kind, row.id);
            assert_eq!(event.tier, row.rank);
        }
        // A new roll on a new day gives a new set of rows.
        career.regenerate_daily_board_at(0x9E37_79B9);
        let second = career.board().events;
        assert_ne!(
            first.iter().map(|e| e.kind).collect::<Vec<_>>(),
            second.iter().map(|e| e.kind).collect::<Vec<_>>()
        );
    }

    #[test]
    fn the_board_carries_the_weeks_tournament() {
        let mut career = ArenaCareer::default();
        let tournament = career.tournament.clone().expect("the week's board");
        let board = career.board();
        for (event, row) in board.tournament_events.iter().zip(&tournament.rows) {
            assert_eq!(event.kind, row.id);
        }
        assert_eq!(board.tournament_prize_item, Some(tournament.prize_id));
        // The company field is stored less one, as the winners table reads
        // it.
        assert_eq!(
            board.tournament_companies.first().copied(),
            Some(i32::from(tournament.companies[0]) - 1)
        );
        // A new week draws a new prize and avoids the last eight.
        let old = career.tournament.as_ref().expect("the old board").clone();
        career.regenerate_tournament_at(0x2468_ACE0);
        let new = career.tournament.as_ref().expect("the new board");
        assert_ne!(new.prize_id, 0);
        assert_ne!(career.prize_history[0], 0);
        assert_ne!(old.companies, new.companies);
    }

    #[test]
    fn a_default_career_lists_ten_candidates_and_the_seed_decides_them() {
        let career = ArenaCareer::default();
        assert_eq!(career.hire_pool.len(), crate::hirelings::POOL_SIZE);
        assert_eq!(career.company.copper, crate::career::STARTING_COPPER);
        // The list is generated from the fixed placeholder seed; the same
        // seed gives the same ten.
        let mut again = ArenaCareer {
            hire_pool: Vec::new(),
            hire_previous: 0,
            hire_catalog: crate::arena_outfit::ProfessionCatalog::default(),
            random: crate::arena::Mt19937::new(DEFAULT_RANDOM_SEED),
            ..ArenaCareer::default()
        };
        again.fill_hire_pool_at(0x1234_5678);
        assert_eq!(again.hire_pool, career.hire_pool);
        assert!(career.hire_pool.iter().all(|m| !m.name.is_empty()));
        assert!(
            career
                .hire_pool
                .iter()
                .any(|m| m.profession != Profession::Combatant)
        );
        // Each candidate's levels are its skill words over 5000, which the
        // fee and the upkeep read.
        for member in &career.hire_pool {
            assert_eq!(
                member.skills,
                member.skill_xp.map(|xp| u32::from(xp) / 5000)
            );
        }
    }

    #[test]
    fn a_new_week_clears_the_entries_and_draws_the_rival_field() {
        let mut career = ArenaCareer::default();
        career.entries[2].count = 1;
        career.entries[2].team_id = 1;
        career.regenerate_tournament_at(0x2468_ACE0);
        assert_eq!(career.entries, Roster::default());
        let board = career.tournament.as_ref().expect("the week's board");
        assert_eq!(career.rivals, board.companies.map(i32::from));
        assert!(
            career
                .rivals
                .iter()
                .all(|&company| (1..=19).contains(&company))
        );
    }

    #[test]
    fn a_hire_takes_the_price_and_books_the_joining() {
        let mut career = ArenaCareer::default();
        career.company.roster.clear();
        career.company.copper = 1000;
        career.company.spent = 0;
        career.log.records.clear();
        let member = career.hire_pool[0].clone();
        let price = career.hire_price(0);
        let before = career.hire_pool.len();
        assert_eq!(career.hire_candidate(0), Ok(member.profession));
        assert_eq!(career.hire_pool.len(), before - 1);
        assert_eq!(career.company.copper, 1000 - price);
        assert_eq!(career.company.spent, price);
        assert_eq!(career.company.hires, 1);
        assert_eq!(career.company.roster.len(), 1);
        assert_eq!(career.company.roster[0].name, member.name);
        assert_eq!(career.company.roster[0].skill_xp, member.skill_xp);
        assert_eq!(career.log.records.len(), 1);
        assert_eq!(career.log.records[0].id, crate::staff::LOG_JOINED);
        // A short purse is refused without spending.
        let mut poor = ArenaCareer::default();
        poor.company.copper = 0;
        assert_eq!(poor.hire_candidate(0), Err(crate::career::HireError::Funds));
        assert_eq!(poor.company.spent, 0);
        assert_eq!(poor.hire_pool.len(), crate::hirelings::POOL_SIZE);
    }
}
