//! The hub's windows on a [`GuiTask`]: the match board, Manage Company and
//! the trade window, drawn through the GUI draw path.
//!
//! The screens' own rules are [`exa_ui::match_board`], [`crate::manage`] and
//! [`crate::shop_window`]. This module opens them from [`crate::arena_hub`]'s
//! click hook, keeps them on the window manager's task and paints their
//! frames, tables and stock through [`crate::gui_paint`] and
//! [`exa_ui::gui_draw`], as the in-game screens do. The board's paint list is
//! the board's own ([`Board::paint`]); the manage window's roster rows are
//! [`ManageRefresh::roster`]; the trade window's item panels are
//! [`crate::shop_panels`]'s and its bottom row [`ShopWindow`]'s.
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
//! (drawn as panels), the trade window's character preview, and the trade
//! window's own close control. Escape and a right-button release
//! close the top window.

use std::collections::HashMap;

use bevy::prelude::*;
use exa_ui::form::{ClientRect, FadeStep, FormFade, UNPLACED, place_unset};
use exa_ui::gui_controls::Control;
use exa_ui::gui_core::{FormHost, FormId, GuiRect, GuiTask, NO_CONTROL};
use exa_ui::gui_draw::{GuiFrames, GuiTextures};
use exa_ui::line_skin::shown_palette;
use exa_ui::match_board::{
    Board, BoardAction, BoardInput, BoardPaint, CellValue, CloseState, CoinDraw, Rect, TableLayout,
    TableSlot, Widget,
};
use exa_ui::skin::SkinPalette;
use exa_ui::table_view;

use crate::arena::Roster;
use crate::arena_board;
use crate::career::{Company, Member, Profession};
use crate::gui_paint;
use crate::item_container::ListChild;
use crate::manage::{ManageDay, ManageRecord, ManageRefresh, manage_refresh};
use crate::report::ReportFighter;
use crate::shop_panels::{ShopDrag, ShopDrop, ShopPanel, ShopPanels};
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
/// The centred placement word the hub's forms use. The constructors' own
/// placement words are not traced, so centring is a guess.
pub const CENTRE: u32 = 0x8000 | 0x800;
/// The button style the screens' constructors share.
pub const BUTTON_STYLE: u32 = 0x40_3320;

