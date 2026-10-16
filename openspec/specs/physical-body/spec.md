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
The system SHALL drive a living physical body toward its animated pose: its link muscles turn every joint, and a pose drive turns the pelvis, lumbar, thorax and feet (gain 0.03·K, K = 19200/83²) and the head (0.01·K) toward their animated orientation about where each part's origin is, with no position target. Thighs, shins, arms and hands SHALL have no pose drive. The carry holds the body up in place of gravity.

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

### Requirement: Living bodies in motion
The system SHALL give a living physical body the animated pose of each tick: its targets are the body root's world transform times the clip pose. A standing body SHALL be carried with its root's motion since the last tick across the ground and about the vertical only, together with where the last tick left it, its level contacts and last tick's targets, and its pelvis's origin held over the pose's pelvis by 0.168 × tone² × energy⁴ a tick, critically damped (Exarust's own).

#### Scenario: Turning far from the origin
- **WHEN** a living body 80 m from the world's origin turns with its pose
- **THEN** its particles move no faster than the pose's own motion

#### Scenario: Carried by the controller
- **WHEN** the root of a body standing on a floor moves across it and down
- **THEN** its particles, where the last tick left each part and its level contacts move with the root across the floor but not down, while where it is drawn from stays

#### Scenario: Held on the controller
- **WHEN** a standing body's pelvis is off where the controller carries the pose's pelvis
- **THEN** it is pulled back across the ground, and the hold never moves it up or down

### Requirement: Blows and daze
The system SHALL let a blow change the velocity of the struck part and take a share of the body's energy (at most 90 %, never below 0.05), which recovers by 0.005 per tick up to the body's most energy, 1 for a human. Blows SHALL be queued and dealt by the next tick, whichever frame sends them.

#### Scenario: Stagger
- **WHEN** a body's chest is shoved at 3 m/s with daze 0.7
- **THEN** the chest is pushed off its pose, the body sags, and both recover within about a second

#### Scenario: Blow between ticks
- **WHEN** a blow is sent in a frame that runs no tick
- **THEN** it waits in the queue and the next tick deals it

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
The system SHALL make a standing body whose balance was below 0.6 last tick fall: it loses its controller's support and hold, keeps its tone, plays Fall Forward when its lean points ahead of it and Fall Backward otherwise, and it is not steered. Once it has lain for 2.5 s and its energy is back to 0.9, it SHALL get up with the get-up clip whose first pose lies the way the body does, placed under the body, its controller settling on the ground there.

