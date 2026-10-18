# Spec Delta

## ADDED Requirements

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

## MODIFIED Requirements

### Requirement: Climbing with the controller
Until the feet step and the legs bend (Exarust's own), while a living body that has not fallen has its character controller on the ground, standing or moving, the target pelvis height SHALL read each foot's ground no lower than the controller's floor less 6 cm. A controller in the air or under a fallen body SHALL change nothing.

#### Scenario: Level ground
- **WHEN** a body walks or stands with its controller floating 0, 2 or 5 cm over a level floor
- **THEN** its target is the original's every tick: in `--autowalk play arena` the same as without the rule, the pelvis 99.4–102.2 cm walking and 101.5 cm standing

#### Scenario: Stairs
- **WHEN** a body walks up four 18 cm steps with its controller, stopping on the third
- **THEN** it keeps its balance and ends on the landing, its pelvis 72 cm higher; with the original's target alone the recovery step keeps its balance, its feet end on the landing and its pelvis ends at least 5 cm lower

#### Scenario: Stair room
- **WHEN** `exarust --autowalk --at 7.5,-0.95,64.4 play exanima01` or `--at 7.6,-1.3,66.2` walks the player up the stairs to the door
- **THEN** it climbs to the landing without falling on the stairs and stands there

#### Scenario: Perched
- **WHEN** a body's grounded controller stands 12 cm over the floor under its feet, or is in the air
- **THEN** its target is 6 cm higher than the original's, or the original's

#### Scenario: Fallen
- **WHEN** a body has just fallen, its feet on the floor and its grounded controller 12 cm over it
- **THEN** its target is the original's every tick, as with its controller on the floor
