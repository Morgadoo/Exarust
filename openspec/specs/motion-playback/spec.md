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
The system SHALL sample a clip at any time by interpolating rotations spherically and the root position linearly between the keys around it; before its first key and past its last a looping clip SHALL blend from its last key back to its first (by time / first key, or (time − last key) / first key, no further than the first key) and one played once SHALL hold its first and last keys.

#### Scenario: Between keys
- **WHEN** a clip is sampled halfway between two keys
- **THEN** each bone's rotation is halfway along the arc between the keys' rotations

#### Scenario: Outside the keys
- **WHEN** a looping clip with keys at 10 and 20 is sampled at 5, 25 and 99 frames, and one played once at 3 and 99
- **THEN** the looping one is halfway from its last key to its first twice, then at its first key; the other is at its first key, then its last

### Requirement: Character playback
`exarust character [moveset/action]` SHALL show the human body playing the requested clip in a loop.

#### Scenario: Walk
- **WHEN** `exarust character Default/Walk` runs
- **THEN** the body walks in place with arms at its sides

### Requirement: Skinned body
The system SHALL skin the body's parts and worn garments with weights built from their seam triangles as the original builds them, so that the shipped bodies' parts stay joined in any pose. Edge falloffs SHALL follow the boundary marker class. Garments SHALL run the body's seam groups before groups of their marked triangles, and a position SHALL belong to the first group that reaches it.

#### Scenario: Walking
- **WHEN** the body plays a walk cycle
- **THEN** no gaps open at shoulders, elbows, hips or knees

#### Scenario: Shipped seams
- **WHEN** the installed human or ogre body is skinned
- **THEN** every two vertices of different parts at one rest position carry the same bones and weights

#### Scenario: A position in two groups
- **WHEN** a garment's marked triangle shares a corner with a wide seam
- **THEN** the corner takes the seam group's bone and share, not the marked group's

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

### Requirement: Backwards walk
Backing up SHALL play Walk and advance its clock by the locomotion clock's step on the backpedal row's negated rate. A looping clock below 0 SHALL wrap by the last key's time, and the locomotion SHALL keep the wrapped clock.

#### Scenario: Backing up
- **WHEN** a human backs up aligned at 1.476 cm a tick playing Walk
- **THEN** its clock loses 0.736 frames a tick

#### Scenario: Negative wrap
- **WHEN** a looping clip's clock is 0.3 and the tick subtracts 0.8, and the last key is at 80
- **THEN** the clock is 79.5

### Requirement: Run overlay
While Walk plays and the run blend is above zero, each bone SHALL be the Walk sample slerped toward the Run sample at the same keys, before any pose hold, by 0.7 times the run blend (in double precision), or by the run blend itself while the sprint latch is kept, whatever the stance level and the speed. The root SHALL stay the Walk sample.

#### Scenario: Full run
- **WHEN** the two clips differ by 90° about one axis and the run blend is 1
- **THEN** that bone is 63° from the walk toward the run, and the root is the walk's

#### Scenario: No run
- **WHEN** the run blend is 0
- **THEN** the pose is the walk sample

#### Scenario: Full sprint
- **WHEN** the sprint latch is kept, the run blend is 1 and the two clips differ by 90° about one axis
- **THEN** that bone is the run's, and the root is the walk's

#### Scenario: Half sprint
- **WHEN** the sprint latch is kept, the run blend is 0.5 and the two clips differ by 90° about one axis
- **THEN** that bone is 45° from the walk toward the run

#### Scenario: Level at 0.9
- **WHEN** the latch is kept, the stance level is 0.9, the speed is above 3.5 cm a tick and the run blend is 1
- **THEN** that bone is the run's

#### Scenario: Speed at 3.5 cm
- **WHEN** the latch is kept, the stance level is above 0.9, the speed is 3.5 cm a tick and the run blend is 1
- **THEN** that bone is the run's