/// A confirmed trade at the hub's shop or merchant: the sell basket's
/// items and the buy basket's, in basket order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ShopTrade {
    pub sold: Vec<u32>,
    pub bought: Vec<u32>,
}

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
    /// The hire list: ten candidates, newest first, each a generated record
    /// with the role the candidate draw gave it.
    pub hire_pool: Vec<Member>,
    /// The profession grid's drift, shared across the list's generations.
    pub hire_catalog: crate::arena_outfit::ProfessionCatalog,
    /// The appearance variant the next candidate avoids.
    pub hire_previous: u32,
    /// The object database's facts the hire list's outfit pass and price
    /// read ([`HireFacts`]); none until the hub has read them.
    pub hire_facts: Option<std::sync::Arc<HireFacts>>,
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
    /// Staff places whose employee left the company (a dismissal, a death in
    /// a match, a departure before one), in order, for the staff list's
    /// share of the leaving: the kit's release into the arsenal, which needs
    /// the arena's stores ([`crate::hub_forms::apply_staff_changes`]).
    pub staff_left: Vec<usize>,
    /// Items the company won (the week's prize, a season's four rewards),
    /// in order, for the arsenal to stow free of any holder once each is
    /// made an item instance in the live object database
    /// ([`crate::arena_hub_live::hub_stow_won`]).
    pub won_items: Vec<crate::arena_rewards::WonItem>,
    /// The profession nibbles (4 a merchant, 5 a physician) of specialists
    /// who left, whose figure the hub takes away
    /// ([`crate::arena_hub_live::remove_left_figures`]).
    pub figures_left: Vec<u32>,
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
                outfit: Default::default(),
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
            hire_pool: Vec::new(),
            hire_catalog: crate::arena_outfit::ProfessionCatalog::default(),
            hire_previous: 0,
            hire_facts: None,
            daily_rows: crate::arena::generate_daily_board(0x1234_5678, || 0.5),
            restock_pending: false,
            tournament: None,
            prize_history: [0; 8],
            random: crate::arena::Mt19937::new(DEFAULT_RANDOM_SEED),
            roll: 0,
            warn_manager: true,
            rivals: [0; crate::career::RIVAL_FIELD],
            staff_left: Vec::new(),
            won_items: Vec::new(),
            figures_left: Vec::new(),
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
    /// Before a match the first specialist who is not well enough leaves
    /// the company: under 700 the log records his death (0x60000, his
    /// name's sixteen bytes) and the death count rises, else, under 65000,
    /// it records his leaving (0xA0000); then the removal runs
    /// ([`Self::remove_employee`]). A later specialist waits for the next
    /// match.
    pub fn leave_before_match(&mut self) -> Option<crate::career::Departure> {
        let index = self.company.unfit_specialist()?;
        let member = &self.company.roster[index];
        let words = crate::staff::name_words(&name16(&member.name));
        let died = member.health < crate::career::DIED_BELOW;
        if died {
            self.log.append_words(crate::arena_match::LOG_DIED, words);
            self.company.deaths = self.company.deaths.wrapping_add(1);
        } else {
            self.log.append_words(crate::report::LOG_LEFT, words);
        }
        let member = self.remove_employee(index)?;
        Some(if died {
            crate::career::Departure::Died(member.name)
        } else {
            crate::career::Departure::Left(member.name)
        })
    }

    /// The career's share of an employee's removal (the original's one
    /// removal routine, run for a dismissal, a death in a match and a
    /// departure before one): the member at `index` leaves the roster, the
    /// id `index + 1` is dropped from the tournament entries and the log's
    /// "saved" records with the later ids moved down, and the staff place
    /// is queued for the kit's release ([`Self::staff_left`]).
    pub fn remove_employee(&mut self, index: usize) -> Option<Member> {
        let member = self.drop_employee(index)?;
        let id = u32::try_from(index).unwrap_or(u32::MAX).wrapping_add(1);
        crate::staff::renumber_saved(&mut self.log, id);
        Some(member)
    }

    /// [`Self::remove_employee`] without the log's renumbering, for a death
    /// in a match: the match renumbered the log's "saved" records when the
    /// fall was resolved ([`crate::arena_match::book_death`]) and the roster
    /// place leaves when the match is settled.
    pub fn drop_employee(&mut self, index: usize) -> Option<Member> {
        if index >= self.company.roster.len() {
            return None;
        }
        let member = self.company.roster.remove(index);
        let id = u32::try_from(index).unwrap_or(u32::MAX).wrapping_add(1);
        crate::staff::unlink_entries(&mut self.entries, id);
        self.staff_left.push(index);
        Some(member)
    }

    /// Copy the player's loose items into the goods when the shop has not
    /// traded yet. A later open keeps the goods the trade left.
    pub fn seed_goods(&mut self, carried: impl IntoIterator<Item = u32>) {
        if self.goods.is_empty() {
            self.goods.extend(carried.into_iter().filter(|id| *id != 0));
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
                old.iter()
                    .find(|carried| carried.id == id)
                    .copied()
                    .unwrap_or(exa_items::possession::Carried {
                        id,
                        flags: 0,
                        x: 0,
                        y: 0,
                    })
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
                self.hire_facts.as_deref(),
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
        let facts = self.hire_facts.clone();
        let mut counter = crate::arena_match::counter_now();
        crate::hirelings::renew_pool(&mut pool, &mut self.roll, || {
            let member = draw_hire_member(
                counter,
                &mut catalog,
                &mut previous,
                random,
                facts.as_deref(),
            );
            counter = counter.wrapping_add(0x9E37_79B9_7F4A_7C15);
            member
        });
        self.hire_catalog = catalog;
        self.hire_previous = previous;
        self.hire_pool = pool;
    }

    /// What a listed candidate costs: the rank from its six skill levels and
    /// the hire fee with four fifths of its outfit's value (each worn and
    /// held item's value word priced as a shop line).
    pub fn hire_price(&self, index: usize) -> u32 {
        self.hire_pool.get(index).map_or(0, |member| {
            crate::career::Company::listed_price(&member.skills, &self.gear_words(member))
        })
    }

    /// The value words of what a member's record wears and holds.
    pub fn gear_words(&self, member: &Member) -> Vec<u16> {
        let Some(facts) = self.hire_facts.as_deref() else {
            return Vec::new();
        };
        member
            .outfit
            .items
            .iter()
            .chain(member.outfit.hands.iter().filter(|&&hand| hand != 0))
            .filter_map(|id| facts.values.get(id).copied())
            .collect()
    }

    /// The hub has read the object database: the hire list's outfit pass
    /// can run. A candidate drawn before (the play-mode hub's default list)
    /// is dressed now from a fresh roll. [Exarust's: the original's list is
    /// always drawn with the database loaded.]
    pub fn set_hire_facts(&mut self, facts: std::sync::Arc<HireFacts>, counter: u64) {
        self.hire_facts = Some(facts.clone());
        for (index, member) in self.hire_pool.iter_mut().enumerate() {
            if member.outfit != crate::hirelings::Loadout::default() {
                continue;
            }
            let roll = (counter as u32).swap_bytes().wrapping_add(index as u32);
            let mut lcg = crate::arena_outfit::Lcg::new(roll);
            // The level is the combatant's skill sum's [Exarust's: the list
            // does not keep the generator's level].
            let level = member.skills.iter().sum::<u32>() as i32 / 2;
            dress_candidate(member, level, &mut lcg, &facts, 0);
        }
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
        let gear = self.gear_words(&member);
        self.company.hire_listed(
            member.name.clone(),
            member.profession,
            &member.skills,
            &gear,
        )?;
        if let Some(joined) = self.company.roster.last_mut() {
            joined.skill_xp = member.skill_xp;
            joined.techniques = member.techniques;
            joined.loadouts = member.loadouts.clone();
            joined.outfit = member.outfit.clone();
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
        self.board_with(None, &|item| item)
    }

    /// The board with the arena's lineup order (the staff's order list,
    /// when it holds one entry per pool slot) and the item kind the weapon
    /// check compares. The tournament entries and winners are the career's
    /// six rows, the season's winners the company's week slots, and each
    /// fighter's event values its loadouts' first hand items.
    pub fn board_with(&self, order: Option<&[i32]>, kind_of: &dyn Fn(u32) -> u32) -> Board {
        let mut board = arena_board::open(
            &self.company,
            &self.player,
            self.company.day,
            &self.daily_rows,
        );
        let members = std::iter::once(&self.player).chain(&self.company.roster);
        for (fighter, member) in board.fighters.iter_mut().zip(members) {
            let (values, kinds) = arena_board::event_values(member, kind_of);
            fighter.event_values = values;
            fighter.event_weapons = kinds;
        }
        board.entries = arena_board::roster_entries(&self.entries, kind_of);
        board.week_winners = self.company.week_winners;
        if let Some(order) = order.filter(|order| order.len() == board.fighters.len()) {
            board.lineup = order.to_vec();
            board.rebuild_lineups();
        }
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
        // The report's hire lines count the hire list's professions; a log
        // line names its fighter by id: 0 is the manager, an employee its
        // place in the roster plus one.
        let listed: Vec<Profession> = self.hire_pool.iter().map(|m| m.profession).collect();
        manage_refresh(
            day,
            &manager,
            &employees,
            &self.log.records,
            &listed,
            |id| {
                let member = match id {
                    0 => Some(&self.player),
                    id => self.company.roster.get(usize::from(id) - 1),
                };
                member.map(|member| name16(&member.name))
            },
        )
    }
}

pub(crate) fn name16(name: &str) -> [u8; 16] {
    let mut out = [0u8; 16];
    let bytes = name.as_bytes();
    let len = bytes.len().min(15);
    out[..len].copy_from_slice(&bytes[..len]);
    out
}

/// What the hire list reads from the object database: the outfit tables
/// the candidates are dressed from, each body item's apparel rank byte (the
/// loadout trim's) and each item's value word (the price's).
#[derive(Debug, Default)]
pub struct HireFacts {
    pub tables: crate::arena_outfit::OutfitTables,
    pub apparel_rank: HashMap<u32, u8>,
    pub values: HashMap<u32, u16>,
}

impl PartialEq for HireFacts {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// The eight profile bytes a specialist candidate is dressed from: the
/// physician's flag byte 0x14, the merchant's and trainer's 4, then rank
/// limit 0 and 100, 10, 10. [The original passes five bytes on its stack;
/// the last three the profile pass may read are left as they were, taken as
/// 0 here.]
pub const fn specialist_profile(profession: Profession) -> [u8; 8] {
    let flags = match profession {
        Profession::Physician => 0x14,
        _ => 4,
    };
    [flags, 0, 100, 10, 10, 0, 0, 0]
}

/// The hire-list candidate's outfit pass, after its record's: a
/// combatant is dressed by level (its level as the rank, a twentieth of it
/// as the height bias) and its six loadouts are copies of that kit, those
/// of the ranks below its level trimmed to their rank; a specialist is
/// dressed from its profile ([`specialist_profile`]) and holds nothing.
/// The outfit continues the candidate's shared sequence.
pub fn dress_candidate(
    member: &mut Member,
    level: i32,
    lcg: &mut crate::arena_outfit::Lcg,
    facts: &HireFacts,
    record_flags: u32,
) {
    let character = crate::arena_outfit::OutfitCharacter {
        skill_word: 0,
        experience: u32::from(member.skill_xp[3]),
        warmth: 0.0,
        record_flags,
    };
    let combatant = member.profession == Profession::Combatant;
    let outfit = if combatant {
        crate::arena_outfit::generate_by_level_lcg(
            level,
            level as f32 * 0.05,
            0,
            lcg,
            &facts.tables,
            &character,
        )
    } else {
        crate::arena_outfit::generate_profile_lcg(
            &specialist_profile(member.profession),
            lcg,
            &facts.tables,
            &character,
        )
    };
    let hands: Vec<u32> = outfit
        .hands
        .iter()
        .flatten()
        .map(|held| held.variant)
        .collect();
    let mut kit = crate::hirelings::Loadout::default();
    for item in &outfit.items {
        if hands.contains(&item.id.0) || facts.tables.facts(item.id).is_none() {
            continue;
        }
        kit.items.push(item.id.0);
    }
    // A specialist's two hands are cleared after its outfit.
    if combatant {
        for (slot, held) in outfit.hands.iter().enumerate() {
            if let Some(held) = held {
                kit.hands[slot] = held.variant;
            }
        }
        let ranked = crate::hirelings::ranked_loadouts(&kit, level, |item| {
            facts.apparel_rank.get(&item).copied()
        });
        for (slot, loadout) in member.loadouts.iter_mut().zip(ranked) {
            *slot = exa_formats::characters::Loadout {
                word: 0,
                hands: [
                    exa_formats::characters::ItemRef::new(loadout.hands[0], 0),
                    exa_formats::characters::ItemRef::new(loadout.hands[1], 0),
                    exa_formats::characters::ItemRef::NONE,
                    exa_formats::characters::ItemRef::NONE,
                ],
                worn: loadout
                    .items
                    .iter()
                    .map(|&id| exa_formats::characters::ItemRef::new(id, 0))
                    .collect(),
            };
        }
    }
    member.outfit = kit;
}

/// Draw one hire-list candidate. The profession, and a combatant's or
/// trainer's level, come from [`crate::hirelings::draw_candidate`]; the
/// candidate's record then runs the random-NPC pass from the draw's roll
/// (the original reseeds its shared sequence from the performance counter
/// before each candidate, then draws the role and the record from it), and
/// with the object database read its outfit pass ([`dress_candidate`])
/// continues the same sequence.
fn draw_hire_member(
    counter: u64,
    catalog: &mut crate::arena_outfit::ProfessionCatalog,
    previous: &mut u32,
    random: &mut crate::arena::Mt19937,
    facts: Option<&HireFacts>,
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
        outfit: Default::default(),
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
    if let Some(facts) = facts {
        dress_candidate(
            &mut member,
            candidate.level,
            &mut lcg,
            facts,
            profile.look.flags,
        );
    }
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

/// The shop window and its four item panels.
#[derive(Clone, Debug, PartialEq)]
pub struct ShopModel {
    pub window: ShopWindow,
    pub merchant: bool,
    /// The pack, the two baskets and the stock, as item panels
    /// ([`crate::shop_panels`]).
    pub panels: ShopPanels,
    /// Each item's value word, which a basket's change callback sums. An
    /// item with none counts nothing.
    pub prices: HashMap<u32, u16>,
    /// The character preview's record: the opener's copy of the player's
    /// ([`crate::shop_preview`]); `None` until the preview's sync copies it.
    pub preview: Option<exa_formats::characters::CharacterRecord>,
    /// A drag off the preview in flight.
    pub preview_drag: Option<crate::shop_preview::PreviewDrag>,
}

impl ShopModel {
    /// A basket's change callback: the value word of each item it holds,
    /// summed again (the sell basket's quartered) and the bottom row
    /// redone. The other panels have none.
    pub fn basket_changed(&mut self, panel: ShopPanel) {
        let ids = self.panels.ids(panel);
        let words: Vec<u16> = ids
            .iter()
            .map(|id| self.prices.get(id).copied().unwrap_or(0))
            .collect();
        match panel {
            ShopPanel::Buy => {
                self.window.buy = words;
                self.window.buy_items = ids;
                self.window.buy_changed();
            }
            ShopPanel::Sell => {
                self.window.sell = words;
                self.window.sell_items = ids;
                self.window.sell_changed();
            }
            ShopPanel::Pack | ShopPanel::Stock => {}
        }
    }
}

/// The trade window's pack as the layout loads it: each of the company's
/// `goods` with the first unused entry of the player's `carried` list of
/// that id (its flags and stored place), or none.
pub fn pack_list(goods: &[u32], carried: &[exa_items::possession::Carried]) -> Vec<ListChild> {
    let mut used = vec![false; carried.len()];
    goods
        .iter()
        .map(|&id| {
            let found = carried
                .iter()
                .enumerate()
                .position(|(i, c)| !used[i] && c.id == id);
            match found {
                Some(i) => {
                    used[i] = true;
                    let c = &carried[i];
                    ListChild {
                        id,
                        flags: c.flags,
                        x: c.x,
                        y: c.y,
                    }
                }
                None => ListChild {
                    id,
                    flags: 0,
                    x: 0,
                    y: 0,
                },
            }
        })
        .collect()
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

/// The board's fighter tables between presses: the last press (table, row
/// and clock) for the double press, the drag a press began (its table, its
/// 0-based row, its kind and its grab offset in the row) and whether the
/// press was the tables' (its release is theirs too).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BoardTables {
    pub last_press: Option<(TableSlot, usize, i64)>,
    pub drag: Option<TableDrag>,
    pub took: bool,
    /// The table the pad has focused ([`HubWindows::board_pad_key`]).
    pub pad_table: Option<TableSlot>,
    /// The focused table view's activation-key flag (its MenuSel press
    /// state, which [`table_view::TableView::command`] keeps).
    pub pad_pressed: bool,
    /// The bottom button the pad has focused (its control index), in
    /// place of a table.
    pub pad_button: Option<usize>,
}

/// A row a table's press picked up ([`table_view::TableReply::Drag`]):
/// its table (Manage Company's roster names [`TableSlot::Roster`]), its
/// 0-based row, the drag kind, the grab in the row and the screen point
/// the press was at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TableDrag {
    pub slot: TableSlot,
    pub row: usize,
    pub kind: i32,
    pub grab: (i32, i32),
    pub from: (i32, i32),
}

impl TableDrag {
    /// Whether the pointer has gone past the window manager's drag
    /// distance from the press ([`exa_ui::gui_core::drag_limit`]), where
    /// the original's drag (and its label) begins.
    pub fn moved(&self, pointer: (i32, i32), screen_height: i32) -> bool {
        let limit = exa_ui::gui_core::drag_limit(screen_height);
        (pointer.0 - self.from.0).abs() > limit || (pointer.1 - self.from.1).abs() > limit
    }
}

/// The board's move callbacks take the moved fighter's row out of the
/// table it left ([`table_view::TableView::remove_row`] on that table's
/// view as it stood before the move): a selection past the new end moves
/// to the new last row, and the board keeps it.
fn remove_moved_row(
    board: &mut Board,
    slot: TableSlot,
    row: usize,
    mut view: table_view::TableView,
) {
    view.remove_row(row as i32);
    let selected = (view.selected > 0).then(|| (view.selected - 1) as usize);
    match slot {
        TableSlot::Roster => board.available_selection = selected,
        TableSlot::Lineup => board.team_selection = selected,
        _ => {}
    }
}

/// What the pointer did on the board's company (available) and lineup
/// tables.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoardTableAction {
    /// Not the tables'.
    None,
    /// The tables took the press or release (a row selected, a drag begun
    /// or a move refused); the world click it makes is not the hub's.
    Took,
    /// A company fighter joined the lineup (a double press or a drop).
    Joined,
    /// A lineup fighter was moved within the lineup (a drop on it).
    Reordered,
    /// The lineup fighter at this row went back to the company table.
    Left(usize),
}

/// What the pad's word did on the board.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoardPad {
    /// The tables' answer.
    Table(BoardTableAction),
    /// `MenuBak`: the form's back slot (the window closes, as Escape).
    Back,
    /// `MenuSel` on a focused bottom button: its key message (0xc0 with
    /// the activation word) toggles it and runs its callback, the one its
    /// click runs; the hub clicks it at this screen point.
    Click((i32, i32)),
}

/// The board's bottom buttons the pad can focus, by control index, with
/// the layout rectangle each sits on: End Day, Close, Enter and Forfeit
/// (their records carry the pad's focus style 0x400000; the tab buttons'
/// do not).
const BOARD_PAD_BUTTONS: [usize; 4] = [
    exa_ui::match_board::control::DAY_END,
    exa_ui::match_board::control::CLOSE,
    exa_ui::match_board::control::ENTER,
    exa_ui::match_board::control::SECONDARY,
];

