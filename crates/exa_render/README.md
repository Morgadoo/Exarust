# `exa_render`

Rendering on Bevy/wgpu: material plug-ins, skinned characters, terrain, grass, lights and shadow cube maps, voxel GI, post-processing.

**Phase:** Phase 2 (static world), Phase 5 (lighting fidelity) (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: skeleton.

## Shader builder

`shader_template` expands the original's `.flb` templates (`{?…?}`
conditionals with the `Mat./Shd./Obj./Env.` flags, `{%LOOP%}`,
`{%expr%}`, includes and the generated `{$Interface}`) and is pure and
tested; `shader_program` holds the material factory's material and shader
words, the plug-in-masked feature word (`feature_key`, also used for map
binding and the emissive's doubling), the program choice
(`shading/shadeopt/shadehar/shadesss/shadetrl`), the shadow and displacement
sample splits, the warm-up variants; `shader_uniforms` the 252-byte `Global`
block, the 112-byte `Material` block and the per-light `vec4`s.
`exa-tool shader` prints a material's original GLSL for comparison. Rules
and evidence: [`docs/formats/FLB.md`](../../docs/formats/FLB.md) and
[`docs/RENDERING.md`](../../docs/RENDERING.md) (Shader builder).

## Modules

- `shader_template`, `shader_program`, `shader_uniforms`: the original's
  template expansion, program choice and uniform blocks (above).
- `shader_reload`: the `cshd` debug command's reload of the 12 embedded render shaders through the asset server (the original's command reloads its shader programs; Exarust has no file watcher, so this rebuilds the compiled-in sources).
- `bound_sphere`: the original's smallest enclosing sphere (move-to-front over a support set of 1–4 points, in f64), which fits its sun shadow map; with the f32/f64 vector helpers ([`docs/RENDERING.md`](../../docs/RENDERING.md), Sun shadow fit).
- `camera`: the camera node's 70° default and its near and far planes (100 and 12 500 engine units; 1 and 125 m; held as data, no camera applies them yet), and the 3D view's constructor values: exposure 1, bloom 1, post on, no scene camera slot ([`docs/RENDERING.md`](../../docs/RENDERING.md), Cameras).
- `shadow_map`: the shadow-map plumbing, pinned by tests and not driving the renderer yet: the allocator's quality index and class pick by light kind, the size tables with the cube halving, shift and signed hardware cap, the separate draw-time cube sizing and its light offset, the pool of equally sized cube maps handed to lights, the base map-size slot, the directional map's near/far/mask, the caster pass's cull face, the caster program selection, the shared cube texture pool (unused by the shipped game), and the directional view and orthographic matrices ([`docs/RENDERING.md`](../../docs/RENDERING.md), Lights).
- `splines`: the thaumaturgy skill tree's lines: the pre-built 2 049-quad index table, strip coordinates, line widths, the tessellation with its chord normal, curvature term (join modes), vertex-index fade and extrapolated close, the 32-byte vertex layout and the line fragment rule in WGSL ([`docs/RENDERING.md`](../../docs/RENDERING.md), Skill-tree splines).
- `glow`: `GlowMaterial`, the thaumaturgy effects' additive stand-in (vertex colour times a tint, both faces, no shadows) until their own programs are ported ([`docs/THAUMATURGY.md`](../../docs/THAUMATURGY.md), Drawing).
- `sun_fit`: the fitted orthographic region of the sun's one shadow map in a terrain level: the clipping planes the camera's view direction picks at the scene's edges and the level's floor and ceiling (cut with `exa_physics::intersect::ConvexVolume`), the corner merge, the 200 cm radius quantisation with its 0.3/0.1 step hysteresis, the texel-grid centre snap, the fixed ±8000 cm depth, the light's frame and the cascade matrices Bevy's shadow pass uses; `SunFit` keeps the half extent between frames ([`docs/RENDERING.md`](../../docs/RENDERING.md), Sun shadow fit).
- `gi`: the voxel GI volume: one diffusion step a frame through the occupancy texture, the per-channel base ambient, the direct light injected sector by sector from the level's 6 m cell lists (groups of up to twelve lights, joined and merged as the original merges them), point lights shadowed by the cached cube pool, the sun from the fitted shadow map, and the voxel lights (`gi.wesl`; [`docs/RENDERING.md`](../../docs/RENDERING.md), Voxel GI). The manager keeps the original's block presets, the eight corner offsets and quad indices, the emissive shape list and draw box, the neighbour margin copy's columns and UV offsets, and the dynamic update's dirty region. A voxel light carries the node's base colour, current and base intensity, base and active radius and flag word, with the 3× modern and double-precision 0.6 legacy colour paths, the activation copy and the clone copy.
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
- `geometry`: the original's planar vertex streams (format-word flags, per-stream sizes and buffer offsets, attribute locations, index-size encoding and CPU index batches) as tested pure rules; not called by the running renderer, where Bevy's mesh allocator takes their place.
- `queue`: the deferred draw queue (1023 entries, bucketed by the sector's light count at weight 0.004 or more capped at six, kept as a list grouped by bucket with the original's head and behind-the-first insertion rules).
- `capture`: the still-image capture plan (eight samples jittered from the recorded `view` table, blended at `1 − 1/(i+1)`, half-size downsample) and the two recorded downsample shader sources.
- `quality`: the settings screen's live renderer options: `ShadowQuality`
  sizes the point lights' cube faces and the sun map, `MultisampleSetting`
  the scene camera's MSAA while super-sampling is off, and
  `supersample::SuperSampling` turns the supersampled scene's target off and
  on ([`docs/UI.md`](../../docs/UI.md#what-actually-takes-effect)).
- `passes`: the original's fixed frame walk (clear, alpha-tested and opaque
  classes with depth writes, hook lists, the depth-biased decal class and
  blended per-sector lists with depth writes off, the particles' depth
  pre-pass and colour, the environment's weather), the blended list's
  layer/opacity selection and the opaque list's grouping by material program
  key, with the Bevy phase each step maps to
  ([`docs/RENDERING.md`](../../docs/RENDERING.md), Pass order).
- `view`: the 3D view's arithmetic: the perspective frustum's half extents,
  the super-sampling levels (1/2/3/4/8 sub-views, up to four a pass, their
  sub-pixel jitters and sharpen strengths), the separate render-scale mode,
  the point-to-window projection, the framebuffer viewport and seven-deep
  region stacks, the original's integer rectangle clip and the view copy's
  quad ([`docs/RENDERING.md`](../../docs/RENDERING.md), Supersampling).
- `shell`: `ShellMaterial`, the rooms' shells drawn opaque, unlit pure black, and the fading lid overlay (blended, no depth test, drawn last) ([`docs/RENDERING.md`](../../docs/RENDERING.md), Rooms and black fill).
- `electricity`: the electric arcs' rules and material. `ArcMaterial` is the original's `_ElecMat01` fragment program (the texture's red mixed a third into the ribbon's V, the 24th and 6th powers, EleCol and EdgCol, squared, additive and two-sided; `arc.wesl`); the module also holds the 84-byte arc record, the weapon, spark-burst and device presets (sustained or one-shot), the charge an electrified item spends, the arc checks' release and rebuild rules and the jittered tendril ribbon (U along, V across) ([`docs/RENDERING.md`](../../docs/RENDERING.md), Electricity).
- `flame`: `FlameMaterial` for the fire presets' particles (`flame.wesl`, premultiplied blending).
- `material`: the original's surface material, its programs and the
  `set_texture_bias` pass that a texture-quality change runs over every
  loaded material (the original sets the LOD bias on each bind).
- `material_state`: the default material factory's type table (five built-ins, the reset and plug-in registration), its factory list (priority-ordered, every matching record called), the normal-map predicate, and the loader's diffuse scale order and gloss floor that `exa_assets` uses.
- `smoke`: the original's `shadesmk_f` smoke and mist program (`shadesmk.wesl`, line for line without its fog and dither), its `UVOfst` scroll and state key (not spawned yet).
- `quads`: the data path of the original's 2-D immediate-draw batcher: the begin scale, flipped y, cumulative 16-level translation stack, texture coordinates and RGBA colours, the quad index template and line pairs, the projections, the six built-in vertex programs and their attribute locations (nothing draws a batch yet).

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
