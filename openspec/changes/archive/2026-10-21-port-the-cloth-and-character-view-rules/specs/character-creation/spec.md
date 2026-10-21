## ADDED Requirements

### Requirement: Centred slider weight
The system SHALL set a centred creation slider's weight to its position less one half, negated, so the middle gives a negative zero and the ends -0.5 and +0.5.

#### Scenario: Slider in the middle
- **WHEN** a centred slider is at 0.5
- **THEN** its weight is a negative zero

### Requirement: Decomposition maps and layers
The system SHALL name the decomposition passes' maps by their shipped unit order (diffuse: humandcmpw, ancientdifm, skindecomp, humandcmpf; specular: humandcmpw, ancientdifm, humandcmpg; normal: ancientnrmm0, humandcmpn) and SHALL cap the normal pass at four layers.

#### Scenario: More than four weights
- **WHEN** five normal layers carry weights
- **THEN** only the first four are blended
