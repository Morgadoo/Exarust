## ADDED Requirements

### Requirement: Weapon blood spots
Adding a hidden source SHALL add nothing and return 0. Otherwise the source SHALL take the first free spot, or grow the list, with an amount 0, a scale of its two lengths summed and divided by 80 in double precision, an inverse of the square root of 80 over that sum, a tag 0, and the 1-based index returned.

#### Scenario: Two sources
- **WHEN** a source of lengths 40 and 40 is added
- **THEN** it takes spot 1 with amount 0, scale 1 and inverse 1
- **WHEN** a source of lengths 10 and 30 is added
- **THEN** it takes spot 2 with scale 0.5 and inverse the square root of 2

#### Scenario: Hidden source
- **WHEN** a source carrying `0x400000` is added
- **THEN** nothing is added and the return is 0

### Requirement: Weapon blood clearing
Clearing the count SHALL drop every spot. Clearing a spot SHALL drop its source, amount and scale. Adding to a spot's amount SHALL keep it within 0 through 1.

#### Scenario: Reuse and clamp
- **WHEN** spot 1 has had 0.4 and then 0.8 added, and then −2
- **THEN** its amount is 1 and then 0
- **WHEN** spot 1 is cleared and a new source is added
- **THEN** the new source takes spot 1 and the list keeps two spots
- **WHEN** the count is cleared
- **THEN** the list is empty
