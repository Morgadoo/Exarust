# Design

## Behaviour

The bitmap is all ones after the rest payload, then the target vertices (the controller's unmatched list) are cleared. A pass walks every unknown vertex and, over its adjacency, keeps only neighbours already known, weights each by `1/√(distance squared)`, and accumulates the delta and the weight sum. A positive sum stores `acc / sum` and marks the vertex for promotion.

All vertices marked in a pass are promoted to known together, so they only feed the next pass. A pass that changes nothing ends the diffusion. The same equations fill the skin records; the record variant is already modelled by `fill_skin_record`.

## Model

`diffuse_morph_deltas(deltas, known, neighbours)` updates the deltas and the known flags in place and returns the number of changing passes. `neighbours[v]` holds `(neighbour, squared distance)`.

## Open questions

The packed adjacency and the corner lookup are the caller's resolution; the model takes resolved neighbours.

## Docs

`RF_Morphing` is **40%** (6,843 of 17,099 bytes). The headline stays 14.4% and 5.7%.
