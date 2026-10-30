## ADDED Requirements

### Requirement: Particle frame light
The system SHALL work out a particle emitter's light once a rendered frame: intensity density·((1 + 0.05x)·base) and emissive scale 0.8·density·(0.9 + 0.2x) in double precision, with a full light update on the first frame, on every frame of a one-shot template and on a frame strictly brighter than the peak, which becomes 1.1× that intensity.

#### Scenario: A walked light
- **WHEN** a small fire's emitter has density 0.7 and walk 0.3
- **THEN** its emissive scale is 0.5376

### Requirement: Particle frame sound walk
The system SHALL walk an emitter's sound once a rendered frame, with volume bounds 0.8 and 1.2 × the template volume worked out and compared in double precision, keeping a NaN volume.

#### Scenario: A volume past its bound
- **WHEN** a template of volume 0.7 walks a volume of 10
- **THEN** the volume becomes the double product 0.7·1.2 narrowed once

### Requirement: Particle level of detail
The system SHALL give each fire emitter the original's detail value (the small and 0.5–1 flames' size, twice it for the 0.35–0.5 and large flames, four times the tier's size for every bed) and SHALL compute the level of detail as detail·1000 over the squared distance from the camera to the emitter's node, held at one or more.

#### Scenario: A camera ten centimetres away
- **WHEN** an emitter of detail 0.4 is seen from 10 cm
- **THEN** its level of detail is 4

### Requirement: Fire loop sound
The system SHALL play a fire template's named loop on one looping voice per flame at the emitter's walked volume and pitch and the flame's node, and SHALL free that voice and its buffer reference when the flame is despawned.

#### Scenario: Many frames
- **WHEN** a flame's loop is re-levelled for ten frames
- **THEN** one voice plays it

### Requirement: Arc texture scroll
The system SHALL advance an arc's texture offset by 0.015 a tick in double precision and, once it passes 2, take one off it.

#### Scenario: Past two
- **WHEN** an arc has ticked 134 times from zero
- **THEN** its texture offset is about 1.01
