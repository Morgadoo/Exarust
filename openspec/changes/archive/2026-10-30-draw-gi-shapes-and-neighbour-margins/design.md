# Design

## Shapes in the compute pass

The original draws a quad per slice with each shape's own program. Exarust's
light step is one compute pass, so each shape is a uniform entry (program,
clipped box, first slice kept signed, the body's uniforms) and the pass
evaluates the matching body for the voxels inside the box. The bodies are
the `gi_shape` CPU bodies with `select`-based `min`/`max` (NaN kept as the
first argument, as the GL compare does) and the drivers' half-away
`round`. Sixteen shapes and eight points per multi-point shape are carried;
with none registered the loop is empty.

## Nine resident blocks

`GiTextureCache` keeps a block's light and scratch textures keyed by its
sector and size. A rebuilt block reuses them and owes no warm-up; occupancy
is always uploaded fresh. Only the active block steps, so a resident
neighbour's light is its last step's [adaptation: the original steps all
nine].

## Margin copies

After each step a small compute pass per resident neighbour samples the
neighbour's light at the destination texel's coordinates plus the table's
offset and writes it into the step's target, which then becomes the light.
The original copies before injecting; Exarust's copy lands after the step's
clear, so the next step reads it (one step later).

## Slot move and uniforms

`plan_light_slots` is the do-while scan; the sky light is Exarust's only
matrix-shadow light, so groups keep their order until spot lights exist.
The uniforms keep the original's operation order in centimetres.
