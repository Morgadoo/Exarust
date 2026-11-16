## Why

The function oracle's first pass found glam's normalise and lerp differ
from the original's routines in the last bit, a door leaf's set-up using
the masked normaliser where the original's is unmasked, and a swing's
expected time saturating where the original returns the integer
indefinite. About seventy more floating-point-heavy partial rows could be
checked the same way in minutes each.

## Evidence

The original's code of each function run in the emulator on the build
worker (SSE floating point and the approximate reciprocals answered by the
worker's processor), 5000 fuzzed cases each, replayed against the Rust
ports. 110 case files replay identical after the fixes below; two
differences are recorded (the sun view matrix takes an exact reciprocal
where the original takes `RSQRTSS`, the renderer's colour decode takes
`powf` where the original takes its own sRGB routine).

## What changes

- `engine_math`: the engine's lerps (`a + (b − a)·t`), the unmasked
  approximate normaliser, the negations, the `RSQRTSS` length and distance,
  the frame and 4×4 products, the rigid inverse and relative frame, the
  library sine through the x87 unit.
- The original's lerp and exact normaliser at their callers' ports (clip
  roots, camera shake, bend target, balance probes, feet drawing, lever
  normals, twist and head look); the door leaf's set-up takes the
  original's rejection and unmasked normaliser.
- A swing's expected time is `CVTSS2SI`'s word; the stalls take the low 32
  bits of both lengths and the shorter, unsigned.
- Port fixes the oracle showed: sector weight normalisations, the link push
  order, a moved sweep's negative zero, NaN health, the colour picker's
  channel, the card rescale's low word, the keep-off step, danger sight,
  the frustum's sine, the spot shaping, the sun vector's zero, the blast
  bearing and close word, the loyalty tick count.
- `exa_oracle`: the registry in batch modules, 112 ports.

## Out of scope

Switching the remaining glam matrix, inverse, length and negation calls at
their call sites (the engine's forms exist and are checked; the call sites
are listed in the lane's notes), DEC-7 (the approximate reciprocals stay
the processor's).
