## Why

The domain wheel (TDomainWheel) drew and hit-tested its ring at a fixed GUI
scale of one and a fixed 32-pixel size word, so it was right only at the
four-pixel interface unit. The original reads two globals the GUI level
switch writes: the scale (the unit times a quarter) and the wheel's size word
(eight units, written when its texture loads). At unit 5 (a 900 to 1050
pixel tall window) the ring is 95 pixels and the hit reach 26 pixels; at
unit 8 it is 152 and 41. The wheel's confirm also kept a 256 ms hold of its
own, where the original refuses while the parent tree's domain fade-out bit
is set and adds that bit (keeping a running fade-in's bit). The tree built
its child without the wheel's constructor form.

## What Changes

- The wheel's hit test reads the unit: ring `round(76 · unit/4)`, reach
  `round((8 · unit · 0.65)²)`; the tree's pointer passes the live GUI unit.
- The wheel's paint takes the GUI scale `unit · 0.25` (the ring, outer ring,
  frame and label ring follow it).
- The confirm is gated by the tree's fade-out bit; the click ORs the bit in.
- The tree's wheel child is built from `effects::wheel::wheel_form`.

## Impact

- `crates/exa_game/src/effects/wheel.rs`, `crates/exa_game/src/thaum/tree.rs`,
  `crates/exa_game/src/power_screen.rs`, `crates/exa_game/README.md`.
- Unit 4 (the grid's 1280 by 720 quadrants) draws and hits exactly as before.
