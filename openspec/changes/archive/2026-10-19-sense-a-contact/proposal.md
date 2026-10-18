# Proposal

## Why

Phase 6 still has no perception. `docs/PLAN.md` §7 asks for the NPC brain's targets. A path and a follow order are already in.

## What Changes

- A contact is seen out to 12 m, 6 m while asleep, and 18 m once tracked. Closeness is at most 0.99.
- Hostility toward a negative attitude grows each tick and decays by 0.999. With neutral traits it settles near 0.0375 times the attitude's size.
- That heat chooses notice, watch, engage or flee. An engaging duellist takes the guard.

## Capabilities

### New Capabilities

- `npc-perception`: range, hostility and the notice, watch, engage or flee choice.

### Modified Capabilities

## Impact

- `exa_ai::sense`, and the duellist, which engages when the choice says so.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the perception and target-selection notes already recorded from the original. No format field changes, so the corpus step is a skip.
- Out of scope: hearing, idle activities, and the attack chosen once the fight has started.
