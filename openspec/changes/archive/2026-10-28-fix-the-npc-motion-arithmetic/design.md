# Design

The rules stay pure functions in `exa_ai` (`personality`, `combatant`,
`motion`, `lifecycle`, `presence`) on plain values in centimetres. Only the
personality and the ally sense run in the game today: both change the
numbers an NPC brain is built and perceives with, not the systems.

Exarust's `Gaze` and `Turning` split the original's look slots across two
structs; the eased offset is one slot (`Gaze::offset`, `Turning::offset`),
so routines that write it write both. Double-precision steps are written as
`f64` products and sums cast back to `f32`, as the listing converts.

The walk's NaN handling follows the listing's parity jumps. The
re-placement's zero-forward case keeps the stored facing (labelled a guess:
the engine's basis builder is outside the pack).