#### Scenario: Knocked over
- **WHEN** `exarust --shove 60 play arena` deals the player a body blow
- **THEN** the player falls onto its back, lies a moment, gets up with `Get Up 2` without a hop, and stands where it fell

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
The system SHALL sleep a limp body by the original's countdown: 256 when woken, then each tick, by its fastest part's last motion, back to 256 over 40 cm/s, held at 30–32 between 20 and 40 cm/s, and two off under 20 cm/s; at 1 it sleeps. Its first tick asleep SHALL take its velocity away; later ticks SHALL do nothing. A living body SHALL never sleep. A blow or going limp SHALL wake a body; level contacts SHALL NOT. A body woken by a blow SHALL start without its earlier contacts.

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
`exarust --trace <file.csv>` SHALL write one row per traced body (the player's, or else every simulated body) after every tick, with the retail body probe's columns: the pelvis particles; its heights (the lift's target included), tone, energy, support, balance, counter; every part's target; the foot contacts; the motion and clock; then its sleep countdown and state. Lengths in cm, the clock in clip frames; columns not simulated yet stay empty.

#### Scenario: Standing in the arena
- **WHEN** `exarust --trace t.csv play arena` runs for a minute
- **THEN** `t.csv` has one row per tick for the player's body, its clock advancing 60/83 of a frame per row, and its pelvis 1.7 cm over its target pelvis height

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
Each tick the character tick SHALL set: the stance factor s, the race's standing (human 0.5), rising 0.01 a tick toward 0.7 + 0.3·run while moving; T = tone·s; part factors 0.5, a foot half its lower leg's, the head a quarter of the thorax's, a thigh 0.25 standing and 0.2 moving, shoulders, arms and hands 1.25 × arm stiffness; each leg limit link's strength s ← 0.9s + 0.1t, t = 0.5·S² standing, 0.4 moving.

#### Scenario: Walking firms the body
- **WHEN** a standing human starts walking
- **THEN** its stance factor rises from 0.5 to 0.7 in 20 ticks, reaches 1 running, keeps it while it moves on, and is 0.5 again when it stops

#### Scenario: Leg limit
- **WHEN** a living human stands for one tick
- **THEN** its leg limit links pull at 0.23, settling at 0.5, at 0.4 while it moves and toward 0 without support

### Requirement: Arm stiffness
The arms' stiffness SHALL be 0.5 off balance (balance under 0.6, falling, lying or getting up), 0.2 at tone under 0.85 or in the walk and run cycles, and otherwise x ← 0.8x + 0.08·(0.5 + 0.5·race arm relaxation), never under min(0.4·|turn left in rad|, 0.4). An arm part's anchor relaxation SHALL be capped at 0.6 − (x − 0.1)·0.8.

#### Scenario: Idle human arms
- **WHEN** an alert human stands still
- **THEN** its arms settle at stiffness 0.4 and pull with σ = 0.25, their relaxation capped at 0.36

#### Scenario: Walking and off balance
- **WHEN** the human walks, and later falls
- **THEN** its arms pull with σ = 0.175 walking and 0.3125 off balance

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
Each tick each leg of a living body SHALL query the ground 5 cm ahead of its ankle, a 10 cm sphere from 70 cm above it (a 5 cm one from it while down or getting up). Ground over half the leg under the pelvis is out of reach, except getting up. A foot is grounded within 20 cm (30 moving) of its ground plus its ground offset. S SHALL be grounded × balance; a standing body without a grounded foot loses its balance unless its controller stands on the ground.

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
- **THEN** its leg finds the floor under both, where a standing body finds the shelf and the step's edge

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
`exarust --joint-trace <file.csv>` SHALL write, after every tick, one row per part of every simulated body not in deep sleep: the tick, body and bone; the joint's measures in degrees, cm and mm; the clip's turn split the same way; whether a blow landed on the part in the tick; the body's sleep state and countdown; its clip and clock.

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
Until the feet step and the legs bend (Exarust's own), while a living body that has not fallen has its character controller on the ground, standing or moving, the target pelvis height SHALL read each foot's ground no lower than the controller's floor less 6 cm. A controller in the air or under a fallen body SHALL change nothing.

#### Scenario: Level ground
- **WHEN** a body walks or stands with its controller floating 0, 2 or 5 cm over a level floor
- **THEN** its target is the original's every tick: in `--autowalk play arena` the same as without the rule, the pelvis 99.4–102.2 cm walking and 101.5 cm standing

#### Scenario: Stairs
- **WHEN** a body walks up four 18 cm steps with its controller, stopping on the third
- **THEN** it keeps its balance and ends on the landing, its pelvis 72 cm higher; with the original's target alone it loses its balance

#### Scenario: Stair room
- **WHEN** `exarust --autowalk --at 7.5,-0.95,64.4 play exanima01` or `--at 7.6,-1.3,66.2` walks the player up the stairs to the door
- **THEN** it climbs to the landing without falling on the stairs and stands there

#### Scenario: Perched
- **WHEN** a body's grounded controller stands 12 cm over the floor under its feet, or is in the air
- **THEN** its target is 6 cm higher than the original's, or the original's

#### Scenario: Fallen
- **WHEN** a body has just fallen, its feet on the floor and its grounded controller 12 cm over it
- **THEN** its target is the original's every tick, as with its controller on the floor

### Requirement: Feral sprint gait
The sprint gait of a character SHALL be its race's sprint row when its feral level is 0, and otherwise every column of that row moved linearly toward the race's jog row by the feral level.

#### Scenario: Half feral human
- **WHEN** a human of feral level 0.5 sprints
- **THEN** its sprint thrust is 411.1 per second, its clip rate 15.05 frames per second and its brake distance 2.25 m

#### Scenario: Zombie
- **WHEN** a human of feral level 1.0 sprints
- **THEN** it uses the jog row: thrust 350 per second
