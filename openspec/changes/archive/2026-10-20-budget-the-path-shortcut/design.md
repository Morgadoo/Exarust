# Design

## Behaviour

The points and their costs are walked as in the unbudgeted shortcut. When testing a jump from the last kept index `kept` to `next`, the clear predicate receives `cost[next] − cost[kept] + 10`. A clear jump drops the current point; a blocked one keeps it. The first and last points are always kept.

## Model

`shortcut_path_with_budget(points, costs, clear)` where `clear(a, b, budget)`.

## Open questions

The two-layer floor rule is separate.

## Docs

`TPathFinder` is **61%**. The headline stays 14.4% and 5.7%.
