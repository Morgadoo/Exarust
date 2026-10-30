# Proposal

## Why

docs/PLAN.md, Phase 2 (static world), the terrain paragraphs: the round-39
terrain7 lane audited the static terrain and its texture compositor against
the original's instructions and found one deviation, the composite
coordinate `TexPos` for a node deeper than the terrain's block level. It also
added a selector invariant test. The review re-read the coordinate routine's
assembly and corrected the lane's description of it.

## What Changes

- `TexPos` takes the level difference as a wrapping 32-bit subtraction with
  the shift count masked to five bits, so a node deeper than the block level
  keeps no block bits and reads half its split; the half split is a signed
  32-bit power of two; the whole-tile test is signed.
- The docs say what the live draw passes: the block node's own level (3),
  not the level of the set it is baked into.
- A test drives 400 blocks through eighty frames of a sweeping focus and
  checks that no set is handed out twice and that the pools always balance.

## Evidence

The objdump listing of the coordinate routine and its caller in the terrain
draw: the node's level and column/row read as 32-bit fields, `1 << level`
converted from a signed 32-bit integer over the tile side, a signed
`level <= 0` branch to the 0.5 constant, a 32-bit `terrain − level`
subtraction used as a `shr` count (masked to five bits by the instruction),
the shifted index sign-extended and subtracted in 64 bits.

## Out of scope

No new routine is wired: the live draw only reaches the equal-level path.
The composite path stays opt-in behind `EXA_TERRAIN_SETS=1`; the GPU mask
and height passes, the Global Wetness blend and the priority slot remain
tested models as before.
