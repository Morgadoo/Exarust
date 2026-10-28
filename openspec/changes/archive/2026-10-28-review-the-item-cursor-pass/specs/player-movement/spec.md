## ADDED Requirements

### Requirement: Item cursor turn input
The system SHALL turn a lifted item from the pad's raw move stick when its squared length is over 0.1 (compared in double precision), its forward part about the camera's axis and its sideways part about the world's up axis, and otherwise from the move keys, Forward over Back and Left over Right, each giving the turn speed; the speed SHALL be the ramp factor times the last frame's milliseconds times 0.0025, multiplied in double precision; mouse motion SHALL NOT turn the item.

#### Scenario: Forward stick
- **WHEN** the move stick is fully forward on a 100 ms frame at the base factor
- **THEN** the item turns 0.0625 rad about the camera's axis and none about the up axis

#### Scenario: Stick in the dead zone
- **WHEN** the move stick is 0.3 long and no key is held
- **THEN** the item does not turn

### Requirement: Item cursor placement
The system SHALL place a lifted item at the cursor ray's level hit, the held item left out of the pick, when the hit's height is not under −100000 cm, else where the ray crosses that height unless the ray climbs, and SHALL place it whether or not its box stays within 280 cm up and −1000 cm down.

#### Scenario: Box out of range
- **WHEN** the placement point puts the item's box above 280 cm
- **THEN** the item is still placed there and the box test answers no

### Requirement: Item cursor release
The system SHALL leave a plainly released lifted item where the cursor put it, SHALL give it to a GUI item slot it is over, and SHALL put a lift cancelled by the movement request back at its saved frames unless the world hover bit is set.

#### Scenario: Plain release
- **WHEN** the left button is released over the level with no hover word
- **THEN** the item stays where it was placed

#### Scenario: Cancelled lift
- **WHEN** the lift is cancelled with no hover word
- **THEN** the item goes back to the frames saved at the lift
