# Design

## Behaviour

Stepping back tests the child's list index; a value at or below zero does nothing. Otherwise the list rebuild runs with the value less one.

Stepping forward always rebuilds with the value plus one. Both steps call the same rebuild.

## Model

`step_creation_list(index, up)` returns whether the rebuild ran and the index passed to it. A blocked back step leaves the index and reports no rebuild.

## Open questions

The rebuild body is not in.

## Docs

These methods are 43 and 34 bytes. `SG_Character_Create` moves from 22% to about 23%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
