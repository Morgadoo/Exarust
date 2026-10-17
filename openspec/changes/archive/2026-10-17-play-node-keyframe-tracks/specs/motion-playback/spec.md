## ADDED Requirements

### Requirement: Node keyframe tracks
The system SHALL decode the keyframe tracks a stored scene node carries after its body, whatever its kind: the play mode (free, once, reverse, loop; any unknown word free), the duration and the keys of time (seconds) and position (the node's translation in its parent's space, metres). A track that does not decode SHALL keep its error and nothing SHALL panic. It SHALL hold for every chunk tree of the installed packs, nested packs included.

#### Scenario: Shipped tracks
- **WHEN** every chunk tree of the installed packs is decoded
- **THEN** exactly one track is found, on node `Box001` under `Plane001` of `uwa01 brkswtch` in `Objects.rpk`: once, 3 s, keys at 0, 1, 2 and 3 s with z at 0, −5, −5 and 0 cm, and no other node kind than a mesh carries a collision body or cloth marker

#### Scenario: Truncated track
- **WHEN** a track's keys run past the end of its chunk
- **THEN** the node keeps an error for that track and the rest of the file decodes

### Requirement: Keyframe curve
The system SHALL place a track's node at curve time t: on the first key's position up to its time, on the last key's from its time on, and between them on the cubic Hermite curve of the bracketing keys, each inner key's slope the chord of its neighbours scaled by the segment's share of their span; the first segment SHALL leave its first key with no slope and the last segment SHALL arrive along its own chord.

#### Scenario: The brick's curve
- **WHEN** the shipped track is sampled at 0.5, 1.5 and 2.5 s
- **THEN** the node's z is −2.1875, −5.625 and −2.8125 cm

#### Scenario: Outside the keys
- **WHEN** a track is sampled before its first key or after its last
- **THEN** the node takes the first or the last key's position

### Requirement: Playing keyframe tracks
Starting a node SHALL start every track of it and of the nodes under it that is not playing, at the clock's time, and swap once and reverse. Each frame a playing track's node SHALL take the curve's position at the time since its start: as is (free), clamped at the duration (once), counted down from it (reverse), wrapped by it (loop). Once and reverse SHALL end at their limit, leaving the node there. The clock SHALL stand still while the game is paused.

#### Scenario: First and second press
- **WHEN** the secret switch's object is started, started again while its brick moves, and started once more after the brick stopped
- **THEN** the first play runs in reverse (−2.8125 cm 0.5 s in) and ends 3 s later at rest, the start during it changes nothing, and the last play runs forward (−2.1875 cm 0.5 s in)

#### Scenario: Placed object
- **WHEN** a placed object whose child node carries a track is started
- **THEN** the child node moves along the curve in its parent's space and the object's placement stays

#### Scenario: Debug start
- **WHEN** F7 is pressed in a level
- **THEN** every keyframe track of the level starts and the log names each with its mode, duration and position
