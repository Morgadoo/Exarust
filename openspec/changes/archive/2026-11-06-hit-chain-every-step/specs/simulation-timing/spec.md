## MODIFIED Requirements

### Requirement: Hit chain in the simulation
The hit chain SHALL run every simulation step, in the step's post-step, from the strikers' and the targets' frames where that step left their bodies, followed by the systems that react to its results; the blows it sends SHALL be dealt first in the next step, alike with and without the bridge.

#### Scenario: A scripted walk
- **WHEN** the arena runs a scripted walk with fixed frames, in one world and behind the bridge
- **THEN** every joint of every traced tick is the same in both runs

#### Scenario: A duel
- **WHEN** the arena duel runs with fixed frames in one world and behind the bridge
- **THEN** the same hits land at the same ticks with the same damage, and every joint of every traced tick is the same

#### Scenario: A blow in a frame without a tick
- **WHEN** a blow is sent in a frame that runs no tick
- **THEN** it is dealt by the next tick

### Requirement: Once-a-frame simulation work
Work the fixed step does once at the start of each frame SHALL run in the simulation's world behind the bridge, at the first step after a frame began, and give the same results as before the frame's fixed loop in one world.

#### Scenario: Tick counts between frames
- **WHEN** bodies tick several times in a frame
- **THEN** each body's poses match the one-world run step for step
