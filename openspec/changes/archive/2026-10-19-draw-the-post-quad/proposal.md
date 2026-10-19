# Proposal

## Why

A held form draws a lower tab, and the quad that follows that tab is still missing.

## What Changes

- The same bit draws one quad from the grip and a post.
- A grip of 10 and a post of 40 at unit 5 runs from 11, 35 to 31, 55.
- A clear bit 0x200 and a clear hold bit draw none.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the quad that follows the lower tab.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the anchor after the lower tab. No new corpus files.
- Out of scope: the compare after that quad.

## Phase

Phase 7, the interface. The quad, not the compare after it.
