# Design

The step end keeps a `(index, distance)` pair seeded with `(0, sentinel)`
and replaces it only on a strictly smaller horizontal squared distance, so
ties keep the earlier foot and a NaN distance never wins; with no winner
the first leg is planted, as the original's index register starts at 0.

The cadence target reuses `engine_math::round_to_i64` and narrows to
`i32`, which is the original's 64-bit conversion followed by a 32-bit
store. The pitch impulse reuses `engine_math::sign` rather than an inline
copy.
