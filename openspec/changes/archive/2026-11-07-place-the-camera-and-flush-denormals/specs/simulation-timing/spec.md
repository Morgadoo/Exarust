## ADDED Requirements

### Requirement: Engine floating-point mode
The main thread, the simulation thread and every task-pool thread SHALL set the SSE control register's flush-to-zero and denormals-are-zero bits and the exception masks (`0x8FC0` ORed in) before running anything, leaving the rounding mode at round-to-nearest.

#### Scenario: A denormal on the simulation thread
- **WHEN** the smallest normal float is halved on a thread in the mode
- **THEN** the result is zero
