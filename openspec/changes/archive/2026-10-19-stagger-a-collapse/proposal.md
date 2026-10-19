# Proposal

## Why

A collapsing body still squats on its feet. The dying tick never starts the stagger step that moves the chest.

## What Changes

- A body that is already stepping takes no stagger.
- A balance that widens above 0.4 scales the reach by the balance. A lower balance scales the widened reach by 0.4. A float balance of 0.4 takes the first path.
- The span squares the tone plus one half and multiplies by that length and the stored square root of the scale.
- The target adds the velocity's reciprocal-square-root unit times the span to the chest. A zero velocity leaves the chest.
- Landings and the step kind are 0.

## Capabilities

### New Capabilities

### Modified Capabilities

- `physical-body`: the collapse's stagger step.

## Impact

- `exa_combat::collapse`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`, and `docs/formats/OTHER.md`.
- Evidence: the dying tick's stagger step. No new corpus files.
- Out of scope: the footwork that walks the target, the dropped weapon, and injuries beyond damage.

## Phase

Phase 4, the first duel. The collapse's stagger step.
