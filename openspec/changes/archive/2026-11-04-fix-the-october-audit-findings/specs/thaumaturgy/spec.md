# Spec Delta

## ADDED Requirements

### Requirement: Shock weapon hits
A hit by a blade that still carries shock-weapon arcs SHALL give the struck body the full shock at strength 0.01 along the striker's tracked swing velocity: the shock's window, its sound, the burn, the stagger and its cry; and a spark burst at the contact. A blade whose arcs were ended SHALL do neither.

#### Scenario: The window refuses a second hit
- **WHEN** an armed blade hits a body with 1 yellow, and again within 150 ms
- **THEN** the body is left with 0.99, its next shock is due 500 to 700 ms on and its shock-stagger flag is set; the second hit takes nothing
