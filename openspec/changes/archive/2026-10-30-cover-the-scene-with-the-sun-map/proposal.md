# Proposal

## Why

The original's sun map (`TShadowMapDirect`) has two modes. Exarust had the fitted mode for terrain levels; every other sunlit level used Bevy's two cascades instead of the original's whole-scene map. The render routine's caster bias (polygon offset and a depth nudge on the casters' matrix only) and its caster walk were also missing.

## What Changes

- A level without a terrain gets the whole-scene map: the scene box's centre rotated into the light's frame, the box's radius raised to the scene's 1 000 cm minimum (`min > h`, a NaN never raising) as half extent and up-sun reach, the down-sun reach through the engine's `maxss` helper (a NaN radius gives the minimum), no texel snap, no hysteresis; softness `(c + 1)·(200 / h)` without the 0.15 floor. Bevy samples the single cascade out to the scene's diameter.
- The fitted mode has no minimum extent: only the whole-scene branch reads the scene's minimum.
- The caster bias: polygon factor `1/(h·0.001) + 1` (double, stored single) plus `(c'·20)/(h·0.01)` (double), one unit; the depth nudge `0.01·w`, `w = ½·min((c' + 1)·500 / D, 0.3)` with the 0.3 compared in double, added to the casters' projection only. Exarust writes the nudge, halved and negated for its `[0, 1]` reverse-Z range, into Bevy's drawing matrix only.
- The caster walk as a tested model (`exa_render::sun_casters`): node caster flag, node map-visible bit, the head kept without the test and without moving the volume, later nodes tested and their vectors pushing the volume in place, per-part test and visible bit, one face switch per part, per-mesh draw flag and alpha-cut program.
- The map's sampler, depth bits, constructor seeds and the optional RGB8 colour twin behind a switch that ships off, pinned as data.

## Capabilities

### New Capabilities

### Modified Capabilities

- `rendering`: the sun's whole-scene map, its caster bias and caster walk.

## Impact

- `exa_render::sun_fit`, `exa_render::sun_casters` (new), `exa_render::shadow_map`; `exa_world::sun_shadow`, `exa_world::environment`, `exa_world::LevelLoad`.
- `docs/RENDERING.md` (Sun shadow fit), the crate READMEs.
- Evidence: REA (Ghidra) assembly of the component's update (both branches, the softness, the box and its matrices), the render routine (bias arithmetic, the walk), the fit, the map creation and colour twin, the engine's max helper (one `maxss`), the box-from-planes ortho builder and the look-at basis, the convex-volume box test and face push, and the scene constructors that store the minimum extent.

## Out of scope

- The softness upload and the original's 8-tap disc filter (Bevy has no directional filter-radius uniform without PCSS); the mirroring of the softness onto each view's light aliases.
- The caster-side polygon offset (Bevy's shadow pass has no caster polygon-offset state).
- A driver for the caster walk (Exarust has no scene-node caster list) and the caster-volume builder of the intersection unit.
- The scene's own radius fields: Exarust takes the scene box's half-diagonal for both the half extent and the far field.
- The frozen-matrices flag, the other camera mode and the map's GL lifecycle.

## Phase

Phase 5, visual and audio fidelity: the sun's shadow map.
