## ADDED Requirements

### Requirement: Light comparison options
The viewer SHALL offer, as debug options: `--debug-lights M`, which logs once, two seconds into play, every point light within M metres of the player with its colour, intensity, range, GI multiplier and shaping, plus the GI base ambient and the scene ambient; `--no-torch`, which starts play without the carried torch; and `--floor-torch x,y,z`, which lays a lit torch on its side at that point once the player stands in the level.

#### Scenario: The start room's lights
- **WHEN** exanima01 is played with `--debug-lights 15 --no-torch`
- **THEN** the log lists the pool light (shaping (0, 16), intensity 60) and the fill light (intensity 0.1, GI multiplier 0) over the spawn, and no torch fire
