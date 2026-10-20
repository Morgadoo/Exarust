## ADDED Requirements

### Requirement: Creation deferred callback
Flushing SHALL do nothing unless the pending flag is set and the stamp plus 1 is below the clock. Then the stamp SHALL take the clock, the flag SHALL clear and the callback SHALL run. The result SHALL be true either way.

#### Scenario: Waiting, firing and idle
- **WHEN** the pending flag is clear
- **THEN** nothing fires and the result is true
- **WHEN** the flag is set, the stamp 10 and the clock 11
- **THEN** nothing fires
- **WHEN** the flag is set, the stamp 10 and the clock 12
- **THEN** the stamp becomes 12, the flag clears and the callback runs

### Requirement: Creation view attach
A tip SHALL be released only when present, and a view SHALL be stored and its callback attached only when present.

#### Scenario: Present and absent
- **WHEN** the tip is present
- **THEN** it is released
- **WHEN** the view is absent
- **THEN** nothing is stored or attached
