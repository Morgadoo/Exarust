# Proposal

## Why

Bodies were drawn as rigid parts, which opened gaps and slivers at every
joint. The original skins the body with weights it computes from the parts'
seam triangles when a character loads.

## What Changes

- `exa_formats::rfc`: per-triangle flags (`0x3DC3`), position ids, and the
  mesh skin-mode word.
- `exa_assets::skin`: seam groups, joint assignment, loop splits and
  distance blending, as the original's builder does.
- The model loader gives body-part meshes joint attributes. Apparel takes
  its weights from the nearest body vertex.
- `exa_anim::bind_skins` binds skinned meshes to the character's body parts.

Out of scope: hiding covered body slots, culling apparel triangles by cull
bits, apparel fit variants.

## Capabilities

### New Capabilities

### Modified Capabilities
- `motion-playback`: skinned bodies.

## Impact

`crates/exa_formats`, `crates/exa_assets`, `crates/exa_anim`,
`crates/exa_world`, `crates/exa_character`, docs.
