# Design

`head_extra` takes every worn piece of a body that has an eleventh-slot
node (the caller's responsibility; every shipped humanoid has one) and the
record's physique. Only the head mark filters; display bytes are integers
so the strict compare is the original's COMISS without a NaN case. The
result is `(physique − 0.5) × 8 + height`, the subtraction and product
first, as the instructions do.

The mark is read as "any of the eight bytes non-zero", matching the
qword compare. What the qword holds is not established and is labelled a
guess in code.

The coverage cap uses `if c > 1.0 { 1.0 } else { c }` instead of
`f32::min`, which would return 1.0 for NaN.
