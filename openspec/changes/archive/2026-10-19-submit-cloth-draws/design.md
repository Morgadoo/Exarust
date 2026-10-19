# Design

## Context

The cloth lists two cull passes. The material pipeline culls nothing when a surface is two-sided, and otherwise keeps the default. Nothing connects the pass list to a pipeline.

## Goals / Non-Goals

**Goals:**

- Submit one material per pass, with the pipeline culling the face the pass names.

**Non-Goals:**

- Spawning an entity for every cloth marker.
- A separate cloth render node.
- Body-to-body contact and voxelising.

## Decisions

1. The physics cloth maps its passes onto the renderer's front and back faces.
2. The material stores that face on its pipeline key. When the key names a face, the pipeline culls it. Otherwise a two-sided surface still culls nothing.
3. The world crate, which already depends on both, builds the materials from the cloth.

## Risks / Trade-offs

- The original also has a third hook that draws once with culling off. This slice keeps the single-sided cloth as a back cull, which draws the front.

## Migration Plan

No data migration.

## Open Questions

None for this slice.
