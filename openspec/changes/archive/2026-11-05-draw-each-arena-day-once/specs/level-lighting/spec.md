# Spec Delta

## ADDED Requirements

### Requirement: One arena day per load
A level load SHALL take its outdoor preset seed once and draw every pass of that load (the visibility rules while models spawn and the lighting) from it. The seed SHALL be the low 32 bits of a 100 ns counter with their four bytes reversed, unless `EXA_SCENE_SEED` fixes it. Nothing of the arena career (rank, day, matches fought) SHALL enter the draw; only the arena's night flag does.

#### Scenario: Two passes of a load
- **WHEN** an outdoor arena loads with seed 24
- **THEN** both passes see the same time of day, daylight scale and rain target

#### Scenario: Counter word
- **WHEN** the counter's low word is 0x12345678
- **THEN** the seed is 0x78563412

### Requirement: Time of day stands still unless driven
The time of day SHALL stay where the load's draw put it, the scene environment's time speed being zero as the shipped game leaves it. With `EXA_TIME_SPEED=<s>` the weather tick SHALL advance it by `dt × s` and re-place the sun or moon, and log the time, daylight, sun direction and colour, ambient and fog every 120 frames. With `EXA_TIME_OF_DAY=<t>` the load SHALL place the body at `t` after the draw, keeping the draw's weather.

#### Scenario: Driven into the night
- **WHEN** a clear afternoon on `arenao3` runs at 0.01 days a second for 51 s
- **THEN** the time has moved 0.51 days and the daylight has fallen below 0.5

#### Scenario: Not driven
- **WHEN** no time speed is set
- **THEN** the time of day after a minute of play equals the draw's
