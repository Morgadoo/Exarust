## ADDED Requirements

### Requirement: Golem-suit leg gains
A character body's golem-suit flag SHALL be set when the body is built for a golem race. While it is set, the leg routine SHALL halve the standing thigh factor and quarter the leg limit's target.

#### Scenario: A golem stands
- **WHEN** a golem-race body is built and stands
- **THEN** its golem-suit flag is set and its thigh factor is the standing factor times 0.5

### Requirement: Balance step stand-still stamp
The end of a balance step SHALL stamp the body's stand-still clock with the game clock plus 100 ms, the step's kind read before the step's bytes clear; the end of any other step SHALL leave the stamp.

#### Scenario: A balance step ends
- **WHEN** a balance step ends at game clock 50 ticks
- **THEN** the stand-still clock is 50 ticks in milliseconds plus 100

### Requirement: Per-tick foot factors
The foot servo SHALL scale the motion it measures by 1.0375 before dividing by the squared gap, and a stepping foot's approach SHALL scale its closing speed by 80/83 (single precision).

#### Scenario: A foot moving a twentieth of its gap
- **WHEN** a foot moves 0.49 cm along a 10 cm gap in a tick
- **THEN** the servo leaves it alone, as it already closes over 5 % of the gap

### Requirement: Off-balance arms
The arms SHALL take the off-balance stiffness only on the posture block's own test (last tick's balance under 0.6 or getting up); a low balance measured this tick or an unsupported body SHALL leave the arm setter on its other branches.

#### Scenario: Unsupported but not off balance
- **WHEN** a body that is not off balance by the posture block is unsupported
- **THEN** its arms do not take the off-balance stiffness

### Requirement: Dormant lying capture
A dormant body lying still SHALL not get up: its tone and energy SHALL be 0 and its whole pose SHALL be captured.

#### Scenario: A dormant body lies
- **WHEN** a dormant body's posture stage runs while it lies still
- **THEN** its tone and energy are 0, its condition stays down and a whole capture is pending

### Requirement: Get-up heading
A get-up SHALL compare its rising direction with the body's locomotion heading (the root's facing without one) and turn only when they are more than about 18 degrees apart; a get-up playing with a NaN progress SHALL keep playing.

#### Scenario: Heading along the rise
- **WHEN** a body's heading is within 18 degrees of its rise but its root faces 90 degrees away
- **THEN** the get-up does not turn it

### Requirement: Femur posture order
The femur posture turn SHALL be applied after each thigh's clip turn (the posture turn times the clip turn), with the engine's product sums.

#### Scenario: A splayed thigh
- **WHEN** a thigh's clip turn is not about the posture axis
- **THEN** its posed turn is the posture turn times the clip turn

### Requirement: Held items share their holder's sleep
Before the solver, a linked held item and its holder SHALL take the larger of their two sleep countdowns; an item whose holder sleeps SHALL fall asleep, and a touched item SHALL wake its sleeping holder. A released item SHALL keep its own sleep.

#### Scenario: A sleeping holder
- **WHEN** a holder and its held item are both asleep and the item is touched
- **THEN** the holder wakes with the item's countdown

### Requirement: Height Query start on a face
A Height Query sphere that starts touching or inside a face SHALL not meet it, as the original's swept nearest test takes only hits past the start.

#### Scenario: A probe on the floor
- **WHEN** a probe's lowest point starts on the floor
- **THEN** the query meets nothing there, and a millimetre above it finds the floor
