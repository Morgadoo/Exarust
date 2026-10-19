# Proposal

## Why

Phase 6 rates a contact and then has the duellist cycle three fixed swings. Once the fight has started, the original picks overhead, thrust or a side from reach, preference and a facing score, and it refuses the swing below eight tenths of maximum energy.

## What Changes

- An unarmed fighter inside eight tenths of reach can swing overhead; otherwise it may step to the other side.
- An armed fighter stepping forward slowly thrusts when its preference wins the draw, and takes the overhead when it is old or its hands are open.
- A facing score above 4 takes the other side. Beyond eight tenths of reach a quadruped thrusts on one draw in four.
- Energy under eight tenths of the maximum refuses the swing. The duellist starts the blow this choice names.

## Capabilities

### New Capabilities

- `npc-attack`: the blow chosen once a fight has started.

### Modified Capabilities

## Impact

- `exa_ai::fight`, and the duellist that starts the chosen blow.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the combat state's attack choice. No format field changes, so the corpus step is a skip.
- Out of scope: hearing, idle activities, the footwork search, and the clip index within a side.
