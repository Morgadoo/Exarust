## MODIFIED Requirements

### Requirement: Support pin pull
In each iteration a supported living body SHALL pull its two thorax pins and its pelvis origin in order, each reading the pushes before it, then the pelvis origin again, toward the goals its tick's carry set (one set a tick, not led or clamped again in the iterations). One pull, in cm: 0.2·e·|e|/60 under 60, else 12 along e, less this tick's horizontal displacement, capped at 8, times S²·(G·(P·F)), G the pin-gain (from 0.08), F the pin factor. A gap under 1.19e-7 cm², walking, or off its feet (down, getting up, dormant or dead) pulls nothing.

#### Scenario: Pelvis origin twice
- **WHEN** a supported pelvis origin and a thorax pin are each 10 cm from their clamped targets and have not moved this tick
- **THEN** the thorax pin takes one application and the pelvis origin takes a second after the first push has moved it

#### Scenario: One application inside 60 cm
- **WHEN** a supported pin is 10 cm from its clamped target, its horizontal displacement this tick is 1 cm along that error, and the body is an alert human (S = 1, pin-gain 0.08, T = 0.5, E = 1, pin factor 1.25) that is not getting up
- **THEN** one application is (0.2·10·10/60 − 1) cm along the error, times 0.05, which is −0.0333 cm

#### Scenario: One application past 60 cm
- **WHEN** the same pin is 80 cm from its target and has not moved this tick
- **THEN** one application is 12 cm along the error, capped at 8 cm, times 0.05, which is 0.4 cm

#### Scenario: A tiny gap
- **WHEN** a supported pin's horizontal gap is 0, then under 1.19e-7 cm², and it moved 1 cm this tick
- **THEN** one application is zero both times

#### Scenario: Walking or off its feet
- **WHEN** a supported body with its chest 10 cm off the feet centre is walking, then off its feet while getting up, then walking in a shock stagger
- **THEN** the pins do not move it

#### Scenario: Collapsing
- **WHEN** that body is collapsing instead, and not walking
- **THEN** one application is the standing pull

#### Scenario: Chest returns
- **WHEN** a supported standing body's chest is held about 10 cm off the feet centre
- **THEN** later ticks of the real step bring the chest back toward the feet centre

#### Scenario: Goals once a tick
- **WHEN** a body is carried while a step leads its thorax, and its pins are then pulled
- **THEN** the pulls move it toward the goals the carry led, and the goals do not change between iterations

