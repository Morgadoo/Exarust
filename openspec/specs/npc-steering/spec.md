# npc-steering Specification

## Purpose
How an NPC follows a path: the heading and desired velocity it writes, the same fields the player writes.

## Requirements

### Requirement: Path order
Following a path SHALL write a heading and a desired velocity. Along a straight path the heading already faces, the velocity SHALL be the player's walk speed, or the player's jog speed when the urgency is at least 1. A path of one point SHALL leave the velocity at zero.

#### Scenario: Straight walk
- **WHEN** the path runs 8 m forward and the heading already faces it
- **THEN** the velocity is 1.4 m/s along that heading

#### Scenario: Urgent
- **WHEN** the same path is followed with urgency 1
- **THEN** the velocity is 3.6 m/s along that heading

### Requirement: Duellist uses the order
A duellist closing on its target SHALL set its body's desired velocity to the order for the straight path from itself to the target.

#### Scenario: Closing in
- **WHEN** a duellist 8 m from its target has entered combat
- **THEN** its desired velocity equals that order and it plays the combat walk

### Requirement: Door on the path
A walker with no path, a cursor at the end, or no door SHALL keep walking. A door more than 0.7 open SHALL be left alone. A door at 0.7 or below SHALL hold the walker. If that door is not already toggling, the walker SHALL toggle it. A door already toggling SHALL stay as it is.

#### Scenario: A shut hinge holds, an open slider does not
- **WHEN** a walker with no path meets a shut hinge, a walker meets a slider at travel 0.29, a walker meets a shut hinge, a walker meets a slider at travel 0.3, and a walker meets a shut hinge that is already toggling
- **THEN** the first two keep walking and those doors stay as they were, the shut hinge holds the walker and aims open for 100 ticks, the slider at 0.3 holds the walker and opens for 60 ticks, and the hinge already toggling holds the walker without a new aim

### Requirement: Door wait
A walker not at a door SHALL stay as it is, door included. A door that lets the walker through SHALL move it past the door and let the search run. At exactly ten seconds the wait SHALL still hold. One millisecond later the walker SHALL seek again, clear the avoided node, and skip the cursor's low five bits. When both happen, seeking SHALL win and the search SHALL already be allowed. A start near the top of the clock SHALL not give up at a small now. Heading sway is not in this rule.

#### Scenario: Ten seconds still waits, the next millisecond seeks
- **WHEN** a walker already seeking meets a shut hinge one millisecond after ten seconds, a waiting walker meets a shut hinge at cursor 3 at exactly ten seconds and again one millisecond later, a waiting walker meets a slider at travel 0.29 at the start of the wait, that slider is met again at cursor 32 one millisecond after ten seconds, and a wait that started near the top of the clock meets the same slider at a small now
- **THEN** the seeking walker and its hinge stay as they were, the ten-second hinge is toggled open and the wait still holds with the search held and the avoided node kept, the next millisecond seeks with the avoided node cleared and bit 3 skipped while the search stays held and the hinge is not toggled again, the early slider moves the walker past the door and lets the search run without changing the door, the late slider seeks with the search allowed, the avoided node cleared and bit 0 skipped, and the wrapped clock moves past the door without seeking

### Requirement: Span arrival
A walker not approaching a span SHALL stay as it is. While the clock is still at the stamp and the walker is busy, the approach SHALL stay as it is. Otherwise a missing span, or a horizontal distance past 250 cm from the midpoint, SHALL send the walker back to seeking. The first miss SHALL set only the high skipped bit. A later miss SHALL also skip the cursor's low five bits. A distance of 250 cm SHALL start the door wait at the current clock. The aim snap after the choice is not in this rule.

#### Scenario: Two hundred fifty centimetres starts the wait
- **WHEN** a walker already at a door is stepped far past a span, a busy approach at its stamp is stepped past the span, an approach at its stamp that is not busy is stepped at exactly 250 cm, a busy approach one millisecond later meets no span at cursor 3, that approach meets a point 251 cm out again, and an approach meets the midpoint one millisecond later while busy
- **THEN** the walker at the door stays there, the busy approach at its stamp stays there with no skip, the point at 250 cm starts the door wait at that clock, the missing span seeks and sets only the high bit, the later miss also skips bit 3, and the midpoint starts the door wait at the later clock

