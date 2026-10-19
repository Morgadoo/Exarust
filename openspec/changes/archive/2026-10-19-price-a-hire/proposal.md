# Proposal

## Why

Phase 7 can put a fighter on the roster. A recruitment does not yet cost the 12 copper the night's purse is checked against.

## What Changes

- A hire costs 12 copper. Fewer than 12 refuses the hire and leaves the copper and the roster unchanged.
- A full combatant roster or a second specialist refuses without spending.
- A hire that joins subtracts 12 and adds 12 to the amount spent.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the recruitment price.

## Impact

- `exa_game::career`, the hire the roster already uses.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the recruitment that refuses under 12 copper and then spends that amount. No format field changes, so the corpus step is a skip.
- Out of scope: item prices, emblems, and the reward item identities. A candidate's own listed price is not this flat recruitment.
