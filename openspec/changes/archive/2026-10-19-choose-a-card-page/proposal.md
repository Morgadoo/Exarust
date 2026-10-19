# Proposal

## Why

Phase 4 places icons in an inventory pane. The description card does not yet choose a page or print its coin value.

## What Changes

- An id below 1, a container, a missing model, and a model mark of 0 have no card.
- A map is a map and has no stats page.
- The coin word is divided by 4, unless the caller sets the high bit.
- A weapon that is not withheld is the weapon page. An armour type from 1 through 6 is the apparel page. A shield that is not withheld is the shield page. Anything else is the plain page.

## Capabilities

### New Capabilities

### Modified Capabilities

- `inventory-pane`: the item card's page and printed value.

## Impact

- `exa_items::card`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`, and the `exa_items` README.
- Evidence: the item-card page choice. No new corpus files. Tests use synthetic items.
- Out of scope: the icon picture and the stat bars.
