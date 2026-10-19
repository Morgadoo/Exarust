# Proposal

## Why

The main menu's key handler routes four masks: confirm plays a sound and calls the confirm virtual, forward and back call their helpers, and the two direction nibbles move the selection within the limit. Exarust does not model it.

## What Changes

- Mask 0x100 confirms when the selection is positive, with the sound.
- Masks 0x200 and 0x400 call the forward and back helpers.
- Direction 0x8000 moves down to the next entry, clamped to at least 1 and below the limit.
- Direction 0x4000 moves up when the selection is above 1.
- The helper's result is returned, or 0 when nothing runs.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the main-menu key.

## Impact

- `exa_ui::menu`, from `main_menu_key`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 328-byte method's four mask arms and two direction guards. No new corpus files.
- Out of scope: the confirm, forward and back helper bodies.

## Phase

Phase 7, the interface. The key routing.
