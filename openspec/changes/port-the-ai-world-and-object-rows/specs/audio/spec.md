## ADDED Requirements

### Requirement: A story track plays on through a door
A story level's track SHALL be played only when it differs from the track playing on their first 15 characters; a story door change SHALL leave the music to its transition (which stops the track only when the destination's differs), so a level whose track plays already keeps it playing.

#### Scenario: Two levels with the same track
- **WHEN** a story level whose track is the one playing loads
- **THEN** the track is not stopped or started again
