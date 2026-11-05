## ADDED Requirements

### Requirement: Rise in the simulation
A dead character due to rise SHALL be revived, and its clips and controller given back, in the simulation's once-a-frame work before the frame's ticks, alike with and without the bridge.

#### Scenario: A corpse past its clock
- **WHEN** a dead body with the rise rule and enough health passes its revival time
- **THEN** after the next frame's ticks it is down, has its muscles, clips and controller back, and has no death clock, in one world and behind the bridge alike
