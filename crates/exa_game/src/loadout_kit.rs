//! The hub loadout form's item side: the fighter's kit list, the arsenal
//! box it takes from, the character preview between them, and the drop
//! rules that move items from one to the other.
//!
//! The form keeps three item views besides its records:
//! - the **kit list** (left, 4, 180, 228 wide, 492 tall times the form's
//!   wide-interface scale): the items the shown person has taken out of the
//!   arsenal. The manager's is the hub stash, an employee's is his kit list
//!   ([`crate::loadout::reload_source`]). An entry the shown rank wears or
//!   holds in one of its two hands carries the equipped mark (0x200,
//!   [`exa_ui::loadout::EQUIPPED_MARK`]).
//! - the **arsenal box** (right, 236 wide, 628 tall, 548 wide-interface, at
//!   y 88, four pixels from the form's right edge): the box of the arsenal
//!   shelf the shelf and box buttons chose. An entry lent out to a fighter
//!   carries the lent bit (0x400, the roster page's filter).
//! - the **preview** (x 236, the form's width less 2 × 236 + 8): the shown
//!   person wearing the shown rank's kit.
//!
//! The rules, from the form's own callbacks:
//! - **Equip** (a drag out of the arsenal box onto the preview): an entry
//!   whose flags share the filter is refused; then the equip gate
//!   ([`exa_ui::loadout::may_equip`]): the item's rank at most the tab's (5
//!   when none) and the loadout's cost plus the item's within twice the
//!   rank's point limit. A weapon costs its points less what it displaces
//!   from the hands ([`exa_ui::loadout::item_cost`]); a shield and an
//!   apparel cost their own five-bit points. The drag's start lends the
//!   entry (its flags gain 0x400), the item joins the kit list, the preview
//!   takes it (a weapon or shield into the hands by the character view's
//!   take-in rule, an apparel after the conflict pass), and the change
//!   stores the outfit into the tab's loadout.
//! - **Return** (a drag of a kit-list entry onto the arsenal box): when the
//!   item's shelf is the one shown, the first box entry lent with the item
//!   takes it back (the lent bit clears); otherwise the arsenal is asked
//!   whether it lent the item ([`crate::arsenal::Arsenal::release`], the
//!   stock's lent query, every box of the item's shelf). An item neither
//!   took back goes into the shown box with no flags when its shelf is the
//!   one shown, else onto its shelf's first box with no flags. The entry
//!   leaves the kit list and every loadout keeps at most as many hand
//!   copies as the list still holds, losing its first worn copy
//!   ([`exa_character::record::CharacterData::loadout_remove_item`]).
//!
//! The drags are the item panel's own ([`crate::item_container`]): a left
//! press on an entry of either view marks it dragged with the grab at half
//! the icon, and the left release ends it: an arsenal entry released over
//! the preview is equipped, a kit-list entry released over the arsenal box
//! is returned, a release in the entry's own view only moves it there
//! ([`Container::drop_entry`]), and a release anywhere else cancels and
//! the entry stays. The dragged icon follows the pointer. The two views
//! lay their entries out by the item panel's grid pass
//! ([`crate::item_container::grid_layout`]) [guess: the stored list
//! positions and the box sort are not read].
//!
//! Evidence: the loadout form's constructor, its drop, accept and drag
//! callbacks for both containers and the character view, the record view,
//! the tab switch, the points line, the mark and the arsenal's shelf
//! routines, read from `Exanima.exe` (`context/re/arena-manage.md`).

use bevy::prelude::*;
use exa_character::record::CharacterData;
use exa_formats::characters::{CharacterRecord, ItemRef, Loadout};
use exa_formats::objdb::Item;
use exa_items::possession::{self, Hand, Possessions};
use exa_ui::loadout::{EQUIPPED_MARK, EquipFacts, EquipKind, HeldWeapon, LoadoutMode};

use crate::arsenal::{Arsenal, ArsenalEntry, ItemKind, shelf_of};
use crate::item_container::{Cell, Container, Entry, grid_layout};

/// The roster page's filter: the bit a lent arsenal entry carries.
pub const LENT: u32 = 0x400;
/// The kit list's place on the form at the base unit: x, y and width; its
/// height is [`KIT_HEIGHT`] times the form's scale.
pub const KIT_RECT: [i32; 3] = [4, 180, 228];
pub const KIT_HEIGHT: f32 = 492.0;
/// The arsenal box's width, y and height (narrow and wide), four pixels
/// from the form's right edge.
pub const STOCK_WIDTH: i32 = 236;
pub const STOCK_Y: i32 = 88;
pub const STOCK_HEIGHT: i32 = 628;
pub const STOCK_HEIGHT_WIDE: i32 = 548;
/// The preview's left edge and the margin it leaves.
pub const PREVIEW_X: i32 = 236;
pub const PREVIEW_MARGIN: i32 = 8;
/// The grid pass's seed [guess: the panel's own generator is not seeded
/// by anything the form sets].
pub const GRID_SEED: u32 = 0;

/// The three views on the screen, in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KitRects {
    pub kit: [i32; 4],
    pub stock: [i32; 4],
    pub preview: [i32; 4],
}

/// The views of a form at `origin`, `width` wide and `height` tall on the
/// screen (its design size is 1024 × 768 at unit 4), at GUI `unit`, in the
/// narrow or wide interface. The places are the constructor's at unit 4,
/// scaled by `unit / 4`.
pub fn kit_rects(origin: (i32, i32), width: i32, height: i32, unit: i32, wide: bool) -> KitRects {
    let s = |v: i32| v * unit / 4;
    let scale = exa_ui::loadout::opacity(wide);
    let kit_h = exa_ui::engine_math::round_i64(KIT_HEIGHT * scale) as i32;
    let stock_h = if wide {
        STOCK_HEIGHT_WIDE
    } else {
        STOCK_HEIGHT
    };
    let stock_w = s(STOCK_WIDTH);
    KitRects {
        kit: [
            origin.0 + s(KIT_RECT[0]),
            origin.1 + s(KIT_RECT[1]),
            s(KIT_RECT[2]),
            s(kit_h),
        ],
        stock: [
            origin.0 + width - stock_w - s(4),
            origin.1 + s(STOCK_Y),
            stock_w,
            s(stock_h),
        ],
        preview: [
            origin.0 + s(PREVIEW_X),
            origin.1,
            width - s(PREVIEW_X * 2 + PREVIEW_MARGIN),
            height - s(4),
        ],
    }
}

