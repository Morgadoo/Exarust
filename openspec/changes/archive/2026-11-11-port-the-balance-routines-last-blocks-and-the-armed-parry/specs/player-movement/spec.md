## MODIFIED Requirements

### Requirement: Crouch
The system SHALL set the player's crouch amount to 1 while the Crouch binding is held and the body is not moving or is down, and SHALL lower it by 0.02 each tick while it is above 0. The body's bearing SHALL carry the amount. Above a crouch of 0.1 the locomotion clip clock SHALL run × (1 − 0.1 × crouch); a crouched attacker SHALL pick the first side attack. The footwork's step length SHALL not change with the crouch.

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

### Requirement: Queued step taken on landing
A tick on which the footwork changes a foot (a reached or stalled foot lands, or the other foot lands) with a queued step waiting SHALL check it by the body's queue rule; taken, the step starts anew (no balance step, plain kind), the queue count −2 before the episode's first landing, −1 after, and the chosen foot SHALL not start that tick. The behaviour SHALL then clamp and snap the target and take the queued word back.

#### Scenario: A landing takes the queue
- **WHEN** a queued step 10 cm along the line to the target waits and the other foot lands
- **THEN** the step is taken with the count −2 and the chosen foot does not start

#### Scenario: Nothing changed
- **WHEN** the same step waits and no foot changes
- **THEN** it waits and the chosen foot starts
