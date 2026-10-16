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
The system SHALL simulate a limp body as the original does. Each part is a rigid tetrahedron of four particles laid over its mesh, integrated at 83 Hz under gravity, and joined to its parent at its origin and by the skeleton's links. A limp pivot (slack at most 1.19e-7 cm) SHALL close strength/(1 + 0.001|d|²) of its gap d in cm; a slack link SHALL damp its length inside the slack and take back strength times its excess beyond it.

#### Scenario: Body at rest
- **WHEN** a body goes limp above a floor
- **THEN** its parts fall, come to rest on the floor, every slack link stays within its slack and every part stays within a centimetre of its socket

### Requirement: Body iterations
The system SHALL solve a body in the original's order each tick: 20 iterations in which each part takes its links to its parent, then its level contacts from the last tick, then restores its edges (a living body children first, from the hands and feet in; a limp body parents first, depth first, from the pelvis out); then one test of the sphere colliders against static geometry, and 8 friction passes.

#### Scenario: Link and contact together
- **WHEN** a limb resting on a floor is pulled down by its parent within a tick
- **THEN** at the end of the tick its link is within its slack and its spheres are above the floor

#### Scenario: Solving order
- **WHEN** a body's parts branch from the pelvis into two chains
- **THEN** a limp body takes each chain from the pelvis out, one chain after the other, and a living body takes every part after all of its children

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
The system SHALL give a living physical body the animated pose of each tick: its targets are the body root's world transform times the clip pose, and the pose's own motion is this tick's targets against last tick's. The body SHALL be carried with its root's motion since the last tick, together with where the last tick left it, its level contacts and last tick's targets. A living body's links and level contacts SHALL be solved from the hands and feet in to the pelvis.

#### Scenario: Turning far from the origin
- **WHEN** a living body 80 m from the world's origin turns with its pose
- **THEN** its particles move no faster than the pose's own motion

#### Scenario: Carried by the controller
- **WHEN** the root of a body standing on a floor moves
- **THEN** its particles, where the last tick left each part and its level contacts move with the root, while where it is drawn from stays

### Requirement: Blows and daze
The system SHALL let a blow change the velocity of the struck part and take a share of the body's energy (at most 90 %, never below 0.05), which recovers by 0.005 per tick. Blows SHALL be queued and dealt by the next tick, whichever frame sends them.

#### Scenario: Stagger
- **WHEN** a body's chest is shoved at 3 m/s with daze 0.7
- **THEN** the chest is pushed off its pose, the body sags, and both recover within about a second

#### Scenario: Blow between ticks
- **WHEN** a blow is sent in a frame that runs no tick
- **THEN** it waits in the queue and the next tick deals it

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
The system SHALL step the simulation, character controllers and bodies alike, at the original's 83 Hz in the fixed step, so that the same ticks give the same result at any frame rate. A slow frame SHALL catch up at most 4 steps; the rest of the backlog SHALL be dropped, as the original drops it.

#### Scenario: Slow frame
- **WHEN** a frame takes 200 ms
- **THEN** at most 4 fixed steps run, and game time advances 48 ms

#### Scenario: Any frame rate
- **WHEN** a carried, driven, shoved body and a limp one run for three seconds at 30, 60, 144 and 240 FPS
- **THEN** after every tick every particle is exactly where it is at any other frame rate

### Requirement: Smooth character motion
The system SHALL draw characters and their body parts between their last two fixed steps, by the fraction of the next step already elapsed. Each body part SHALL be drawn under its drawn parent, so that no part is interpolated twice, and where its ticks put it whatever the frame did to its character or body root.

#### Scenario: Walking at 37 FPS
- **WHEN** the player walks while the frame rate is not a divisor of 83
- **THEN** the drawn position advances evenly from frame to frame

#### Scenario: Parts between ticks
- **WHEN** a falling body is drawn at 144 FPS
- **THEN** each part lies the elapsed fraction of the way from where the tick before last left it to where the last tick did

