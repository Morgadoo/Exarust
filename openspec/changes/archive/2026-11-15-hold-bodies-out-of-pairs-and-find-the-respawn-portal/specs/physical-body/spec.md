## MODIFIED Requirements

### Requirement: Disabled body
A disabled body SHALL meet no other body in the pair stage and break a grab on it. It SHALL still gather level contacts: the level detection leaves out only an asleep, parked or kinematic body.

#### Scenario: Disabled pair
- **WHEN** one of two overlapping balls is disabled
- **THEN** the pair stage finds no touch

#### Scenario: A disabled body on the floor
- **WHEN** a disabled body's part reaches into the floor
- **THEN** the level detection finds a contact for it

## ADDED Requirements

### Requirement: Held bodies leave the kept pairs
Holding a body SHALL empty its parts' world contacts. The pair drop SHALL remove every kept pair with a held body from the list the last detection left for the next tick's iterations, moving the last pair into each removed place and testing that place again. The release SHALL let the next detection find the body's pairs again.

#### Scenario: A held body's pair
- **WHEN** one body of a kept pair is held and the pairs are dropped
- **THEN** that pair is gone, the other pairs keep their order with the last in its place, and the held body's world contacts are empty

### Requirement: Spawn snap holds the player
The player's third-tick spawn snap SHALL hold its body around the ground placement and drop the kept pairs with it; the AI's re-entry placement SHALL hold nothing.

#### Scenario: The player is snapped to the ground
- **WHEN** the spawn snap places the player's body
- **THEN** the kept pair with the body is dropped and its world contacts are empty
