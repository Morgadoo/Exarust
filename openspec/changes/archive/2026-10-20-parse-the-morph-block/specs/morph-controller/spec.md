## ADDED Requirements

### Requirement: Morph block parse
Parsing SHALL require the tag `0xEDC0C589`, then read the unknown word, the first count and its points, the owner's skipped points, the second count and, for each entry, the first points and the owner's points. A truncated or wrongly tagged block SHALL return none.

#### Scenario: One entry and failures
- **WHEN** the block has one first point, two skipped points and one entry
- **THEN** the first points and the entry's two runs are read in order
- **WHEN** the last four bytes are missing
- **THEN** parsing returns none
- **WHEN** the tag differs
- **THEN** parsing returns none
