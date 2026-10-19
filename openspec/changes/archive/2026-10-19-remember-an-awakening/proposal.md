# Proposal

## Why

Phase 6 tells witnesses when a knock-out drops its dormant flag. That tick does not yet store a memory word for each person who can see.

## What Changes

- On the tick that drops the dormant flag, each person who can see has their memory word updated. A person who cannot see, and a non-person, stay. A later tick leaves those words.
- A short count adds one and keeps counting. The hit that meets the need stamps the seconds plus the delay and sets the high bit.
- A last use inside its cooldown, one still ahead of now, and a never-again nibble stay. An activated stamp inside its window refreshes to now. One at the window re-arms with the delay. A future stamp reports updated and stays. The held-off flag leaves an activated word.

## Capabilities

### New Capabilities

### Modified Capabilities

- `npc-idle`: the memory word written when a knock-out is noticed.

## Impact

- `exa_ai::idle`, on the wake tick already shipped.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the memory-word update and its cooldown and delay tables. No format field changes, so the corpus step is a skip.
- Out of scope: the role-by-role dialogue walk, the grip pass, the arm-mass scale, and steering along an accepted path.
