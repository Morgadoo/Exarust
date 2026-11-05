# Proposal

## Why

Round 53's glowtrade lane found that the trade window's row labels drew
about one row above the rows that clicks and card requests hit, and its
buy and sell baskets were not drawn, so they gave no cards. The loadout
lane took its two item views' drags as presses. The owner asked for the
hub windows to be the original's.

Tracing the trade window's constructor showed that the rows themselves
were the stand-in: the original has no row layout to align. The pack, the
two baskets and the stock are all free-form item panels, the same panel the
inventory uses, each with its own required flags. Drawing them as item
panels removes the label/row mismatch: what is drawn is what is hit.

Phase: Phase 7 arena milestones of docs/PLAN.md (no PLAN.md edit in this
change).

Evidence (REA, Ghidra on `Exanima.exe`, checked against the disassembly):
the trade window's constructor (its border word, its five children and
each panel's required-flag mask and value and change callbacks), the hub's
opener (the stock list and the grid seed it passes: the day counter plus
one), the window's layout (the pack loaded from the player's inventory
list, the stock from the shelf with the shop's flag and laid out by the
seeded grid pass), the two basket callbacks (each sums its items' value
words), and the item panel's accept, drop, drag-ended and mouse slots
(`context/re/component-map/sections/items.md`).

## What Changes

- `exa_game::shop_panels` (new): the trade window's four item panels
  (pack, sell basket, buy basket, stock) in the window's client (the
  constructor's border of 8), with their required flags; the pack's load
  from the player's list, the stock's load and seeded grid layout; the
  hit test, the card request's item and the drag (press, accept, drop,
  drag end, cancel).
- `exa_game::hub_windows`: the trade window's row stand-in, its row
  height and row-based clicks are gone; the panels are drawn with their
  icons and the dragged icon at the pointer; each basket's change
  re-sums its value words; a confirmed trade moves the baskets' items to
  the other side; the closed window's icons are released.
- `exa_game::arena_hub_live`: the drag's input; the shop's copies are
  registered with the item catalogue when the stock is made, so their
  icons render at once; the scripted purchase drags its stock entry into
  the buy basket.
- `exa_game::loadout_kit`: the kit list's and arsenal box's drags are the
  item panel's: press, dragged icon, release on the preview (equip), on
  the arsenal box (return), in the own view (move) or elsewhere (cancel).
- `exarust`: `--drag-at x,y>x,y@frame[-end]`, a scripted left drag.

## Out of scope

- The character preview's drop (an item from the pack dressed on the
  preview) and its take-off.
- The bottom row's own record template (places of the totals, the label
  and the Trade button); its guessed places are kept.
- The "Trade" button's own callback (the confirmed trade stays Exarust's
  settle).
- The item panel's alpha hit test (the icon's own pixels): the rectangle
  test is used, as the inventory's live panels do.
- A comparison with the retail hub: loading an arena career in the user's
  prefix runs the hub's arrival and may rewrite that save.
