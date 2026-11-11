## ADDED Requirements

### Requirement: Grid batch exit
With `--exit-when-done` and no `--restart`, the grid host SHALL exit once every quadrant that was given a scene has a game that ended.

#### Scenario: Four scenes with their own frame counts
- **WHEN** a grid runs four scenes with `--frames` and `--exit-when-done`
- **THEN** the host exits once the last of the four games exits, without a screenshot
