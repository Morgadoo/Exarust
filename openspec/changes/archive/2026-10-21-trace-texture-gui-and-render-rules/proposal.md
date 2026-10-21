# Proposal

## Why

The original's image converters, texture bind cache, render-list choice,
deferred draw queue, still capture, vertex layouts, post-chain fades, GUI
skin globals, textured skin and table view had no counterpart in Exarust, or
only a partial one. A first pass over them was reviewed against the recorded
instructions; several rules were invented or misread, and are corrected here.

## What Changes

- Image formats: the blue-first and ZYX uncompressed orders, `DXT1a`,
  `ALPHATC`, `LATC` and `DXT5n` decode through the registered converter
  family, using the original's integer block arithmetic.
- A per-unit texture bind cache and the 16-bit hundredths conversion of
  texture parameters.
- The renderer's list choice from a material's flag word, its lookup of an
  existing state and the flags a released state keeps.
- The deferred draw queue (capacity, buckets by light count, insertion
  order) and the still-image capture plan (eight jittered samples, blend
  weights, half-size downsample).
- The planar vertex streams and CPU index batches.
- The post chain's screen and desaturation fades.
- The GUI blend switch with its one-word cache.
- The GUI skin: the two base hooks, the globals (default skin, activation,
  unclamped alpha and its rounded byte), the skin texture bind rule, the
  atlas size and the textured skin's setup, frame and shaders.
- The table view: hover reply, default row and centre, caret and anchor
  while editing, the choice list, the header inset and the shadow below the
  header.

## Capabilities

### New Capabilities

### Modified Capabilities

- `texture-decoding`: uncompressed channel orders, the registered
  compressed variants, the bind cache.
- `rendering`: list choice, draw queue, capture, vertex layouts, fades.
- `user-interface`: GUI blend cache, skin globals and textured skin, table
  view details.

## Impact

- `exa_formats::{pixel, rfi}`, `exa_assets::texture`,
  `exa_render::{passes, queue, capture, geometry, post}`,
  `exa_ui::{blend, skin, table_view, text_layout}` and their READMEs;
  `docs/RENDERING.md`, `docs/UI.md`, `docs/formats/RFI.md`.
- Evidence: the recorded decompilation and assembly of the image
  converters, the texture class, the renderer's list and queue routines, the
  geometry class, the post-process unit, the GUI graphics and skin units and
  the table view; constants read from the executable's data (jitter table,
  blend table, shader sources). Every numeric rule was re-checked against the
  assembly, not the decompiler's output.
- Wired: the image decoders are reached through the existing texture
  loaders. Everything else is tested pure rules that the running renderer and
  UI do not yet call (Bevy's allocator, pipelines and blend states replace
  them): the bind cache, list choice, queue, capture, geometry, fades, blend
  cache, skin and the table view's choice list.
- Out of scope: driving the capture or fades from the running renderer, an
  image encoder, the choice list's own control, the owner-callback query of
  the table, a skin object model for the Bevy UI, and the GPU readback node
  of the screen-quad snippets.

## Phase

Phase 5, visual fidelity: textures, render order, post chain and the GUI
skin the interface is drawn with.
