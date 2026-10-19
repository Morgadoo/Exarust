# Proposal

## Why

Phase 7 can confirm a shop basket, but the copper totals are still supplied by the caller. A shop line already stores the price as a marked count, and the sale is a quarter of the basket.

## What Changes

- A shop line's low fourteen bits are a count. Both high marks price it at 1440 copper each, the higher mark alone at 144, the lower mark alone at 12, and no mark at 1.
- A buy basket is the sum of those prices. A sell basket is that sum divided by 4, dropping the remainder.
- A confirmed trade uses those two totals.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the price of a shop line and the quartered sale.

## Impact

- `exa_game::career`, the shop trade the company already confirms.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`, `crates/exa_game/README.md`.
- Evidence: the shop line's marked count, decoded at 1, 12, 144 or 1440 copper, summed for a buy and quartered as a basket for a sale. No format field changes, so the corpus step is a skip.
- Out of scope: packing a copper amount back into a word, the description card, emblems, a candidate's listed price, reward item identities, and which key fits a door.
