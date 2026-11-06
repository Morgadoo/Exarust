## ADDED Requirements

### Requirement: Player input ends the narrator's offer
While the narrator's reply panel offers at least one choice, the player behaviour tick SHALL end every narrator conversation on the next update when the player holds the left or right button over the game view, holds a movement key (forward, left, back, right) or pushes the movement stick past 0.1 squared.

#### Scenario: A step away
- **WHEN** the narrator's panel offers a choice and the player holds forward
- **THEN** the narrator's conversations end

#### Scenario: Another speaker
- **WHEN** a character's panel offers a choice and the player holds forward
- **THEN** nothing ends
