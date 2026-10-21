## ADDED Requirements

### Requirement: Engine fast sine and cosine together
The system SHALL compute the fast sine and cosine of one angle from a single quadrant fold (the magnitude times 4/pi rounded to the nearest even integer, up to an even quadrant), the sine taking the sine polynomial in even half-quadrants and the cosine polynomial in odd ones and the cosine the other way about, the sine's sign being the input's sign xor the quadrant's bit 2 and the cosine's the complement of bit 2 of the quadrant less two.

#### Scenario: Quadrant ends
- **WHEN** the angle is 0, pi and pi/2
- **THEN** the pair is (0, 1), (negative zero, -1) and (1, negative zero)

### Requirement: Keyframe curve evaluation
The system SHALL evaluate a keyframe curve with ordered compares, so a NaN time takes the plain branch, and SHALL wrap a looping curve's time by its duration before the segment search.

#### Scenario: Looping curve past its end
- **WHEN** a looping curve of duration 2 s is sampled at 2.5 s
- **THEN** it answers as at 0.5 s

### Requirement: Quaternion slerp
The system SHALL blend two rotations with the engine's slerp: the four-component dot in the engine's summation order, the second rotation negated when the dot is negative, a straight blend for nearly parallel rotations, and the fast sine for the weights.

#### Scenario: Opposite signs
- **WHEN** the second rotation is the negation of the first
- **THEN** the blend is the first rotation
