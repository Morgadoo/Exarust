# Design

## Behaviour

The scale must be at least the epsilon. The highest slot with a positive id is found from slot 3 down. Each source entry is read in order; an id below one ends the merge. An id equal to a current slot adds `source weight × scale` to that slot. A new id appends at the highest used slot plus one while that is below 3, and later entries see the new bound.

## Model

`MorphWeights { ids: [i32; 4], weights: [f32; 4] }` and `merge_morph_weights(current, source_ids, source_weights, scale)` returning the merged slots. The epsilon is `f32::from_bits(0x33ff_ffd7)`.

## Open questions

The caller is not in.

## Docs

This method is 228 bytes. `RF_Morphing` moves from 6% to about 7%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
