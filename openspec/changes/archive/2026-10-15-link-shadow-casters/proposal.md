## Why

Every mesh within a light's range cast into all six faces of its cube
shadow map, re-rendered every frame. In exanima01 (229 lights, many of
radius 15 m, about 13 000 meshes) Bevy's per-face bookkeeping grew to about
12 GB within half a minute, and the frame rate fell from about 21 to under
2 FPS. The original links a node to a light only where the light still
matters at the node: their spheres overlap and the attenuated luma there
exceeds 0.01. Only linked nodes cast. A light's own object is left out of
that light's map only. Exarust had instead taken those meshes out of every
map.

## What Changes

- Shadow links: a mesh casts into a light when their bounding spheres
  overlap and (1 − 0.8·d/R)⁴ · luma(colour · intensity) > 0.01, as the
  original links nodes to lights. Links are relinked incrementally when
  lights or meshes move.
- Links reach Bevy as render layers: one layer per shadowed light, layer 0
  plus the linked lights' layers on each mesh, all of them on the scene
  cameras. The supersampling resolve quad moves to layer 1.
- A light's own object is excluded from that light's map only.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `rendering`: shadow casters per light.

## Impact

- `exa_world` (`shadow_links.rs`, `lib.rs`), `exa_render`
  (`supersample.rs`), `docs/RENDERING.md`.
