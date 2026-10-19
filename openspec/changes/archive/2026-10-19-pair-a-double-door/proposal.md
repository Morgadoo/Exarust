# Proposal

## Why

Phase 2 slides a door and drives a hinge. A double door does not yet let one leaf go when the other is open, and the two leaves do not yet pass through each other.

## What Changes

- A single shut door stays shut at exactly 0.05 rad short of closed, and leaves once it is farther and not latched, while still being held.
- A paired door whose partner is not within 0.1 rad is not held, and leaves the shut state when its own angle is short of closed. A near partner keeps it shut and held.
- Exactly 0.1 rad short does not count as near. Past closed does.
- A paired door adds the pair bits to its layer.

## Capabilities

### New Capabilities

### Modified Capabilities

- `object-physics`: the partner of a double door.

## Impact

- `exa_physics::door`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the hinged door step's partner check. No format field changes, so the corpus step is a skip.
- Out of scope: the lock and its key, and the slam's random pitch.
