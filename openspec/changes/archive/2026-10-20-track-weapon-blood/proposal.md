# Proposal

## Why

The original's `TWeaponBlood` keeps one spot per source that has drawn blood on a weapon: the source, the amount it has left, and the two terms the stain needs from the weapon's length. Exarust has a single scalar of blade blood but not the original's list, so the combat section is still at 0% reimplemented.

## What Changes

- A hidden source adds nothing and returns 0.
- Any other source takes the first free spot or grows the list, stores its two lengths summed as the scale over 80 and the inverse square root of 80 over them, starts at amount 0 and tag 0, and returns its 1-based index.
- Clearing the count empties the list; clearing a spot drops its source, amount and scale; adding to the amount clamps 0 through 1.

## Capabilities

### New Capabilities

### Modified Capabilities

- `combat`: the weapon's blood list.

## Impact

- `exa_combat::weapon_blood`, from `WeaponBlood`.
- `docs/PLAN.md`, `docs/RENDERING.md` and `docs/COMPONENT-MAP.md`: `TWeaponBlood` becomes **53%** (453 of 853 bytes); the combat section becomes **14%** reimplemented. The headline stays 14.4% and 5.7%; the fresh sum stays about 15.5%.
- Evidence: `0x100100630` (count 0), `0x100100640` (the add: the `0x400000` hidden bit, the free-slot scan, the growth, the source pointer, `(0x14 + 0x34) × 0.0125` in double precision at `0x10030fb60`, `sqrt(80 / sum)` at `0x10030fb68`, amount 0 at `0x8` and tag 0 at `0x14`), `0x100100770` (clear source, amount and scale), `0x1001007c0` (amount plus delta clamped 0 through 1). No new corpus files.
- Out of scope: the 1,207-byte spatter draw (`0x100100820`) and the constructor (`0x100100530`).

## Phase

Phase 5, combat bodies. The weapon's blood list.
