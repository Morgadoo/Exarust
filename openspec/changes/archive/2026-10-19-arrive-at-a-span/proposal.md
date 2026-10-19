# Proposal

## Why

Phase 6 can wait at a door and give up after ten seconds. The approach that starts that wait does not yet notice a span within 250 cm, or mark a miss.

## What Changes

- A walker that is not approaching a span stays as it is.
- While the clock is still at the stamp and the walker is busy, the approach stays as it is.
- A missing span, or a point farther than 250 cm from the midpoint, sends the walker back to seeking. The first miss sets only the high skipped bit. A later miss also skips the cursor's low five bits.
- A distance of 250 cm starts the door wait at the current clock.

## Capabilities

### New Capabilities

### Modified Capabilities

- `npc-steering`: arriving at a waypoint span.

## Impact

- `exa_ai::path`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the path step's approach. No format field changes, so the corpus step is a skip.
- Out of scope: the aim snap after the choice, and the other path phases.
