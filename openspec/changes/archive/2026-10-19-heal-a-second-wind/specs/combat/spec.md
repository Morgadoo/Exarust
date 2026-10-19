# Spec Delta

## ADDED Requirements

### Requirement: An active second wind heals yellow
While a second wind is active, one living health tick SHALL add the double 0.0003 to yellow and store the float. Yellow bits 0x3d000000 become 0x3d013a93. The wind stays active at game clock 3012 when its ready clock is 180012, and at 3024 it still adds and then ends. A quiet wind and a dead fighter add nothing.

#### Scenario: Clock 3024 ends a wind that was ready at 180012
- **WHEN** an attacking fighter with yellow bits 0x3d000000 and an active wind ready at 180012 is ticked at game clock 12, at 3012, and at 3024, and the same tick runs for a quiet wind and for a dead fighter
- **THEN** the first three yellow bits are 0x3d013a93, the first two stay active, the third ends, and the quiet and dead fighters keep yellow bits 0x3d000000
