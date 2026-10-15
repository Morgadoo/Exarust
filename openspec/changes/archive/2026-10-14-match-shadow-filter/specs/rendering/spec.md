## ADDED Requirements

### Requirement: Light self-exclusion
The system SHALL leave a light's own object out of that light's shadows, as the original does: the light's siblings, its parent and its grandparent cast none. A light at its object's root excludes nothing.

#### Scenario: Wall torch
- **WHEN** a torch burns in its wall holder
- **THEN** the wall shows the holder's thin bracket shadow and a narrow cone below the torch head, not a wide shadow of the whole handle

### Requirement: Point shadow filter
The system SHALL filter point-light shadows as the original does: eight taps of a fixed disc around the light direction, turned per pixel, over a radius of (softness + 1)·0.005, with a normal offset of 0.008·(1 − |N·L|)·d and distances compared 0.5 % short.

#### Scenario: Soft torch shadows
- **WHEN** a torch shadows a nearby surface
- **THEN** the shadow's edge is softened by the torch's flickering softness