/// What the rules read about an item: its record, when the database holds
/// it, and its hand kind.
pub struct ItemLookup<'a> {
    pub find: &'a dyn Fn(u32) -> Option<&'a Item>,
    pub kind: &'a dyn Fn(u32) -> Option<possession::ItemKind>,
    pub shelf: &'a dyn Fn(u32) -> ItemKind,
}

/// The item's rank: the top three bits of its rank word's low byte.
fn item_rank(item: &Item) -> u8 {
    (item.rank_word as u8) >> 5
}

/// A weapon in a hand as the cost rule reads it.
fn held_weapon(id: u32, lookup: &ItemLookup) -> Option<HeldWeapon> {
    if id == 0 {
        return None;
    }
    let item = (lookup.find)(id)?;
    let weapon = item.weapon?;
    Some(HeldWeapon {
        family: weapon.flags & 0xf0,
        points: item.loadout_rank,
    })
}

/// A hand's shield points, when it holds a shield.
fn shield_points(id: u32, lookup: &ItemLookup) -> Option<i32> {
    if id == 0 {
        return None;
    }
    let item = (lookup.find)(id)?;
    (item.has_shield && !item.has_weapon).then_some(item.loadout_rank)
}

/// The equip rule's kind of an item for a loadout: a weapon (its rank and
/// what it costs with the loadout's hands), a shield or an apparel (rank
/// and five-bit points), or nothing the rule knows.
pub fn equip_kind(id: u32, loadout: &Loadout, lookup: &ItemLookup) -> EquipKind {
    let Some(item) = (lookup.find)(id) else {
        return EquipKind::None;
    };
    if let Some(weapon) = item.weapon {
        let family = weapon.flags & 0xf0;
        let first = held_weapon(loadout.hands[0].id, lookup);
        // The one-handed branch reads the first hand's shield section, the
        // two-handed one the second hand's.
        let second_shield = if family == 0x10 {
            shield_points(loadout.hands[0].id, lookup).filter(|_| loadout.hands[1].id != 0)
        } else {
            shield_points(loadout.hands[1].id, lookup)
        };
        let cost =
            exa_ui::loadout::item_cost(family, item.loadout_rank, item.torch, first, second_shield);
        return EquipKind::Weapon {
            rank: item_rank(item),
            cost,
        };
    }
    if item.has_shield {
        return EquipKind::Armour {
            rank: item_rank(item),
            points: item.loadout_rank,
        };
    }
    if item.has_body {
        return EquipKind::Worn {
            rank: (item.body_rank as u8) >> 5,
            points: item.worn_rank(),
        };
    }
    EquipKind::None
}

/// The loadout's cost, in the half points the gate compares.
pub fn loadout_cost(loadout: &Loadout, lookup: &ItemLookup) -> i32 {
    CharacterData::loadout_rank(loadout, |id| (lookup.find)(id))
}

/// The gate's limit for a tab: twice the rank's point limit.
pub fn tab_limit(tab: i32) -> i32 {
    crate::manage::points_line(0, tab, &crate::arena::POINT_LIMITS).1 as i32
}

/// Why an equip was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The box entry is already lent out.
    Lent,
    /// The rank or the points do not allow it.
    Gate,
    /// The apparel's conflict pass kept a locked piece on.
    Locked,
    /// No such entry.
    Missing,
}

/// What an equip did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Equipped {
    pub id: u32,
    /// The loadout's cost after it, in half points.
    pub cost: i32,
    /// Worn items the conflict pass took off.
    pub removed: Vec<u32>,
}

/// An arsenal box entry dragged onto the preview of `loadout` (tab `tab`):
/// the filter, the gate, then the lend, the kit list and the loadout.
pub fn equip_from_stock(
    entries: &mut [ArsenalEntry],
    index: usize,
    list: &mut Vec<u32>,
    loadout: &mut Loadout,
    tab: i32,
    lookup: &ItemLookup,
) -> Result<Equipped, Refusal> {
    let entry = *entries.get(index).ok_or(Refusal::Missing)?;
    if entry.flags & LENT != 0 {
        return Err(Refusal::Lent);
    }
    let facts = EquipFacts {
        from_kit: false,
        held: 0,
        copies: 0,
        allowed: true,
        kind: equip_kind(entry.item, loadout, lookup),
    };
    let used = loadout_cost(loadout, lookup);
    if !exa_ui::loadout::may_equip(LoadoutMode::Arena, tab, used, tab_limit(tab), facts) {
        return Err(Refusal::Gate);
    }
    let item = (lookup.find)(entry.item);
    let mut removed = Vec::new();
    let hand_kind = (lookup.kind)(entry.item).unwrap_or_default();
    if hand_kind.weapon.is_some() || hand_kind.shield {
        let mut p = Possessions {
            out: [hand_of(loadout.hands[0]), hand_of(loadout.hands[1])],
            stowed: [hand_of(loadout.hands[2]), hand_of(loadout.hands[3])],
            ..Possessions::default()
        };
        crate::doll_view::hold_item(&mut p, entry.item, 0, hand_kind, false, |id| {
            (lookup.kind)(id)
        });
        loadout.hands = [p.out[0], p.out[1], p.stowed[0], p.stowed[1]]
            .map(|hand| ItemRef::new(hand.id, hand.flags));
    } else if let Some(item) = item {
        let mut data = CharacterData::of_record(CharacterRecord {
            worn: loadout.worn.clone(),
            ..CharacterRecord::default()
        });
        let taken = data
            .equip_conflicts(item, |id| (lookup.find)(id))
            .ok_or(Refusal::Locked)?;
        removed = taken.iter().map(|worn| worn.id).collect();
        loadout.worn = data.record.worn;
        loadout.worn.push(ItemRef::new(entry.item, 0));
    } else {
        return Err(Refusal::Gate);
    }
    entries[index].flags |= LENT;
    list.push(entry.item);
    Ok(Equipped {
        id: entry.item,
        cost: loadout_cost(loadout, lookup),
        removed,
    })
}

