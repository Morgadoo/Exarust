# Proposal

## Why

The arena's six loadout tiers define the weapons opponents carry and the point budget of each tier. The table at `0x1002bb6a0` was not modelled; only the match templates were.

## What Changes

- The six tiers' weapon rows, read byte-for-byte from the binary: 8, 11, 10, 14, 4 and 4 entries.
- The multi-variant rows (Axe, War Hammer, Spiked Mace) and their non-zero variant selection.
- The point limits `5, 10, 15, 20, 30, 40`.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the arena's loadout tiers.

## Impact

- `exa_game::arena`, from `WeaponEntry`, `WEAPON_TIERS`, `weapon_tier`, `pick_variant` and `point_limit`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `XA_Arena` becomes **20%** in the source rating. The headline rises slightly.
- Evidence: the six 0x158-byte rows at `0x1002bb6a0` (names, variant ids and counts), the variant picker `0x100184250`, and the limit table `0x1002bcd30`. No new corpus files.
- Out of scope: the loadout cost calculation, the opponent spawn, the live board and the shop.

## Phase

Phase 7, arena. The loadout tiers.
