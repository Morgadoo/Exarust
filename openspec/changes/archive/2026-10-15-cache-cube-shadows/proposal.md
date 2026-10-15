## Why

Bevy re-renders all six faces of every shadowed light every frame. The
original renders a cube face only when it is needed (it holds something in
view) and dirty (the light or a caster in it moved), so a still dungeon
renders almost no shadow faces. Even with shadow links and walls, the log
room ran at about 16 FPS under load, against the original's 31 FPS.
Character meshes also had no bounds (`NoFrustumCulling`), so they were never
culled, for the camera or per cube face.

## What Changes

- Exarust's own cube shadow pass for static casters:
  - a pool of 64 cubes of 512² 16-bit faces;
  - faces rendered only when both needed and dirty;
  - the original's linear depth, distance / (R·k_c), with k_c from the
    caster's shadow-bias level;
  - back faces culled, alpha cut for masked materials.
- Shaders compare 0.995·d / (R·k_r) against the cached cube, and multiply
  by Bevy's cube for skinned casters. Light records carry the cube slot and
  radius.
- Shadow links split casters: static meshes join their lights' cube caster
  lists, and a change of links or a move dirties those cubes. Skinned meshes
  keep their light layers. A light keeps Bevy's shadow map only while a
  skinned mesh is linked to it.
- Skinned meshes carry per-joint bounds and `DynamicSkinnedMeshBounds`
  instead of `NoFrustumCulling`.
- `--no-shadows` sets a `PointShadows` resource.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `rendering`: cached cube shadows.

## Impact

- `exa_render` (`cube_shadows.rs`, `cube_shadow.wesl`, `shading.wesl`,
  `exa.wesl`, `lights.rs`, `material.rs`, `lib.rs`), `exa_world`
  (`shadow_links.rs`, `lib.rs`, `terrain`), `exa_anim`, `exa_assets`
  (`model.rs`), `exarust` (`main.rs`, `viewer.rs`), `docs/RENDERING.md`,
  `docs/PERFORMANCE.md`.
