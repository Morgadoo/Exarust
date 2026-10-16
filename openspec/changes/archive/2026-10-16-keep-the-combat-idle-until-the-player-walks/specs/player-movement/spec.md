## MODIFIED Requirements

### Requirement: Clips follow movement
The system SHALL play the walk clip when the player walks and the run clip when it runs, starting them from any looping clip, and SHALL end them in the idle clip when the player stops out of combat mode. Standing still out of combat mode, the steering SHALL start no other clip: the stance decides, so the combat idle stays until the player walks.

#### Scenario: Start and stop
- **WHEN** the player starts walking and then stops
- **THEN** the clip changes from idle to walk and back to idle

#### Scenario: Combat idle after combat mode
- **WHEN** Tab turns combat mode off while the player stands in Cmbt - Idle with the Defence stance, and it then turns in place, walks and stops
- **THEN** it keeps Cmbt - Idle while standing and turning, plays the walk, and stops in idle

### Requirement: Carried torch
The system SHALL give the player the game's lit torch in play mode in story levels. It goes in the right hand with the "Torch" moveset, or in the left hand with "1H & Torch" when a weapon is held. Each hand places it by the original's grip table: an offset plus Euler angles applied about X, then Z, then Y of the hand. Practice and arena levels SHALL start the player without a torch. The torch's light SHALL also light its holder.

#### Scenario: Starting a level
- **WHEN** play mode starts in a dungeon
- **THEN** the player holds a burning torch raised in the right hand that lights and shadows the room

#### Scenario: Starting a story level
- **WHEN** `exarust play exanima01` runs
- **THEN** the player holds a burning torch raised in the right hand, and it lights and shadows the room and the player's own skin

#### Scenario: Arena match
- **WHEN** `exarust play arenao1` runs
- **THEN** the player carries no torch, walks with the default moveset, and no torchlight falls on the body

#### Scenario: Exploring with the torch
- **WHEN** the player carrying the torch in the right hand stands out of combat mode
- **THEN** it holds the torch up beside its head as "Torch" Idle asks: the elbow bent about 123°, the upper arm about 40° out to the side, the wrist at shoulder height

#### Scenario: Torch in combat mode
- **WHEN** the player carrying the torch in the right hand attacks in combat mode with no target near
- **THEN** the attack ends in "Torch" Cmbt - Idle, the carry of the reference shots: the upper arm tucked and 32° forward, the elbow bent about 100°, the wrist 32 cm before the shoulder
