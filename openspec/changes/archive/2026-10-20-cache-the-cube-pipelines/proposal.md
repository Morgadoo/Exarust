# Proposal

## Why

`prepare_draws` asks Bevy's mesh specializer for a pipeline every frame. While a pipeline is still queued, a repeated request hits Bevy 0.20's debug descriptor check, which panics because the id is not yet in the pipeline cache. A local cache removes the repeated request and the panic, and avoids the work.

## What Changes

- `PreparedCubes` keeps a map from `(mesh id, key)` to the specialized pipeline id.
- A hit reuses the id without calling the specializer; a miss specializes once and stores the id.
- The dev profile then runs the level without the pipeline-cache debug panic.

## Capabilities

### New Capabilities

### Modified Capabilities

- `rendering`: the cube-shadow pipeline cache.

## Impact

- `exa_render::cube_shadows`, in `prepare_draws`.
- `docs/PLAN.md`; no coverage change.
- Evidence: the 826-byte reproducer is the dev-profile launch, which panicked at `bevy_render` `pipeline_cache.rs:323` from `cube_shadows.rs:651` with `len 53, index 54`, while the `play` profile rendered correctly. No new corpus files.
- Out of scope: Bevy's internal specializer.

## Phase

Rendering. The cube-shadow pipeline cache.
