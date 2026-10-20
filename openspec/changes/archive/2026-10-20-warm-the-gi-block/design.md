# Design

## Why the swap was cold

`exa_world::gi_volume` inserts a new `GiVolume` when the block build finishes.
`exa_render::gi::create_textures` saw it as the active volume, allocated an
empty RGB light volume, and `give_materials_gi` pointed every surface at it.
The render world then ran 16 propagation steps a frame. For the first ~40
frames the block was mostly black: direct light was still shadowed by the
walls and rocks, and the GI ambient that should fill those shadows had not
spread yet. The visible result was hard black patches in shadow and a soft
round halo of already-lit voxels.

## Holding the old block

The active `GiTextures` stays in place. A rebuild allocates a second
`GiPendingTextures` and the render world's bind group and propagation pass
target it while it warms. `swap_warm_textures` runs in the main-world
`PostUpdate` chain after `create_textures`; once `GiWarmUp::finished` reports
that the pending serial has run all of its steps, it replaces `GiTextures`
with the pending textures and removes `GiPendingTextures`. Every material is
then rewritten to the new light handle and origin because
`give_materials_gi` now tests `is_changed`, not only `is_added`.

The first block still uses the active path, so the loading screen's existing
wait keeps its behaviour: the first frame after loading is lit.

While a rebuilt block warms, a point outside the old block falls back to the
environment ambient, as it did outside any GI block; there is no cold black
volume to sample. The old block is at most one rebuild (two sectors) behind,
so it still covers the current view.

## Level changes

`collect_source` removes `GiTextures` and `GiPendingTextures` together with
the old `GiVolume`. The next level's first block takes the active path again.
Without this, the first block of a new level would be treated as a rebuild and
the old level's GI would stay bound until the new one warmed.

## Tests

`a_rebuilt_block_waits_for_its_warm_up` inserts an active block and a new
volume; after a frame the active serial is unchanged and a pending block
exists. Reporting the pending serial's full warm-up makes the next frame swap
it and remove the pending resource. `the_first_block_is_used_while_it_warms`
checks the loading path: no active block means the first volume becomes active
immediately.
