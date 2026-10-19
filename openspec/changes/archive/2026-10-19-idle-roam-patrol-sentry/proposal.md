# Proposal

## Why

Phase 6 hears a noise and then has no idle of its own. A roam walks out to a wander point and back toward home, a patrol repeats the wander, and a sentry holds the guard pose.

## What Changes

- A roam waits, walks to a wander point, waits 500 ms plus a draw of 2000 ms, walks to a home-side point, waits 1000 ms plus a draw of 4000 ms, and repeats.
- A patrol waits 500 ms plus a draw of 2000 ms between wander walks and does not walk home.
- A sentry holds the guard pose. A wait whose deadline equals the clock continues.
- During a wait, a disturbance above 0.3 is walked to and the phase does not advance.

## Capabilities

### New Capabilities

- `npc-idle`: roam, patrol and sentry.

### Modified Capabilities

## Impact

- `exa_ai::idle`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the roam, patrol and sentry ticks. No format field changes, so the corpus step is a skip.
- Out of scope: follow, sleep, the wander-point picker, and the statue-mode scale.
