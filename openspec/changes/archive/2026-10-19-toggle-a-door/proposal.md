# Proposal

## Why

Phase 2 opens a door when a signalled wait ends. A toggle from the AI or a script does not yet choose open or close.

## What Changes

- A toggle opens a latched door first. A hinge's openness is its angle minus closed, times −0.7. Above 0.7 the aim fraction is 1, otherwise 0, including exactly 0.7. The hinge drives that aim for 100 ticks, or 60 when fast, and wakes for 256 ticks.
- A slider's openness is 1 minus its travel. Above 0.7 it closes for 60 ticks. At or below 0.7 it opens for 60 ticks and clears the shut flag. A latched plain slider jumps to 0.6 before that choice.

## Capabilities

### New Capabilities

### Modified Capabilities

- `object-physics`: the door toggle.

## Impact

- `exa_physics::door`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the toggle command and its openness. No format field changes, so the corpus step is a skip.
- Out of scope: an AI choosing to toggle because a path is blocked.
