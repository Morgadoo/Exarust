# Design

## Context

REA on the grid builder (see `docs/formats/RFC.md`, "Grid building") gave the
full rule set; this change re-expresses it in `exa_world::grid`.

## Goals / Non-Goals

**Goals:** faithful piece selection and placement; levels recognisable.
**Non-Goals:** per-room mesh merging, the alternate builder, collision.

## Decisions

- Pieces are spawned as entities per placement (simple, debuggable); merging
  into per-room meshes is a later performance change.
- Ceilings are off by default in the viewer (overhead camera) and on with
  `--ceilings`; the original toggles them with an engine option whose source
  is not yet identified.
- Special-tile masks and floor flags are data tables transcribed from the
  executable.

## Risks / Trade-offs

- Entity counts (thousands of pieces) are high for software rendering; real
  GPUs handle them, and merging is planned.
