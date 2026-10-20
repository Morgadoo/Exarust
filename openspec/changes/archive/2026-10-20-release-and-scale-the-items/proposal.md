# Proposal

## Why

The item container's release clears the low flag bit on every entry, and its scale divides each entry's position by the divisor and multiplies by the unit. Exarust does not model either.

## What Changes

- Release clears bit 0 of every entry's 0x14 flag word and runs the base once.
- Scale divides each x and y by the argument and multiplies by the unit, rounding to nearest even, then runs the base once.

## Capabilities

### New Capabilities

### Modified Capabilities

- `inventory-pane`: the container's release and scale.

## Impact

- `exa_items::pane`, from `release_items` and `scale_items`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 104- and 141-byte container methods. No new corpus files.
- Out of scope: the base methods.

## Phase

Phase 3, items and inventory. The container's release and scale.
