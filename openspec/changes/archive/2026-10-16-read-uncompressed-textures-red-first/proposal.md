## Why

The ember piles in the fire pits of the outdoor arenas (`arenao3`,
`arenao5`) glow blue in Exarust. In the original they glow red and orange,
and their own voxel light is orange (1, 0.52, 0). The embers' material
(`embersa_low` in `Objlib/embers a01`) puts `EmbersAEL.rfi` in its emissive
slot, and that texture is an uncompressed RGB8 image in packed storage.
Exarust's decoder read every uncompressed colour image as B, G, R(, A), as
the community notes say, so the orange glow came out blue.

The same swap hit every uncompressed colour image in the data:
- RGB8 normal maps, which hold z in their third byte, gave the shader z as
  x. Surfaces using them (weapon hafts, langets and pommel guards, the iron
  library materials, oak bark and leaves) were lit as if they faced
  sideways.
- Foliage (`OakLfB01D`, `PlantsBFA01D`, `BluWd02_01AD`) turned teal.
- Packed specular maps (`LampOrnA01S`) swapped intensity and metalness.

Phase 5 (Visual and audio fidelity): materials and emissive surfaces drawn
as the original's.

## What Changes

- Uncompressed RGB8 and RGBA8 images decode in their stored order, red
  first, for raw and packed storage alike. The environment cube already read
  its faces this way and is unchanged.
- A unit test on synthetic bytes pins the order for raw and packed RGB8 and
  RGBA8, and for an RGB8 normal map.
- `exa-tool corpus images` tallies every image by format word and storage
  and checks that each uncompressed RGB8 normal map decodes with z in blue.
- `docs/formats/RFI.md` states the order and how it was established;
  `docs/RENDERING.md`, `docs/PLAN.md` and the community-notes page follow.

Evidence:
- reverse engineering of `Exanima.exe` 0.9.5 through REA: the texture
  format table the upload reads (RGB8 → `GL_RGB`, RGBA8 → `GL_RGBA`, both
  unsigned bytes, no BGR entry), the format lookup (words with bit 20 set
  and bit 31 clear compare under a mask and share the RGB8 entry), the
  packed-storage unpacker (pixels copied byte for byte) and the
  `glTexImage2D` binding the upload calls;
- the embers' material record (emissive slot `EmbersAEL.rfi`, emissive
  colour 0.588 grey, no tint) and voxel light (orange);
- a decode of all 86 uncompressed 3- and 4-channel images in the retail
  packs in both orders: the 42 normal maps hold z (about 255) in the third
  byte, and foliage, fire particles and tutorial pictures look natural only
  in the stored order;
- a capture of the original's main menu: its title (`maintitle01`, packed
  RGBA8) is steel blue (blue above green above red), as it decodes red
  first;
- screenshots of `arenao3` and `arenao5` before and after.

Out of scope:
- A capture of the running original at the fire pits.
- Uses of the thaumaturgy icons and `pwrcrstl01e`; nothing in Exarust or in
  the data's materials draws them yet.
- Any other emissive or fire behaviour.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `texture-decoding`: uncompressed colour images keep their stored channel
  order.

## Impact

`exa_formats` (`rfi`); every consumer of decoded textures (`exa_assets`
model and paint textures, `exa_world` terrain planes, `exa-tool png` and
glTF export) sees the corrected order. `exa_tool` gains `corpus images`.
Docs: `docs/formats/RFI.md`, `docs/RENDERING.md`, `docs/PLAN.md`,
`docs/research/online-research.md`, `crates/exa_tool/README.md`.