#### Scenario: Under a walking character
- **WHEN** a character walks under its controller and its body is drawn at 30, 60, 144 or 240 FPS
- **THEN** each part is drawn where its last two ticks put it, under the drawn character

#### Scenario: Root moved by the frame
- **WHEN** a frame moves and turns a body's root after the ticks, as getting up or turning toward a foe does
- **THEN** each part is still drawn where its last two ticks put it, with no one-frame jump

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

### Requirement: World tick order
The system SHALL run each 83 Hz tick of the bodies as one world, in the original's order: every living body's character tick, the player's last; the queued blows; the integration of every awake body; 20 iterations over all bodies; the bodies' level detection; 8 friction passes over all bodies; and the post-step of every body.

#### Scenario: Character ticks first
- **WHEN** a tick runs with the player and other living bodies
- **THEN** every living body's targets are set before any body is integrated, the player's after every other body's

#### Scenario: One body alone
- **WHEN** a single body ticks in the world
- **THEN** it ends the tick exactly where ticking it alone leaves it

### Requirement: Clips sampled every tick
The system SHALL advance a living physical body's clip clock by one tick, at 60 frames per second, and sample its clip inside the body's character tick, not once a frame. A clip that ends into its follow-up SHALL give the follow-up's first pose in that tick. Bodies that are not physical SHALL keep playing their clips once a frame.

#### Scenario: Clip clock at any frame rate
- **WHEN** a physical body plays a clip for 60 ticks at 30, 60, 144 or 240 FPS
- **THEN** its clock advances 60/83 of a frame per tick, and its targets after each tick are the body root's world transform times the clip's pose at that tick

#### Scenario: Frames leave the clip alone
- **WHEN** frames pass on a physical body's clip
- **THEN** only its ticks advance the clock

