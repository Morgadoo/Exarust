# Proposal

## Why

The inventory's key override closes on the same 0x2002 mask as the journal and loot, but through a stored callback when one exists and through the form close otherwise. Exarust does not model the callback path.

## What Changes

- A mask with both 0x2000 and 0x2 closes and returns 1.
- The close uses the stored callback with argument 0 when it exists, and the form close otherwise.
- Any other mask returns the base handler's result.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the inventory close key.

## Impact

- `exa_ui::menu`, from `inventory_key_close`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 116-byte inventory override's callback pointer at 0x188 and its argument at 0x190. No new corpus files.
- Out of scope: the callback and the form close bodies.

## Phase

Phase 7, the interface. The close path.
