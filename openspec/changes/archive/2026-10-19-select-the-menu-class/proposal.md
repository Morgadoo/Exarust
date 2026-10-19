# Proposal

## Why

The class-select screen stores a class index and, when the index and the sound flag are both positive, plays its selection sound. It returns 3. Exarust does not model it.

## What Changes

- The index is stored at field 0x6c.
- A positive index with a positive sound flag plays the selection sound.
- The method returns 3.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the class-select index.

## Impact

- `exa_ui::menu`, from `select_menu_class`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 88-byte class-select method's store, guard and return. No new corpus files.
- Out of scope: the sound call's body.

## Phase

Phase 7, the interface. The index and the guard.
