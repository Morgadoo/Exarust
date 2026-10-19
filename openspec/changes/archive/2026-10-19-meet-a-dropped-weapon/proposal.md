# Proposal

## Why

A dropped weapon is a free body, and the body step never sees it against another body.

## What Changes

- One shipped body step records a contact when a dropped weapon's sphere starts on another body's sphere.
- The contact names both bodies and has one hit.
- The weapon leaves the place one step alone would leave it.

## Capabilities

### New Capabilities

### Modified Capabilities

- `physical-body`: a dropped weapon meeting another body.

## Impact

- The dying tick's free body, stepped by `step_bodies`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`, and `docs/formats/OTHER.md`.
- Evidence: body–body detection already pushes overlapping spheres and keeps the touch. No new corpus files.
- Out of scope: the flag mask that keeps bits other than 0, 16 and 17, and injuries beyond damage.

## Phase

Phase 4, the first duel. A dropped weapon meeting another body.
