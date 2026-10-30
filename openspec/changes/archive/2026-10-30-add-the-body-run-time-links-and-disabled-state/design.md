# Design

## Link adder

The original's adder appends a record to the body's own link array; in
Exarust a body's links are one array split by part, so the new link goes at
the end of its part's range and the later ranges and the leg-limit index
shift. The original stores the slack squared in centimetres and compares
the raw slack with the epsilon before squaring; Exarust's laws take the
distance in metres, so the comparison scales the slack to centimetres and
the distance is kept. The parent's share is stored negated by the
original and kept positive here, as every Exarust body link keeps it (the
solver negates it).

## Third particle set

The post-step rolls the previous set into the third and the current into
the previous, so between ticks the third set is where the last post-step
but one left the body (one tick ago) and mid-step it is two ticks ago.
Exarust's tick start is that same set, so the post-step copies it. The
sleeping branch rolls once more on the first tick of sleep.

## Disabled state

The cursor carry sets the disabled bit with the asleep bits, so both of
the sector's level passes skip such a body; Exarust's flag stands for the
whole carry state and skips its level detection outright.

## Clone

The original's clone copies the rest particles inline and shares the
sphere array with its template until the unshare routine, which nothing
calls; Rust's copying clone gives the same observable result.
