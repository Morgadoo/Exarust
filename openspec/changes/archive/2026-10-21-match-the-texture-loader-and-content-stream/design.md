# Design

Extends [docs/formats/RFI.md](../../../docs/formats/RFI.md) (Generated mip
levels: first level, colour space, header flags) and
[docs/RENDERING.md](../../../docs/RENDERING.md) (Colour).

## Behaviour

**Word.** `texture::material_word` is the record's load flags or-ed with
the quality mask; the model loader adds `DATA | NO_LOD_BIAS` (0x300) for
normal, gloss and height maps and the specular of a kind-bit-0 material,
`NO_LOD_BIAS` for a diffuse with a subsurface colour
(`Material::has_secondary_colour`: r² + g² + b² > 0 in f32, so NaN and
underflow do not count). `rfi_to_texture` ors the header flags from bit 16
up (`IMAGE_FLAG_MASK`). `TextureSampler::from_flags` reads filters, wrap
and anisotropy; anisotropy is cut to 1 when a filter is nearest because
wgpu rejects it otherwise.

**Levels.** `RfiHeader::first_mip_level` computes the first level against
the device side (`DEVICE_MAX_TEXTURE_SIZE`, wgpu's default 8192); the
original's static 256 is overwritten by the driver's value before any
upload. A compressed image without a chain has its mip bits cleared and its
first level forced to 0, on the BC and the CPU path. Mipmapping needs the
mip bit in both the word and the quality mask. A level past a short stored
chain clamps to the last stored one (the original would upload nothing;
the shipped data never asks).

**Colour space.** `word_is_data`: an uncompressed word with bit 16, which
the upload routine itself sends to the linear column; a compressed `0x82`
or `0x92` word, whose table entries have no sRGB column.

**Manager.** `TextureName` cuts at the last dot within its sixteen bytes
(index above zero) and folds A–Z; `Ord` compares the four little-endian
`i32` words by the sign of their wrapping difference. `acquire` and
`insert_new` take a reference on a hit and leave the record alone. Exarust
keys by name and colour space (Bevy needs separate sRGB and linear images).
The model loader looks images up by the stored name, which finds three more
shipped images (names with an extension or a trailing space).

**Placeholder.** `placeholder_image` is RGB8 (64, 64, 64) with the
placeholder's header flags 0x20070000 joined to the word. The model loader
uses it for a shipped image that fails to parse or convert. An unshipped
image stays no map: the original resolves names through registered loaders
that also serve images it builds at run time, which Exarust cannot tell
apart from absent ones (`shipped_materials_missing_images`: of 25 297 layers about 1 870 name
no shipped image, 900 of them reflection maps Exarust does not load; the
rest are mostly development scenes and apparel bases).

**Loader queue.** `RingQueue` keeps the one-free-slot rule and grows (the
texture ring by 128; the content rings, which block their producer in the
original, grow too). `promote` rotates to the front and marks the request;
an upload-pending key is found. `wants_to_yield` is uploads pending and the
last decoded request not promoted.

**Content stream.** `ContentStream::step` returns `Step::LevelObject`,
`Step::Requests { job, object, character }`, `Step::Streamed` or
`Step::Idle`. Streamed results go to their own ring that `pump` drains
before the attachments; requested characters stay for `fetch_character`.

## Open questions

- The compressed-format table entries the driver callback adds: the data
  rule for compressed words rests on the documented `0x82`/`0x92` split,
  not on the table itself (REA: the callback that fills the table).
- What `TextureDetail`'s handler clamps it to (settings handler, REA); the
  default 3 matches the retail user's file.
- The generator plugin's stopping rule for non-power-of-two images; Exarust
  stops where a dimension no longer halves exactly.
- Whether the opacity slot's sRGB upload (the original's word has no data
  bit) should replace Exarust's linear opacity maps: a picture change left
  for a capture.
- The creation-time priority bit (requests created with the promoted mark
  set by a global the loader reads): which caller sets it.

## Performance

The corpus conversion at the retail settings takes about 25 s of CPU in the
test profile where the previous loader took about 240 s with the same
colour spaces: 3 835 single-level BC4 images decoded on the CPU no longer
get a Kaiser chain, as the original never mips a compressed image without a
stored chain. Level loads lose that share of the mip generation PLAN.md
measured (about 2.5 s on `exanima01`); the cost is that those textures are
sampled from their base level only, as in the original.
