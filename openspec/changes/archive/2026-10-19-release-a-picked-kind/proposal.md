# Proposal

## Why

A kind mark counted a call and returned. The two calls after that hit, and the view test in front of the pick, were not run.

## What Changes

- A cursor below 0, or at the view width or height, clears the hit and does not count a kind call. A cursor inside keeps a supplied hit.
- A marked item releases a held count above 0, or clears a pointer when the count is not above 0. Byte 1 counts a close.
- The first pane tagged 0xce1c2013 for that item is shown. With no pane, a marked span counts a build.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the view test, the held release, and the pane open after a kind mark.

## Impact

- `exa_game::menu`, from `MatchGate::run_gate`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the view compares, the held-count branch, and the pane walk that matches the tag and the item. No new corpus files.
- Out of scope: the scene walk inside the view, and the pane builder.

## Phase

Phase 7, arena career. This prefix, not the scene walk.
