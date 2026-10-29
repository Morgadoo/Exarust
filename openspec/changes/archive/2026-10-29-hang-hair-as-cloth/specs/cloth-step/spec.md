## ADDED Requirements

### Requirement: Cloth parameter record
A cloth SHALL be set up from a parameter record: the built-in record (mode `0x7208`, slide 0.1 cm, retention 0.5 a second, drag 0.05, padding 2 cm, gravity 1, four passes) unless its node hands it another. The per-tick retention SHALL be 0.98977 times `exp(ln(r) · 1/83)` computed in double and stored as a float.

#### Scenario: The built-in record
- **WHEN** a banner's cloth is set up
- **THEN** it keeps 0.98977 · 0.5^(1/83) of its velocity a tick, drags 0.05 toward the mean, and meets the level with 2 cm padding and a 0.1 cm slide

### Requirement: Constraint mode
A record whose mode word has `0x100` in its `0xF00` nibble SHALL make all its passes in the step and none after the body push; any other SHALL make one in the step and the rest after the push.

#### Scenario: The hair's record
- **WHEN** a cloth is set up from the hair's record (mode `0x104`, drag 0.2, padding 0, slide 0, two passes)
- **THEN** its step makes two constraint passes, its post-body pass makes none, and its level contact uses a zero radius and no slide

### Requirement: Particle-mode body contact
A particle-mode cloth SHALL take each body's spheres in order, padded 1 cm (none on its owner), skip a sphere apart from the cloth's box (tested with the squared padded radius as its radius), and push each particle inside the padded sphere straight out to its surface when the push's squared length is at least 1e-4 by bit pattern. A body that pushed SHALL hand the cloth its countdown when that is higher.

#### Scenario: A particle inside a sphere
- **WHEN** a still particle sits 5 cm from the centre of a 10 cm sphere of another body
- **THEN** it moves out to 11 cm from the centre

### Requirement: Particle-mode friction
A pushed particle's travel over the tick, less the body's motion at it (tetrahedron weights with the approximate reciprocal), SHALL lose its part along the push; the part across it SHALL be cancelled when its square is within a tenth of the push's square (and over the engine's small float), else shortened by `√(0.1·push²/travel²)` of itself.

#### Scenario: A particle sliding over the scalp
- **WHEN** a particle 5 cm from an owner sphere's centre of radius 10 cm travelled 3 cm across the push
- **THEN** it moves out to the surface and keeps 3·(1 − √(2.5/9)) cm of that travel

### Requirement: Owned cloth
A particle-mode cloth SHALL never sleep below its owner's countdown, the owner being the body its node's parent carries, and its particles SHALL keep their world motion when the owner moves the node while its pins follow the node.

#### Scenario: Hair on a living character
- **WHEN** a hair cloth hangs from the head of a character whose body is awake (countdown 256)
- **THEN** the cloth stays awake; when the body sleeps, the cloth may sleep
