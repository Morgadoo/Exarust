# Proposal

## Why

Phase 6 accepts a searched path when it is long enough. It does not yet pick the point 100 cm ahead of where the body stands on that path.

## What Changes

- A ground point projects onto the nearest segment. The fraction stays unclamped, and the goal is 100 cm further along, measured from that foot.
- An equal distance keeps the earlier segment. A vertical segment is skipped. Exactly 100 cm left spills, and a zero step has no goal.

## Capabilities

### New Capabilities

### Modified Capabilities

- `npc-steering`: the in-scene point 100 cm ahead on an accepted path.

## Impact

- `exa_ai::path`, beside the accept rule already shipped.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the in-scene projection and the 100 cm step. No format field changes, so the corpus step is a skip.
- Out of scope: the out-of-scene slide, the run bit, and the line walk.
