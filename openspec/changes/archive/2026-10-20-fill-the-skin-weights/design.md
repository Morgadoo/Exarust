# Design

## Behaviour

A record is four `u32` joints followed by four `f32` weights; joint 0 ends the list. The merge is the same four-slot merge the controller already uses: a scale below `1.19209e-7` does nothing; each source joint above 0 is added to an equal slot or appended to a free one.

Normalization walks the slots from 0, stops at the first zero joint, sets a negative weight to 0 (excluded from the sum), sums the others, records the last slot visited, and multiplies the weights through that slot by `1/sum` (1.0 at `0x10030d918` divided by the sum).

A target fill merges each neighbour record at `1/√(distance squared)` into an empty record, sums those weights, and scales the merged record by `1/sum`. With no neighbour the sum is 0 and the record is left alone.

A relaxation pass copies the records, halves each record's weights, merges each neighbour at `0.5·(1/√(distance squared))`, normalizes, and commits. The cook runs three passes.

## Model

`SkinRecord`, `merge_skin_record`, `scale_skin_record`, `normalize_skin_record`, `fill_skin_record(record, &[(SkinRecord, f32)])` and `relax_skin_records(records, neighbours)`. Distance arguments are squared distances; neighbours are vertex ids.

## Open questions

The target bitmap and the stream's registration are not in.

## Docs

`RF_Morphing` is **37%** (6,327 of 17,099 bytes). The headline stays 14.4% and 5.7%.
