# Proposal

## Why

`RF_Dynamics_Volumes` (14.6 KB) voxelises a rigid body's mesh for its mass, centre of mass and colliders. Its summary and its float-keyed sort are exact and unimplemented.

## What Changes

- The volume summary visits the padded count grid, sums each filled cell's centre `(i, j, k) − 0.5`, and returns `(sum/count/dim − 0.5)·scale` with mass `count·Π(2·scale/dim)`.
- An empty volume returns the centre `(0, 0, 0)` and the fallback mass 0.1.
- The float-keyed quicksort uses a middle pivot, values below it from the left and above it from the right.

## Capabilities

### New Capabilities

### Modified Capabilities

- `object-physics`: the dynamics volume's summary and sort.

## Impact

- `exa_physics::volume`, from `VolumeSummary`, `EMPTY_MASS`, `sort_by_key` and `summarise`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `RF_Dynamics_Volumes` becomes **5%** in the source rating. The headline rises slightly.
- Evidence: the volume weigh (the padded grid walk, the centre subtraction, the 1.0, 2.0 and 0.5 constants and the 0.1 fallback) and its quicksort. No new corpus files.
- Out of scope: grid construction, solid marking, corner masks, sphere extraction and the TVoxelMap object.

## Phase

Phase 7, physics. The dynamics volume's summary.
