# Design

## Culling append

The generator appends each worn garment's chosen slot nodes through the
culling append with the slot's cull bits over that garment's visible
mask, and the body's slots afterwards through the plain append. The
merge keeps the cull test on the part's per-triangle flags and appends
only the vertices a kept triangle uses (reviewed earlier: numbering only).
The flags travel with each built group from the loader to
`merge_dressed`, so the rule runs on the drawn garments.

## Joint capture

`capture_joints` is a pure walk over any node type, so the Bevy system
passes `Children` and `BodyPart` lookups. Exarust's bone numbers come
from node names; a bone-0 node or an entity without a body part passes
through (labelled guess), so a body whose top node is numbered `00`
still binds.

## Decomposition

The mask target is the job's size, so the passes read the mask texel at
their own texel and `textureOffset` neighbours directly; the mask is
stored as a byte (R8 target) and read with green and blue zero, as the
sampler fills a one-channel texture. The eight maps are decoded through
the texel cache only for a job whose amount is above zero; a map that
does not decode leaves the pass out.
