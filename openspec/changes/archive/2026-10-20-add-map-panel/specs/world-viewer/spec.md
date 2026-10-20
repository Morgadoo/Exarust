# Spec Delta

## ADDED Requirements

### Requirement: Map panel
The system SHALL open a map panel with F9 in the level and play views, showing the current map and switches for point shadows, every room, the navigation grid and the body overlay that apply at once.

#### Scenario: Toggle shadows
- **WHEN** the shadows switch is pressed
- **THEN** point lights stop or start casting shadows without a reload

### Requirement: Map switching
The map panel SHALL list the installation's levels. Choosing one SHALL restart Exarust on it in the same view, keeping the other command-line flags except those tied to a place or a script.

#### Scenario: Switch to an outdoor arena
- **WHEN** the player opens the panel in `play exanima01` and chooses `arenao3`
- **THEN** Exarust restarts in `play arenao3` with the same flags, minus any `--at` or `--camera`

### Requirement: Panel input
While the map panel is open, mouse buttons and the wheel SHALL act on the panel only, and the system pointer SHALL be shown instead of the world cursor.

#### Scenario: Clicks stay in the panel
- **WHEN** the panel is open and the left button is pressed over it in combat mode
- **THEN** the player does not attack
