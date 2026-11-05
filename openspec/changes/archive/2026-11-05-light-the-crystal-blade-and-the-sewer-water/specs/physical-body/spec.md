# Spec Delta

## ADDED Requirements

### Requirement: Walking wet step
When a walking foot plants under the scene's water (the contact's height below the water level), a wet step SHALL follow the walking footstep at the same place:
- the depth share is min(1, 0.05·(−water − 1 − contact)), the sum in single precision and the scale in double;
- the pitch is (0.3·p + 0.9)(0.05·thrust + 0.9);
- the gain is volume · depth · (0.5·g + 0.2)(0.01·thrust² + 0.01), with the draw's half and the thrust's square in single precision;
- p and g are the engine's unit draws, taken in that order after the comparison;
- thrust is the gait's first column.

#### Scenario: A deep plant
- **WHEN** a foot plants at −30 cm under a −25 cm surface with thrust 2 and draws 0 and 0
- **THEN** the depth share is 1, the pitch 0.9 and the gain 0.01
