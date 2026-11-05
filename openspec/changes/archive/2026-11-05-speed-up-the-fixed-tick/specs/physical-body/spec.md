## ADDED Requirements

### Requirement: Holder lookup by key
A system that finds a held item's holder by key (the entity of the body's first part) SHALL take the first body with that key in its query's order, as a search per item does, and SHALL find every holder it needs in one walk over the bodies per tick.

#### Scenario: Two bodies with one key
- **WHEN** bodies A, B and C are walked in that order, A and C have key K, and an item is held by K
- **THEN** the item's holder is A

### Requirement: Pair search over sleepers
The pair stage SHALL test a sleeping part's grid cell, computed as its pose computes it, before the rest of its pose, and SHALL take the part into the tick's detection exactly when that cell is one of the cells around an awake part.

#### Scenario: A far sleeper
- **WHEN** a sleeping part's cell is two cells from every awake part
- **THEN** it is not found for the tick, and its cell equals its pose's cell bit for bit
