# Proposal

## Why

The combatant's forget method removes a target from its counted list by moving the last entry over it, then clears the stored hover, contact, hit and contact targets when they match. Exarust does not model it.

## What Changes

- A found target removes its entry: the last entry takes its place (or the last is dropped), the count drops by one.
- A missing target changes nothing.
- The two stored pointer slots clear when they equal the target.
- The body's hit target, compared through its 0xd40, and its contact target clear when equal.

## Capabilities

### New Capabilities

### Modified Capabilities

- `combat`: forgetting a combatant's target.

## Impact

- `exa_combat::damage`, from `forget_combat_target`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 234-byte forget method's find, swap and four clears. No new corpus files.
- Out of scope: the target tables' owners.

## Phase

Phase 3, bodies and combat. Forgetting a target.
