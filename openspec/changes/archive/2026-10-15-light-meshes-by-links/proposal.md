## Why

The original lights each object with at most its six strongest links to
lights, strongest first. An object with no link gets no point light at all.
Exarust lit every surface with every light whose range reached it, so faint
far lights still added to every mesh. Once lights stop at walls, links are
also what will keep light from leaking into the next room.

## What Changes

- A record buffer for lights, bound to every surface material: each light
  gets an id, carried in its depth-bias field, and its record holds the
  vertical shaping that used to ride in Bevy's bias fields.
- Per-mesh light lists in the same buffer: the six strongest links of each
  mesh, with the mesh's slot carried in its mesh tag above the shadow-bias
  level. The shaders skip lights not on a mesh's list.
- Shadow links keep each mesh's link weights; render layers and light lists
  both come from them. Lights without shadow maps are linked too.
- Terrain tiles and meshes that cast no shadows keep every light in reach.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `rendering`: lights per mesh.

## Impact

- `exa_render` (`lights.rs`, `shading.wesl`, `exa.wesl`, `material.rs`,
  `gi.rs`, `lib.rs`), `exa_world` (`shadow_links.rs`, `lib.rs`, `grid.rs`,
  `terrain`), `docs/RENDERING.md`.
