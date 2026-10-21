# golem-guardians Specification

## Purpose
Tracks the golem guardians: how they wake and sleep, the alarm zones that
wake them, and the return that sends them home and puts the player back.

## Requirements

### Requirement: Guardian wake
Waking a guardian whose body is not exempt SHALL show its node, clear its dormant, stance and pose words, keep its body awake for 256 ticks, take the body's place and angle as its home, set its glow material to full and give a wake sound pitch of 0.9 plus a fifth of a draw; only while the body's sleep latch is set SHALL the latch clear, each limb take back its stored mass and each part's value be multiplied by 50. A wake SHALL install no behaviour.

#### Scenario: A latched guardian wakes
- **WHEN** a hidden guardian whose latch is set, with limb scales a twentieth of their masses, is woken with a draw of 0.5
- **THEN** its node shows, its limbs take their masses, its part values are fifty times what they were, its latch is clear and the pitch is 1.0

#### Scenario: An unlatched guardian keeps its scales
- **WHEN** a guardian whose latch is clear is woken
- **THEN** its limb scales are unchanged

### Requirement: Guardian sleep
Putting a guardian to sleep SHALL hide its node, mark it dormant with stance 1 and pose 2, clear its alarm word, install the dormant behaviour and fade its glow material out; the first time, each limb SHALL take its stored mass times 0.05 and each part its value times 0.02, both in double precision, and the latch SHALL be set.

#### Scenario: A second sleep keeps the scales
- **WHEN** a guardian already latched is put to sleep
- **THEN** its limb and part scales are unchanged

### Requirement: Guardian radius query
The radius query SHALL offer a body to its callback when the body's node carries the query flag and the squared distance of its x and z from the point is strictly under the radius squared, height not counted, and SHALL count the bodies the callback accepts. A sector index SHALL be kept when the whole index is from zero to under the sector count.

#### Scenario: Height does not count
- **WHEN** a body stands 5000 cm above the point
- **THEN** it is offered at a radius of 800

#### Scenario: The edge is outside
- **WHEN** a body stands exactly 800 cm away
- **THEN** it is not offered at a radius of 800

### Requirement: Alarm trigger
Checking an alarm zone SHALL arm it with its point and number unless another zone is armed and the test of the player against the armed zone answers false, which keeps the armed zone. The wake callback SHALL accept any dormant guardian and give it the player as a target with a radius word of 10.

#### Scenario: Another zone is refused
- **WHEN** zone 4 is armed, zone 5 is checked and the test answers false
- **THEN** zone 4 stays armed

### Requirement: Alarm return
Every awake guardian within 3000 cm of the player whose body is not already returning SHALL be put back at its home, its home filled from the body's place and angle when the home's x is exactly 0, its speed words set to 1, and SHALL fall asleep. When one did, the `Alarm_` node whose box holds the armed point (squared distance under 0.1) SHALL place the player 200 cm past the box's far face along its third axis at a height of 10, facing the axis's heading plus π plus a draw less one half.

#### Scenario: The post past the box
- **WHEN** the alarm box is centred at (100, 50, 0) with a third half extent of 25 along z and the draw is 0.5
- **THEN** the player is put at (100, 10, 225) facing π
