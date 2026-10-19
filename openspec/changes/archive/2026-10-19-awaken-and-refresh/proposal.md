# Proposal

## Why

Phase 6 raises tone after a knock-out. The tick that drops the dormant flag does not yet tell witnesses, and the tick that finishes the clip base does not yet refresh the weapons.

## What Changes

- The tick that drops the dormant flag tells each person who can see the body, and nobody else. A later tick tells nobody.
- When the clip base reaches 1, the reach is the greatest weapon reach, or the race reach when no hand bit is set.
- No weapon prefers thrust 0. A zero slash, crush and pierce sum prefers 10. Otherwise the preference is the rounded thrust share times 5, plus 1, and at most 9.

## Capabilities

### New Capabilities

### Modified Capabilities

- `npc-idle`: the awaken notice and the weapon refresh on a wake.

## Impact

- `exa_ai::idle`, on the wake tick already shipped.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the waking tick's event and the weapon refresh it calls. No format field changes, so the corpus step is a skip.
- Out of scope: the witness memory update, the grip pass, and the arm-mass scale.
