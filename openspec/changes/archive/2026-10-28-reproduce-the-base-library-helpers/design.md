# Design

The helpers are plain functions and small types in the crates that already
hold their neighbours: names and compares in `exa_formats::text`, packed
colours in `exa_formats::colour`, vector helpers in
`exa_physics::engine_math`, hook lists in a new `exa_game::hooks`, the
deferred ring's tags in `exa_game::frame`.

- A Rust closure has no address, so hooks and deferred calls carry
  caller-supplied `u64` identities; order, duplicate refusal and
  drop-on-false are the original's.
- Packing to three bytes leaves the destination's fourth byte alone in the
  original; the Rust functions return a word with that byte zero.
- Slices carry their length, so the bounded compares use the common length
  of the two slices where the original takes a count.
- Nothing in the running frame loop changed: no system calls the new code
  yet, so frame time and load time are unaffected.
