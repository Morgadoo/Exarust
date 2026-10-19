# Proposal

## Why

Phase 7 pays a match and hires a roster, then the day never ends. Ending a day pays wages, a day whose number mod 7 is 1 resolves the week's six events, a multiple of 7 drops a badly injured entry, and four won weeks grant the season.

## What Changes

- A merchant's wage is 2, a physician's is 12, a trainer's is the sum of six skill levels, and a combatant's is half that sum, at most 5, plus 1.
- Ending the day adds 1 to the day, which starts at 0, and subtracts the wage bill.
- Three or more of the six events, with no tie, take the week. The company's id is 1.
- Health under 3276 leaves a tournament entry on a day that is a multiple of 7.
- Winning all four weeks grants four season rewards.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: wages, the day calendar, and a four-week season.

## Impact

- `exa_game::career`. The session file still does not store the day.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the wage routine, the end-of-day tick, and the tournament verdict. No format field changes, so the corpus step is a skip.
- Out of scope: the hire price, how a tier scales a fee, overnight healing, and the reward item identities.
