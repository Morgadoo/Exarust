# Proposal

## Why

The skill selector routes a key to the base handler while a positive selection is open on a page above one, or while the word is nine with its mode byte clear; otherwise it closes with 0x40. Exarust does not model that routing.

## What Changes

- A positive selection with a page above 1 goes to the base handler.
- A word of nine with a clear mode byte also goes to the base handler.
- Any other state closes and returns 0x40.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the skill-selector key.

## Impact

- `exa_ui::menu`, from `skill_selector_key`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 131-byte selector method's two guards and its 0x40 fallback. No new corpus files.
- Out of scope: the base handler and the close bodies.

## Phase

Phase 7, the interface. The key routing.
