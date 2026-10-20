# Design

## Behaviour

The result starts at 0 on page 2 and −1 otherwise. For every node, the mouse must lie within `23 × unit` of the centre; the comparison passes at equality. Inside that disc, five samples are taken at `j × 0.2` of a turn, for j from 0 through 4. Each offset is `round(unit × 18.3 × sin)` for x and `round(unit × 18.3 × cos)` for y, both in double precision before the round, then scaled by the turn; x adds and y subtracts.

A sample strictly within `5 × unit` of the mouse folds the result with `(j | result) + 1` and ends that node's sample loop. Every node is still visited, so a later hit folds into the earlier result.

## Model

`skill_node_hit(hit, nodes)` takes the mouse, the unit, the turn, the page and the node centres, and returns the folded result. The node position virtual is out of scope.

## Open questions

Why the fold uses a bitwise OR is not established; the return's meaning is not named.

## Docs

This method is 512 bytes. `TSkillUI` moves from 4% to about 7%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
