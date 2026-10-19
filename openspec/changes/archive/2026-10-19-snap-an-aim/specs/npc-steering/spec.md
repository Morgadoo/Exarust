# Spec Delta

## ADDED Requirements

### Requirement: Aim snap
An aim snap SHALL copy the body's place and set the heading, the stored heading and the base heading to the yaw of the body's heading, ignoring its height. The roll SHALL step. The timer SHALL be that step's high part times 3000, plus the clock, plus 1000. The pace and the marker SHALL clear. The stance SHALL become idle: the old clip starts Idle over 30 frames, another clip stays, and Idle playing does not restart. A wander look at that clock SHALL keep its offset.

#### Scenario: A defence clip becomes idle, a walk does not
- **WHEN** a defence stance playing its own clip is snapped from a heading whose height is 9 and whose forward part is 1, at clock 2000 with roll 1, and a wander look is then stepped at that clock, and a defence stance playing a walk is snapped from a heading along the side, and an idle stance already playing Idle is snapped from a heading straight back
- **THEN** the place is copied, the yaw is 0, the roll is 134775814, the timer is 3094, the pace and the marker are clear, Idle plays over 30 frames, the look keeps its offset, the side heading's yaw is about 1.5708 and the walk clip stays, and the backward yaw is about 3.1416 with Idle left as it was
