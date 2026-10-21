# Proposal

## Why

A stored slot's satellite point starts from that slot's ring position and then adds the sample offset at 18.3 units. Exarust has the ring position and the hit test's inline samples but not this routine.

## What Changes

- The node angle is `(stored − 2.5) × 2π / count + 1 − turn`.
- The node x radius is the product of both offsets, the unit and the cosine; the node y radius uses only the first offset, the unit and the sine.
- The sample angle is `(sample − 1) × 0.2` of a turn.
- The rounded `unit × 18.3 × cos` is added to x and the rounded `unit × 18.3 × sin` is subtracted from y, each times the turn.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the stored-angle skill satellite.

## Impact

- `exa_ui::skill`, from `skill_satellite`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `TSkillUI` moves from 9% to 12%.
- Evidence: the 533-byte satellite placement: the stored array, the `2.5`, the asymmetric x and y radii, the sample `0.2` and `18.3`, the unit global, the turn and the page and scroll centre shifts. No new corpus files.
- Out of scope: the caller that supplies the stored slot and sample.

## Phase

Phase 4, UI. The stored-angle skill satellite.
