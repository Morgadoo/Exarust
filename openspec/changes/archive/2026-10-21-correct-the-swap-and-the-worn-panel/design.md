# Design

## Behaviour

- Swap: the record flag toggles and the held marks move as before; the
  hands' plan is computed; when it is empty (no hand changes) the original
  frees it and never runs the step that exchanges the record's two sets.
- Worn panel: each worn item, then each out hand holding a shield, is
  offered to the icon cache with variant 0; only a positive slot adds an
  entry, and only then are the two position draws made.
- Condition: the factor is `(1 − d·k₁)·(f32(q·k₂) + 0.8)` with `k₁` the
  double 0.4/255 and `k₂` the float 0.5/255, the product in double and
  stored as float. Slash, pierce and thrust multiply in float, crush computes
  `(0.7c + 0.3)·byte` in double; each converts to the nearest integer and
  keeps its low byte, then caps at 160.
- Count draws: the span `max − min + 1` is sign-extended to 64 bits and
  multiplied by the zero-extended generator state; the high word is the
  draw.

## Model

`swap_sets` swaps `out` and `stowed` only for `Some` plan. `worn_layout`
takes an icon callback returning the slot and half extents.
`weapon_stats::condition_factor` is the exact factor and
`instance::condition_factor` re-exports it.

## Open questions

None.

## Docs

`docs/PLAYER.md`, `docs/UI.md`.
