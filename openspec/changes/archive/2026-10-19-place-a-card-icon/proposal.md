# Proposal

## Why

Phase 4 shows an item card's page and its stat rows. The card does not yet place the icon picture.

## What Changes

- A plain card centres the icon in the span, and places it from the content plus four times the unit.
- A stats card places the icon from the unit. A picture wider than eleven units shifts that place.

## Capabilities

### New Capabilities

### Modified Capabilities

- `inventory-pane`: where the card draws its icon.

## Impact

- `exa_items::card`, beside the apparel rows.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`, and the `exa_items` README.
- Evidence: the card's icon place. The icon size starts at 160 and the unit at 4. No new corpus files. Tests use synthetic numbers.
- Out of scope: description phrases.
