# `exa_render`

Rendering on Bevy/wgpu: material plug-ins, skinned characters, terrain, grass, lights and shadow cube maps, voxel GI, post-processing.

**Phase:** Phase 2 (static world), Phase 5 (lighting fidelity) (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: skeleton.

## Shader builder

`shader_template` expands the original's `.flb` templates (`{?…?}`
conditionals with the `Mat./Shd./Obj./Env.` flags, `{%LOOP%}`,
`{%expr%}`, includes and the generated `{$Interface}`) and is pure and
tested; the `{$Interface}` of a geometry-stage program expands nothing, as
the builder's does. `shader_program` holds the material factory's material
and shader words, the plug-in-masked feature word (`feature_key`, also used
for map binding and the emissive's doubling), the program choice
(`shading/shadeopt/shadehar/shadesss/shadetrl`), the shadow and displacement
sample splits, the warm-up variants, the surface entry's `<set>_f` name
rule, the height layer's one-texel switch-off and the sub-frame count;
`shader_uniforms` the 252-byte `Global` block (the sub-frame offset times
the frame scale in `ViewPos.w` and `TimeDlt.x`, `−scale/factor` in
`TimeDlt.y`, the eye/sun-eye midpoint
in `FogPos`, whose `w` is the scene height limit capping `y`), the 112-byte `Material` block, the per-light `vec4`s and the
refraction background's choice. The builder's separate shadow-declaration
callback is `shader_template::interface_shadow_samplers` (one `Shadow<i>`
per slot, each tested from its own kind). `exa-tool shader` prints a
material's original GLSL for comparison.
`shader_cache` keeps the program cache: one bucket per material and shader
word pair, one entry per object and environment word pair, a failed build stored and not
retried, reference counts, the last-bound program and the stale-flags bit,
and a warm-up pass through the 29 rows and their light counts.
`shader_bindings` holds the uniform-block bindings, the texture units the
maps pack into (and the fixed environment, GI, screen-copy and shadow
units), the cached uniform locations, the linker's attribute locations
for every program, the `SdwSmp` upload window and the eight exact tap
offsets.
`light_slots` fills a draw's six light slots in the original's order and
values.
`texture_wait` is the material loader's wait for its first five maps'
uploads before the parallax pass reads a height map's size (the original's
`Sleep(0)` spin; Exarust loads finished images through Bevy).
`draw_state` is the drawing state's per-draw plan: the object word, the
gather-or-clear of the light slots, the material block's fade and emissive
rewrites, the liquid-level and GI-corner uploads, the super-sampling
sub-frames with their tap windows and clocks, the single-pass (no super-sampling)
variant, and the effect parent matrix. Nothing is wired into the WESL
renderer (Bevy owns the pipelines, draws and lights).
Rules and evidence: [`docs/formats/FLB.md`](../../docs/formats/FLB.md) and
[`docs/RENDERING.md`](../../docs/RENDERING.md) (Shader builder).

## Modules

- `shader_template`, `shader_program`, `shader_uniforms`: the original's
  template expansion, program choice and uniform blocks (above).
- `shader_cache`, `shader_bindings`, `light_slots`: the program cache, the
  per-program bindings and the per-draw light slots (above).
- `texture_program`: the procedural texture programs of `textures.flb`
  (the entry alone as a fragment shader), the six built-in vertex shaders
  the draw state's stream word picks and their fixed `MPrj` projection;
  Exarust bakes those textures on the CPU.
- `texture_wait`: which of a material's first five map slots the loader
  waits for, and the poll order (above).
- `hair_program`: the hair program (`shadehar_f`, class 3) that `hair.wesl`
  draws for materials with `flags::HAIR`: the two anisotropic strand lobes,
  the wrapping rim, the ambient weight, the bent reflection and the alpha
  branches, pinned in single precision; its test validates `hair.wesl`
  with naga. `flags::NO_SPECULAR` drops the specular sum of a record
  without `MatSpc` in every program.
