## ADDED Requirements

### Requirement: Character list tick order
The sector step SHALL tick the living characters in the order of the simulation's character list, from its last entry down to its first. A body that registers SHALL be appended; the player SHALL instead take the first entry, that entry's body moving to the end. A body that leaves (hidden out of the scene, no longer living, removed) SHALL have its place taken by the last entry, and a body that comes back SHALL join at the end.

#### Scenario: Three bodies and a player
- **WHEN** bodies 1 and 2, the player 3 and body 4 register in that order, then body 2 leaves and body 9 registers
- **THEN** the first step ticks 4, 1, 2, 3 and after the change the list is 3, 4, 1, 9, ticked 9, 1, 4, 3

### Requirement: Level hit gates and weights
A level hit SHALL take its push in cm: a squared length (summed (x² + y²) + z²) under 1e-5, or NaN, SHALL record nothing. The deepest point's weights SHALL come from the exact barycentric routine over the current particles; when the largest weight's magnitude, by the original's maximum chain, exceeds 1 (a NaN does not), the sphere centre's weights over the rest particles SHALL be used.

#### Scenario: A deepest point far outside
- **WHEN** a level hit's deepest point lies 3 m from a 6 cm part
- **THEN** the stored contact takes the sphere centre's weights

### Requirement: Level hit plane and push
A level hit's normal SHALL be the push over its length summed y² + (x² + z²), and its plane the point's distance along the normal in cm less 0.1 subtracted in double. The push SHALL move the particles over the weights normalised to their sum and then to their largest, and the part's strain SHALL take the push's length summed (x² + z²) + y². The impact travel SHALL be the smoothed velocity at the weights summed in pairs.

#### Scenario: The stored plane
- **WHEN** a ball lands on a floor through the live solver
- **THEN** every stored contact's plane equals (float)(double(n·q in cm) − 0.1) in metres

### Requirement: Pair hit weights and split
A pair hit SHALL take the sphere side's weights as the level hit does, the touched side's over its rest particles by the exact barycentric routine, and the normal by the exact normalisation. The immediate push SHALL split by f = φ_A · (2 / (φ_A + φ_B)), the reciprocal taken first, A moving by its share times (2 − f) and B back by its share times f.

#### Scenario: Spreads whose orders differ
- **WHEN** A's contact weights are (1, 39, 0, 0) and B's (3, 37, 3, 0)
- **THEN** f is φ_A · (2 / (φ_A + φ_B)), which differs from 2φ_A / (φ_A + φ_B) in the last bit, and each particle moves by that split

### Requirement: Level friction precision
The level friction pass SHALL work in cm: the slip across the normal as `s + n·(−s·n)`, its square summed (x² + y²) + z², the limit `(float)((double(n_y²)·0.99 + 0.01)·double(0.5·friction))`. A limit strictly over the square SHALL cancel the slip; any other, a NaN too, SHALL scale it by `(float)(double(limit)·−0.01)`.

#### Scenario: Slow, fast and NaN
- **WHEN** a 0.2 cm slip, a 3 cm slip and a NaN friction meet a contact with friction 0.8
- **THEN** the first is cancelled, the second scaled by a hundredth of the limit backwards, the third NaN

### Requirement: Contact slip
A part's slip at a point SHALL be the point weighted over the current particles less the point weighted over where the last tick left them, each sum taken term by term (((w₀p₀ + w₁p₁) + w₂p₂) + w₃p₃), then subtracted.

#### Scenario: A moved part
- **WHEN** two particles of a part move and the slip is taken at weights (0.31, 0.17, 0.29, 0.23)
- **THEN** it equals the two term-by-term weighted sums subtracted

### Requirement: All-pairs sector order
In a sector stepped all pairs (a scene without a grid) the bodies SHALL meet the level before the sim objects' contacts run; in a grid sector the sim objects' contacts SHALL run before the level detection.

#### Scenario: A resting ball in an all-pairs sector
- **WHEN** the solver steps a resting ball with a stage marking an all-pairs sector
- **THEN** the sim objects' contacts run after the level detection, with the ball's level contacts already found
