# Design

**Where the values travel.** The filter radius and polygon factor depend on
the fit (half extent, softness, `c'`), which is per camera, while the
shaders only see what is bound to them. The light records' storage buffer
is already bound to every surface material (and so to their prepass
pipelines); its first element had three free lanes, and a same-size
rewrite of a shader buffer is a queue write, so updating the three floats
when they change costs nothing. One set of values serves every camera: the
active camera with the lowest order (the game's view). Another active
camera with a different fit would read the main view's radius and offset.

**The polygon offset.** wgpu's depth bias is fixed per pipeline, and the
factor changes with the half extent and with `c'`; quantising it into
pipeline variants would recompile the shadow pipelines whenever the sky
changes. The offset is written in the caster fragment instead:
`frag_depth = z − (factor · max(|∂z/∂x|, |∂z/∂y|) + units)`, the same
quantity GL computes (its spec allows the max for the gradient's length).
Every sun caster then runs a fragment stage that writes depth, which
gives up early depth testing in the sun pass; on `arenao3` the frame time
did not change measurably (CPU-bound, 12.4–12.6 ms p50 either way).

**The receivers.** Exarust's surface shaders already replace Bevy's point
shadow filter; the sun gets the same treatment, reading Bevy's directional
map and comparison sampler (linear, clamp to edge, as the original's) with
the cascade's sampling matrix. A light with more than one cascade or a zero
radius falls back to Bevy's filter, so `--bevy-sun-cascades` keeps working.

**Culling.** Bevy's prepass pipeline culls nothing by default. The original
culls back faces for plain casters (front faces for its node bit `0x800`,
which no shipped mesh sets) and nothing for alpha-tested ones. Two-sided
materials and cloth draws keep Exarust's existing rule (as its cube
casters do), a guess recorded in the code.
