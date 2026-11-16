## ADDED Requirements

### Requirement: Particle dose on a character
A particle dose on a character's part SHALL add to the character's burn strength when the part's skin bone index (its bone less one) is above zero; any other body SHALL keep nothing. Every character SHALL carry a burn.

#### Scenario: The lumbar and the pelvis
- **WHEN** a fire's particle meets a character's lumbar part and then its bone-1 part with a dose of 0.5
- **THEN** the burn strength is 0.5 for the lumbar and unchanged for the other
