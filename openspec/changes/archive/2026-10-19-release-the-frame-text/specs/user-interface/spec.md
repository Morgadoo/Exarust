## ADDED Requirements

### Requirement: Frame text
The frame helper SHALL release one string slot. That slot SHALL start empty. An empty slot SHALL not be freed. A held band, a wide form, and a style that is not that band each SHALL release it once. The bytes the jump skips SHALL not run.

#### Scenario: A plain form releases an empty slot
- **WHEN** unit 5 dispatches a form that is not wide, with no style bits and a close mark of 0
- **THEN** the string slot is empty, it is released once, and it is not freed
