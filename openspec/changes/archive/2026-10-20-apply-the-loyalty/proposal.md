# Proposal

## Why

`TPwrLoyalty_C` (`0x1001e4690`) applies a loyalty effect to a looked-up target, refuses a missing or marked one, and removes it again. Exarust has the spirit form and sight but not the loyalty power.

## What Changes

- Event 2 on an applying loyalty returns 2; otherwise it moves the state to 2.
- An applying loyalty refuses a missing or marked target into state 2; a valid target applies with marker `0x10010a`, sets state 0 and returns 0.
- A removing loyalty removes the target and returns 2.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the loyalty power.

## Impact

- `exa_game::mind`, from `LoyaltyState`, `LoyaltyStep`, `LOYALTY_MARKER` and `step_loyalty`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Thaum_Mind` becomes **12%**. The published headline stays 14.4% and 5.7%; the fresh sum stays about 15.5%.
- Evidence: `0x1001e4690`: the state at `+0x18`, the target at `+0x28`, the lookup `0x10014e230`, the `+0xdf4 & 0x800` refusal, the effect marker `0x10010a`, the target's `+0xd40->+0x5a=1`, and the removal `0x1001e4610`. No new corpus files.
- Out of scope: the controlled-list allocation and the visual.

## Phase

Phase 7, thaumaturgy. The loyalty power.
