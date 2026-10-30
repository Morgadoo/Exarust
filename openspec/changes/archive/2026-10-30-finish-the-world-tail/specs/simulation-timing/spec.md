## MODIFIED Requirements

### Requirement: Environment tick conversion
The system SHALL convert an environment to one tick by raising each per-second retention that is above zero to `1/ticks` and storing zero for any other retention, a NaN included, scaling the gravity vector by the reciprocal of `ticks` squared as a float multiply, and squaring each speed threshold after dividing it by the tick count.

#### Scenario: The inert environment
- **WHEN** the inert environment is converted at 83 ticks
- **THEN** every retention and the gravity are zero

#### Scenario: A NaN retention
- **WHEN** an environment whose fast retention is NaN, slow retention −1 and resting retention 0.05 is converted at 83 ticks
- **THEN** the retentions are 0, 0 and 0.96455044

## ADDED Requirements

### Requirement: Scene sector creation
The simulation SHALL create one sector per sector node of a scene batch; a new sector that holds no static nodes yet SHALL hand its node's children back for immediate registration, and when the scene shares the first sector every node of the batch SHALL append its children to that sector's pending list without changing its scene word or node.

#### Scenario: A shared batch
- **WHEN** a shared batch of two sector nodes with children 30 and 31, 32 is added after a first sector exists
- **THEN** no new sector is created, the first sector keeps its scene and node, and its pending list is 30, 31, 32