fn hand_of(slot: ItemRef) -> Hand {
    Hand {
        id: slot.id,
        flags: slot.flags,
    }
}

/// Where a returned item went.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Returned {
    /// The shown box's lent entry took it back.
    Box,
    /// The arsenal's lent query took it back elsewhere on its shelf.
    Lent,
    /// Unlent, it joined the shown box with no flags.
    ShownBox,
    /// Unlent, it went onto its own shelf's first box with no flags.
    Shelf,
    /// It goes on no shelf and stays out of the arsenal.
    Nowhere,
}

/// What a return did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReturnDone {
    pub id: u32,
    pub went: Returned,
    /// The copies the kit list still holds: the hand copies each loadout
    /// may keep.
    pub kept: usize,
    /// The shown tab's loadout changed.
    pub tab_changed: bool,
}

/// A kit-list entry dragged onto the arsenal box showing `shown` (shelf,
/// box). `loadouts` are the person's six; `tab` the shown one.
#[allow(clippy::too_many_arguments)]
pub fn return_to_stock(
    arsenal: &mut Arsenal,
    shown: (usize, usize),
    list: &mut Vec<u32>,
    index: usize,
    loadouts: &mut [Loadout],
    tab: i32,
    lookup: &ItemLookup,
) -> Option<ReturnDone> {
    let id = *list.get(index)?;
    let kind = (lookup.shelf)(id);
    let shelf = shelf_of(kind);
    let on_shown = shelf == Some(shown.0);
    let mut went = None;
    if on_shown
        && let Some(entry) = arsenal.shelves[shown.0][shown.1]
            .iter_mut()
            .find(|entry| entry.item == id && entry.flags & LENT != 0)
    {
        entry.flags &= !LENT;
        went = Some(Returned::Box);
    }
    if went.is_none() && arsenal.release(id, kind, LENT) {
        went = Some(Returned::Lent);
    }
    let went = went.unwrap_or_else(|| {
        if on_shown {
            arsenal.shelves[shown.0][shown.1].push(ArsenalEntry {
                item: id,
                flags: 0,
                first: 0x28,
                second: 4,
            });
            Returned::ShownBox
        } else if arsenal.stow(id, kind, 0) {
            Returned::Shelf
        } else {
            Returned::Nowhere
        }
    });
    list.remove(index);
    let kept = list.iter().filter(|&&held| held == id).count();
    let mut tab_changed = false;
    for (at, loadout) in loadouts.iter_mut().enumerate() {
        let changed = CharacterData::loadout_remove_item(loadout, id, kept);
        if changed && at as i32 == tab {
            tab_changed = true;
        }
    }
    Some(ReturnDone {
        id,
        went,
        kept,
        tab_changed,
    })
}

/// The list entries' flags: the equipped mark on each item the loadout
/// wears or holds in one of its first two hands.
pub fn list_flags(list: &[u32], loadout: &Loadout) -> Vec<u32> {
    list.iter()
        .map(|&id| {
            let held = loadout.hands[0].id == id || loadout.hands[1].id == id;
            let worn = loadout.worn.iter().any(|worn| worn.id == id);
            if held || worn { EQUIPPED_MARK } else { 0 }
        })
        .collect()
}

/// A panel holding `items` (id and flags) laid out by the grid pass in a
/// client `size`, with `icon` px icons; `slot` gives each item's icon slot
/// and half extents.
pub fn panel(
    items: &[(u32, u32)],
    size: (i32, i32),
    icon: i32,
    mut slot: impl FnMut(u32, u32) -> (usize, (i32, i32)),
) -> Container {
    let slots: Vec<(usize, (i32, i32))> = items.iter().map(|&(id, f)| slot(id, f)).collect();
    let halves: Vec<[i32; 2]> = slots.iter().map(|(_, h)| [h.0, h.1]).collect();
    let cells = grid_layout(items.len(), size.0, size.1, icon, &halves, GRID_SEED);
    let entries = items
        .iter()
        .zip(&slots)
        .zip(cells.into_iter().chain(std::iter::repeat(Cell::default())))
        .map(|((&(id, flags), &(icon, _)), cell)| Entry {
            cell,
            id,
            icon,
            flags,
            drag: 0,
        })
        .collect();
    Container {
        entries,
        client: size,
        icon_size: icon,
        ..Container::default()
    }
}

/// The open kit: who is shown, at which tab and box, and the two panels.
#[derive(Clone, Debug, Default)]
pub struct KitState {
    /// The roster row shown (0 the manager).
    pub row: i32,
    pub tab: i32,
    pub shelf: u32,
    pub box_index: u32,
    pub rects: Option<KitRects>,
    pub list: Container,
    pub stock: Container,
    /// The preview's record: rebuilt when `revision` changes.
    pub revision: u64,
    /// What the panels were built from.
    pub snapshot: KitSnapshot,
    /// The drag in flight.
    pub drag: Option<KitDrag>,
}

/// A drag out of one of the kit's views.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KitDrag {
    /// From the arsenal box (else the kit list).
    pub from_stock: bool,
    pub index: usize,
    pub icon: usize,
    pub grab: Cell,
}

/// What the views are built from: rebuilt when any of it changes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct KitSnapshot {
    pub row: i32,
    pub tab: i32,
    pub shelf: u32,
    pub box_index: u32,
    pub rects: Option<KitRects>,
    pub list: Vec<u32>,
    pub boxed: Vec<(u32, u32)>,
    pub loadout: Loadout,
}

