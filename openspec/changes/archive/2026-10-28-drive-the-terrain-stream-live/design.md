# Design

The live state is a resource inserted when the terrain build finishes and
removed when the next level begins. The build keeps each tile's entity and
its blocks; the frame hook's insert sets the tile's `Visibility`, so nodes
are toggled per tile rather than relinked per sector.

A tile is marked built (`0x828`) when its entity is spawned, not when the
worker's geometry is done: the textures may load over later steps, and an
insert handled before the entity exists would leave the tile hidden.

The selection runs after the hook only when the hook ran (the delay branch
does nothing else). The priority slot has no direct caller and the frame
hook does not call it, so it is exposed as `reprioritise` rather than run
every frame, which would reset the ring and collapse the bake ages.

The light distance dots each outward plane's normal with the box's axes
(summed `(x + y) + z`), copies the component signs onto the half extents
for the support, and keeps the largest non-negative clearance from 0. The
plane list of a light is not in the pack: a point light's reach is modelled
as a box of 1.25 stored radii (a labelled guess), and the level's lights
stand in for the room's.

The hosted sets are about 235 MB at the default detail and nothing draws
them, so their allocation is opt-in.
