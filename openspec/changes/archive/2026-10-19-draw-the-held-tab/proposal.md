# Proposal

## Why

A held form draws a bar, and the tab that follows that bar is still missing.

## What Changes

- Bit 0x200 on that held form draws one tab.
- A grip of 10, 20 at unit 5 sits from 16, 30 to 26, 40.
- Mask bits 1 and 4 together raise the tab by one more unit.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the tab that follows the held bar.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the tab after the held bar. No new corpus files.
- Out of scope: the next anchor after that tab.

## Phase

Phase 7, the interface. The held tab, not the next anchor.
