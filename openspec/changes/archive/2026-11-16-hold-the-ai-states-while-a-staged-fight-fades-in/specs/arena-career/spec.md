## ADDED Requirements

### Requirement: The arena staging sets the staging freeze
Standing a match's first round, its next round and a tournament's bracket round SHALL set the staging freeze, posted for the next fixed step when the frame has a fixed queue.

#### Scenario: First round and next round
- **WHEN** a match's first round is stood with a fixed queue, and a next round is stood by the end screen's Continue without one
- **THEN** the freeze is set once the queue runs, and at once for the next round
