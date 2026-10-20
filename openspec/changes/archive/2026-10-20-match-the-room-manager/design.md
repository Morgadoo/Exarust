# Design

## Behaviour

The original works in centimetres: a point's cell is `floor(p · 0x3ADA740E) + W/2`, and a cell's centre is `(cell − W/2 + 0.5)·600`. Exarust keeps metres in its public API, so `cell_of` multiplies by `100 · INV_CELL_CM`, the same float product the binary applies to centimetres. Room rectangles convert with the same 600 cm cell and the 0–280 cm ceiling band.

The edge word packs four side nibbles in the low 16 bits and four flags in the high 16. A quarter turn rotates the side nibbles cyclically and rotates the high nibbles' low three bits, leaving their bit 3 in place. The migration step takes `atan2(x, z)` in `[−π, π]`: `|v| < π/4` steps −Z, `|v| > 3π/4` steps +Z, `v ∈ (π/4, 3π/4)` steps −X and `v ∈ (−3π/4, −π/4)` steps +X — opposite to the vector.

## Model

`Rooms::cell_of` uses `INV_CELL_CM`; `rect_world_cm`, `rotate_edge_word` and `cell_step_from_dir` are pure functions.

## Open questions

The room render objects, partner mask, migration payload, descriptor buffers, segment tests and object attachment are separate.

## Docs

`TGridEnvManager` is **45%**. The headline rises.
