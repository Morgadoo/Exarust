## ADDED Requirements

### Requirement: Volume summary
The system SHALL summarise a padded voxel grid by summing each filled cell's centre minus 0.5, and SHALL return the centre (sum/count/dim − 0.5)·scale and the mass count·Π(2·scale/dim).

#### Scenario: A unit volume
- **WHEN** a 1×1×1 volume has one filled cell at its centre and scale (1, 1, 1)
- **THEN** the centre is (0, 0, 0) and the mass is 8

#### Scenario: An empty volume
- **WHEN** no cell is filled
- **THEN** the centre is (0, 0, 0) and the mass is 0.1

### Requirement: Volume sort
The system SHALL sort entries by their float key with a middle-pivot two-way quicksort.

#### Scenario: Equal and out-of-order keys
- **WHEN** the keys are 3, 1, 2, 1
- **THEN** they are ordered 1, 1, 2, 3
