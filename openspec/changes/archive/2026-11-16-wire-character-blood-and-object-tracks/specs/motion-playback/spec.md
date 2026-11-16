## ADDED Requirements

### Requirement: Used object plays its tracks
The player's use of an object SHALL start the keyframe tracks of the used node and of the nodes under it, and the tracks SHALL play from that frame by the track clock and curve.

#### Scenario: The secret switch
- **WHEN** the player double-clicks the secret switch's object
- **THEN** its brick sinks 2.8125 cm 0.5 s in, and 3 s on it rests where it started and leaves the playing list
