## Why

The character view quits at startup in most runs. Some play starts quit too:
`play exanima01 --at 68.8,0.3,-48.3` failed 4 times out of 4. wgpu reports a
validation error: a pipeline with Bevy's skinned mesh layout is drawn with the
model-only mesh bind group.

The body and apparel meshes carry joint indices and weights, so Bevy builds
their pipelines with the skinned layout as soon as they have a `Mesh3d`. They
get their `SkinnedMesh` later, once the skeleton's parts are known, which can
be a frame or more after they are spawned. A frame drawn in between pairs the
skinned pipeline with the unskinned bind group, and wgpu stops the app. Play
mode usually hides this behind its loading screen, which waits for dressing.

Phase 2 (Characters): bodies and apparel drawn skinned, without crashes.

## What Changes

- A skinned part (`SkinnedPart`) keeps its mesh handle and is spawned without
  a `Mesh3d`. `bind_skins` inserts the `Mesh3d` together with its
  `SkinnedMesh`, so the mesh is never drawn without its skin.
- Code that reads skinned meshes before they are bound uses the handle on
  `SkinnedPart`: the limp body's part boxes and the hiding of body parts
  under apparel.
- `docs/formats/RSQ.md` states the rule.

## Capabilities

### New Capabilities

### Modified Capabilities

- `motion-playback`: skinned meshes are drawn only once bound to the skeleton.

## Impact

- `exa_assets::SkinnedPart` gains `mesh` and is no longer `Copy`.
- `exa_world` and `exa_character` spawn skinned parts without `Mesh3d`;
  `exa_anim::bind_skins` adds it; `exa_game::limp` and the apparel covering in
  `exa_character` read the handle on `SkinnedPart`.
- A skinned mesh that never finds its skeleton is not drawn. Before, drawing
  it failed validation.
