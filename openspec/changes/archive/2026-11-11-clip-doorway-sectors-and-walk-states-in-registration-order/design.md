# Design

Extends [docs/RENDERING.md](../../../../docs/RENDERING.md) (Blended order,
Sectors on the live path, Portal scissor).

## Scissor on Bevy's phases

- The transparent phase is a sorted phase drawn item by item into one
  render pass, so a scissor set by one item holds for the items after it.
  After the sector walk's sort, `insert_scissors` rebuilds the scene
  view's phase with a scissor item before every change of rectangle (the
  first scissored entry, a change of sector, the first entry after them
  without the bit). The items are transient (dropped by the next frame's
  phase), keyed by render-world entities of their own with a placeholder
  main entity, so the batcher finds no mesh for them and breaks the batch
  there; their batch range indexes the frame's rectangles.
- The rectangles are the original's (window pixels, rounded) doubled to
  the 2 × 2 samples of the super-sampled image and held inside the view's
  viewport (wgpu rejects a scissor outside the target).
- The opaque and alpha-tested phases are binned and drawn as multi-draws;
  a per-entry scissor would mean unbatching them, so they keep the
  frustum cull only.
- A single view sets no scissor, as the original's flush makes no call.

## Registration order

The sector's per-material list is appended at registration, in the same
order as the sector's mesh list, so a mesh's position in that list orders
it within its state; the rank is read back from the sort value (its
offset from the list's base in whole 2048 m spacings, the view depth
rounded off).
