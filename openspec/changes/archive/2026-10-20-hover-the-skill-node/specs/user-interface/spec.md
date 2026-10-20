## ADDED Requirements

### Requirement: Skill hover
Setting the hover SHALL store a positive index or current and return 3. The candidate SHALL be the index when bits 8–10 and the low 3 bits are positive and its grid byte is set, else 0. An unchanged candidate SHALL leave the stamps; a new positive candidate with a different last positive SHALL stamp the clock, else the stamp SHALL take the elapsed with the under-200 correction. The candidate SHALL become previous and, when positive, last positive. With both non-positive the base runs.

#### Scenario: Hover, miss and stamp
- **WHEN** the index is 0, the current is 0 and the base is 7
- **THEN** the result is 7 and the base ran
- **WHEN** the index is 0x0203, grid byte 20 is set, previous is 0, stamp 900 and clock 1000
- **THEN** the candidate and last positive are 0x0203
- **AND** the stamp stays 900
- **WHEN** the last positive is 0x0101 instead
- **THEN** the stamp becomes 1000
- **WHEN** the candidate equals the previous
- **THEN** the stamps do not change
- **WHEN** the grid byte is clear
- **THEN** the previous stays 0
