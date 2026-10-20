# Design

## Behaviour

The controller keeps an index block at `this+0x270` (element 0's data and count at `this+0x288`) and parts at `this+0x280` with a last index at `this+0x278`. The rebuild allocates a new map, sets `new[i] = input[old[i]]`, frees the old map data and stores the new one.

For each part from 0 through the last index, a record with a non-zero flags word at `+8` is skipped. Otherwise the stride is the component count (bits 16 through 19 of the kind at `+4`) times the element size (bits 20 through 23: `0xA` is 8 bytes, `0xB` is 1, `0xF` is 4, any other code 0). The part's count is at `+0xc` and its bytes at `+0x10`. The new bytes are `new[j·stride + k] = old[input[j]·stride + k]` over the count and stride; the old buffer is freed and replaced.

The refresh always rebuilds the arrays (`0x100063e90`), the index table (`0x100067380`) and the vertex data (`0x100068730` with 1). With bit 0 set it first purges (`0x1000640c0` with 1, 1), then marks the owner dirty (`0x100051110`), stamps `+0xe4` with 1, and if the bound object at `+0x2e0` exists calls its virtuals `0xc8` (with the controller) and `0xd0` (with 0).

## Model

`rebuild_morph_remap(map, input, parts, refresh, has_bound)` returns the new map, the rebuilt parts and a `MorphRefresh` naming the steps. `morph_element_size` and `morph_stride` expose the kind's arithmetic.

## Open questions

The bound object's two virtual slots and the purge guard's exact owner comparison are engine plumbing; only their occurrence is modelled.

## Docs

`RF_Morphing` is **27%** (4,686 of 17,099 bytes). The headline stays 14.4% and 5.7%.
