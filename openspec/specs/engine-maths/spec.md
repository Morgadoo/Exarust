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

### Requirement: Engine vector helpers
The 3D length SHALL be `sqrt((x² + z²) + y²)` and the projection onto a unit SHALL scale the unit by `(x·ux + z·uz) + y·uy`. Component minimum and maximum SHALL give the second operand when either is NaN. The heading SHALL be zero below a squared XZ length of 1e-8, π at a cosine of -1 or less, zero above 1, the fast arc cosine otherwise, negated for a negative x.

#### Scenario: Projection order
- **WHEN** (2^-24, 1, 2^-24) is projected onto (1, 1, 1)
- **THEN** each component is 1 + 2^-23

#### Scenario: NaN in a component minimum
- **WHEN** (NaN, 2, NaN) and (1, 5, 3) are combined by the component minimum
- **THEN** the result is (1, 2, 3)

### Requirement: Engine fast vector reciprocal
The engine's fast vector reciprocal SHALL be the processor's approximate reciprocal of each component, as one packed operation on (x, y, z, 0): within 1.5·2⁻¹² of the exact value, a zero giving infinity of its sign and a NaN staying NaN.

#### Scenario: Each lane on its own
- **WHEN** the fast reciprocal of (3, −0.125, 10000) is taken
- **THEN** each component is the processor's scalar approximate reciprocal of that component, within 1.5·2⁻¹² of the exact one

### Requirement: One engine maths implementation
Every crate that may depend on the physics crate SHALL take the fast logarithm, the fast exponential and the approximate reciprocal square root from its engine maths module rather than a copy of its own.

#### Scenario: The damage model's power
- **WHEN** the damage model raises 4 to 0.5
- **THEN** it uses the shared logarithm and exponential and gets about 2

### Requirement: Fast arctangent in the light flood
The light flood SHALL take the engine's fast arctangent for a mesh's centre angle and for each opening end, and the library's arctangent in double precision for the mesh's half angle.

#### Scenario: An opening end
- **WHEN** the flood measures the angle to a point one cell right and two cells ahead
- **THEN** it equals the engine's fast arctangent of (1, 2) bit for bit

### Requirement: Angle wrap in the search and the parry
The search sweep and the parry pick SHALL wrap a heading gap once, comparing it in double precision with the double π and stepping by the f32 full turn; the parry's share SHALL be computed in double precision as θ × 1/π + 0.5.

#### Scenario: The single-precision π
- **WHEN** a gap equals π in single precision
- **THEN** it is wrapped to minus that value

### Requirement: Engine floor and ceiling at the call sites
The room index, the search flood's anchor, the level room lookup and the GI block pick SHALL round down with the engine's floor, and the GI half extents up with its ceiling, giving the integer's smallest value for NaN or a value out of range.

#### Scenario: A NaN position
- **WHEN** the GI block of a NaN position is picked
- **THEN** its cell coordinates are the integer's smallest value

### Requirement: Copies of the fast sine and cosine
Every copy of the engine's fast sine and cosine SHALL form both results by the engine's mask arithmetic and agree with the engine's routine bit for bit, a NaN cosine past the reduction's range included.

#### Scenario: A huge angle
- **WHEN** the factory's copy takes 3e9
- **THEN** its cosine is NaN, as the engine's

### Requirement: Paired squared distance
The contact eviction SHALL measure the squared distance of two 4-float points as (dx² + dz²) + (dy² + dw²).

#### Scenario: Small squares in pairs
- **WHEN** the squares are 1, 2^-24, 0 and 2^-24
- **THEN** the distance is the next float above 1

### Requirement: Denormals read as zero by the conversions
The floor and ceiling conversions SHALL read a denormal input as a zero of its sign, as the original's conversion does under denormals-are-zero.

#### Scenario: The smallest denormals
- **WHEN** the smallest positive denormal is ceiled and its negative floored
- **THEN** both give 0

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
