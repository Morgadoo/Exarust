## ADDED Requirements

### Requirement: Expert unlock write
In the practice mode, a novice-ladder win SHALL show tip 5 and, when the expert unlock was not already stored or written, write the 4-byte value 1 under settings path [1, 0x6431ac01] once and flush the store; an expert-ladder win SHALL show tip 6 and write nothing; outside the practice mode neither tip nor write SHALL be made.

#### Scenario: Two novice wins
- **WHEN** a practice player wins the novice ladder twice with no stored unlock
- **THEN** the first win writes [1, 0, 0, 0] under [1, 0x6431ac01] and the second writes nothing

#### Scenario: An arena-mode round end
- **WHEN** the practice round's last fight ends while the mode is the arena
- **THEN** no tip 5 and no settings write are made

### Requirement: Flow action switch
Each motion the flow asks for (the story start's, the delayed wake's, the respawn's and the practice start's and next fight's) SHALL start the numbered action on the player's animator unless the player is already in that action, looping when its clip loops and otherwise playing once back into the combat or idle action.

#### Scenario: A body already in the action
- **WHEN** the practice motion is asked for while the player's animator already runs that action
- **THEN** the animator is left as it is

### Requirement: Practice spawn point
The practice start and each next fight SHALL place the player at x 0, z 500 cm with the spawn table's half turn, replacing only the vertical with the body's height.

#### Scenario: A practice start
- **WHEN** a practice start runs with a standing height of 92 cm
- **THEN** the player is placed at (0, 97, 500)
