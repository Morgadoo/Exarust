## ADDED Requirements

### Requirement: Skill pointer mask
An index below −1 SHALL return the base result. Mask bit 1 SHALL leave on a positive page, return on a closing word of 1, finish on a larger one, and otherwise clear when state bit 1 is clear and return 1. An index at or below zero with a selection SHALL hide for 200 and hint. The result SHALL otherwise be 16.

#### Scenario: Base, clearing and hiding
- **WHEN** the index is −2 and the base returns 7
- **THEN** the base runs and the result is 7
- **WHEN** mask bit 1 is set, the page is 0, the closing is 0 and state bit 1 is clear
- **THEN** the clear runs and the result is 1
- **WHEN** mask bit 1 is set and the closing is 1
- **THEN** the result is 16 and nothing clears
- **WHEN** the index is 0, a selection exists and the hide is 0
- **THEN** the hide becomes 2 and the deadline is the clock plus 200

### Requirement: Skill pointer selection
Index `0x10000` with mask bit `0x2000` SHALL show for 500 at state byte 4, return on state bit 1, or gain bit 1 and set the other deadline to the clock plus the delay. A selected index with flag `0x1ea` clear SHALL repage, select on index 1, or write its slot when the used byte is clear, then hint and hide.

#### Scenario: Showing, repaging and writing
- **WHEN** the index is `0x10000`, mask bit `0x2000` is set and the state byte is 4
- **THEN** the state becomes 1 and the show deadline is the clock plus 500
- **WHEN** the index is 5, mask bit `0x2000` is set and both selections are zero with repage 1
- **THEN** the high slot is 5, the hint runs, the hide is 1 and the deadline is the clock plus 300
- **WHEN** the index is 1 and a high selection exists
- **THEN** the selection runs
- **WHEN** the used byte is clear and the selection result is positive
- **THEN** the selection runs and the row and column slot is written
