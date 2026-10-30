# Design

The draw is `slots[(count · state) >> 32]` where `count` is the number of
non-zero slots among the row's three and `state` the LCG after one step.
Exarust's rows are slices that may hold fewer than three ids; a missing
slot reads as zero, as the original's zero-filled table does. A drawn
zero is returned as is (no weapon) rather than mapped to `None`, so the
caller sees exactly what the original passes on; the live hire path in
`arena_records` already treats a zero id as no weapon.
