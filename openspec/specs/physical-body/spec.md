# physical-body Specification

## Purpose
TBD - created by archiving change add-ragdoll. Update Purpose after archive.

## Requirements

### Requirement: Body data
The system SHALL decode each skeleton bone's physical body chunk: its links to the parent part (flags, a point on the part and one on the parent, slack, stiffness, smoothing, and the axis of sliders), its foot contact points and its pins.

#### Scenario: Knee pivot
- **WHEN** `humanoid.rsq` is parsed
- **THEN** each lower leg has two links to its femur, one of them with zero slack

### Requirement: Limp bodies
The system SHALL simulate a limp body as the original does. Each part is a rigid tetrahedron of four particles laid over its mesh, integrated at 83 Hz under gravity with speed-banded velocity retention. Links between parts are hard radii in the parts' rest frames, solved 20 times per tick. Sphere colliders are pushed out of static geometry 8 times per tick with friction.

#### Scenario: Body at rest
- **WHEN** a body goes limp above a floor
- **THEN** its parts fall, come to rest on the floor, and every link stays within its slack

### Requirement: Going limp
The system SHALL turn a posed body asked to go limp into a ragdoll from its current pose, stop animating it and remove its character controller.

#### Scenario: Corpses
- **WHEN** a level places a dead character
- **THEN** its body collapses where it was placed

#### Scenario: Character view
- **WHEN** `exarust --limp character` runs
- **THEN** the body drops to the floor in a sprawl with its parts joined

### Requirement: Muscles
The system SHALL drive a living physical body toward its animated pose. Each particle is pulled toward its place in the pose with damping relative to the pose's own motion. The gain is 0.084 per tick (0.07 for hands) times tone² times energy⁴, and gravity is compensated by tone times energy. The body is carried along with its character controller's motion.

#### Scenario: Driven walk
- **WHEN** `exarust character` plays the walk on a physical body
- **THEN** the body walks with its parts within a centimetre of the animated pose

### Requirement: Blows and daze
The system SHALL let a blow change the velocity of the struck part and take a share of the body's energy (at most 90 %, never below 0.05), which recovers by 0.005 per tick.

#### Scenario: Stagger
- **WHEN** a body's chest is shoved at 3 m/s with daze 0.7
- **THEN** the chest is pushed off its pose, the body sags, and both recover within about a second

### Requirement: Balance
The system SHALL keep each living body's balance. The centre of mass, projected ahead by its velocity, is measured against the segment between the feet with a threshold of (energy·0.75 + 0.25)·30 cm. The score is stability⁸. A counter grows by 10 − 10·score per tick and at 100 sets the balance to the score. Otherwise balance recovers by (5 − speed in cm/tick)·0.01 per tick.

#### Scenario: Standing still
- **WHEN** an alert body stands with its centre of mass over its feet
- **THEN** it keeps its balance indefinitely

### Requirement: Falling and getting up
The system SHALL make a body whose balance drops below 0.6 fall: it loses its controller's support, its tone drops to 0.5, it plays the fall clip in the direction it is going, and it is not steered. Once it has lain for 2.5 s and its energy is back to 0.9, it SHALL get up with the get-up clip whose first pose lies the way the body does, placed under the body.

#### Scenario: Knocked over
- **WHEN** `exarust --shove 60 play arena` deals the player a body blow
- **THEN** the player falls onto its back, lies a moment, gets up with `Get Up 2`, and stands where it fell

### Requirement: Fixed simulation rate
The system SHALL step the simulation at the original's 83 Hz. A slow frame SHALL catch up at most 4 steps; the rest of the backlog SHALL be dropped, as the original drops it.

#### Scenario: Slow frame
- **WHEN** a frame takes 200 ms
- **THEN** at most 4 fixed steps run, and game time advances 48 ms

### Requirement: Smooth character motion
The system SHALL draw characters between their last two fixed steps, by the fraction of the next step already elapsed.

#### Scenario: Walking at 37 FPS
- **WHEN** the player walks while the frame rate is not a divisor of 83
- **THEN** the drawn position advances evenly from frame to frame

### Requirement: Sleeping limp bodies
The system SHALL stop simulating a limp body whose particles all stayed slower than 1 cm/s for 2 s, until it is shoved.

#### Scenario: Corpse at rest
- **WHEN** a corpse has lain still for 2 s
- **THEN** it is not simulated until a blow lands on it

### Requirement: Ragdoll stability
The system SHALL keep bodies stable under blows: a particle SHALL move at most 25 m/s, a single contact push SHALL be at most twice the sphere's radius, and a fallen body SHALL align its targets to its pelvis's position and rotation only, without scale.

#### Scenario: Deep contact
- **WHEN** a body's sphere is found deep inside level geometry
- **THEN** it is pushed out at most twice its radius per contact pass, and no particle gains more than 25 m/s
