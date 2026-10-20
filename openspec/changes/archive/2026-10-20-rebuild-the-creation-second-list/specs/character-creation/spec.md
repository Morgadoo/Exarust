## ADDED Requirements

### Requirement: Creation second list rebuild
Rebuilding the second list SHALL store the index at `0x1a4` and the child's `0xa4` and stop when the band is non-zero. Otherwise it SHALL build the list, fail on a non-positive count, store the index, replace the old list, run the sub-routine, caption widget 9 with `index + 0x41` and draw when allowed.

#### Scenario: Banded, built and empty
- **WHEN** the band is non-zero
- **THEN** the index is stored and no list is built or drawn
- **WHEN** the band is zero, the index is 1, the count 3 and an old list exists
- **THEN** the index is stored, the list replaced, the caption B and the draw runs
- **WHEN** the count is 0
- **THEN** the rebuild fails and nothing is stored or drawn