/// The hub loadout form's item views, while it is open on the roster.
#[derive(Resource, Debug, Default)]
pub struct KitPanels {
    pub open: Option<KitState>,
    /// Set by a press that changed the stores, so the panels rebuild.
    pub dirty: bool,
    /// The preview's revision counter.
    pub revision: u64,
}

/// What the preview shows: its screen rect, the record and the worn
/// items, and a revision that changes when either does.
#[derive(Resource, Debug, Default, Clone)]
pub struct KitPreview {
    pub rect: Option<exa_ui::gui_core::GuiRect>,
    pub record: Option<CharacterRecord>,
    pub revision: u64,
}

/// The shown row's person: the manager (row 0) or an employee.
fn member_mut(
    career: &mut crate::hub_windows::ArenaCareer,
    row: i32,
) -> Option<&mut crate::career::Member> {
    if row < 1 {
        Some(&mut career.player)
    } else {
        career.company.roster.get_mut(row as usize - 1)
    }
}

fn member(career: &crate::hub_windows::ArenaCareer, row: i32) -> Option<&crate::career::Member> {
    if row < 1 {
        Some(&career.player)
    } else {
        career.company.roster.get(row as usize - 1)
    }
}

/// The shown row's kit list: the hub stash for the manager, the
/// employee's kit list otherwise ([`crate::loadout::reload_source`]).
fn list_of(stores: &crate::save_live::ArenaStores, row: i32) -> &[u32] {
    if row < 1 {
        &stores.stash
    } else {
        stores
            .staff
            .employees
            .get(row as usize - 1)
            .map_or(&[], |employee| employee.kit.as_slice())
    }
}

/// The open roster form's view state: its row, tab, shelf and box, and
/// the form's place and unit.
fn open_view(
    windows: &crate::hub_windows::HubWindows,
) -> Option<(crate::manage::ArsenalForm, exa_ui::record_form::RecordForm)> {
    let arsenal = windows.forms.arsenal?;
    if arsenal.page != crate::manage::PAGE_ROSTER {
        return None;
    }
    let form = windows
        .forms
        .get(crate::hub_forms::FormWindow::Loadout)?
        .clone();
    Some((arsenal, form))
}

/// Keeps the kit's views in step with the open form: rebuilt when the
/// shown row, tab, shelf or box changes or a press changed the stores;
/// the points line from the tab's loadout; the preview's record.
#[allow(clippy::too_many_arguments)]
pub fn sync_kit(
    mut windows: ResMut<crate::hub_windows::HubWindows>,
    career: Res<crate::hub_windows::ArenaCareer>,
    stores: Option<Res<crate::save_live::ArenaStores>>,
    mut icons: Option<ResMut<crate::item_panel_draw::PanelIcons>>,
    catalogue: Option<Res<exa_items::catalogue::ItemCatalogue>>,
    game: Option<Res<exa_assets::Game>>,
    mut items: ResMut<crate::doll_view::DollItems>,
    mut panels: ResMut<KitPanels>,
    mut preview: ResMut<KitPreview>,
    players: Query<&CharacterData, With<crate::player::PlayerBody>>,
) {
    let (Some((arsenal, form)), Some(stores)) = (open_view(&windows), stores) else {
        if panels.open.take().is_some() {
            info!("loadout kit: the form's item views close");
        }
        preview.rect = None;
        return;
    };
    let row = arsenal.selection;
    let tab = arsenal.tab;
    let r = form.rect();
    let rects = kit_rects((r.x, r.y), r.w, r.h, form.unit(), false);
    let Some(person) = member(&career, row) else {
        return;
    };
    let loadout = person.loadouts[tab.clamp(0, 5) as usize].clone();
    let list = list_of(&stores, row).to_vec();
    let shown = (arsenal.shelf as usize, arsenal.box_index as usize);
    let boxed: Vec<(u32, u32)> = stores
        .arsenal
        .shelves
        .get(shown.0)
        .and_then(|shelf| shelf.get(shown.1))
        .map(|entries| entries.iter().map(|e| (e.item, e.flags)).collect())
        .unwrap_or_default();
    let snapshot = KitSnapshot {
        row,
        tab,
        shelf: arsenal.shelf,
        box_index: arsenal.box_index,
        rects: Some(rects),
        list: list.clone(),
        boxed: boxed.clone(),
        loadout: loadout.clone(),
    };
    if !panels.dirty
        && panels
            .open
            .as_ref()
            .is_some_and(|open| open.snapshot == snapshot)
    {
        return;
    }
    panels.dirty = false;
    items.ensure(game.as_deref());
    let flags = list_flags(&list, &loadout);
    let listed: Vec<(u32, u32)> = list.iter().copied().zip(flags).collect();
    let icon = 40 * form.unit();
    let catalogue = catalogue.as_deref();
    let mut reserve = |id: u32, flags: u32| match icons.as_deref_mut() {
        Some(icons) => icons.reserve(id, flags, catalogue),
        None => (0, (icon / 2, icon / 2)),
    };
    let mut list_panel = panel(&listed, (rects.kit[2], rects.kit[3]), icon, &mut reserve);
    // The kit list's highlight is the equipped mark, the arsenal box's the
    // lend [guess: the word each container is given is its yellow mask].
    list_panel.yellow_mask = EQUIPPED_MARK;
    let mut stock_panel = panel(&boxed, (rects.stock[2], rects.stock[3]), icon, &mut reserve);
    stock_panel.yellow_mask = LENT;
    let find = |id: u32| items.get(id);
    let kind = |id: u32| catalogue.and_then(|c| c.kind(id));
    let shelf = |_: u32| ItemKind::default();
    let lookup = ItemLookup {
        find: &find,
        kind: &kind,
        shelf: &shelf,
    };
    let cost = loadout_cost(&loadout, &lookup);
    if let Some(form) = windows.forms.get_mut(crate::hub_forms::FormWindow::Loadout)
        && let Some(line) = crate::manage::points_line(cost, tab, &crate::arena::POINT_LIMITS).2
    {
        form.set_text(exa_ui::arena_forms::loadout::POINTS, &line);
    }
    info!(
        "loadout kit: row {row} rank {tab} shelf {} box {}: list {:?}, box {:?}, hands {:?}, worn {:?}, points {cost} of {}",
        shown.0,
        shown.1,
        list,
        boxed,
        loadout.hands.map(|h| h.id),
        loadout.worn.iter().map(|w| w.id).collect::<Vec<_>>(),
        tab_limit(tab)
    );
    // The preview: the manager's live record, else the creator's defaults,
    // wearing the tab's kit.
    let mut record = if row < 1 {
        players
            .iter()
            .next()
            .map(|data| data.record.clone())
            .unwrap_or_else(CharacterRecord::reset)
    } else {
        CharacterRecord::reset()
    };
    record.name = person.name.clone();
    record.hands = loadout.hands;
    record.worn = loadout.worn.clone();
    let [px, py, pw, ph] = rects.preview;
    preview.rect = Some(exa_ui::gui_core::GuiRect::new(px, py, pw, ph));
    if preview.record.as_ref() != Some(&record) {
        panels.revision += 1;
        preview.revision = panels.revision;
        preview.record = Some(record);
    }
    let centre = |panel: &Container, at: [i32; 4]| -> Vec<(u32, i32, i32)> {
        let half = panel.icon_size / 2;
        panel
            .entries
            .iter()
            .map(|e| (e.id, at[0] + e.cell.x + half, at[1] + e.cell.y + half))
            .collect()
    };
    info!(
        "loadout kit: entry centres (id, x, y): list {:?}, box {:?}",
        centre(&list_panel, rects.kit),
        centre(&stock_panel, rects.stock)
    );
    panels.open = Some(KitState {
        row,
        tab,
        shelf: arsenal.shelf,
        box_index: arsenal.box_index,
        rects: Some(rects),
        list: list_panel,
        stock: stock_panel,
        revision: panels.revision,
        snapshot,
        drag: None,
    });
}

