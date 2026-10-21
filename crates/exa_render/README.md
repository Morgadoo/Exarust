# `exa_render`

Rendering on Bevy/wgpu: material plug-ins, skinned characters, terrain, grass, lights and shadow cube maps, voxel GI, post-processing.

**Phase:** Phase 2 (static world), Phase 5 (lighting fidelity) (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: skeleton.

## Shader builder

`shader_template` expands the original's `.flb` templates (`{?…?}`
conditionals with the `Mat./Shd./Obj./Env.` flags, `{%LOOP%}`,
`{%expr%}`, includes and the generated `{$Interface}`) and is pure and
tested; `shader_program` holds the flag tables, the shader word and program
choice (`shading/shadeopt/shadehar/shadesss/shadetrl`), the shadow-sample
split, the warm-up variants; `shader_uniforms` the 252-byte `Global` block,
the 112-byte `Material` block and the per-light `vec4`s. `exa-tool shader`
prints a material's original GLSL for comparison. Rules and evidence:
[`docs/formats/FLB.md`](../../docs/formats/FLB.md) and
[`docs/RENDERING.md`](../../docs/RENDERING.md) (Shader builder).

## Modules

- `shader_template`, `shader_program`, `shader_uniforms`: the original's
  template expansion, program choice and uniform blocks (above).
- `bound_sphere`: the original's smallest enclosing sphere (move-to-front over a support set of 1–4 points, in f64), which fits its sun shadow map; with the f32/f64 vector helpers ([`docs/RENDERING.md`](../../docs/RENDERING.md), Sun shadow fit).
- `glow`: `GlowMaterial`, the thaumaturgy effects' additive stand-in (vertex colour times a tint, both faces, no shadows) until their own programs are ported ([`docs/THAUMATURGY.md`](../../docs/THAUMATURGY.md), Drawing).
- `sun_fit`: the fitted orthographic region of the sun's one shadow map in a terrain level: the clipping planes the camera's view direction picks at the scene's edges and the level's floor and ceiling (cut with `exa_physics::intersect::ConvexVolume`), the corner merge, the 200 cm radius quantisation with its 0.3/0.1 step hysteresis, the texel-grid centre snap, the fixed ±8000 cm depth, the light's frame and the cascade matrices Bevy's shadow pass uses; `SunFit` keeps the half extent between frames ([`docs/RENDERING.md`](../../docs/RENDERING.md), Sun shadow fit).
- `gi`: the voxel GI volume: one diffusion step a frame through the occupancy texture, the per-channel base ambient, the direct light injected sector by sector from the level's 6 m cell lists (groups of up to twelve lights, joined and merged as the original merges them), and the voxel lights (`gi.wesl`; [`docs/RENDERING.md`](../../docs/RENDERING.md), Voxel GI).
- `image_noise`: the original's procedural noise: white noise from its
  linear congruential sequence, byte octaves (the terrain's blend-rule
  maps) and their ridge fold, ridged float octaves, and `NoiseField`, the
  animated field of the force and spirit powers ([`docs/RENDERING.md`](../../docs/RENDERING.md),
  Procedural noise).
- `mipmap`: the original's resampling unit: the generated chain (a bank of
  13 normalised Kaiser kernels for colour words, a uniform bank for data
  words, each level filtered directly from the base, sides shifted right
  with a floor of one and a final 1×1 level, each axis wrapping or clamping
  by the image's own flags; `ChainOptions::for_image`), filtered in the
  space the format word picks (linear, squared colour or centred), the
  chain from a later first level, which the texture detail
  setting uploads as the base (all levels from it, or that level alone when
  mipmapping is off), the resample to an arbitrary size (the chain's level
  for a power-of-two ratio, else the phase-shifted window), normal-map
  renormalisation for three- and four-channel images flagged
  `0x0400_0000`, the sharpen kernel, kernel normalisation, bytes-per-pixel,
  the sub-image blits and the canvas placement of the icon atlas
  ([`docs/formats/RFI.md`](../../docs/formats/RFI.md), Generated mip levels).
