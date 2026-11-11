## ADDED Requirements

### Requirement: Story continue exit nudge
A story continue SHALL move the placed player out of the first level exit zone, in placement order, that holds it: within the zone's radius plus 50 cm of its centre and within 50 (squared centimetres) of its box. The player SHALL move along the zone's axis with the smaller clearance, past the box's side by 75 cm, before it is stood on the floor. A continue without a placement SHALL not nudge.

#### Scenario: A save inside an exit
- **WHEN** a story save is continued with the player 20 cm inside an exit zone whose half depth is 50 cm along z and half width 100 cm along x
- **THEN** the player is moved 105 cm along the zone's z axis and stays in the level
