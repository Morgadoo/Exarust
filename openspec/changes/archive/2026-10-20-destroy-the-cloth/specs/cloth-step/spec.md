## ADDED Requirements

### Requirement: Cloth destroy
The destructor SHALL make one release and one final call on a positive free flag, free each of four stored pointers present, free the origin only for a root, destroy three managed fields, and free each list entry and the list when present. The base call SHALL always run.

#### Scenario: The freeing path
- **WHEN** the free flag is set, all five pointers are present, the cloth is its root and the list holds 3 entries
- **THEN** one release, five frees, three field calls, 3 list frees and one list free run
- **AND** the base and final calls each run once

#### Scenario: The quiet path
- **WHEN** the free flag is clear and no pointer or list is present
- **THEN** only the three field calls and the base call run
