# Design

## Why a kept canvas gives the same bytes

A pixel's value is the blends of the batches covering it, in order, over a
transparent start. Comparing old and new batch lists index by index (extra
batches on either side count as changed), a pixel outside every box a
changed batch walks (old and new triangles, the same floor/ceil boxes the
rasteriser walks) is covered by the same batches in the same order and keeps
its value. The changed boxes are merged when they touch; when they would
cover half the canvas the whole canvas is redrawn. A new size, a new wheel
unit (the wheel texture is resampled), new texts on the bitmap-font path or
new sheets redraw everything.

## Row spans

Along a row each barycentric numerator is linear in the column. Each edge
keeps the columns on one side of its root, widened by a rounding bound
(16 eps times the products' magnitudes plus 4 eps times the area, over the
edge's slope) and two pixels, so every column the single-precision inside
test passes stays in the span. Non-finite numbers keep the whole box. The
per-pixel test is unchanged.

## Bands

The region's rows are split into at most one band per compute thread (16
rows at least). A band draws every batch clipped to its rows; pixels are
independent, so the order per pixel is kept.

## Alpha and layer

The canvas holds premultiplied colour. The handed-over copy divides each
colour by its alpha (rounded, capped at 255; alpha zero is black), only in
the redrawn regions. The layer is 910: over the heartbeat border (890), the
death fade (895) and the HUD (up to 900); under the splash (950), the level
transition (960) and the cursor (1000).
