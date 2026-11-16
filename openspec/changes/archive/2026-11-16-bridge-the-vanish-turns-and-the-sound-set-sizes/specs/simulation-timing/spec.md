## ADDED Requirements

### Requirement: Moved systems find their data
Every message and resource a system moved into the simulation step reads or writes SHALL exist in the simulation world (bridged, or moved there), so no moved system is skipped for a missing message queue or resource.

#### Scenario: A Vanish in the arena
- **WHEN** the player casts Vanish in `play arena --opponent`
- **THEN** the casters' walk runs in the simulation step and the body vanishes, comes back and shows