### Requirement: Sprint clock
With the sprint latch kept, a forward gait's clock rate SHALL be × (1 − 0.1 × the encumbrance), on the jog row too. While sprinting (the latch, a stance level above 0.9, over 3.5 cm a tick) the forward gait SHALL blend from the walk toward the feral sprint row, linearly by the run blend while running and by its square root otherwise. The feral sprint row SHALL be the sprint row blended to the jog row by the feral level.

#### Scenario: Unencumbered sprint
- **WHEN** a human sprints at feral level 0, run blend 1 and 4.337 cm a tick
- **THEN** its clock gains 0.727 frames a tick

#### Scenario: Feral sprint
- **WHEN** a human sprints at feral level 1, with no encumbrance, run blend 1 and 4.337 cm a tick
- **THEN** its clock gains 0.894 frames a tick

#### Scenario: Sprint in armour
- **WHEN** a human sprints at feral level 0, encumbrance 0.5, run blend 1 and 4.337 cm a tick
- **THEN** its clock gains 0.691 frames a tick

#### Scenario: Level at 0.9
- **WHEN** the latch is kept, the stance level is 0.9, the run blend is 1 and the speed is 4.337 cm a tick
- **THEN** its clock gains 0.894 frames a tick

#### Scenario: Speed at 3.5 cm
- **WHEN** the latch is kept, the stance level is above 0.9, the run blend is 1 and the speed is 3.5 cm a tick
- **THEN** its clock gains 0.76 frames a tick

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

