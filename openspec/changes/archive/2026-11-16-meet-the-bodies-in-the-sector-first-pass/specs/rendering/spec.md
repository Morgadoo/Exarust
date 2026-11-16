## ADDED Requirements

### Requirement: Particle body pass
An emitter's body pass SHALL zero its contact count, walk its live ring in one range or, wrapped, two, and give the part `(float)count × dose`. Each live particle SHALL take a radius of `(float)(start_life − life) × (collision_growth / start_life)` in the part's frame and go on unless `(r + R)² < d²` against the node's bound sphere or `r²` is under its squared overhang past the node's box.

#### Scenario: An aged particle off a face
- **WHEN** a particle 2 cm off a part's face has a 5 cm radius
- **THEN** it is pushed off the face to its radius, loses a tick of life, takes −0.98 of the push into its velocity and the part takes one dose

### Requirement: Particle push-out
The node mesh's two-sided sphere query SHALL push a particle's sphere off each face's closest point: with `d` the centre less the point, a `d²` at or past `r²` or under the engine's small float SHALL leave it, a NaN going on, and any other SHALL move it by `d·(r/√d² − 1)`.

#### Scenario: A NaN distance
- **WHEN** the sphere's centre is NaN
- **THEN** the push is applied and the centre stays NaN