/// The entry under a screen point in a panel drawn at `rect`.
fn entry_at(panel: &Container, rect: [i32; 4], at: (i32, i32)) -> Option<usize> {
    let (x, y) = (at.0 - rect[0], at.1 - rect[1]);
    if x < 0 || y < 0 || x >= rect[2] || y >= rect[3] {
        return None;
    }
    crate::item_container::hit_test(panel.entries.len(), panel.icon_size, x, y, |i| {
        let e = &panel.entries[i];
        (e.cell, 0, true)
    })
}

/// Whether a screen point is inside a view's rect.
fn inside(rect: [i32; 4], at: (i32, i32)) -> bool {
    at.0 >= rect[0] && at.1 >= rect[1] && at.0 < rect[0] + rect[2] && at.1 < rect[1] + rect[3]
}

/// What a kit drag's release does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KitRelease {
    /// An arsenal entry over the preview: equip it.
    Equip(usize),
    /// A kit-list entry over the arsenal box: return it.
    Return(usize),
    /// Over the entry's own view: move it there.
    Reposition,
    /// Anywhere else: the entry stays.
    Cancel,
}

/// The release of `drag` at screen point `at` over the kit's `rects`.
pub fn kit_release(drag: KitDrag, rects: KitRects, at: (i32, i32)) -> KitRelease {
    let own = if drag.from_stock {
        rects.stock
    } else {
        rects.kit
    };
    if inside(own, at) {
        KitRelease::Reposition
    } else if drag.from_stock && inside(rects.preview, at) {
        KitRelease::Equip(drag.index)
    } else if !drag.from_stock && inside(rects.stock, at) {
        KitRelease::Return(drag.index)
    } else {
        KitRelease::Cancel
    }
}

