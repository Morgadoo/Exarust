## ADDED Requirements

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
The system SHALL build a character's body and garments at its weights before skinning: positions plus each target's offsets at its weight (weights below about 1.2e-7 skipped), normals from the new shape, boxes refitted, and each body part's origin moved with its landmark so its translation follows under its moved parent. Weights that change nothing, and races without a morph body, SHALL keep the model as stored.

#### Scenario: A short broad body
- **WHEN** the human body is built at physique (1, 1) and height 1
- **THEN** the pelvis is 11.2 cm lower, the knee 5.7 cm closer to the hip, the parts' boxes and vertices changed, and building it again returns the same variant

#### Scenario: The model's body
- **WHEN** the human body is asked for at the neutral physique, or a skeleton at any physique
- **THEN** the shared model is returned unchanged

#### Scenario: Garments follow
- **WHEN** a character of physique (1, 1) and height 1 wears `trouser_a`
- **THEN** the trousers' vertices move with the body while their skin weights stay those of the unmorphed fit

### Requirement: Characters in their physique
Characters a level places SHALL take their records' physique, body and apparel. The bodies Exarust makes without a record (the player, the duellist, the character view) SHALL keep the model's body unless `--physique` gives the character view's or the player's body one.

#### Scenario: Debug physiques
- **WHEN** the character view is shot in a light run without `--physique`, with `--physique 0,1,-1` and with `--physique 1,1,1` (also dressed in a jacket, trousers and boots)
- **THEN** the first matches the unmorphed body within run-to-run noise, the second stands taller and slighter, the third shorter and broader, and its garments follow its shape
