## ADDED Requirements

### Requirement: Path stamp clear
The stamp clear SHALL fill the first 256 entries, the entries 0xfeff through 0xffff and both edge entries of rows 1 through 254 with the blocked value, leaving the interior to the caller. A short table SHALL be left alone.

#### Scenario: The border ring
- **WHEN** a 256×256 table of zeroes is cleared
- **THEN** the first and last rows, the first and last columns and the entry at 0xfeff hold the blocked value
- **THEN** the interior, such as 0xfe01, stays zero
- **WHEN** the table is shorter than 65,536 entries
- **THEN** it is unchanged
