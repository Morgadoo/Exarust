## MODIFIED Requirements

### Requirement: Kept bodies reset
A body the next round keeps SHALL be reset: yellow health takes red, the shield arm is full, the condition is 0, the standby behaviour, and for anyone but the player the AI's clocks are held 10000 ms with the first a further high share of the stepped roll times 10000 and its combat target is forgotten. The kept opponent SHALL be reset before the continuing fighter.

#### Scenario: The opponent who won stays
- **WHEN** a reserve round follows a lost round whose fighter is dead
- **THEN** the opponent who won is reset with its clocks held from the round's start

## ADDED Requirements

### Requirement: Kept bodies placed
A kept body SHALL restart motion 3 with no blend and no lead-in from its held pose and be put living at its round place: the opponent's place for a kept opponent, the company's facing −z for the continuing fighter, the bracket place for a tournament's player, its pelvis at the fresh pose's stand height over the world's origin.

#### Scenario: A kept opponent
- **WHEN** a kept opponent's round place is (300, 5, −200) cm facing −z
- **THEN** its motion restarts, its pelvis stands at (3, −2) m at its stand height and it faces −z
