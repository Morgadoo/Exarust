# physique Specification

## Purpose
Gives every character the body shape the original gives it: the race's
morph set, the weights a character record gives its targets, the morphs
carried over to the body parts and the apparel fitted to them, and the
morphed bodies, joints and garments built before skinning.

## Requirements

### Requirement: Morph sets
The system SHALL decode the morph set a mesh node carries after its body: the landmarks, the base position of each element of the mesh's position stream and, per target, the landmarks' and the positions' offsets, in metres. A set that does not decode SHALL keep its error and nothing SHALL panic. It SHALL hold for every `*morph.rfc` of the installed `Resource.rpk`.

#### Scenario: Shipped sets
- **WHEN** `humanmorph.rfc` and `ancientmorph.rfc` are decoded
- **THEN** each carries one set on its body mesh, with 18 landmarks, 2286 positions (the mesh's position-stream count) and 11 targets, landmark 1 at (−9.07, −8.46, 3.50) cm, and the height target drops the pelvis landmark 11.20 cm, the knees 6.21 cm and the feet not at all

#### Scenario: Truncated set
- **WHEN** a set's targets run past the end of its chunk, or a count is negative or more than the chunk can hold (any count of targets that hold nothing)
- **THEN** the node keeps an error for its set and the rest of the file decodes

### Requirement: Physique weights
The system SHALL weigh a race's targets from a character record as the original's builder does: the two physique values kept within 0…1 and adjusted by the height, the picker's four corners, the feral level, the female bit (targets 5 and 6), the age (target 4, the shares of targets 0 and 4) and the height (target 10), with the height kept within −1…1 and the age and feral level within 0…1.

#### Scenario: Neutral and new records
- **WHEN** the physique (½, 1) with height, age and feral level 0, and a new record's physique (0, 0), are weighed
- **THEN** the first weighs every target 0 and the second target 3 alone, fully

#### Scenario: A broad old feral woman
- **WHEN** physique (0.8, 0.3), height ½, age ½, feral level ½, female, is weighed
- **THEN** the weights are 0.2120 (target 1), 0.6100 (2), 0.5261 (4), 1 (5), 0.1875 (6) and 0.5 (10), the others 0

#### Scenario: A slight tall man
- **WHEN** physique (0.2, 0.6), height −0.4, age and feral level 0, male, is weighed
- **THEN** the weights are 0.3 (target 0), 0.2 (2), 0.2 (3) and −0.4 (10), the others 0

### Requirement: Transferred morphs
The system SHALL give a body part or garment piece its share of the morphs from the race's morph body: per position element, the barycentric blend of each target's offsets at the nearest body surface within 10 cm, less the offset of the landmark nearest the piece's origin, in the piece's frame; elements with no surface within reach SHALL take the inverse-distance mean of their set neighbours, pass after pass. A garment piece SHALL be placed on its bone's base body part.

#### Scenario: The human body's parts
- **WHEN** the 18 parts of `humanbase.rfc` take their shares from `humanmorph.rfc`
- **THEN** every element finds the body within 10 cm, each part's nearest landmark is its own bone's, and the height target moves the pelvis 11.20 cm down and the right thigh 11.92 cm down and 1.50 cm inward

#### Scenario: Away from the body
- **WHEN** an element lies 30 cm from the body beside one that touches it, and a separate triangle lies 50 cm away
- **THEN** the first takes its neighbour's offset and the triangle's elements stay where they are while the piece's origin moves

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

### Requirement: Characters in their physique
Characters a level places SHALL take their records' physique, body and apparel. The bodies Exarust makes without a record (the player, the duellist, the character view) SHALL keep the model's body unless `--physique` gives the character view's or the player's body one.

#### Scenario: Debug physiques
- **WHEN** the character view is shot in a light run without `--physique`, with `--physique 0,1,-1` and with `--physique 1,1,1` (also dressed in a jacket, trousers and boots)
- **THEN** the first matches the unmorphed body within run-to-run noise, the second stands taller and slighter, the third shorter and broader, and its garments follow its shape

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

### Requirement: Morph apply order
A morphed mesh SHALL start from its base positions and its base origin and add, target by target in order, each offset times the target's weight in place, skipping a weight below about 1.2e-7 in magnitude and applying a NaN weight.

#### Scenario: Two tiny offsets
- **WHEN** two targets at weight 1 each move a position at 1.0 by 2^-24
- **THEN** the position stays 1.0, as the original's in-place sums give

### Requirement: Transfer neighbour order
The transfer SHALL walk an off-body element's neighbours in the mesh's incidence as its load leaves it (the element's last face first, each face's corners last first, split into smoothing-key runs), each the element at its face's next corner.

#### Scenario: Two faces
- **WHEN** an element off the body has a neighbour on bone 1 in its first face and one on bone 2 in its second
- **THEN** its filled skin record holds bone 2 in its first slot

### Requirement: Morphed box refit
A morphed mesh's box and sphere SHALL be refitted as after any mesh edit: the box of its positions with half extents at least 1e-4 and then 0.001 cm, and the farthest position's squared distance rooted above about 1.2e-7 cm².

#### Scenario: A flat part
- **WHEN** a morphed part lies in one plane
- **THEN** its box keeps 0.001 cm across the plane
