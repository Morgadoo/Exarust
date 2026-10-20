## ADDED Requirements

### Requirement: Creation label refresh
Refreshing SHALL do nothing while the clock is less than 300 above the last stamp. Otherwise the stamp SHALL take the clock, the label SHALL be built when no widget exists, and the widget SHALL be drawn.

#### Scenario: Throttled, built and existing
- **WHEN** the clock is 100 and the last stamp 0
- **THEN** nothing runs and the stamp stays 0
- **WHEN** the clock is 400, the last stamp 0 and no widget exists
- **THEN** the label is built, the widget is drawn and the stamp becomes 400
- **WHEN** the widget already exists
- **THEN** the label is not built and the widget is still drawn
