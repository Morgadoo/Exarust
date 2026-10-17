## ADDED Requirements

### Requirement: Collapsing body
While a living body collapses its frame SHALL follow its pelvis exactly every tick, its lift SHALL be multiplied by its tone squared (beside 2(t − 0.5)), its feet SHALL be probed as standing, its support pins' gain SHALL not recover, and its clip SHALL run at 0.6 of the base rate a tick. A living body's pins' gain SHALL start at 0.08 and recover 0.001 a tick while under the double 0.08, settling at 0.081.

#### Scenario: Lift
- **WHEN** a standing body's lift at full tone is f, and it collapses at tone 0.9 and 0.75
- **THEN** its lift is 0.648·f, then 0.28125·f, and none at tone 0.5

#### Scenario: Frame
- **WHEN** a collapsing body's pelvis moves
- **THEN** its frame's origin follows it the same tick, at any balance and support

## MODIFIED Requirements

### Requirement: Going limp
The system SHALL turn a posed body asked to go limp into a ragdoll from its current pose, stop animating it and remove its character controller. It SHALL keep the motion it has unless the request gives it a velocity.

#### Scenario: Corpses
- **WHEN** a level places a dead character
- **THEN** its body collapses where it was placed

#### Scenario: Character view
- **WHEN** `exarust --limp character` runs
- **THEN** the body drops to the floor in a sprawl with its parts joined

#### Scenario: Death
- **WHEN** a collapsing body dies at a tone of 0.25
- **THEN** it goes limp the same tick with the velocity its parts have, none added

### Requirement: Arm stiffness
The arms' stiffness SHALL be 0.2 while the body collapses, 0.5 off balance (balance under 0.6, falling, lying or getting up), 0.2 at tone under 0.85, the character's walking arms (0.2 unless feral) in the walk and run cycles, and otherwise x ← 0.8x + 0.08·(0.5 + 0.5·race arm relaxation), never under min(0.4·|turn left in rad|, 0.4). An arm part's anchor relaxation SHALL be capped at 0.6 − (x − 0.1)·0.8.

#### Scenario: Idle human arms
- **WHEN** an alert human stands still
- **THEN** its arms settle at stiffness 0.4 and pull with σ = 0.25, their relaxation capped at 0.36

#### Scenario: Walking and off balance
- **WHEN** the human walks, and later falls
- **THEN** its arms pull with σ = 0.175 walking and 0.3125 off balance

#### Scenario: Zombie arms
- **WHEN** a fully feral human walks, and is then dazed to tone 0.8
- **THEN** its arms' stiffness is 0.05 walking and 0.2 dazed

#### Scenario: Collapsing arms
- **WHEN** a body collapses, off balance, at tone 0.6
- **THEN** its arms' stiffness is 0.2
