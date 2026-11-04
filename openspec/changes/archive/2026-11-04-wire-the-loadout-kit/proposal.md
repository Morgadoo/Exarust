# Proposal

## Why

Wiring backlog item P12: the hub's loadout form showed its records (rows,
rank, points, the shelf and box buttons) but none of its item side. The
kit list, the arsenal box and the character preview were not drawn, no
item could move between the arsenal and a fighter's kit, the points line
always read a cost of 0, and the arena save dropped the manager's six
per-rank kits.

Phase: the arena career milestone of docs/PLAN.md (hub windows); no
PLAN.md edit in this change.

Evidence (REA, Ghidra on `Exanima.exe`): the loadout form's constructor
(the two item containers' places, masks and callbacks, the character
view), its arsenal container's drop handler, the kit container's drop and
accept callbacks, the arsenal container's drag start, the character
view's drop, accept and change callbacks, the record view, the tab
switch, the points routine, the loadout cost, the equipped mark, the box
rebuild, the close, the arsenal's lend query and stow routine, and the
loadout's record copy and item removal.

## What Changes

- `exa_game::loadout_kit` (new): the kit list, the arsenal box and the
  preview's places; the equip from the arsenal (filter, gate, lend, the
  hands' take-in or the apparel's conflict pass) and the return to the
  arsenal (the shown box's lend, the lend query, the stow, the loadouts'
  trim); the panels' build, the presses and the paint.
- `exa_game::hub_windows`: the loadout form is filled black before its
  records and draws its kit after them.
- `exa_game::creator_view`: the creator's view also shows the hub loadout
  preview (the record wearing the shown rank's kit).
- `exa_game::save_live`: the arena save writes the manager's record with
  his six loadouts into the world character database; the load gives the
  career its six loadouts back.
- `exa_game::arena_hub_live`: the shelf facts are read in the hub and the
  `--hub-employee-kit` tool runs (both were never scheduled).
- `exarust`: `--hub-arsenal id,...` stows items in the arsenal for checks.

## Out of scope

- The containers' own drag and drop (a press stands for a drag).
- The figure's held weapons on the preview; the encumbrance value.
- Employees' per-rank kits in the save (the arena load does not restore
  the roster).
- The hire, specialist and event pages' item side.