### Requirement: Wander look
A look that is not wandering SHALL stay as it is. A wandering look SHALL copy the heading and start from the extracted angle. At the clock the offset and the roll SHALL stay, and the look SHALL ease toward the heading. Behind the clock the offset SHALL be the unit times 3.6 minus 1.8, the roll SHALL step, and the next timer SHALL add 4000 plus the high part of that step times 4000. An aim past half a turn SHALL wrap once. Posture 1 SHALL ease by 0.03. The flag call is not in this rule.

#### Scenario: At the clock the offset stays, behind it the offset is drawn
- **WHEN** a look that is not wandering is stepped behind its timer, a wandering look at heading 0 is stepped at its clock from extracted angle 1, the same look is stepped one millisecond later from unit 1, and a heading of 4 at posture 1 is stepped at its clock
- **THEN** the other look stays as it was, the on-time look is 0.91 with its offset and roll kept, the late look has offset 1.8, roll 134775814, timer 5126 and look 0.162, and the heading of 4 eases to about −0.0685

### Requirement: Aim snap
An aim snap SHALL copy the body's place and set the heading, the stored heading and the base heading to the yaw of the body's heading, ignoring its height. The roll SHALL step. The timer SHALL be that step's high part times 3000, plus the clock, plus 1000. The pace and the marker SHALL clear. The stance SHALL become idle: the old clip starts Idle over 30 frames, another clip stays, and Idle playing does not restart. A wander look at that clock SHALL keep its offset.

#### Scenario: A defence clip becomes idle, a walk does not
- **WHEN** a defence stance playing its own clip is snapped from a heading whose height is 9 and whose forward part is 1, at clock 2000 with roll 1, and a wander look is then stepped at that clock, and a defence stance playing a walk is snapped from a heading along the side, and an idle stance already playing Idle is snapped from a heading straight back
- **THEN** the place is copied, the yaw is 0, the roll is 134775814, the timer is 3094, the pace and the marker are clear, Idle plays over 30 frames, the look keeps its offset, the side heading's yaw is about 1.5708 and the walk clip stays, and the backward yaw is about 3.1416 with Idle left as it was

### Requirement: Start a long path
A path of fewer than two points SHALL NOT be followed, even when forced. Two points SHALL be followed when forced, or when the first and last are more than 150 cm apart. Exactly 150 cm SHALL NOT be followed.

#### Scenario: Exactly 150 cm stays, one centimetre more follows
- **WHEN** an empty path is forced, a single point is forced, two points 150 cm apart are not forced, two points 151 cm apart are not forced, and two points on top of each other are forced
- **THEN** the empty path and the single point are not followed, the 150 cm path is not followed, and the 151 cm path and the forced path are followed

### Requirement: Path carrot
A ground point SHALL project onto the nearest path segment and keep the unclamped fraction. The goal SHALL be 100 cm further along that path, measured from the unclamped foot. An equal distance SHALL keep the earlier segment. A vertical segment SHALL be skipped. Exactly 100 cm left SHALL spill, and a zero step SHALL have no goal.

#### Scenario: One hundred centimetres past the foot
- **WHEN** a ground point 50 cm behind a 200 cm segment is projected, a point on a corner is projected onto a path that then rises, a point at the top of a vertical segment is projected, and a point at the start of a 100 cm segment with another 100 cm after it is projected
- **THEN** the foot behind is fraction -0.25 and the goal is 50 cm along, the corner keeps the first segment and the goal is 20 cm up and 100 cm along the next, the vertical segment is skipped and the goal is the last point, and the exact 100 cm has no goal while its cursor stays at the start

### Requirement: Out of scene
When the low two scene bits are clear, a step SHALL advance 2 cm along the segment. The standing height SHALL add the pelvis base, and the ground point SHALL keep the path height. A length that is not strictly past the fraction times the length plus 2 SHALL cross once. The new fraction SHALL be (1 - (1 - fraction) times the length) times (1 - fraction), minus 2, divided by the next length. No horizontal length SHALL count as 0.1. A set low bit SHALL not slide.

#### Scenario: Two centimetres, or one crossing
- **WHEN** a body out of the scene is at the start of a 100 cm segment with pelvis base 90, another has fraction 0.75 on an 8 cm segment with 8 cm after it, another is at the start of a vertical segment with 20 cm after it and pelvis base 4, another is 1 cm from the end of a last segment, and bodies with a low scene bit set are stepped, including one whose only set bit is higher
- **THEN** the first fraction is 0.02, the ground is 2 cm along, and the standing height is 90, the second fraction is -0.28125 and the ground is 5.75 cm along, the vertical crosses onto the next segment behind its start and stands 4 above the path, the last segment stops at its end, a low bit does not slide, and the higher bit still slides