/// A board button's rectangle in the board's client.
fn board_button_rect(
    layout: &exa_ui::match_board::BoardLayout,
    index: usize,
) -> exa_ui::match_board::Rect {
    use exa_ui::match_board::control;
    match index {
        control::DAY_END => layout.end_day_button,
        control::CLOSE => layout.close_button,
        control::ENTER => layout.enter_button,
        _ => layout.forfeit_button,
    }
}

/// Whether the pad reaches a board button: its record (class 1 with its
/// template style) and its live state pass the reach test
/// ([`exa_ui::gui_controls::pad_reaches`]: shown, enabled, the focus
/// style).
fn board_button_reachable(board: &exa_ui::match_board::Board, index: usize) -> bool {
    let state = if index == exa_ui::match_board::control::ENTER {
        board.enter_state()
    } else {
        board.controls[index]
    };
    let mut record = exa_ui::gui_controls::Control::new(1, 0, 0, 0, 0);
    record.style = exa_ui::match_board::TEMPLATE_RECORDS[index].style;
    record.state = state;
    exa_ui::gui_controls::pad_reaches(&record)
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
    /// The last right press was an item panel's card request (or the
    /// card's own): its release does not close the window.
    pub card_press: bool,
    /// Icon handles a closed window's item panels held, for the icon
    /// manager to release.
    pub released_icons: Vec<usize>,
    /// The board's two fighter tables' pointer state
    /// ([`HubWindows::board_table_input`]).
    pub board_tables: BoardTables,
}

