# Design

## Behaviour

The controller's record array at `this+0x20` holds 28-byte records: a joint word at 0, an offset vector at `0x4` and a base vector at `0x10`. The owner's joint table at `+0x290` holds three words a joint, each an index into the owner's remap at `+0x270`'s word array; the remapped word indexes the controller's point array at `this+0x60`, 12 bytes a point.

For each record the original takes the three points as the rows of a 3 × 3 basis and multiplies the record's offset by it: the output component j is `row0[j]·x + row1[j]·y + row2[j]·z` (the helper `0x100018820` broadcasts each component and adds the three scaled rows). It subtracts the controller's base at `this+0x78` (`0x1000191a0`), applies the owner's rows at `+0x40` the same way (`0x100018b00`), adds the record's base (`0x1000190b0`) and stores the point into the owner's array (`+0x280`, 12 bytes a vertex). The loop runs once per owner output word.

The local `transform(this+0x78, owner+0x40)` the function computes first is never read; the model leaves it out.

## Model

`apply_morph_vertices(base, owner_rows, joints, remap, points, vertices)` returns one point per `MorphVertex { joint, offset, base }`.

## Open questions

The owner's output count source is not needed for the point maths.

## Docs

`RF_Morphing` is **14%** (2,346 of 17,099 bytes). The headline stays 14.4% and 5.7%; the fresh sum stays about 15.5%.
