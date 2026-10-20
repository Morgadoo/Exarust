## ADDED Requirements

### Requirement: Tree decay fields
Mask 0x10 SHALL subtract `(2 + base) × first × 4` and `(2 + base) × second × 4` from fields 0x17c and 0x180, stamp the first start and return 3. Mask 0x20 SHALL call the 0x180 virtual with the runtime value × −10 rounded and return 3.

#### Scenario: The two terms
- **WHEN** the mask is 0x10, the base is 3, the first float is 1, the second is 2 and the fields are 100 and 200
- **THEN** the fields become 80 and 160, the start is stamped and the result is 3
- **WHEN** the mask is 0x20 and the runtime value is 2.5
- **THEN** the virtual receives −25 and the result is 3

### Requirement: Tree decay mode
In mode 3 mask 0x200 SHALL call 0x250 when present and mask 0x100 SHALL call 0x248 when present. Otherwise a mask carrying 0x300 SHALL dim once unless the dim bit is set.

#### Scenario: The mode callbacks
- **WHEN** the mode is 3 and the mask is 0x200 with a 0x250 callback
- **THEN** it runs
- **WHEN** the mode is 0, the mask carries 0x100 and the dim bit is clear
- **THEN** the tree dims once
- **WHEN** the dim bit is set
- **THEN** nothing runs
