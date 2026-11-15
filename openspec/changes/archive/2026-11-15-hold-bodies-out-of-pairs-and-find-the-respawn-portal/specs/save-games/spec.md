## MODIFIED Requirements

### Requirement: Pending respawn on load
A story load of a save whose transition word is 2 (a respawn pending) SHALL set the word to 1 and load the level with the session-start arrival word. When the level names a respawn portal (its reconstruction device) the player SHALL stand at its frame 15 cm up and settle 16 ticks; otherwise the saved placement SHALL stand and nothing settles. Any other save SHALL settle nothing.

#### Scenario: A save written while a respawn was pending
- **WHEN** such a save is continued
- **THEN** the continue's placement is followed by the portal placement with 16 ticks of settling, and the loaded session's transition word is 1
