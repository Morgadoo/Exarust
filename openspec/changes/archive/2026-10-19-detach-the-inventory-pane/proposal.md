# Proposal

## Why

The inventory pane has a detach routine that marks its owner, places a visible child, releases or frees its stack, copies the pane globals and runs the base method. A second override maps an argument of −1 to page −3. Exarust does not model either.

## What Changes

- An already inactive pane returns true without doing anything.
- Otherwise the owner's word gains 0x20000000, and a visible child is placed at the pane's place.
- A stack above one is decremented and its top cleared; a stack of one is freed and cleared.
- The loot pane copies its origin to the globals.
- The base method runs and its result returns.
- Argument −1 stores page −3.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the inventory pane's detach and page sentinel.

## Impact

- `exa_ui::menu`, from `detach_inventory` and `inventory_page`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 259-byte detach and 41-byte page methods. No new corpus files.
- Out of scope: the base method and the child placement.

## Phase

Phase 7, the interface. The pane lifecycle.
