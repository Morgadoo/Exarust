## ADDED Requirements

### Requirement: Hook lists and deferred cancellation
A method hook SHALL be inserted before the first entry with a strictly greater priority and removed by code and data; a procedure hook with a code already listed SHALL be refused. Dispatch SHALL drop every hook that answers false. A deferred call SHALL be removable by its data or procedure tag, the remaining calls keeping their order.

#### Scenario: Cancel the middle call
- **WHEN** five tagged calls are queued and the third's data tag is removed
- **THEN** the next run calls 1, 2, 4 and 5 in that order

### Requirement: Millisecond clock
The millisecond clock SHALL be `(counter - start) × 1000 / frequency` with a wrapping product and a truncating division.

#### Scenario: Truncation
- **WHEN** a 1 MHz counter reads 1 999 999 ticks after its start
- **THEN** the clock reads 1999 ms
