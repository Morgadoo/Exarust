# Spec Delta

## ADDED Requirements

### Requirement: Apparel bars
An apparel card SHALL always show coverage as that byte times 0.05. It SHALL show impact, slash, crush, pierce, and encumbrance only when that byte times 0.05 is above 0.25.

#### Scenario: A quarter stays hidden
- **WHEN** coverage is 5, impact is 5, slash is 6, crush is 0, pierce is 20, and encumbrance is 5
- **THEN** coverage is 0.25, impact, crush, and encumbrance are absent, slash is 0.3, and pierce is 1
