# Design

## Behaviour

The owner is stored on the controller and the controller's working point is zeroed. When the owner's helper is missing, the owner's controls are ensured only if they are missing too, a helper of its class is created, and two of its virtuals run. A frame is built from the owner's orientation and the owner's output is refreshed on (with 1).

The owner's output count sizes the 28-byte record array. For each vertex the owner's output point is transformed by the frame, the nearest candidate is found by the nearest callback (squared distance `(dx² + dy²) + dz²`, strictly below the current best, starting at the maximum float), and when one is found the record takes its index, the barycentric weights of that candidate in its control triangle (the basis solver) and the vertex as its base. With none, the record takes joint 0, a zero offset, the unmatched count rises and the vertex index is appended to the unmatched list. The owner output is refreshed off at the end.

## Model

`start_morph(owner_points, candidates, controls, frame, has_helper, has_controls)` returns `MorphStart { created_helper, ensured_controls, records, unmatched }`. The frame is four 3-float rows: three axis rows and the translation.

## Open questions

The helper's virtuals are engine plumbing; only their occurrence is modelled.

## Docs

`RF_Morphing` is **23%** (4,006 of 17,099 bytes). The headline stays 14.4% and 5.7%.
