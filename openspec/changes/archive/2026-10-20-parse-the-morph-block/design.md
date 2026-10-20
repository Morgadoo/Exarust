# Design

## Behaviour

After the tag and unknown word, the first count gives a run of points (three words each). The owner's point count then skips a second run. The second count gives the number of entries; each entry holds the first count's points followed by the owner's points.

Counts must be non-negative and fit the remaining bytes. A short read or a foreign tag returns none.

## Model

`parse_morph_block(bytes, owner_points)` returns `MorphBlock { unknown, points_a, skipped, count_b, entries }`, where each entry holds `first` and `second` point vectors. A cursor reads little-endian words with bounds checks.

## Open questions

What the unknown word means is not established.

## Docs

This method is 412 bytes. `RF_Morphing` moves from 8% to about 11%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
