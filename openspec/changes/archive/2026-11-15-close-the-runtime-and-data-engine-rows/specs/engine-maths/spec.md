## ADDED Requirements

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