/// The kit's views take the left button as their item panels do: a press
/// on an entry starts its drag, and the release equips (an arsenal entry
/// over the preview), returns (a kit-list entry over the arsenal box),
/// moves it in its own view or cancels ([`kit_release`]).
#[allow(clippy::too_many_arguments)]
pub fn kit_input(
    mouse: Res<ButtonInput<MouseButton>>,
    windows_q: Query<&Window, With<bevy::window::PrimaryWindow>>,
    scripted: Option<Res<exa_ui::ScriptedPointer>>,
    windows: Res<crate::hub_windows::HubWindows>,
    mut career: ResMut<crate::hub_windows::ArenaCareer>,
    stores: Option<ResMut<crate::save_live::ArenaStores>>,
    kinds: Option<Res<crate::arsenal::ShelfKinds>>,
    catalogue: Option<Res<exa_items::catalogue::ItemCatalogue>>,
    game: Option<Res<exa_assets::Game>>,
    mut items: ResMut<crate::doll_view::DollItems>,
    mut panels: ResMut<KitPanels>,
    icons: Option<Res<crate::item_panel_draw::PanelIcons>>,
) {
    let pressed = mouse.just_pressed(MouseButton::Left);
    let released = mouse.just_released(MouseButton::Left);
    if !pressed && !released {
        return;
    }
    let top = windows.forms.top().map(|f| f.window);
    if top != Some(crate::hub_forms::FormWindow::Loadout) {
        if let Some(open) = panels.open.as_mut()
            && open.drag.take().is_some()
        {
            crate::item_container::clear_drags(&mut open.list.entries);
            crate::item_container::clear_drags(&mut open.stock.entries);
        }
        return;
    }
    let (Some(open), Some(mut stores)) = (panels.open.as_mut(), stores) else {
        return;
    };
    let Some(rects) = open.rects else {
        return;
    };
    let Some(at) = windows_q.single().ok().and_then(|window| {
        let at = match scripted.as_deref() {
            Some(scripted) => scripted.position(window),
            None => window.cursor_position(),
        }?;
        Some((
            (at.x * window.scale_factor()) as i32,
            (at.y * window.scale_factor()) as i32,
        ))
    }) else {
        return;
    };
    if pressed && open.drag.is_none() {
        let hit = entry_at(&open.stock, rects.stock, at)
            .map(|i| (true, i))
            .or_else(|| entry_at(&open.list, rects.kit, at).map(|i| (false, i)));
        if let Some((from_stock, index)) = hit {
            let view = if from_stock {
                &mut open.stock
            } else {
                &mut open.list
            };
            let grab = crate::item_container::drag_grab(view.icon_size);
            let entry = &mut view.entries[index];
            entry.drag |= 1;
            open.drag = Some(KitDrag {
                from_stock,
                index,
                icon: entry.icon,
                grab,
            });
            info!(
                "loadout kit: drag of {} from the {} at {at:?}",
                entry.id,
                if from_stock {
                    "arsenal box"
                } else {
                    "kit list"
                }
            );
        }
    }
    if !released {
        return;
    }
    let Some(drag) = open.drag.take() else {
        return;
    };
    let release = kit_release(drag, rects, at);
    info!("loadout kit: the drag ends at {at:?}: {release:?}");
    let (stock_hit, list_hit) = match release {
        KitRelease::Equip(index) => (Some(index), None),
        KitRelease::Return(index) => (None, Some(index)),
        KitRelease::Reposition => {
            let (view, rect) = if drag.from_stock {
                (&mut open.stock, rects.stock)
            } else {
                (&mut open.list, rects.kit)
            };
            let half = icons
                .as_deref()
                .and_then(|icons| icons.half(drag.icon))
                .unwrap_or((0, 0));
            let id = view.entries.get(drag.index).map_or(0, |e| e.id);
            let flags = view.entries.get(drag.index).map_or(0, |e| e.flags);
            view.drop_entry(
                crate::item_container::DropDrag {
                    same_panel: true,
                    linked: false,
                    id,
                    icon: drag.icon,
                    flags,
                    bulk: 0,
                },
                drag.grab,
                Cell::new(at.0 - rect[0], at.1 - rect[1]),
                half,
            );
            return;
        }
        KitRelease::Cancel => {
            crate::item_container::clear_drags(&mut open.list.entries);
            crate::item_container::clear_drags(&mut open.stock.entries);
            return;
        }
    };
    crate::item_container::clear_drags(&mut open.list.entries);
    crate::item_container::clear_drags(&mut open.stock.entries);
    let (row, tab) = (open.row, open.tab);
    let shown = (open.shelf as usize, open.box_index as usize);
    items.ensure(game.as_deref());
    let catalogue = catalogue.as_deref();
    let find = |id: u32| items.get(id);
    let kind = |id: u32| catalogue.and_then(|c| c.kind(id));
    let shelf = |id: u32| {
        kinds
            .as_deref()
            .map_or_else(ItemKind::default, |k| k.kind(id))
    };
    let lookup = ItemLookup {
        find: &find,
        kind: &kind,
        shelf: &shelf,
    };
    let Some(person) = member_mut(&mut career, row) else {
        return;
    };
    let crate::save_live::ArenaStores {
        arsenal,
        stash,
        staff,
    } = &mut *stores;
    let list = if row < 1 {
        Some(stash)
    } else {
        staff
            .employees
            .get_mut(row as usize - 1)
            .map(|employee| &mut employee.kit)
    };
    let Some(list) = list else {
        warn!("loadout kit: row {row} has no kit list on the staff");
        return;
    };
    if let Some(index) = stock_hit {
        let entries = &mut arsenal.shelves[shown.0][shown.1];
        let id = entries.get(index).map_or(0, |e| e.item);
        let loadout = &mut person.loadouts[tab.clamp(0, 5) as usize];
        match equip_from_stock(entries, index, list, loadout, tab, &lookup) {
            Ok(done) => info!(
                "loadout kit: {} takes {} from shelf {} box {} into rank {tab}: hands {:?}, worn {:?}, points {} of {} (off {:?})",
                person.name,
                done.id,
                shown.0,
                shown.1,
                loadout.hands.map(|h| h.id),
                loadout.worn.iter().map(|w| w.id).collect::<Vec<_>>(),
                done.cost,
                tab_limit(tab),
                done.removed
            ),
            Err(refusal) => info!(
                "loadout kit: {} cannot take {id} into rank {tab}: {refusal:?}",
                person.name
            ),
        }
    } else if let Some(index) = list_hit
        && let Some(done) = return_to_stock(
            arsenal,
            shown,
            list,
            index,
            &mut person.loadouts,
            tab,
            &lookup,
        )
    {
        info!(
            "loadout kit: {} returns {} to the arsenal ({:?}); each loadout keeps {} hand copies",
            person.name, done.id, done.went, done.kept
        );
    }
    panels.dirty = true;
}

