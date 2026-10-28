## ADDED Requirements

### Requirement: Spawn placement
The spawn SHALL run the placement leg snap after sampling the first clip, move the character's feet to the character ground the snap finds and sample the snapped frame again.

#### Scenario: A raised spawn
- **WHEN** a character is spawned with its feet 5 cm over a floor
- **THEN** its controller and model root are moved down to the floor's height before its first tick

### Requirement: Trailing-foot request
A clip start that replaces Walk or Run SHALL leave a trailing-foot request with a deadline 500 ms on, unless the movement decision moves the body that tick. The body tick SHALL plant the trailing foot after its sample while the request is 2 and set it to 1. A request strictly past its deadline SHALL be dropped and the behaviour told; the movement decision's move branch SHALL drop it.

#### Scenario: Leaving the walk
- **WHEN** a standing body's Walk is replaced by an attack at 1000 ms
- **THEN** the next sample plants its trailing foot, the request stays 1 at 1500 ms and is dropped at 1501 ms

#### Scenario: Moving
- **WHEN** a body with a request is moved by the movement decision
- **THEN** its request is 0

## MODIFIED Requirements

### Requirement: Femur posture
While the behaviour is in combat and the condition is neither off its feet nor walking, the thighs SHALL be turned about the axis (0, 0, 1) lerped 0.4 toward (-1, 0, 0) by (1 - record height) times -1.5 degrees; otherwise they SHALL be turned 5 degrees with the state word's splay bit, else keep the clip's turn; the sign alternates by leg.

#### Scenario: Standing
- **WHEN** the behaviour is not in combat and the splay bit is clear
- **THEN** the thighs keep the clip's turn

#### Scenario: Walking in combat
- **WHEN** the behaviour is in combat and the condition is walking
- **THEN** the thighs keep the clip's turn

### Requirement: Placement leg snap
A living body placed into the scene SHALL probe each leg 5·scale cm along its shin from its clip ankle with a 10 cm sphere from the frame's height plus the leg's length 300 cm down, a miss 10 000 cm under the frame, and average the hits into the character's ground. Each anchor SHALL be its clip ankle at its ground plus the sole, its ground and floor that ground, and the standing leg IK SHALL write its corrections at once with no rate and no thigh turn.

#### Scenario: A platform
- **WHEN** a biped is placed over a platform 15 cm up
- **THEN** each leg's ground and floor are 15 cm, each anchor 15 cm plus the sole, the character's ground 15 cm, the feet placed at 15 cm, and the corrections' rates zero
