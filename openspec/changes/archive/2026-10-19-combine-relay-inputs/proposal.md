# Proposal

## Why

Phase 2 names a relay kind and does not yet run it. A combiner does not set a bit, a pair does not forward, and a sequence does not count.

## What Changes

- A combiner sets the source's bit from the value's low bit. A match signals 1, or 0 when inverted. A mismatch signals the opposite, unless silenced. The bit is still written when silenced.
- The second combiner signals 1 on a match. A mismatch forwards the original value to the source's second target when the state's low bit is clear.
- A pair sends the same value and source to both targets.
- A sequence ignores 0. A marked input counts one step and flags a wrong order when the mark overlaps the previous one. Eight clean steps signal the first target. Fewer steps, or a wrong order, signal the second. Either finish resets.

## Capabilities

### New Capabilities

### Modified Capabilities

- `object-physics`: the relay bodies.

## Impact

- `exa_physics::relay`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the relay steps. No format field changes, so the corpus step is skipped.
- Out of scope: the broadcast across every sector.
