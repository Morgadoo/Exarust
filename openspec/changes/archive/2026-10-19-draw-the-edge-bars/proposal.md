# Proposal

## Why

The dispatcher draws the title and four corners, and the edge bars that follow those corners are still missing.

## What Changes

- The same style that draws the corners draws edge bars after them.
- With no title, unit 5 on a form of 100 by 80 draws a top bar from 33, 0 to 67, 10.
- A title of width 40 splits that bar at gap 17 and adds two caps.
- A style that is not 0x10, and bit 0x400000, draw no edges.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the edge bars that follow the title and the corner squares.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the edge bars after the four corners. No new corpus files.
- Out of scope: the drawing that follows those bars.

## Phase

Phase 7, the interface. The edge bars, not the drawing after them.
