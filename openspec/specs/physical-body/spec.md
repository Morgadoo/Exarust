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
The system SHALL turn a posed body asked to go limp into a ragdoll from its current pose, stop animating it and remove its character controller. It SHALL keep the motion it has unless the request gives it a velocity.

#### Scenario: Corpses
- **WHEN** a level places a dead character
- **THEN** its body collapses where it was placed

#### Scenario: Character view
- **WHEN** `exarust --limp character` runs
- **THEN** the body drops to the floor in a sprawl with its parts joined

#### Scenario: Death
- **WHEN** a collapsing body dies at a tone of 0.25
- **THEN** it goes limp the same tick with the velocity its parts have, none added

### Requirement: Muscles
The system SHALL drive a living physical body toward its animated pose: its link muscles turn every joint, and a pose drive turns the pelvis, lumbar, thorax and feet (gain 0.03·K, K = 19200/83²) and the head (0.01·K times its character's head gain) toward their animated orientation about where each part's origin is, with no position target. Thighs, shins, arms and hands SHALL have no pose drive. The carry holds the body up in place of gravity.

#### Scenario: Driven walk
- **WHEN** `exarust character` plays the walk on a physical body
- **THEN** the body walks with every joint within 15° of the clip's angle, but for the tick its loop wraps, and no limb flails

#### Scenario: Limbs by their muscles
- **WHEN** a living body's arms and legs are aimed turned away from how they are turned
- **THEN** the pose drive leaves their particles where they are

#### Scenario: Gains
- **WHEN** an alert human stands, walks and runs
- **THEN** its pose drive's gain is 0.0314 on the pelvis, spine and feet and 0.0105 on the head standing, 0.0497 walking and 0.0836 running

#### Scenario: Rigid translation
- **WHEN** a body is moved and moves as a whole, each part turned as its pose turns it
- **THEN** the pose drive pushes no particle

#### Scenario: Turned part
- **WHEN** a part at rest is turned 0.3 rad about the vertical off its pose
- **THEN** one tick turns it back by G of the angle, about its origin, which stays where it is

#### Scenario: Feral heads
- **WHEN** a half-feral human and a fully feral one stand alert
- **THEN** their heads' pose gain is 0.0052 and none, their trunks' still 0.0314

### Requirement: Living bodies in motion
The system SHALL give a living physical body the animated pose of each tick on its character frame. A standing body SHALL be carried with its root's motion since last tick, less its controller's follow, across the ground and about the vertical only, with where the last tick left it, its level contacts, its feet's anchors, last tick's targets and its frame. No spring of 0.168 × tone² × energy⁴ SHALL pull or brake its pelvis toward that frame.

#### Scenario: Turning far from the origin
- **WHEN** a living body 80 m from the world's origin turns with its pose
- **THEN** its particles move no faster than the pose's own motion

#### Scenario: Carried by the controller
- **WHEN** the root of a body standing on a floor moves across it and down
- **THEN** its particles, where the last tick left each part and its level contacts move with the root across the floor but not down, while where it is drawn from stays

#### Scenario: Held on the controller
- **WHEN** a standing body's pelvis, still or moving, sits off where its frame puts the pose's pelvis
- **THEN** the step does not pull it across the ground by 0.168 × tone² × energy⁴, and does not brake its horizontal velocity by twice the square root of that gain

#### Scenario: A standing turn
- **WHEN** the player turns a quarter standing in the arena
- **THEN** its planted feet are carried round with it, within 0.1 cm of their anchors

### Requirement: Blows and daze
The system SHALL let a blow change the velocity of the struck part and take a share of the body's energy (at most 90 %, never below 0.05), which recovers by 0.005 per tick up to the body's most energy: its character's, 1 for a human that is not feral. Blows SHALL be queued and dealt by the next tick, whichever frame sends them.

#### Scenario: Stagger
- **WHEN** a body's chest is shoved at 3 m/s with daze 0.7
- **THEN** the chest is pushed off its pose, the body sags, and both recover within about a second

#### Scenario: Blow between ticks
- **WHEN** a blow is sent in a frame that runs no tick
- **THEN** it waits in the queue and the next tick deals it

#### Scenario: Zombie energy
- **WHEN** a fully feral body (most energy 0.9) spawns at energy 1, and later is dazed to 0.5
- **THEN** its first tick leaves it at 0.9, and it then recovers 0.005 a tick back to 0.9

### Requirement: Balance
Each tick a living body SHALL score its lean: q = u_y⁸ standing, u_y⁴ moving, plus 0.05 × the race's balance bonus, at most 1, u being the unit offset from its support point to its chest. While q ≤ 0.99 an instability counter SHALL grow by 10 − 10q rounded half to even; else it falls by 5 to no less than 80 standing still, or by 10 to 0. At 100 balance SHALL be q; below, it recovers by (5 − v)·0.01 a tick at a slow chest speed v under 5 cm/tick.

#### Scenario: Standing still
- **WHEN** an alert body stands upright on the ground
- **THEN** it keeps its balance indefinitely: balance 1, its counter at 80 from its first tick

#### Scenario: Lean
- **WHEN** a human's chest leans 44 cm off its support point, 130 cm above it
- **THEN** it scores 0.698, and under 0.6 only past 52.2 cm

#### Scenario: Second wobble
- **WHEN** a lean scoring 0.55 comes to a fresh body, and to one that has stood steady
- **THEN** the fresh body loses its balance after 25 ticks, the other after 5

#### Scenario: Rounding
- **WHEN** the score is 0.95, 0.75 or 0.55
- **THEN** the counter grows by 0, 2 or 4

### Requirement: Falling and getting up
In its posture stage, from last tick's balance, a living body under 0.6 (double) and not lying still SHALL steady itself upright (lean height over 0.8): the Defence stance (blend 40) still, Rebalance (20) moving; else stagger (below) or fall: Fall Forward when the lean is ahead, else Fall Backward, no blend, when it is sideways (|lean · forward| ≤ 0.4), fast (slow speed² ≥ 8), lost (≤ 0.1) or 80 cm off. Starts are eased fast; a fall leaves the controller.

#### Scenario: Knocked over
- **WHEN** `exarust --shove 60 play arena` deals the player a body blow
- **THEN** the player falls onto its back, lies a moment, gets up with `Get Up 2` without a hop, and stands where it fell

#### Scenario: From the lying body
- **WHEN** a get-up starts with a part turned 0.4 rad off its clip's pose
- **THEN** after its first tick that part's target turns as the part does

#### Scenario: Leans
- **WHEN** a body at balance 0.45 leans 30° ahead still, then 45° ahead, 45° back, 50° to its side, and 45° ahead with a slow speed² of 8
- **THEN** it takes the Defence stance, staggers with Get up, staggers with Get Up 2, falls backward, and falls forward

#### Scenario: Float thresholds
- **WHEN** the lean's height is the float 0.8, or its forward share the float 0.4 with the balance the float 0.1
- **THEN** the body steadies itself upright, or staggers, as the original's double compares give

#### Scenario: Dazed while falling
- **WHEN** a dazed body falls with its balance between 0.3 and 0.6
- **THEN** its fall's or Rebalance's clip starts again every tick after the daze's Defence stance, until it is down

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
The system SHALL sleep a limp body by the original's countdown: 256 when woken, then each tick, by its fastest part's last motion, back to 256 over 40 cm/s, held at 30–32 between 20 and 40 cm/s, two off under 20 cm/s; at 1 it sleeps. Its first tick asleep SHALL take its velocity away; later ticks do nothing. A living body SHALL never sleep. Blows and body contacts (the lower countdown takes the higher) SHALL wake a body without its contacts; going limp SHALL wake it; level contacts SHALL NOT.

#### Scenario: Corpse at rest
- **WHEN** a limp body's parts have moved under 20 cm/s in every tick for 128 ticks since it was woken
- **THEN** it falls asleep in that tick and is not simulated until a blow lands on it

#### Scenario: Woken corpse
- **WHEN** a sleeping corpse on the floor is shoved
- **THEN** it wakes with none of its earlier contacts and is tested afresh where it lies

#### Scenario: Creeping body
- **WHEN** a limp body's fastest part keeps moving between 20 and 40 cm/s
- **THEN** it never falls asleep

#### Scenario: Deep sleep
- **WHEN** a body has slept through one tick
- **THEN** it has no velocity and its particles stay exactly where they are in every later tick

#### Scenario: Living body
- **WHEN** a living body stands, lies fallen or gets up for any time
- **THEN** it stays awake, and it sleeps by its speed only once it goes limp

#### Scenario: Touched by a falling body
- **WHEN** a body falls onto a sleeping one
- **THEN** the sleeper wakes with the falling body's countdown and none of its level contacts

### Requirement: Ragdoll stability
The system SHALL keep bodies stable under blows: a particle SHALL carry at most 25 m/s (Exarust's guard; the original has none).

#### Scenario: Fallen far from the origin
- **WHEN** a living body 72 m from the world's origin falls and lies still while its pose keeps being aimed
- **THEN** no particle moves faster than 5 m/s

#### Scenario: Deep contact
- **WHEN** a body's sphere would end a tick deep behind a floor it crossed
- **THEN** it is taken back to where it first touched the floor, and no particle gains more than 25 m/s

### Requirement: Fallen muscles act inside the body
A fallen body SHALL keep its tone and, once its support is 0, have no pose drive: only its link muscles act, pulling its parts against each other, so nothing damps its bulk motion and it falls as fast as a limp body.

#### Scenario: A fallen body kicking its legs
- **WHEN** a fallen body lies on its back on a floor while its pose kicks its legs and swings its arms for six seconds
- **THEN** its centre of mass stays within 30 cm of where it lay

#### Scenario: Falling
- **WHEN** a fallen body and a limp one drop for half a second
- **THEN** the fallen one falls at least 0.8 times as fast

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
The system SHALL apply friction at level contacts from the part's own coefficient μ (limbs 2.5, feet 0.5 as built; a living body's lower legs and feet by their foot's state) and the contact normal n, with the limit 0.5·μ·(0.01 + 0.99·n_y²) in cm² per tick². A slide this tick whose square is under the limit is cancelled; a larger one is reduced by 0.01 times the limit of itself. Friction SHALL run in 8 passes after detection and in each of the next tick's 20 link iterations.

#### Scenario: Floor and wall
- **WHEN** a limb's contact point slides 1.1 cm in a tick on a floor, or 0.1 cm on a vertical wall
- **THEN** the slide is cancelled, while a slide of 1.2 cm on the floor or 0.2 cm on the wall is reduced by only 1.25 % or 0.0125 %

#### Scenario: Limp body at rest
- **WHEN** a limp body has dropped onto a floor
- **THEN** it does not creep, every sphere rests within 3 mm of touching the floor, and it falls asleep

### Requirement: World tick order
The system SHALL run each 83 Hz tick of the bodies as one world, in the original's order: a pre-step; every living body's character tick, the player's last, each body's whole tick before the next; the queued blows; the integration of every awake body; 20 iterations, each the pair pass then every body's links; body–body, then body–level detection; 8 friction passes, each the pairs' then the level's; the post-step of every body; then the per-character post-step.

#### Scenario: Character ticks first
- **WHEN** a tick runs with the player and other living bodies
- **THEN** every living body's targets are set before any body is integrated, the player's after every other body's

#### Scenario: One body alone
- **WHEN** a single body that touches no body part, its own or another's, ticks in the world
- **THEN** it ends the tick exactly where ticking it alone leaves it

#### Scenario: The tick's sets
- **WHEN** a tick runs
- **THEN** the pre-step runs first, then the character ticks, the solver and the per-character post-step

### Requirement: Clips sampled every tick
The system SHALL advance a living physical body's clip clock by the original's rate for what the body does, take a started clip's capture and sample its clip inside the body's character tick, not once a frame. A clip that ends into its follow-up SHALL start it, captured, in that tick. Bodies that are not physical SHALL keep playing their clips once a frame, at 60 frames a second.

#### Scenario: Clip clock at any frame rate
- **WHEN** a physical body stands in a clip for 60 ticks at 30, 60, 144 or 240 FPS
- **THEN** its clock advances 0.6 of a frame per tick, and its targets after each tick are the clip's pose at that tick on the body's frame

#### Scenario: Frames leave the clip alone
- **WHEN** frames pass on a physical body's clip
- **THEN** only its ticks advance the clock

### Requirement: Body trace
`exarust --trace <file.csv>` SHALL write a row per traced body (the player's, else every character body, no prop's) a tick: the retail probe's columns, then Exarust's (sleep, frame, root, blend, hold, rate, progress, end flag, ankles, condition; swing rate, strength, word; locomotion flags, moving, free walk, yaw, heading, turn rate, alignment, speed, reach, stop, goal x and z). Lengths cm, words hex; empty if unsimulated.

#### Scenario: Standing in the arena
- **WHEN** `exarust --trace t.csv play arena` runs for a minute
- **THEN** `t.csv` has one row per tick for the player's body, its clock advancing 0.6 of a frame per row, its blend and hold 0, its condition `0x0`, and its pelvis 1.7 cm over its target pelvis height

#### Scenario: A released punch
- **WHEN** `exarust --opponent --trace t.csv --press LMB@200 play arenao1` runs
- **THEN** the punch's rows show the rate 1.62, then P falling by 0.3 a row with the state word's stalling and released bits, and the stance from the punch's sixth row

#### Scenario: Planted feet
- **WHEN** the player stands in the arena
- **THEN** both foot contacts read `0x2`, their anchors 9.7 cm up, and its ankles within 0.5 cm of them

#### Scenario: The idle clock's columns
- **WHEN** a standing body's trace runs through its Idle clip's wrap
- **THEN** every row's `rate` is 0.6, its `progress` the clock over the last key, and `clip_end` is 1 on the wrap's row alone

#### Scenario: Walking and stopping
- **WHEN** `exarust --trace t.csv --press W@100-250 play arena` runs
- **THEN** the rows show `moving` with flags `0x40` at 1.687 cm a tick, then the soft stop's `0x70` and the stop reason `braked`, then `standing`

### Requirement: Race body parameters
The system SHALL decode, for every record of the installed `races.rdb`, the body parameters the original reads: the clip-rate base, support-pin factor, size, scale, mass divisor, balance bonus, stance factor, arm relaxation, limb damping ratio, and four gait rows (walk, jog, sprint, backpedal) of thrust, push scale, brake distance (the loader's 0.5, 2, 2.5, 0.5 m), clip rate and boost speed. A missing race SHALL get the original's fallback: every factor 1.0 and gaits with no push.

#### Scenario: Human
- **WHEN** the installed `races.rdb` is parsed
- **THEN** every record decodes, and Human has support-pin factor 1.25, balance bonus 1.0, stance factor 0.5, arm relaxation 1.0 and damping ratio 1.0, with gait thrusts 88.9, 350, 472.2 and 77.8 per second and push scale 550 in every row

#### Scenario: Skeleton
- **WHEN** the installed `races.rdb` is parsed
- **THEN** Skeleton has stance factor 0.75 and arm relaxation 0.5

#### Scenario: Unknown race
- **WHEN** a character's race byte names no race in the database
- **THEN** its body parameters are the fallback, whose gait table cannot move it

#### Scenario: Clip-rate base
- **WHEN** the installed `races.rdb` is parsed
- **THEN** Ghoul and Ghoul H have a clip-rate base of 1.5 and every other race 1.0

#### Scenario: Large races
- **WHEN** the installed `races.rdb` is parsed
- **THEN** Ogre has size 1.5 and scale 1.5, the harvester 1.8 and 1.8, Fael 1.5 and 1.35, Golem Guard 1.4 and 1.2, and Sir, Fael and the demon a mass divisor of 1.1

#### Scenario: Brake distances
- **WHEN** any race's gait rows are read, the fallback's included
- **THEN** a walking body brakes 0.5 m from its goal, jogging 2 m, sprinting 2.5 m and backpedalling 0.5 m

### Requirement: Joints at the part origins
The system SHALL join every part that has a parent part by its skeleton links and then a pivot from its origin to its place in the parent, the model's local translation of the part. A link's parent point SHALL be the stored point plus that translation, unturned. A correction SHALL be shared m/(m + m_parent), m being 0.175 times the bone's mobility, and spread over each part's particles by the point's weights, unclamped. A joint's range SHALL come from its links alone.

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

#### Scenario: Elbow and knee ranges
- **WHEN** the installed `humanoid.rsq` links of each forearm and lower leg are turned about their hinge axes
- **THEN** each elbow bends from straight to 120° and not past straight, where 10° past straight stretches its long link 1.8 cm beyond its slack, and each knee bends from 10° to 130°

### Requirement: Living joint corrections
In a living body a pivot (slack at most 0.01 cm) SHALL close d/(1 + 0.001|d|) of its gap d in cm, in full however far apart, and a slack link SHALL take back its excess beyond the slack at full weight, cut to 80 cm.

#### Scenario: Pivot pass
- **WHEN** a living pivot's two points are 10 cm apart
- **THEN** one pass closes 10/1.01 cm of the gap, shared by the two parts

#### Scenario: Torn limit
- **WHEN** a living slack link is stretched 3 m beyond its slack
- **THEN** one pass takes back 80 cm

### Requirement: Link strength and smoothing
Each link SHALL keep a strength: 1, except a lower leg's first link with over 10 cm of slack, its leg limit, at 0.2 when a living body is built, then following its leg's gait, and 1 once the body goes limp; the femur's first such link SHALL be its leg limit. Each link SHALL keep the smoothing (1 − (r² − 25)·0.001)², at most 0.99 and 0 within √0.1 cm, r being its point's distance from the part's origin in cm.

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

### Requirement: Link muscles
In a living body each slack link SHALL pull its point toward its anchor, where the clip's joint angle puts it on the parent: e = anchor − point, plus the gap's change over the tick × D·k/(0.5|e|² + 1.4), D = (E − 0.5)² (0 below E = 0.5), k = 50 moving, else 40 / stance factor, ×0.6 on the pelvis and legs; then e/(1 + c|e|), c = 0.1, in cm; times strength × part factor × √E·T, the excess beyond the slack added at full weight.

#### Scenario: Elbow swing
- **WHEN** a living arm's clip swings the elbow 40° over 10 ticks and holds it
- **THEN** the elbow is within 5° of the clip 20 ticks after the swing ends, without overshooting it

#### Scenario: Idle and walking damping
- **WHEN** an alert human stands
- **THEN** its link muscles damp with gain 14.3 (8.6 on the pelvis and legs) and pull at √E·T = 0.5, and walking they damp with gain 8.9

#### Scenario: Dazed
- **WHEN** a body's energy falls to a quarter
- **THEN** its link muscles pull at half strength and are not damped

#### Scenario: Momentum
- **WHEN** one pass of a link muscle moves a part and its parent
- **THEN** the body's momentum is unchanged

### Requirement: Muscle gains
Each tick the character tick SHALL set: the stance factor s, the race's standing (human 0.5; 1 dormant or in the shock's stagger), rising 0.01 a tick (double) toward 0.7 + 0.3·run (last tick's run blend) while the decision walks, runs or backpedals it, never past it, keeping what it reached above it; T = tone·s; part factors 0.5, a foot half its lower leg's, the head a quarter of the thorax's × its head factor, shoulders, arms and hands 1.25 × scale × arm stiffness, and the legs' gains.

#### Scenario: Walking firms the body
- **WHEN** a standing human starts walking
- **THEN** its stance factor rises from 0.5 to 0.7 in 20 ticks, reaches 1 running, keeps it while it moves on, and is 0.5 again when it stops

#### Scenario: The rise stops on its target
- **WHEN** a walking body's stance factor is 0.695, then 0.845, at a run blend of 0.5
- **THEN** it is 0.705, then 0.85, a tick later

#### Scenario: Dormant
- **WHEN** a standing body is dormant
- **THEN** its stance factor is 1

#### Scenario: Leg limit
- **WHEN** a living human stands for one tick
- **THEN** its leg limit links pull at 0.23, settling at 0.5, at 0.4 while it walks or runs and toward 0 without support

#### Scenario: Feral heads and short limbs
- **WHEN** a fully feral and a half-feral human are built, and a short human (scale 0.95) falls
- **THEN** their head factors are 0.025 and 0.075, and the short human's arms pull with σ = 0.297 off balance

### Requirement: Leg gains
Each tick the character tick SHALL set the legs' gains by the movement decision's leg mode: a thigh's factor 0.2 walking, 0.6 running, 0.25 × scale standing or backpedalling (in double precision); each leg limit link's strength s ← 0.9s + 0.1t (double precision), t = 0.4 walking or running, 0.5·S² standing or backpedalling. A collapsing body's thighs and leg limits SHALL keep their gains.

#### Scenario: Running thighs
- **WHEN** a human runs, then backpedals
- **THEN** its thighs pull at 0.6, then at 0.25

#### Scenario: Collapsing
- **WHEN** a collapsing body's leg mode is the run
- **THEN** its thighs and leg limits keep the gains they had

### Requirement: Arm stiffness
The arms' stiffness SHALL be 0.2 while the body collapses, 0.5 off balance (balance under 0.6, falling, lying or getting up), 0.2 at tone under 0.85, the character's walking arms (0.2 unless feral) in the walk and run cycles, and otherwise x ← 0.8x + 0.08·(0.5 + 0.5·race arm relaxation), never under min(0.4·|turn left in rad|, 0.4). An arm part's anchor relaxation SHALL be capped at 0.6 − (x − 0.1)·0.8.

#### Scenario: Idle human arms
- **WHEN** an alert human stands still
- **THEN** its arms settle at stiffness 0.4 and pull with σ = 0.25, their relaxation capped at 0.36

#### Scenario: Walking and off balance
- **WHEN** the human walks, and later falls
- **THEN** its arms pull with σ = 0.175 walking and 0.3125 off balance

#### Scenario: Zombie arms
- **WHEN** a fully feral human walks, and is then dazed to tone 0.8
- **THEN** its arms' stiffness is 0.05 walking and 0.2 dazed

#### Scenario: Collapsing arms
- **WHEN** a body collapses, off balance, at tone 0.6
- **THEN** its arms' stiffness is 0.2

### Requirement: Anchor relaxation
Each tick a part with link muscles SHALL move its relaxation λ toward 0.6 − 32|Δa|² (cm², the most one of its anchors moved since the last tick) × the race's arm relaxation, capped by the part: up by 0.015 a tick, down at once, and not at all for a negative target. A supported lower leg and its thigh SHALL instead take clamp(0.6 − 0.001·| |hip − foot contact|² − (2r)² |, 0, 0.2), r the contact's (the ankle's) distance from the knee, rising 0.01.

#### Scenario: Held pose
- **WHEN** an idle arm's clip holds still for 40 ticks
- **THEN** its relaxation rises to its cap, 0.36

#### Scenario: Fast and slow motion
- **WHEN** an arm's anchor then moves 0.84 cm in a tick, and next 0.1 cm
- **THEN** its relaxation stays at 0.36, then falls to 0.28

#### Scenario: Standing leg
- **WHEN** a human stands with its hip 85 cm from its ankle, 42 cm from the knee
- **THEN** its leg's relaxation is 0.2

### Requirement: Balance point
Each tick a living body SHALL take its chest, the mean of its thorax pins, and smooth the chest's last motion by 0.4 (fast) and 0.1 (slow). The support point SHALL be the closest point to the chest on the segment between its foot contacts, the ankles (parameter taken horizontally), moved halfway to their centre standing. A landing SHALL lead a standing chest by the slow velocity × 5, fading 0.05 a tick; moving with support, by 3 × the fast velocity; a crouch takes it 10 cm back.

#### Scenario: Chest and ankles
- **WHEN** a human body is built from the installed skeleton and model
- **THEN** its chest is the mean of its shoulder pins, (0, 9.65, 1.25) cm in the thorax, and each leg stands on its foot's origin, 41.95 cm under the knee, its leg 85.15 cm long

#### Scenario: Landing
- **WHEN** a body regains the ground
- **THEN** its landing weight is 5, and 4.95 the next tick

#### Scenario: Smoothed chest
- **WHEN** a standing body's chest moves 1 cm in a tick
- **THEN** a tick later its fast velocity is 0.4 cm and its slow velocity 0.1 cm a tick

### Requirement: Vertical chest speed
A slow vertical chest speed v over 3 cm/tick SHALL set the instability counter to 100 and, for a body grounded last tick, cut its score by (1 − 0.333(v − 3))², to 0 above 6 cm/tick.

#### Scenario: Sinking
- **WHEN** a standing body sinks 5 cm a tick
- **THEN** on the tenth tick its slow vertical speed passes 3 cm/tick, its counter is 100 and its balance the cut score

### Requirement: Height Query
The level SHALL answer the original's Height Query: a sphere whose lowest point starts at a given point is swept straight down at most a given depth, and the answer is the height of its lowest point where it first meets the front of an upward face, or where it starts if it starts against one. Living bodies are not part of the level.

#### Scenario: Floor and step
- **WHEN** a 10 cm sphere drops from 70 cm onto a floor, and beside a 30 cm step whose edge is 5 cm from its centre line
- **THEN** it finds the floor, and 28.66 cm on the step's edge

#### Scenario: Faces turned away
- **WHEN** a face turned down lies over the floor, and when the sphere starts behind the floor
- **THEN** the first is passed through, and nothing is found

### Requirement: Support
Each tick each leg of a living body SHALL query the ground 5 cm ahead of its ankle from 70 cm above it (from it down or getting up) with a 10 cm sphere walking, else 5 cm + 10 cm × an anchored foot's share. Ground over half the leg under the pelvis is out of reach, but getting up. A foot is grounded within 20 cm (30 moving) of its ground plus its ground offset. S SHALL be grounded × balance; a standing body without a grounded foot loses its balance unless its controller stands on the ground.

#### Scenario: Standing in the arena
- **WHEN** `exarust --trace t.csv play arena` runs for a minute
- **THEN** every row has the player's support and balance at 1 and its counter at 80

#### Scenario: No ground
- **WHEN** a standing body has no ground under its feet
- **THEN** its balance and support are 0 and it counts the ticks it is airborne; a moving body keeps its balance

#### Scenario: Perched
- **WHEN** a standing body's controller stands on a stone its feet hang beside
- **THEN** it keeps its balance, with support 0

#### Scenario: Out of reach
- **WHEN** a 65 cm block stands under one foot of a standing human
- **THEN** that foot has no ground, the other is grounded, and its support is 1

#### Scenario: Getting up near the ground
- **WHEN** a body getting up has no foot grounded and its pelvis 20 cm over the ground
- **THEN** its support is half its balance, which rises 0.05 a tick

#### Scenario: Down
- **WHEN** a body lying or getting up has a shelf 40 cm over its ankle, and a 5 cm step's edge 7 cm beside its probe
- **THEN** its leg finds the floor under both, where a standing body finds the shelf and the floor, and a walking body the shelf and the step's edge (2.14 cm)

### Requirement: Joint measurements
The system SHALL measure each joint of a simulated body where the last tick left it: its turn from the model's rest pose relative to its parent, split into a twist about the part's bone axis and a swing of that axis (with the swing's axis); in a living body, the angle to the clip's turn; and each link's gap against its slack, the largest share of slack a slack link uses, and the pivot's gap. A joint in its rest pose SHALL measure 0.

#### Scenario: Rest pose
- **WHEN** a part sits at its rest turn relative to its parent
- **THEN** its total, swing and twist are 0

#### Scenario: Bend and twist
- **WHEN** a part's turn is a 30° twist about its bone followed by a 60° swing about an axis across it
- **THEN** its swing reads 60° about that axis and its twist 30°

#### Scenario: Link past its slack
- **WHEN** a thigh swings 60° at the hip with a link 30 cm down it and 5 cm of slack
- **THEN** that link reads 25 cm past its slack, 600 % of its range, and is the worst

### Requirement: Joint trace
`exarust --joint-trace <file.csv>` SHALL write, after every tick, one row per part of every simulated character body not in deep sleep (props' bodies never): the tick, body and bone; the joint's measures in degrees, cm and mm; the clip's turn split the same way; whether a blow landed on the part in the tick; the body's sleep state and countdown; its clip and clock.

#### Scenario: Duel
- **WHEN** `exarust --opponent --joint-trace j.csv play arena` runs for a minute
- **THEN** `j.csv` has 18 rows per tick per fighter, and the rows of the ticks a hit lands mark the struck part as shoved

### Requirement: Pose drive
After inertia each particle of a pose-driven part SHALL be pushed by G·o·(1 − c), only when c < 1, its vertical cut to a tenth. o runs from the particle to R·r + O: its rest vertex under the part's animated rotation, at the part's origin on its particles. c = o·v/(s|o|²), v its last displacement, s the stance factor. G = gain·t²·T·E⁴·(0.5T + 0.5)·S², t the tone, T = tone·s. Only with support.

#### Scenario: Closing particle
- **WHEN** a particle closes on its target at s|o| a tick or faster, and another moves away from it
- **THEN** the first is not pushed and the second is pushed harder than G·o

#### Scenario: Moving part
- **WHEN** a part turned a little off its pose moves at 1.6 cm a tick
- **THEN** its particles whose offsets point along the motion are not pushed, and those whose offsets point against it are pushed harder than G·o

#### Scenario: No support
- **WHEN** a body lies on its back on the floor, or stands on its controller with no ground under its feet
- **THEN** its pose drive moves no particle

#### Scenario: Chest shove
- **WHEN** a standing body is shoved at the chest at 1.5 m/s
- **THEN** its torso sways and turns back to its pose, its balance stays at 0.6 or more, and it comes to rest

### Requirement: Carry
After its inertia each particle of a living part over half its tone t SHALL move by g(1 − a) + carry·a, any other by g: a = t × its support share (pelvis and thighs 1, lower legs 0.5, feet 0.25) or t × E (the upper body). Each character tick SHALL set the carry to gravity, then, with support, take the lift g·f·S off it.

#### Scenario: Shares
- **WHEN** a part of tone 0.9 takes the carry at energy 0.6
- **THEN** a is 0.9 on the pelvis and thighs, 0.45 on the lower legs, 0.225 on the feet and 0.54 above the legs, and nothing at a tone of 0.5

#### Scenario: No support
- **WHEN** a body at full tone and energy has no ground under its feet
- **THEN** its carry is gravity and every particle falls at g

#### Scenario: Fallen
- **WHEN** a fallen body lies on the floor
- **THEN** its feet have no ground within reach and nothing lifts it

#### Scenario: Dazed
- **WHEN** a body hovering on its lift, its feet off the floor, is held at energy 1, 0.75 and 0.5
- **THEN** its pelvis settles 0.3 cm over its target, 1.2 cm and 3.0 cm under it, where its weight balances the lift

### Requirement: Lift
With support the lift SHALL be f = 0.075(h + 15), at most 1.5, standing (a negative f lifts nothing, unless crouched past 0.9: down to −10), or f = (0.4·run + 0.2)(h + 5), at most 2, moving (none when negative); h = target − pelvis height, in cm. Then f −= (1 − |f|/(|f| + 2))·E²·v, v the pelvis's vertical displacement over the last tick, and f ×= 2(t − 0.5) over a tone of a half.

#### Scenario: Balance point
- **WHEN** an alert body stands on its legs, its pelvis 1.7 cm over its target
- **THEN** f is 1 and it stays there; set down 1 cm into the floor it rises less than 2 cm over where it comes to rest

#### Scenario: Gains
- **WHEN** the pelvis is 5 cm under its target standing, at the target walking, 3.3 cm over it running
- **THEN** f is 1.5, 1 and 1; falling 1 cm a tick at f = 1 adds 2/3 × E²

#### Scenario: Tone
- **WHEN** a standing body's tone is 0.75, then 0.5
- **THEN** its lift is half, then nothing

#### Scenario: Get-up
- **WHEN** `exarust --trace t.csv --shove 60 play arena` gets the player up with `Get Up 2` into Idle
- **THEN** its pelvis rises to its standing height, overshoots it by less than 6 cm (10.7 cm under the old vertical hold) and is back within a centimetre in under 100 ticks

### Requirement: Target pelvis height
Each leg's height SHALL be its ground + crouch·60 + (1 − crouch)·(the clip root's height over its animated ankle + its ground offset: the sole height, moving also the ankle's lift over the walk's lowest). The target SHALL be the lowest foot's, blended by half toward the other's while that one's contact height over its ground plus the lowest's is under 40 cm (less up to 60); standing, capped at ground + sole + leg length, plus 5 cm.

#### Scenario: Idle human
- **WHEN** a human's clip root is 91.4 cm over its ankles on flat ground, its sole 9.68 cm and its legs 85.15 cm
- **THEN** its target is capped at 99.83 cm and it stands with its pelvis 101.5 cm over the ground

#### Scenario: Step
- **WHEN** both feet stand on a 30 cm step, or the platform under a standing body is taken away
- **THEN** its target rises by 30 cm, or drops by 30 cm and its pelvis settles 30 cm lower without losing its balance

#### Scenario: One foot up
- **WHEN** one foot stands on a 30 cm step
- **THEN** standing the lower foot caps the target, and moving it is the mean of the two feet's, uncapped

#### Scenario: Blend
- **WHEN** the other foot's spread is 35, 55 or 60 cm, or it has no ground
- **THEN** it is blended in by 0.5, 0.125 or not at all, or not at all

#### Scenario: No ground
- **WHEN** no foot has ground
- **THEN** the target is −1000 m and nothing lifts the body

### Requirement: Climbing with the controller
The lift SHALL read each foot's own ground. The character controller's floor SHALL NOT raise that ground, on level ground, on stairs, perched or fallen.

#### Scenario: Level ground
- **WHEN** a body walks or stands with its controller floating 0, 2 or 5 cm over a level floor
- **THEN** its target is the original's every tick: in `--autowalk play arena` the same as without the rule, the pelvis 99.4–102.2 cm walking and 101.5 cm standing

#### Scenario: Stairs
- **WHEN** a body walks up four 18 cm steps with its controller, stopping on the third
- **THEN** it keeps its balance at or above 0.9 and its feet end on the landing, and the controller's floor does not raise the pelvis

#### Scenario: Stair room
- **WHEN** `exarust --autowalk --at 7.5,-0.95,64.4 play exanima01` or `--at 7.6,-1.3,66.2` walks the player up the stairs to the door
- **THEN** the four-step synthetic walk is this change's stair gate; a fall on that exanima01 walk is the missing gait probe and is recorded, and the controller floor is not restored

#### Scenario: Perched
- **WHEN** a body's grounded controller stands 12 cm over the floor under its feet, or is in the air
- **THEN** its target is the original's in both cases

#### Scenario: Fallen
- **WHEN** a body has just fallen, its feet on the floor and its grounded controller 12 cm over it
- **THEN** its target is the original's every tick, as with its controller on the floor

### Requirement: Leg bend
A leg SHALL bend onto its bend target. The knee SHALL come from the law of cosines on the two bones: 30° when the cosine is above 0.866, and 180° when it is below −1. The correction from the clip SHALL advance at most 0.02 rad a tick (0.2 while stepping) plus its stored rate, never past the target. The rate SHALL become half that sum, within ±0.1 rad. Muscles SHALL follow the correction on the next tick, not the tick that computed it.

#### Scenario: Folded to 40 cm
- **WHEN** both bones are 40 cm and the hip-to-target distance is 40 cm
- **THEN** the unsmoothed knee angle is 60°

#### Scenario: Folded too sharp
- **WHEN** both bones are 40 cm and the hip-to-target distance is 20 cm
- **THEN** the unsmoothed knee angle is 30°

#### Scenario: Next tick
- **WHEN** the stored correction and its rate are 0 and the new correction is past 0.02 rad
- **THEN** the pose the muscles follow on that tick is unchanged, and the next tick's correction is 0.02 rad

### Requirement: Thigh turn
While a leg is stepping, its thigh SHALL turn by 75% of the angle from its rest direction to the direction from the hip to the bend target, taken in the character frame. While it is not stepping, that extra turn SHALL lose 10% a tick.

#### Scenario: Three quarters of the way
- **WHEN** that angle is 40° and the leg is stepping
- **THEN** the thigh's extra turn is 30°

#### Scenario: Decay
- **WHEN** the leg is not stepping and the extra turn is 30°
- **THEN** the next tick it is 27°

### Requirement: Step lift
An anchored stepping foot SHALL lift by −(1 − crouch) · f · (1.5 − the stance factor). s SHALL be 0.4 times the horizontal distance to the anchor and no more than 30 cm. r SHALL be D²/(3s)², D the distance to the anchor. f SHALL be ((2 − r)·r·3 − 2)·s when D is at most 3s, and 0.5·s otherwise. An unanchored foot's lift SHALL become 0.8 of itself each tick, and an anchored foot that is not stepping 0.9 of itself.

#### Scenario: A human step of 40 cm
- **WHEN** the horizontal distance and D are both 40 cm, the crouch is 0 and the stance factor is 0.5
- **THEN** the lift is −11.5185 cm

#### Scenario: Decay of a free foot
- **WHEN** an unanchored foot's lift is 10 cm
- **THEN** the next tick it is 8 cm

### Requirement: Bend target
An anchored foot's bend target SHALL be the midpoint of the foot and its anchor, at the anchor's height. An unanchored foot's bend target SHALL be its anchor, lowered by twice the lift term stored before clamping. Standing at the target height, not stepping, that term is 1.125 cm, so the drop is 2.25 cm.

#### Scenario: Anchored midpoint
- **WHEN** the foot is 20 cm from its anchor and the foot is anchored
- **THEN** the bend target is 10 cm from each, at the anchor's height

#### Scenario: Unanchored drop
- **WHEN** the foot is not anchored and the body stands at its target height, not stepping
- **THEN** the bend target is the anchor lowered by 2.25 cm

### Requirement: Feral sprint gait
The sprint gait of a character SHALL be its race's sprint row when its feral level is 0, and otherwise every column of that row moved linearly toward the race's jog row by the feral level.

#### Scenario: Half feral human
- **WHEN** a human of feral level 0.5 sprints
- **THEN** its sprint thrust is 411.1 per second, its clip rate 15.05 frames per second and its brake distance 2.25 m

#### Scenario: Zombie
- **WHEN** a human of feral level 1.0 sprints
- **THEN** it uses the jog row: thrust 350 per second

### Requirement: Character frame
A living body's targets SHALL be its clip pose on a frame that turns with the body, its origin the pelvis less the pose's root offset across the ground, at the pelvis's height. Each character tick it SHALL follow the pelvis: moving, onto the pelvis; without support, exactly; standing, past 5 cm while support is under 0.95, else not at all; down (balance under 0.3), past 5 cm; getting up, past 10 cm in the clip's first quarter, then not at all.

#### Scenario: Steady stance
- **WHEN** a standing body at support 1 is pushed 9 cm across the floor
- **THEN** its frame stays put and it comes to rest within 1 cm of its spot

#### Scenario: Unsteady body
- **WHEN** a standing body at support 0.95 is pushed 14 cm across the floor
- **THEN** its frame follows it, and it comes to rest no more than 5 cm back from the farthest it went, within 2 cm of where its frame puts its pelvis

#### Scenario: Support threshold
- **WHEN** a standing body's support is 0.95 as a float, which is under the double 0.95, or the next float up
- **THEN** its frame follows past 5 cm, or stays put

#### Scenario: Moving
- **WHEN** a body moves
- **THEN** its frame's origin is its pelvis origin every tick

#### Scenario: Getting up
- **WHEN** a get-up's pose puts the pelvis 30 cm off the frame's origin in the clip's first quarter
- **THEN** the frame moves under the body at once; later the pelvis 8 cm off leaves it, 12 cm moves it, and past the first quarter it stays

#### Scenario: Standing in the arena
- **WHEN** `exarust --trace t.csv play arena` stands the player still
- **THEN** its frame does not move while its support is 0.95 or more

#### Scenario: Far from the origin
- **WHEN** a steady standing body 90 m from the world's origin stands for 600 ticks
- **THEN** its frame does not move at all

### Requirement: Controller follows the frame
A living character's controller SHALL follow its body's frame (Exarust's own): each fixed step, before its own walk, it moves its feet a tenth of the way across the ground to where the frame's origin puts the body root, at most 2 cm, through the controller. The body's carry SHALL leave that move out.

#### Scenario: Glide
- **WHEN** the frame's origin is 50 cm from the feet
- **THEN** the feet go 2 cm a step until it is 20 cm away, then a tenth of the rest, and stop within a millimetre of it

#### Scenario: Follow left out of the carry
- **WHEN** a standing body's root moves 30 cm across the ground, by the controller's follow or by its walk
- **THEN** the body and its frame stay where they are, or are carried 30 cm

#### Scenario: Shoved over
- **WHEN** `exarust --trace t.csv --shove 60 play arena` knocks the player over and it gets up
- **THEN** its controller follows the falling body, never moves more than 2 cm a tick, and comes to stand at the frame's origin

### Requirement: Pose hold
A living body's sampled turns SHALL be pulled toward the pose it holds (its pelvis's and legs' targets, its other parts' physical turns, taken while it is dazed, its blow sag on or its support under 1) by w = 1 − (1 − S)²·(1 − s)·(E/Emax)⁴ while dazed (energy more than 0.05 under its most, in double precision), else (1 − S)·s, when w is over 1.19e-7. The sag SHALL fade 0.01 a tick and be 0 once the body falls.

#### Scenario: Dazed on its feet
- **WHEN** a body standing with support 1 is dazed to half its energy
- **THEN** its targets are its physical pose until its energy is back over 0.95

#### Scenario: Dazed lying
- **WHEN** a lying body (support 0) has half its energy
- **THEN** its turns are pulled 0.9375 of the way to its physical pose

#### Scenario: Alert
- **WHEN** an alert body has no blow sag
- **THEN** its targets are its clip's pose

#### Scenario: Threshold
- **WHEN** a body's energy is 0.95 as a float, a hair under the double 0.95, or the next float up
- **THEN** it is dazed, or it is not

#### Scenario: Feral threshold
- **WHEN** a fully feral body (most energy 0.9) has energy 0.85 as a float, or the float under it, and lies with energy 0.45
- **THEN** it is not dazed, or it is, and lying its turns are pulled 0.9375 of the way to its physical pose

### Requirement: Drawing sleeping bodies
The system SHALL draw only awake bodies between their last two ticks: a body in deep sleep SHALL keep the transforms it was last drawn with, and a body registered asleep and never simulated SHALL keep its nodes' transforms.

#### Scenario: Registered asleep
- **WHEN** a prop body has slept since it was registered for 60 frames
- **THEN** its node's transform is the one the level gave it

#### Scenario: Falls asleep
- **WHEN** a limp body falls into deep sleep
- **THEN** it is drawn once where its last tick left it and not redrawn until it wakes

### Requirement: Collapsing body
While a living body collapses its frame SHALL follow its pelvis exactly every tick, its lift SHALL be multiplied by its tone squared (beside 2(t − 0.5)), its feet SHALL be probed as standing, its support pins' gain SHALL not recover, and its clip SHALL run at 0.6 of the base rate a tick. A living body's pins' gain SHALL start at 0.08 and recover 0.001 a tick while under the double 0.08, settling at 0.081.

#### Scenario: Lift
- **WHEN** a standing body's lift at full tone is f, and it collapses at tone 0.9 and 0.75
- **THEN** its lift is 0.648·f, then 0.28125·f, and none at tone 0.5

#### Scenario: Frame
- **WHEN** a collapsing body's pelvis moves
- **THEN** its frame's origin follows it the same tick, at any balance and support

### Requirement: Character tick stages
Each living body's character tick SHALL run nine stages in the original's order (upkeep, readiness, anchor, behaviour, posture, attack, movement, sampler, carry), finishing them before the next body's starts. Its work SHALL sit in them: regeneration (upkeep); condition, dying tick, falls, get-ups, fall check (posture); swing, health (attack); movement decision and turn, balance, ground (movement); the clip (sampler); targets, lift (carry). A body dying in its posture stage SHALL skip the rest.

#### Scenario: One character after another
- **WHEN** two characters tick
- **THEN** the first runs all nine stages, in order, before the second runs its first

#### Scenario: Who ticks
- **WHEN** a tick runs with the player, a walking character with a controller, a standing body, a limp body and a body whose clips play by frames
- **THEN** the walking character and the standing body tick with their controllers (none for the standing body), the player last, and the limp body and the frame-played body do not tick

#### Scenario: Killed in its posture stage
- **WHEN** a collapsing body's tone reaches 0.25 in its posture stage beside a standing body
- **THEN** it is dead and limp at once and runs no attack, movement, sampler or carry stage, while the standing body runs them all

### Requirement: Solver stages
The solver SHALL offer stages for added work at the original's points of its sector step: after integration; in each of the 20 iterations (numbered 1 to 20) before every body's links and after them; after the iterations, before the level contacts are dropped; then the sim objects, body–body detection and sim-object contacts, before level detection; at the start of each of the 8 friction passes; after the post-step. A stage with no work SHALL change nothing.

#### Scenario: Stage order
- **WHEN** a recording stage is solved with a body resting on the floor and a body pulled apart at a joint
- **THEN** it is called after integration, then 20 times at the pair pass and after the links (the pulled body moving between them), after the iterations with the level contacts still held, at the sim objects, detection and sim-object contacts with none held, 8 times at friction with this tick's contacts, then after the post-step

#### Scenario: No work, no change
- **WHEN** a body ticks 120 times with a stage that has no work
- **THEN** its particles are exactly where the solver without stages leaves them

### Requirement: Contacts between bodies
The system SHALL keep body parts out of each other, for every body: after the iterations each awake part meets the parts near it, awake or asleep, that it has not met this tick. The part with the smaller key round(100/mobility) brings its spheres against the other's, each where it is, or swept from the last tick when it moved over 0.1 cm² relative to the other part. Each hit pushes both at once; a pair keeps up to four points, spread widest over the sphere side.

#### Scenario: Two limbs meet
- **WHEN** two limbs with 10 cm spheres are thrown at each other at 2 m/s each
- **THEN** they stop at their spheres in the tick they meet, 0.6 cm in, and part again; without the contacts they pass through each other

#### Scenario: A cup on a table
- **WHEN** a light body (mobility 10, an 8 cm sphere) drops onto a heavy sleeping one (mobility 0.05, 6 cm spheres) lying on the floor
- **THEN** it wakes the heavy one, and both sleep again with the light one resting on the heavy one's spheres, 14 cm from the nearest centre

#### Scenario: Two sleepers
- **WHEN** two sleeping bodies lie against each other
- **THEN** no contact is looked for and both stay asleep

#### Scenario: Which side brings its spheres
- **WHEN** a human's pelvis (mobility 0.3) and hand (mobility 1.67) meet
- **THEN** their keys are 333 and 60, and the hand brings its spheres

### Requirement: Groups within a body
Two parts of one body SHALL skip each other when their group masks share a bit. From each root, with a bit counter from 0 wrapping at 32, a part with children SHALL take the next bit, its parent's bit and the two bits its parent was given, and a leaf its parent's bit; each child then SHALL also take its parent's two given bits. A character's two feet SHALL share the union of their masks.

#### Scenario: A human's own parts
- **WHEN** a human body of 18 parts is built
- **THEN** 13 bits are given; legs, pelvis, spine and head never meet each other; the lower body meets the humeri, forearms and hands but not the shoulders; the lumbar meets the forearms and hands, the thorax and head the hands; a shoulder, humerus or forearm meets only the other hand; the two humeri and the two forearms skip each other; the hands meet

### Requirement: Layers between bodies
Parts of two bodies SHALL skip each other when, in some channel k from 2 to 15 (membership bit 2k, exclusive bit 2k + 1), both are members and either is exclusive. A character's lower legs SHALL be in channel 4, exclusive (0x300), while it lives and in none once it collapses or goes limp; its feet in channel 2, exclusive (0x30). A character's feet SHALL never meet other parts.

#### Scenario: Channels
- **WHEN** layer masks are compared
- **THEN** 0x300 and 0x300 skip each other, 0x300 and 0 meet, 0x30 and 0x10 skip, 0x10 and 0x10 meet, 0x40 and 0xC0 skip, and bits 0 to 3 make no channel

#### Scenario: Living lower legs
- **WHEN** two living characters' lower legs and feet overlap
- **THEN** no contact is found; once one collapses or goes limp its lower legs meet the other's, and the feet still never do

### Requirement: Immediate push of a body contact
Each hit SHALL push its two parts apart at once. Each side's contact weights, negatives dropped and normalised to sum 1, give φ = 1/(largest weight). With f = 2φ_A/(φ_A + φ_B), the sphere side SHALL move by its mobility share m_A/(m_A + m_B) × (2 − f) of the push and the other side back by its share × f, each spread so its largest weight takes it whole. Each push SHALL count toward the part's 0.1 cm edge restore.

#### Scenario: A point against a centroid
- **WHEN** a hit on one particle of a pelvis meets a hand's centroid
- **THEN** f is 0.4: the pelvis's particle takes its share × 1.6 of the push, each of the hand's particles its share × 0.4 back, and the two parts' momentum is unchanged

### Requirement: Pair pass
In each of the next tick's 20 iterations, before any body's links, each kept point deeper than 0.01 cm along its normal SHALL be taken back to 0.01 cm, less the sphere side's travel relative to the other this tick across the normal: with L = (0.01 + 0.99·n_y²)·μ_A·μ_B in cm² a tick², a travel whose square is at most L SHALL lose 0.01·L of itself and a larger one all of it. A pair needing nothing SHALL be dropped for the rest of the tick.

#### Scenario: Reversed friction
- **WHEN** two limbs side by side (μ 2.5 each) keep a point 0.51 cm deep
- **THEN** it is taken back 0.5 cm by their shares; a travel of 0.2 cm across loses 0.000625 of itself and one of 0.3 cm is cancelled; with one limb on the other (n_y 1) 2.4 cm is kept and 2.6 cm cancelled

#### Scenario: Within the slop
- **WHEN** a kept point is 0.009 cm deep
- **THEN** nothing moves

#### Scenario: Dropped once satisfied
- **WHEN** a pair's parts have moved apart before a pass
- **THEN** the pair is dropped, and pushed back into each other later in the tick its parts are not corrected

### Requirement: Pair friction
In each of the 8 friction passes, before the level friction, each point of the pairs found this tick SHALL have its slip across its normal, relative to the other side this tick, cancelled under L = (0.01 + 0.99·n_y²)·μ_A·μ_B and kept less 0.01·L of itself over it, each side moving toward the other's slip by its mobility share over its weights. Pairs whose μ_A·μ_B is under 1.19·10⁻⁷ SHALL NOT rub.

#### Scenario: Stick and slide
- **WHEN** a limb slipped 0.2 cm across a pair's point with another limb side by side
- **THEN** the slip is cancelled; a slip of 0.3 cm loses only 0.000625 of itself

#### Scenario: Almost no friction
- **WHEN** a limb lying on another with a friction product of 1.1·10⁻⁷ slipped 3·10⁻⁴ cm across their point
- **THEN** the slip is kept; with a product of 1.2·10⁻⁷ it is cancelled

### Requirement: Pair trace
`exarust --pair-trace <file.csv>` SHALL write, after every tick, one row per pair of body parts its body–body detection found in contact: the tick; each side's body, bone and whether it lives, the side that brought its spheres first; whether the two are one body; the hits; the pushes summed and the longest, and the mean touched point, in cm.

#### Scenario: A row
- **WHEN** a contact between a living hand and a corpse's pelvis is written
- **THEN** its row names both bodies and bones, marks them as two bodies, and gives the pushes and the point in cm

### Requirement: Ground under an unsupported body
Each tick a living body whose feet give it no support SHALL ask the level for the ground under its pelvis: a 20 cm sphere from 20 cm above it, swept 2,000 cm down, the ground 100 m down when it meets nothing. A supported body SHALL not ask. A body getting up within 30 cm of that ground has support.

#### Scenario: Depths
- **WHEN** a body off its feet hangs with its pelvis 1.03 m over floors 15 m, 18.5 m and 19 m below where it stood
- **THEN** it finds the first two and, for the third, ground 100 m down

#### Scenario: Standing
- **WHEN** a body stands on the floor
- **THEN** it has no ground under the pelvis measured

### Requirement: Staggers
A stagger SHALL start Get up (lean ahead) or Get Up 2 with a blend of 50 from the whole physical pose and write getting up; when the clip changes its clock SHALL be set to the key before Get up's fourth key (frame 55) or Get Up 2's third (40). Its first sample then drops the blend and the body SHALL hold the clip's first key until its clock reaches it.

#### Scenario: Get Up 2 jumps in
- **WHEN** a body staggers back at full tone
- **THEN** Get Up 2's clock reads −9.25 after its first tick (0.75 frames on it), its first key holds for 27 ticks, and its clip ends into the stance 144 ticks after the start

### Requirement: Getting up from the floor
A body lying still (balance under 0.3, limb motion under 25 cm a tick) SHALL get up once a foot gives support or 100 ticks after support was lost: Get up when its pelvis's front axis height is at most 0 (0.2 while Get up plays, −0.2 while Get Up 2 does), else Get Up 2; past a quarter of the clip an unchanged pick does nothing. It SHALL turn to pelvis → chest (reversed for Get Up 2) when over 18.2° off, clear its feet's states and start with a blend of 20.

#### Scenario: Waiting
- **WHEN** a body lies still without support for 99 ticks, then 100
- **THEN** it waits, then gets up

#### Scenario: Face up
- **WHEN** a supported body lies still on its back
- **THEN** Get Up 2 starts with a blend of 20 from its whole pose, its condition getting up, on its controller

#### Scenario: Rolling over
- **WHEN** a body 30 % through Get up rolls onto its back, or one 40 % through Get Up 2 rolls onto its front
- **THEN** the first changes to Get Up 2 and re-anchors its frame; the second keeps its clip

### Requirement: Rising energy
While getting up a body's energy SHALL gain 0.01 × (1 − progress) a tick beyond its recovery (by last tick's condition and progress), and its balance 0.05 a tick.

#### Scenario: Halfway
- **WHEN** a body is halfway through Get up at energy 0.5
- **THEN** its readiness stage brings it to 0.505 before the tick's own recovery

### Requirement: Steadying on the feet
A standing body at full tone playing a plain clip (no attack, combat idle, walk, run, kick, weapon switch or power) SHALL take the Defence stance (no blend) under balance 0.8 (double) with a fast chest speed² of at most 0.3, else Rebalance (blend 20); weak (tone under 0.85), the Defence stance. A Rebalance SHALL hold the posture switch off for 41 ticks off balance, 27 unsteady.

#### Scenario: Unsteady
- **WHEN** a standing body has balance 0.79 and a fast chest speed² of 0.29, then 0.31, then balance the float 0.8
- **THEN** it takes the Defence stance, then Rebalance, then neither

#### Scenario: Shoved
- **WHEN** `exarust --fixed-frames --shove 60 play arena` dazes the player
- **THEN** it Rebalances at balance 0.64 and sinks onto its planted feet before it is down

### Requirement: Turning toward a fall
Under balance 0.8 (double) and not getting up, a body whose pelvis's front axis has turned more than 45° from its clip's SHALL turn its heading across the ground to chest → pelvis (front axis height over 0.4), pelvis → chest (under −0.4) or the front axis, when that is more than 37° off.

#### Scenario: Twisted
- **WHEN** a body's pelvis faces up while its clip's faces ahead, with its heading 90° off its fall
- **THEN** its heading turns to the direction from its chest to its pelvis

### Requirement: Foot contacts
Each leg of a living body SHALL keep a foot contact at its ankle with the original's state bits (2 touching, 4 anchored, 8 reached, 0x10 lifted, 0x20 no ground, 0x100 stalled) and an anchor. Not walking, a foot that does not touch SHALL be planted where it is once it meets the level. Walking, each foot plants or lifts by its phase. Standing still: lying, none touches; getting up (to 0.9 of its clip) or unsupported, each touches on its stance spot; else one at a time steps to a spot 5 cm off.

#### Scenario: Standing
- **WHEN** a body stands on a floor for 120 ticks
- **THEN** both feet touch (state 0x2), anchored where they landed at the floor plus the sole height, their grips full

#### Scenario: Walking and stopping
- **WHEN** a standing body walks at progress 0, then stops
- **THEN** the near foot stays touching and the far foot lifts; stopped, the lifted foot is planted where it is and the touching foot keeps its anchor

#### Scenario: Getting up
- **WHEN** a body gets up
- **THEN** its start clears every foot's state; in the first fifth of the clip no foot touches, then each touches on its stance spot until nine tenths

#### Scenario: Lying
- **WHEN** a body lies down
- **THEN** no foot touches and its lower legs and feet have no friction

#### Scenario: A step
- **WHEN** a standing foot's stance spot is 4.9 cm, then 5 cm from its anchor
- **THEN** it stays, then steps (anchored, its anchor the spot); the other foot does not step until it has landed, and a stalled step lands where it is

### Requirement: Gait plant phase
A walking foot's phase SHALL be the fraction of clip progress + 1.2 − 0.5×leg. A foot that does not touch SHALL plant when the phase is 0.2 to 0.4. A touching foot SHALL lift when the phase is 0 or at least 0.55. Any other foot SHALL keep its state.

#### Scenario: Progress zero
- **WHEN** the walk's progress is 0
- **THEN** leg 0 is at phase 0.2 and leg 1 at phase 0.7; a free leg 0 plants and a touching leg 1 lifts

### Requirement: Foot grip
In each iteration, after its lower leg's links, a foot of a body over half its tone with support, more than 10 cm under the pelvis, SHALL be driven to its anchor: touching and not lifted, by the PD grip d·(0.1p + 0.004g(1 − p)) less its motion across the ground × (p + 0.03g(1 − p)), or walking by the velocity servo (never down); an anchored foot reaches it within 5 cm; a foot not touching or lifted only up. Its anchor SHALL sit on its ground plus the sole height.

#### Scenario: A quarter turn in place
- **WHEN** a standing body's pose turns 90° about the vertical over 60 ticks
- **THEN** its feet step to their new spots one at a time, and a foot planted with its grip full stays within 1 cm of where it stands

#### Scenario: A lifted foot
- **WHEN** a lifted foot, or one that does not touch, is 3 cm under its anchor and 5 cm aside
- **THEN** it moves only up, and an anchor under it moves it not at all

#### Scenario: The grip
- **WHEN** a foot touches for 250, then 1000 iterations
- **THEN** its grip is 0.25, then 1, and a full grip pulls 2 cm of gap by 0.2 cm and takes back its motion across the ground; 5 cm over its anchor the hold is halved, past 9 cm gone

#### Scenario: A light shove
- **WHEN** a standing body's torso takes a 2 m/s sideways blow
- **THEN** its chest sways about 2 cm and its feet stay within 1 cm of their anchors

#### Scenario: A step's reach
- **WHEN** a stepping foot comes 4.9 cm from its anchor
- **THEN** it has reached it, counts a landing, and its anchor moves halfway to it

### Requirement: Foot friction
Each tick a living body's lower legs and their feet SHALL take their friction from their foot's state: 0 while it does not touch, 2.0 touching, 0 anchored.

#### Scenario: By state
- **WHEN** a standing body's feet touch, then it walks at progress 0
- **THEN** its lower legs and feet have friction 2.0, then 2.0 on the near foot and 0 on the far foot

### Requirement: Sole floor
In each iteration, after its foot's links, a body with support and on its feet SHALL push its toe (the sole's point 20 cm × scale ahead of the ankle along the foot) straight up toward its floor (its ground, never over the lower leg's origin) by the gap × S, at most 0.5 cm while the foot stands (touching, not anchored, not lifted), else 0.1 cm.

#### Scenario: Under the floor
- **WHEN** a standing foot's toe is 3 cm, 0.3 cm, then 0.4 cm at S = 0.5 under its floor
- **THEN** it is pushed 0.5, 0.3, then 0.2 cm (0.1 cm when the foot does not stand), and not at all off its feet or without support

### Requirement: Stance level and leg extension
With support, a body's stance level SHALL follow 1 − v² by a tenth a tick (v the share of its feet's ground difference in their squared distance), and its legs' extension SHALL be the level × (pelvis over the highest anchor ÷ the pose's pelvis over its mean ankle)², at most 1; the grip's gain g is scale × the support pins' factor × the extension.

#### Scenario: A step under one foot
- **WHEN** a body stands with one foot on a 15 cm block and the other on the floor, its feet 11.4 cm apart
- **THEN** its anchors sit 24.5 and 9.5 cm up and its level settles at 0.60, where on level ground it is 1

### Requirement: Support pin pull
In each iteration a supported living body SHALL pull its two thorax pins and its pelvis pin horizontally, in centimetres: raw is 0.2·e·|e|/60 when |e| < 60 and 12 along e otherwise, less this tick's horizontal displacement, capped at 8, times S²·pin-gain·P·pin factor, and times 0.3 while getting up. P is √E·T·E⁴. Pin-gain starts at 0.08.

#### Scenario: One application inside 60 cm
- **WHEN** a supported pin is 10 cm from its clamped target, its horizontal displacement this tick is 1 cm along that error, and the body is an alert human (S = 1, pin-gain 0.08, T = 0.5, E = 1, pin factor 1.25) that is not getting up
- **THEN** one application is (0.2·10·10/60 − 1) cm along the error, times 0.05, which is −0.0333 cm

#### Scenario: One application past 60 cm
- **WHEN** the same pin is 80 cm from its target and has not moved this tick
- **THEN** one application is 12 cm along the error, capped at 8 cm, times 0.05, which is 0.4 cm

#### Scenario: Getting up
- **WHEN** the same 10 cm pin with no displacement is on a body that is getting up
- **THEN** one application is 0.3 of the standing pull

#### Scenario: Chest returns
- **WHEN** a supported standing body's chest is held about 10 cm off the feet centre
- **THEN** later ticks of the real step bring the chest back toward the feet centre

### Requirement: Support pin targets
With support, each thorax pin's target SHALL stay within (60 − 60·crouch)·0.6·scale cm of the feet centre, or within (60 − 30·crouch)·0.6·scale cm while a foot is stepping, and the pelvis pin's target within 60·scale cm. Crouch is 0 until a crouch exists, so a standing thorax reach is 36·scale cm.

#### Scenario: Standing reach
- **WHEN** a supported body of scale 1 is not crouching and no foot is stepping, and a thorax target lies 40 cm from the feet centre
- **THEN** the target used by the pull is 36 cm from the feet centre, and a pelvis target 80 cm out is brought in to 60 cm

#### Scenario: Stepping reach
- **WHEN** the same body crouches fully and a foot is stepping
- **THEN** a thorax target is kept within 18 cm of the feet centre

### Requirement: Unsupported pin targets
With no support the pins SHALL apply no restoring pull, and their targets SHALL only shift 5·scale cm forward.

#### Scenario: No pull
- **WHEN** a body has no support and a pin is 10 cm off its target
- **THEN** one application is zero, and the target is the frame's point shifted 5·scale cm forward

### Requirement: Pins on a moving body
While a living body moves, its frame SHALL follow its pelvis, and the pins SHALL not hold the body on one world spot.

#### Scenario: Not nailed down
- **WHEN** a supported body is moving and is given a horizontal velocity
- **THEN** over later ticks its pelvis leaves the world spot it started on, and its frame's origin leaves that spot with it

### Requirement: Recovery step trigger
A standing supported body whose balance is at least 0.2 SHALL start a recovery step when the chest's horizontal offset from the support point exceeds 1.5 · R · E⁴ · S. R SHALL be (0.75E + 0.25) · base · scale · 0.5 · (1 + the race balance bonus), and 1.25 times that while a step is already going. The base SHALL be 65 cm while the body is down or getting up, and otherwise narrow by 0.5 cm a tick to a floor of 30 cm.

#### Scenario: A settled human
- **WHEN** an alert human (energy 1, support 1, scale 1, balance bonus 1, radius base 30, not already stepping) stands and its chest moves from 44 cm to past 46 cm horizontally from the support point
- **THEN** it is not stepping at 44 cm, and past 46 cm it is stepping with exactly one foot anchored

#### Scenario: Without the race bonus
- **WHEN** energy, support and scale are 1, the radius base is 30 and the balance bonus is 0
- **THEN** the reach is 22.5 cm, 28.125 cm while already stepping, and 48.75 cm when the base is 65 cm

#### Scenario: Half energy
- **WHEN** energy is 0.5, the balance bonus is 0, the base is 30, support and scale are 1, and the body is not stepping
- **THEN** the reach is 0.87890625 cm

#### Scenario: Not ready
- **WHEN** balance is 0.19, or the body has no support, even 100 cm from the support point
- **THEN** no step starts

#### Scenario: Getting up or walking
- **WHEN** a step is already running and the body gets up or starts to locomote
- **THEN** the step ends and none starts

#### Scenario: After being down
- **WHEN** the radius base is 30 cm and the body is down, then it stands for one tick
- **THEN** the base is 65 cm, then 64.5 cm, and it stays at 30 cm once it has narrowed there

### Requirement: Recovery step target
While a step is wanted, the target SHALL be the chest plus a horizontal lead of 15 ticks of the fast velocity when that is at most 2·scale cm a tick, otherwise 30·scale cm along it, each times (2 − S)(2 − E⁴·S), keeping its previous height. The target SHALL be taken only when it clears the standing position, or the current target once a step is going, by more than 10·scale cm.

#### Scenario: A slow chest
- **WHEN** support and energy and scale are 1 and the fast chest velocity is 1 cm a tick
- **THEN** the lead is 15 cm along that velocity

#### Scenario: A fast chest
- **WHEN** the same velocity is 5 cm a tick
- **THEN** the lead is 30 cm along it

### Requirement: Stance origin
During a step the foot spots SHALL be the pose's ankles shifted from the character frame onto the stance origin: the step target, pulled back to within 1.1 step lengths × scale of the frame, at the frame's height. The character frame itself SHALL not move. The step length SHALL be 80 · √scale cm. A stepping foot's anchor SHALL stay within 1.8 step lengths of the other foot.

#### Scenario: Step length
- **WHEN** scale is 1 and the leg extension is 1
- **THEN** the step length is 80 cm

### Requirement: Footwork choice
Standing with support, the footwork SHALL move at most one foot: the highest of staying and either foot, ties keeping the earlier. Stay is the ankles' stance score minus 1. A free foot within 5·scale cm of its spot (10·scale after a stall, outside a step) scores −10. Else a foot scores 1.5 plus the candidate's stance score, plus 10, 2 or 0.5 inside 100, 300 or 600 ms of a landing or the step's start, and minus 10 within 200 ms of a state change.

#### Scenario: Under 5 cm
- **WHEN** one free foot's spot is on its anchor and the other's is 6 cm off
- **THEN** neither steps while both are under 5 cm, and the foot 6 cm off steps

### Requirement: Stance score
The stance score of two feet SHALL be 2(1 − C²) + (1 − W²) + A. A SHALL be 1 minus the chest reference's horizontal distance to the foot segment, divided by 30. W SHALL be the absolute difference of the separation from 60·scale, divided by 60·scale. C SHALL be the same for 55·scale and the side separation, and that separation SHALL be its square root when it is positive.

#### Scenario: Natural and crossed
- **WHEN** the feet are 60 cm apart, the reference lies on the segment, and the side runs from the first foot to the second
- **THEN** the natural order scores 2.524 and the swapped order scores −4.744

### Requirement: Score reference
The stance score's chest reference SHALL be the chest, or, while a recovery step runs, the midpoint of the chest and the thorax pin targets. It SHALL then add 10 ticks of the fast velocity and 100·crouch cm along the facing.

#### Scenario: Ten ticks ahead
- **WHEN** the body is not stepping, the fast velocity is 1 cm a tick forward and the crouch is 0
- **THEN** the reference is the chest plus 10 cm along that velocity

### Requirement: Reach penalty
A candidate foot's score SHALL add ((|g| − 0.65 · the step length) · 0.1)² when the horizontal length of g exceeds 0.6 of the leg length. g SHALL be three quarters of the character frame plus a quarter of the stance origin, minus the anchor.

#### Scenario: Inside the leg
- **WHEN** that length is 50 cm, the leg is 100 cm and the step is 80 cm
- **THEN** the penalty is 0

#### Scenario: Past the leg
- **WHEN** that length is 70 cm, the leg is 100 cm and the step is 80 cm
- **THEN** the penalty is 3.24

### Requirement: Stepping episode
A recovery step SHALL end after 100 ms with no foot anchored, after the chest has stayed within 20·scale cm of the target for more than 32 ticks, after 200 ms while the chest is farther than that and closing under 0.05 cm a tick, or after more than one landing. The other foot landing then SHALL end it too. Ending SHALL plant the free foot nearest its pose spot plus the chest and 5 ticks of fast velocity, and move the standing position so that spot is where the foot is.

#### Scenario: The four ends
- **WHEN** 101 ms have passed with no foot anchored, or the chest has been near for 33 ticks, or 201 ms have passed with the chest 21 cm away and closing at 0.04 cm a tick, or the step has two landings
- **THEN** the step ends

#### Scenario: Still closing
- **WHEN** 50 ms have passed, a foot is anchored, the chest is 50 cm from the target and still closing, and there has been no landing
- **THEN** the step does not end

### Requirement: Thorax lead
A stepping chest inside the trigger SHALL shift each thorax pin target, before the feet-centre clamp, by the horizontal error from the pin to the posed target plus the step offset. Factor: 0.4 if Idle else 0.7 × (direction + 0.5) × (2 − 25·s) × 1.2 × √(body scale) × stance level. Direction is (h²/60 + 0.25)·0.36·forwardness (h in cm). Forwardness is 1 minus the error's absolute side-axis share. s is the fast speed along the error, or 0 when tiny or negative. The pelvis pin SHALL not shift.

#### Scenario: Sideways
- **WHEN** the pin sits on its posed target, the step offset is 40 cm along the character's side, the chest is still, body scale is 1 and the stance level is 1
- **THEN** the posed target moves 33.6 cm that way, or 19.2 cm while Idle plays, and its height is unchanged

#### Scenario: Straight ahead
- **WHEN** that offset is 40 cm straight ahead and the chest is still
- **THEN** the posed target moves 684.768 cm ahead and its height is unchanged

#### Scenario: Closing on the target
- **WHEN** the sideways offset is 40 cm and the chest's fast velocity is 1 cm a tick along it
- **THEN** the posed target moves 23.1 cm that way

#### Scenario: The pin is already along the error
- **WHEN** the posed target is 10 cm to the side of the pin and the step offset is another 40 cm the same way
- **THEN** the posed target moves 25.2 cm that way

#### Scenario: Out of balance
- **WHEN** a step is running and the chest is still past the trigger
- **THEN** the pin pass does not lead the thorax

### Requirement: Knockback step target
A knockback step SHALL keep the target the blow set, including while the body walks, until balance is under 0.2 or it is getting up.

#### Scenario: Walking through it
- **WHEN** a knockback target is 20 cm ahead and the body is walking with balance 1
- **THEN** the step stays on that target

#### Scenario: Balance gone
- **WHEN** that step's balance falls under 0.2
- **THEN** the step ends

### Requirement: Weapon stick
An armed bite above 0.1 SHALL stick. Depth SHALL cap at 1. Above a promoted 0.4 the flags SHALL be 0x211000, and float 0.4 SHALL too. A lower depth SHALL use 0x11000. Strength SHALL be 12 times the depth. Smoothing SHALL add 0.5. The count SHALL be 3 times the truncated product of 83, the unit and the depth, and the span twice that product. Depth 3 SHALL match depth 1. The weapon layer SHALL be 0xC0. The part, its parent and each child SHALL gain bit 0x40.

#### Scenario: A tenth stays free and a deep bite caps
- **WHEN** an unarmed depth of 1 is stuck, a depth of 0.1 is stuck, a depth of 0.25 with smoothing 0.1 and unit 1 is stuck, a float depth of 0.4 with smoothing 0 and unit 1 is stuck, a depth of 1 and a depth of 3 are stuck, a depth of 0.11 with unit 2 is stuck, and a part layer of 3 with a parent of 0 and children 1 and 0x40 is marked
- **THEN** the unarmed bite and the tenth stick nothing, 0.25 uses flags 0x11000, strength 3, smoothing 0.6, count 60 and span 41.5, 0.4 uses flags 0x211000, count 99 and span bits 0x4284cccd, depth 1 uses count 249 and span 166, depth 3 matches that count, span and strength with smoothing 1, unit 2 uses count 54, the weapon layer is 0xC0, and the marked layers are 0x40, 0x43, 0x41 and 0x40

### Requirement: Unarmed parry
A helpless fighter, a break-tangle motion, or yellow under 0.01 SHALL parry nothing. An attacker SHALL be attacking, not cleaving, in front, and within its reach plus 150 cm. Its clock SHALL sit from 0.3 through 0.9 unless the combo flag is set. Size 1.3 or more SHALL be refused without a shield. An unarmed parry SHALL succeed at speed 1 and blend 20. Motions 5 to 7 SHALL take a left clip, and 8 to 10 a right clip, the down clip when the table height is at or below the chest difference.

#### Scenario: A mid swing is parried and an early one is not
- **WHEN** a mid swing at clock 0.5 is in front and within reach, then the same swing is at clock 0.2, then that early swing has the combo flag, then yellow is 0.009, size is 1.3, the blow is behind, and the distance is reach plus 151 cm, and a down swing is read with the chest difference above the table and then below it
- **THEN** the mid swing is left-down at blend 20 and speed 1, the early swing is refused until the combo flag, yellow 0.009, size 1.3, the blow behind, and the far blow are refused, size 1.3 with a shield is accepted, and the down swing is left-down then left

### Requirement: Held bite
A depth above 0.1 SHALL open a grab whose pull is 1 and whose strain is the stick span. The solver link passes SHALL pull the two parts together. A depth of 0.1 SHALL open nothing, and those parts SHALL stay apart.

#### Scenario: A tenth stays apart and a quarter closes
- **WHEN** two parts 8 cm apart are solved with a bite of 0.25, then the same parts are solved with a bite of 0.1
- **THEN** the quarter bite leaves them less than a quarter of the gap apart, and the tenth bite leaves the gap where it started

### Requirement: Armed grip
A cleave SHALL be refused. Inside 80 cm the falloff SHALL be 1, at 165 cm one half, and at 250 cm and beyond 0. Handling 0.5 inside 80 cm SHALL have speed 0.5, blend 10, and strength (1.4 minus handling) times the falloff times 0.75 times 0.6. A passed blow SHALL square the handling and disable the grips. A shield of weight 0.8 and stamina 0.5 SHALL have speed 0.8, blend 16 and flags 0x800. Slot 1 SHALL be used only while attacking outside motions 8 to 10 with two weapons held.

#### Scenario: A close weapon, a passed blow, and a shield
- **WHEN** a cleave is gripped, handling 0.5 is gripped at 40 cm, at 165 cm, at 250 cm, and at 251 cm, a passed blow is gripped, a shield of weight 0.8 and stamina 0.5 is gripped, and an attack of motion 5 with two weapons, motion 9 with two weapons, and motion 5 with one weapon are gripped
- **THEN** the cleave is refused, 40 cm has speed 0.5, blend 10, and strength bits 0x3ecf5c29, 165 cm has strength bits 0x3e4f5c29, 250 cm and 251 cm have strength 0, the passed blow has speed 0.25, blend 5, and flags 0x100000, the shield has speed bits 0x3f4ccccd, blend 16, strength bits 0x3df5c290, and flags 0x800, and the slots are 1, 0, and 0

### Requirement: Striker recoil
A recoil SHALL set strength and the stall rate to 0.5 and SHALL clear the released bit. A class word of 0x90 with the continuation bit, or with a clock whose widening exceeds 0.4 and a counter, SHALL stop there. Any other swing SHALL set stalling, cleaving and recoiled, end at the truncated widened drive times 0.1 times 400 plus now plus 200, use blend 40, and set both arm stiffnesses to 0.2.

#### Scenario: A blocked swing recoils unless two weapons hold it
- **WHEN** a released swing with the window bit and drive 1 is recoiled, drive 0.013 is recoiled, a class word of 0x90 with the continuation bit is recoiled, clock 0.4 with a counter is recoiled, clock 0.4 without a counter is recoiled, and the next lower clock with a counter is recoiled
- **THEN** drive 1 has strength and stall bits 0x3f000000, flags 0x27, end 1240, blend 40 and stiffness bits 0x3e4ccccd, drive 0.013 ends at 1200, the continuation and clock 0.4 with a counter leave no end, and clock 0.4 without a counter and the next lower clock end at 1240

### Requirement: Follow-through
A class word of 0x90 continuing, or a clock widened past 0.4 with a counter, SHALL write nothing. The scale SHALL be gain times recoil times speed times 0.15, floored at 0. A body zone at or under the epsilon SHALL set strength 0, else the square root times the scale. An edge SHALL use one minus the zone times the scale plus the zone. Strength SHALL cap at 1. Stall SHALL be one minus strength times 0.05, widened. Flags SHALL gain 6. The end SHALL be the truncated scale times 400 plus now.

#### Scenario: A wound slows the swing and a continuation does not
- **WHEN** gain, recoil and speed 1 meet zone 0.25, zone 0, the epsilon, an edge at 0.25, gain 100 at zone 1, gain -1, and speed 0.009 at zone 0, and a class word of 0x90 with the continuation bit is followed through
- **THEN** zone 0.25 has strength bits 0x3d99999a, stall bits 0x3d3d70a4, flags 0x7 and end 1060, zone 0 and the epsilon have strength 0, the edge has strength bits 0x3eb9999a, gain 100 has strength 1 and end 7000, gain -1 and speed 0.009 end at 1000, and the continuation writes nothing

### Requirement: Collapse stagger
A body that is already stepping SHALL take no stagger. Otherwise a balance that widens above 0.4 SHALL scale the reach by the balance, and any lower balance SHALL scale the widened reach by 0.4. The span SHALL square the tone plus one half and multiply by that length and the scale root. The target SHALL add the velocity's reciprocal-square-root unit times the span to the chest. A zero velocity SHALL leave the chest. Landings and the step kind SHALL be 0.

#### Scenario: A still chest, a balance of 0.4, and a step already running
- **WHEN** reach 80 at balance 1, tone 1 and scale root 1 is staggered with velocity 3 and 4, reach 0.01 is staggered at balance 0.4 and at the next lower balance, a zero velocity is staggered, a scale root of the square root of 2 is staggered at balance 0.8 and tone 0.5, and a body that is already stepping is staggered
- **THEN** the first step has length bits 0x42a00000 and target bits 0x42d7faf0, 0x42be0000 and 0x430ffca0, reach 0.01 at balance 0.4 has length bits 0x3b83126f, the next lower balance has length bits 0x3b83126e, the zero velocity leaves the chest, the rooted step has length bits 0x42800000 and target bits 0x41200000, 0x42b4f9a3 and 0xc0a00000, landings and the step kind are 0, and the body that is already stepping takes nothing

### Requirement: Stagger walk
A stagger target SHALL pull the stance origin back to within 1.1 step lengths of the frame, and one footwork pass SHALL anchor one foot on its shifted spot. A target inside that length SHALL stay where it is. A target on the frame SHALL move no foot.

#### Scenario: A far target, a near target, and the frame
- **WHEN** a target 2 m ahead is walked, a target half a metre ahead is walked, a target on the frame is walked, and the stagger from reach 80, balance 1 and velocity 3 and 4 is walked
- **THEN** the far target pulls the origin to z bits 0x3f6147ae and anchors the first foot there with state 4, the near target stays at z bits 0x3f000000, the frame moves no foot, and the stagger pulls the origin to bits 0x3f072b02 and 0x3f343958 and anchors the first foot at bits 0x3f53f7cf, 0xbe4ccccc and 0x3f343958

### Requirement: Dying tick walk
A collapsing body's dying tick SHALL walk its stagger from the measured chest and the fast velocity when no step is running. A chest recorded at the origin, then a metre along z, then there again, SHALL anchor one foot on the first tick and leave that foot on the second.

#### Scenario: A measured chest anchors one foot
- **WHEN** a collapsing body records its chest at the origin, then a metre along z, then there again, and its dying tick runs twice
- **THEN** the first tick anchors the first foot and leaves the second, and the second tick leaves that same pair

### Requirement: Weapons dropped at death
A collapsing body's dying tick SHALL drop every held weapon when it has not already. Each keeps its velocity, takes its own mobility, friction 0.75 and a drag of that mobility times 0.175, and clears its striker flags. The hands, the class word and the primary SHALL be empty. A second tick SHALL add nothing. An empty pair SHALL stay empty.

#### Scenario: Two weapons leave the hands and stay loose
- **WHEN** a collapsing body holds two weapons of mobility 1.25, the first with velocity 1.5, and its dying tick runs twice, and an empty pair with a class word is released at the collapse start
- **THEN** the first tick leaves both loose with drag bits 0x3e600000, friction bits 0x3f400000, no striker flags and the velocity 1.5, the hands and the class word and the primary are empty, the second tick adds nothing, and the empty pair stays empty

### Requirement: Dropped weapon owns itself
A collapsing body's dying tick SHALL give a dropped weapon itself as owner and copy that owner onto every child. The weapon leaves its parent's list. The weapon and each descendant take the weapon as their root. The iteration count is 2 plus the parts visited, minus 1. The link count is 0 and the awake byte is 0. A grip at or above 0 becomes -1. A part that has a child, an object type whose low twelve bits are 0x89, and object bit 0x400000, sets its mark to -2.

#### Scenario: A held weapon returns to itself
- **WHEN** a collapsing body holds weapon 3, parent list 5 then 3 then 8, grip 4, child 7 of object type 0x1089 with flags 0x400001 and grandchild 8, and weapon 4 with grip -3, mark 4, parent list 9 and no child, and the dying tick runs
- **THEN** weapon 3, child 7 and grandchild 8 have owner 3 and root 3, iterations are 4, the child's iterations stay 11, links are 0, grip is -1, the awake byte is 0, the parent list is 5 then 8, child 7's mark is -2, weapon 3's mark stays, and weapon 4 keeps grip -3, mark 4, parent list 9 and iterations 2

### Requirement: Dropped weapon enters the solver
A collapsing body's dying tick SHALL put a dropped weapon that has a solid shape into the body solver, awake at 256, with its own mobility, friction 0.75, its spheres, and the velocity it had. One solver tick moves it. A weapon with no solid shape spawns nothing. A second dying tick adds no body.

#### Scenario: A solid weapon falls free
- **WHEN** a collapsing body holds a solid weapon of mobility 1.25 at height 1 with velocity 1.5 and one sphere, and a flat weapon, the dying tick runs twice, and the free body is stepped once against no level
- **THEN** the solid weapon is awake at 256 with friction bits 0x3f400000, mobility bits 0x3fa00000 and that sphere, the flat weapon spawns nothing, the second tick adds no body, and the first particle is at bits 0xbe3a7bb7, 0x3f7fa2b5 and 0xbe4ccccd

### Requirement: Dropped weapon meets another body
A collapsing body's dying tick SHALL put a solid dropped weapon into the solver, and one shipped body step SHALL record a contact when that weapon's sphere starts on another body's sphere. The contact names both bodies and has one hit. The weapon leaves the place one step alone would leave it.

#### Scenario: Two spheres start together
- **WHEN** a collapsing body drops a solid weapon of velocity 1.5 whose sphere starts on another body's sphere, and the shipped body step runs once
- **THEN** the contact names both bodies, has one hit and deepest bits 0x3ec39cd9, and the weapon's first particle is at bits 0xbda3ae15, 0x3f7e43be and 0xbe492d0c

### Requirement: Motile time warps
The system SHALL warp time as `t²(3−2t)` in mode 1 and `0.11t + 1 − 1/(1+8t)` in mode 2, with the reciprocal in single precision.

#### Scenario: Quarter and full
- **WHEN** mode 1 sees 0.25
- **THEN** the warp is 0.15625
- **WHEN** mode 2 sees 1.0
- **THEN** the warp is 0.9988889

### Requirement: Motile clock
The system SHALL advance the spent time, budget and host accumulator by dt, scaled to 0.75 while the tired flag is set.

#### Scenario: Tired
- **WHEN** the tired flag is set and dt is 1
- **THEN** the applied dt and all three accumulators are 0.75

### Requirement: Motile reservoirs
The system SHALL drain the capacity by `rate·spent` and the charge by `rate·drain`, zeroing the level; without a drain it SHALL refill by `0.1·√capacity·rate`, with `rate = (1 or 1.5)/3320 + bonus`.

#### Scenario: A draining and a refilling tick
- **WHEN** the charge is 1, capacity 3, rate 0.5, drain 1, spent 2 and spent rate 0.5
- **THEN** the capacity is 1.998 and the charge 0.499
- **WHEN** the charge is 0, capacity 4, rate 1, drain 0 and the slow flag is set
- **THEN** the rate is 0.99 and the charge is 9.036e-05

### Requirement: Motile inverse mass
The system SHALL distribute inverse masses from the 18-entry segment table with scale `n/((20·m₀ + Σ1/m)·strength)` and mobility `0.175·inverse mass`.

#### Scenario: Four parts
- **WHEN** the root inverse mass is 2, the mass scale 0.5 and the parts have bones 0–3
- **THEN** the scale is 0.691358 and the inverse masses are 0.311111, 0.483951, 0.691358 and 1.935802

### Requirement: Motile friction and action gates
The system SHALL use friction 0.2/0.4 in mode 0, 0.6/0.4 in mode 1 and the base/awareness formula otherwise, smoothing contacts as `0.9c + 0.1t` or `0.8c + 0.04`. A body may act without the `0x82C1` mask, with grip at least 0.7, awareness above 0.7 and no blocking state; readiness SHALL use a 40-tick cooldown and 0.9/0.99 clarity.

#### Scenario: Modes and gates
- **WHEN** the mode is 3 with base 0.5 and awareness 0.8
- **THEN** the friction is 0.1 and the target 0.32
- **WHEN** a clear body has clarity 0.995 and its 20-counter is 21
- **THEN** it is not ready

### Requirement: Motile posture
The system SHALL compute lift as `0.9 + 0.09·lean − 0.6·(1−sag_b) − 0.1·(1−sag_a)` clamped at 0, and compliance as `1 − 0.01·lean + 0.05·(1−sag_a) + 0.30·(1−sag_b)` clamped at 1, with the second operand on NaN.

#### Scenario: Two postures
- **WHEN** lean, sag_a and sag_b are all 1
- **THEN** lift and compliance are 0.99
- **WHEN** lean is 0.4, sag_a 0.5 and sag_b 0
- **THEN** lift is 0.286 and compliance 1

### Requirement: Motile follow
The system SHALL advance the follow point by 0.6 of the target delta plus the velocity gain (Y a tenth), with gain 0.0125 or +0.0025 while active up to 0.15.

#### Scenario: One step
- **WHEN** the point is (0.1, 1, −0.2), the velocity (0.03, 0, 0), the target (0.4, 1, −0.2) and the gain 0.0125
- **THEN** the point becomes (0.280225, 1, −0.2)

### Requirement: Motile ramps and tone
The system SHALL ramp posture by 0.004 to 1, decay support by 0.02 to 0, and compute the tone weight from the warm blend or the flagged/unflagged error and scale paths.

#### Scenario: Tone paths
- **WHEN** the error is 0.42 in a warm state
- **THEN** the weight is 0.71
- **WHEN** the error is 0.42, unflagged and not warm
- **THEN** the weight is 0.1764
- **WHEN** the error is 0.42, flagged with scale 1.5 and state 0
- **THEN** the weight is 0.171108

### Requirement: Motile cadence and angles
The system SHALL compute cadence as `trunc((1 − 0.25·exertion)·1660/83)`, apply the pitch impulse with a 0.95 decay, wrap angles with 2π and test impacts as `threshold·scale < |v|²`.

#### Scenario: Cadence and wrap
- **WHEN** exertion is 0.5
- **THEN** the cadence target is 17
- **WHEN** the angle is 3.5
- **THEN** it wraps to −2.7831855
- **WHEN** the threshold is 2, scale 1 and squared impact 3
- **THEN** the impact crosses

### Requirement: Motile shock and split
The system SHALL clear the 0x40 shock bit only when the unsigned clock is past the deadline, and SHALL enter the posture block when balance is below 0.6 or the condition is 0x300.

#### Scenario: Shock and split
- **WHEN** the deadline is 1000 and the clock 1000
- **THEN** the bit does not clear
- **WHEN** the clock is 1001
- **THEN** it clears
- **WHEN** balance is 0.59
- **THEN** the posture block is entered

### Requirement: Motile attack arms
The system SHALL compute the attack arm base and length from the posture, clock, size and weapon class, with the stab 0.3 scale and the second arm's `(1−clock)²·p·4`.

#### Scenario: Two clocks
- **WHEN** p is 1, clock 0.1, size 1 and class 0
- **THEN** the length is 1.190995 and the other arm 1
- **WHEN** clock is 0.6
- **THEN** the length is 1.01596 and the other arm 0.64

### Requirement: Motile arm and phase leaves
The system SHALL compute the arm floor, parry ramp, gait clamp, auto-parry deadline, switch phase and get-up scale.

#### Scenario: Leaves
- **WHEN** the turn is 0.5
- **THEN** the arm floor is 0.2
- **WHEN** the parry previous is 0
- **THEN** the ramp is 0.04
- **WHEN** the gait is 6.334
- **THEN** the gait arm is 0.05
- **WHEN** the 0x20 flag is set with ea4 1000
- **THEN** the deadline is 1400
- **WHEN** progress is 0.6
- **THEN** the phase is 0x10004

### Requirement: Motile route order
The system SHALL check the collapse gate, the shock bit, the posture split and the posture leaves before the clip branches.

#### Scenario: Gate and shock
- **WHEN** bit 0x800 is set
- **THEN** the route is Skip
- **WHEN** bit 0x40 is set
- **THEN** the route is Shock

### Requirement: Motile posture routes
The system SHALL route condition 0x300 to get-up, balance below 0.3 with motion below 25 to lying still, an upright lean to upright, the stagger window to stagger and otherwise to fall.

#### Scenario: Leaves
- **WHEN** balance is 0.29 and motion 0
- **THEN** the route is LyingStill
- **WHEN** balance is 0.59 and the lean is upright
- **THEN** the route is Upright

### Requirement: Motile clip routes
The system SHALL route clips 5–12 to attack, tone below 0.85 to weak, clip 0x0D to kick, parrying to parry, clip 4 to parry-end, clips 0x22–0x28 to power, clips 0–1 to gait, clip 0x1F to switch and the rest to standing.

#### Scenario: Branches
- **WHEN** the clip is 5
- **THEN** the route is Attack
- **WHEN** the tone is 0.8 and the clip 3
- **THEN** the route is Weak
- **WHEN** the clip is 0x0D
- **THEN** the route is Kick

### Requirement: Parallel per-body work
The system SHALL run each body's own work in a tick (integration, links, level detection, level friction, post-step) in parallel across bodies, keeping the original's interleaving with the pair passes and pair friction for the bodies those touch, with results identical to stepping the bodies one at a time.

#### Scenario: Free bodies
- **WHEN** several bodies that touch no other body tick together
- **THEN** each ends the tick exactly where the interleaved, one-at-a-time path leaves it

### Requirement: Rigid body translate
A translate SHALL move each particle of a rigid body by the offset times its weight, and with its restore flag add the offset's length to the body's strain and restore the six edge lengths once the strain passes 0.1 cm; without the flag the strain SHALL be left alone.

#### Scenario: Weighted translate
- **WHEN** a body is translated by 0.4 mm with weights 1, ½, 0, 2 and no restore flag
- **THEN** its particles move by 0.4, 0.2, 0 and 0.8 mm and its strain is unchanged

### Requirement: Rigid body placed at a matrix
Placing a rigid body at a matrix SHALL put its particles, their previous points and their tick's start at the matrix times the rest points, with its contacts and strain left as they are.

#### Scenario: At rest
- **WHEN** a shoved body is placed at a matrix
- **THEN** its next tick only adds gravity

### Requirement: Rigid body settling
While its settling countdown runs, a rigid body SHALL take each tick's gravity with no velocity carried or recorded.

#### Scenario: Settling
- **WHEN** a body settles for three ticks
- **THEN** each tick drops it by one tick's gravity, and the next tick's velocity is the last tick's drop alone
