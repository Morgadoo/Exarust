# Proposal

## Why

docs/PLAN.md, Phase 2 (static world), the terrain paragraphs: when a block
turns high resolution and no high set is free, it takes the high set of a
block that is now low; that holder, unless blank, gets a low set carrying a
copy of its composite (the original's set copy, which blits each target with
the built-in copy program). Exarust modelled the copy's draws but rebaked the
holder from its layers. The round-35 terrain6 lane stalled after three
commits; this change reviews and completes them.

## What Changes

- The selector records each copy it orders (holder, source high set,
  destination low set), in both steal paths, in order.
- `exa_render` hosts a copy queue and a full-screen copy pipeline
  (`terrain_copy.wesl`, source texture and sampler) blitting colour, then
  each of normal, gloss and displacement both sets have.
- `terrain::composite_draw` queues a copy only for a holder that draws its
  baked high set (current colour target), with no bake in flight, and moves
  it onto the low set the same frame; others take the layer rebake.
- Copies run before the frame's bakes, so the block that took the high set
  bakes into it after the copy has read it (the original's order).
- Review correction: the lane changed the far cut-off so a NaN distance
  dropped the block. The far compare jumps on parity to the keep branch,
  so a NaN block stays selected at low resolution (the existing rule).

## Evidence

The selection's steal paths call the set copy with the holder before
reassigning its set, then bake the taker; the far cut-off's parity jump
targets the keep branch; the near cut-off's parity jump targets the low
branch. The copy routine binds the destination targets in order and issues
one copy draw per target with no mipmap pass.

## Out of scope

Exact source filtering of the copy (linear here), copies whose sets are not
yet on the GPU (they wait a frame, the bake order then not guaranteed), and
the remaining terrain routines listed in the pack.
