## ADDED Requirements

### Requirement: Twenty-word layer level
The apparel cache SHALL order garments by their layer level, the highest layer byte over all twenty occupancy words (bones 1 to 20), inserting each after every entry of a level at most its own.

#### Scenario: Bone twenty
- **WHEN** a garment whose only layer-3 byte lies on bone 20 is worn before a layer-0 garment
- **THEN** the layer-0 garment is planned first

### Requirement: Slot-walk stack overflow
The slot assembler SHALL log a body-part stack overflow when its walk stack holds more than 31 entries and SHALL continue the walk.

#### Scenario: Forty children
- **WHEN** a slot's node sits among forty children of one node
- **THEN** the slot still chooses it

### Requirement: Build clamps
The build SHALL read the record's feral level and age within 0 to 1 (a NaN kept) wherever the look, darkening and stats use them.

#### Scenario: Over one
- **WHEN** a record's feral level is 1.5 on a race whose skin darkens with age
- **THEN** the darkening is 1.0
