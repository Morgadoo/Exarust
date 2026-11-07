## ADDED Requirements

### Requirement: Controls off under a form
The player's controls SHALL be off from their creation, on from the Character or Spectate mode's start, off at its end, and off while the skill screen, the power tree or the arena arsenal is open. While off, the key handler, the pad routine, the movement request, the behaviour's inputs, the mouse capture and the idle camera SHALL take no input; turning them off SHALL clear the move request and the locomotion flags once.

#### Scenario: Walking with the skill screen open
- **WHEN** the skill screen is open and the forward key is held
- **THEN** the player does not move, and after the screen closes the key walks again

#### Scenario: The skills key while open
- **WHEN** the skill screen is open and its key is pressed
- **THEN** nothing happens; the right button still closes it
