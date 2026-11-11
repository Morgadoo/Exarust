## ADDED Requirements

### Requirement: Constant displacement shell
A Constant Displacement's start SHALL make a displacement shell on the caster's body before its tendril image (the shell's two creation draws first), with leave-to-finish cleared: it SHALL open over 400 ms, rest open while the power runs and close over 300 ms when the power ends, drawn over copies of the caster's skinned meshes with `Interior` 0.

#### Scenario: The constant ends
- **WHEN** the Constant Displacement ends with its shell open
- **THEN** the shell closes over 300 ms and draws nothing more

### Requirement: Vanish over a constant's shell
A Vanish or Long Door cast while a Constant Displacement runs on the caster SHALL take the constant's shell instead of making one (no creation draws, its phased body lent, the power's shared flag set), run it through its phases and leave it open for the constant at its end.

#### Scenario: A vanish over a constant displacement
- **WHEN** a Vanish is cast and released while a Constant Displacement runs on the caster
- **THEN** the constant's resting shell closes as the body vanishes, reopens with the come-back and is left resting open when the vanish ends

### Requirement: Vanish body draw
While a vanish's show call holds the body out of the scene, the depth half SHALL draw it depth only, the fade half SHALL draw its parts blended at the shown amount over that depth, and otherwise it SHALL be hidden. While the body's fade bit is set (from a staging entry to the hide) it SHALL cast dithered at one less the shown amount of the last fade-half frame.

#### Scenario: The body's shadow
- **WHEN** a Vanish's body has blended out over 350–450 ms
- **THEN** its shadow is gone until the come-back's fade half brings it back

## MODIFIED Requirements

### Requirement: Vanish displacement shell
The system SHALL draw a Vanish or Long Door cast's displacement shell over a copy of the caster's skinned meshes, opening over 400 ms from the cast, closing over 300 ms when the body vanishes, reopening from the come-back and closing with leave to finish when the come-back ends, with `Interior` at one less the vanish's shown amount; the body itself is drawn as the vanish body draw says.

#### Scenario: A released Vanish
- **WHEN** a player's Vanish is cast at 0 ms and released at 2000 ms
- **THEN** the body blends out over 350–450 ms, is hidden from then until the come-back's fade half, the shell is drawn with `Interior` 1 while the body is hidden, and the shell leaves after the come-back's close
