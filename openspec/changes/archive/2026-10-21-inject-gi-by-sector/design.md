# Design

Extends `docs/RENDERING.md` (Voxel GI). No `docs/formats/` page changes:
the change reads the wall grid's plane A and the light records already
documented there.

## Units and order

The marks, the box test and the shaping test run in centimetres. Bevy's
view planes face inwards and are in metres. They are negated and scaled, so
the distance is ((x·nx + y·ny) + z·nz) + w and the reach is
(nx·hx + ny·hy) + nz·hz, as the instructions add them. A half extent takes
the normal's sign bit ORed into its own. The shaping distance sums
dy² + (dz² + dx²), and its threshold is the original's constant
(`0x33ffffd7`, just below `f32::EPSILON`). NaN offsets count as none, as the
unordered compare branches to zero. NaN plane distances keep the box.

## Cells and blocks

The focus cell is ⌊x · 100 · (1/600)⌋ + ⌊size/2⌋, rounded down: the
original's round helper sets MXCSR toward −∞. Cell i's centre is
((i + 0.5) − ⌊size/2⌋) × 600 cm. The block's first voxel column is
(s − ⌊size/2⌋) × 30 − 8. Exarust still keeps one block centred on the focus
cell and clamped inside the grid. The original lays its blocks out over the
built cells.

## Grouping

The sector pass reads the marks and floors as the original does (a sector
is considered when marked and its floor is below the block's depth). The
merge pass's fill estimate uses 32-bit products and a 64-bit sum, compared
strictly. Lights are kept for the grouping even when the injection does not
know them, and dropped from the draw, as the original drops a light without
its GI flag only after grouping.

## Frame cost

Per frame: at most 64 × 5 box tests of six planes, the grouping of 64
sectors on fixed twelve-light arrays, and one kept 42 KiB uniform buffer.
The pass loops over its group's lights only (twelve at most, before 32), and
unmarked voxels skip the propagation.

## Open questions

- Which convex volume the scene hands the marks (the camera frustum, or one
  narrowed by portals): REA on the scene's volume writer; Exarust uses the
  scene camera's frustum.
- The light list of a level without a grid and its order: REA on the
  scene's global light list. Exarust takes every point light by id, twelve
  at most.
- The level environment flag that also draws the top slice: REA on the
  environment record's loader.
- The picture against the retail game: a retail capture in a lit room
  while the camera turns (sectors leaving the view go dark).
