# Design

## Behaviour

The first pass adds `source[i] × weight` to `destination[i]` for the point count at `0x48`, using the array at `0x60` and the weight at the `0x50` header. The second pass does the same for each of the `0x4c` entries at `0x50`, stride 32, using the entry's points and its weight at offset 0. Both are the same weighted add in opposite directions.

## Model

`blend_morph(destination, source, weight)` returns the pairwise sums, stopping at the shorter list.

## Open questions

The two vector calls are not in.

## Docs

This method is 355 bytes. `RF_Morphing` moves from 3% to about 5%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