/// The flags the trade window's stock panel loads its items with: the
/// shop's own items. Its buy basket takes only items with the bit, its pack
/// and sell basket only items without it, and an item card shows the value
/// of an item with the bit whole (else a quarter).
pub const SHOP_ITEM_FLAGS: u32 = 0x8000_0000;

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

    /// The pointer on the open board's company and lineup tables, at a
    /// screen point, through their table views ([`Board::table_view`]).
    /// The pointer's row is hovered ([`table_view::TableView::hover_at`]),
    /// then the press goes to the view ([`table_view::TableView::press`]):
    /// a press on a row selects it, and a second press on the selected row
    /// within the GUI's double-press time is the double press, which
    /// activates it: a company fighter joins the lineup through the rank
    /// check ([`Board::double_click_available`]), a lineup fighter goes back
    /// ([`Board::double_click_team`]). A press that does not activate
    /// begins the row's drag (the view's drag press: the fighter tables'
    /// kind and the grab). A release over a table offers it the drag: the
    /// table takes the drop when the kind is its own and its accept check
    /// passes ([`table_view::TableView::accepts_drop`]; the lineup's is the
    /// rank check, [`Board::can_drop`]), and its drop callback runs
    /// ([`table_view::TableView::owner_query`]): the lineup's takes a
    /// company row at the row under the pointer (no row appends,
    /// [`exa_ui::match_board::BoardInput::DropOnTeam`]), the company table's
    /// takes a lineup row back and refuses its own rows.
    pub fn board_table_input(
        &mut self,
        at: (i32, i32),
        pressed: bool,
        released: bool,
        now: i64,
    ) -> BoardTableAction {
        if self.kind != HubWindowKind::Board || !self.forms.is_empty() {
            self.board_tables = BoardTables::default();
            return BoardTableAction::None;
        }
        let (Some(bounds), Some(board)) = (self.bounds(), self.board.as_ref()) else {
            return BoardTableAction::None;
        };
        let border = self.board_form.as_ref().map_or(0, |f| f.geometry.border);
        let local = (at.0 - bounds.left - border, at.1 - bounds.top - border);
        let visible = board.visible_tables();
        // The fighter table under the point, its view with the pointer's
        // row hovered, and the point in its client.
        let over = |slot: TableSlot| -> Option<(table_view::TableView, (i32, i32))> {
            if !visible.contains(&slot) {
                return None;
            }
            let table = board.table_layout(slot);
            if !table.rect.contains(local.0, local.1) {
                return None;
            }
            let point = table.to_client(local);
            let mut view = board.table_view(slot);
            view.hover_at(point.1);
            Some((view, point))
        };
        let roster = over(TableSlot::Roster);
        let lineup = over(TableSlot::Lineup);
        let font = exa_ui::gui_font::gui_level(exa_ui::gui_font::level_for_unit(board.unit)).main;
        let state = &mut self.board_tables;
        if pressed {
            state.drag = None;
            state.took = false;
            let hit = [(TableSlot::Roster, roster), (TableSlot::Lineup, lineup)]
                .into_iter()
                .find_map(|(slot, view)| {
                    view.filter(|(view, _)| view.hover > 0).map(|v| (slot, v))
                });
            let Some((slot, (mut view, point))) = hit else {
                state.last_press = None;
                return BoardTableAction::None;
            };
            state.took = true;
            let row = (view.hover - 1) as usize;
            let double = state.last_press.is_some_and(|(t, r, when)| {
                t == slot && r == row && now - when <= i64::from(exa_ui::gui_core::DOUBLE_CLICK)
            });
            let mut flags = table_view::pointer::PRESS;
            if double {
                flags |= table_view::pointer::DOUBLE;
            }
            let mut events = Vec::new();
            view.press(point.0, flags, font, &mut events);
            let Some(board) = self.board.as_mut() else {
                return BoardTableAction::None;
            };
            for event in &events {
                match *event {
                    table_view::TableEvent::Selected(at) => {
                        let at = Some((at - 1) as usize);
                        if slot == TableSlot::Roster {
                            board.available_selection = at;
                        } else {
                            board.team_selection = at;
                        }
                    }
                    table_view::TableEvent::Activated(at) => {
                        state.last_press = None;
                        let at = (at - 1) as usize;
                        let input = if slot == TableSlot::Roster {
                            BoardInput::DoubleClickAvailable(at)
                        } else {
                            BoardInput::DoubleClickTeam(at)
                        };
                        let moved = board.handle(input) == BoardAction::Changed;
                        if moved {
                            remove_moved_row(board, slot, at, view);
                        }
                        return match (moved, slot) {
                            (true, TableSlot::Roster) => BoardTableAction::Joined,
                            (true, _) => BoardTableAction::Left(at),
                            _ => BoardTableAction::Took,
                        };
                    }
                    _ => {}
                }
            }
            state.last_press = Some((slot, row, now));
            if let table_view::TableReply::Drag { row, grab, kind } =
                view.press(point.0, table_view::pointer::DRAG, font, &mut events)
            {
                state.drag = Some(TableDrag {
                    slot,
                    row: row as usize,
                    kind,
                    grab,
                    from: at,
                });
            }
            return BoardTableAction::Took;
        }
        if !released {
            return BoardTableAction::None;
        }
        let took = std::mem::take(&mut state.took);
        let drag = state.drag.take();
        let Some(board) = self.board.as_mut() else {
            return BoardTableAction::None;
        };
        let action = drag.and_then(|drag| {
            let from = drag.row;
            if let Some((view, _)) = &lineup {
                // The lineup's accept check: the rank check on a company
                // row; a row of its own is moved within it.
                let refuses = drag.slot == TableSlot::Roster
                    && !board
                        .available
                        .get(from)
                        .is_some_and(|&pool| board.can_drop(pool));
                if !view.accepts_drop(drag.kind, false, refuses) {
                    return Some(BoardTableAction::Took);
                }
                let to = (view.hover > 0).then(|| (view.hover - 1) as usize);
                let source = board.table_view(drag.slot);
                let mut answer = BoardTableAction::Took;
                let mut drop = || {
                    if drag.slot == TableSlot::Roster
                        && board.handle(BoardInput::DropOnTeam { from, to }) == BoardAction::Changed
                    {
                        answer = BoardTableAction::Joined;
                        return 1;
                    }
                    if drag.slot == TableSlot::Lineup
                        && board.handle(BoardInput::ReorderTeam { from, to })
                            == BoardAction::Changed
                    {
                        answer = BoardTableAction::Reordered;
                        return 1;
                    }
                    0
                };
                if view.owner_query(Some(&mut drop)) != 0 {
                    if answer == BoardTableAction::Reordered {
                        // The rows stay in the lineup: only its selection
                        // is kept within it.
                        board.team_selection =
                            board.team_selection.filter(|&row| row < board.team.len());
                    } else {
                        remove_moved_row(board, drag.slot, from, source);
                    }
                }
                return Some(answer);
            }
            let (view, _) = roster.as_ref()?;
            if !view.accepts_drop(drag.kind, false, false) {
                return Some(BoardTableAction::Took);
            }
            let source = board.table_view(drag.slot);
            let mut answer = BoardTableAction::Took;
            let mut drop = || {
                // The company table's drop refuses its own rows.
                if drag.slot == TableSlot::Lineup
                    && board.handle(BoardInput::DropOnAvailable(from)) == BoardAction::Changed
                {
                    answer = BoardTableAction::Left(from);
                    return 1;
                }
                0
            };
            if view.owner_query(Some(&mut drop)) != 0 {
                remove_moved_row(board, drag.slot, from, source);
            }
            Some(answer)
        });
        match action {
            Some(action) => action,
            None if took => BoardTableAction::Took,
            None => BoardTableAction::None,
        }
    }

    /// The pad's menu word on the open board (the hub window on the focus
    /// stack), whose key slot is the base form's: every word is
    /// unanswered ([`crate::pad_menu::fallback`]). `MenuBak` is the form's
    /// back slot ([`BoardPad::Back`]); a navigation step is the directional
    /// pick among the visible tables, each placed by its slot 31
    /// ([`table_view::TableView::centre_and_default`]: its client's centre,
    /// a table with no selection selecting its first row), from the focused
    /// table's centre; the rest (`& 0xf322`) is the focused table's slot 30
    /// ([`table_view::TableView::command`]): a scroll step moves its
    /// selection, `MenuSel` arms then activates the selected row, and the
    /// company and lineup tables' activation words (the scroll stick right
    /// or left) move the selected fighter across as the double press does.
    pub fn board_pad_key(&mut self, key: crate::pad_menu::MenuKey) -> BoardPad {
        use crate::pad_menu::{Fallback, UNANSWERED, fallback};
        if self.kind != HubWindowKind::Board || !self.forms.is_empty() {
            return BoardPad::Table(BoardTableAction::None);
        }
        let back_closes = exa_ui::form::DEFAULT_ATTRIBUTES & 2 != 0;
        match fallback(UNANSWERED, key.key, false, back_closes) {
            Fallback::Answered => BoardPad::Table(BoardTableAction::None),
            Fallback::Back => BoardPad::Back,
            Fallback::Pick => {
                self.board_pad_pick(key.cursor);
                BoardPad::Table(BoardTableAction::Took)
            }
            Fallback::Control(word) => match self.board_tables.pad_button {
                Some(index) => self.board_button_key(index, word),
                None => BoardPad::Table(self.board_pad_command(word)),
            },
        }
    }

    /// A focused bottom button's key message: `MenuSel` (0x100, the
    /// activation word) clicks it while the pad still reaches it; any
    /// other word does nothing.
    fn board_button_key(&mut self, index: usize, word: u32) -> BoardPad {
        let (Some(board), Some(bounds)) = (self.board.as_ref(), self.bounds()) else {
            return BoardPad::Table(BoardTableAction::None);
        };
        if word & exa_ui::gui_controls::ACTIVATE == 0 || !board_button_reachable(board, index) {
            return BoardPad::Table(BoardTableAction::None);
        }
        let border = self.board_form.as_ref().map_or(0, |f| f.geometry.border);
        let rect = board_button_rect(&board.layout(), index);
        let at = (
            bounds.left + border + rect.x + rect.w / 2,
            bounds.top + border + rect.y + rect.h / 2,
        );
        info!("hub board: the pad's MenuSel clicks button {index} at {at:?}");
        BoardPad::Click(at)
    }

    /// The directional pick among the board's visible tables.
    fn board_pad_pick(&mut self, cursor: Vec2) {
        use crate::gamepad::{PickTarget, pick_control};
        let Some(board) = self.board.as_mut() else {
            return;
        };
        let current = self.board_tables.pad_table;
        let mut slots = Vec::new();
        let mut targets = Vec::new();
        let mut origin = IVec2::ZERO;
        let layout = board.layout();
        // The bottom buttons the pad reaches are candidates beside the
        // tables (ids after them).
        let buttons: Vec<usize> = BOARD_PAD_BUTTONS
            .into_iter()
            .filter(|&index| board_button_reachable(board, index))
            .collect();
        for &slot in board.visible_tables() {
            let client = board.table_layout(slot).client();
            let (_, centre) = board.table_view(slot).centre_and_default();
            let centre = IVec2::new(client.x + centre.0, client.y + centre.1);
            if current == Some(slot) {
                origin = centre;
            }
            targets.push(PickTarget {
                id: slots.len() as i32,
                centre,
                reachable: true,
                current: current == Some(slot),
            });
            slots.push(slot);
        }
        for (i, &index) in buttons.iter().enumerate() {
            let rect = board_button_rect(&layout, index);
            let centre = IVec2::new(rect.x + rect.w / 2, rect.y + rect.h / 2);
            let focused = self.board_tables.pad_button == Some(index);
            if focused {
                origin = centre;
            }
            targets.push(PickTarget {
                id: (slots.len() + i) as i32,
                centre,
                reachable: true,
                current: focused,
            });
        }
        let Some(id) = pick_control(cursor, origin, &targets, board.unit) else {
            return;
        };
        if let Some(&index) = (id as usize)
            .checked_sub(slots.len())
            .and_then(|i| buttons.get(i))
        {
            self.board_tables.pad_button = Some(index);
            self.board_tables.pad_table = None;
            self.board_tables.pad_pressed = false;
            info!("hub board: the pad picks the bottom button {index}");
            return;
        }
        self.board_tables.pad_button = None;
        let slot = slots[id as usize];
        let mut view = board.table_view(slot);
        let before = view.selected;
        let (selected, _) = view.centre_and_default();
        if selected != before && selected > 0 {
            board.select_table_row(slot, (selected - 1) as usize);
        }
        self.board_tables.pad_table = Some(slot);
        self.board_tables.pad_pressed = false;
        info!("hub board: the pad picks the {slot:?} table (row {selected})");
    }

    /// The focused table's command word.
    fn board_pad_command(&mut self, word: u32) -> BoardTableAction {
        let (Some(slot), Some(board)) = (self.board_tables.pad_table, self.board.as_mut()) else {
            return BoardTableAction::None;
        };
        if !board.visible_tables().contains(&slot) {
            self.board_tables.pad_table = None;
            return BoardTableAction::None;
        }
        let mut view = board.table_view(slot);
        view.pressed = self.board_tables.pad_pressed;
        let events = view.command(word);
        self.board_tables.pad_pressed = view.pressed;
        let mut action = BoardTableAction::Took;
        for event in events {
            match event {
                table_view::TableEvent::Selected(row) => {
                    board.select_table_row(slot, (row - 1) as usize);
                    info!("hub board: the pad selects {slot:?} row {row}");
                }
                table_view::TableEvent::Activated(row) => {
                    let at = (row - 1) as usize;
                    let input = match slot {
                        TableSlot::Roster => BoardInput::DoubleClickAvailable(at),
                        TableSlot::Lineup => BoardInput::DoubleClickTeam(at),
                        _ => continue,
                    };
                    if board.handle(input) == BoardAction::Changed {
                        remove_moved_row(board, slot, at, view.clone());
                        info!("hub board: the pad moves {slot:?} row {row} across");
                        action = if slot == TableSlot::Roster {
                            BoardTableAction::Joined
                        } else {
                            BoardTableAction::Left(at)
                        };
                    }
                }
                _ => {}
            }
        }
        action
    }

    /// A form-local point of the trade window in its client, where the
    /// panels' places are.
    fn shop_client(point: (i32, i32)) -> (i32, i32) {
        let border = crate::shop_window::BORDER;
        (point.0 - border, point.1 - border)
    }

    /// The item under a form-local point of the open trade window and the
    /// flags its panel holds it with: an entry of the stock or the buy
    /// basket (the shop's flag) or of the pack or the sell basket.
    pub fn shop_item_at(&self, point: (i32, i32)) -> Option<(u32, u32)> {
        let shop = self.shop.as_ref()?;
        shop.panels.item_at(Self::shop_client(point))
    }

    /// The form-local centre of entry `index` of a trade window panel.
    pub fn shop_entry_centre(&self, panel: ShopPanel, index: usize) -> Option<(i32, i32)> {
        let shop = self.shop.as_ref()?;
        let container = shop.panels.get(panel);
        let entry = container.entries.get(index)?;
        let [x, y, _, _] = panel.client();
        let border = crate::shop_window::BORDER;
        let half = container.icon_size / 2;
        Some((
            border + x + entry.cell.x + half,
            border + y + entry.cell.y + half,
        ))
    }

    /// The form-local middle of a trade window panel.
    pub fn shop_panel_middle(panel: ShopPanel) -> (i32, i32) {
        let [x, y, w, h] = panel.client();
        let border = crate::shop_window::BORDER;
        (border + x + w / 2, border + y + h / 2)
    }

    /// A left press at a form-local point of the trade window: over an
    /// item it starts the item's drag. `bulk` is the item's bulk byte.
    pub fn shop_press(
        &mut self,
        point: (i32, i32),
        bulk: impl FnOnce(u32) -> i32,
    ) -> Option<ShopDrag> {
        let shop = self.shop.as_mut()?;
        shop.panels.press(Self::shop_client(point), bulk)
    }

    /// Starts the drag of entry `index` of a trade window panel, as a press
    /// on it does (the scripted purchase's).
    pub fn shop_press_entry(&mut self, panel: ShopPanel, index: usize) -> Option<ShopDrag> {
        self.shop.as_mut()?.panels.press_entry(panel, index, |_| 0)
    }

    /// The left release at a form-local point of the trade window ends the
    /// drag; a move into or out of a basket runs its change callback.
    pub fn shop_release(
        &mut self,
        point: (i32, i32),
        half: (i32, i32),
        model_found: bool,
    ) -> Option<ShopDrop> {
        let shop = self.shop.as_mut()?;
        shop.panels.drag?;
        let dropped = shop
            .panels
            .release(Self::shop_client(point), half, model_found);
        if let ShopDrop::Moved { from, to, .. } = dropped {
            for panel in [from, to] {
                if panel.is_basket() {
                    shop.basket_changed(panel);
                }
            }
        }
        Some(dropped)
    }

    /// The shop's trade button from a form-local click: the button is inside
    /// the page's bottom row. A confirmed trade takes the sold items out of
    /// the pack and returns both baskets: the arena's trade hooks send the
    /// bought items to the arsenal, not to the pack
    /// ([`crate::arsenal::Arsenal::trade`]).
    pub fn shop_click(
        &mut self,
        point: (i32, i32),
        career: &mut ArenaCareer,
        icons: Option<&mut crate::item_panel_draw::Placing<'_>>,
    ) -> Option<ShopTrade> {
        let shop = self.shop.as_mut()?;
        // The bottom row's own record places are not traced; the guessed
        // rect is where the page's bottom-left "Trade" button draws.
        let trade = GuiRect::new(0, SHOP_SIZE.1 - 32, 120, 28);
        if !trade.contains(point.0, point.1) {
            return None;
        }
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
        if !done {
            return None;
        }
        shop.window.money = career.company.copper;
        // The arena's bought hook shelves the bought items in the arsenal,
        // so they do not join the pack.
        let icons = shop.panels.settle_bought_away(icons);
        self.released_icons.extend(icons);
        for id in &sold {
            if let Some(at) = career.goods.iter().position(|held| held == id) {
                career.goods.remove(at);
            }
        }
        career.goods_changed = true;
        Some(ShopTrade { sold, bought })
    }

    /// Opens the board on a career.
    pub fn open_board(&mut self, career: &ArenaCareer) {
        self.open_board_with(career, None, &|item| item);
    }

    /// Opens the board with the arena's lineup order and item kinds
    /// ([`ArenaCareer::board_with`]).
    pub fn open_board_with(
        &mut self,
        career: &ArenaCareer,
        order: Option<&[i32]>,
        kind_of: &dyn Fn(u32) -> u32,
    ) {
        self.close();
        self.board_tables = BoardTables::default();
        let mut board = career.board_with(order, kind_of);
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
        // New table views: the roster selects its first row.
        self.forms.manage_tables = crate::hub_forms::ManageTables::default();
        self.kind = HubWindowKind::Manage;
        self.forms.push(crate::hub_forms::FormWindow::Manage, form);
        self.push(
            HubWindowKind::Manage,
            GuiRect::new(r.x, r.y, r.w, r.h),
            Vec::new(),
        );
    }

    /// Opens the trade window: the shop's shelf, or the merchant's own. The
    /// layout loads the pack from `pack` (the player's list,
    /// [`pack_list`]) and the stock from the shelf with the shop's flag,
    /// laid out by the grid pass seeded with the day counter plus one, as
    /// the hub's opener passes it. `slot` gives an item's icon slot and
    /// half extents.
    pub fn open_shop(
        &mut self,
        merchant: bool,
        career: &ArenaCareer,
        pack: &[ListChild],
        mut slot: impl FnMut(u32, u32) -> (usize, (i32, i32)),
    ) {
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
        let unit = if self.unit > 0 { self.unit } else { 4 };
        let mut panels = ShopPanels::new(unit);
        panels.load_pack(pack, &mut slot);
        panels.load_stock(stock, career.company.day.wrapping_add(1), &mut slot);
        let shop = ShopModel {
            window,
            merchant,
            panels,
            // Filled from the catalogue after the window opens.
            prices: HashMap::new(),
            preview: None,
            preview_drag: None,
        };
        self.shop = Some(shop);
        self.kind = HubWindowKind::Shop;
        self.push(
            HubWindowKind::Shop,
            centred(SHOP_SIZE, self.task.screen),
            Vec::new(),
        );
        // The merchant's trade adds its options form.
        if merchant {
            self.open_merchant_options(career);
        }
    }

    /// Fill each item's value word. `price_of` reads the catalogue (the
    /// item, or the copy's source record).
    pub fn price_shop_items(&mut self, price_of: impl Fn(u32) -> u16) {
        let Some(shop) = self.shop.as_mut() else {
            return;
        };
        for panel in crate::shop_panels::SHOP_PANELS {
            for id in shop.panels.ids(panel) {
                shop.prices.insert(id, price_of(id));
            }
        }
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
        // The base form's close: the window and its child forms close and
        // the window manager forgets them.
        if let Some(form) = self.form.take() {
            self.task.close_form(form);
        }
        self.task.delete_doomed();
        self.task.root = None;
        self.task.active = None;
        self.task.active_control = NO_CONTROL;
        self.kind = HubWindowKind::None;
        self.board = None;
        self.board_form = None;
        self.manage = None;
        if let Some(shop) = self.shop.take() {
            self.released_icons.extend(shop.panels.icons());
        }
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
    /// The interface level's small atlas of built-in glyphs (a combo box's
    /// arrow), when the GUI has one.
    pub(crate) glyphs: Option<&'a exa_ui::GuiLevelState>,
    /// The form keeps the draw skin's own instance (the colour picker),
    /// not the line skin: the skin's control slots are the draw skin's.
    pub(crate) base_skin: bool,
}

