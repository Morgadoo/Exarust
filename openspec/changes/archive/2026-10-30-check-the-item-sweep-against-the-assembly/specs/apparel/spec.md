# Spec Delta

## ADDED Requirements

### Requirement: Palette ramp precision
The palette ramps SHALL take the family step times 0.13 in double precision stored as a float and multiply it by k = 7 − shade in single precision; the linear ramp SHALL add the base in single precision and the lifted ramp SHALL add base + 0.1 and square in double precision. The value SHALL be capped at 1 by an ordered compare, so a not-a-number value is kept.

#### Scenario: The lightest linear shade
- **WHEN** family 1 (base 0.2, step 0.8) is evaluated at shade 0 on the linear ramp
- **THEN** its value has the bits 0x3f6d9169

### Requirement: Palette saturation precision
The palette saturation SHALL be (1 − (0.09k)²) times the family's in double precision, and below a value of 0.4, compared in double precision, it SHALL be multiplied by (v + 0.6)² in double precision.

#### Scenario: The darkest shade's saturation
- **WHEN** family 1 is evaluated at shade 7 on the linear ramp
- **THEN** its value is 0.2 and its saturation has the bits 0x3e33e71b

### Requirement: Paint draw texture units
The paint pass draws SHALL bind every one of a material's layer textures, layer i at texture unit 2 + i (3 + i in the normal draw), and the pattern and painted-pattern draws SHALL bind the item's pattern picture at the colour layer count plus two. The normal draw SHALL use the alternate-normal program when the item's flag word has the mask 8.

#### Scenario: Steel's colour layers
- **WHEN** steel's colour pass is drawn
- **THEN** its five layers bind at units 2 to 6 and a pattern picture would bind at unit 7

#### Scenario: Steel's normal layers
- **WHEN** steel's normal pass is drawn
- **THEN** its first layer binds at unit 3
