## ADDED Requirements

### Requirement: A scripted run ends after its frames
With `--frames n` and no `--screenshot`, a run SHALL end after n frames counted from the end of the load, as the screenshot's frames are counted.

#### Scenario: A grid scene
- **WHEN** a grid quadrant runs `play arena --opponent --fixed-frames --frames 1800`
- **THEN** its game logs that 1800 frames were played and exits with code 0
