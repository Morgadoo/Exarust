# Design

## Behaviour

The method returns at once when the counter at `0x1e0` is positive. Otherwise it builds the two footprint bounds (the same arithmetic as `path_extents`) and computes the new padded area `(x1 − x0 + 6) × (y1 − y0 + 6)`.

For each existing rectangle in order it takes the union (smallest low, largest high) and its padded area. When the new area plus the existing area is greater than the union area, it replaces that rectangle with the union and returns. When no rectangle takes it, the method appends the footprint; if the length has reached the capacity at `0xcc`, it adds eight and grows the storage through `0x10000b8e0`.

## Model

`FootprintList` holds the rectangles and the capacity. `plan_footprint(busy, list, rect)` returns `Blocked`, `Merged(index)` or `Appended(index)`. The arithmetic stays in 32 bits, as the original. The existing `Movers` queue is a different method: it keeps sixteen and drops the oldest, so it is left alone.

## Open questions

The caller that builds the nine local values and the extents is not in.

## Docs

This method is 929 bytes. `TPathFinder` moves from 26% to about 30%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
