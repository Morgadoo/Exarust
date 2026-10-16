# motion-playback Specification

## Purpose
Decodes the original skeleton and motion clips and poses characters with them,
providing the pose targets for physically driven bodies.

## Requirements

### Requirement: Skeleton and motion set decoding
The system SHALL decode the skeleton bones with their hierarchy and the motion set's actions, movesets and keyed poses from `humanoid.rsq`, consuming the file exactly.

#### Scenario: Retail file
- **WHEN** `humanoid.rsq` is decoded
- **THEN** it yields 18 bones, 48 actions and 32 movesets, and the default moveset defines all 48 actions

### Requirement: Moveset fallback
The system SHALL resolve an action missing from a moveset through the moveset's parent chain.

#### Scenario: Inherited walk
- **WHEN** the `1H & Shield` moveset's walk is requested
- **THEN** the walk of its ancestor `Default` is returned

### Requirement: Pose sampling
The system SHALL sample a clip at any time by interpolating rotations spherically and the root position linearly between keys, clamping outside the clip.

#### Scenario: Between keys
- **WHEN** a clip is sampled halfway between two keys
- **THEN** each bone's rotation is halfway along the arc between the keys' rotations

### Requirement: Character playback
`exarust character [moveset/action]` SHALL show the human body playing the requested clip in a loop.

#### Scenario: Walk
- **WHEN** `exarust character Default/Walk` runs
- **THEN** the body walks in place with arms at its sides

### Requirement: Skinned body
The system SHALL skin the human body with weights derived from its parts' seam triangles, so that adjacent parts stay joined in any pose, and SHALL skin worn apparel with the weights of the nearest body vertices.

#### Scenario: Walking
- **WHEN** the body plays a walk cycle
- **THEN** no gaps open at shoulders, elbows, hips or knees

### Requirement: Clips in place
The system SHALL remove a clip's horizontal root travel (the difference between its last and first keys, spread over the clip) when posing a body, since the character controller moves the body.

#### Scenario: Walk cycle
- **WHEN** the walk loops
- **THEN** the pelvis stays over the character instead of advancing and snapping back

### Requirement: Stature
The system SHALL scale a clip's root position by the posed body's leg length (thigh plus shin) relative to the human body's, so bodies of every race stand on the floor.

#### Scenario: Golem
- **WHEN** `exarust --race 16 character Idle` runs
- **THEN** the golem stands with its feet on the floor

### Requirement: Skins bound before drawing
The system SHALL draw a skinned body or apparel mesh only once it is bound to its character's body parts, so that no frame draws a mesh with skin attributes without its skin.

#### Scenario: Character view start
- **WHEN** the character view starts and its body is dressed over several frames
- **THEN** no frame fails GPU validation and the body appears skinned once bound

#### Scenario: Covered body parts
- **WHEN** apparel covers a body part before the part's mesh is bound
- **THEN** that body part stays hidden after binding
