# physical-body Specification

## Purpose
Simulates every character's body as the original does: limb parts as rigid
particle tetrahedra stepped at 83 Hz, muscles that pull them toward the
animation, balance, falls and get-ups, blows, and limp bodies.

## Requirements

### Requirement: Body data
The system SHALL decode each skeleton bone's physical body chunk: its links to the parent part (flags, a point on the part and one on the parent, slack, stiffness, smoothing, and the axis of sliders), its foot contact points and its pins.

#### Scenario: Knee pivot
- **WHEN** `humanoid.rsq` is parsed
- **THEN** each lower leg has two links to its femur, one of them with zero slack

### Requirement: Limp bodies
The system SHALL simulate a limp body as the original does. Each part is a rigid tetrahedron of four particles laid over its mesh, integrated at 83 Hz under gravity with speed-banded velocity retention, and joined to its parent by links that are hard radii in the parts' rest frames.

#### Scenario: Body at rest
- **WHEN** a body goes limp above a floor
- **THEN** its parts fall, come to rest on the floor, and every link stays within its slack

### Requirement: Body iterations
The system SHALL solve a body in the original's order each tick: 20 iterations in which each part takes its links to its parent, then its level contacts from the last tick, then restores its edges (a limp body from the pelvis outward); then one test of the sphere colliders against static geometry, and 8 friction passes.

#### Scenario: Link and contact together
- **WHEN** a limb resting on a floor is pulled down by its parent within a tick
- **THEN** at the end of the tick its link is within its slack and its spheres are above the floor

### Requirement: Going limp
The system SHALL turn a posed body asked to go limp into a ragdoll from its current pose, stop animating it and remove its character controller.

#### Scenario: Corpses
- **WHEN** a level places a dead character
- **THEN** its body collapses where it was placed

#### Scenario: Character view
- **WHEN** `exarust --limp character` runs
- **THEN** the body drops to the floor in a sprawl with its parts joined

### Requirement: Muscles
The system SHALL drive a living physical body toward its animated pose. Each particle is pulled toward its place in the pose with damping relative to the pose's own motion. The gain is 0.084 per tick (0.07 for hands) times tone² times energy⁴, and gravity is compensated by tone times energy.

#### Scenario: Driven walk
- **WHEN** `exarust character` plays the walk on a physical body
- **THEN** the body walks with its parts within a centimetre of the animated pose

### Requirement: Living bodies in motion
The system SHALL spread the pose's motion over a frame's ticks as a motion of each part about itself, and solve a living body's links and level contacts from the hands and feet in to the pelvis. The body SHALL be carried along with its character controller's motion, together with where the last tick left it and its level contacts.

#### Scenario: Turning far from the origin
- **WHEN** a living body 80 m from the world's origin turns with its pose
- **THEN** its particles move no faster than the pose's own motion

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
The system SHALL stop simulating a limp body whose particles all stayed slower than 1 cm/s for 2 s, until it is shoved. A sleeping body SHALL NOT be tested against the level. Level contacts SHALL neither wake a body nor keep it awake. A woken body SHALL start without the contacts it had before it slept.

#### Scenario: Corpse at rest
- **WHEN** a corpse has lain still for 2 s
- **THEN** it is not simulated until a blow lands on it

#### Scenario: Woken corpse
- **WHEN** a sleeping corpse on the floor is shoved
- **THEN** it wakes with none of its earlier contacts and is tested afresh where it lies

### Requirement: Ragdoll stability
The system SHALL keep bodies stable under blows: a particle SHALL move at most 25 m/s; a fallen body SHALL align its targets to its pelvis's position and rotation only, without scale; and the switch to those targets SHALL NOT count as motion.

#### Scenario: Fallen far from the origin
- **WHEN** a living body 72 m from the world's origin falls and lies still while its pose keeps being aimed
- **THEN** no particle moves faster than 5 m/s

#### Scenario: Deep contact
- **WHEN** a body's sphere would end a tick deep behind a floor it crossed
- **THEN** it is taken back to where it first touched the floor, and no particle gains more than 25 m/s

