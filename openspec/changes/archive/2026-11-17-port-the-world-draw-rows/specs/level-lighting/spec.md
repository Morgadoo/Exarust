## ADDED Requirements

### Requirement: Automatic sun reads the grid rooms
A level whose record leaves the sun automatic SHALL get a sun only when it has no lights and no grid rooms.

#### Scenario: A grid level without lights
- **WHEN** a record level in automatic sun mode loads with no lights and five grid rooms
- **THEN** it has no sun, while the same level with no rooms has one
