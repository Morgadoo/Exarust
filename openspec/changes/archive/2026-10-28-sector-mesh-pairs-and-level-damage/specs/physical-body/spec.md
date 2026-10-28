# Spec Delta

## MODIFIED Requirements

### Requirement: Pair friction
In each of the 8 friction passes, before the level friction, each point of the pairs found this tick SHALL have its slip across its normal, relative to the other side this tick, cancelled under L = (0.01 + 0.99·n_y²)·μ_A·μ_B and kept less 0.01·L of itself over it, each side moving toward the other's slip by its mobility share over its weights. Only pairs whose μ_A·μ_B is under 1.19·10⁻⁷ SHALL NOT rub; a product at the epsilon or NaN rubs.

#### Scenario: Stick and slide
- **WHEN** a limb slipped 0.2 cm across a pair's point with another limb side by side
- **THEN** the slip is cancelled; a slip of 0.3 cm loses only 0.000625 of itself

#### Scenario: Almost no friction
- **WHEN** a limb lying on another with a friction product of 1.1·10⁻⁷ slipped 3·10⁻⁴ cm across their point
- **THEN** the slip is kept; with a product of 1.2·10⁻⁷, or exactly the epsilon, it is cancelled

## ADDED Requirements

### Requirement: Mesh pair contacts
The more mobile part's spheres SHALL be tested against the other part's collision triangles in its frame: swept from the previous relative pose when they moved over 0.1 cm² (never in a pair with one placed body, and with no static-only gate), after touching the other's mesh box and mesh sphere, one triangle at a time with the sphere re-read after each accepted hit. A part with no triangles SHALL be met through its spheres, pre-tested against their own bound.

#### Scenario: A fast sphere and a wall
- **WHEN** a sphere crosses a wall part's triangle at 60 m/s in one tick, with or without the static-only mode
- **THEN** the swept query catches it in front of the wall with one touch

#### Scenario: Behind the face
- **WHEN** a sphere's centre lies behind a mesh triangle
- **THEN** it is not pushed
