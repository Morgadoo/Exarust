# Design

## Behaviour

The grid is one floor layer with a floor byte and the two penalty bytes per cell. The start cell becomes node 1 (node 0 is the sentinel), with `g = 0` and `f = 10·sqrt(dx² + dy² + (A/3)²)`, the pseudo-height bias. The open list is a list of node indices; a pop scans it from the newest to the oldest and keeps the first strictly smaller `f`, so the newest wins a tie.

Each pop checks the acceptance distance against the best heuristic seen, then expands the eight neighbours in NW, N, NE, W, E, SW, S, SE order. A neighbour enters when `path_step_ok` accepts it. A free cell creates a node with `g`, `f`, the parent and an open stamp; an open cell with a smaller `f` updates its `g`, `f` and parent; a blocked stamp is skipped. The best node tracks the smallest heuristic.

The search stops when the open list is empty, the node count reaches 4095, or the best heuristic is under the acceptance distance. The path walks the best node's parents to the sentinel, reverses, and drops a point whose direction from the previous point matches the previous direction. The goal is reached only if the best node sits on the goal cell.

## Model

`PathGrid { width, height, floor, a, b }`, `PathRequest { start, goal, goal_floor, quality, shift, accept }`, `PathSearch { path, reached, nodes }` and `search_path`.

## Open questions

The two-layer floor choice, the greedy line-walk shortcut and the endpoint penalty fix-up are not in.

## Docs

`TPathFinder` is **44%**. The headline stays 14.4% and 5.7%.