impl<'a> Look<'a> {
    pub(crate) fn new(
        fade: f32,
        unit: i32,
        textures: &'a GuiTextures,
        glyphs: Option<&'a exa_ui::GuiLevelState>,
    ) -> Self {
        Self {
            palette: shown_palette(fade),
            fade,
            unit,
            textures,
            glyphs,
            base_skin: false,
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

/// The trade window's frame through the game skin's form draw: the layers
/// under its children are pushed, the title and frame layers returned for
/// after them, with the committed words its children are held by.
fn paint_window(
    painted: &mut gui_paint::Painted,
    bounds: GuiRect,
    title: &str,
    look: &Look,
) -> (gui_paint::LineFormLayers, gui_paint::LineFormWords) {
    let (font, size) = screen_font(look.textures, look.unit);
    let (mut form, words) = gui_paint::paint_top_form(
        bounds,
        look.unit,
        &look.palette,
        look.fade,
        gui_paint::TRADE_WORDS,
        Some((title, &font, size)),
    );
    painted.layers.extend(form.under());
    (form, words)
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
    pointer: Option<(i32, i32)>,
    drag: Option<TableDrag>,
) -> gui_paint::Painted {
    let r = form.rect();
    let bounds = GuiRect::new(r.x, r.y, r.w, r.h);
    let mut painted = gui_paint::Painted {
        client: client(bounds),
        ..Default::default()
    };
    let mut form = form.clone();
    sync_board_form(board, &mut form);
    let (frame, words) = crate::hub_forms::paint_form(&mut painted, &form, look);
    let (ox, oy) = form.origin();
    let origin = GuiRect::new(ox, oy, r.w, r.h);
    let inset = ox - r.x;
    let form_client = GuiRect::new(ox, oy, r.w - 2 * inset, r.h - 2 * inset);
    // The child views' frames, drawn after their contents as the form draw
    // draws a child's frame after its client.
    let mut child_frames = Vec::new();
    let mut held =
        |rect: GuiRect, child: gui_paint::FormWords, painted: &mut gui_paint::Painted| {
            let mut layers = gui_paint::paint_held_form(
                rect,
                form_client,
                words.attributes,
                look.unit,
                &look.palette,
                look.fade,
                child,
            );
            painted.layers.extend(layers.under());
            child_frames.extend(layers.over());
        };
    for item in board.paint() {
        if template_drawn(&item) {
            continue;
        }
        match item {
            // The picture pane and the reward panel are base forms the
            // constructor gives style 0x82 and 0x80.
            BoardPaint::Frame {
                widget: Widget::Picture,
                rect,
            } => held(at(origin, rect), gui_paint::pane_words(0x82), &mut painted),
            BoardPaint::Frame {
                widget: Widget::RewardPanel,
                rect,
            } => held(at(origin, rect), gui_paint::pane_words(0x80), &mut painted),
            // The season picture is a picture record: its class draws only
            // its image, no frame.
            BoardPaint::Frame {
                widget: Widget::SeasonPicture,
                ..
            } => {}
            BoardPaint::Frame { rect, .. } => {
                paint_panel(&mut painted, at(origin, rect), look);
            }
            BoardPaint::Tab { .. } => {}
            BoardPaint::Table { slot, rect } => {
                held(at(origin, rect), gui_paint::TABLE_WORDS, &mut painted);
                // The table view's draw puts the selected row's band under
                // its cells.
                let b = exa_ui::match_board::TABLE_BORDER;
                let r = at(origin, rect);
                let client =
                    GuiRect::new(r.left + b, r.top + b, r.width() - 2 * b, r.height() - 2 * b);
                let mut view = board.table_view(slot);
                paint_table_selection(&mut painted, &mut view, client, pointer, look);
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
            // The weapon pane's Choose: a button record of the pane, drawn
            // as the line skin draws a button (its pressed and hover states
            // are not tracked for a pane's record).
            BoardPaint::Button { rect, text, .. } => {
                let (font, size) = screen_font(look.textures, look.unit);
                for layer in gui_paint::paint_button(
                    at(origin, rect),
                    0,
                    look.unit,
                    &look.palette,
                    &font,
                    size,
                    text,
                    false,
                    false,
                )
                .layers
                {
                    painted.push(layer);
                }
            }
        }
    }
    painted.layers.extend(child_frames);
    painted.layers.extend(frame);
    if let (Some(drag), Some(pointer)) = (drag, pointer) {
        paint_drag_label(
            &mut painted,
            &board.table_view(drag.slot),
            drag.row,
            drag.grab,
            pointer,
            look,
        );
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

/// A dragged row's label at the pointer less its grab, over everything
/// ([`table_view::TableView::drag_label`] and
/// [`table_view::TableView::paint_drag_label`]): the box, then the row's
/// first cell in the text group's ninth shade.
fn paint_drag_label(
    painted: &mut gui_paint::Painted,
    view: &table_view::TableView,
    row: usize,
    grab: (i32, i32),
    pointer: (i32, i32),
    look: &Look,
) {
    let (font, size) = screen_font(look.textures, look.unit);
    let (label, pen) = view.drag_label(row as i32, pointer, grab, &font);
    let mut batch = exa_ui::skin::SkinBatch::new();
    view.paint_drag_label(&mut batch, &look.palette, label);
    painted.push(exa_ui::gui_draw::GuiLayer::new(
        batch,
        exa_ui::gui_draw::LayerTexture::Named(exa_ui::gui_draw::SKIN_TEXTURE),
    ));
    let text = view
        .rows
        .get(row)
        .and_then(|row| row.cells.first())
        .cloned()
        .unwrap_or_default();
    if let Some(texture) = size {
        painted.push(gui_paint::caption_layer(
            &font,
            texture,
            &text,
            pen,
            look.palette.text[table_view::DRAG_LABEL_INK],
        ));
    }
}

/// A table view's selected-row band ([`table_view::TableView::paint_selection`])
/// in the table's client at `client` (screen pixels), the view hovering the
/// pointer's row when the pointer is in the client.
fn paint_table_selection(
    painted: &mut gui_paint::Painted,
    view: &mut table_view::TableView,
    client: GuiRect,
    pointer: Option<(i32, i32)>,
    look: &Look,
) {
    view.hover = 0;
    if let Some((x, y)) = pointer
        && client.contains(x, y)
    {
        view.hover_at(y - client.top);
    }
    // The line skin's header line first (the hub's tables keep the line
    // skin, whose opening sets bit 1 of its mark word): the span bar in the
    // idle control ink along the header's foot.
    let mark = exa_ui::skin::open_skin(look.unit, 0, true).map_or(0, |skin| skin.mark);
    let [x, y, w] = view.header_rule();
    let bar = exa_ui::skin::draw_span_bar(
        look.unit * 2,
        (client.left + x) as f32,
        (client.top + y) as f32,
        w as f32,
        look.unit,
        exa_ui::skin::SPAN_VIEW_UNSET,
    );
    let mut rule = exa_ui::skin::line_skin_batch();
    rule.set_ink(exa_ui::line_skin::line_control_inks(look.fade)[1]);
    for (quad, sample) in bar.quads.iter().zip(bar.binds) {
        let uv = sample.map(f32::from_bits);
        for (k, (px, py)) in quad.iter().enumerate() {
            rule.set_uv(uv[k * 2], uv[k * 2 + 1]);
            rule.vertex(*px, *py);
        }
    }
    rule.end_quads();
    if mark & 1 != 0 {
        painted.push(exa_ui::gui_draw::GuiLayer::new(
            rule,
            exa_ui::gui_draw::LayerTexture::Named(exa_ui::gui_draw::SKIN_TEXTURE),
        ));
    }
    if view.selection_band().is_none() {
        return;
    }
    let mut batch = exa_ui::skin::SkinBatch::new();
    view.paint_selection(&mut batch, &look.palette);
    gui_paint::offset_batch(&mut batch, client.left, client.top);
    painted.push(exa_ui::gui_draw::GuiLayer::new(
        batch,
        exa_ui::gui_draw::LayerTexture::Named(exa_ui::gui_draw::SKIN_TEXTURE),
    ));
}

/// Manage Company: its table-built form, then its two tables (the roster's
/// headers, selected-row band, rows and health gauges, the report's day
/// title, band and lines).
fn paint_manage(
    refresh: &ManageRefresh,
    form: &exa_ui::record_form::RecordForm,
    look: &Look,
    selected: [i32; 2],
    pointer: Option<(i32, i32)>,
    drag: Option<TableDrag>,
) -> gui_paint::Painted {
    let r = form.rect();
    let bounds = GuiRect::new(r.x, r.y, r.w, r.h);
    let mut painted = gui_paint::Painted {
        client: client(bounds),
        ..Default::default()
    };
    let (frame, words) = crate::hub_forms::paint_form(&mut painted, form, look);
    let pane = |i: usize| {
        form.child(i)
            .map(|c| GuiRect::new(c.x, c.y, c.w, c.h))
            .unwrap_or(bounds)
    };
    let roster = pane(0);
    let report_table = pane(1);
    // Both tables are table views held by the form: no backdrop, the panel
    // frame on their edges, drawn after their rows.
    let (ox, oy) = form.origin();
    let inset = ox - r.x;
    let form_client = GuiRect::new(ox, oy, r.w - 2 * inset, r.h - 2 * inset);
    let mut child_frames = Vec::new();
    for table in [roster, report_table] {
        let mut layers = gui_paint::paint_held_form(
            table,
            form_client,
            words.attributes,
            look.unit,
            &look.palette,
            look.fade,
            gui_paint::TABLE_WORDS,
        );
        painted.layers.extend(layers.under());
        child_frames.extend(layers.over());
    }
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
    for (table, rows) in [
        (crate::hub_forms::ROSTER_TABLE, refresh.roster.len()),
        (crate::hub_forms::REPORT_TABLE, refresh.report.len()),
    ] {
        let client = if table == crate::hub_forms::ROSTER_TABLE {
            roster
        } else {
            report_table
        };
        let corner = (client.left, client.top);
        if let Some((mut view, _)) = crate::hub_forms::manage_table_view(form, table, rows, corner)
        {
            view.selected = selected[table];
            paint_table_selection(&mut painted, &mut view, client, pointer, look);
        }
    }
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
    painted.layers.extend(child_frames);
    painted.layers.extend(frame);
    if let (Some(drag), Some(pointer)) = (drag, pointer)
        && let Some((mut view, _)) = crate::hub_forms::manage_table_view(
            form,
            crate::hub_forms::ROSTER_TABLE,
            refresh.roster.len(),
            (roster.left, roster.top),
        )
    {
        for (at, line) in refresh.roster.iter().enumerate() {
            if let Some(cell) = view.rows.get_mut(at).and_then(|row| row.cells.first_mut()) {
                *cell = line.name.text.as_bytes().to_vec();
            }
        }
        paint_drag_label(&mut painted, &view, drag.row, drag.grab, pointer, look);
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

/// The trade window's frame, its item panels with their icons (and the
/// dragged icon at the pointer) and its bottom row.
fn paint_shop(
    shop: &ShopModel,
    bounds: GuiRect,
    look: &Look,
    icons: Option<&mut crate::item_panel_draw::PanelIcons>,
    pointer: Option<(i32, i32)>,
) -> gui_paint::Painted {
    let mut painted = gui_paint::Painted {
        client: client(bounds),
        ..Default::default()
    };
    let (mut window, window_words) =
        paint_window(&mut painted, bounds, crate::shop_window::TRADE_TITLE, look);
    // The children's places are in the page's client, inside its border.
    let border = crate::shop_window::BORDER;
    let inner = GuiRect::new(
        bounds.left + border,
        bounds.top + border,
        bounds.width() - 2 * border,
        bounds.height() - 2 * border,
    );
    // The children are held by the window: no backdrop, the panel frame
    // on their edges.
    let held = |rect: GuiRect, words: gui_paint::FormWords| {
        gui_paint::paint_held_form(
            rect,
            inner,
            window_words.attributes,
            look.unit,
            &look.palette,
            look.fade,
            words,
        )
    };
    let character = crate::shop_window::CHARACTER_VIEW;
    let character = at4(
        inner,
        (character[0], character[1], character[2], character[3]),
    );
    let mut view = held(character, gui_paint::DOLL_WORDS);
    painted.layers.extend(view.under());
    // The preview's 3D view shows through the canvas there.
    painted.push(crate::gui_screens::clear_layer(character));
    painted.layers.extend(view.over());
    let mut panel_frames = Vec::new();
    for panel in crate::shop_panels::SHOP_PANELS {
        let [x, y, w, h] = panel.rect();
        let mut form = held(at4(inner, (x, y, w, h)), gui_paint::ITEM_PANEL_WORDS);
        painted.layers.extend(form.under());
        panel_frames.extend(form.over());
    }
    let mut frames_drawn = false;
    if let Some(icons) = icons {
        for panel in crate::shop_panels::SHOP_PANELS {
            let [x, y, w, h] = panel.client();
            let r = [inner.left + x, inner.top + y, w, h];
            let container = shop.panels.get(panel);
            trace!(
                "hub shop paint: {panel:?} at {r:?}: {:?}",
                container
                    .entries
                    .iter()
                    .map(|e| (e.id, e.icon, e.cell, e.drag, icons.rendered(e.icon)))
                    .collect::<Vec<_>>()
            );
            if let Some(mut layer) = icons.layer(container) {
                gui_paint::offset_batch(&mut layer.batch, r[0], r[1]);
                layer.clip = Some(r);
                painted.push(layer);
            }
        }
        // The panels' and the window's frames over the entries, under a
        // dragged icon.
        painted.layers.append(&mut panel_frames);
        painted.layers.extend(window.over());
        frames_drawn = true;
        if let (Some(drag), Some(pointer)) = (shop.preview_drag, pointer) {
            // A drag off the preview: its item's icon under the pointer.
            let shown = crate::item_container::Container {
                entries: vec![crate::item_container::Entry {
                    cell: crate::item_container::Cell::new(
                        pointer.0 - drag.grab.x,
                        pointer.1 - drag.grab.y,
                    ),
                    id: drag.item.id,
                    icon: drag.icon,
                    flags: drag.item.flags,
                    drag: 0,
                }],
                icon_size: shop.panels.pack.icon_size,
                ..crate::item_container::Container::default()
            };
            if let Some(layer) = icons.layer(&shown) {
                painted.push(layer);
            }
        }
        if let (Some(drag), Some(pointer)) = (shop.panels.drag, pointer) {
            paint_dragged(
                &mut painted,
                icons,
                shop.panels.get(drag.source),
                drag.icon,
                drag.grab,
                pointer,
            );
        }
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
    if !frames_drawn {
        painted.layers.append(&mut panel_frames);
        painted.layers.extend(window.over());
    }
    painted
}

/// The dragged item's icon at the pointer less its grab, as the item
/// panel's drag draws it: the entry `icon` of `source` (marked dragged, so
/// its panel skips it) drawn shown at the pointer.
pub(crate) fn paint_dragged(
    painted: &mut gui_paint::Painted,
    icons: &mut crate::item_panel_draw::PanelIcons,
    source: &crate::item_container::Container,
    icon: usize,
    grab: crate::item_container::Cell,
    pointer: (i32, i32),
) {
    let Some(entry) = source
        .entries
        .iter()
        .find(|e| e.icon == icon && e.drag & 1 != 0)
    else {
        return;
    };
    let shown = crate::item_container::Container {
        entries: vec![crate::item_container::Entry {
            cell: crate::item_container::Cell::new(pointer.0 - grab.x, pointer.1 - grab.y),
            drag: 0,
            ..*entry
        }],
        green_mask: source.green_mask,
        yellow_mask: source.yellow_mask,
        icon_size: source.icon_size,
        ..crate::item_container::Container::default()
    };
    if let Some(layer) = icons.layer(&shown) {
        painted.push(layer);
    }
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
            hub.task.resize(size);
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
/// before it would never reach the canvas. The item card draws after the
/// windows: it is an upper-layer form and the windows are on the normal
/// layer ([`crate::item_card::CARD_ATTRIBUTES`]).
pub fn add_hub_draw(app: &mut App) {
    app.add_systems(
        Update,
        draw_hub_windows
            .after(crate::gui_screens::draw_screens)
            .after(crate::arena_hub_live::HubWindowSystems)
            .before(crate::item_card::draw_card)
            .in_set(exa_physics::FramePhase::Hub),
    );
}

/// The hub draw's optional resources: the loadout kit's views, the item
/// icons, the scripted pointer and the interface level's glyph atlas.
type HubDrawExtras<'w> = (
    Option<Res<'w, crate::loadout_kit::KitPanels>>,
    Option<ResMut<'w, crate::item_panel_draw::PanelIcons>>,
    Option<Res<'w, exa_ui::ScriptedPointer>>,
    Option<Res<'w, exa_ui::GuiLevelState>>,
    Option<Res<'w, bevy::diagnostic::FrameCount>>,
);

/// The Company Status form's emblem: the company's emblem 48 units
/// square, centred across the form's client and four units down from its
/// top ([`exa_ui::dialogs::status_emblem_place`]), in screen pixels. `None`
/// while its pictures are not loaded.
pub fn status_emblem(
    form: &exa_ui::record_form::RecordForm,
    emblem: &exa_ui::EmblemRecord,
    textures: &GuiTextures,
) -> Option<exa_ui::gui_draw::GuiLayer> {
    let unit = form.unit();
    let client_width = form.rect().w - 2 * form.geometry.border;
    let (centre, top, size) = exa_ui::dialogs::status_emblem_place(client_width, unit);
    let quad = crate::emblem::emblem_quad(centre, top, size, unit);
    let mut layer = crate::emblem::emblem_layer(emblem, textures, quad)?;
    let (ox, oy) = form.origin();
    gui_paint::offset_batch(&mut layer.batch, ox, oy);
    Some(layer)
}

/// The form's caret bit: the blink turns it every fifth frame (lit
/// without a frame count).
fn caret_lit(frames: Option<&bevy::diagnostic::FrameCount>) -> bool {
    frames.is_none_or(|f| (f.0 / 5) % 2 == 0)
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
    (kit, mut icons, scripted, glyphs, frame_count): HubDrawExtras,
    loadout_dots: Option<Res<crate::loadout_kit::LoadoutDots>>,
    career: Option<Res<ArenaCareer>>,
) {
    let Some(windows) = windows else {
        return;
    };
    // The company dialog and the colour picker stand on their own, without
    // a hub window under them.
    let forms_only = !windows.is_open() && !windows.forms.is_empty();
    if !windows.is_open() && !forms_only {
        // A reopened window fades in again even if its form id comes back.
        fade.form = None;
        return;
    }
    if game_screens.is_none() {
        frames.clear();
    }
    let bounds = match windows.bounds() {
        Some(bounds) => bounds,
        None if forms_only => GuiRect::new(0, 0, windows.task.screen.0, windows.task.screen.1),
        None => return,
    };
    let empty = GuiTextures::default();
    let textures = textures.as_deref().unwrap_or(&empty);
    let unit = screen
        .single()
        .ok()
        .map(|window| exa_ui::scale::gui_scale(window.physical_height(), 0) as i32)
        .unwrap_or(4);
    let now = time.map_or(0, |time| time.elapsed().as_millis() as u32);
    let pointer = screen.single().ok().and_then(|window| {
        let at = match scripted.as_deref() {
            Some(scripted) => scripted.position(window),
            None => window.cursor_position(),
        }?;
        let at = (at * window.scale_factor()).as_ivec2();
        Some((at.x, at.y))
    });
    let alpha = windows
        .form
        .map_or(1.0, |form| fade.alpha(form, windows.kind, now));
    let look = Look::new(alpha, unit, textures, glyphs.as_deref());
    let painted = match windows.kind {
        HubWindowKind::Board => {
            windows
                .board
                .as_ref()
                .zip(windows.board_form.as_ref())
                .map(|(board, form)| {
                    let drag = windows.board_tables.drag.filter(|drag| {
                        pointer.is_some_and(|at| drag.moved(at, windows.task.screen.1))
                    });
                    paint_board(board, form, &look, pointer, drag)
                })
        }
        HubWindowKind::Manage => windows.manage.as_ref().and_then(|manage| {
            windows
                .forms
                .get(crate::hub_forms::FormWindow::Manage)
                .map(|form| {
                    let tables = &windows.forms.manage_tables;
                    let drag = tables.drag.filter(|drag| {
                        pointer.is_some_and(|at| drag.moved(at, windows.task.screen.1))
                    });
                    paint_manage(manage, form, &look, tables.selected, pointer, drag)
                })
        }),
        HubWindowKind::Shop => windows
            .shop
            .as_ref()
            .map(|shop| paint_shop(shop, bounds, &look, icons.as_deref_mut(), pointer)),
        HubWindowKind::None => forms_only.then(gui_paint::Painted::default),
    };
    let Some(mut painted) = painted else {
        debug!("hub windows: {:?} open but nothing to paint", windows.kind);
        return;
    };
    // The windows stacked over it, bottom to top, drawn shown.
    let shown = Look::new(1.0, unit, textures, glyphs.as_deref());
    for open in &windows.forms.forms {
        if open.window == crate::hub_forms::FormWindow::Loadout {
            // The loadout form's paint fills it black before the base
            // form draws ([`exa_ui::loadout::BACKDROP_INK`]).
            let r = open.form.rect();
            painted.push(exa_ui::loadout::solid_layer(
                [r.x, r.y, r.w, r.h],
                [0, 0, 0, u8::MAX],
            ));
        }
        if open.window == crate::hub_forms::FormWindow::Company {
            // The company dialog's paint: a quad over the whole form in its
            // colour, opaque black, before the base paint.
            let r = open.form.rect();
            let [red, green, blue, alpha] = crate::emblem::COMPANY_FORM_COLOUR;
            painted.push(exa_ui::loadout::solid_layer(
                [r.x, r.y, r.w, r.h],
                [red, green, blue, alpha],
            ));
        }
        let options = &windows.forms.options;
        let extras = crate::hub_forms::FormExtras {
            focus: options
                .focus
                .filter(|(window, _)| *window == open.window)
                .map(|(_, record)| record),
            picker: (open.window == crate::hub_forms::FormWindow::Picker)
                .then(|| options.picker.as_ref().map(|state| &state.picker))
                .flatten(),
            key_focus: options
                .key_focus
                .filter(|(window, _)| *window == open.window)
                .map(|(_, record)| record),
            edits: options
                .edits
                .iter()
                .filter(|e| e.window == open.window)
                .map(|e| (e.record, e.view))
                .collect(),
            caret_lit: caret_lit(frame_count.as_deref()),
            dots: (open.window == crate::hub_forms::FormWindow::Loadout)
                .then(|| loadout_dots.as_deref().and_then(|dots| dots.sheet.as_ref()))
                .flatten(),
        };
        if open.window == crate::hub_forms::FormWindow::Status
            && let Some(career) = career.as_deref()
            && let Some(layer) = status_emblem(&open.form, &career.company.emblem, textures)
        {
            // The status form's paint draws the company's emblem before
            // its base paint.
            painted.push(layer);
        }
        let frame = match open.window {
            crate::hub_forms::FormWindow::Manage => None,
            // The picker keeps the draw skin's default instance: its
            // background and records are drawn in place, with no frame
            // layers over its children.
            crate::hub_forms::FormWindow::Picker => {
                crate::base_form::paint_base_form(&mut painted, &open.form, &shown, &extras);
                None
            }
            _ => {
                Some(crate::hub_forms::paint_form_with(&mut painted, &open.form, &shown, &extras).0)
            }
        };
        if open.window == crate::hub_forms::FormWindow::Emblem
            && let Some(state) = options.emblem.as_ref()
        {
            crate::option_forms::paint_emblem_preview(&mut painted, &open.form, state, &shown);
        }
        if open.window == crate::hub_forms::FormWindow::Loadout
            && let Some(kit) = kit.as_deref()
        {
            crate::loadout_kit::paint_kit(&mut painted, kit, icons.as_deref_mut(), &shown, pointer);
        }
        painted.layers.extend(frame.into_iter().flatten());
    }
    if let Some(popup) = windows.forms.options.popup.as_ref() {
        crate::option_forms::paint_popup(&mut painted, popup, &shown);
    }
    if let Some(dragging) = windows.forms.options.dragging.as_ref() {
        crate::option_forms::paint_colour_drag(
            &mut painted,
            dragging,
            windows.forms.options.last_at,
        );
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

    /// The report names a saved fighter by id (0 the manager) and counts
    /// the hire list's professions; the board takes the career's
    /// tournament entries and the season's week winners.
    #[test]
    fn the_report_and_the_board_read_the_career() {
        let mut career = career();
        career.player.name = "Boss".into();
        career
            .company
            .hire("Anna", crate::career::Profession::Combatant)
            .unwrap();
        career.log.append_value(crate::arena_match::LOG_SAVED, 1);
        career.log.append_value(crate::arena_match::LOG_SAVED, 0);
        career.hire_pool = vec![career.company.roster[0].clone(); 2];
        career.hire_pool[0].profession = crate::career::Profession::Merchant;
        career.hire_pool[1].profession = crate::career::Profession::Merchant;
        let report = career.manage().report;
        let texts: Vec<&str> = report.iter().map(|line| line.text.as_str()).collect();
        assert!(texts.contains(&"Anna was saved from lethal injury."));
        assert!(texts.contains(&"Boss was saved from lethal injury."));
        assert!(texts.contains(&"2 merchants are available for hire."));
        assert!(
            report
                .iter()
                .any(|line| line.code == crate::report::LINE_MERCHANTS)
        );

        career.company.week_winners = [1, 5, 0, 0];
        career.entries[2].count = 1;
        career.entries[2].team_id = 1;
        career.entries[2].entries[0].actor_id = 1;
        career.entries[2].entries[0].weapon = [0x0000_0003_0000_0042, 0];
        let board = career.board_with(None, &|item| item + 1);
        assert_eq!(board.week_winners, [1, 5, 0, 0]);
        assert_eq!(board.entries[2].winner, 1);
        assert_eq!(
            board.entries[2].participants,
            vec![exa_ui::match_board::Participant {
                slot: 1,
                value: 0x42,
                weapon: 0x43,
            }]
        );
        assert!(board.has_entrants());
        // On the tournament's day the board opens on the tournament tab.
        career.company.day = 7;
        assert_eq!(career.board().tab, BoardTab::Tournament);
        career.entries = Default::default();
        assert_ne!(career.board().tab, BoardTab::Tournament);
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
            outfit: Default::default(),
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

    /// Icon slots for tests: one per item, numbered from 1.
    fn slots() -> impl FnMut(u32, u32) -> (usize, (i32, i32)) {
        let mut next = 0;
        move |_, _| {
            next += 1;
            (next, (80, 80))
        }
    }

    fn open(windows: &mut HubWindows, merchant: bool, career: &ArenaCareer) {
        let pack = pack_list(&career.goods, &[]);
        windows.open_shop(merchant, career, &pack, slots());
    }

    #[test]
    fn the_shop_lists_its_shelf_and_the_merchant_his_own() {
        let mut career = career();
        career.company.stock = vec![5, 6];
        career.merchant_stock = vec![9];
        let mut windows = HubWindows::default();
        open(&mut windows, false, &career);
        let shop = windows.shop.as_ref().unwrap();
        assert_eq!(shop.panels.ids(ShopPanel::Stock), [5, 6]);
        assert!(!shop.merchant);
        windows.close_window();
        // The closed window's icons go back to the icon manager.
        assert_eq!(windows.released_icons, [1, 2]);
        open(&mut windows, true, &career);
        let shop = windows.shop.as_ref().unwrap();
        assert_eq!(shop.panels.ids(ShopPanel::Stock), [9]);
        assert!(shop.merchant);
    }

    #[test]
    fn the_pack_takes_the_carried_places() {
        let carried = [
            exa_items::possession::Carried {
                id: 4,
                flags: 0x100,
                x: 32,
                y: 48,
            },
            exa_items::possession::Carried {
                id: 4,
                flags: 0,
                x: 64,
                y: 16,
            },
        ];
        let list = pack_list(&[4, 4, 5], &carried);
        assert_eq!((list[0].flags, list[0].x), (0x100, 32));
        assert_eq!((list[1].flags, list[1].x), (0, 64));
        assert_eq!((list[2].id, list[2].x), (5, 0));
    }

    /// The board's tables draw their selected rows' bands through their
    /// table views: the three standard lists from the start, the company
    /// table once a row is selected; a drag past the drag distance draws
    /// its label at the pointer.
    #[test]
    fn the_board_tables_draw_selections_and_the_drag_label() {
        let mut windows = HubWindows::default();
        windows.task.screen = (1280, 720);
        windows.open_board(&career());
        let textures = GuiTextures::default();
        let look = Look::new(1.0, 4, &textures, None);
        let board = windows.board.clone().unwrap();
        let form = windows.board_form.clone().unwrap();
        let bands = |board: &Board| {
            [
                TableSlot::Match,
                TableSlot::Rank,
                TableSlot::Tier,
                TableSlot::Roster,
            ]
            .iter()
            .filter(|&&slot| board.table_view(slot).selection_band().is_some())
            .count()
        };
        assert_eq!(bands(&board), 3);
        let plain = paint_board(&board, &form, &look, None, None).layers.len();
        let mut selected = board.clone();
        selected.available_selection = Some(0);
        assert_eq!(bands(&selected), 4);
        let more = paint_board(&selected, &form, &look, None, None)
            .layers
            .len();
        assert_eq!(more, plain + 1);
        let drag = TableDrag {
            slot: TableSlot::Roster,
            row: 0,
            kind: exa_ui::match_board::FIGHTER_DRAG,
            grab: (4, 12),
            from: (300, 400),
        };
        assert!(!drag.moved((301, 401), 720));
        assert!(drag.moved((320, 400), 720));
        let dragged = paint_board(&selected, &form, &look, Some((320, 400)), Some(drag));
        assert_eq!(dragged.layers.len(), more + 1);
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

    /// Drags entry `index` of `from` into `to` through the window's own
    /// press and release.
    fn drag(windows: &mut HubWindows, from: ShopPanel, index: usize, to: ShopPanel) -> ShopDrop {
        let at = windows.shop_entry_centre(from, index).unwrap();
        assert!(windows.shop_press(at, |_| 0).is_some());
        windows
            .shop_release(HubWindows::shop_panel_middle(to), (80, 80), true)
            .unwrap()
    }

    #[test]
    fn a_sale_leaves_the_goods_and_a_purchase_leaves_the_window() {
        let mut career = career();
        career.company.stock = vec![7];
        career.goods = vec![9];
        let mut windows = HubWindows::default();
        open(&mut windows, false, &career);
        windows.price_shop_items(|id| match id {
            7 => 5,
            9 => 12,
            _ => 0,
        });
        drag(&mut windows, ShopPanel::Pack, 0, ShopPanel::Sell);
        assert_eq!(windows.shop.as_ref().unwrap().window.sell_items, [9]);
        let point = (10, SHOP_SIZE.1 - 20);
        assert!(windows.shop_click(point, &mut career, None).is_some());
        assert_eq!(career.goods, Vec::<u32>::new());
        assert!(career.company.stock.contains(&9));
        // The sold item is on the shop's side now.
        let shop = windows.shop.as_ref().unwrap();
        assert!(shop.panels.ids(ShopPanel::Stock).contains(&9));
        open(&mut windows, false, &career);
        windows.price_shop_items(|id| if id == 7 { 5 } else { 0 });
        drag(&mut windows, ShopPanel::Stock, 0, ShopPanel::Buy);
        let trade = windows.shop_click(point, &mut career, None).unwrap();
        assert_eq!(
            trade,
            ShopTrade {
                sold: vec![],
                bought: vec![7]
            }
        );
        // The purchase goes to the arsenal through the arena's bought
        // hook, not into the pack.
        assert_eq!(career.goods, Vec::<u32>::new());
        assert!(!career.company.stock.contains(&7));
        let shop = windows.shop.as_ref().unwrap();
        assert!(shop.panels.ids(ShopPanel::Pack).is_empty());
    }

    /// The stock's and the buy basket's items are held with the shop's
    /// flag, the pack's and the sell basket's with none; a point between
    /// the icons or on an empty basket holds none.
    #[test]
    fn the_trade_window_finds_the_item_a_card_is_asked_for() {
        let mut career = career();
        career.company.stock = vec![7, 8];
        career.goods = vec![9];
        let mut windows = HubWindows::default();
        open(&mut windows, false, &career);
        let at = |w: &HubWindows, panel, i| w.shop_entry_centre(panel, i).unwrap();
        let stock0 = at(&windows, ShopPanel::Stock, 0);
        assert_eq!(windows.shop_item_at(stock0), Some((7, SHOP_ITEM_FLAGS)));
        let stock1 = at(&windows, ShopPanel::Stock, 1);
        assert_eq!(windows.shop_item_at(stock1), Some((8, SHOP_ITEM_FLAGS)));
        let pack0 = at(&windows, ShopPanel::Pack, 0);
        assert_eq!(windows.shop_item_at(pack0), Some((9, 0)));
        let sell = HubWindows::shop_panel_middle(ShopPanel::Sell);
        assert_eq!(windows.shop_item_at(sell), None);
        // The baskets' items give cards too.
        drag(&mut windows, ShopPanel::Stock, 1, ShopPanel::Buy);
        let buy0 = at(&windows, ShopPanel::Buy, 0);
        assert_eq!(windows.shop_item_at(buy0), Some((8, SHOP_ITEM_FLAGS)));
        drag(&mut windows, ShopPanel::Pack, 0, ShopPanel::Sell);
        let sell0 = at(&windows, ShopPanel::Sell, 0);
        assert_eq!(windows.shop_item_at(sell0), Some((9, 0)));
    }

    #[test]
    fn a_basket_sums_its_items_value_words() {
        let mut career = career();
        career.company.stock = vec![7, 9];
        let mut windows = HubWindows::default();
        open(&mut windows, false, &career);
        windows.price_shop_items(|id| if id == 7 { 5 } else { 12 });
        assert!(matches!(
            drag(&mut windows, ShopPanel::Stock, 0, ShopPanel::Buy),
            ShopDrop::Moved { id: 7, .. }
        ));
        let shop = windows.shop.as_ref().unwrap();
        assert_eq!(shop.panels.ids(ShopPanel::Stock), [9]);
        assert_eq!(shop.window.buy_items, [7]);
        assert_eq!(shop.window.buy, [5]);
        // Dragged back, the basket is empty again.
        drag(&mut windows, ShopPanel::Buy, 0, ShopPanel::Stock);
        let shop = windows.shop.as_ref().unwrap();
        assert!(shop.window.buy_items.is_empty());
        assert_eq!(shop.window.totals.0, 0);
        // The sell basket refuses a shop item.
        assert_eq!(
            drag(&mut windows, ShopPanel::Stock, 0, ShopPanel::Sell),
            ShopDrop::Cancelled
        );
    }

    #[test]
    fn the_shop_trade_button_settles_through_the_company() {
        let mut career = career();
        career.company.stock = vec![7];
        let mut windows = HubWindows::default();
        open(&mut windows, false, &career);
        windows.shop.as_mut().unwrap().window.add_buy(3);
        let point = (10, SHOP_SIZE.1 - 20);
        assert!(windows.shop_click(point, &mut career, None).is_some());
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
            // Without font textures the captions add no layers: each
            // window's shadow, backdrop and corner frame (the line skin's
            // form draw), the panels, buttons and table panes give 19, the
            // two tables' header rules (the line skin's span bar) two, and
            // the roster's selected first row its band.
            assert_eq!(frames.frames.len(), 1);
            assert_eq!(frames.frames[0].layers.len(), 22);
            // A rule's bar: the bar record from two units in, a header row
            // (six units) less half a unit and one down the table's client.
            let unit = 4.0;
            assert!(frames.frames[0].layers.iter().any(|l| {
                l.batch.vertices.as_chunks::<4>().0.iter().any(|q| {
                    (q[0].u, q[0].v) == (0.3, 0.1)
                        && q[1].y - q[0].y == 2.0 * unit
                        && q[3].x - q[0].x > 10.0 * unit
                })
            }));
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
    fn the_loadout_fighter_list_draws_the_line_skins_rail() {
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
        windows.open_loadout(0, &career());
        let list = windows
            .forms
            .get(crate::hub_forms::FormWindow::Loadout)
            .map(|form| {
                let c = &form.controls()[exa_ui::arena_forms::loadout::LIST];
                let (ox, oy) = form.origin();
                (ox + c.x, oy + c.y, c.w, c.h)
            })
            .expect("the loadout form");
        app.world_mut().insert_resource(windows);
        for _ in 0..3 {
            app.update();
        }
        // The combo box's back is the rail: its thumb samples the rail's
        // record (0.5, 0.8 first) one to three units short of the box's
        // right edge, two units down.
        let frames = app.world().resource::<GuiFrames>();
        let thumb = frames
            .frames
            .iter()
            .flat_map(|f| f.layers.iter())
            .flat_map(|l| l.batch.vertices.as_chunks::<4>().0)
            .find(|q| q[0].u == 0.5 && q[0].v == 0.8 && q[2].u == 0.55)
            .expect("the rail's thumb");
        let (x, y, w, _) = list;
        let unit = 4.0;
        assert_eq!(thumb[0].x, (x + w) as f32 - unit);
        assert_eq!(thumb[2].x, (x + w) as f32 - 3.0 * unit);
        assert_eq!(thumb[0].y, y as f32 + 2.0 * unit);
    }

    #[test]
    fn the_status_form_draws_the_company_emblem_at_its_top_centre() {
        let career = ArenaCareer::default();
        let form = crate::hub_forms::status_form(&career, (1280, 720), 4);
        let mut textures = GuiTextures::default();
        let emblem = &career.company.emblem;
        assert!(status_emblem(&form, emblem, &textures).is_none());
        let solid = exa_ui::gui_draw::GuiTexture::solid([255, 255, 255, 255]);
        for name in exa_ui::preview_textures(emblem) {
            textures.textures.insert(name, solid.clone());
        }
        let layer = status_emblem(&form, emblem, &textures).expect("the emblem draws");
        let exa_ui::gui_draw::LayerTexture::Owned(image) = &layer.texture else {
            panic!("the emblem carries its rendered image");
        };
        // 48 units at unit 4.
        assert_eq!(image.width, 192);
        let (ox, oy) = form.origin();
        let client = form.rect().w - 2 * form.geometry.border;
        assert_eq!(
            crate::emblem::emblem_quad(client / 2, 16, 48, 4),
            [client / 2 - 96, 16, client / 2 + 96, 208]
        );
        let xs: Vec<f32> = layer.batch.vertices.iter().map(|v| v.x).collect();
        let ys: Vec<f32> = layer.batch.vertices.iter().map(|v| v.y).collect();
        let min = |v: &[f32]| v.iter().cloned().fold(f32::MAX, f32::min);
        assert_eq!(min(&xs), (ox + client / 2 - 96) as f32);
        assert_eq!(min(&ys), (oy + 16) as f32);
    }

    #[test]
    fn the_emblem_forms_framed_name_field_draws_the_line_skins_underline() {
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
        windows.open_company();
        let field = windows
            .forms
            .get(crate::hub_forms::FormWindow::Emblem)
            .and_then(|form| {
                let (ox, oy) = form.origin();
                form.controls()
                    .iter()
                    .find(|c| c.class == exa_ui::controls::TEXT_FIELD && c.style & 0xf0 != 0)
                    .map(|c| (ox + c.x, oy + c.y, c.w, c.h))
            })
            .expect("the emblem form's framed name field");
        app.world_mut().insert_resource(windows);
        for _ in 0..3 {
            app.update();
        }
        // The field's back calls the line skin's inner frame slot: the
        // capped span's middle bar runs from one pixel and four units in,
        // its top on the field's bottom less two pixels.
        let (x, y, w, h) = field;
        let unit = 4.0;
        let frames = app.world().resource::<GuiFrames>();
        let found = frames
            .frames
            .iter()
            .flat_map(|f| f.layers.iter())
            .flat_map(|l| l.batch.vertices.as_chunks::<4>().0)
            .any(|q| {
                q[0].x == (x + 1) as f32 + 4.0 * unit
                    && q[0].y == (y + h - 2) as f32
                    && q.iter().any(|v| v.x == (x + w - 1) as f32 - 4.0 * unit)
            });
        assert!(found, "the underline's middle bar");
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

    /// A candidate's price adds four fifths of what its outfit is worth,
    /// and the hire keeps the outfit and the loadouts.
    #[test]
    fn a_candidates_outfit_is_priced_and_kept() {
        let mut career = ArenaCareer::default();
        career.company.roster.clear();
        career.company.copper = 100_000;
        let mut facts = HireFacts::default();
        facts.values.insert(7, 100);
        facts.values.insert(9, 12);
        career.hire_facts = Some(std::sync::Arc::new(facts));
        let bare = career.hire_price(0);
        career.hire_pool[0].outfit = crate::hirelings::Loadout {
            hands: [9, 0],
            items: vec![7, 8],
        };
        career.hire_pool[0].loadouts[5].word = 3;
        let skills = career.hire_pool[0].skills;
        assert_eq!(
            career.hire_price(0),
            crate::career::Company::listed_price(&skills, &[100, 12])
        );
        assert!(career.hire_price(0) > bare);
        assert!(career.hire_candidate(0).is_ok());
        let joined = career.company.roster.last().unwrap();
        assert_eq!(joined.outfit.items, vec![7, 8]);
        assert_eq!(joined.loadouts[5].word, 3);
    }

    /// The specialists' outfit profiles: the physician's flag byte 0x14,
    /// the others' 4.
    #[test]
    fn a_specialist_is_dressed_from_its_profile() {
        assert_eq!(
            specialist_profile(Profession::Physician),
            [0x14, 0, 100, 10, 10, 0, 0, 0]
        );
        assert_eq!(
            specialist_profile(Profession::Trainer),
            [4, 0, 100, 10, 10, 0, 0, 0]
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
