## MODIFIED Requirements

### Requirement: Volume sort
The system SHALL sort entries by their float key with the original's middle-pivot two-way quicksort: the pivot halfway between the current low and the call's high, the left scan passing keys below the pivot and the right keys above it, a swap while they have not crossed, the left side sorted by recursion and the right by looping.

#### Scenario: Equal and out-of-order keys
- **WHEN** the keys are 3, 1, 2, 1
- **THEN** they are ordered 1, 1, 2, 3

## ADDED Requirements

### Requirement: Volume neighbour scans
The system SHALL walk all 26 neighbours of a cell, in x, then y, then z order with the far corner (+1, +1, +1) last, in the near-surface test, the corner spheres' shared-corner walk and their pull.

#### Scenario: A thin slab's fallback
- **WHEN** a 6 × 6 × 2 cell slab too thin for a candidate takes its fallback spheres
- **THEN** the middle sphere sits on its corner point and the sphere at x = 1 is pulled 24 steps of 0.02 half cells toward +x
