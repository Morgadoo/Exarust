# Design

The painter stays on the CPU. The GPU-side structures (job ring, surface
cache, scratch targets, program variants, pass uniforms) are reproduced as
data and index arithmetic with tests, because nothing in Exarust compiles
the original's programs; the paint itself follows the same rules.

A factory part is painted into its own target sized as its atlas region
(the factory atlas times the part's atlas scale, rounded to nearest),
which samples the shared atlas at the same density as the original. The
region quad draws `offset .. offset + size·crop` with texture coordinates
`0..crop` and the assembler maps mesh coordinates to `uv·size + offset`,
so a column of the per-part target is drawn when its centre lies left of
`crop·width`, and sampled at an unscaled u.

The geometry cache is keyed by the seeds, flags, stored selection and
materials: the part selection and the deform values do not depend on the
quality byte, so a condition change rolls the recipe again for the area
colours and repaints the cached meshes.
