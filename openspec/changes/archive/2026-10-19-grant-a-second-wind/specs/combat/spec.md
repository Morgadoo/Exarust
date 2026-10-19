# Spec Delta

## ADDED Requirements

### Requirement: A second wind fires instead of the heartbeat
When a blow leaves a fighter alive under 0.05 yellow, a granted second wind whose ready clock is behind the game clock SHALL fire, wait 180000 ms, and skip the heartbeat message. At that ready clock it waits and the heartbeat message is written. The heal sound uses volume bits 0x3dcccccd and pitch bits 0x3f800000.

#### Scenario: Clock 12 fires a wind that was ready at 11
- **WHEN** a parried fighter already under 0.05 yellow has a granted wind ready at 11, the game clock is 12, and the hit path runs, and the same blow runs with the wind ready at 12
- **THEN** the first fires, waits until 180012, writes no heartbeat message, and uses volume bits 0x3dcccccd and pitch bits 0x3f800000, and the second writes the heartbeat message and does not fire
