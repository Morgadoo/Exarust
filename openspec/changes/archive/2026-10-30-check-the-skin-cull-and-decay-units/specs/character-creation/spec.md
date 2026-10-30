# Spec Delta

## MODIFIED Requirements

### Requirement: Decomposition maps and layers
The system SHALL name the decomposition passes' maps by their shipped unit order (mask: humanseams, humandcmpw; diffuse: humandcmpw, ancientdifm, skindecomp, humandcmpf; specular: humandcmpw, ancientdifm, humandcmpg; normal: ancientnrmm0, humandcmpn) and SHALL blend up to five normal layers, the fifth reading the mask rebound at `Tex5`.

#### Scenario: Fifth layer
- **WHEN** five normal layers carry weights and the mask texel is (0.25, 0, 0) with flat other maps
- **THEN** the fifth layer at full weight moves the stored normal to (0.375, 0.25), and four layers leave it flat

#### Scenario: More than four weights
- **WHEN** five normal layers carry weights
- **THEN** all five are blended, the fifth reading the mask in place of its own map