- `quality`: the settings screen's live renderer options: `ShadowQuality`
  sizes the point lights' cube faces and the sun map, `MultisampleSetting`
  the scene camera's MSAA while super-sampling is off, and
  `supersample::SuperSampling` turns the supersampled scene's target off and
  on ([`docs/UI.md`](../../docs/UI.md#what-actually-takes-effect)).
- `shell`: `ShellMaterial`, the rooms' shells drawn opaque, unlit pure black, and the fading lid overlay (blended, no depth test, drawn last) ([`docs/RENDERING.md`](../../docs/RENDERING.md), Rooms and black fill).
- `electricity`: the electric arcs' rules and material. `ArcMaterial` is the original's `_ElecMat01` fragment program (the texture's red mixed a third into the ribbon's V, the 24th and 6th powers, EleCol and EdgCol, squared, additive and two-sided; `arc.wesl`); the module also holds the 84-byte arc record, the weapon, spark-burst and device presets (sustained or one-shot), the charge an electrified item spends, the arc checks' release and rebuild rules and the jittered tendril ribbon (U along, V across) ([`docs/RENDERING.md`](../../docs/RENDERING.md), Electricity).
- `flame`: `FlameMaterial` for the fire presets' particles (`flame.wesl`, premultiplied blending).

## Scope

- Material plug-ins from `.rml` libraries mapped onto Bevy materials (`StandardMaterial` first, custom `Material` + WGSL later).
- Characters: skinning, layered apparel, procedural skin/decal texture generation (morph sets are applied on the CPU, `exa_assets::morph`).
- Lighting: point/spot lights with cube/spot shadow maps; voxel global illumination (original engine has a VXGI pass); ambient occlusion map.
- Terrain (`.rft` + `.ftb` material layers), grass/foliage instancing, particles, blood spatter, power effects.
- Post-processing: bloom, blur, tone mapping matching the original look.

## Terrain

The terrain material lives in `exa_world::terrain`; its shader
(`terrain.wesl`) shares `exa_render::shading` with the lit model program, so
terrain and models see the same lights, GI, reflections and shadows. Its
palette normal map is applied in the original's tangent frame (+X projected
onto the surface, crossed with the vertex normal), its gloss is the
composite gloss (roughness 1 − it, a full gloss being a mirror whose one
singular point the shared light keeps at 0), and its specular is masked by
the red of its diffuse colour (the composite times the terrain's 0.75).
Rules:
[`docs/RENDERING.md`](../../docs/RENDERING.md) (Terrain).

## Blood

`material::BloodProgram` is the original's dirt mode 4 in its three
programs (cloth, skin, hair): the vertex colour's alpha (255 clean, lowered
by wounds) against the tiling speckle `spatter01` bound as the dirt map,
with the original's tints and wet roughness. `exa.wesl` runs it only for
meshes with a colour stream, so clean characters draw as before. Rules:
[`docs/RENDERING.md`](../../docs/RENDERING.md) (Blood).

`stain::StainMaterial` is the original's weapon-stain pass: a second draw
of a bloodied weapon's mesh that multiplies the frame by the blood
gradient `BldGr02D` (read as data, repeating across, clamped along), keyed
on the mesh's position in the weapon's frame so that it runs down from
the tip as far as the weapon's blood takes it (`stain.wesl`; depth writes
off, the depth nudged toward the camera, no shadows). Rules:
[`docs/RENDERING.md`](../../docs/RENDERING.md) (Blood on weapons).

`spray::SprayMaterial` is the original's blood droplet material
(`_BloodPrt02`): the droplet texture `BldPrt02` (sRGB, clamped) mixed
toward white by each droplet's vertex alpha and
multiplied into the frame (`spray.wesl`; unlit, both faces, no depth
writes, no shadows). Rules: [`docs/RENDERING.md`](../../docs/RENDERING.md)
(Blood sprays).
