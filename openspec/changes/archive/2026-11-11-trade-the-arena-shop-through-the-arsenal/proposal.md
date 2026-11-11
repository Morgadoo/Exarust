## Why

PLAN Phase 7 (arena career), gap lane `arena2`. Round 143's XA_Arena routine
table left the biggest routines uncalled: the shop's sale and purchase hooks
into the arsenal (the live shop traded through the career's goods), the roster
drag, the apparel, weapon and shield row fillers, the roster and report
double presses.

## Evidence

REA (Exanima.exe), read in the assembly: the hub's object click opens the
trade window for the shop and for the company merchant and installs two hooks
the window's transaction calls, one per sold item (the arsenal's sale:
unique items everywhere, else one copy and the holders' allowance) and, in
place of the pack, one per bought item (promote to the world tier, stow with
no flags on its shelf). The outfit generators' first call runs the table
pass once: the apparel filler (body section: class from the armour type,
rank and points, twenty floats from the coverage words and the padding and
slash bytes, constants 0.01, 0.05 and 0.5 in double precision), the weapon
filler (weapon section's rank word at its 0x42) and the shield filler
(shield section's word at 0x08), each giving a zero weight ten. Manage
Company's roster table takes a drop hook (the drag reorder with the id
remap) and both tables a double-press hook (the loadout form on a row; the
report line's action).

## What Changes

- exa_game: `Arsenal::trade` and holders over the stash, the staff kits and
  own items and the members' loadouts; `HubWindows::shop_click` returns the
  trade and the pack no longer takes purchases; `HubShop::arena_trade`.
- exa_game: `convert_pool_entry` and `rank_entries` run the table pass;
  `ItemFacts` carries the body, weapon and shield facts; the dead
  `fill_*_rows` duplicates are gone.
- exa_formats: `Item` keeps the raw coverage words and the weapon and shield
  rank words.
- exa_game: `HubWindows::manage_table_input`, `manage_table_action`,
  `open_loadout_at`, `Staff::drop_target`.
- Debug tools: `--shop-sell ID`, `--manage-table STEPS`, a repeatable
  `--hub-employee-kit`.

## Impact

Purchases land in the arsenal; sales strip the company's holders. Generated
NPC outfits now score apparel by the coverage floats and draw weapons whose
table weight was zero, so generated fighters dress and arm differently.
