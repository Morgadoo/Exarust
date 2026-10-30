# Design

The original draws each tracked mind as one 16-segment, 15 cm UV sphere
(poles on y, normals on) whose points it shapes once; Exarust already ports
that sphere for the mind effects (`uv_sphere`), so the draw unrolls its
triangle strips into a list instead of building a second sphere.

The original's object matrix copies the head node's 3×4 frame into a 4×4
whose first three rows are the node's axes (w 0) and last row its origin,
scales those rows by the record's three scales and multiplies by each
layer's view-projection: row vectors. Bevy's column-vector `Mat4` therefore
takes `sight_basis`'s rows as columns, scaled from centimetres to metres.

The noise object is made at 256², four octaves and amplitude 1, its two
plane-building weights set to 0.5 (the same pair the force noise and the
spirit screen set), then given four octaves, a 0.25 s period and target 1.
It ticks on `BodyTicks` with `EffectRoll`, as the force noise does, and
converts with `thaum::visuals::noise_bytes`.

The draw turns the depth test off and applies a state block whose blend
word is 0, which as read would blend ZERO/ZERO; the fragment writes an alpha
of `MindCol0.w · facing`, so Exarust blends by it in the transparent pass
with the depth compare Always. The blend factors and the cull state are
labelled guesses.

Without an anchor the system despawns stray spheres and returns before
making the sphere or the noise.
