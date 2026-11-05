# Spec Delta

## MODIFIED Requirements

### Requirement: Light comparison options
The viewer SHALL offer, as debug options: `--debug-lights M`, which logs once, two seconds in, the point lights within M metres of the player (the camera in a level view) with their colour, intensity, range, GI multiplier and shaping, and the GI and scene ambients; `--no-torch`, which starts play without the torch; and `--floor-torch x,y,z[,rx,ry,rz]`, which lays a lit torch at that point once the player stands in the level, on its side or, with six values, turned by those X, Y, Z degrees.

#### Scenario: The start room's lights
- **WHEN** exanima01 is played with `--debug-lights 15 --no-torch`
- **THEN** the log lists the pool light (shaping (0, 16), intensity 60) and the fill light (intensity 0.1, GI multiplier 0) over the spawn, and no torch fire

#### Scenario: A torch laid as retail's
- **WHEN** exanima01 is played with `--no-torch --floor-torch 80.45,0.2,40.88,99,0,0`
- **THEN** the torch lies along the world's Z with its flame end on the floor

#### Scenario: The menu's lights
- **WHEN** the level view shows menuscene with `--debug-lights 30`
- **THEN** the log lists its two fires and the omni light around the camera

## ADDED Requirements

### Requirement: Fire light pin
With `EXA_FIRE_LIGHT=<f>` every fire light SHALL stay at f × its template intensity instead of following its emitter's density and flicker, while the flames keep running.

#### Scenario: A held fire light
- **WHEN** the menu scene runs with `EXA_FIRE_LIGHT=1.159`
- **THEN** both torch lights stay at intensity 4.147
