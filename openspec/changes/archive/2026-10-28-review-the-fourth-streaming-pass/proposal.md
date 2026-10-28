# Proposal

## Why

The fourth DeepSeek pass on the content stream and texture units (lane
`streaming4`, docs/PLAN.md Phase 2, Static world: texture loading and the
content stream) added the image-reader dispatch, the pack and entry a queued
load keeps, the terrain path's live lookup and stream-first load, grey BC4
specular and emissive maps, streamed fire textures with their flame
material rebound, and the character builders' saved chunks, size words,
request flags and saved AI pose, and the placed object's root node flags.
Its review checked each rule against the original's instructions and found
three wrong: the size word constant is 1/65535, not 65537/2^32 (the value
was right, its reasoning and docs were not); only the streamed builder of a
body that is not dead defaults an unset size pair, while the code defaulted
it for every builder; and the extension lookup kept a leading-dot name
without an extension and truncated a long extension where the original
copies none.

## What Changes

- content-streaming: new Character size words, Character request flags and
  Saved character chunks; Placed object flags names the copied field.
- texture-decoding: new Image reader dispatch; Single-channel diffuse also
  covers the specular and emissive slots.
- Code: `SIZE_WORD_SCALE` written as 1/65535, `streamed_size_words` added
  and `character_scales` no longer defaulting, `NO_AI_START` replacing
  `KEEP_SAVED_AI`, `image_format::extension_of` following the original's
  copy rule and the message keeping the name's case.

## Evidence

The two character builders, the placed-object reader, the streamed
character handoff that stores the saved blob, the texture create routines,
the image dispatch with its extension extractor, its case-folding 16-byte
compare and the registration of the one "rfi" reader in the full
disassembly; the size constant read from the executable's data.

## Out of scope

A Bevy driver for the content stream (the character and placed-object
rules have no caller in the game yet), the terrain material path calling
the stream-first load, provider create variants, the runtime reader
registry, and the fast-load flag.
