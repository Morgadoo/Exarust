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
