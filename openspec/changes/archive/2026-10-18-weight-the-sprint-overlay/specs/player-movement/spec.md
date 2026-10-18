# Spec Delta

## ADDED Requirements

### Requirement: Sprint latch
Shift with the cursor further than 3 m SHALL keep the sprint latch. Shift without that far cursor SHALL NOT keep it.

#### Scenario: Cursor beyond 3 m
- **WHEN** Shift is held and the cursor is further than 3 m
- **THEN** the sprint latch is kept

#### Scenario: Walk key
- **WHEN** Shift is held and the cursor is not that far
- **THEN** the sprint latch is not kept
