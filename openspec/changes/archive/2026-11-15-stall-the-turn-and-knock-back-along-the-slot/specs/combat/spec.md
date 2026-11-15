## ADDED Requirements

### Requirement: An interrupted swing stands still
A hard weapon-body hit that interrupts a swing SHALL set its stall rate to 1 and its strength to 0, so its clip stands still until the stance returns 600 ms on; a stab's or overhead's hard hit SHALL set the strength to 0 and leave the rate.

#### Scenario: An overhead's hard hit
- **WHEN** an overhead's weapon body strikes a thrown body hard with its window open
- **THEN** the swing's strength is 0 and the striker's turn stall is the recorded strength

## MODIFIED Requirements

### Requirement: Knockback step
A blow whose impact J is over 0.5 SHALL start a knockback step of the striker's slot vector (its tracked velocity in cm a tick, not normalised, its height zeroed) times 2·(20·strikes + 10)·J, plus 20 ticks of the chest's fast velocity, capped at the step length. Strikes SHALL be 0 until that power exists. Impact of 0.5 or less SHALL NOT start that step.

#### Scenario: A full impact
- **WHEN** impact is 1, strikes are 0, the slot vector is 1 cm a tick along the swing and the chest is still
- **THEN** the step is 20 cm along the swing

#### Scenario: Force strikes
- **WHEN** impact is 1 and strikes are 1, with a slot vector of 1 cm a tick and a still chest
- **THEN** the step is 60 cm along the swing

#### Scenario: At the line
- **WHEN** impact is 0.5
- **THEN** no knockback step is started

#### Scenario: A moving chest
- **WHEN** impact is 1, strikes are 0, the slot vector is 1 cm a tick and the chest moves 1 cm a tick along the swing
- **THEN** the step is 40 cm along the swing

#### Scenario: A fast swing
- **WHEN** impact is 0.6 and the striker's slot moves (9, 4, 12) cm a tick
- **THEN** the step is the whole step length along (0.6, 0, 0.8)
