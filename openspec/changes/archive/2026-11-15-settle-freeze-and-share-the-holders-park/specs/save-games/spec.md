## ADDED Requirements

### Requirement: Pending respawn on load
A story load of a save whose transition word is 2 (a respawn pending) SHALL set the word to 1, place the player at the respawn portal 15 cm up when a portal is known (at the saved placement otherwise) and give the placed player 16 ticks of settling. Any other save SHALL settle nothing.

#### Scenario: A save written while a respawn was pending
- **WHEN** such a save is continued
- **THEN** the continue's placement is followed by 16 ticks of settling and the loaded session's transition word is 1
