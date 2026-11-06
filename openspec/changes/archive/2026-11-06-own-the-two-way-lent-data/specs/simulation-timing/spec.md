## ADDED Requirements

### Requirement: Step writes to frame data
A fixed-step system that changes data the frame owns SHALL see its own write at once and SHALL have the same write applied on the frame's copy after the step, before the step's other events, alike in one world, inline and on the simulation's thread.

#### Scenario: A step appends to frame data
- **WHEN** the frame and each fixed step append to the same frame-owned list
- **THEN** both sides see the same list after every frame as one world does
