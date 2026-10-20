## ADDED Requirements

### Requirement: Morph weight merge
Merging SHALL do nothing when the scale is below the epsilon. Up to four source entries SHALL be read, stopping at the first id below one. An id already present SHALL add `weight × scale`; a new id SHALL append to the highest used slot plus one when that leaves room.

#### Scenario: Add, append and stop
- **WHEN** the current slots are 1 and 2 with weights 0.5 and 0.5, the source is 2 and 3 with weights 1 and 0.5, and the scale 2
- **THEN** slot 2 weighs 2.5 and slot 3 is appended at 1
- **WHEN** the scale is 0
- **THEN** nothing changes
- **WHEN** the first source id is 0
- **THEN** the merge stops
