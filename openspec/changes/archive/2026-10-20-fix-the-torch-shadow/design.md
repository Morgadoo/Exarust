# Design

## Near plane

Bevy projects point-shadow faces with `perspective_infinite_reverse` and the
light's `shadow_map_near_z`. The shadow comparison in `shading.wesl`
reconstructs the same projection from `clustered_lights[*].light_custom_data`
and compares the receiver's distance 0.5 % short. A 10 cm near plane puts the
close torch's character depths into a narrow band near the projection's
singularity, where the stored and reconstructed depths disagree over a wide
floor area. The original's point shadows start at 3 cm (`NEAR` in
`cube_shadows.rs`), so both the cached cubes and Bevy's skinned maps use that
plane.

The fix belongs in `light_bundle`: every level light and fire gets
`shadow_map_near_z = 0.03`. Bevy's `PointLight` default is 0.1.

## Light ids

`shading.wesl` needs a small integer per light to index the cached-cube and
shaping records. It reads it from the clustered light's `shadow_depth_bias`,
the only spare f32 in Bevy's point-light buffer. Storing the raw id makes the
field invalid as a bias for any Bevy-native surface: ids reach the hundreds,
while Bevy's default bias is 0.08.

The id is therefore encoded as `0.08 + id · 1e-5` and decoded with
`round((bias - 0.08) / 1e-5)`. One step is far above f32's precision at 0.08
(about 7e-9), and a whole level's ids add only a few thousandths to the bias.
The shader constants and the Rust helpers (`LIGHT_ID_BIAS`, `LIGHT_ID_STEP`,
`light_bias`, `light_id_of`) are the same pair, and a unit test round-trips
the ids through them.

## Verification

`target/verify/shadow-fixed.png` shows the old blob at the torch's first
frame. `shadow-near.png` shows the 3 cm plane's body-shaped shadow. The arena
sun shadows and the cached static shadows are unchanged.
