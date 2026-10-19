# Proposal

## Why

A collapse can name a stagger target. The feet still stay where they are.

## What Changes

- The stance origin is the stagger target, pulled back to within 1.1 step lengths of the frame, at the frame's height.
- Each stance spot moves with that origin.
- One footwork pass anchors one foot on its shifted spot and leaves the other foot.
- A target inside that length is not pulled back. A target on the frame moves no foot.

## Capabilities

### New Capabilities

### Modified Capabilities

- `physical-body`: the footwork that walks a collapse stagger target.

## Impact

- `exa_physics::body::Ragdoll::walk_stagger`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`, and `docs/formats/OTHER.md`.
- Evidence: the footwork stance origin and the one-foot step. No new corpus files.
- Out of scope: the dying tick calling the walk, the dropped weapon, and injuries beyond damage.

## Phase

Phase 4, the first duel. The footwork that walks a stagger target.
