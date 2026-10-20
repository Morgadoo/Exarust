# Spec Delta

## ADDED Requirements

### Requirement: Point shadow depth range
The system SHALL render point-light shadow maps with the original's 3 cm near plane, including the maps Bevy still renders for skinned characters.

#### Scenario: Torch held close
- **WHEN** a character carries a torch whose light sits just above its hand
- **THEN** the character's shadow follows its head, torso and legs instead of smearing into a floor-wide blob

### Requirement: Light-id encoding
The system SHALL carry a light's id in its clustered depth-bias field above Bevy's default 0.08 bias, in 1e-5 steps, and SHALL decode it as round((bias − 0.08) / 1e-5).

#### Scenario: Skinned shadow bias
- **WHEN** a light with a high record id lights a skinned character
- **THEN** the field remains a valid shadow bias near 0.08 rather than the raw id
