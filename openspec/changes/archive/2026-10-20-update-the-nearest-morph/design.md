# Design

## Behaviour

Setup stores the target at `0x38` and the target's count at `0x48`, allocates an array at `0x40` of twelve bytes a count and another at `0x60`, then copies twelve bytes a count from the target's points into the first array.

The nearest update computes a distance through `0x100019ae0`. When it is strictly below the stored `0x14` the best point at `0x8` (twelve bytes), the extra at `0x10`, the distance and the marker at `0x18` are replaced. An unordered distance is not below and does not replace.

## Model

`setup_morph(count)` returns the count and the allocation and copy flags. `nearest_morph(distance, current, candidate, best)` returns the best point and whether it was replaced.

## Open questions

The distance, allocation and copy bodies are not in.

## Docs

These methods are 100 and 66 bytes. `RF_Morphing` moves from 0% to about 1%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
