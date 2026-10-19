## ADDED Requirements

### Requirement: Parent skin hit test
The hit test SHALL return 0x80000011 or 0x80000012 for flag 0x4000, and OR 0x80000014 or 0x80000018 for flag 0x8000, before any probe. Otherwise flag bit 0 SHALL probe −4, −5 and −10 above its y threshold and fall back to −3; flag 0x200 SHALL probe −9 and −10 from x 0x44 and fall back to −8; flag 0x100 SHALL probe −14 and −13 from y 0x3c and fall back to −12. A probe SHALL hit only when the supplied rectangle contains the point inclusively.

#### Scenario: The edge codes
- **WHEN** flag 0x4000 is set with size 10 and x 0
- **THEN** the result is 0x80000011
- **WHEN** flag 0x8000 is set with size 10, y0 100 and y 95
- **THEN** the result is 0x80000018
- **WHEN** both flags are set with x 0 and y 0
- **THEN** the result is 0x80000011 | 0x80000014

#### Scenario: The threshold probes
- **WHEN** flag bit 0 and bit 1 are set, unit 5, limit 0x40 50, and y 40, with a rectangle for −4
- **THEN** the result is −4
- **WHEN** the same point has no rectangle for any probed code
- **THEN** the result is −3

#### Scenario: The right and bottom edges
- **WHEN** flag 0x200 is set and x is 90 at limit 0x44 90, with no rectangle for −9 but one for −10
- **THEN** the result is −10
- **WHEN** neither has a rectangle
- **THEN** the result is −8
- **WHEN** flag 0x100 is set and y is 90 at limit 0x3c 90, with no rectangle for −14 or −13
- **THEN** the result is −12
