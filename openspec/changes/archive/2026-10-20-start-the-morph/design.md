# Design

## Behaviour

The owner is stored at `this+0x58` and the controller's working point is zeroed. When the owner's helper at `+0x2e0` is missing, the owner's controls at `+0x2a8` are ensured (`0x10005f6b0`) only if they are missing too, a helper of class `0x349ee0` is created, and its virtuals `0xc8` and `0xd0` run. A frame is built from the owner's orientation and the owner's output is refreshed on (`0x10026b880` with 1).

The owner's output count at `+0x280` sizes the 28-byte record array. For each vertex the owner's output point is transformed by the frame (`0x100018870`), the nearest candidate is found by the callback `0x1000de0b0` (squared distance `(dx² + dy²) + dz²`, strictly below the current best, starting at the maximum float), and when one is found the record takes its index, the barycentric weights of that candidate in its control triangle (`0x1000dd580`) and the vertex as its base. With none, the record takes joint 0, a zero offset, the unmatched count rises and the vertex index is appended to the unmatched list at `+0x30`. The owner output is refreshed off at the end.

## Model

`start_morph(owner_points, candidates, controls, frame, has_helper, has_controls)` returns `MorphStart { created_helper, ensured_controls, records, unmatched }`. The frame is four 3-float rows: three axis rows and the translation.

## Open questions

The helper's virtuals are engine plumbing; only their occurrence is modelled.

## Docs

`RF_Morphing` is **23%** (4,006 of 17,099 bytes). The headline stays 14.4% and 5.7%.
