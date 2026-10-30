## ADDED Requirements

### Requirement: Light range
A level light SHALL fall off with distance only when its node carries the range flag (`0x40000`) and its stored radius is above zero, as the original's light activation does. Any other light, a NaN radius included, SHALL reach everywhere at full strength. This SHALL hold for every light in the installed levels and object library.

#### Scenario: The flag without a radius
- **WHEN** a light node carries the range flag and stores a radius of 0
- **THEN** its light does not fall off with distance

#### Scenario: The start room's fill
- **WHEN** exanima01's start room fill light (white, intensity 0.1, radius 2450 cm, range flag) is loaded
- **THEN** it falls off to nothing at 30.625 m
