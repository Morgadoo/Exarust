# Design

Rules: `crates/exa_game/src/thaum/effect_pass.rs`,
`crates/exa_game/src/thaum/effect_material.rs`,
`crates/exa_game/src/effects/shell.rs`, `crates/exa_game/src/thaum/cast.rs`
and [`docs/THAUMATURGY.md`](../../../docs/THAUMATURGY.md).

## Decisions

- A pass plan returns no draws below one pass; `depths` lists the layer
  terms the pass still sets (layers `0..=passes`, none for the shell when
  it does not draw), so a renderer can apply and restore them.
- Optional uniforms are `Option`s on each draw: absent when the pass binds
  none, when display bit 0x40 is clear, or when the coordinate has no
  location.
- `effect_pass` reuses `effect_geometry`'s divisor, layer count and row
  helpers; the double literal term lives in `force_row`.
- The node's two words stay apart: `flags` (0x20, 0x21000000, 0x40000) and
  `state` (0x9000, the dead bit).
- The Long Door's height uses the lowest part as the original's body
  height offset (labelled a guess in code).
