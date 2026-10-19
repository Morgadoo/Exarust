# Proposal

## Why

An unarmed parry is chosen. An armed defender still does not set the grip that meets the blow.

## What Changes

- A cleave is refused.
- The falloff is 1 inside 80 cm, one half at 165 cm, and 0 at 250 cm and beyond.
- A weapon's strength is (1.4 − handling) times that falloff times 0.75 times 0.6. A passed blow squares the handling and disables the grips.
- A shield's speed is 1 minus half the weight times stamina, its flags are 0x800, and its strength is stamina times weight times the falloff times 0.75 times 0.4.
- Slot 1 is used only while attacking outside motions 8 to 10 with two weapons held.

## Capabilities

### New Capabilities

### Modified Capabilities

- `physical-body`: the armed parry grip.

## Impact

- `exa_combat::parry`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`, and `docs/formats/OTHER.md`.
- Evidence: the armed parry grip. No new corpus files.
- Out of scope: the predicted weapon point, shields as bodies, and injuries beyond damage.

## Phase

Phase 4, the first duel. The grip an armed parry enables.
