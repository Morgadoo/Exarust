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
The system SHALL link the movesets into a tree, each one's parent being the nearest moveset before it named exactly as its parent, and SHALL build a character's clip table from its base moveset (the first of that name, case ignored) and the base's ancestors, overlaid from the root down: each clip a moveset defines replaces the table's. It SHALL build each table once per base and held-item config; a base no moveset is named after SHALL play nothing.

#### Scenario: Inherited walk
- **WHEN** the `1H & Shield` moveset's walk is requested
- **THEN** the walk of its ancestor `Default` is returned

#### Scenario: Names used twice
- **WHEN** the movesets of the installed `humanoid.rsq` are linked
- **THEN** the golem's `1H & Shield` (#16) hangs under the golem's `One Handed` (#15), not the human's (#1), and the ogre's under the ogre's

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
The system SHALL leave out the root's travel along z of the walk, the run and the backpedal (and of Exarust's interim `Combat Walk`), whose travel the body's own motion makes. Every other clip SHALL keep its root's travel.

#### Scenario: Walk cycle
- **WHEN** the walk loops
- **THEN** the pelvis stays over the character instead of advancing and snapping back

#### Scenario: Get-up travel
- **WHEN** `Get Up 2` is sampled at its last key
- **THEN** its root is the key's own, (6.1, 81.7, −1.8) cm, not its first key's place across the ground

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

### Requirement: Clip clocks
A clip's clock SHALL gain frames each body tick, from the race's clip-rate base: 0.6 × base standing, × 1.25 off balance; tone × (1.5 − 0.5 × encumbrance) × base × 0.6 getting up above half tone (tone × 0.75 × base on the tick it starts); the swing rate attacking; the gait's clip rate ÷ 83 × (0.8 × speed + 1) walking. A clip SHALL end when its clock less its blend reaches its last key; a looping one SHALL then wrap by that key's time, its frames before the first key blending from the last.

#### Scenario: Standing human
- **WHEN** a human stands in Idle
- **THEN** its clock gains 0.6 frames a tick and wraps from 80 to 0 every 133 ticks

#### Scenario: Getting up and walking
- **WHEN** a human at full tone gets up, or walks 1.687 cm a tick
- **THEN** its clock gains 0.9, or 0.764 frames a tick

#### Scenario: Walk loop
- **WHEN** the walk wraps from its last key (80) to frame 0
- **THEN** its pose blends from the last key back to the first over 10 frames, its targets turning no joint more than 4° a tick

#### Scenario: Getting up in plate
- **WHEN** a human at full tone in armour of encumbrance 0.625 gets up
- **THEN** its clock gains 0.7125 frames a tick

#### Scenario: A get-up's first tick
- **WHEN** a human at full tone starts getting up, from the floor or staggering
- **THEN** its clock gains 0.75 frames on that tick and 0.9 a tick after

### Requirement: Clip starts from the body
Starting a clip SHALL capture the root, the pelvis's and legs' last targets and the upper body's physical turns (a get-up from the floor: every part's, the pelvis's on the frame), which become the targets. The clip SHALL blend from the capture to its first key by clock / (first key + blend), linear, smooth f²(3 − 2f) or fast 1 + 0.11f − 1/(1 + 8f); once it aims past its first key the blend SHALL end and the clock lose it.

#### Scenario: First sample
- **WHEN** a clip starts
- **THEN** its first sample is the capture, and a get-up's next tick is 0.9/(first key + 20) of the way to the first key

#### Scenario: On time
- **WHEN** Get up (first key 20) and Get Up 2 (15) start with a blend of 20 at 0.9 a tick, or Idle with none at 0.6
- **THEN** they reach their first key after 44, 38 and 33 ticks

### Requirement: Follow-up blends
A clip played once SHALL end into its follow-up with a blend of twice its weapon's recovery after an attack (actions 5–12), 60 frames from action 0x21, 20 from 0x1F and none below, eased smoothly; with no follow-up it SHALL hold its last key.

#### Scenario: End of a get-up
- **WHEN** `Get Up 2` ends into Idle
- **THEN** Idle starts from the get-up's last root, (6.1, 81.7, −1.8) cm, and blends to its own over 20 frames, the pelvis target never moving 1 cm in a tick

#### Scenario: End of an attack
- **WHEN** a duellist's attack ends into Cmbt - Idle
- **THEN** no joint moves more than 2° a tick in the 15 ticks after

