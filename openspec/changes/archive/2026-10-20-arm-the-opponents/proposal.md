# Proposal

## Why

Opponent spawn still lacked the two-stage weapon pick and the level base. The decode gives both: a tier's weapon row is drawn with the LCG, then one non-zero variant; the level is 60000 + rand(9999) for a company and 50000 + rand(9999) otherwise.

## What Changes

- `pick_weapon_row(tier, rng)`, 1-based, and `pick_weapon_variant(tier, row, rng)` with the 0-based row `set_weapon` receives.
- `spawn_level(company, rng)` with the two bases.
- Tests against seed 12345: tier 1 row 5, tier 2 row 1 variant 419, and the two level bases.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the opponent's weapon and level.

## Impact

- `exa_game::arena`, from `pick_weapon_row`, `pick_weapon_variant` and `spawn_level`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `XA_Arena` becomes **30%** in the source rating. The headline rises slightly.
- Evidence: `0x100183f80` (the weapon row), `0x100184250` (the variant), `0x100185ef0`/`0x100185d00` (the company/generic levels). No new corpus files.
- Out of scope: the actor/body creation, condition and equipment costs.

## Phase

Phase 7, arena. The opponent's weapon and level.
