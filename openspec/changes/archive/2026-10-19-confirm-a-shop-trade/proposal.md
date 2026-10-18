# Proposal

## Why

Phase 7's arena shop does not yet accept or refuse a trade. The company and its copper are already in.

## What Changes

- Two empty baskets are refused.
- A buy whose price exceeds the sell plus the company's copper is refused.
- Otherwise the copper changes by the difference, and that difference is recorded as spent or earned.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the shop confirm.

## Impact

- `exa_game::career::Company::trade`.
- `docs/PLAN.md`.
- Evidence: the trade-window note (confirm disabled when money is short or the baskets are empty, balance is pay or receive). No format field changes, so the corpus step is a skip.
- Out of scope: item prices, the shop stock, and the window.
