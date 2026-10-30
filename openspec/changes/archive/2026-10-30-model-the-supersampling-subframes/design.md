# Design

`subframe` keeps the engine's row-major `[[f32; 4]; 4]` because the
original uploads its matrices without a transpose and the summation order of
its products matters; the projection reuses `exa_physics::engine_math::
frustum`, whose column-major array has the same bytes. The view keeps the
one level table: `subframe` reads sample counts, jitters and the per-quarter
state from `view::SupersampleLevel` instead of a second copy, and the
quarter's `Global` offset, tap fraction and resolve weight stay in
`SupersampleLevel::schedule`.

The jitter read past a level's own run is kept because the original does
it: the 15-sample level's last sub-frame takes the 24-sample level's first
entry, and the 66-sample level's last one runs past the table's zero padding
into the shadow-tap table the executable stores next to it; `view` returns
those two numbers from `shader_bindings::SHADOW_TAPS`.

The depth-lane rewrite reads the first pass's lane each time a flagged
geometry is drawn, so successive flagged draws in one quarter compound, as
the engine's in-place write does; `Subframes::apply_depth_offset` mirrors
that.

The per-draw emissive bit lives on the render object's own word, which is
distinct from the drawing state's material key (that key is taken from the
material when it is bound); the scale it selects is the render object's own.
