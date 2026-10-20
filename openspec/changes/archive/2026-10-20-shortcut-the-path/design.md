# Design

## Behaviour

A path of two points or fewer is returned unchanged. Otherwise the first point is kept, and from the last kept point each following point is tested: when the direct line from the kept point to the point after it is clear, the current point is dropped and the next becomes current; when it is blocked, the current point is kept. The final point is always appended. The clear predicate is supplied by the caller (the line walk over the grid).

This is the safe form of the original's string pulling: the original walks its stored path backwards with a `dg + 10` line-walk budget and its own index order. The exact index boundary is not reproduced, but the rule — keep a bend only when the direct line is blocked — is.

## Model

`shortcut_path(points, clear)` returns the shortened cell path.

## Open questions

The original's budget and its exact backward indexing are not in.

## Docs

`TPathFinder` is **59%**. The headline stays 14.4% and 5.7%.
