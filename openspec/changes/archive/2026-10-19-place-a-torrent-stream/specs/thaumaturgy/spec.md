# Spec Delta

## ADDED Requirements

### Requirement: Torrent streams
A torrent SHALL round eight tenths of its span to the nearest even integer. A stream SHALL sit at 16 times its direction, then its height times 1.5 minus 30. The mesh SHALL ask for 13 groups for each stream plus 8, 12 bytes a group, and 8 indices a group.

#### Scenario: Two streams and a dropped birth
- **WHEN** spans of 1, 2 and 3 are counted, a stream along height 1 is placed, and a mesh of 2 streams is sized
- **THEN** the counts are 1, 2 and 2, the stream is at height -6, and 2 streams ask for 10 records, 130 groups, 1560 bytes and 1040 indices

### Requirement: Torrent push
A torrent reach SHALL be the radius times the scale, plus the margin, plus 10. A hit SHALL raise the first channel by 0.005 of the age over the sample count, and the second by 0.01 of that, while the second is under 1.8. A point inside the reach SHALL move by 0.01 of one minus its distance squared over the reach squared. A point on the reach SHALL stay. A point past it SHALL be missed.

#### Scenario: A reach, a hit, and a push
- **WHEN** a radius of 2 at scale one half and margin 1 is reached, age 1 of 2 samples raises channels at 0, the second channel at 1.8 still rises and the next float does not, and a point at the origin is pushed along a unit step inside a reach of 10, on that reach, and past it
- **THEN** the reach is 12, the channels are 0.0025 and 0.005, the inside point moves to 0.01, the point on the reach stays, and the point past it is missed
