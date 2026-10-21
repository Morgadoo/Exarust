# Proposal

## Why

The original's image codec sizes, copies, creates and converts images by
rules Exarust only partly had; its light nodes flicker and fade; its smoke
and mist use their own program; its 2-D immediate-draw helpers batch
quads in pixel coordinates; its material factory keeps a type table and a
priority-ordered list of builders; its record loader builds a
texture-coordinate matrix. A first pass at these was reviewed against the
original's machine code and much of it was found invented, mislabelled or
duplicated; this change keeps what matches and corrects or removes the
rest.

Phase 5 (Visual and audio fidelity), the renderer-fidelity milestone.

## What Changes

- Images: a level's size from the format word's fields; a face's size with
  the chain only when the flags say so (never times six); the converters'
  pixel count with the chain and six cube faces; a packed stream unpacking
  to its data size; copies stepping faces by the data size; the create and
  render-target calls; resize and release; the whole-image conversion
  through the registered formats and their fallbacks.
- Light nodes: the light body's flicker and fade parameters are decoded;
  the flicker and fade run once a frame at the scene's capped frame time
  with the engine's generator in the original's double-precision steps; a
  lever's scale; the parent mesh's first part takes the emissive share on
  a material copy of its own.
- Smoke and mist: the `shadesmk_f` program line for line and its `UVOfst`.
- The 2-D batcher's data path, its projections, built-in vertex programs
  and attribute locations.
- The material factory's type table with its reset, the factory list's
  ordering, the normal-map predicate; the loader's texture-coordinate
  matrix; every word of the material block kept.
- Terrain blocks carry the original's flat slab bounds.
- Removed: an invented image loader registry and pixel-pair table, a
  duplicate material-to-state builder, node-life systems that rewrote
  shared materials and part visibility, light aliases, an offscreen icon
  renderer, scene-content stand-ins, kd-tree thread and query stand-ins
  and a static-collider queue change that lost one mesh per full batch.

## Out of scope

- Spawning smoke meshes or drawing 2-D batches (their callers are not
  ported); the original's fog and dither in the smoke program.
- The material factory's builder dispatch and per-material data, light
  aliases, per-light shadow map sizes, the icon renderer, the background
  kd-tree builder thread and its query install, the holder-scene walk.
- Fixes to the wired material path in `exa_assets` that the review found
  (late opaque rewrite, premultiply rule, emissive and shadow-cast
  conditions); they need their own change and a visual check.

## Evidence

REA (Ghidra 12.1.4) on the original's image codec, light source, scene
node, geometry, material, smoke, immediate-draw, material factory, icon
and kd-tree units, read against their assembly; constants read from the
executable (the quad index and texture-coordinate templates, the
projection template, the built-in vertex programs and attribute names,
the GL entry points, the generator's scale); the light body loader's word
order from its disassembly; the shipped `shadesmk_f` text. Real data: the
shipped environment cube's six faces (`EXANIMA_DIR`).
