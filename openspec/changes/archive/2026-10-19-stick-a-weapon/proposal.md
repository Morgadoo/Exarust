# Proposal

## Why

A blow already computes how far a weapon bites. The link that sticks the weapon in the part is not in yet.

## What Changes

- An armed bite above 0.1 sticks. An unarmed bite, and a depth of 0.1, sticks nothing.
- The depth is capped at 1. Flags are 0x11000 or 0x211000 around the widened 0.4. Strength is 12 times the depth. Smoothing adds 0.5.
- The count is 3 times the truncated product of 83, the unit and the depth. The span is twice that product. A depth of 3 matches a depth of 1.
- The weapon layer becomes 0xC0. The struck part, its parent and each child gain bit 0x40.

## Capabilities

### New Capabilities

### Modified Capabilities

- `physical-body`: the weapon stick link.

## Impact

- `exa_combat::bite`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`, and `docs/formats/OTHER.md`.
- Evidence: the bite link after an armed hit. No new corpus files. Tests use synthetic numbers.
- Out of scope: the link solver holding the weapon, injuries beyond damage, and practice mode.

## Phase

Phase 4, the first duel. The grab a weapon makes by sticking.
