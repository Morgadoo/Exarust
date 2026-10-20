# Proposal

## Why

Shadows from the player's carried torch were cut along straight lines. Each
caster in a light's cube keeps the faces its bounding sphere touches, and a
face draws only those casters; the faces were worked out again when the
caster moved, not when the light did. The torch moves every frame, so its
casters kept the faces of where the torch had been, and a caster that had
crossed into another face (as seen from the torch now) was left out of it.

## What Changes

- A light's casters keep their bounding sphere with their faces.
- When a light moves, every one of its casters' faces is worked out again
  from the light's new position, as the original refreshes a moved light's
  links.
- The frame profiler reports the cube work (faces, draws) and the links'
  work (lights moved, casters moved, casters tried).

## Capabilities

### Modified Capabilities

- `rendering`: cube faces follow a moved light.

## Impact

- `exa_render::cube_shadows` (`CasterFaces`, `CubeStats`),
  `exa_world::shadow_links` (`LinkStats`), `exarust::profiler`.
- `docs/RENDERING.md`.
- Evidence: `context/re/optimizations.md` §4.2 and §4.4 (a light's
  refresh relinks its nodes and recomputes their face masks).

## Phase

Phase 8, performance (a regression of `cache-character-shadows`).
