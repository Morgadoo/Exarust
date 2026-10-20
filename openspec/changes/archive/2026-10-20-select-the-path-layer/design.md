# Design

## Behaviour

The current floor `c` and the two words `(ceil, floor)` are read. A zero layer-0 floor refuses. If `floor0 <= c+4`, layer 0 is chosen with that floor. If `floor0 > c+4` and `c+10 > floor0`, the choice is refused; otherwise (a floor 5–10 above) the layer marker is −1 and the floor is kept for the fallback.

Layer 1 is then considered. A zero floor keeps layer 0 when there is one and refuses otherwise. If `floor1 <= c+4`, a higher floor than the kept one switches to layer 1; a lower one keeps layer 0. If `floor1 > c+4` and `c+10 <= floor1`, layer 0 is kept or refused. In the narrow band `c+4 < floor1 < c+10`, a non-zero layer-1 ceiling or a floor below the kept one refuses; otherwise layer 0 is kept or refused.

Finally the rise `chosen − c` must lie in −20 through +4; the result is the layer byte and the floor.

## Model

`LayerWord = (u8, u8)`, `choose_path_layer(current_floor, layer0, layer1) -> Option<(u8, u8)>`.

## Open questions

The two bytes are named ceiling and floor from the branch usage; the exact height semantics remain inferred.

## Docs

`TPathFinder` is **63%**. The headline stays 17.0%.
