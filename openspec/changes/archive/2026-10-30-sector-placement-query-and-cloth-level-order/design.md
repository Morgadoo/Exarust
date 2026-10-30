# Design

The original's grid sector steps every simulated object first (its step
ends by rebuilding the object's bound sphere), re-buckets it by that
bound, and in a later loop gathers for each object the bodies in the nine
cells around it, applies the post-collision planes, and then the static
records of those nine cells that pass the bound pre-test. Exarust's
`cloth::step` holds the integration, the body stage, the planes and the
level contact, so it is split before the level contact: the stage runs
`step_before_level`, gathers the static records from the stepped bound,
runs the level contact and then `finish_step` (sleep and reaction). The
body gather still uses the bound before the step (out of scope).

The placement query reads three bounds per body: the bound sphere of its
collision spheres, the sphere around its mesh box, and the mesh box
itself oriented by the body's matrix. `PlacementBody` carries the mesh
bound in the body's own space with its frame so the existing
`mesh_box_distance2` (the original's oriented routine, already used by
the all-pairs gate) serves both box gates. The two box compares jump on
when unordered, so a NaN distance passes, as in the pair stage.

The object-body mask is 0x804 (0x805 for a sleeping object): bits 4 and
0x800. The original sets both, with the asleep bit, on an item taken onto
the cursor; Exarust's `Ragdoll::disabled` stands for that state.
