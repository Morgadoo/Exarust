# Proposal

## Why

Phase 6 can ease a wandering look. The aim snap that follows a span choice does not yet copy the body's place, take its heading's yaw, or ask for the idle stance.

## What Changes

- An aim snap copies the body's place.
- The heading, the stored heading and the base heading become the yaw of the body's heading. The heading's height is ignored.
- The roll steps. The timer is that step's high part times 3000, plus the clock, plus 1000.
- A pace word and the body's marker are cleared.
- The stance becomes idle. The old stance's clip starts Idle over 30 frames. Another clip stays. Idle already playing does not restart.

## Capabilities

### New Capabilities

### Modified Capabilities

- `npc-steering`: the aim snap on a path.

## Impact

- `exa_ai::path`, calling the stance helper already shipped.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the aim snap the path step runs after a span choice. No format field changes, so the corpus step is a skip.
- Out of scope: the other path phases.
