# Proposal

## Why

The class-select navigation confirms, forwards or backs, and moves the selection to the next or previous available class using a bitmask. Exarust does not model it.

## What Changes

- Mask 0x100 confirms a positive selection with result 7 and the class form; a zero selection returns 0.
- Masks 0x200 and 0x400 call the forward helper and return 0.
- Direction 0x8000 moves down to the first available class at or after the selection, up to class 3.
- Direction 0x4000 moves up from selection − 2, or to 1 when the selection is at most 2.
- The move runs through the selection virtual only when the target differs.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the class-select navigation.

## Impact

- `exa_ui::menu`, from `navigate_class`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 437-byte navigation method's confirm, calls and two bit scans. No new corpus files.
- Out of scope: the forward call and the selection virtual.

## Phase

Phase 7, the interface. The class navigation.
