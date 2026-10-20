# Spec Delta

## ADDED Requirements

### Requirement: Crouch
The system SHALL set the player's crouch amount to 1 while the Crouch binding is held and the body is not moving or is down, and SHALL lower it by 0.02 each tick while it is above 0. The body's bearing SHALL carry the amount. Above a crouch of 0.1 the locomotion clip clock SHALL run × (1 − 0.1 × crouch); above 0.5 the footwork's step length SHALL be × 0.7; a crouched attacker SHALL pick the first side attack.

#### Scenario: Crouch in place
- **WHEN** the player holds X while standing
- **THEN** the crouch amount is 1 and the body lowers its pelvis

#### Scenario: Ease out
- **WHEN** the player releases X at a crouch of 1
- **THEN** the amount falls by 0.02 a tick and reaches 0 within 50 ticks

#### Scenario: Not while walking
- **WHEN** the player presses X while walking
- **THEN** the crouch amount does not rise

#### Scenario: Slower crouched walk
- **WHEN** the player walks at a crouch of 1
- **THEN** the walk clip's clock runs at 0.9 of its rate
