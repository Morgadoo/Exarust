## ADDED Requirements

### Requirement: Normal-map orientation
The system SHALL apply normal maps in the original's tangent frame. The tangent runs along +u and the bitangent along −v of the stored UVs. The map's red and green channels are used as stored, with z rebuilt as √(1 − x² − y²).

#### Scenario: Wall relief
- **WHEN** a torch lights a stone wall from below
- **THEN** the mortar grooves read as recessed, lit on the faces turned toward the torch
