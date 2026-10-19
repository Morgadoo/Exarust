# Proposal

## Why

Phase 4 shows a weapon card's rows and an apparel card's rows. A shield card does not yet show its rows.

## What Changes

- Weight is that byte times 0.01. Impact is that byte times 0.05.
- Encumbrance is that byte times 0.05, and it is absent when the byte is 0.
- When the flags share a bit with 0x06000000, weight is 1 and encumbrance is 8.

## Capabilities

### New Capabilities

### Modified Capabilities

- `inventory-pane`: the shield card's rows.

## Impact

- `exa_items::card`, beside the apparel rows.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`, and the `exa_items` README.
- Evidence: the shield card's three bars. No new corpus files. Tests use synthetic bytes.
- Out of scope: rank, points, and description phrases.
