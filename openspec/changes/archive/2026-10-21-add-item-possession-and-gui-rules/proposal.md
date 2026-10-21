# Proposal

## Why

Exarust equipped items only once, from fixed loadouts, and had no model of
what a character holds, carries and keeps in its weapon sets, of the item
windows, the icon cache, or of how a copy of an item differs from its
template. Containers had no contents and random tables were not read.

## What Changes

- Read the object database's container (`0x20000`) and random-table
  (`0x200`) sections and the model section's bulk and compact price.
- Keep each character's possessions as its record does: two weapon sets,
  the worn list and the carried list with held marks; swap the sets, give
  items, ask whether a character has an item through nested containers,
  take items into the hands and out of them, forget a loose item, and the
  golem core crystals. Read the record's hand flags.
- Put the possessions on every placed character and the player, with an
  object-database catalogue; Switch Equipped (R) swaps the player's sets.
- Model the inventory window, the arena arsenal, the ref-counted icon
  cache, the item GUI's sockets, coin, rating and slider controls, drag
  payloads, popup placement, icon framing and halo, and the item panel's
  tint, clamps, held entries and the looted body's worn panel.
- Model item copies: the condition from the template (with the armour
  variants), the price, desirability, protection, spreads and rolls, the
  palettes and part colours, the apparel roll, the refresh of a copy from
  its template, grades, and the world-load fill of containers from random
  tables.

## Capabilities

### New Capabilities

- `item-instances`: copies of items, their rolls and the random tables.

### Modified Capabilities

- `carried-items`: weapon sets, held marks, giving, having, hands, cores.
- `inventory-pane`: the inventory window, arsenal, icon cache, item GUI
  controls and panel entries.

## Impact

- `exa_formats::containers` (new), `exa_formats::characters`
  (`hand_flags`).
- `exa_items`: `possession`, `catalogue`, `inventory_window`, `arsenal`,
  `icon_cache`, `item_gui`, `instance`, `palette`, `refresh`, `randomise`;
  `ItemsPlugin` builds the catalogue and carries out swaps.
- `exa_game::item_container` (panel entries), `exa_game::player` (Switch
  Equipped), the `exarust` viewer (placed characters' possessions).
- Docs: `docs/formats/OTHER.md`, `docs/PLAYER.md`, `docs/UI.md`, the
  `exa_items` README.
- Evidence: REA on `Exanima.exe` 0.9.5 (the character possession routines,
  the inventory, arsenal, icon cache and item GUI code, the item-instance
  helpers and the random-table pass), with the addresses in
  `context/re/item-gui-lane.md`; `objdb.rdb` statistics for the new
  sections (24 random tables, 137 containers).
- Out of scope: drawing any item window or icon (no icon renderer or atlas
  yet), re-attaching held models at run time after a swap, instantiating
  table picks into a level's database, the item-card text templates and
  pronouns, the handling sound, and the swap animation's timing.

## Phase

Phase 4 (items and equipment), `docs/PLAN.md` section 7 item 2.
