# Proposal

## Why

The main menu's forward action reads the stored page: page 1 runs action 0, page 8 runs action 1, each plays a sound when the input is ready, and both enter the session. Any other page does nothing. Exarust routes the forward key but does not model the page action.

## What Changes

- Page 1 returns action 0 with the session entry.
- Page 8 returns action 1 with the session entry.
- Either sounds only when the input is ready.
- Any other page returns no action, no sound and no session entry.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the main menu's forward action.

## Impact

- `exa_ui::menu`, from `forward_main_menu`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `TMainMenu` is not rated separately in the largest-components table, so no published percentage moves. The headline stays 14.4% and 5.7%; the fresh sum stays about 15.5%.
- Evidence: the forward action: the page against 1 and 8, the input-readiness global, the sound call, the page action with 0 or 1, and the session entry with 1. No new corpus files.
- Out of scope: the menu's draw, constructor and the global back router.

## Phase

Phase 4, UI. The main menu's forward action.
