# Proposal

## Why

The combatant's second head-tracking entry (`0x1000c0cd0`) is the same relaxation as the first but looks 150 cm ahead and has no −0.6 pitch floor. Exarust models the first variant only.

## What Changes

- The far variant uses a 150 cm look-ahead.
- It keeps the same look bands, π/100 slew and ±π/4 yaw clamp.
- It has no pitch floor.

## Capabilities

### New Capabilities

### Modified Capabilities

- `combat`: the far head-tracking variant.

## Impact

- `exa_combat::aim`, from `track_head_far`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `TCombatant` becomes **33%**. The published headline stays 14.4% and 5.7%; the fresh sum stays about 15.5%.
- Evidence: `0x1000c0cd0`: the 150.0 look-ahead at `0x10030a160` and the absent pitch floor at `0xc1153`, everything else matching `0x1000c0790`. No new corpus files.
- Out of scope: the caller's eye frame.

## Phase

Phase 5, combat bodies. The far head variant.
