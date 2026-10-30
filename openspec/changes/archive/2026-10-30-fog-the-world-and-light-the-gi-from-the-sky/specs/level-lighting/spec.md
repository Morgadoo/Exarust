# Spec Delta

## ADDED Requirements

### Requirement: World fog
The system SHALL fog every lit world surface (hair included) as the original's world programs do under fog: with b = min(d / s, 1) for the eye distance d and the stored fog span s (distance² · far plane + 100 cm), the surface colour SHALL be mixed by b towards the frame's scratch colour times (ambient · 0.5 + 0.5), after its emissive term; an unlit surface with only an opacity map SHALL have its alpha scaled by 1 − b. A scene without fog SHALL draw no fog.

#### Scenario: Default span
- **WHEN** a fogged scene keeps the constructor's fog distance of 1 and far plane of 50 000 cm
- **THEN** the fog reaches full strength 501 m from the eye, and a distance-2 fog reaches it at 2 001 m

#### Scenario: Cloudy sky
- **WHEN** a cloudiness of 2.5 narrows the weather's fog to about 75 m
- **THEN** the world's fog still runs over the stored 501 m from the eye

#### Scenario: Indoor level
- **WHEN** an indoor level is drawn
- **THEN** it has no fog and looks as before

## MODIFIED Requirements

### Requirement: Linear outdoor fog
The system SHALL draw the outdoor preset's fog with a linear falloff from the eye to the stored fog span, in the frame's scratch colour (the horizon under a sky); the weather's per-frame near and far distances SHALL stay in the model and not move it.

#### Scenario: Outdoor arena
- **WHEN** an outdoor arena is lit
- **THEN** its fog starts at the camera, reaches full strength at the stored span and takes the horizon colour

### Requirement: Lightning brightens the fog only
A lightning strike SHALL NOT turn on the scene's flash-clear switch. While a strike runs, the frame SHALL keep the clear colour, the world's fog, the sun and the ambient it had without the strike, and the record's fog colour SHALL be the record's colour plus the lightning blue (0.2, 0.3, 1.0) times I/(I + 2)·3 for the strike's intensity I, whether or not the level fogs; that colour SHALL light the GI's sky light, and when the strike ends the record's colour SHALL return.

#### Scenario: Strike in an outdoor arena
- **WHEN** a strike 316 cm from the view runs in an outdoor arena
- **THEN** its attenuation is about 0.8775, every frame clears with the level's background and keeps the sun, the ambient and the world's fog, and the GI's sky light follows the intensity

#### Scenario: Strike end
- **WHEN** the strike's end time passes
- **THEN** its intensity is zero and the sky light is the record's fog colour again

#### Scenario: Fogged scene without a sky
- **WHEN** the weather starts a strike in a fogged scene without a sky
- **THEN** the flash-clear switch stays off and the frame clears with the colour it cleared with before the strike
