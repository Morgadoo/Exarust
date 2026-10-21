# Design

`texture_settings(base, live)` is the pure rule: `base` with
`with_quality(live.texture_quality)`, the live texture detail (negative
values as 0) and the live super-sampling level. The options system, which
already pushes the shadow, multisample and super-sampling options, writes
it into `ModelCache::textures` when it differs, so the cache's change flag
only fires on a real change.

## Open questions and how they were answered

- Whether a texture slider reloads the textures already on the device: not
  traced; Exarust applies the settings to later decodes only.

No format page changes; the page extended is `docs/UI.md`.
