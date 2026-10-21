# Proposal

## Why

The original's sun has one orthographic shadow map, not a cascade chain (`TShadowMapDirect`, 5.6 KB). In a level with a terrain it fits that map to the camera's view every frame: it clips the view at the scene's edges, takes the smallest sphere around what is left, quantises its radius to 200 cm steps with a hysteresis, snaps the centre to the texel grid and gives the box a fixed ±80 m depth. Exarust's sun used two Bevy cascades over 60 m on a 2048² map. A first pass ported the fit but chose the clipping planes from the sun's direction and a fixed vertical box; the assembly picks them from the camera's backward axis and the level's floor and ceiling, and fits only terrain levels.

## What Changes

- The sun's map is 4096², the original's at its top shadow detail.
- In a level with a terrain, each active camera's frustum (out to the original camera's 125 m far plane) is clipped by the planes its view direction picks at the level's box and the environment record's floor and ceiling, fitted with the smallest enclosing sphere, quantised with the hysteresis, snapped and given the ±8000 cm depth; the sun's Bevy cascades for that camera become the one fitted map, which Bevy's shadow pass, culling and shaders use. A frame that fits nothing keeps the previous map.
- Levels without a terrain keep Bevy's two cascades over 60 m (standing in for the original's whole-scene map), as does every level with `--bevy-sun-cascades`.
- The fit's conversions and comparisons follow the processor's (nearest, ties to even; NaN and overflow to the lowest integer; double-precision constants where the original compares in double).
- The view volume is cut with the engine's convex-volume clip already in `exa_physics::intersect`.

## Capabilities

### New Capabilities

### Modified Capabilities

- `rendering`: the sun's fitted shadow map.

## Impact

- `exa_render::sun_fit` (new), `exa_render::SUN_SHADOW_SIZE`; `exa_render` depends on `exa_physics`.
- `exa_world::sun_shadow` (new), the sun's spawn in `exa_world::environment`, the level box and terrain extent in `exa_world` and `exa_world::terrain`; `exarust --bevy-sun-cascades`.
- `docs/RENDERING.md` (Sun shadow fit), `docs/DEBUGGING.md`, the crate READMEs.
- Evidence: REA (Ghidra) assembly and constants of the ten functions of the component, the vector helpers they call (squared distance, normalise, negate, scale, point transform, the look-at basis, the convex clip), the camera view's constructor and per-frame projection setup (its camera node, near and far), the camera constructor's defaults, the scene's constructor and the environment-record loader (floor and ceiling copied into the scene), and the terrain loader that marks a scene as having a terrain.

## Out of scope

- The whole-scene mode of levels without a terrain (its minimum extent and far field values are untraced) and the cached re-render it allows.
- The softness the update writes into the sun light (Bevy has no directional filter-radius uniform without PCSS) and the original's 8-tap disc filter.
- The caster-side polygon offset and NDC depth nudge; Bevy's depth and normal biases apply.
- The other camera mode's two vertical planes (Exarust has no such camera mode), the frozen-matrices flag, the optional colour side texture and the GL object lifecycle.
- The original scene's own bounds: Exarust's box is the props' origins and the terrain's tiles.

## Phase

Phase 5, visual and audio fidelity: the sun's shadow map.
