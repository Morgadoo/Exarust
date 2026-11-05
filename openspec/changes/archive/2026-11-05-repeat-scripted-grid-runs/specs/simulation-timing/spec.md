## ADDED Requirements

### Requirement: Scripted grid runs repeat
A grid quadrant running with fixed frames SHALL ignore the host's forwarded input, so that two runs of the same scripted scene repeat whatever the host's mouse does.

#### Scenario: Four duels in one grid
- **WHEN** the arena duel runs twice in one world and twice behind the bridge in one grid, with fixed frames
- **THEN** over their first 100 traced ticks the joint angles differ by no more than a hundredth of a degree on average
