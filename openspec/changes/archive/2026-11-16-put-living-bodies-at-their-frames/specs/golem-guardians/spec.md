## MODIFIED Requirements

### Requirement: Alarm return in the story revive
The story revive of a knocked-out player (five seconds after its body died with red health of at least 0.01) SHALL run the alarm return after its prelude closes and before its save and the body's revive. A guardian sent home and the player put outside SHALL be put living at their places, the pelvis at the pose's pelvis target over the world's origin, and woken for 32 ticks. With its statue byte set again, a guardian's sense SHALL clear its concern for the player, so it leaves combat.

#### Scenario: Golems sent home in play
- **WHEN** the player walks into the gate of pass 474 without its token, the two golems that wake fight it, and it falls with red health left
- **THEN** five seconds later both golems stand at their homes as statues at rest and the player lies 200 cm past the gate box's third-axis face

#### Scenario: The revive's order
- **WHEN** the story revive runs
- **THEN** the alarm return comes after the prelude closes and before the save and the body's revive

#### Scenario: A golem sent home is placed living
- **WHEN** a golem whose home is (900, 0, 7500) cm facing 2 rad is sent home
- **THEN** it is put living at (9, 75) m facing 2 rad at its pelvis target, its held items re-placed
