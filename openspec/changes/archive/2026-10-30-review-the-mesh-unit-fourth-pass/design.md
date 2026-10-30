# Design

## Swept-sphere travel

The original's query record has one float that is both the sweep length
and the best distance so far. `Face::swept_sphere` therefore takes one
`best` argument and uses it as the travel; `Mesh::swept_sphere_hit`
starts it at the sweep length and lowers it to each accepted hit. The
only observable difference from separate values is the grazing branch
(a sweep parallel to the plane uses its end point) and the approach
rejection's float edge; a floor-and-wall test pins the grazing case.

## Texture matrix

The loader keeps each resolved material's optional matrix in the model
cache by the same key as its handle; assembly rewrites the Bevy mesh's
`UV_0` before it is added, once per built group, so the cost is one
lookup per group and a pass over the coordinates only for materials that
ask for it. Apparel and the race skin are built at run time and store no
matrix (a labelled guess: the original reads the runtime object's flag).
