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
- Evidence: the 533-byte `0x1002255b0`: the stored array `0x1b8`, `2.5` at `0x10033acf8`, the asymmetric x and y radii from `0x1a0`/`0x1a4`, the sample `0.2` at `0x10033acb8`, `18.3` at `0x10033acc0`, the unit `0x1002ad3f0`, the turn `0x178` and the page/scrolled centre shifts. No new corpus files.
- Out of scope: the caller that supplies the stored slot and sample.

## Phase

Phase 4, UI. The stored-angle skill satellite.
