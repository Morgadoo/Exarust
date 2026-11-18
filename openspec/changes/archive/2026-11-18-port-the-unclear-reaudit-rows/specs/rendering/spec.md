## ADDED Requirements

### Requirement: Hair cloth passes
A cloth whose record carries the hair bit (4) SHALL be drawn as two culled passes, front faces culled first and back faces culled after, at the hair's place in the blended walk; a level cloth SHALL keep its single draw.

#### Scenario: A hair cloth
- **WHEN** a hair cloth is built
- **THEN** its entity draws the front-culled copy of its material and a child draws the back-culled one just after it
