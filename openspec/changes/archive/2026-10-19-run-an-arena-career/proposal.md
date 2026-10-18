# Proposal

## Why

Phase 7 (M7 "arena season") has no company, roster or match. `docs/PLAN.md` lists arena career as still open.

## What Changes

- A new company starts with 343 copper, shown as gold, silver and copper at 12 to 1.
- It hires at most seven combatants and one merchant, physician or trainer. The manager holds the eighth fighter's place.
- Entering a standard-board match takes its fee. A win pays the prize. Experience is the template amount times the share of opponent health dealt, plus half when a trainer is hired.

## Capabilities

### New Capabilities

- `arena-career`: the company, the roster and a match on the standard board.

### Modified Capabilities

## Impact

- `exa_game::career`.
- `docs/PLAN.md`.
- Evidence: the arena career notes already taken from the original (money seed, coin rate, roster cap, the six standard matches, fee, prize, experience at tier 0). No format field changes, so the corpus step is a skip.
- Out of scope: the hire price, tier scaling, the shop, wages, seasons, emblems and the match UI.
