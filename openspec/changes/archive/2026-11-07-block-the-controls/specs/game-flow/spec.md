## ADDED Requirements

### Requirement: Death and knock-out block the controls
The flow SHALL block the player's controls at the knock-out (after its tip), at the death (after its tip and the save's fate, before the end menu), and at the practice ladder's last win in the practice mode (after its tip and settings write), and SHALL release them at the revive after the wake is held.

#### Scenario: Knock-out steps
- **WHEN** the story player is knocked out
- **THEN** the steps hide the game view, take the controls, show tip 2 and block the controls, in that order
