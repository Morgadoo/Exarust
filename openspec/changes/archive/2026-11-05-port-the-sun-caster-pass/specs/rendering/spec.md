# Spec Delta

## ADDED Requirements

### Requirement: Sun shadow filter
A surface SHALL take the sun's shadow from the single fitted map at its unbiased map position, through the fragment's disc taps (the point lights' taps) offset by the light slot's radius `softness · 0.5 · 0.005` in the map's texture space, each a hardware 2 × 2 depth comparison, averaged.

#### Scenario: A softer sky widens the edge
- **WHEN** a 800 cm fitted map reads softness sources 0, 1 and 3
- **THEN** the radius is 0.00125, 0.0025 and 0.005 of the map: 2, 4 and 8 cm

#### Scenario: The constructor's fit
- **WHEN** the 3 000 cm fit reads source 0
- **THEN** the softness floors at 0.15 and the radius is 0.000375 of the map (2.25 cm)

### Requirement: Sun shadow map edges
Outside the fitted map the border texels SHALL stand, and the map SHALL have no far bound. With Bevy's own cascades the system SHALL keep Bevy's filter.

#### Scenario: A receiver past the fitted range
- **WHEN** a surface lies 130 m from the camera inside the fitted map
- **THEN** it still takes the map's shadow

### Requirement: Sun caster pass state
Every caster drawn into the sun map SHALL write its depth moved away from the sun by the polygon factor times the larger of its depth's two per-texel derivatives plus one 16-bit depth step (2⁻¹⁶). A plain caster SHALL cull its back faces and SHALL NOT discard; an alpha-tested caster SHALL cull no faces and SHALL drop the texels whose cut texture (the opacity map, else the diffuse map) has an alpha below ½, without the material's opacity.

#### Scenario: The constructor's offset
- **WHEN** the 3 000 cm fit reads source 0 (so c' = 1.125)
- **THEN** the factor is 1 + 1000/3000 + 2000·1.125/3000 ≈ 2.083 and the constant is 2⁻¹⁶

#### Scenario: A blended caster that is not alpha-tested
- **WHEN** a blended surface with opacity above 0.8 and no opacity flag casts
- **THEN** it casts whole, back faces culled