### Requirement: Fallen muscles act inside the body
The system SHALL damp a fallen body's limbs relative to the body's own rigid motion (its bulk velocity and its spin), and its drive SHALL change neither the body's momentum nor its angular momentum.

#### Scenario: A fallen body kicking its legs
- **WHEN** a fallen body lies on its back on a floor while its pose kicks its legs and swings its arms for six seconds
- **THEN** its centre of mass stays within 30 cm of where it lay

### Requirement: Links keep momentum
The system SHALL make a link change its two parts' momenta by equal and opposite amounts, wherever its points lie in their tetrahedra.

#### Scenario: Links keep momentum
- **WHEN** a link is stretched between a part whose point lies inside its tetrahedron and a parent whose point lies outside its own
- **THEN** the link iterations leave the body's momentum unchanged

### Requirement: Level faces push from their front
The system SHALL test the sphere colliders of every awake body part against the level's triangles once per tick, after the link iterations. A triangle SHALL push a sphere only from its front, the side its winding faces in its own mesh, and only when the centre lies in front of its plane within the radius. The push SHALL move the centre off the closest point on the triangle by the overlap; it moves the part at once and is not clamped.

#### Scenario: Face seen from behind
- **WHEN** a sphere's centre lies behind a floor
- **THEN** the floor does not push it and it falls away

#### Scenario: Mirrored level mesh
- **WHEN** a level mesh is placed mirrored
- **THEN** its faces push from the same side as when it is placed unmirrored

### Requirement: Swept level contacts
The system SHALL sweep a sphere whose centre moved more than 3.2 mm since the end of the last tick from where that tick left it. While the end stays in front of a triangle the push SHALL be the one at the end; an end that crossed the triangle SHALL be taken back to where the sphere first touched it.

#### Scenario: Fast fall
- **WHEN** a ball of 6 cm radius falls onto a level floor at 24 m/s
- **THEN** its centre never passes below the floor, and it comes to rest touching it

#### Scenario: Thin wall
- **WHEN** a 6 cm ball flies at 20 m/s into a wall of zero thickness from its front, and another from behind
- **THEN** the first stops in front of the wall and the second passes through

#### Scenario: Crossing within a tick
- **WHEN** a sphere's centre would end a tick 4 cm under the floor it was 1 cm above
- **THEN** it is taken back above the floor in that tick

### Requirement: Stored level contacts
The system SHALL keep each level push as a contact of its part, at most four per part (the four spread widest over the part). Through the next tick's 20 link iterations each contact SHALL hold its point of the part on the outer side of a plane 1 mm under the touched surface; the contacts are then dropped.

#### Scenario: Limb dragged by its links
- **WHEN** a limb lying on the floor is pulled down by its parent at 20 m/s
- **THEN** during that tick's link iterations its sphere centres stay above the floor

#### Scenario: Knocked down far from the origin
- **WHEN** a living body standing on a level floor 90 m from the world's origin takes a dazing 4 m/s blow to the torso and loses its balance
- **THEN** it falls without any sphere centre passing below the floor and lies within 10 m of where it stood

### Requirement: Level friction
The system SHALL apply friction at level contacts from the part's own coefficient μ (2.5 for limbs, 0.5 for feet) and the contact normal n, with the limit 0.5·μ·(0.01 + 0.99·n_y²) in cm² per tick². A contact point's slide this tick whose square is under the limit is cancelled; a larger slide is reduced by 0.01 times the limit of itself. Friction SHALL run in 8 passes after detection and again in each of the next tick's 20 link iterations.

#### Scenario: Floor and wall
- **WHEN** a limb's contact point slides 1.1 cm in a tick on a floor, or 0.1 cm on a vertical wall
- **THEN** the slide is cancelled, while a slide of 1.2 cm on the floor or 0.2 cm on the wall is reduced by only 1.25 % or 0.0125 %

#### Scenario: Limp body at rest
- **WHEN** a limp body has dropped onto a floor
- **THEN** it does not creep, every sphere rests within 3 mm of touching the floor, and it falls asleep
