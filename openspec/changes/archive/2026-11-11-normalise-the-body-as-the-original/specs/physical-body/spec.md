## ADDED Requirements

### Requirement: Body normalisations
The body class's routines SHALL normalise as the original's do. The torso twist's first normalisation, the midline's half turn (in the living and pose-only samplers and in the plain and whole captures), the thorax hunch's rebuilt basis, the femur posture's axis, the daze pose's random axis, the turn toward a fall, the get-up's direction and the facing it writes, and the dying tick's wandering heading SHALL scale by the processor's approximate reciprocal square root (`RSQRTSS`) of the squared length summed `(x² + z²) + y²`, a zero vector staying zero. The captures' and the daze pose's quaternions SHALL be normalised by the engine's four-lane normaliser (`(y² + w²) + (x² + z²)`, its exact reciprocal root); the femur posture's and the daze pose's turns SHALL be the engine's quaternion of an axis and angle (its fast sine and cosine of the half angle over the axis's exact length); the twist's followed yaw SHALL take its direction from the engine's sine and cosine; and a turn toward a fall or a get-up SHALL take its yaw from the engine's heading of the written facing. The standing and locomoting leg IK SHALL keep their exact normalisations, as the original's do.

#### Scenario: A get-up's direction
- **WHEN** a body gets up facing down with its chest 0.6 cm ahead of its pelvis across the ground
- **THEN** the direction's length is RSQRTSS(0.36) × 0.6, within 12 bits of 1, and the facing it writes is normalised again the same way

#### Scenario: The femur posture's axis
- **WHEN** a body takes the splayed femur posture
- **THEN** its axis is (−0.4, 0, 0.6) scaled by RSQRTSS(0.52) and its turn the engine's quaternion of 5° about it

## MODIFIED Requirements

### Requirement: Femur posture
While the behaviour is in combat and the condition is neither off its feet nor walking, the thighs SHALL be turned about the axis (0, 0, 1) lerped 0.4 toward (-1, 0, 0), normalised by the approximate reciprocal square root, by (1 - record height) times -1.5 degrees, as the engine's quaternion of that axis and angle; otherwise they SHALL be turned 5 degrees with the state word's splay bit, else keep the clip's turn; the sign alternates by leg.

#### Scenario: Standing
- **WHEN** the behaviour is not in combat and the splay bit is clear
- **THEN** the thighs keep the clip's turn

#### Scenario: Walking in combat
- **WHEN** the behaviour is in combat and the condition is walking
- **THEN** the thighs keep the clip's turn

### Requirement: Start capture half-lean
A start capture, plain or whole, SHALL replace the physical turn of the lumbar and the thorax (the midline bones between the legs and the head) with that turn whose forward axis is moved halfway toward the same turn taken out of the twist frame, whose up axis is made square to that forward and whose side is their cross product, each normalised by the approximate reciprocal square root. The held pose SHALL keep the measured turns.

#### Scenario: Twisted upright spine
- **WHEN** a plain start captures an upright thorax while the twist is 0.6 rad of yaw
- **THEN** the captured thorax turn is −0.3 rad of yaw

#### Scenario: A whole capture
- **WHEN** a whole capture (a get-up's) takes the same thorax under the same twist
- **THEN** its captured thorax turn is the same −0.3 rad of yaw
