# `exa_render`

Rendering on Bevy/wgpu: material plug-ins, skinned characters, terrain, grass, lights and shadow cube maps, voxel GI, post-processing.

**Phase:** Phase 2 (static world), Phase 5 (lighting fidelity) (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: skeleton.

## Modules

- `bound_sphere`: the original's smallest enclosing sphere (move-to-front over a support set of 1–4 points, in f64), which fits its sun shadow map; with the f32/f64 vector helpers ([`docs/RENDERING.md`](../../docs/RENDERING.md), Sun shadow fit).
- `glow`: `GlowMaterial`, the thaumaturgy effects' additive stand-in (vertex colour times a tint, both faces, no shadows) until their own programs are ported ([`docs/THAUMATURGY.md`](../../docs/THAUMATURGY.md), Drawing).
- `image_noise`: the original's procedural noise: white noise from its
  linear congruential sequence, byte octaves (the terrain's blend-rule
  maps) and their ridge fold, ridged float octaves, and `NoiseField`, the
  animated field of the force and spirit powers ([`docs/RENDERING.md`](../../docs/RENDERING.md),
  Procedural noise).
- `mipmap`: the Kaiser mip chain, filtered in the space the format word
  picks (linear, squared colour or centred; [`docs/formats/RFI.md`](../../docs/formats/RFI.md),
  Generated mip levels).
- `shell`: `ShellMaterial`, the rooms' shells drawn opaque, unlit pure black, and the fading lid overlay (blended, no depth test, drawn last) ([`docs/RENDERING.md`](../../docs/RENDERING.md), Rooms and black fill).

## Scope

- Material plug-ins from `.rml` libraries mapped onto Bevy materials (`StandardMaterial` first, custom `Material` + WGSL later).
- Characters: skinning, layered apparel, procedural skin/decal texture generation (morph sets are applied on the CPU, `exa_assets::morph`).
- Lighting: point/spot lights with cube/spot shadow maps; voxel global illumination (original engine has a VXGI pass); ambient occlusion map.
- Terrain (`.rft` + `.ftb` material layers), grass/foliage instancing, particles, blood spatter, power effects.
- Post-processing: bloom, blur, tone mapping matching the original look.

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
