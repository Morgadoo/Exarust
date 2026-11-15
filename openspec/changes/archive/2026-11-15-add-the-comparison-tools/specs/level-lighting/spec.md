## MODIFIED Requirements

### Requirement: One arena day per load
A level load SHALL take its outdoor preset seed once and draw every pass of that load (the visibility rules while models spawn and the lighting) from it. The seed SHALL be the low 32 bits of a 100 ns counter with their four bytes reversed, unless `--env-seed <n>` or `EXA_SCENE_SEED` fixes it. Nothing of the arena career (rank, day, matches fought) SHALL enter the draw; only the arena's night flag does.

#### Scenario: Two passes of a load
- **WHEN** an outdoor arena loads with seed 24
- **THEN** both passes see the same time of day, daylight scale and rain target

#### Scenario: Counter word
- **WHEN** the counter's low word is 0x12345678
- **THEN** the seed is 0x78563412

## ADDED Requirements

### Requirement: Fixed outdoor seed
With `--env-seed <n>` (a nonzero word) every outdoor preset draw of the run SHALL use `n`: the load's preset seed, the engine stream's seed and the preset's fresh-seed fallback, so two runs of an outdoor level give the same day and weather. Zero SHALL be refused, as it is the original's word for a fresh seed.

#### Scenario: Two loads with one seed
- **WHEN** the seed is fixed and `arenao4`'s lighting is drawn twice
- **THEN** both draws have the same time of day, daylight scale, rain target and cloudiness
