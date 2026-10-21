# Design

## Behaviour

The controller keeps an index block (element 0's data and count) and a part array with a last index. The rebuild allocates a new map, sets `new[i] = input[old[i]]`, frees the old map data and stores the new one.

For each part from 0 through the last index, a record with a non-zero flags word is skipped. Otherwise the stride is the component count (bits 16 through 19 of the kind) times the element size (bits 20 through 23: `0xA` is 8 bytes, `0xB` is 1, `0xF` is 4, any other code 0). The part has a count and a byte buffer. The new bytes are `new[j·stride + k] = old[input[j]·stride + k]` over the count and stride; the old buffer is freed and replaced.

The refresh always rebuilds the arrays, the index table and the vertex data (with 1). With bit 0 set it first purges (with 1, 1), then marks the owner dirty, stamps the owner's dirty word with 1, and if the bound object exists calls two of its virtuals (with the controller, then with 0).

## Model

`rebuild_morph_remap(map, input, parts, refresh, has_bound)` returns the new map, the rebuilt parts and a `MorphRefresh` naming the steps. `morph_element_size` and `morph_stride` expose the kind's arithmetic.

## Open questions

The bound object's two virtual slots and the purge guard's exact owner comparison are engine plumbing; only their occurrence is modelled.

## Docs

`RF_Morphing` is **27%** (4,686 of 17,099 bytes). The headline stays 14.4% and 5.7%.
