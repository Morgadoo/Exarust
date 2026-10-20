# Design

## Behaviour

The grid carries the floor byte and the two penalty bytes per cell. A cell is walkable when its floor is non-zero and `A + B < 80`. A walkable start returns its own cell and `height = base + floor·10`.

When the start is not walkable, the sweep visits the start, then expands its eight neighbours inside the 32×32 window centred on the start (in-bounds only). The open list is popped by the smallest squared distance from the start, the newest winning a tie. A popped walkable cell is returned; a reached node count of 220 ends the sweep with nothing. The height of the returned cell uses the same `base + floor·10`.

The original's two entry variants (a blocked `0xff` A byte or no layer) and its two-layer floor pick are not separated; the model uses the one floor.

## Model

`SnapGrid { width, height, floor, a, b }`, `SnapResult { cell, height, nodes }` and `snap_to_walkable(grid, start, base_height)`.

## Open questions

The two entry variants and the layer pick are not in.

## Docs

`TPathFinder` is **58%**. The headline stays 14.4% and 5.7%.
