## ADDED Requirements

### Requirement: Journey level exit
A teleporter journey's level change SHALL pass no flags: no portal use, no AI hand-over and no long transition hold. A level exit with the portal bit (the level portals' zone exits) SHALL ask for the portal use on the player and the AI hand-over and set the long hold.

#### Scenario: A journey to level 7
- **WHEN** a journey's resting bubble shrinks away and asks for level 7
- **THEN** the level exit's steps hold no portal use and no AI hand-over, and the transition uses the 600 ms fade
