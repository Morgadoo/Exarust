# Proposal

## Why

A cheaper agent ported the cloth driver, the keyframe curves, the AI
perception bases, the character decomposition material, the character
creation sliders, the GUI character preview and the level pathfinder
installation from decompiler output. A review against the original's
machine code found rules that did not match (NaN and loop-wrap handling in
the curves, the slerp's dot order and trigonometry, the yaw of the preview,
the decomposition's unit map and layer cap, the centred slider's signed
zero, a duplicated tone scale) and a cloth driver that mutated a mesh asset
shared between copies of a model. This change records the corrected rules.

Phase 5 (Visual and audio fidelity) for the decomposition material, the
creation sliders and the preview; the cloth driver and the curves serve
Phase 3 (Bodies) and the pathfinder and perception bases Phase 6 (AI).

## What Changes

- Cloth: each cloth node takes a private copy of its mesh asset; a node
  whose mesh cannot be read is skipped for good; the importer gate needs
  the marker and the scene's cloth flag.
- Engine maths: the fast sine and cosine together; the keyframe curves'
  NaN, loop wrap and ordered compares; the slerp with the engine's dot
  order, threshold and fast sine.
- Navigation: the abstract pathfinder's neutral answers and the install
  state (level grid, or the 64 x 64 fallback while building or absent).
- Character creation: the centred slider's weight keeps the signed zero;
  the tone grid's blend target has a single definition; the decomposition
  passes follow the shipped programs (unit map, four-layer normal cap).
- Preview: the character-view tick uses the engine's arc-cosine yaw, the
  double-precision products and wrap, the sign-bit signum, the fast sine
  and cosine for the cursor direction and the stored aim vector for the
  head blend.

## Out of scope

- A caller for the decomposition passes (no corpse path yet), the cloth
  blood painter, swept-hit and pull helpers, and the triangle-to-free-vertex
  binding and swept-triangle gather of the cloth units.
- The cloth pin rule (a labelled guess), the preview's pointer point
  (Exarust intersects the mouse ray with a plane), and the source of the
  stored aim vector.
- The approximate reciprocal square root of the engine's normalise (Exarust
  uses the exact one).
- `nav::tests::shipped_levels`, which already fails on the integration
  branch (a floor count one over).

## Evidence

REA (Ghidra 12.1.4) assembly of the original's cloth, keyframe, character
view, morph, decomposition and pathfinding units; the engine's vector
helpers, signum, yaw, arc-cosine and fast sine and cosine read from the
executable; the decomposition programs dumped from `textures.flb` with
`exa-tool shader`. Hand-worked numbers in the unit tests.
