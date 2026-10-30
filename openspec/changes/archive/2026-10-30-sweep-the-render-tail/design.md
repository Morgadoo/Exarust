# Design

## Free camera

The key vector subtracts and adds the camera frame's rows in A, D, S, W
order and then adds the controller's movement vector; when asked it zeroes
the height and normalises past 2⁻²³ with the engine's normalise. The
tether holds an axis at `sign(offset)·limit + anchor` with the engine's
sign helper, or scales a radial offset onto the radius. The drift and the
zoom spring keep the original's precision: single-precision sums, double
compares and the double 0.998, with the NaN branches the JP jumps take.
The lift walks every sector (the wall probe alone reads the visible bit)
and reuses `exa_physics::intersect::{spheres_separated,
point_obb_distance_squared, segment_sphere}` and
`nav_camera::{mesh_in_reach, lift_out_of_geometry}`, which already port
the engine's routines.

## Light update and alias

The light's derive and update read the node's flags word: `0x20000` blacks
the second colour and skips the running sum, `0x10000` blacks the GI
colour; the factor is the record's multiplier. The constants are main's
(`NODE_NO_GI`, `NODE_NO_SECOND_COLOUR`), shared with `DerivedLight`. The
alias's spot term is `max((exp(ln 2·(t·cone)) − 1)·exponent, 0)` with the
fast logarithm and exponential `exa_assets::vertex_cache` already holds,
and its constants come from the same functions; the flat factor is the
runtime's double sine or cosine. The face mask's threshold is the node's
bounding radius.

## Matrices and fit

The light view matrix stores right, up and dir as the rows of a
column-major matrix with `right = dir × up_hint`. The scene fit divides by
the runtime sine and puts the moved node at the box's centre plus the old
direction times the distance.

## Kept off the running path

The prepass keeps main's alpha cut for every masked surface, because the
terrain's plant set and the generated apparel and factory materials are
masked without the record's alpha-tested bit, and the cube caster cuts
them (`ExaMaterial::cuts_shadow`). The texture loader reads the parsed
file in place.
