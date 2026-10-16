## Context

Formats: `docs/formats/RFC.md` (voxel light node, placed objects' condition
byte) and `docs/formats/OTHER.md` (object database model section). Rules:
`docs/RENDERING.md` (Surfaces, Blending, Voxel GI).

The original injects voxel lights per resident GI block, after its
propagation draw, as additive draws of a 6 × 6-column quad over 7 slices
into the same 8-bit sRGB target. Exarust propagates in one compute pass
that writes a scratch copy, so the injection has to happen inside that
pass.

## Decisions

- **Inside the propagation pass.** Each voxel adds the voxel lights whose
  box holds it to its propagated light before the sRGB encoding. The
  original adds after quantising the propagation to 8 bits; the difference
  is one rounding. Voxels the step does not write (below a sector's floor,
  the top slice, empty sectors, margins) keep their light, so nothing
  builds up there.
- **The box on the CPU.** The CPU works out each light's voxel box (the
  corner rounding, margins, the k + 1 slices near the bottom) and the
  shader only tests it; the weight's formula is in both, and the Rust copy
  is what the tests check against the original's figures (22.0 and 3.8
  voxel-equivalents over the box, open and blocked).
- **Their own list.** Up to 64 voxel lights per block in the same uniform
  buffer, nearest the block's centre first, apart from the 32 point
  lights. The corpus has at most a handful in a level.
- **Condition scaling on a copy of the model.** A placed item with the
  condition flag and a byte other than 0 spawns a copy of its model whose
  voxel lights under the root carry the scaled intensity and whose meshes
  under the root use copies of their materials with the emissive colour
  scaled (the original scales the material uniform's emissive colour, not
  its alpha, for those meshes); the cached model and the shared materials
  stay as they are.
- **Additive surfaces.** Bevy's `AlphaMode::Add` blends ONE,
  ONE_MINUS_SRC_ALPHA. The shader writes (colour·alpha, 0), which is
  SRC_ALPHA, ONE. Colour and alpha are clamped to 1 first, since the
  original's 8-bit target clamps a fragment before blending and Exarust's
  scene target is a float one.
- **Alpha clamped on every surface.** The fall-off takes a premultiplied
  glass's alpha past 1 at its outline (`glsbrnjar` reaches 1.6); on the
  float target ONE_MINUS_SRC_ALPHA would then go negative and subtract
  what is behind. The shader clamps alpha to [0, 1] for all surfaces.
- **Fall-off for every record above 0.01.** The original's test that
  looks like an opaque-blend check compares the low nibble of the blend
  word with `0x100`, which never holds, so every record whose amount
  exceeds 0.01 takes the fall-off, its floor off the opacity and its
  weight on lit emissive maps. On opaque surfaces only the emissive weight
  shows.

## Open questions

- The emissive scale a fire's density gives its parent mesh (the same
  material-uniform path as an item's condition) is not implemented. To
  answer with REA on the fire's update.
- The steady-state brightness of the GI near the salve: no RenderDoc
  capture or probe of the retail volume. To answer with a capture of the
  menu.
- How the retail light globes (`gldlg01`, additive with a fall-off) look:
  no capture on file. To answer with a retail screenshot at an
  `exanima01` globe (`retail-exanima` skill).
- Readers of the voxel light's stored radius, and the node flag `0x10` on
  object voxel lights.
