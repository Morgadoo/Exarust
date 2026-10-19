# Proposal

## Why

The sprint latch never tightens the run cap. A far cursor and the walk keys share the looser cap.

## What Changes

- While the sprint latch is kept, the run blend stops at 1 − 0.4 times encumbrance.
- The walk keys, Shift included, stop at 1 − 0.3 times encumbrance.
- Encumbrance 0.5 reaches 0.8 with the cursor 4 m ahead, and 0.85 without it.

## Capabilities

### New Capabilities

### Modified Capabilities

- `player-movement`: the run cap while the sprint latch is kept.

## Impact

- `exa_game` steering, which calls `run_cap` with the latch.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the locomotion run cap, the latch bit, and the doubles 0.4 and 0.3. No new corpus files.
- Out of scope: a blow calling the knock-out, spawning the practice partner, and the stride terms.

## Phase

Phase 3, bodies. The tighter run cap.
