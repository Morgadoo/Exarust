## Why

Numeric ports are checked against the original's assembly by hand, which is
slow, and the decompiler reorders float sums and misstates the NaN rules of
minimum and maximum, so hand readings have been wrong and later lanes have
had to correct them. "Matches the original" should be an automatic check.

## Evidence

The original's machine code of each checked function, run in a CPU emulator
outside the repository under the engine's floating-point mode (flush to
zero, denormals are zero, every exception masked, round to nearest), with
its SSE arithmetic answered by the processor itself (the emulator's own
floating point ignores flush-to-zero and picks the wrong NaN) and fuzzed
inputs heavy in zeros, signed zeros, denormals, NaNs, infinities and equal
values. The recorded cases name each function by a registry name only; the
map from a name to the original's code, the specs and the case files stay
outside the repository.

33 ports were replayed, 5000 cases each: 29 are identical (some differ only
in a NaN's sign or payload, which the replay counts and does not fail); the
floor and ceiling conversions read a denormal below or above zero where the
original reads zero (fixed here); the door leaf's unmasked normaliser, glam's
normalise and lerp, and a swing's expected time on a non-finite rate differ
as their ledger rows say or as reported.

## What changes

- `exa_oracle` (new crate): the case file reader, the registry of ports by
  name, the replay of a file under the engine's floating-point mode with the
  first differing case described field by field, and a test, ignored by
  default, that replays every case file under `EXA_ORACLE_CASES`.
- `exa_physics::engine_math`: `floor_to_i32` and `ceil_to_i32` read a
  denormal as a zero of its sign, as the original's conversion does under
  denormals-are-zero.

## Out of scope

The recorder itself (it carries the original's addresses), x87 code, and
the functions whose setup is most of the work (large loops over the
engine's containers).
