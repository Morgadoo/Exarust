# Design

## Context

Streaming textures replace a placeholder under a handle after the first
frame, so everything that captured the placeholder's view (materials, cube
shadow bind groups) must be told. Bevy queues an asset-modified event only
when the mutable borrow is dereferenced, so a replacement that only
`get_mut`s the image would not rebind.

## Decisions

- **Per-request marks.** The loader's in-progress mark is keyed by request
  id, as the original keys it per record: a finished decode removes only the
  mark its own load set, so a newer load of the same texture keeps its mark.
- **Pump, decode, pump.** The loader yields while uploads wait. The frame
  collects finished decodes, pumps uploads, starts new decodes, and pumps
  again, so decodes are never held back by uploads the frame could have
  drained.
- **Rebinding.** The replaced image is written through the asset's mutable
  borrow, then the materials that hold it are touched and the cube-shadow
  bind groups are rebuilt.
- **BC4 as luminance.** The original uploads BC4 as luminance; Exarust
  uploads one red channel and the shaders repeat it, selected by a material
  flag. Emissive and unpacked specular maps are not greyed yet; the
  original greys them through the same flag, which is left for a later pass.
- **Player focus.** The arrival word picks the actor kind; a placed
  actor is the player when its operative record's kind matches, and a
  marker record is the player when its first word equals the current
  arrival variable and its second is all ones. Anything else is not the
  player, whatever its own kind.
- **Fetch.** The result ring is searched first on every fetch; only a
  miss falls back to the default.
- **Screenshot wait.** The flag-gated unattended screenshot waits for the
  loader's `is_idle`, capped at 1800 frames, so it never hangs a script on
  a stuck load.

## Risks

Streaming moves decode work off the main thread but not the upload; the
four-in-flight cap and the per-frame pump bound the frame cost. Load and
frame times were not measured in this review (no local builds).
