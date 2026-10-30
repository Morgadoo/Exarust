# Design

`composite::tex_pos` mirrors the instruction stream: `1i32.wrapping_shl`
for both powers of two (the instruction masks the count), `wrapping_sub`
then `& 31` for the shift, the logical shift on the unsigned index followed
by a sign-extending widening, and an `i64` subtraction converted to `f32`
(round to nearest, as the conversion instruction does). The lane wrote the
difference as a byte subtraction; with the five-bit mask the counts agree,
but the original subtracts 32-bit fields, so the comment says so.

The live callers in `composite_draw` pass the block's own level (3), equal
to the terrain's, so the running game's coordinate is unchanged.
