## Why

Round 33's blood audit read the spray unit's jet routine and reset against the assembly. The jet's frame was made unit exactly and tied to the struck part by a full inverse, where the original normalises with the processor's approximate reciprocal square root and ties the frame by the transpose of the part's turn. The world reset that drops the spray jets had been removed, although the original runs it on the spray emitter.

Phase 5 (visual and audio fidelity) in docs/PLAN.md.

## What Changes

- The jet's frame normalises its axis and its first axis with the approximate reciprocal square root, and is tied to the struck part by the transpose of the part's turn.
- A world teardown drops every jet; droplets already flying, the emission clock and the slot sizes stay.
- The weapon's blood sum uses the original list's own clamp rule, shared by the list model and the running game's per-weapon blood.

## Evidence

The spray unit's add-jet routine (normalise, cross with (0, 0, 1), relative-frame helper, twice the turned axis) and reset (source count to 0, bound corners zeroed), the weapon-blood add routine's two compares, read in the lightfx pack's assembly.

## Out of scope

The original's weapon-blood list being cleared at the same reset (the running game keeps a weapon's blood on the weapon), the emitter's bound corners (the spray mesh is never culled).
