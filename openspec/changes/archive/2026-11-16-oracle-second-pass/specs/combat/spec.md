## ADDED Requirements

### Requirement: A swing's expected time as the conversion's word
A swing's expected milliseconds SHALL be the 64-bit conversion of the count as `CVTSS2SI` makes it (a negative count zeroed by `ms − ms`, a NaN or out-of-range count the integer indefinite), and a stall SHALL end after the shorter of its own length and the expectation, both taken as their low 32 bits, unsigned.

#### Scenario: A swing with no rate
- **WHEN** a swing of zero speed is released
- **THEN** its expectation is the integer indefinite and the stall ends at once
