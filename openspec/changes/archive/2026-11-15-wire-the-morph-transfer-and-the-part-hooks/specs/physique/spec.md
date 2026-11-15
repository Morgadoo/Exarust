## MODIFIED Requirements

### Requirement: Transferred morphs
The system SHALL give a body part or garment piece its share of the morphs from the race's morph body: per position element, the barycentric blend of each target's offsets at the nearest body surface within 10 cm, less the offset of the landmark nearest the piece's origin, in the piece's frame; elements with no surface within reach SHALL take the inverse-distance mean of their set neighbours, pass after pass. A garment piece SHALL be placed on its bone's base body part.

#### Scenario: The human body's parts
- **WHEN** the 18 parts of `humanbase.rfc` take their shares from `humanmorph.rfc`
- **THEN** every element finds the body within 10 cm, each part's nearest landmark is its own bone's, and the height target moves the pelvis 11.20 cm down and the right thigh 11.92 cm down and 1.50 cm inward

#### Scenario: Away from the body
- **WHEN** an element lies 30 cm from the body beside one that touches it, and a separate triangle lies 50 cm away
- **THEN** the first takes its neighbour's offset and the triangle's elements stay where they are while the piece's origin moves

## ADDED Requirements

### Requirement: Transfer weights on the triangle's plane
The barycentric weights of an element's nearest body point SHALL be solved as the original solves them: the point projected along the triangle's face normal (the edges' cross product scaled by the processor's reciprocal square root estimate) onto the plane through its first corner, two weights from the 2x2 solve the larger summed x or y extent picks (NaN picks the y-plus-z rows), and the third weight one less the other two, unclamped.

#### Scenario: A point off the triangle
- **WHEN** the weights of a point beyond a triangle's edge are solved
- **THEN** one weight is negative and the three sum to one

### Requirement: Diffusion weight at the same place
In the transfer's diffusion, a set neighbour at the same place as the element SHALL weigh 1/0, as in the original with its floating-point exceptions masked, so the element's mean comes out NaN; every garment piece of the installed `Characters.rpk` SHALL still take finite morphs from both races' morph bodies.

#### Scenario: Shipped garments
- **WHEN** every garment piece is transferred from `humanmorph.rfc` and `ancientmorph.rfc` on its bone's base body part
- **THEN** no position element of any piece has a non-finite offset

### Requirement: Morph set on the first root
The race's morph body SHALL be the set on the morph file's first root, as the original's race loader takes it; each shipped morph file SHALL carry its one set there, so the loader's fallbacks (a child carrying the set, the numbered target meshes) never run on shipped data.

#### Scenario: Shipped morph files
- **WHEN** `humanmorph.rfc` and `ancientmorph.rfc` are read
- **THEN** the first node of each is a root carrying the file's only morph set
