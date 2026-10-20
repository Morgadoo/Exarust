# Design

## Behaviour

The ring slot at 0xdc advances by one modulo 16. For each axis, a radius is the sum of three absolute local values times the three extents; axis 0 reads local 0, 3 and 6 and axis 1 reads local 2, 5 and 8. The coordinate is the first of those local values.

The low bound is `axis >> 1` plus `round((coordinate − radius) × inverse − 0.5)`, floored at 3. The high bound is the same with the radius added and 0.5 added, capped at `axis − 4`. The two bounds are stored per slot.

## Open questions

The transform that builds the nine local values is not in.

## Docs

This method is 413 bytes. `TPathFinder` moves from 25% to about 26%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
