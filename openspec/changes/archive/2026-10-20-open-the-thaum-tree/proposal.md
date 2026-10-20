# Proposal

## Why

The power tree's close clears a global, runs its reset and base and clears a byte; its show drives the focus, refreshes on a flag, sets a bit, stamps the clock and stores a value. Exarust does not model either.

## What Changes

- Close clears the global qword, runs the inner reset once, clears the byte and runs the base once.
- Show calls the focus with `(flags & 7) | 0x800` when the global count is positive.
- It refreshes once when the pane's 0x60 word carries 0x20, sets bit 2 at 0x21d, stamps 0x224 with the clock plus the offset and stores 0x1bc globally.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the power tree's close and show.

## Impact

- `exa_game::thaum`, from `close_tree` and `show_tree`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 59- and 121-byte methods. No new corpus files.
- Out of scope: the focus and refresh bodies.

## Phase

Phase 5, thaumaturgy. The tree's close and show.
