## Why

TRenderer's gap (round 146): the per-sector and per-entry pixel scissor,
the per-quadrant sub-frame regions of the super-sampled view, and the
registration order within a state (Exarust kept Bevy's distance order).

## Evidence

REA on the renderer's list walks and the deferred draw queue:

- The scene pass walks the alpha-tested, opaque, decal and blended lists
  each per sector. For a sector whose visibility word meets the mask the
  walk binds the sector, and when the sector keeps a rectangle it either
  sets the region (single view) or, super-sampled, builds a second set of
  four sub-frame viewports (the rectangle offset by each sub-frame's
  origin, rows flipped) and projections cropped to the rectangle; then it
  walks the list's states and, for each, the sector's per-material entry
  list in registration order, queuing every visible entry with the
  `0x1000_0000` bit of its word as a flag.
- The queue's flush, super-sampled, switches the draw to the cropped set
  on each change of the flag (the mesh draw picks its viewport and
  projection by it, per sub-frame); single view, it notes the change and
  makes no call (compiled out), and the region alone never turns the
  scissor test on.
- The rectangle is the rounded screen bounds of the first vertex of each
  doorway frustum face, through the view's window projection.

## What Changes

- exa_render: `sector_walk` re-sorts the decal and blended lists by
  (list, sector, rank, registration position) from `DrawSlot`s, and on a
  super-sampled scene view inserts transient scissor items into the
  transparent phase that set the render pass's scissor to a doorway
  sector's rectangle (doubled, held in the viewport) for its entries with
  the bit, and back to the whole view after them.
- exa_world: `scene_live` writes each mesh's sector and list position, and
  every frame the doorway sectors' rectangles (window pixels, the
  original's projection) and the meshes with the bit.
- exarust: `--no-sector-scissor`, `--no-registration-order`.
