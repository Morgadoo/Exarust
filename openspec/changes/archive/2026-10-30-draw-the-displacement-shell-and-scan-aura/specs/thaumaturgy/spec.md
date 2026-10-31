## ADDED Requirements

### Requirement: Vanish displacement shell
The system SHALL draw a Vanish or Long Door cast's displacement shell over a copy of the caster's skinned meshes, opening over 400 ms from the cast, closing over 300 ms when the body vanishes, reopening from the come-back and closing with leave to finish when the come-back ends, with `Interior` at one less the vanish's shown amount; and SHALL hide the caster's own meshes while the vanish draws the body and neither half of its segment pass shows it, drawing the depth half as a depth-only copy.

#### Scenario: A released Vanish
- **WHEN** a player's Vanish is cast at 0 ms and released at 2000 ms
- **THEN** the body is seen until about 400 ms, hidden from then until the fade-in passes one half, the shell is drawn with `Interior` 1 while the body is hidden, and the shell leaves after the come-back's close
