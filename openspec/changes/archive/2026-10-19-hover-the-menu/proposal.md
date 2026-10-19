# Proposal

## Why

The main menu resolves its hover, compares it to the selected entry, and on a change plays a sound at most once per 200 clock units and returns 3. Exarust does not model it.

## What Changes

- The hover becomes the stored selection when they match, else zero.
- An unchanged selection returns 0 and does nothing else.
- A change stores it and returns 3, playing a sound only when the entry is non-zero and the last sound is over 200 units old.
- The clock stamp updates only when the sound is played.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the main-menu hover.

## Impact

- `exa_ui::menu`, from `update_menu_hover`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 199-byte menu method's selection compare, 0xc8 threshold and 3 return. No new corpus files.
- Out of scope: the hover virtual's body and the sound.

## Phase

Phase 7, the interface. The hover state.
