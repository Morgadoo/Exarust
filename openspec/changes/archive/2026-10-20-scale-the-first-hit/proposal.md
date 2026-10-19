# Proposal

## Why

The combatant's hit method scales the first hit by its stat, raises the target's hit to the threshold when it is lower and marks the target for a special update. Exarust does not model that step.

## What Changes

- A blocked or helpless combatant returns without changes.
- The first hit scales by `(1 + stat) × 0.025 + 0.95` in double precision.
- A target without bit 0x100000 takes the threshold when its hit is lower and gains the bit.
- A body with the bit makes one special call.

## Capabilities

### New Capabilities

### Modified Capabilities

- `combat`: the first-hit scaling and target mark.

## Impact

- `exa_combat::damage`, from `apply_combat_hit`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 238-byte combatant hit method and its 1.0/0.025/0.95 constants. No new corpus files.
- Out of scope: the special call and the hit's physical resolution.

## Phase

Phase 3, bodies and combat. The first-hit scaling.
