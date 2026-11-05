## MODIFIED Requirements

### Requirement: Thorax lead
A stepping chest inside the trigger SHALL shift each thorax pin target, before the feet-centre clamp, by the horizontal error from the pin to the posed target plus the step offset. Factor: 0.4 if Idle else 0.7 × (direction + 0.5) × (2 − 25·s) × 1.2 × √(body scale) × stance level. Direction is (h²/60 + 0.25)·0.36·forwardness (h in cm). Forwardness is 1 − |error's side-axis share|; s is the fast speed along the error (0 when tiny or negative). The offset SHALL be taken from the frame's origin.

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

### Requirement: Support pin targets
With support, each thorax pin's target SHALL stay within (60 − 60·crouch)·0.6·scale cm of the feet centre, or within (60 − 30·crouch)·0.6·scale cm while the body's step episode runs, and the pelvis pin's target within 60·scale cm. Crouch is 0 until a crouch exists, so a standing thorax reach is 36·scale cm.

#### Scenario: Standing reach
- **WHEN** a supported body of scale 1 is not crouching and runs no step episode, and a thorax target lies 40 cm from the feet centre
- **THEN** the target used by the pull is 36 cm from the feet centre, and a pelvis target 80 cm out is brought in to 60 cm

#### Scenario: Stepping reach
- **WHEN** the same body crouches fully and its step episode runs
- **THEN** a thorax target is kept within 18 cm of the feet centre

### Requirement: Stance origin
During a step the foot spots SHALL be the pose's ankles shifted from the character frame onto the stance origin: the step target, pulled back to within (1.1·step length)·scale of the frame (in double, whatever the step's kind), at the frame's height. The character frame itself SHALL not move. The step length SHALL be 80 · √scale cm. A stepping foot's anchor SHALL stay within 1.8 step lengths of the other foot.

#### Scenario: Step length
- **WHEN** scale is 1 and the leg extension is 1
- **THEN** the step length is 80 cm

## ADDED Requirements

### Requirement: Pelvis lead
While a step leads, the pelvis pin's posed target SHALL shift, before the feet-centre clamp, by the horizontal error e from its point to the posed target plus the step offset, times 0.7 (0.4 in Idle) × (0.75·s + 1) × 1.2·scale × level. For a plain step s is e's cosine with the frame's forward axis, halved when negative, else times (|e| / (reach·extension))²; a long step has s = 0.

#### Scenario: Ahead and back
- **WHEN** the pin sits on its posed target and the offset is 40 cm straight ahead, then straight back, with an 80 cm step length, scale and level 1
- **THEN** the posed target moves 39.9 cm ahead, then 21 cm back

#### Scenario: A held step backwards
- **WHEN** footwork steps 50 cm behind the chest are laid whenever none runs, for three seconds on level ground
- **THEN** the body never loses its support and moves more than 50 cm back
