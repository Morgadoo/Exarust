# Spec Delta

## ADDED Requirements

### Requirement: Wet steps
Under the scene's water both foot sound paths SHALL play the `StpWet` set:
- the footwork's release step, at the foot's contact after its footstep;
- the walk's gait plant, at the footstep's place.

A level without water (water level −100000) SHALL play none.

#### Scenario: The sewer's channel
- **WHEN** the player's foot lets go at −52 cm in `exanima05_sw`
- **THEN** a `StpWet` step plays after the `StpSft` footstep