### Requirement: Body trace
`exarust --trace <file.csv>` SHALL write one row per traced body (the player's, or without a player every simulated body) after every tick, with the retail body probe's columns: the pelvis particles; the body's heights, tone, energy, support and balance; every part's target; the foot contacts; the motion and its clock. Lengths SHALL be in cm and the clock in clip frames; columns not simulated yet SHALL stay empty.

#### Scenario: Standing in the arena
- **WHEN** `exarust --trace t.csv play arena` runs for a minute
- **THEN** `t.csv` has one row per tick for the player's body, its clock advancing 60/83 of a frame per row, and its pelvis at the clip root's height above the floor

### Requirement: Race body parameters
The system SHALL decode, for every record of the installed `races.rdb`, the body parameters the original reads: the support-pin factor, the balance bonus, the stance factor, the arm relaxation factor, the limb damping ratio, and four gait rows (walk, jog, sprint, backpedal) of thrust, push scale, clip rate and boost speed. A race the database lacks SHALL get the original's fallback: every factor 1.0 and an all-zero gait table.

#### Scenario: Human
- **WHEN** the installed `races.rdb` is parsed
- **THEN** every record decodes, and Human has support-pin factor 1.25, balance bonus 1.0, stance factor 0.5, arm relaxation 1.0 and damping ratio 1.0, with gait thrusts 88.9, 350, 472.2 and 77.8 per second and push scale 550 in every row

#### Scenario: Skeleton
- **WHEN** the installed `races.rdb` is parsed
- **THEN** Skeleton has stance factor 0.75 and arm relaxation 0.5

#### Scenario: Unknown race
- **WHEN** a character's race byte names no race in the database
- **THEN** its body parameters are the fallback, whose gait table cannot move it

### Requirement: Joints at the part origins
The system SHALL join every part that has a parent part by its skeleton links and then a pivot from its origin to its place in the parent, the model's local translation of the part. A link's parent point SHALL be the stored point plus that translation, unturned. A correction SHALL be shared m/(m + m_parent), m being 0.175 times the bone's mobility, and spread over each part's particles by the point's weights, unclamped.

#### Scenario: Shipped human body
- **WHEN** the human body is built from the installed `humanoid.rsq` and `humanbase.rfc`
- **THEN** each femur has 3 links and each hand 4, the right humerus's first link meets the shoulder at (−21.58, −8.56, 6.51) cm, and the right knee's pivot sits at (−0.30, −43.19, −1.09) cm in the femur

#### Scenario: Shares
- **WHEN** a knee's link is corrected
- **THEN** the lower leg takes 0.588 of the correction and the femur 0.412

#### Scenario: Hips under a shove
- **WHEN** a standing body takes a 4 m/s blow to the torso
- **THEN** after every tick each femur's origin is within 2 mm of its socket on the pelvis

#### Scenario: Knee hinge
- **WHEN** a lower leg is struck sideways and forward
- **THEN** it turns about the knee's lateral axis and stays within 5° of the plane across it

### Requirement: Living joint corrections
In a living body a pivot (slack at most 0.01 cm) SHALL close d/(1 + 0.001|d|) of its gap d in cm, in full however far apart, and a slack link SHALL take back its excess beyond the slack at full weight, cut to 80 cm.

#### Scenario: Pivot pass
- **WHEN** a living pivot's two points are 10 cm apart
- **THEN** one pass closes 10/1.01 cm of the gap, shared by the two parts

#### Scenario: Torn limit
- **WHEN** a living slack link is stretched 3 m beyond its slack
- **THEN** one pass takes back 80 cm

### Requirement: Link strength and smoothing
Each link SHALL keep a strength: 1, except a lower leg's first link with over 10 cm of slack, its leg limit, at 0.2 while the body lives and 1 once it goes limp; the femur's first such link SHALL be its leg limit. Each link SHALL keep the smoothing (1 − (r² − 25)·0.001)², at most 0.99 and 0 within √0.1 cm, r being its point's distance from the part's origin in cm.

#### Scenario: Human leg
- **WHEN** a living human body is built
- **THEN** each lower leg's 12.66 cm link pulls at 0.2, each femur's 14.88 cm link is its leg limit, and the femur's point 12.49 cm from its origin has smoothing 0.755

#### Scenario: Gone limp
- **WHEN** the body goes limp
- **THEN** each lower leg's limit link pulls at full strength

### Requirement: Limp joint damping
Inside its slack a limp link SHALL update its smoothed length L ← s·L + (1 − s)·|d|, s being its smoothing and |d| its length, and take its points back to the L they had by strength·(|d| − L)/|d| of the gap. Beyond the slack L SHALL be the slack. L SHALL start at the posed distance when the body is built and at the distance, at most the slack, when the body goes limp.

#### Scenario: One tick of a damped link
- **WHEN** a limp link with smoothing 0.755 inside its slack is stretched 1 cm in a tick and each of the 20 passes closes its correction
- **THEN** 0.20 cm of the stretch is left at the end of the tick

#### Scenario: Swinging leg
- **WHEN** a limp leg swings at its hip inside the range of its links
- **THEN** its swing falls below a fifth within ten ticks, while a leg held by its hip alone keeps over four fifths

#### Scenario: Joint at rest
- **WHEN** a body with a joint bent inside its range and at rest goes limp
- **THEN** the joint stays where it is

#### Scenario: Stretched past the slack
- **WHEN** a body goes limp with a link stretched past its slack
- **THEN** that link's smoothed length is its slack

### Requirement: Velocity retention
A limp part SHALL keep its velocity as the original's default environment gives it: 0.426, 0.25 or 0.05 per second (0.98977, 0.98344 or 0.96455 per tick) when the farthest of its particles moved over 40/83 cm, over 20/83 cm, or less in the last tick. A living body's limbs SHALL keep the fast retention times u², u being 1 for a human, whatever their speed.

#### Scenario: Limp bands
- **WHEN** a limp part's farthest particle moved 0.49 cm, exactly 40/83 cm, or exactly 20/83 cm in the last tick
- **THEN** it keeps 0.98977, 0.98344 or 0.96455 of its velocity this tick

#### Scenario: Living limbs
- **WHEN** a living human body's part drifts at 6 cm/s
- **THEN** it keeps 0.98977 of its velocity per tick
