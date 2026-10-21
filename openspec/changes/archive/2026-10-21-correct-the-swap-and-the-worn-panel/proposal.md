# Proposal

## Why

A review of the item possession and GUI rules against the original found
three departures:

- A weapon-set swap traded the two sets even when neither hand's item
  changed. The original exchanges them in the step that re-dresses the
  hands, and skips that step when no hand changes, so the hands' lit and
  flipped flags stayed with their sets.
- A looted body's worn panel placed every worn item and held shield. The
  original asks the icon cache first and leaves out an item it cannot draw,
  which then draws no random numbers either.
- Two condition factors existed: the exact mixed-precision one of the item
  copies, and an all-float one used by the factory weapons' stat pass,
  which also clamped a product past 255 where the original lets the byte
  wrap before the cap.
- A random table's count draw multiplied its span unsigned; the original
  sign-extends it, which matters for a table whose maximum is below its
  minimum.

## What Changes

- The swap trades the sets only when a hand changes.
- The worn panel skips items without an icon.
- One condition factor, the exact one, serves both the item copies and the
  factory weapons' stat pass; that pass scales crush in double and lets a
  byte wrap before the cap, as the copies do.
- The count draw sign-extends its span.
- The item catalogue reads the object database once instead of retrying a
  missing or unreadable one every frame.

## Capabilities

### New Capabilities

### Modified Capabilities

- `carried-items`: the weapon-set swap.
- `inventory-pane`: panel entries (the worn panel).
- `item-instances`: weapon condition; count draws (a new requirement).

## Impact

- `exa_items::possession::Possessions::swap_sets`,
  `exa_game::item_container::worn_layout`,
  `exa_formats::weapon_stats::{condition_factor, condition}`,
  `exa_items::instance::condition_factor`, `exa_items::catalogue::load_catalogue`,
  `exa_items::randomise::Fill`.
- `docs/PLAYER.md` (weapon sets), `docs/UI.md` (item windows).
- Evidence: REA on the weapon-set swap, its hand plan and its deferred
  re-dress step; on the looted body's worn panel and its add step; on the
  weapon condition pass (assembly for the precisions); on the count draw
  (assembly: a sign-extended 64-bit multiply).
- Out of scope: everything the lane left undone (held-model attachment,
  windows, table picks).

## Phase

Phase 4 (items), `docs/PLAN.md` section 7 item 2.
