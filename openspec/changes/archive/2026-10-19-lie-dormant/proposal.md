# Proposal

## Why

Phase 6 can follow a leader. Sleep does not yet lie dormant: the dormant condition and flag, tone 0, a statue, and gait 2, with a tick that changes nothing.

## What Changes

- Sleep sets the condition to dormant and sets the dormant flag without clearing the others.
- Tone and the tone target become 0. The body is a statue and its gait is 2.
- The tick installed afterwards changes nothing. A second sleep keeps the flag and the dormant condition.

## Capabilities

### New Capabilities

### Modified Capabilities

- `npc-idle`: sleeping.

## Impact

- `exa_ai::idle`, writing the condition, the state flags, and the body's tone.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the idle sleep step. No format field changes, so the corpus step is a skip.
- Out of scope: waking when a target is seen, and the statue sleep countdown.
