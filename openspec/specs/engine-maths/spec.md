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
