# Spec Delta

## ADDED Requirements

### Requirement: Lightning brightens the fog only
A lightning strike SHALL NOT turn on the scene's flash-clear switch. While a strike runs, the frame SHALL keep the clear colour, the fog's near and far distances, the sun and the ambient it had without the strike, and the record's fog colour SHALL be the record's colour plus the lightning blue (0.2, 0.3, 1.0) times I/(I + 2)·3 for the strike's intensity I; when the strike ends the record's colour SHALL return.

#### Scenario: Strike in an outdoor arena
- **WHEN** a strike 316 cm from the view runs in an outdoor arena
- **THEN** its attenuation is about 0.8775, every frame clears with the level's background and keeps the sun, the ambient and the fog's reach, and the fog colour follows the intensity

#### Scenario: Strike end
- **WHEN** the strike's end time passes
- **THEN** its intensity is zero and the fog colour is the record's again

#### Scenario: Fogged scene without a sky
- **WHEN** the weather starts a strike in a fogged scene without a sky
- **THEN** the flash-clear switch stays off and the frame clears with the colour it cleared with before the strike
