# Spec Delta

## ADDED Requirements

### Requirement: Death fade clock
The death fade's time SHALL be the body ticks' game clock, starting at the pit tick's own clock, so the fade completes only after the tick that kills the player. The world's sounds SHALL follow it as 1 − the share, and the game SHALL pause once when the share passes 1. The end-of-session menu SHALL draw over the black.

#### Scenario: The death comes first
- **WHEN** the pit falls on tick 84 and the frames read the game clock at any share of the next tick
- **THEN** no frame pauses the game before the counter has passed the death tick (334)

#### Scenario: A pit death ends in the end menu
- **WHEN** `exarust --story-session --at 10.5,200,65 play exanima06` drops the player
- **THEN** the player dies 4,008 ms after it is lost, the game pauses after it, and the end menu's EXIT and "You have died." show over the black

### Requirement: Death fade reset
Leaving the session, the story's end-menu exit and the story's respawn SHALL reset the death fade, the exit and the respawn before anything else: a fade that paused the game gives the game time back, and the black goes.

#### Scenario: The reset gives the time back
- **WHEN** a completed fade has paused the game and the story's end-menu exit runs
- **THEN** the game time runs again and the black is gone; a pause the fade did not make stays
