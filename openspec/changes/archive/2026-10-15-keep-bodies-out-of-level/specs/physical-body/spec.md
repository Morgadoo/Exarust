## ADDED Requirements

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

## MODIFIED Requirements

### Requirement: Limp bodies
The system SHALL simulate a limp body as the original does. Each part is a rigid tetrahedron of four particles laid over its mesh, integrated at 83 Hz under gravity with speed-banded velocity retention. Each tick runs 20 iterations from the pelvis outward: each part takes its links to its parent (hard radii in the parts' rest frames), then its level contacts from the last tick, then restores its edges. The sphere colliders are then tested against static geometry once, and their friction applied in 8 passes.

#### Scenario: Body at rest
- **WHEN** a body goes limp above a floor
- **THEN** its parts fall, come to rest on the floor, and every link stays within its slack

### Requirement: Muscles
The system SHALL drive a living physical body toward its animated pose. Each particle is pulled toward its place in the pose with damping relative to the pose's own motion. The pose's motion over a frame is spread over its ticks as a motion of each part about itself. The gain is 0.084 per tick (0.07 for hands) times tone² times energy⁴, and gravity is compensated by tone times energy. A living body's links and level contacts are solved from the hands and feet in to the pelvis. The body is carried along with its character controller's motion, together with where the last tick left it and its level contacts.

#### Scenario: Driven walk
- **WHEN** `exarust character` plays the walk on a physical body
- **THEN** the body walks with its parts within a centimetre of the animated pose

### Requirement: Ragdoll stability
The system SHALL keep bodies stable under blows: a particle SHALL move at most 25 m/s; a fallen body SHALL align its targets to its pelvis's position and rotation only, without scale; the switch to those targets SHALL NOT count as motion; a fallen body's muscles SHALL act inside the body: its limbs are damped relative to the body's own rigid motion (its bulk velocity and its spin), and the drive SHALL change neither the body's momentum nor its angular momentum; and a link SHALL change its two parts' momenta by equal and opposite amounts, wherever its points lie in their tetrahedra.

#### Scenario: Fallen far from the origin
- **WHEN** a living body 72 m from the world's origin falls and lies still while its pose keeps being aimed
- **THEN** no particle moves faster than 5 m/s

#### Scenario: A fallen body kicking its legs
- **WHEN** a fallen body lies on its back on a floor while its pose kicks its legs and swings its arms for six seconds
- **THEN** its centre of mass stays within 30 cm of where it lay

#### Scenario: Links keep momentum
- **WHEN** a link is stretched between a part whose point lies inside its tetrahedron and a parent whose point lies outside its own
- **THEN** the link iterations leave the body's momentum unchanged

#### Scenario: Deep contact
- **WHEN** a body's sphere would end a tick deep behind a floor it crossed
- **THEN** it is taken back to where it first touched the floor, and no particle gains more than 25 m/s

### Requirement: Sleeping limp bodies
The system SHALL stop simulating a limp body whose particles all stayed slower than 1 cm/s for 2 s, until it is shoved. A sleeping body SHALL NOT be tested against the level. Level contacts SHALL neither wake a body nor keep it awake. A woken body SHALL start without the contacts it had before it slept.

#### Scenario: Corpse at rest
- **WHEN** a corpse has lain still for 2 s
- **THEN** it is not simulated until a blow lands on it

#### Scenario: Woken corpse
- **WHEN** a sleeping corpse on the floor is shoved
- **THEN** it wakes with none of its earlier contacts and is tested afresh where it lies
