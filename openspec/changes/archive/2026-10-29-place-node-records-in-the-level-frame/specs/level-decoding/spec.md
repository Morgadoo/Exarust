## MODIFIED Requirements

### Requirement: Placement transform records
The system SHALL apply a placed prop's or object's transform records before its bodies register: each is its node's frame in the level, taken into the node's local transform from its parent's frame at the placement, then its nested records apply to that node's children. A record SHALL find its node from the one it applies to (the root at the top level): that node for an empty name, else its first direct child of exactly that name, and is skipped with its nested ones when that node is missing.

#### Scenario: First story level
- **WHEN** `exanima01.rfc`'s props are read against their objects
- **THEN** every record finds its node and some transform records carry nested ones

#### Scenario: A wall ring
- **WHEN** a wall ring placed turned half round has a record putting the ring 10 cm along the model's z from the placement
- **THEN** the ring's local transform is 10 cm along z and its frame in the level is the record's