### Requirement: Clip table from held items
The system SHALL overlay on a character's table the moveset its held-item config names (low 16 bits: One Handed 0x1 … Polearm 0x40), searched under each ancestor of the base from the root down, children first, each one found with its ancestors below that ancestor; then the Torch (0x10010000) and Flaming (0x10020000) overlays when the config shares a bit with their mask, searched among the children of the weapon movesets found and of their ancestors below that ancestor.

#### Scenario: Golem with a sword
- **WHEN** a golem's table (base `Golem`) is built for the One Handed config
- **THEN** it draws on `Default`, `Golem`, the human `One Handed` and the golem's own `One Handed`, whose clips win

#### Scenario: Torch over a shield
- **WHEN** a human's table is built for a shield with a light in the main hand
- **THEN** Walk, Cmbt - Idle, Attack Mid and Idle come from `Torch` and the Defence stance from `1H & Shield`

#### Scenario: Human weapons
- **WHEN** a human's table is built for one held weapon or a light
- **THEN** it holds the same clips as the chosen moveset's own chain of parents

### Requirement: Node keyframe tracks
The system SHALL decode the keyframe tracks a stored scene node carries after its body, whatever its kind: the play mode (free, once, reverse, loop; any unknown word free), the duration and the keys of time (seconds) and position (the node's translation in its parent's space, metres). A track that does not decode SHALL keep its error and nothing SHALL panic. It SHALL hold for every chunk tree of the installed packs, nested packs included.

#### Scenario: Shipped tracks
- **WHEN** every chunk tree of the installed packs is decoded
- **THEN** exactly one track is found, on node `Box001` under `Plane001` of `uwa01 brkswtch` in `Objects.rpk`: once, 3 s, keys at 0, 1, 2 and 3 s with z at 0, −5, −5 and 0 cm, and no other node kind than a mesh carries a collision body or cloth marker

#### Scenario: Truncated track
- **WHEN** a track's keys run past the end of its chunk
- **THEN** the node keeps an error for that track and the rest of the file decodes

### Requirement: Keyframe curve
The system SHALL place a track's node at curve time t: on the first key's position up to its time, on the last key's from its time on, and between them on the cubic Hermite curve of the bracketing keys, each inner key's slope the chord of its neighbours scaled by the segment's share of their span; the first segment SHALL leave its first key with no slope and the last segment SHALL arrive along its own chord.

#### Scenario: The brick's curve
- **WHEN** the shipped track is sampled at 0.5, 1.5 and 2.5 s
- **THEN** the node's z is −2.1875, −5.625 and −2.8125 cm

#### Scenario: Outside the keys
- **WHEN** a track is sampled before its first key or after its last
- **THEN** the node takes the first or the last key's position

### Requirement: Playing keyframe tracks
Starting a node SHALL start every track of it and of the nodes under it that is not playing, at the clock's time, and swap once and reverse. Each frame a playing track's node SHALL take the curve's position at the time since its start: as is (free), clamped at the duration (once), counted down from it (reverse), wrapped by it (loop). Once and reverse SHALL end at their limit, leaving the node there. The clock SHALL stand still while the game is paused.

#### Scenario: First and second press
- **WHEN** the secret switch's object is started, started again while its brick moves, and started once more after the brick stopped
- **THEN** the first play runs in reverse (−2.8125 cm 0.5 s in) and ends 3 s later at rest, the start during it changes nothing, and the last play runs forward (−2.1875 cm 0.5 s in)

#### Scenario: Placed object
- **WHEN** a placed object whose child node carries a track is started
- **THEN** the child node moves along the curve in its parent's space and the object's placement stays

#### Scenario: Debug start
- **WHEN** F7 is pressed in a level
- **THEN** every keyframe track of the level starts and the log names each with its mode, duration and position

### Requirement: Starting a few keys in
A clip started a few keys in SHALL have its capture meant for that key and its clock at the key before it. A sample aiming at any key other than the one its capture is meant for SHALL drop the blend (the clock loses it) and sample the keys; a clip played once then holds its first key until its clock reaches it.

#### Scenario: Get up three keys in
- **WHEN** Get up (keys 20, 40, 55, 70, 80) starts with a blend of 50 three keys in and ticks 0.75 frames, then 0.9 a tick
- **THEN** its clock starts at 55, reads 5.75 after its first sample, which is the first key exactly, and the first key holds for 16 more ticks

#### Scenario: Key 0
- **WHEN** a clip is started at key 0, or at a key past its last
- **THEN** its start is unchanged
