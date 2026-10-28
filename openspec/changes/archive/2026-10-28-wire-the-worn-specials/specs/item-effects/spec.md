## ADDED Requirements

### Requirement: Worn effects in play
A spawned body SHALL get the effect list its worn items register: a healing item only when the body takes remedies, a golem suit's and gauntlets' ticks on any body, each item once.

#### Scenario: No remedies
- **WHEN** a body whose rules do not take remedies wears only a healing item
- **THEN** it gets no effect list

### Requirement: Effect list once a second
The effect list SHALL run when the game clock is past the stored clock, which then becomes the clock plus 1000 ms; an empty list SHALL clear the two impact counters instead.

#### Scenario: Once a second
- **WHEN** a body's list runs at 12 ms
- **THEN** the next run is at 1012 ms; nothing runs at 1012 ms and it runs again at 1013 ms

### Requirement: Worn healing
A healing item whose flag byte's low five bits are 2 and whose charge is at most 0xFE SHALL add 0.004 of the maximum to red health in double precision, holding red at the maximum when it passes it and otherwise advancing the charge.

#### Scenario: Healing step
- **WHEN** red is 0.1 of 0.25 at a run
- **THEN** red becomes 0.1 + 0.001 and the charge advances by one

#### Scenario: Held at the maximum
- **WHEN** red is 0.2495 of 0.25 at a run
- **THEN** red becomes 0.25 and the charge does not advance
