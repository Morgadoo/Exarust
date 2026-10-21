## MODIFIED Requirements

### Requirement: Skinned body
The system SHALL skin the body's parts and worn garments with weights built from their seam triangles as the original builds them, so that the shipped bodies' parts stay joined in any pose. Edge falloffs SHALL follow the boundary marker class. Garments SHALL run the body's seam groups before groups of their marked triangles, and a position SHALL belong to the first group that reaches it.

#### Scenario: Walking
- **WHEN** the body plays a walk cycle
- **THEN** no gaps open at shoulders, elbows, hips or knees

#### Scenario: Shipped seams
- **WHEN** the installed human or ogre body is skinned
- **THEN** every two vertices of different parts at one rest position carry the same bones and weights

#### Scenario: A position in two groups
- **WHEN** a garment's marked triangle shares a corner with a wide seam
- **THEN** the corner takes the seam group's bone and share, not the marked group's
