## ADDED Requirements

### Requirement: Stored-contact friction in double
The stored-contact pass SHALL compute its friction limit `(n_y²·0.99 + 0.01)·(0.5·friction)` and its slide factor `limit·−0.01` in double precision, cancelling the slip only when the limit is strictly over its square, and SHALL read the contact point as the engine's paired weighted point.

#### Scenario: A slow slip on a floor
- **WHEN** a limb's stored contact on a floor slipped 1.1 cm this tick
- **THEN** the slip is cancelled, and a 1.2 cm slip is scaled by −0.0125

### Requirement: The thigh's extra turn compares
The thigh's extra turn SHALL take no angle when the product of the squared lengths is not over the engine epsilon (or is NaN), a right angle when the cosine is under it, none when the cosine is over 1 or NaN, and SHALL build its quaternion with the engine's axis-angle routine about the unnormalised cross product.

#### Scenario: A NaN direction
- **WHEN** the rest direction is NaN
- **THEN** the extra turn is the identity quaternion

### Requirement: The extra turn's decay
Not stepping, the extra turn SHALL move a tenth of the way to the identity by the engine's four-lane lerp, then be normalised by the engine's quaternion normaliser.

#### Scenario: A 30 degree turn decays
- **WHEN** a held 30° extra turn decays once
- **THEN** it equals the lerp toward the identity by 0.1, normalised, about 27°

### Requirement: Grab re-anchor by exact weights
A weapon grab marked for re-anchoring SHALL store the original's exact barycentric weights of the weapon's grab point over the struck part's particles, without a flatness guard.

#### Scenario: A re-anchored grab
- **WHEN** a grab with the re-anchor flag is pulled
- **THEN** its struck weights are the exact barycentric weights of the weapon point

### Requirement: Drawn matrix through the rest basis
A body part's drawn matrix SHALL be the rest basis (the inverse of the four rest points widened with w = 1) times the interpolated particles, each row summed `((b0·p0 + b1·p1) + b2·p2) + b3·p3`, not orthonormalised.

#### Scenario: Drawing between ticks
- **WHEN** a part is drawn halfway between its last two ticks
- **THEN** its matrix maps the rest points onto the halfway particles
