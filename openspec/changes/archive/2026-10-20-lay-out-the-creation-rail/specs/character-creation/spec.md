## ADDED Requirements

### Requirement: Creation rail layout
Laying out SHALL store `width − before − unit × count` and return it. Unless compact mode is on, the thumb SHALL be `(count − 4) × unit` and the start `width − (count − 2) × unit`, each in the low 16 bits.

#### Scenario: Full and compact
- **WHEN** the count is 6, the unit 4, the width 100 and the before 10
- **THEN** the view is 66
- **AND** the thumb is 8 and the start 84
- **WHEN** compact mode is on
- **THEN** the view is still 66 and neither rail value is written
