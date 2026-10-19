# Proposal

## Why

Phase 7 can recruit for a flat 12 copper. A candidate on the hire list has a separate price: a square of the rank, plus four fifths of the gear.

## What Changes

- The listed price is the square of one more than half of six skill levels, and that half stops at 5, times 12, plus four fifths of the gear rounded to the nearest copper.
- The gear is the sum of the shop line prices. No skills and no gear cost 12.
- A purse below the price refuses before a full roster is considered. A full roster or a second specialist refuses without spending.
- A hire that joins subtracts the price and adds it to the amount spent.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the price shown for a candidate.

## Impact

- `exa_game::career`, beside the flat recruitment.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the hire panel writes the coin dword from the rank square and four fifths of the summed line prices, and the hire spends that dword. No format field changes, so the corpus step is a skip.
- Out of scope: the flat recruitment of 12, and the other emblem controls.
