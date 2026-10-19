# Proposal

## Why

Phase 4 places icons in an inventory pane. The pane does not yet refuse a marked flag, a missing required mark, or an unlisted id, and a limit of 0 still refuses.

## What Changes

- A shared bit with the reject mask refuses the drop.
- The item's flags masked by the required mask must equal the required value.
- An own-only pane accepts only an id already listed with the low bit set.
- A per-item limit of 0 does not refuse for size. A capacity of 0 does not refuse for bulk.

## Capabilities

### New Capabilities

### Modified Capabilities

- `inventory-pane`: the flag and ownership checks on a drop.

## Impact

- `exa_items::pane`, on the drop the pane already ships.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the container accept test. No new corpus files. Tests use synthetic icons.
- Out of scope: the icon picture and the description card.
