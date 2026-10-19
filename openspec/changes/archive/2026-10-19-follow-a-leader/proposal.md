# Proposal

## Why

Phase 6 can pick a wander point. A follower does not yet stop by how the leader faces it, choose an urgency, or replan when the leader has moved.

## What Changes

- No leader stands when there is no activity or panic is at least 1, and otherwise runs the activity.
- Not walking, inside (1 + the leader's facing toward the follower / 2) times 100 cm, closes. At that distance it does not.
- The band is 400 cm, 280 while already following, and 1200 while busy.
- Outside it, urgency is 0 up to 800 cm, 1 up to 1600 cm, and 2 past 1600 cm or when the leader sprints.
- More than 100 cm from the planned spot replans. Exactly 100 cm does not. The plan stops at 50 cm.

## Capabilities

### New Capabilities

### Modified Capabilities

- `npc-idle`: following a leader.

## Impact

- `exa_ai::idle`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the idle follow step. No format field changes, so the corpus step is a skip.
- Out of scope: the path search, the activity tick, and sleep.
