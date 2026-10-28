# Design

## Off-screen pass

A pending icon (`PanelIcons::pending`) is prepared by loading its model
through the shared `ModelLoader` (object packs, then factory weapons) and
baking an `IconScene`. A prepared icon spawns a `Camera3d` rendering into a
2× image, a directional sun and the model's parts, all on
`exa_render::supersample::OFFSCREEN_LAYER` and the camera and sun marked
`Offscreen`. A `ReadbackOnce` is requested six frames later; the observer
resolves the pixels, applies the halo when the plan asks for it, fills the
atlas and slot and despawns the job. A job not answered in 60 frames is
painted on the CPU and freed.

## Isolation

A `Camera3d` is taken by every system that filters on `SceneCamera` or on
`Camera3d` alone. The `Offscreen` marker is excluded from `SceneCamera`,
the light-layer pass, the post adoption, GI (its sun, its shadow views and
its once-per-frame propagation), the cube-shadow faces and the sun-shadow
fit and publish, and it is extracted to the render world for the view
checks. The shadow linker rewrites a caster's layers to the world's, so the
icon parts carry `NotShadowCaster`. The off-screen layer sits between the
resolve layer and the first light layer so a scene camera's layer set never
includes it.

## Camera

`item_renderer::frame_distance` is the scene fit in the original's units;
it is homogeneous in the half extents and the framing distance, so
`item_icon` calls it in metres. The extents are projected in centimetres so
the helper's 0.0001 widening keeps the original's size.
