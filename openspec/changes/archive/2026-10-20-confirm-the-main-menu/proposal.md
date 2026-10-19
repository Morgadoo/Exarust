# Proposal

## Why

The main menu's confirm copies the selected entry to field 0x204 and, on the 0x1000 direction, stores and sounds the choice. Its select override stores an index and sounds it at most once per 200 clock units. Exarust does not model either.

## What Changes

- A zero or negative confirm selection copies field 0x70 to 0x204 and returns 0x40.
- A positive selection with direction 0x1000 stores itself at 0x70, copies it, plays a sound and returns 7; another direction does not copy and returns 0x40.
- The select stores its index, plays a sound when the index is positive, the sound is ready and the stamp is over 200 units old, updates the stamp only then, and returns 3.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the main-menu confirm and select.

## Impact

- `exa_ui::menu`, from `confirm_main_menu` and `select_main_menu`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 152- and 153-byte menu methods. No new corpus files.
- Out of scope: the sound call.

## Phase

Phase 7, the interface. The confirm and select.
