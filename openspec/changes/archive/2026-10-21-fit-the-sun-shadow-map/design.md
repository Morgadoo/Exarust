# Design

## Behaviour

**Mode.** The update fits the map only when the scene has a terrain (the terrain loader marks it); otherwise it uses a whole-scene map from the scene's bounding sphere with a minimum extent, re-rendered only when dirty. Exarust keeps Bevy's cascades there.

**View volume.** The update copies the camera view's convex volume: the camera's frustum, six quads, built from the camera node's field of view, near and far (constructor defaults 100 cm and 125 m). Exarust builds it from each active camera's projection, its far clamped to 125 m.

**Planes.** The fit reads the camera node's third frame row, its backward axis (the camera looks along its negation). For each axis a component < 0 adds the plane (+axis, high) and one > 0 the plane (−axis, −low), kept where `normal · p − w ≤ 0`; NaN adds none (unordered compares skip both). High and low on x and z are the scene box's centre ± half extent. On y: if the scene has an environment record, the record's highest and lowest points (whole cm, copied into the scene by the level loader); otherwise the box's x centre ± half extent, as the instructions read them. The update first stores 150 and 650 cm into the box's vertical centre and extent; the fit never reads them. The clip is the engine's convex-volume clip (`exa_physics::intersect::ConvexVolume::clip_by_plane`, whose offset is −w).

**Fit.** Corners in face order, merged when the squared distance `(x² + y²) + z²` is below the single 0.1; fewer than two stop the update (the old map and matrices stay). Points ×0.01, sphere (`bound_sphere`), centre and radius ×100. Steps: `r/200` single, widened, + 0.4 double, converted to a 64-bit integer by the processor's default rounding, back to single × 200. Grow when `200·0.3 + h < r`, shrink when `h − 200·0.1 > r`, both double. Centre through the look-at basis (normalise sums `(x² + z²) + y²`, reciprocal square root; Exarust's is exact), x and y divided by `2h / size` (single), converted, multiplied back. Depths 8000 cm each side.

**Bevy.** The fit writes one `Cascade` per camera after Bevy's cascade builder and before its frusta update; the light's `CascadeShadowConfig` becomes one bound at 125 m. The view origin sits 80 m up-sun of the centre so the box is [0, 160 m] in front of it, reverse-Z. Texel size `2h / 4096` m feeds Bevy's normal bias.

## Model

`exa_render::sun_fit`: `ClipPlane`, `SceneBox`, `VerticalRange`, `view_clip_planes`, `slab_clip_planes`, `LightBasis`, `frustum_volume`, `volume_corners`, `convert_nearest`, `quantised_half_extent`, `next_half_extent`, `snapped_centre`, `sun_softness`, `SunFit`, `SunMatrices`. `exa_world::sun_shadow`: `SunShadowRegion` (box, vertical range, terrain flag), `SunShadowFit` (the flag), `SunFitState` (per camera fit and last matrices), the `fit_sun_shadows` system.

## Open questions

- The camera far plane the game sets for its play camera: answered by REA on the game's camera setup (the play-mode view creation) or a capture of the running game's camera node.
- The scene box's source (the scene sector's bounds, including the terrain?): answered by REA on the scene bounds maintenance routines.
- The whole-scene mode's minimum extent and far field (two scene fields with no traced writer for the shipped levels): answered by REA xrefs on those fields.
- The camera mode index that selects the two vertical planes: answered by REA on its writers.

## Docs

`docs/RENDERING.md` (Sun shadow fit); no `docs/formats/` page changes (the environment record's floor and ceiling are already in `docs/formats/RFC.md`).
