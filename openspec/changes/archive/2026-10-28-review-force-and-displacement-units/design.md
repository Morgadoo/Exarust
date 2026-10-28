# Design

Rules: [`docs/THAUMATURGY.md`](../../../docs/THAUMATURGY.md),
`crates/exa_game/src/thaum/{casting,force,displacement,visuals,effect_material,veil,cast}.rs`.

## Decisions

- `casting::{within, not_above, not_below}` express a `COMISS`/`COMISD`
  whose refuse branch sits behind `JP`; each call site names the side the
  instruction refuses.
- `ShaderBook` keeps the made shader objects and the registered
  `ReleaseUnit`s separately: a release runs its unit's callback once (it
  zeroes the handles it frees) and leaves the shaders made, as the
  "registered" bytes and the shader globals do.
- `TendrilImage::end` carries the end routines; `ShellWear` keeps only its
  start state.
- The long door's copied vector is the creator input's heading, the
  character's frame forward axis; the AI cast's is none.
