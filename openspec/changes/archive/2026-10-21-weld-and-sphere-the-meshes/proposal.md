# Proposal

## Why

The original's mesh tools weld duplicate stream elements and compact the
channels before a character, item or grid mesh goes to render form, flip a
mesh's winding for mirrored grid pieces, and build the mind effects' UV
sphere. Exarust had the render-form conversion and the subdivision but none
of these. A review against the original's machine code also found three
subdivision rules that did not match.

Phase 5 (Visual and audio fidelity), the run-time mesh editing milestone
started by `smooth-the-meshes`.

## What Changes

- The weld: each stream that owns its channel sorts its elements by their
  first component with the original's quicksort, merges a later element
  into an earlier live one within reach (0.01 cm for positions, √1e-9 for
  texture coordinates), splices the incidence rings in the original's
  order and renumbers the survivors. Streams sharing the channel follow;
  the character pass folds a welded-away position's skin influences into
  its survivor and normalises them.
- The generic-format incidence (each corner through a reference list), the
  incidence rebuild and the winding flip.
- The UV sphere of the mind effects: its vertex writer (fast sine,
  double-precision cosine terms, texture scales, tangents, colours) and
  its strip layout with patched indices.
- Subdivision: a zero-length texture edge takes the plain branch of the
  fold test (the original's ordered compare); a mesh past the face limit
  is still refitted and converted.

## Out of scope

- Calling the weld from a loader: Exarust's skinning, weapon factory and
  grid environments have no editable-mesh stage yet.
- Drawing the mind effects (the sphere has no caller) and the effect
  shells.
- The voxeliser wrappers and the live-scene walk of the original's
  smoothing unit (no voxeliser or scene-object hierarchy in Exarust); the
  `msm` command's per-object flags.

## Evidence

REA (Ghidra 12.1.4) on the original's mesh-tools, subdivision and
polyhedra units, read against their assembly; the bytes of the unlisted
texture-coordinate key accessor and of the engine's fast sine, vector
subtract and normalise read from the executable. Real data: the smoothing
check over every mesh of `CharExt.rpk` (`EXANIMA_DIR`).
