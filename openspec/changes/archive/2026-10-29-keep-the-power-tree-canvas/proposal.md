# Proposal

## Why

docs/PLAN.md, Phase 6 (the power tree screen, thaumtree lane). On the GPU
check (RTX 3090, 1280x720) the open power tree ran at 19 FPS against 60 in
play. Every 33 ms it rasterised the whole window in software, about 41 ms of
shading (the full-window marks backdrop alone 22 ms), and uploaded a new
image. The canvas also had no layer, so the HUD drew over it, and it was
handed over with premultiplied colour to an image node that blends straight
alpha.

## What Changes

- The running screen keeps its canvas between redraws: the new draw is
  compared with the last batch by batch, and only the boxes the changed
  batches can touch are cleared and drawn again with every batch. The bytes
  equal a whole redraw.
- The redrawn rows are cut into bands drawn on the compute pool; each
  triangle row walks a conservative span instead of its whole box; the
  frames sheet's coarse level is made once when the sheets load.
- The image is re-uploaded only when redrawn, into the same asset.
- The canvas is handed to the image node with straight alpha, as the GUI
  draw path does, and draws at `GlobalZIndex` 910, over the HUD's layers.
- docs/THAUMATURGY.md and the exa_game README describe the kept canvas.

## Evidence

Measured in tests with the installed packs (the GPU check's frame, a hundred
frames after the key): a whole redraw 41 ms single-threaded at the test
profile; the kept canvas redraws about 102 000 pixels (the wheel and the link
glow) in 18 ms single-threaded, 2.5 to 4.5 ms with bands on the worker.
Pixel identity is tested against the whole-box rasteriser and against a
whole redraw of the real tree. No new retail behaviour: what is drawn is
unchanged.

## Out of scope

A GPU path for the tree's programs; throttling the wheel's shimmer below the
33 ms redraw.
