# Spec Delta

## MODIFIED Requirements

### Requirement: Light comparison options
The viewer SHALL offer, as debug options: `--debug-lights M`, which logs once, two seconds into play, every point light within M metres of the player with its colour, intensity, range, GI multiplier and shaping, plus the GI base ambient and the scene ambient; `--no-torch`, which starts play without the carried torch; and `--floor-torch x,y,z[,rx,ry,rz]`, which lays a lit torch at that point once the player stands in the level, on its side or, with six values, in the frame those X, Y, Z angles (degrees) give.

#### Scenario: The start room's lights
- **WHEN** exanima01 is played with `--debug-lights 15 --no-torch`
- **THEN** the log lists the pool light (shaping (0, 16), intensity 60) and the fill light (intensity 0.1, GI multiplier 0) over the spawn, and no torch fire

#### Scenario: A torch laid as retail's
- **WHEN** exanima01 is played with `--no-torch --floor-torch 80.45,0.2,40.88,99,0,0`
- **THEN** the torch lies along the world's Z with its flame end on the floor
