# Spec Delta

## ADDED Requirements

### Requirement: The brains' tick
The AI (the brains, the duellists and their walks) SHALL run once per body tick, after the presence check and before the character controllers move and the character ticks run. A frame that runs several body ticks SHALL run it once for each, and a frame that runs none SHALL not run it, so its timings count body ticks whatever the frame rate.

#### Scenario: Passes per body tick
- **WHEN** a lone brain duels the player in the arena at about 30 frames a second and at 60 frames a second of game time
- **THEN** it has its target pass every other body tick in both runs, and enters combat the same number of body ticks after its first pass

### Requirement: The target pass's counter
The target pass's counter SHALL be the game loop's step counter as a tick's brains read it: the body ticks run so far plus one.

#### Scenario: The counter is the body tick
- **WHEN** the rotation runs with 41 body ticks done, again on the same tick, then with 45 done
- **THEN** the counter reads 42, stays 42, then reads 46
