## ADDED Requirements

### Requirement: Light animation parameters
A light body whose flags carry 0x10000 or 0x20000 SHALL be read with four more words after its shaping words: one skipped, then the flicker amplitude, speed and emissive share, and the node's sub-chunks after them. Other light bodies SHALL carry no parameters.

#### Scenario: A flickering light
- **WHEN** a light body with flag 0x10000 ends with the words 0, 0.25, 0.5, 0.75 and a key-track sub-chunk
- **THEN** its amplitude is 0.25, its speed 0.5, its emissive share 0.75, and the key track is read
