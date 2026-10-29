# Proposal

## Why

docs/PLAN.md, Phase 2 (static world), the terrain paragraphs: the round-30
terrain5 lane stalled after one commit. That commit fixed the composite set
passes' mask window at the block level, modelled the set copy's draws and
the static terrain's deactivation, and wired a release step for the hosted
composite sets. The review checked each against the assembly and found the
tile's mask and height passes moved to the wrong node level, an invented
shared 256² working target, the quad's texcoord units and vertex misread as
a position and a colour, the extension list deactivated in reverse, and a
shader object described as a noise texture.

## What Changes

- The composite set passes upload and sample the block's 32-texel mask
  window holding 30 cells (the composite wrapper stores node level 3),
  whatever the set's level.
- The tile's mask and height passes run at the tile's root node: a
  256-texel window holding 240 cells, the whole palette slice, a 241/256
  height texcoord, drawn into the caller's 240² and 244² viewports; both
  upload the root node's mask window.
- A pass vertex is unit-0 palette coordinates, unit-1 mask coordinates and
  the emitted vertex in the viewport's unit square (the height pass's
  0.987705 is 241/244).
- A set copy draws one full-target quad per target the features keep, the
  vertex and unit-0 texcoord the same 0/1 corner, no unit-1 texcoord.
- The static terrain's deactivation (hooks, worker pause, one-time release
  registration, extensions in attach order) and the release's caches
  (program entries, a shared shader) are tested models.
- When a level's terrain is gone, the hosted composite sets, queued bakes
  and per-block draw states are dropped (wired, cheap).

## Evidence

REA and the objdump listing on the composite wrapper (node record with level
3 and the block's column and row), the block compositor, the tile's mask
and height passes and their caller (240×240 RGB and 244×244 red read-backs,
the fixed 4.016667 square), the immediate-draw helpers (two texcoord units,
then the emitting vertex call), the set-copy wrapper, the static terrain's
deactivation (forward extension loop, flag-gated second thread call, the
100 ms release registration) and the release (program and shader deletes
by the GL loader's names).

## Out of scope

The tile's mask and height read-backs are not run on the GPU (Exarust
builds the density and height images on the CPU); the set copy is not
wired (Exarust rebakes the block from its layers); the release's further
teardown outside the component; the composite path stays opt-in behind
`EXA_TERRAIN_SETS=1`.
