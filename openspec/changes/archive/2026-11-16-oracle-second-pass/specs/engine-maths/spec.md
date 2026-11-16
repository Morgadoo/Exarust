## ADDED Requirements

### Requirement: The engine's lerp
Ports of the original's vector lerps SHALL compute `a + (b − a)·t` per component, not `a·(1 − t) + b·t`.

#### Scenario: The oracle's first differing case
- **WHEN** a = (−81.22808, 0.6715302, 52.456017), b = (−0.10922561, −54.247555, 80.28549) and t = −0.9491083
- **THEN** the lerp's y and z have the bits 0x42532ECA and 0x41D057B7

### Requirement: The exact normaliser's sum
Ports of the original's exact normaliser SHALL sum the squares `y² + (x² + z²)` and scale by one exact division `1 / √s`.

#### Scenario: A vector glam rounds otherwise
- **WHEN** (0.76246774, 93.80813, 5.5258827) is normalised
- **THEN** y and z have the bits 0x3F7F8C6F and 0x3D70DABC

### Requirement: The unmasked approximate normaliser
The door leaf's set-up SHALL normalise with the approximate reciprocal square root and no zero test, so a vector that reads as zero gives NaN.

#### Scenario: The zero vector
- **WHEN** the zero vector is normalised by the unmasked normaliser
- **THEN** its components are NaN

### Requirement: The library sine keeps a denormal
The runtime library's sine and cosine SHALL widen and narrow through the x87 unit, which ignores denormals-are-zero and flush-to-zero.

#### Scenario: A denormal angle
- **WHEN** the library sine of a denormal angle is taken
- **THEN** the result is that denormal, not zero
