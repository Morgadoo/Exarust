# `exa_render`

Rendering on Bevy/wgpu: material plug-ins, skinned characters, terrain, grass, lights and shadow cube maps, voxel GI, post-processing.

**Phase:** Phase 2 (static world), Phase 5 (lighting fidelity) (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: skeleton.

## Modules

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
