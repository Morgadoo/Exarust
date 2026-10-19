# Proposal

## Why

Phase 6 can approach a span and wait at a door. The look that those phases ease toward the heading does not yet draw its offset or wrap past half a turn.

## What Changes

- A look that is not wandering stays as it is.
- A wandering look copies the heading and starts from the extracted angle.
- At the clock the offset and the roll stay. Behind the clock the offset is the unit times 3.6 minus 1.8, the roll steps, and the next timer adds 4000 plus the high part of that step times 4000.
- An aim past half a turn wraps once. Posture 1 eases by 0.03.

## Capabilities

### New Capabilities

### Modified Capabilities

- `npc-steering`: the wandering look on a path.

## Impact

- `exa_ai::path`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the look step the path phases run. No format field changes, so the corpus step is a skip.
- Out of scope: the flag call, the angle extract, and the aim snap.
