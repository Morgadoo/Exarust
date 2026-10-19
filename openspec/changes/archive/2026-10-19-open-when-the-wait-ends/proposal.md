# Proposal

## Why

Phase 2 ends a signalled wait and does not yet open the door. The 120th tick should clear the latch and start the open.

## What Changes

- When a signalled wait ends it opens the door. The latch, the shut flag and the pull count clear.
- A hinge that is not motorised goes idle and is shoved 3 cm along local Z. A motorised hinge sets its fraction to 0 and drives open for 100 ticks, with the same shove.
- A sliding door that is not motorised jumps to fraction 0.6 for 80 ticks and wakes. A motorised sliding door drives open for 300 ticks, keeps its fraction, and wakes.

## Capabilities

### New Capabilities

### Modified Capabilities

- `object-physics`: the open a signalled wait ends by calling.

## Impact

- `exa_physics::door`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the open command. No format field changes, so the corpus step is a skip.
- Out of scope: the unlatch sound's random pitch, and telling navigation.
