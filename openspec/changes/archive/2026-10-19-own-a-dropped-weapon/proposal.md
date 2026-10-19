# Proposal

## Why

A weapon that leaves a collapsing fighter still names the fighter as its owner and stays in its parent's part list.

## What Changes

- The dying tick gives a dropped weapon itself as owner and copies that owner onto every child.
- The weapon leaves its parent's list. The weapon and each descendant take the weapon as their root.
- The iteration count is 2 plus the parts visited, minus 1. The link count is 0 and the awake byte is 0.
- A grip at or above 0 becomes -1. A negative grip stays.
- A part that has a child, an object type whose low twelve bits are 0x89, and object bit 0x400000, sets its mark to -2.

## Capabilities

### New Capabilities

### Modified Capabilities

- `physical-body`: the owner a dropped weapon takes.

## Impact

- `exa_combat::collapse`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`, and `docs/formats/OTHER.md`.
- Evidence: the drop's owner store, the child-owner walk, the parent unlink, and the root walk that counts parts. The iteration base is the integer 2. No new corpus files.
- Out of scope: a free weapon body in the solver, the body-flag mask that keeps bits other than 0, 16 and 17, and injuries beyond damage.

## Phase

Phase 4, the first duel. The owner of a weapon dropped at a collapse.
