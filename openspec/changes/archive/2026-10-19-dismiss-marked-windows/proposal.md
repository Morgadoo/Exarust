# Proposal

## Why

Beginning an arena match dismisses marked children on two lists before anyone leaves the company. That walk is not in the menu command yet.

## What Changes

- Walk each list from the last child back to the first.
- Dismiss a child when its flags, masked by 0xce100000, equal that mask.
- A count of 0 or less dismisses nobody. A child past the count stays.
- Run the walk before the specialist leave. A command that does not begin dismisses nobody.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the window walk at the start of an arena match.

## Impact

- `exa_game::menu::MatchGate::begin_match`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the two list walks and the flag mask. No new corpus files.
- Out of scope: the method that runs on a dismissed child, and the call after the two lists.

## Phase

Phase 7, arena career. The dismiss walk, not the window class.
