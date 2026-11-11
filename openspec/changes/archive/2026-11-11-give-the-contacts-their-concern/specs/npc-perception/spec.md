## ADDED Requirements

### Requirement: The player's contacts
The player's behaviour SHALL keep, each player tick, a list of the characters within 12 m (twelve at most) with each one's closeness, its facing weight as the view and the list's score as the concern, and the NPC perception's threat estimate SHALL read that list when the observed character is the player.

#### Scenario: A hostile NPC faced by the player
- **WHEN** a player 300 cm from a hostile NPC faces it
- **THEN** the NPC's contact for the player gets a concern target and an alarm above 0

### Requirement: The facing weight's direction
The player's contact list SHALL measure the direction from the other character to the player, so a character straight ahead of both the body and the cursor has a facing weight of 1 and one behind both 0.

#### Scenario: Ahead and behind
- **WHEN** one character stands 100 cm ahead of the player and another 100 cm behind, the cursor ahead
- **THEN** the first weighs 1 and the second 0

### Requirement: Contact writes reach the brain
A concern or hostility written on a brain's combatant contact after the brain's AI tick copied its own there (a hit reaction, an `AddConcern`) SHALL become the brain's value before its next integration and target pass.

#### Scenario: Two hits
- **WHEN** the player wounds a duel brain twice
- **THEN** the second hit's concern rise starts from the first's result and the brain's pass reads the sum
