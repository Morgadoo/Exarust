## ADDED Requirements

### Requirement: Band call
On that band the dispatcher SHALL count one call with argument 0. A close mark of -4 still counts one, and the square remains. A style that is not that band SHALL count none. A wide bit SHALL count none. The bytes the jump skips are not in.

#### Scenario: A held band counts one
- **WHEN** unit 5 dispatches a held form of width 100 and height 80 that is not wide, with style bits 0x80, a close mark of 0, an edge of 4, 4, a right room of 400, and a bottom room of 80
- **THEN** the call count is 1
