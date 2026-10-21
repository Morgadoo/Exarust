# Proposal

## Why

`TGUIItemContainer` (26.1 KB) is the free-form item panel: it scatters item cells, relaxes their overlaps, hit-tests them, and saves and loads their positions. Exarust has no item GUI at all. Its geometry and index maths are exact and now decoded.

## What Changes

- The grid has `cols = round(√(n·W/H))` and `rows = ceil(n/cols)`; entries fill backwards and the last partial row spreads over the panel.
- The cell is 40 GUI units; the pad is a sixteenth and the jitter an eighth.
- The engine's LCG is `state = state·0x8088405 + 1`, with the high half of `state·k`.
- Relaxation uses five passes with divisors 1–5 and slack 2/1; scattering uses a fixed seed and five passes with divisors 2, 2, 3, 3, 4.
- A hit test walks the entries from the top, using the fixed cell box and the icon's alpha.
- Drop clamps the cell centre around the icon; drop raises the entry to the end; removal shifts the tail left.
- The accept test checks the reject mask, self-drop, model, size, capacity, required mask and own-icon rule.
- Character-list positions use sixteenths of a unit, DB positions eightieths; a popup's inner panel is `size·unit + 2·border`.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the item panel's layout, hit test and drop.

## Impact

- `exa_game::item_container`, from `Rng`, `grid_shape`, `grid_cell`, `grid_layout`, `relax_one`, `scatter_all`, `hit_test`, `remove_entry`, `raise`, `drop_cell`, `accepts`, the codecs and `popup_inner`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `TGUIItemContainer` becomes **20%** in the source rating. The headline rises.
- Evidence: the panel's grid, relax, scatter, hit test, accept, drop and remove, list codec, DB codec and popup routines. The LCG is the panel's own; the seeds are `0x80b71a` and `0xba28`. No new corpus files.
- Out of scope: the drag payload and mouse handlers, the item cards, the icon cache, the doll load and the popup placement.

## Phase

Phase 7, items. The item panel's geometry.
