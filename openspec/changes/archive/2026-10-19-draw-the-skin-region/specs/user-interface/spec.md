## ADDED Requirements

### Requirement: Skin scroll region
With origin 10, 20, inset 5, 4, extent 100, 80, totals 150, 120 and fractions one half and one quarter, the region SHALL clip at 15, 24 of size 95 by 76 and scroll by 28 on x and 11 on y. Flag 0x800 SHALL zero both offsets. Mode bit 0 or a rejected clip SHALL stop after the first begin. Flag 0x80 SHALL add a 0x30 detail at the scroll offsets with offset one unit; flag 0x100000 SHALL add a 0x50 detail with offset four units.

#### Scenario: The clip and scroll
- **WHEN** the mode is 0 and the clip is accepted
- **THEN** the clip is 15, 24, 95, 76
- **AND** the scroll is 28, 11
- **AND** two begins, two scissor calls and one line end are counted
- **AND** every quad and every visible child is replayed

#### Scenario: The early stops
- **WHEN** mode bit 0 is set
- **THEN** only one begin runs and nothing is clipped
- **WHEN** the clip is rejected
- **THEN** only one begin runs and nothing is clipped

#### Scenario: The details and translation
- **WHEN** flag 0x800 is set
- **THEN** the scroll is 0, 0
- **WHEN** flag 0x80 is set
- **THEN** the detail is at 28, 11 with mode 0x30 and offset 5
- **WHEN** flag 0x100000 is set
- **THEN** the detail has mode 0x50 and offset 20
- **WHEN** the translation applied
- **THEN** it moves by −10 and 20
