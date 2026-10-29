## ADDED Requirements

### Requirement: Cloth body part index
The cloth stage SHALL offer each cloth exactly the body parts a full re-bucket of every body by its mesh box's centre would link in the nine 6 m cells around the cloth's own, each cell's parts in the bodies' order then the parts' order, while re-bucketing only the bodies changed since its last tick and reading only the cells around its cloths. It SHALL keep no index while no cloth is simulated.

#### Scenario: A body moves on
- **WHEN** one of three indexed bodies changes and moves one cell on while the others are unchanged
- **THEN** the index re-buckets that body only and every cell holds what a fresh index of all three holds, in the same order

#### Scenario: A level of sleepers
- **WHEN** fifteen hair cloths hang among 4 800 sleeping bodies of ten parts for 400 ticks
- **THEN** each hair's particles are bit for bit those of a full re-bucket every tick, and the stage re-buckets only the bodies marked changed
