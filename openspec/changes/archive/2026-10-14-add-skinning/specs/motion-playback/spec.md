# Spec Delta

## ADDED Requirements

### Requirement: Skinned body
The system SHALL skin the human body with weights derived from its parts' seam triangles, so that adjacent parts stay joined in any pose, and SHALL skin worn apparel with the weights of the nearest body vertices.

#### Scenario: Walking
- **WHEN** the body plays a walk cycle
- **THEN** no gaps open at shoulders, elbows, hips or knees
