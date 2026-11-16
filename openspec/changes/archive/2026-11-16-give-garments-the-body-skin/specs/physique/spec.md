## ADDED Requirements

### Requirement: Body skin records
For a race with a morph body the system SHALL merge the base model's body parts (mesh nodes in the first root's tree not named with `#` or `_`) in the first root's frame with their skin weights, weld the merge, and give each base position of the morph set the skin record of the nearest merged position, the first of equally near ones.

#### Scenario: The shipped bodies
- **WHEN** the human and ancient bodies build their records
- **THEN** every base position lies within 1.6 cm of its record's position and every record has a joint

### Requirement: Skin through the transfer
A garment piece whose render word has bit `0x200000` SHALL take its weights through the transfer: each element SHALL merge the records of its surface triangle's corners at their barycentric weights, a weight below the smallest merging nothing, into four slots.

#### Scenario: Between two corners
- **WHEN** an element lies over the middle of an edge between a corner on bone 1 and one on bone 2
- **THEN** its record holds bones 1 and 2 at one half each

### Requirement: Skin fill and relaxation
Elements off the body SHALL be cleared and filled from their neighbours already set at 1/distance, pass after pass; every record SHALL be normalised with negative weights set to 0; the corners of triangles with a `0xC00000` flag bit SHALL then be relaxed three times to half their record plus half of each neighbour's at 1/distance, read before the pass, and normalised.

#### Scenario: Never reached
- **WHEN** a triangle of the piece lies a metre from the body with no element near it
- **THEN** its elements keep a record with no joint

### Requirement: Garments skinned at any physique
A garment worn on a race with a morph body SHALL take the body's skin even when the physique changes no shape, keeping its positions.

#### Scenario: A neutral body
- **WHEN** `trouser_a` is worn on a human of neutral physique
- **THEN** its positions are the unmorphed ones and its joints those of a short, broad human's

## MODIFIED Requirements

### Requirement: Morphed bodies
The system SHALL build a character's body and garments at its weights before skinning: positions plus each target's offsets at its weight (weights below about 1.2e-7 skipped), normals from the new shape, boxes refitted, and each body part's origin moved with its landmark so its translation follows under its moved parent. Weights that change nothing, and races without a morph body, SHALL keep a body model as stored.

#### Scenario: A short broad body
- **WHEN** the human body is built at physique (1, 1) and height 1
- **THEN** the pelvis is 11.2 cm lower, the knee 5.7 cm closer to the hip, the parts' boxes and vertices changed, and building it again returns the same variant

#### Scenario: The model's body
- **WHEN** the human body is asked for at the neutral physique, or a skeleton at any physique
- **THEN** the shared model is returned unchanged

#### Scenario: Garments follow
- **WHEN** a character of physique (1, 1) and height 1 wears `trouser_a`
- **THEN** the trousers' vertices move with the body while their skin weights stay those of the neutral fit
