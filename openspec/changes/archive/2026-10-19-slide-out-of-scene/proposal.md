# Proposal

## Why

Phase 6 aims 100 cm ahead while a body is in the scene. Out of the scene that step does not yet slide the body 2 cm along the path.

## What Changes

- When the low two scene bits are clear, a step advances 2 cm along the current segment. The standing height adds the pelvis base. The ground point keeps the path height.
- A length that is not strictly past the fraction times the length plus 2 crosses one segment. The new fraction is (1 − (1 − fraction) · length) · (1 − fraction) − 2, divided by the next length. No horizontal length counts as 0.1.
- A set low bit does not slide.

## Capabilities

### New Capabilities

### Modified Capabilities

- `npc-steering`: the 2 cm slide used when a body is out of the scene.

## Impact

- `exa_ai::path`, beside the in-scene carrot already shipped.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the out-of-scene branch of path following. No format field changes, so the corpus step is a skip.
- Out of scope: the run bit, the soft stop, and the line walk.
