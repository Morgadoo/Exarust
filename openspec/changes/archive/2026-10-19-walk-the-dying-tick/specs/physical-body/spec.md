# Spec Delta

## ADDED Requirements

### Requirement: Dying tick walk
A collapsing body's dying tick SHALL walk its stagger from the measured chest and the fast velocity when no step is running. A chest recorded at the origin, then a metre along z, then there again, SHALL anchor one foot on the first tick and leave that foot on the second.

#### Scenario: A measured chest anchors one foot
- **WHEN** a collapsing body records its chest at the origin, then a metre along z, then there again, and its dying tick runs twice
- **THEN** the first tick anchors the first foot and leaves the second, and the second tick leaves that same pair
