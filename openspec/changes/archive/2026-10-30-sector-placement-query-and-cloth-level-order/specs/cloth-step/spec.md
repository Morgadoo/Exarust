# Spec Delta

## ADDED Requirements

### Requirement: Cloth static contacts after the step
The cloth stage SHALL gather a cloth's static records and their triangles from the cloth's bound after its step, then run the level contact, then end the tick (sleep and reaction), and SHALL offer a cloth no body contacts from a disabled body.

#### Scenario: Level contact between step and finish
- **WHEN** a cloth particle 1 cm over an upward-facing level triangle is stepped up to the level contact
- **THEN** no level test has run; the level contact then lifts it, and the finish leaves a downward reaction

#### Scenario: Disabled body
- **WHEN** an enabled and a disabled body lie in a cloth's cell
- **THEN** one part is offered to the cloth