### Requirement: Clip rate by posture
The posture rules SHALL set a clip's rate each body tick from the race's clip-rate base: × 1.25 off balance (falling, lying, a get-up's first tick), × (1.5 − 0.5 × encumbrance) getting up, the swing's rate attacking, × 2 kicking or parrying at tone 0.85 or more, else × 1; under that tone, and in the walk or the run without a parry, the rate SHALL stay as last set. The walking clock SHALL ignore the rate.

#### Scenario: Dazed in the Defence stance
- **WHEN** a human off balance plays its Defence stance, or any clip that is not its stance
- **THEN** its clock gains 0.6 frames a tick, or 0.75

#### Scenario: Kicking
- **WHEN** a human kicks at full tone, or weak (tone 0.8)
- **THEN** its clock gains 1.2 frames a tick, or 0.6

#### Scenario: A parry doubles the rate
- **WHEN** a human parries at full tone, including while its stance plays
- **THEN** its clock gains 1.2 frames a tick

#### Scenario: Off balance a parry keeps the fall rate
- **WHEN** a parrying human off balance plays its stance
- **THEN** its clock gains 0.75 frames a tick

#### Scenario: An attack off balance
- **WHEN** a human off balance plays an attack's clip
- **THEN** its clock gains 1.25 frames a tick, the rate as it is

### Requirement: Sampler pipeline
A living body's sampler SHALL take each sample in the original's steps, in order: the keys blended, the gait overlay, the hold (normalised, kept as the targets), the femur posture, the clip chain (each part placed by its rest translation and turn under its parent), the IK chain, the torso twist, the head look and the foot pitch; then the drive's anchors and the feet. A step whose rules are not ported SHALL be the identity.

#### Scenario: Identity steps
- **WHEN** a standing human samples Idle
- **THEN** its parts' placements are the keys blended on the chain, and its trace's clip columns are as before the pipeline: the clock 0.6 a tick, no blend, no hold

### Requirement: Foot pitch
A foot SHALL pitch about its side axis so a point 20 cm times the body scale ahead of the ankle meets the ground. Off its feet the foot SHALL keep the clip; lying down and not getting up, the stored pitch SHALL ease toward 0 by 0.02 rad a tick plus its rate. A correction of 0.1 cm or under SHALL leave the pitch, and the angle SHALL ease by at most 0.01 rad a tick plus its stored rate. A placement SHALL store that angle at once and clear the rate.

#### Scenario: Shin tipped up
- **WHEN** a foot's shin points straight up, its contact is on its ground, and its pitch starts at 0
- **THEN** the pitch is 0.01 rad on the first tick, toward the angle that levels that toe

#### Scenario: A placement
- **WHEN** a placed shin points straight up and its foot is placed with it
- **THEN** the pitch is the whole angle that levels the toe and the rate is 0

### Requirement: Clocks without a body tick
A body without a body tick (the character view's figure, a non-physical character) SHALL tick its clip clock by the same body ticks (83 a second), counted from the frames' time with what is under a tick owed to the next frame, each tick gaining 0.6 × the race's base as a standing body's. No clip clock SHALL run per frame.

#### Scenario: Two 50 ms frames
- **WHEN** a human figure without a body tick is drawn for two 50 ms frames
- **THEN** its clock ticks 8 times, gaining 4.8 frames, and 0.3 of a tick is owed; a base of 1.5 gains 7.2

### Requirement: Clock quantities exposed
Each tick SHALL keep the frames its clock gained and whether its end test fired, and each sample SHALL write the progress, (clock − blend) / the last key's time (0 at a start), first; the swing clock, the posture rules and the body trace read them.

#### Scenario: The walk's wrap
- **WHEN** the walk (last key 80) at 79 gains 0.6 and is sampled, then gains 0.6 again
- **THEN** the sample's progress is 0.995, and the second tick's end test fires with the clock at 0.2

### Requirement: Locomotion clock
The locomotion clock's step SHALL be, with support, K × (0.8·speed + 1) × the gait's clock rate, K = (1 − k)·(1 − 1.8·|0.5 − frac(2·(progress + 1.5))|)·4 + k, k = 1 − (1 − alignment)² (1 aligned); stopping at no more than 5 cm a tick, rate × speed × K. Without support it SHALL keep the walk's phase from when the body lost it, 0.85 − 0.5·round(frac(progress + 1)), and step (2 − u)·u·(0.8·speed + 0.9)·rate, u = frac(1 + phase − progress). frac SHALL keep the sign of its argument.

#### Scenario: Misaligned
- **WHEN** a human walks 1.687 cm a tick at an alignment of 0.8, at the cycle's start and a quarter in
- **THEN** its step is 0.764 × 0.976, then × 1.12

#### Scenario: Stopping
- **WHEN** a human stops softly at 3 cm a tick, then at 6
- **THEN** its step is 0.3253 × 3, then 0.3253 × 5.8

#### Scenario: In the air
- **WHEN** a human without support walks 1.687 cm a tick at progress 0.3
- **THEN** its phase is 0.85 and its step 0.5836; back on its feet the phase is unset

### Requirement: Motion library fallback
Without `humanoid.rsq` the motion library SHALL be one unnamed action and one `Default` moveset without a parent or clips, with no bones; Exarust SHALL take the same library when the file does not parse.

#### Scenario: No file
- **WHEN** the library is loaded without `humanoid.rsq`
- **THEN** no character finds a clip

### Requirement: Child sort by absolute x
The loader's child sort SHALL fill each slot with the first child, from that slot on, whose absolute x is strictly below the best so far (starting at 3.4e38), the best index starting at 0, so that a slot whose remaining children are all NaN swaps with the first child.

#### Scenario: A NaN child
- **WHEN** children with x NaN and 1.0 are sorted
- **THEN** they end in the order NaN, 1.0

### Requirement: CPU vertex skinning
The CPU vertex skin SHALL blend the joints' 3×4 matrices by weight, starting with the first joint and stopping at the first joint number below 1, and SHALL transform the point once, each component summed as (x·a + y·b) + (z·c + t).

#### Scenario: A stopped blend
- **WHEN** a vertex at (1, 1, 1) has joints 1, 0, 2 with weights 1, 0, 1 and joint 1 translates by (1, 0, 0)
- **THEN** it skins to (2, 1, 1)

### Requirement: Clip clock gains
A clip's clock SHALL gain frames each body tick from its rate (the race's clip-rate base × its posture's factor): 0.6 × rate standing; tone × rate × 0.6 getting up above half tone; the swing rate attacking (the rate itself off balance); the locomotion's step whenever it locomotes, whatever the clip; none while shocked.

#### Scenario: Standing human
- **WHEN** a human stands in Idle
- **THEN** its clock gains 0.6 frames a tick and wraps from 80 to 0 every 133 ticks

#### Scenario: Getting up and walking
- **WHEN** a human at full tone gets up, or walks aligned 1.687 cm a tick
- **THEN** its clock gains 0.9, or 0.764 frames a tick

#### Scenario: Getting up in plate
- **WHEN** a human at full tone in armour of encumbrance 0.625 gets up
- **THEN** its clock gains 0.7125 frames a tick

#### Scenario: A get-up's first tick
- **WHEN** a human at full tone starts getting up, from the floor or staggering
- **THEN** its clock gains 0.75 frames on that tick and 0.9 a tick after

#### Scenario: Shocked
- **WHEN** a body's condition is shocked
- **THEN** its clock gains nothing, unless its clip is an attack's

#### Scenario: A walk clip without locomotion
- **WHEN** the walk still plays on a body that no longer locomotes
- **THEN** its clock gains 0.6 frames a tick

### Requirement: Feral thorax hunch
A pinned part SHALL pitch its local forward axis down by the thorax hunch before the clip chain when the character's feral level, in double, is over 0.1, then rebuild the basis from that forward, the up axis and their cross. The hunch rises 0.01 a tick in double precision to 0.3 on its feet and falls 0.01 to 0 off them. A balanced attack tick (an attack's clip playing, not off balance, shocked or getting up) SHALL take a further 0.02 off.

#### Scenario: Standing thorax
- **WHEN** a pinned part's turn is identity and its hunch is 0.3, with feral level 1
- **THEN** its forward axis drops below −0.25 and the basis stays orthonormal

#### Scenario: Not feral
- **WHEN** the same part's feral level is 0, or 0.05
- **THEN** its turn stays the clip's

#### Scenario: The attack's ease
- **WHEN** a body with hunch 0.3 attacks on balance, and then attacks off balance, shocked or getting up, or stands normally
- **THEN** the hunch is 0.28 after the balanced attack tick and stays 0.3 otherwise

### Requirement: Clock precision
The standing and getting-up gains SHALL each be a double product added to the clock widened to double, the sum narrowed once; an attack's rate and a locomotion step SHALL add in single precision.

#### Scenario: Added in double
- **WHEN** a clock at 0.0274 gains 0.6 × 0.3, or a get-up clock at 0.0548 gains 0.63
- **THEN** it reads 0.20739999 (a single-precision add gives 0.20740001), or 0.68480003

### Requirement: Clip end and loop
A clip SHALL end when its clock less its blend reaches its last key; a looping one SHALL then wrap by that key's time, frames before the first key blending from the last.

#### Scenario: Walk loop
- **WHEN** the walk wraps from its last key (80) to frame 0
- **THEN** its pose blends from the last key back to the first over 10 frames, its targets turning no joint more than 4° a tick

### Requirement: Stance rate reset
While the clip playing is the body's stance and it does not parry (the previous tick's parry flag), the rate SHALL be the race's base, whatever its balance and even while shocked. While it parries, unless off balance, getting up, attacking or weak, the rate SHALL be × 2. A shocked tick otherwise SHALL keep the last rate unless an attack's clip plays.

#### Scenario: Shocked rates
- **WHEN** a shocked body whose rate was 2 plays its stance, or another clip
- **THEN** its rate is the base 1, or stays 2, and its clock gains nothing

### Requirement: Ease arithmetic
The smooth ease SHALL be (3 − 2f)·(f·f) in single precision. The fast ease SHALL be (f·0.11 + 1) − 1/(8f + 1), the first term and the sum in double with the double 0.11, the reciprocal single, narrowed once.

#### Scenario: Ends of the fast ease
- **WHEN** the fast ease is taken at 0 and at 1
- **THEN** it gives 0 and 0.99889

### Requirement: Hold bones
The hold pose and the plain capture SHALL keep the last targets of the same bones, skin bones 1 to 7 (the pelvis and legs), and take the physical turn for the rest.

#### Scenario: The last leg bone
- **WHEN** a hold pose is taken from a body whose seventh bone has a kept target
- **THEN** the seventh bone keeps it and the eighth takes its physical turn
