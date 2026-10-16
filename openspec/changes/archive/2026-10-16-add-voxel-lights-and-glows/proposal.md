## Why

In the original's main menu a Healing Salve glows blue on the left shelf,
next to the skeleton, and lights the wall, the shelf and the cobwebs
around it blue. In Exarust it was a small opaque blue shape that lit
nothing.

The salve is a placed object (`JarSalve A01`). Its model holds the glass
jar, an unlit additive glow mesh, and a `0x3D0C` node. That node is a
voxel light: every frame the original adds its colour straight into the
voxel GI volume around it, and the propagation spreads it. The glow mesh
only brightens its own pixels. Exarust dropped `0x3D0C` nodes as plain
nodes, and drew additive glows as opaque shapes: Bevy's additive mode
blends premultiplied, and the shader gave the glow an alpha of 1 taken
from the emissive colour, where the original takes the opacity (0) plus
an opacity fall-off with the view angle.

Phase 5 (visual and audio fidelity), milestone M5: the main-menu scene.

## What Changes

- `exa_formats`: `0x3D0C` nodes decode as voxel lights (colour,
  intensity, radius); `chunk::tag::MESH_VC` becomes `VOXEL_LIGHT`.
  Materials decode the fall-off amount and exponent (`0x64`, `0x68`).
  Items keep the flags word after their model's name; bit 0 dims their
  voxel lights and glow meshes by the placement's condition byte.
- `exa_world`: model nodes with a voxel light get a `VoxelLight`, in
  level models, props and placed objects alike. Placed items with the
  condition flag scale the voxel lights under their root by
  (1 − c/255)², and the emissive colour of the meshes under it by the
  same share. Placed objects' point lights count in the level's lights.
- `exa_render::gi`: voxel lights near the block go to the propagation
  pass in their own list of up to 64, with the box of voxels each
  reaches. `gi.wesl` adds the original's injection after the
  propagation, in every step and the warm-up.
- Surfaces: unlit emissive takes the opacity as alpha; lit and specular
  materials with an emissive layer double the emissive; the opacity
  fall-off fades alpha with the view angle and weights lit emissive maps;
  additive surfaces draw as colour × alpha with alpha 0 through Bevy's
  premultiplied blending; colour and alpha are clamped to 1 as the
  original's 8-bit target clamps them, alpha on every surface, since the
  fall-off takes a glass's alpha past 1.
- Docs: `docs/RENDERING.md` (Surfaces, Blending, Voxel GI),
  `docs/formats/RFC.md` (voxel light node, condition byte),
  `docs/formats/OTHER.md` (model-section flags), `docs/PLAN.md`.

Evidence: REA (Ghidra) on `Exanima.exe` 0.9.5.2g: the node reader and
the voxel light's colour update, the scene-node registration walk, the
per-block GI injection and its `gi_emission` program, the object
builder's condition scaling, and the material render state (the
fall-off's floor and half floats, MatEmi's alpha and doubling). The
decoded shader programs (`shading_f`, `shadeopt_f`, `gi_emission`).
Corpus: all 23 `0x3D0C` nodes parse; the objdb items with the condition
flag; the menu's and `exanima02`'s placed salves; every additive material
carries a fall-off. Captures: Exarust renders of the menu against the
original's capture. Research notes: `context/re/emissive-light.md`.

Out of scope:
- the emissive scale a fire gives its parent mesh;
- the emissive map's alpha on unlit surfaces without a fall-off;
- several resident GI blocks; a voxel light still reaches only the block
  Exarust keeps;
- a capture of the retail game at an `exanima01` light globe (none on
  file).

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `rendering`: voxel light volume; voxel lights, item glow by condition,
  emissive surfaces and opacity fall-off (new requirements).

## Impact

- `exa_formats` (`rfc.rs`, `chunk.rs`, `material.rs`, `objdb.rs`),
  `exa_assets` (`model.rs`, `weapon.rs`), `exa_render` (`gi.rs`,
  `gi.wesl`, `material.rs`, `exa.wesl`, `exa_prepass.wesl`), `exa_world`
  (`lib.rs`), `exa_tool` (`corpus.rs`), `exa_character` (test data),
  `docs/RENDERING.md`, `docs/formats/RFC.md`, `docs/formats/OTHER.md`,
  `docs/PLAN.md`.