/// Paints the kit's views into the loadout form: the two panels' frames
/// and icons, and the preview's viewport cleared so the 3D view shows.
pub(crate) fn paint_kit(
    painted: &mut crate::gui_paint::Painted,
    panels: &KitPanels,
    icons: Option<&mut crate::item_panel_draw::PanelIcons>,
    look: &crate::hub_windows::Look<'_>,
    pointer: Option<(i32, i32)>,
) {
    let Some(open) = panels.open.as_ref() else {
        return;
    };
    let Some(rects) = open.rects else {
        return;
    };
    let rect = |r: [i32; 4]| exa_ui::gui_core::GuiRect::new(r[0], r[1], r[2], r[3]);
    painted.push(crate::gui_screens::clear_layer(rect(rects.preview)));
    crate::hub_windows::paint_panel(painted, rect(rects.kit), look);
    crate::hub_windows::paint_panel(painted, rect(rects.stock), look);
    if let Some(icons) = icons {
        for (panel, r) in [(&open.list, rects.kit), (&open.stock, rects.stock)] {
            if let Some(mut layer) = icons.layer(panel) {
                crate::gui_paint::offset_batch(&mut layer.batch, r[0], r[1]);
                layer.clip = Some(r);
                painted.push(layer);
            }
        }
        if let (Some(drag), Some(pointer)) = (open.drag, pointer) {
            let view = if drag.from_stock {
                &open.stock
            } else {
                &open.list
            };
            crate::hub_windows::paint_dragged(painted, icons, view, drag.icon, drag.grab, pointer);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arsenal::{SHIELDS, WEAPONS};
    use exa_formats::objdb::Weapon;

    fn blank(id: u32) -> Item {
        Item {
            id,
            name: String::new(),
            parent: 0,
            model: String::new(),
            model_flags: 0,
            worn_model: String::new(),
            appearance: None,
            slot_layers: [0; exa_formats::objdb::SLOTS],
            slot_words: [0; exa_formats::objdb::SLOTS],
            occupancy: [0; exa_formats::objdb::OCCUPANCY_BONES],
            weapon: None,
            armour: None,
            shield: None,
            door: None,
            operative: None,
            burden: None,
            equip_lock: 0,
            torch: false,
            volume: None,
            volume_script: None,
            use_script: None,
            rank_word: 0,
            body_rank: 0,
            loadout_rank: 0,
            armour_weight: 0.0,
            armour_type: 0,
            container: None,
            random_table: None,
            has_body: false,
            has_weapon: false,
            has_shield: false,
        }
    }

    fn weapon(id: u32, word: u16, family: u32) -> Item {
        Item {
            rank_word: word,
            loadout_rank: i32::from(word & 0x1f),
            weapon: Some(Weapon {
                flags: family,
                ..Weapon::default()
            }),
            has_weapon: true,
            ..blank(id)
        }
    }

    fn apparel(id: u32, word: u16) -> Item {
        Item {
            rank_word: word,
            body_rank: word,
            has_body: true,
            worn_model: "x".into(),
            burden: Some(exa_formats::objdb::Burden::default()),
            ..blank(id)
        }
    }

    struct World {
        items: Vec<Item>,
    }

    impl World {
        fn find(&self, id: u32) -> Option<&Item> {
            self.items.iter().find(|item| item.id == id)
        }
    }

    fn with_lookup<R>(world: &World, f: impl FnOnce(&ItemLookup) -> R) -> R {
        let find = |id: u32| world.find(id);
        let kind = |id: u32| {
            world.find(id).map(|item| possession::ItemKind {
                weapon: item.weapon.map(|w| w.flags),
                shield: item.has_shield,
                ..Default::default()
            })
        };
        let shelf = |id: u32| match world.find(id) {
            Some(item) if item.has_weapon => ItemKind {
                known: true,
                weapon: true,
                ..Default::default()
            },
            Some(item) if item.has_body => ItemKind {
                known: true,
                apparel_layer: Some(1),
                ..Default::default()
            },
            _ => ItemKind::default(),
        };
        f(&ItemLookup {
            find: &find,
            kind: &kind,
            shelf: &shelf,
        })
    }

    fn entry(item: u32, flags: u32) -> ArsenalEntry {
        ArsenalEntry {
            item,
            flags,
            first: 0x28,
            second: 4,
        }
    }

    #[test]
    fn a_kit_drag_equips_on_the_preview_and_returns_on_the_box() {
        let rects = kit_rects((0, 0), 1024, 768, 4, false);
        let drag = |from_stock| KitDrag {
            from_stock,
            index: 2,
            icon: 1,
            grab: Cell::new(80, 80),
        };
        let middle = |r: [i32; 4]| (r[0] + r[2] / 2, r[1] + r[3] / 2);
        assert_eq!(
            kit_release(drag(true), rects, middle(rects.preview)),
            KitRelease::Equip(2)
        );
        assert_eq!(
            kit_release(drag(false), rects, middle(rects.stock)),
            KitRelease::Return(2)
        );
        assert_eq!(
            kit_release(drag(true), rects, middle(rects.stock)),
            KitRelease::Reposition
        );
        // A kit item over the preview, or anything over the records: kept.
        assert_eq!(
            kit_release(drag(false), rects, middle(rects.preview)),
            KitRelease::Cancel
        );
        assert_eq!(kit_release(drag(true), rects, (10, 10)), KitRelease::Cancel);
    }

    #[test]
    fn the_views_sit_where_the_constructor_puts_them() {
        let r = kit_rects((0, 0), 1024, 768, 4, false);
        assert_eq!(r.kit, [4, 180, 228, 492]);
        assert_eq!(r.stock, [784, 88, 236, 628]);
        assert_eq!(r.preview, [236, 0, 544, 764]);
        // The wide interface: 492 × 0.83739835 to even is 412.
        let wide = kit_rects((10, 20), 1024, 768, 4, true);
        assert_eq!(wide.kit, [14, 200, 228, 412]);
        assert_eq!(wide.stock[3], 548);
        // Unit 3 scales every place by three quarters.
        assert_eq!(
            kit_rects((0, 0), 768, 576, 3, false).stock,
            [588, 66, 177, 471]
        );
    }

    #[test]
    fn an_arsenal_weapon_is_lent_listed_and_held_within_the_limit() {
        let world = World {
            items: vec![weapon(10, 4, 0x10), weapon(11, (1 << 5) | 2, 0x10)],
        };
        with_lookup(&world, |lookup| {
            let mut entries = vec![entry(10, 0), entry(11, 0)];
            let mut list = Vec::new();
            let mut loadout = Loadout::default();
            let done =
                equip_from_stock(&mut entries, 0, &mut list, &mut loadout, 0, lookup).unwrap();
            assert_eq!(done.cost, 4);
            assert_eq!(entries[0].flags, LENT);
            assert_eq!(list, vec![10]);
            assert_eq!(loadout.hands[0].id, 10);
            // A lent entry is refused, and a rank-1 item at rank 0.
            assert_eq!(
                equip_from_stock(&mut entries, 0, &mut list, &mut loadout, 0, lookup),
                Err(Refusal::Lent)
            );
            assert_eq!(
                equip_from_stock(&mut entries, 1, &mut list, &mut loadout, 0, lookup),
                Err(Refusal::Gate)
            );
            // At rank 1 it may: it replaces the main hand's weapon, so the
            // cost is its two points (4 freed, 2 taken).
            let done =
                equip_from_stock(&mut entries, 1, &mut list, &mut loadout, 1, lookup).unwrap();
            assert_eq!(done.cost, 2);
        });
    }

    #[test]
    fn the_gate_compares_against_twice_the_ranks_limit() {
        assert_eq!(tab_limit(0), 10);
        assert_eq!(tab_limit(5), 80);
        assert_eq!(tab_limit(-1), 100);
        let world = World {
            items: vec![apparel(20, 9), apparel(21, 2)],
        };
        with_lookup(&world, |lookup| {
            let mut entries = vec![entry(20, 0), entry(21, 0)];
            let mut list = Vec::new();
            let mut loadout = Loadout::default();
            equip_from_stock(&mut entries, 0, &mut list, &mut loadout, 0, lookup).unwrap();
            assert_eq!(loadout.worn, vec![ItemRef::new(20, 0)]);
            // 9 + 2 is past 10.
            assert_eq!(
                equip_from_stock(&mut entries, 1, &mut list, &mut loadout, 0, lookup),
                Err(Refusal::Gate)
            );
        });
    }

    #[test]
    fn a_returned_item_goes_back_to_its_lent_entry_and_leaves_the_loadouts() {
        let world = World {
            items: vec![weapon(10, 4, 0x10), apparel(20, 3)],
        };
        with_lookup(&world, |lookup| {
            let mut arsenal = Arsenal::default();
            arsenal.shelves[WEAPONS][0].push(entry(10, LENT));
            let mut list = vec![10, 20];
            let mut loadouts = vec![Loadout::default(); 6];
            loadouts[0].hands[0] = ItemRef::new(10, 0);
            loadouts[2].hands[0] = ItemRef::new(10, 0);
            loadouts[0].worn = vec![ItemRef::new(20, 0)];
            let done = return_to_stock(
                &mut arsenal,
                (WEAPONS, 0),
                &mut list,
                0,
                &mut loadouts,
                0,
                lookup,
            )
            .unwrap();
            assert_eq!(done.went, Returned::Box);
            assert_eq!(done.kept, 0);
            assert!(done.tab_changed);
            assert_eq!(arsenal.shelves[WEAPONS][0][0].flags, 0);
            assert_eq!(loadouts[0].hands[0], ItemRef::NONE);
            assert_eq!(loadouts[2].hands[0], ItemRef::NONE);
            assert_eq!(list, vec![20]);
            // An unlent apparel, its shelf not shown: onto its shelf.
            let done = return_to_stock(
                &mut arsenal,
                (WEAPONS, 0),
                &mut list,
                0,
                &mut loadouts,
                0,
                lookup,
            )
            .unwrap();
            assert_eq!(done.went, Returned::Shelf);
            assert!(loadouts[0].worn.is_empty());
            assert_eq!(arsenal.shelves[crate::arsenal::CLOTHING][0][0].item, 20);
            assert!(list.is_empty());
        });
    }

    #[test]
    fn the_lent_query_and_the_shown_box_take_what_the_box_does_not() {
        let world = World {
            items: vec![weapon(10, 4, 0x10)],
        };
        with_lookup(&world, |lookup| {
            let mut arsenal = Arsenal::default();
            arsenal.shelves[WEAPONS][3].push(entry(10, LENT));
            let mut list = vec![10, 10];
            let mut loadouts = vec![Loadout::default(); 6];
            loadouts[1].hands = [
                ItemRef::new(10, 0),
                ItemRef::new(10, 0),
                ItemRef::NONE,
                ItemRef::NONE,
            ];
            // Box 0 is shown; box 3 holds the lend.
            let done = return_to_stock(
                &mut arsenal,
                (WEAPONS, 0),
                &mut list,
                0,
                &mut loadouts,
                0,
                lookup,
            )
            .unwrap();
            assert_eq!(done.went, Returned::Lent);
            assert_eq!(arsenal.shelves[WEAPONS][3][0].flags, 0);
            // One copy left: each loadout keeps one hand copy.
            assert_eq!(done.kept, 1);
            assert!(!done.tab_changed);
            assert_eq!(loadouts[1].hands[0].id, 10);
            assert_eq!(loadouts[1].hands[1].id, 0);
            // Nothing lent any more: the shown box takes it with no flags.
            let done = return_to_stock(
                &mut arsenal,
                (WEAPONS, 0),
                &mut list,
                0,
                &mut loadouts,
                0,
                lookup,
            )
            .unwrap();
            assert_eq!(done.went, Returned::ShownBox);
            assert_eq!(arsenal.shelves[WEAPONS][0], vec![entry(10, 0)]);
            assert!(arsenal.shelves[SHIELDS][0].is_empty());
        });
    }

    #[test]
    fn the_list_marks_what_the_tab_wears_or_holds() {
        let mut loadout = Loadout::default();
        loadout.hands[1] = ItemRef::new(5, 0);
        loadout.hands[2] = ItemRef::new(6, 0);
        loadout.worn = vec![ItemRef::new(7, 0)];
        assert_eq!(
            list_flags(&[5, 6, 7, 8], &loadout),
            vec![EQUIPPED_MARK, 0, EQUIPPED_MARK, 0]
        );
    }

    #[test]
    fn a_panel_lays_its_entries_out_in_the_client() {
        let panel = panel(&[(1, 0), (2, LENT)], (236, 628), 160, |id, _| {
            (id as usize, (80, 80))
        });
        assert_eq!(panel.entries.len(), 2);
        assert_eq!(panel.entries[1].flags, LENT);
        assert_eq!(panel.entries[1].icon, 2);
        for e in &panel.entries {
            assert!(e.cell.x >= -80 && e.cell.y >= -80 && e.cell.y < 628);
        }
    }
}
