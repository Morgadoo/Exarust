# engine-maths Specification

## Purpose
The engine's own fast maths: compares and rounding conversions, polynomial logarithm, exponential and trigonometry, and the small vector, quaternion, matrix and projection helpers, as the original's instructions compute them.

## Requirements

### Requirement: Engine compares and conversions
The engine's minimum and maximum SHALL return their second operand when either is NaN, the clamp SHALL take the maximum with the low bound and then the minimum with the high one, the sign SHALL be 0 for either zero and follow the sign bit otherwise, and the floor and ceiling conversions SHALL round toward negative and positive infinity, giving the most negative 32-bit word for NaN and out-of-range values.

#### Scenario: NaN and range
- **WHEN** the minimum of NaN and 2, the clamp of NaN between 1 and 3, and the floor of NaN and of 3e9 are taken
- **THEN** they are 2, 1, and the most negative word twice
- **WHEN** −2.1 is floored and −2.9 is ceiled
- **THEN** they are −3 and −2

### Requirement: Engine fast functions
The engine's logarithm, exponential, sine and sine-cosine SHALL be its single-precision polynomials: ln 1 SHALL be the float with bits 0x37800000 and exp 0 the float with bits 0x3F800016; the sine-cosine SHALL pick its two polynomials by mask arithmetic, so that once the angle's reduction saturates (from about 1.69e9) the sine is infinite and the cosine NaN.

#### Scenario: Saturated reduction
- **WHEN** the sine-cosine of 2e9 is taken
- **THEN** the sine is negative infinity and the cosine is NaN

#### Scenario: Small angles
- **WHEN** the sine-cosine of 0 is taken
- **THEN** it is (0, 1)

### Requirement: Engine look basis
The look basis along a unit forward SHALL take the vertical less its part along the forward as its up and the up crossed with the forward as its side, each scaled by the processor's approximate reciprocal square root of its squared length (summed x and z, then y), so the axes are unit to about 12 bits; a forward along the vertical SHALL give NaN axes.

#### Scenario: Level forward
- **WHEN** the basis of the forward (0, 0, 1) is built
- **THEN** its up is (0, 1, 0) and its side (1, 0, 0) within 1e-3

### Requirement: Engine matrix inverse
The 4×4 inverse SHALL eliminate column by column, comparing each lower row's entry in magnitude with the diagonal's (not with the best row so far) and taking the last larger one as the pivot.

#### Scenario: Pivot choice
- **WHEN** a column's diagonal is 1 and the rows below hold 3 and then 2
- **THEN** the row holding 2 is the pivot, as both are larger than the diagonal and it is the last

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