### Requirement: Support pin targets
Each tick the carry SHALL set each support pin's goal from its posed point on the character frame, led while a step leads, then clamp it across the ground about the feet centre: a thorax goal to (60 − 60·crouch)·0.6·scale cm, or (60 − 30·crouch)·0.6·scale cm while the body's step episode runs (the 0.6 taken in double, the reach not floored), the pelvis goal to 60·scale cm. A clamped goal SHALL sit at the feet centre's height; one inside the reach keeps its own. The thorax goals' mean SHALL be kept for the stance score (the frame's origin without thorax pins). Crouch is 0 until a crouch exists, so a standing thorax reach is 36·scale cm.

#### Scenario: Standing reach
- **WHEN** a supported body of scale 1 is not crouching and runs no step episode, and a thorax target lies 40 cm from the feet centre
- **THEN** the target used by the pull is 36 cm from the feet centre, and a pelvis target 80 cm out is brought in to 60 cm

#### Scenario: Stepping reach
- **WHEN** the same body crouches fully and its step episode runs
- **THEN** a thorax target is kept within 18 cm of the feet centre

#### Scenario: The mean
- **WHEN** a standing body is carried
- **THEN** the kept mean is the mean of its thorax goals

### Requirement: Unsupported pin targets
With no support the pins SHALL apply no restoring pull, and the thorax goals SHALL shift 5·scale cm along the frame's forward axis before the lead and the clamp.

#### Scenario: No pull
- **WHEN** a body has no support and a pin is 10 cm off its target
- **THEN** one application is zero, and the thorax goal is the frame's point shifted 5·scale cm forward

### Requirement: Score reference
The stance score's chest reference SHALL be the chest, or, while a recovery step runs, the midpoint of the chest and the mean of the thorax pin goals the last carry kept (led and clamped). It SHALL then add 10 ticks of the fast velocity and 100·crouch cm along the facing.

#### Scenario: Ten ticks ahead
- **WHEN** the body is not stepping, the fast velocity is 1 cm a tick forward and the crouch is 0
- **THEN** the reference is the chest plus 10 cm along that velocity

### Requirement: Thorax lead
A stepping chest inside the trigger SHALL shift each thorax pin goal, before the feet-centre clamp, by the horizontal error e from the pin's world point to the posed goal plus the step offset (h = |e| in cm). Past the epsilon (h² over 1.19e-7): s is the fast velocity along e over h², 0 when ordered below 0; for a plain step the direction is (h²·(1/60) + 0.25)·0.36 (double) × (1 − |side·e/h|) and the exertion value becomes c·h·−0.004 for c = forward·e/h above 0, else 0; for a long (knockback) step s grows by (2500 − h²)·0.0004 + 1 under 2500 cm², m = 1 − forward·e/h, the direction is m − (2 − m)·s·2 and the exertion becomes (1 − m)·h·−0.002 while m is under 1, else 0. At or under the epsilon s and the direction are 0 and the exertion is left. The goal moves by e × (0.4 if Idle else 0.7) × (0.5 + direction) × 85/83 × (2 − 25·s) × kind (1.2, a long step 1) × √(body scale) × stance level. The offset SHALL be taken from the frame's origin. The character's exertion value SHALL take the last thorax pin's write after its locomotion this tick.

#### Scenario: Sideways
- **WHEN** the pin sits on its posed target, the step offset is 40 cm along the character's side, the chest is still, body scale is 1 and the stance level is 1
- **THEN** the posed target moves 34.41 cm that way, or 19.66 cm while Idle plays, its height is unchanged, and the exertion becomes 0

#### Scenario: Straight ahead
- **WHEN** that offset is 40 cm straight ahead and the chest is still
- **THEN** the posed target moves 701.27 cm ahead, its height is unchanged, and the exertion becomes −0.16

#### Scenario: Closing on the target
- **WHEN** the sideways offset is 40 cm and the chest's fast velocity is 1 cm a tick along it
- **THEN** the posed target moves 23.66 cm that way

#### Scenario: The pin is already along the error
- **WHEN** the posed target is 10 cm to the side of the pin and the step offset is another 40 cm the same way
- **THEN** the posed target moves 25.81 cm that way

#### Scenario: A long step
- **WHEN** a long step's offset is 40 cm straight ahead with a still chest, then straight back
- **THEN** the goal moves 28.68 cm ahead with the exertion −0.08, then 143.37 cm back with the exertion 0

#### Scenario: Out of balance
- **WHEN** a step is running and the chest is still past the trigger
- **THEN** the pin pass does not lead the thorax

### Requirement: Pelvis lead
While a step leads, the pelvis pin's posed goal SHALL shift, before the feet-centre clamp, by the horizontal error e from its point to the posed goal plus the step offset, times 0.7 (0.4 in Idle) × (0.75·s + 1) × 85/83 × 1.2·scale (double) × level. For a plain step s is e's cosine with the frame's forward axis, halved when negative, else times (|e| / (reach·extension))²; a long step has s = 0. The pelvis goal takes no unsupported shift.

#### Scenario: Ahead and back
- **WHEN** the pin sits on its posed target and the offset is 40 cm straight ahead, then straight back, with an 80 cm step length, scale and level 1
- **THEN** the posed target moves 40.86 cm ahead, then 21.51 cm back

#### Scenario: A held step backwards
- **WHEN** footwork steps 50 cm behind the chest are laid whenever none runs, for three seconds on level ground
- **THEN** the body never loses its support and moves more than 50 cm back
