# Proposal

## Why

Phase 6 picks a blow once a fight has started. An idle character still does not answer a noise or an alert: a disturbance above 0.3 is walked to, and an alert of 0.2 or more faces a target more than 45° off or looks around after 0.7 to 3 seconds.

## What Changes

- A disturbance above 0.3 names the point to walk to, ahead of the alert.
- An alert of 0.2 or more faces a known target more than 45° off.
- Otherwise that alert looks around once the gap, clamped to 0.7–3 seconds, has elapsed.
- A quiet, unalert character keeps its own activity.

## Capabilities

### New Capabilities

- `npc-hearing`: the idle's answer to a noise and an alert.

### Modified Capabilities

## Impact

- `exa_ai::hear`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the idle check (disturbance above 0.3, then alert at 0.2). No format field changes, so the corpus step is a skip.
- Out of scope: sampling the grid cell, the heading that look-around picks, and the idle activities themselves.
