# Proposal

## Why

Exarust decoded every `.rfi` with one fixed sampler (repeat, trilinear,
16× anisotropy), uploaded every stored level, appended a Kaiser chain to
any single-level power-of-two image decoded on the CPU, and had no model
of the original's texture manager, texture loader queue or content stream
thread. The original's upload routine builds a creation word from the
material and the image header, picks the sampler, the colour space and the
first uploaded level from it, never mips a compressed image without a
stored chain, and uploads a 1×1 grey placeholder for an image it cannot
read. Its loader thread and stream thread feed the main thread through
ordered rings with a priority path.

## What Changes

- The texture creation word: the material's load flags or-ed with the
  texture quality mask, the per-slot data and bias bits, and the image's
  header flags from bit 16 up; the sampler (filters, wrap, anisotropy,
  LOD bias) read from it.
- The texture quality table and the software-format fallback.
- The colour space: data bit, uncompressed format bit 16, compressed
  `0x82`/`0x92` words.
- The first uploaded level from the detail class and `TextureDetail`, and
  the device's largest texture side.
- A compressed image without a stored chain keeps its base and loses its
  mips; the quality mask gates every chain.
- The grey placeholder for an image that cannot be read.
- The texture manager: stored names (sixteen bytes, cut at the last dot,
  folded), the manager's order, the first request's word, references.
- The texture loader queue (ring, promotion, yield, priority load, upload
  pass) and the content stream scheduler (pass order, rings, pump, waits,
  fetches), not yet driven by the runtime.

## Capabilities

### New Capabilities

- `content-streaming`: the texture loader queue and the content stream
  scheduler.

### Modified Capabilities

- `texture-decoding`: the creation word, colour space, first uploaded
  level, placeholder and texture manager; stored levels from the first
  uploaded one.
- `rendering`: which textures get the Kaiser chain.

## Impact

- `exa_formats::rfi` (header flag word, first level), `exa_formats::material`
  (texture load flags, subsurface colour); `exa_render::mipmap` (a chain from
  a later level); `exa_assets::texture`, `texture_cache` (new), `stream`
  (new), `content_stream` (new), `model` (material slots, stored-name
  lookup).
- Docs: `docs/formats/RFI.md`, `docs/RENDERING.md` (Colour), the
  `exa_assets`, `exa_formats` and `exa_render` READMEs.
- Evidence: reverse engineering of `Exanima.exe` (REA, Ghidra, pseudo-code
  and assembly): the upload routine, the wrap, filter and anisotropy
  setters, the format lookup and table, the texture quality and pixel
  format handlers, the material loader's slot words, the manager's search,
  insert and name helpers, the loader thread, promotion, priority load and
  push, the stream thread loop, its pump, waits, fetches, rings and unit
  init. Corpus: every shipped image converts at the retail settings (8 839
  images: classes 200/2/8 616/21, 3 324 with a stored chain, 5 364
  compressed without one, 21 at a later level at `TextureDetail = 3`), and
  a survey of material layers whose image is not shipped.
- Out of scope: driving the loader queue and the content scheduler from the
  level loader, materials and characters (they stay synchronous); the
  loader-plugin registry and run-time image names; the runtime format table
  and BC4 uploads (BC4 still decodes on the CPU); cube and 3D targets in the
  upload; drawing the placeholder for unshipped images; applying the LOD
  bias in the shader.

## Phase

Phase 2, static world (texture loading and level content), with the stream
scheduler for the level loader's later move off the main thread.
