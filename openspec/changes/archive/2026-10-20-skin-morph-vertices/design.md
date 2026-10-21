# Design

## Behaviour

The controller's record array holds 28-byte records: a joint word, an offset vector and a base vector. The owner's joint table holds three words a joint, each an index into the owner's remap word array; the remapped word indexes the controller's point array, 12 bytes a point.

For each record the original takes the three points as the rows of a 3 × 3 basis and multiplies the record's offset by it: the output component j is `row0[j]·x + row1[j]·y + row2[j]·z` (the helper broadcasts each component and adds the three scaled rows). It subtracts the controller's base, applies the owner's rows the same way, adds the record's base and stores the point into the owner's array (12 bytes a vertex). The loop runs once per owner output word.

The local transform of the controller's base by the owner's rows that the function computes first is never read; the model leaves it out.

## Model

`apply_morph_vertices(base, owner_rows, joints, remap, points, vertices)` returns one point per `MorphVertex { joint, offset, base }`.

## Open questions

The owner's output count source is not needed for the point maths.

## Docs

`RF_Morphing` is **14%** (2,346 of 17,099 bytes). The headline stays 14.4% and 5.7%; the fresh sum stays about 15.5%.
