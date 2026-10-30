# Spec Delta

## MODIFIED Requirements

### Requirement: Alarm return
The alarm return SHALL offer each AI body of the 3000 cm radius query around the player to the homeward callback, which SHALL send an awake guardian not already returning home (its home filled from the body when the home's x is exactly 0, speed words 1) and asleep. When one went home, the `Alarm_` node holding the armed point SHALL place the player 200 cm past the box's far face along its third axis at height 10, facing the axis's heading plus π plus a draw less one half.

#### Scenario: The post past the box
- **WHEN** the alarm box is centred at (100, 50, 0) with a third half extent of 25 along z and the draw is 0.5
- **THEN** the player is put at (100, 10, 225) facing π

#### Scenario: Nobody in reach
- **WHEN** the only guardian stands exactly 3000 cm from the player
- **THEN** nobody is sent home and the player is not moved
