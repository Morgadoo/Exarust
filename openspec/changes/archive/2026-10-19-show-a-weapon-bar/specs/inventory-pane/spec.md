# Spec Delta

## ADDED Requirements

### Requirement: Weapon bars
A weapon card SHALL show a slash, crush, pierce, or thrust row only when that byte is above 5. The shown bar SHALL be that byte times 0.05.

#### Scenario: Five stays hidden and six is shown
- **WHEN** slash is 5, crush is 6, pierce is 0, and thrust is 20
- **THEN** slash and pierce are absent, crush is 0.3, and thrust is 1
