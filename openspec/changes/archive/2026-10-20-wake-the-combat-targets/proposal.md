# Proposal

## Why

The combatant's wake method clears two body bits, wakes the body and calls the hit virtual on each target in the body's list. Exarust does not model it.

## What Changes

- Without the wake argument nothing runs.
- With it, bits 0x6 clear from the body's 0xdf8 word.
- The body wake runs once and each listed target's 0x148 virtual runs once.

## Capabilities

### New Capabilities

### Modified Capabilities

- `combat`: waking a combatant's targets.

## Impact

- `exa_combat::damage`, from `wake_combat_targets`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 164-byte wake method's guard, bit clear and list loop. No new corpus files.
- Out of scope: the wake and virtual bodies.

## Phase

Phase 3, bodies and combat. Waking the targets.
