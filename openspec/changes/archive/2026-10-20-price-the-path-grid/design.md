# Design

## Behaviour

World to cell is `(world × (1/unit) + 0.5)` truncated toward zero; cell to world is `(cell − 0.5) × unit`, so a round trip keeps the half-cell convention. The neighbour table is stored in the original's order NW, N, NE, W, E, SW, S, SE with move costs 7 for diagonals and 5 for orthogonals.

A neighbour enters the search only when its floor byte is set, its rise over the current floor is between −20 and +4, its A byte is at most `180/(quality+1)` and its B byte shifted by the query's shift is at most 600. The step's cost is `g + 10·dz² + direction + 2·A + (B << shift)`. The heuristic is `10·sqrt(dx² + dy² + dz²)` in double precision, truncated to an integer.

The two-layer floor selection, the open list and the reconstruction are separate phases.

## Model

`path_cell_coord`, `path_world_coord`, `path_step_ok`, `path_step_cost`, `path_heuristic`, `PATH_NEIGHBOURS`, `PATH_MOVE_COST`.

## Open questions

The A/B byte meanings and the exact two-layer rule are not settled; only their tests and costs are modelled.

## Docs

`TPathFinder` is **32%**. The headline stays 14.4% and 5.7%.
