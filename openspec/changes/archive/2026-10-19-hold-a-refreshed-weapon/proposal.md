# Proposal

## Why

Phase 6 refreshes reach and thrust preference when a knock-out finishes. That refresh does not yet hold the weapon record again, and it does not yet weigh unarmed hands.

## What Changes

- When the clip base reaches 1 and a weapon record is held, that record is held again.
- A ghoul's arm stiffness becomes 1, and its unarmed hands weigh twice the part's mass.
- A Fael's unarmed hands weigh four times, reach 150 cm, and set arm stiffness to one half.

## Capabilities

### New Capabilities

### Modified Capabilities

- `npc-idle`: the holding pass and the hand weight on a finished wake.

## Impact

- `exa_ai::idle`, calling the weapon refresh already shipped in `exa_formats`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the equip refresh's holding pass, the ghoul and Fael rules, and the unarmed hand weight. No format field changes, so the corpus step is a skip.
- Out of scope: steering along an accepted path.
