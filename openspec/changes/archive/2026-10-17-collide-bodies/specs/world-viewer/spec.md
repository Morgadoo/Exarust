## ADDED Requirements

### Requirement: Debug corpses
In play mode, `exarust --corpse x,y,z` SHALL lay a dead human of a new record down at that point (metres), as a level's placed dead lie down; the flag MAY be given more than once.

#### Scenario: A corpse walked into
- **WHEN** `exarust --corpse 0,0,3.4 --press W@150-420 play arena` runs
- **THEN** a limp body falls and counts down to sleep where it was laid, and the player's lower legs meet it and wake it

## MODIFIED Requirements

### Requirement: Prop shove
`--shove-props <frame>` SHALL, at that frame after the load in play mode, deal every prop body within 3 m of the player a blow of 1.5 m/s away from it and 1.5 m/s up, log how many, and once all of them sleep again log after how many ticks and how far each moved. `--shove` SHALL deal its blows to character bodies only.

#### Scenario: Arena tavern table
- **WHEN** `exarust --at 11.2,4.5,12.8 --shove-props 120 play arenao1` runs
- **THEN** 8 prop bodies are shoved (the cups, bowls, tankards and spoons on the tavern table, which is level geometry) and all sleep again about 320 ticks later, six back on the table and two 0.8–0.9 m lower on the floor
