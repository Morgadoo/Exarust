## ADDED Requirements

### Requirement: Frame work from step results
Work only the frame does with a step's results (sounds the step decided on) SHALL run on the frame's world after that step, in the order the step queued it, alike in one world and behind the bridge.

#### Scenario: A prop strikes the floor
- **WHEN** a plain body's part rings an impact in a step
- **THEN** the frame plays its set's sound after that step, at the part's mesh centre, in one world and behind the bridge alike
