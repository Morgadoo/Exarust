## MODIFIED Requirements

### Requirement: Props registered as bodies
The system SHALL register every free or hung body tree (a top body and its body children) of a level's props, placed objects, and own model: asleep (countdown 0) at its nodes' world matrices, at rest, its mobility the stored over its divisor, gravity by shares 4/mᵢ scaled by 4 over their sum over all four masses, the stored spheres against the level with friction 0.75. Mechanisms SHALL register with their rules (sliding leaves only when asked). A blow SHALL wake a body.

#### Scenario: First level
- **WHEN** `exanima01` loads
- **THEN** 4 874 trees of 6 198 bodies register asleep, 283 of them hung from the world (109 hinged door leaves, 40 lever arms and 12 push surfaces among them); its 37 sliding door leaves stay static

#### Scenario: Sliding leaves asked for
- **WHEN** `exanima01` loads with sliding doors asked for
- **THEN** its 37 sliding leaves register too, and no mechanism stays static

#### Scenario: Asleep where placed
- **WHEN** a registered prop 30 cm over the floor is ticked 500 times
- **THEN** its particles stay exactly where its node placed them and it stays asleep

#### Scenario: Shoved
- **WHEN** a prop 10 cm over the floor takes a 0.5 m/s blow
- **THEN** it wakes to 256, lands, slides about 6 cm and sleeps again 140 ticks later, resting on the floor

#### Scenario: Gravity at the centre of mass
- **WHEN** a body with point masses 2, 4, 8 and 8 is integrated once from rest
- **THEN** its particles fall 2, 1, 0.5 and 0.5 times 0.142352 cm

#### Scenario: Equal masses fall at g
- **WHEN** a body with four equal point masses is integrated once from rest
- **THEN** each particle falls 0.142352 cm

## ADDED Requirements

### Requirement: Root slider slack
A root body's slider link to the world SHALL keep its travel as its slack, the slider law's fraction being the projected offset over the travel squared.

#### Scenario: A portcullis rail
- **WHEN** a root body with a 170 cm slider registers
- **THEN** its world link's slack is 1.7 m, not the child's 289

### Requirement: Mechanism environment
A lever's arm and a push surface SHALL run with no gravity on any particle and keep 0.001 of their velocity a second (0.001^(1/83) a tick) in every speed band.

#### Scenario: A lever's particles
- **WHEN** a lever's arm is set up
- **THEN** its gravity shares are all 0

### Requirement: Lever arm body
A lever's arm SHALL register hung by its links with mobility 1, settle on its nearest detent (ties to even), and re-arm a return saved in the word's bits 16 and up.

#### Scenario: A saved return
- **WHEN** a lever with a return time registers with 3 in its word's bits 16 and up
- **THEN** its return is armed to fire 300 ms on

### Requirement: Lever frame
A lever's frame SHALL be built in the parent node's frame: up the axle (the last pin farther than 1 cm) turned by the node's rotation and negated, forward the arm link's tied point less the node's position with its part along up removed, normalised, side up × forward, normalised.

#### Scenario: The wheel's planes
- **WHEN** a lever's axle lies along x
- **THEN** the first wheel plane's normal is x

### Requirement: Lever detents and angle
A lever's first detent SHALL be the tied point's heading in its frame less the half swing, its second one step of twice the half swing further; its angle SHALL be the heading in the frame of the arm point less the node's world position at set-up.

#### Scenario: At its file position
- **WHEN** a lever whose arm sits at −α from its tied point registers
- **THEN** its detents are −α and +α and it reads detent 0 with its angle −α

#### Scenario: At rest
- **WHEN** a lever is woken and left 800 ticks
- **THEN** its arm moves less than 2 mm, sends nothing and sleeps

#### Scenario: Shipped levers
- **WHEN** `exanima01`'s 40 levers are woken and ticked two seconds
- **THEN** none moves a centimetre or leaves its detent, and none signals

### Requirement: Lever drive
Each awake tick after the solver, a lever SHALL run its step with the arm point's motion (cm) and angle, move the arm point onto the arm's radius at the step's drive angle by its weights, send a mechanism signal per target its report names, and count an armed return down every tick, asleep or not, waking the lever when it fires.

#### Scenario: A timed return
- **WHEN** a lever with a 500 ms return is turned onto its second detent and let go
- **THEN** it signals 1, and once the time has run returns to its first detent and signals 0

### Requirement: Push surface body
A push surface SHALL register on its slider links with mobility 0.3 when its registered one is at least 0.5, else 0.03; each awake tick it SHALL set every link's smoothing to 0.1 and run the plate step on the links' travel fractions, writing the mean back, waking itself 256 ticks while it grinds and sending a mechanism signal per target its report names.

#### Scenario: Pushed
- **WHEN** a plate is pushed for 120 ticks and let go for 200
- **THEN** it sinks down its rails, signals 1 to each target once, then rises and signals 0

#### Scenario: A weight
- **WHEN** a weight holds a plate at the bottom of its rails
- **THEN** it presses and signals 1

### Requirement: Signal routing
A mechanism signal SHALL be dispatched by the records of the placed object whose id (tag bits included) is its target: a door order SHALL run on the door rules of every door body under the object's node, each woken; a class 5 push on the push surface under it; relays and jobs SHALL not run.

#### Scenario: A lever and its portcullis
- **WHEN** a lever of `exanima01` is turned onto its other detent with sliding doors asked for
- **THEN** it signals its portcullis, which takes the signalled open

### Requirement: Sliding leaf body
When asked for, a door's sliding leaf SHALL register on its slider links with the door's sliding rules, its travel link the first whose body point is 30–160 cm up, reading shut at a fraction of 0.995 or more; each awake tick it SHALL feed the link's fraction to the sliding step and write the step's fraction and smoothing back, a command's fraction written into the link first.

#### Scenario: Shut
- **WHEN** a leaf at the end of its rails is woken and left 600 ticks
- **THEN** it stays shut where it is and sleeps
