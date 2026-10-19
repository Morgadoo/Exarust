# Proposal

## Why

The combatant's replace method swaps a target pointer for a new one in its counted list, its two stored slots and the body's hit and contact pointers. Exarust does not model it.

## What Changes

- The first entry equal to the old target becomes the new one; a missing entry leaves the list alone.
- Slots 0x778 and 0x770 become the new target when they equal the old.
- The body's hit pointer becomes the new target's 0x10 word and its contact pointer becomes the new target, each when equal to the old.

## Capabilities

### New Capabilities

### Modified Capabilities

- `combat`: replacing a combatant's target.

## Impact

- `exa_combat::damage`, from `replace_combat_target`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 180-byte replace method's find and four substitutions. No new corpus files.
- Out of scope: the target objects.

## Phase

Phase 3, bodies and combat. Replacing a target.