- `shader_reload`: the `cshd` debug command's reload of the 12 embedded render shaders through the asset server (the original's command reloads its shader programs; Exarust has no file watcher, so this rebuilds the compiled-in sources).
- `bound_sphere`: the original's smallest enclosing sphere (move-to-front over a support set of 1–4 points, in f64), which fits its sun shadow map; with the f32/f64 vector helpers ([`docs/RENDERING.md`](../../docs/RENDERING.md), Sun shadow fit).
- `camera`: the camera node's 70° default and its near and far planes (100 and 12 500 engine units; 1 and 125 m; held as data, no camera applies them yet), and the 3D view's constructor values: exposure 1, bloom 1, post on, no scene camera slot ([`docs/RENDERING.md`](../../docs/RENDERING.md), Cameras).
- `shadow_map`: the shadow-map plumbing, pinned by tests and not driving the renderer yet: the allocator's quality index and class pick by light kind, the size tables with the cube halving, shift and signed hardware cap, the separate draw-time cube sizing and its light offset, the pool of equally sized cube maps handed to lights, the base map-size slot, the directional map's near/far/mask/offset seeds and its 16-bit linear clamped compare sampler, the caster pass's cull face, the caster program selection, the shared cube texture pool (unused by the shipped game), and the directional view and orthographic matrices ([`docs/RENDERING.md`](../../docs/RENDERING.md), Lights).
- `splines`: the thaumaturgy skill tree's lines: the pre-built 2 049-quad index table, strip coordinates, line widths, the tessellation with its chord normal, curvature term (join modes), vertex-index fade and extrapolated close, the 32-byte vertex layout and the line fragment rule in WGSL ([`docs/RENDERING.md`](../../docs/RENDERING.md), Skill-tree splines).
- `glow`: `GlowMaterial`, the thaumaturgy effects' additive stand-in (vertex colour times a tint, both faces, no shadows) until their own programs are ported; the arcs, barricades and wisps draw with it ([`docs/THAUMATURGY.md`](../../docs/THAUMATURGY.md), Drawing).
- `force_effects`: the Force effects' ported programs as materials — `ForceRibbonMaterial` (`effectforce_f`, `effecttorrent_f`) and `ForceShellMaterial` (`effectshell_v`/`effectshell_f` with the skinned 2 cm push) — with the records' blend word 0x163 (premultiplied) and the `_ForceNoise` texture and mip chain ([`docs/RENDERING.md`](../../docs/RENDERING.md), Force effects).
- `sun_fit`: the orthographic region of the sun's one shadow map: in a terrain level the fitted mode's clipping planes (the camera's view direction picks them at the scene's edges and the level's floor and ceiling, cut with `exa_physics::intersect::ConvexVolume`), the corner merge, the 200 cm radius quantisation with its 0.3/0.1 step hysteresis, the texel-grid centre snap, the fixed ±8000 cm depth and the light's frame; in every other level the whole-scene mode's centre, radius, near and far, raised to the scene's 1 000 cm minimum extent. Both give the cascade matrices Bevy's shadow pass uses, the light's softness (with its 0.15 floor only in the fitted mode) and the render routine's caster bias (polygon offset `1 + 1000/h + 2000c'/h` with one unit, and the caster depth nudge); `SunFit` keeps the half extent between frames ([`docs/RENDERING.md`](../../docs/RENDERING.md), Sun shadow fit).
- `sun_casters`: the sun map's render routine as a tested walk, not driving anything yet: the caster list's flag, the volume test's two conditions and unordered compares, the list's head skipping the test, a kept node pushing the volume's faces by its stored vector, the parts' map-visible bits, the per-part front/back cull switch through `shadow_map::CasterCull`, and the alpha-tested part that draws both sides ([`docs/RENDERING.md`](../../docs/RENDERING.md), Sun shadow fit).
- `gi`: the voxel GI volume: one diffusion step a frame through the occupancy texture, read at each voxel's own slice centre (the height measured over the block's width), the per-channel base ambient, the direct light injected sector by sector from the level's 6 m cell lists (groups of up to twelve lights, joined and merged as the original merges them), point lights shadowed by the cached cube pool at the original's d·(bias + 0.94)/R compare, the bias from the shadow detail and the light's quality offset (an unranged light's 3.4e38 far plane leaves its lookup lit against Exarust's faces, a labelled guess), the sun from the fitted shadow map, and the voxel lights (`gi.wesl`; [`docs/RENDERING.md`](../../docs/RENDERING.md), Voxel GI). The manager keeps the original's block presets, the eight corner offsets (each axis in double precision) and quad indices, the light uniform layout (`Light[3i..3i+2]`), the surface sample origins (half a voxel up, the bottom for terrain), the emissive shape list (`gi_shape`: the six `shading.flb` fragment bodies
evaluated per voxel, with their uniforms, tracked from each shape entity's
component), draw box, program names, quad and shared uniforms, the three linked programs' sampler units and uniforms (`GiPass::layout`: `Shadow<i>` on 2 + i, `LVox` 0, `OVox` 1; the emission program sets only `OVox`), the neighbour margin copy's columns, UV offsets and quad, the dynamic update's dirty region, and the occupancy sub-upload
(`GiOccupancyPatch`) a moving mesh's changed box goes out as. A voxel light carries the node's base colour, current and base intensity, base and active radius and flag word, with the 3× modern and double-precision 0.6 legacy colour paths, the activation copy and the clone copy.
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
  `0x0400_0000`, the sharpen kernel and the image sharpen at the image's
  own size (`sharpen_rgba8`, the menu cursor's), kernel normalisation, bytes-per-pixel,
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
- `benchmark`: the GPU benchmark's workload, the original's own fragment
  program as a two-dimensional material (a texture read at the uv, the
  square roots, the reads offset by 0.2341, 0.1476, 0.3768 and 0.4276, the
  dot products, lengths, powers, sines and cosines), the 512² texture of one
  byte value and its repeat/linear sampler, and the sixteen full-screen
  quads the measured scene draws ([`docs/UI.md`](../../docs/UI.md#gpu-benchmark)).
- `passes`: the original's fixed frame walk (clear, alpha-tested and opaque
  classes with depth writes, hook lists, the depth-biased decal class and
  blended per-sector lists with depth writes off, the particles' depth
  pre-pass and colour, the environment's weather), the blended list's
  layer/opacity selection and the opaque list's grouping by material program
  key (the layer is bits 4-7 of the blend word), the per-list draw walk (one
  program bind per material group, entries filtered by the renderer's
  visibility mask, sectors in scene order, the entries' `0x1000_0000`
  scissor switches, the blended entry's `1 − opacity` alpha under the
  `0x203` word) and the particle pass (its own visibility mask, both lists
  depth-only per sector, the second with its textures, then per sector both
  lists in colour with per-entry alpha), with the
  Bevy phase each step maps to
  ([`docs/RENDERING.md`](../../docs/RENDERING.md), Pass order).
- `view`: the 3D view's arithmetic: the perspective frustum's half extents,
  the super-sampling levels (1/2/3/4/8 sub-views, up to four a pass, their
  sub-pixel jitters, sharpen strengths and per-pass schedule), the separate
  render-scale mode, the point-to-window projection, the framebuffer
  viewport and seven-deep region stacks, the original's integer rectangle
  clip and the view copy's
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
`terrain_composite::TerrainCompositeHost` hosts the original's offscreen
composite sets as render-target images (sRGB colour, tangent normal, gloss
and half-size displacement at the pools' 1024²/512² sizes), created one a
frame as the live selector holds them and dropped when a set returns to its
pool. `TerrainBakeQueue` takes one block's layers and a set, and a
render-world pass (`terrain_bake.wesl`) bakes it in the compositor's order
(normal, colour, gloss, displacement), one block a frame, with the layer
stacking the per-pixel draw uses (`terrain_layers.wesl`, a shader library
both import). `TerrainCopyQueue` blits a set into another (a block traded
from a high set to a low one) with the original's built-in copy program
(`terrain_copy.wesl`), each destination target the two sets share, in the
same prepass before the frame's draws. `exa_world::terrain::composite_draw`
then draws the block from its set. All of it is opt-in
(`EXA_TERRAIN_SETS=1`).
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
