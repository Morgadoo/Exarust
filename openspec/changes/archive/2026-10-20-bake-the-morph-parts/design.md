# Design

## Behaviour

`0x1000da210(node, array, max)` sizes the array by the node's last child index plus one, zeroes it, then for each child computes `k = (name[0] − 0x30)·10 + (name[1] − 0x30)` from the two name bytes at `+0x140`, writes the child's position at `+0x94` into slot `k`, and reports the largest `k`. The bake calls it once for the model (into `ctl+0x70`, max at `ctl+0x68`) and once per part (into `part+0x18`).

The bake then selects, for each index from 0, the smallest ordinal in the suffix, starting the best at 100; a part whose ordinal is not below 100 stays in place. The ordinal is the integer parsed from the name's characters after the first (the original converts a Pascal substring starting at 2 through the engine's string and `atoi`). Records swap whole, 0x20 bytes each.

Every part's deltas then become its gathered positions minus the model's gathered positions. When the part count exceeds nine, parts 6, 7 and 8 additionally lose part 5's vertices and deltas element by element.

## Model

`morph_child_index(name)` and `gather_morph_children(&[(index, position)])` cover the gather. `bake_morph_parts(model, Vec<BakePart>)` covers the sort, the relative deltas and the part-5 fold. `BakePart.ordinal` is the precomputed name ordinal.

## Open questions

The engine's string conversion and the reallocator's capacity are not modelled; the gather sizes by the largest name index rather than the last child index, which is safe for the same data.

## Docs

`RF_Morphing` is **62%** (10,563 of 17,099 bytes). The headline stays 14.4% and 5.7%.
