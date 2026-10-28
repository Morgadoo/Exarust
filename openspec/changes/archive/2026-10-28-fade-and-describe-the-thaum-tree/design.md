# Design

Rules: [`docs/THAUMATURGY.md`](../../../docs/THAUMATURGY.md) (the power
tree and bar), `crates/exa_game/src/thaum/tree.rs` and
`crates/exa_ui/src/tree_draw.rs`.

## Decisions

- The fades are computed once a frame by `TreeScreen::present_window` and
  handed to the draw as three numbers (backdrop, content, bar child), so
  the software-rasterised draw keeps its single pass.
- `BarPhase::OpeningClosing` carries both clocks: the draw applies the
  opening's rise and pitch, then the closing's fades and shrink, as the
  original's two sequential flag tests do.
- `smoothstep` lives beside `ease` in `tree_draw`; each call site is the
  one the listing shows (the bar's pitch is the smooth step of the ease).
- Text metrics are pixels at the GUI unit: the GUI font's advances times
  the unit over four (its native unit). The description wraps with the
  GUI's own line scan (`text_layout::scan_line`).
- The spline's end handles (±2/30 of the end tangents) are spline
  control data in the original; Exarust's links keep only curve points,
  for both the tree's links and the stretched one.
- The grant routine reads the power's domain tree; `grant_power` takes
  those nodes from its caller, so no second reader is kept.
