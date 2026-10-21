# Design

Formats: [`docs/formats/RSQ.md`](../../../docs/formats/RSQ.md) (Physical
body), [`docs/formats/RFC.md`](../../../docs/formats/RFC.md) (Morph sets).

## Decisions

- Every float rule was read from the assembly, not the pseudo-code: the
  draw order of `FUN`-free random helpers (the first call is the first
  operand the assembly computes), double-precision `CVTSS2SD`/`MULSD`
  sums, `COMISS`/`COMISD` parity branches (a NaN health is not floored, a
  NaN control distance matches, a NaN morph weight applies) and the SIMD
  sum orders of the squared distance and the point transform.
- Duplicates are removed rather than kept beside the existing helpers:
  the AI executor (`exa_combat::attack::execute`), the morph part sort
  (`bake_morph_parts`) and the apparel apply (`evaluate_morph_positions`).
- Functions that only returned constant flags (the creation change,
  refresh and close outcomes) are dropped instead of being credited.
- The built-in race is the executable's record bytes (its non-zero runs),
  read with the same reader as `races.rdb`.

## Open questions

- What the `_Temp` node holds, the meaning of the shock's pain values and
  of the creation paint's `0x10` word: not established; answered by REA on
  their callees when a caller is wired.
- The handle the main-side start sound plays: to be read from the body's
  sound set initialisation (REA).
