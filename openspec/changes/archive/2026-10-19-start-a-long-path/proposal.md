# Proposal

## Why

Phase 6 can ask for a path when a follow replans. It does not yet follow that path only when the ends are more than 150 cm apart, or when the search was forced.

## What Changes

- Fewer than two points is never followed, even when forced.
- Two points are followed when forced.
- Otherwise they are followed only when the first and last are more than 150 cm apart. Exactly 150 cm is not.

## Capabilities

### New Capabilities

### Modified Capabilities

- `npc-steering`: whether a searched path is followed.

## Impact

- `exa_ai::path`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the planner's accept rule. No format field changes, so the corpus step is a skip.
- Out of scope: the search itself, which is already shipped, and steering along the points.
