# Design

## Behaviour

The grid uses the panel's usable width and height (`client − border`, `client − client_top`). Columns are `round(√(count·W/H))` in `f32`; rows are `ceil(count/cols)` in `f64`. Cells fill from the last array index backwards: the final entry takes row 0, column 0. Each coordinate is `round((index + 0.5)·usable/div) − cell/16`, with `div` the smaller of the column count and the remaining entries; the last partial row therefore spreads over the whole width. Every cell then gets two LCG jitter samples of `cell/8`.

Relaxation walks every pair: when the other entry is to the right, the push is `(target + half) − (other − half) + 2`; otherwise `(other + half) − (target − half) + 1`. A push is applied only when it exceeds the pass divisor (1–5 for one entry, 4–8 for the grid), divided with the original's truncation, and moves both entries apart. Scattering seeds the LCG with `0x80b71a`, places each entry in the middle half, and uses divisors 2, 2, 3, 3, 4 with a positive-push guard. Every centre is clamped to `[half, usable − half]`, then shifted by `−cell/2` to the stored top-left.

The hit test runs from the last entry backwards over the fixed `cell × cell` box, skipping `objflags & 0x100`; the icon's alpha decides the pixel. Drop subtracts the grab offset and clamps around the icon's half extents; a drop raises the entry to the end, and removing shifts the tail left.

The accept test rejects the reject mask, a drop of the container's own id, a missing model, a size over the maximum, a capacity overflow (except from the linked panel or itself), and a required-mask mismatch; `only_own` then requires one of the container's entries to carry the icon. Positions are saved in sixteenths (character lists) or eightieths (DB records) of a GUI unit.

## Model

`exa_game::item_container` exposes the pure geometry and index functions plus the `Rng`, `Cell`, `Accept` and `Drag` types.

## Open questions

The drag payload, the mouse handlers, the item cards, the icon cache, the doll load and the popup placement are separate.

## Docs

`TGUIItemContainer` is **20%**. The headline rises.
