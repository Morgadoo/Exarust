# Proposal

## Why

Phase 4 shows a weapon card's damage rows. An apparel card does not yet show its rows.

## What Changes

- Coverage is always that byte times 0.05.
- Impact, slash, crush, pierce, and encumbrance are shown only when that byte times 0.05 is above 0.25.

## Capabilities

### New Capabilities

### Modified Capabilities

- `inventory-pane`: the apparel card's rows.

## Impact

- `exa_items::card`, beside the weapon rows.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`, and the `exa_items` README.
- Evidence: the apparel card's row compare. No new corpus files. Tests use synthetic bytes.
- Out of scope: the icon picture.
