# Spec Delta

## MODIFIED Requirements

### Requirement: Clips follow movement
The system SHALL play the walk clip whenever the player moves forward, walking or running, starting it from any looping clip, and SHALL end it in the idle clip when the player stops out of combat mode. Standing still out of combat mode, the steering SHALL start no other clip: the stance decides, so the combat idle stays until the player walks.

#### Scenario: Start and stop
- **WHEN** the player starts walking and then stops
- **THEN** the clip changes from idle to walk and back to idle

#### Scenario: Combat idle after combat mode
- **WHEN** Tab turns combat mode off while the player stands in Cmbt - Idle with the Defence stance, and it then turns in place, walks and stops
- **THEN** it keeps Cmbt - Idle while standing and turning, plays the walk, and stops in idle

#### Scenario: Running plays the walk
- **WHEN** the player runs forward
- **THEN** the clip is the walk
