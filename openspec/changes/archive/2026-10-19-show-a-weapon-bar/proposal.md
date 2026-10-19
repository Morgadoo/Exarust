# Proposal

## Why

Phase 4 chooses an item card's page. A weapon card does not yet show its damage rows.

## What Changes

- Slash, crush, pierce, and thrust are shown only when that byte is above 5.
- A shown bar is that byte times 0.05.

## Capabilities

### New Capabilities

### Modified Capabilities

- `inventory-pane`: the weapon card's damage rows.

## Impact

- `exa_items::card`, beside the page choice.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`, and the `exa_items` README.
- Evidence: the weapon card's row compare. No new corpus files. Tests use synthetic bytes.
- Out of scope: the icon picture and the apparel rows.
