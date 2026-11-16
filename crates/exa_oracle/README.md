# `exa_oracle`

The function oracle's Rust side: recorded calls of the original's functions
replayed against the Rust ports, bit for bit.

A case file holds many calls of one original function: each call's input
bytes (its arguments in call order, a buffer passed by address as its bytes)
and the bytes it produced (its out buffers, then its return value), recorded
by running the original machine code in an emulator under the engine's
floating-point mode. The recorder, its specs and the case files live outside
the repository (they carry the original's addresses); this crate knows the
functions by name only.

- `ports()` maps each name to a closure that decodes a case's inputs, calls
  the Rust port and encodes its outputs in the case's layout (33 functions:
  the engine maths' normalisers, lerps, minimum and maximum, conversions,
  logarithm, exponential, sine-cosine, cosine, heading, axis-angle
  quaternion, triple product, barycentric weights, the 4x3 weighted
  difference, the face record's builder and closest point, the colour lerp,
  a door leaf's openness and a swing's expected time).
- `replay()` runs every case of a file under
  `exa_physics::float_mode::enter` and reports the function identical, or
  its first differing case with the inputs and each differing output's bits.
  A NaN on both sides with other sign or payload bits is counted, not
  failed (the processor keeps the first NaN operand; the compiler may swap a
  commutative operation's operands).
- `tests/replay.rs` replays every `*.cases` file under `EXA_ORACLE_CASES`;
  it is ignored by default:

  `EXA_ORACLE_CASES=<dir> cargo test -p exa_oracle --test replay -- --ignored --nocapture`

Case file layout (little-endian): `EXAORC01`, a `u32` metadata length and
the UTF-8 metadata (`key=value` lines, then `in <kind> <field>` and
`out <kind> <field>` lines in byte order), the input length, output length
and case count (`u32` each), then each case's input and output bytes. Field
kinds: `f32`, `f64`, `i32`, `u32`, `i64`, `u8`, `padN` and `ref:…` (pointer
words; zero in the case bytes and not compared).

To add a function: record its cases, then register its port here under the
same name.
