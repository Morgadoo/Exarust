# Proposal

## Why

The gate hook returns before it looks at the cursor, and the click and hover hooks are registered and never run.

## What Changes

- A clear mid byte counts a pick when the world is open and the hold byte is clear. A taken item publishes a follow hook and ORs the flags with 5. An actor whose parent is the world counts an open. Kind 0xa00 counts a chest.
- The click hook copies a fast aim, clears the aimed word on a miss with the hold byte, and sets flags 0x11 on a taken item.
- The hover hook stores flags 0x15 and clears its byte and its mark.
- A focus begin still counts one call before the hand words. The focus method body is not in.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the cursor pick, the click hook, and the hover hook.

## Impact

- `exa_game::menu`, from `MatchGate::run_gate`, `MatchGate::run_click`, and `MatchGate::run_hover`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the gate hook's pick chain after the mid-byte return, the click hook's aim copy and item flag, and the short hover body that follows the gate hook. No new corpus files.
- Out of scope: the pick raycast, the rest of the click path, and the focus method body.

## Phase

Phase 7, arena career. These three hook prefixes, not every registered method.
