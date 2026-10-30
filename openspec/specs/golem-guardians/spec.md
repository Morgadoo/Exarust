# golem-guardians Specification

## Purpose
Tracks the golem guardians: how they wake and sleep, the alarm zones that
wake them, and the return that sends them home and puts the player back.

## Requirements

### Requirement: Guardian wake
Waking a guardian whose body rules lack bit `0x100` SHALL clear its race word's statue bit, its statue byte and its stance and pose words, keep its body awake 256 ticks, take its place and angle as home, set its glow material to full and pitch the wake sound at 0.9 plus a fifth of a draw; only while its sleep latch is set SHALL the latch clear, each limb take back its stored mass and each part's value be multiplied by 50. A wake SHALL install no behaviour.

#### Scenario: A latched guardian wakes
- **WHEN** a statue guardian whose latch is set, with limb scales a twentieth of their masses, is woken with a draw of 0.5
- **THEN** its race word loses the statue bit, its limbs take their masses, its part values are fifty times what they were, its latch is clear and the pitch is 1.0

#### Scenario: An unlatched guardian keeps its scales
- **WHEN** a guardian whose latch is clear is woken
- **THEN** its limb scales are unchanged

### Requirement: Guardian sleep
Putting a guardian to sleep SHALL set the statue bit of its race word and its statue byte, with stance 1 and pose 2, clear its condition word, install the dormant behaviour and fade its glow material out; the first time, each limb SHALL take its stored mass times 0.05 and each part its value times 0.02, both in double precision, and the latch SHALL be set.

#### Scenario: A second sleep keeps the scales
- **WHEN** a guardian already latched is put to sleep
- **THEN** its limb and part scales are unchanged

### Requirement: Guardian radius query
The radius query SHALL offer a character to its callback when its record's race word carries the AI bit `0x1000` and the squared distance of its x and z from the point is strictly under the radius squared, height not counted, and SHALL count the characters the callback accepts. The live query SHALL walk the characters standing in the room that holds the point and in the rooms behind that room's exits. A sector index SHALL be kept when the whole index is from zero to under the sector count.

#### Scenario: Height does not count
- **WHEN** a character stands 5000 cm above the point
- **THEN** it is offered at a radius of 800

#### Scenario: The edge is outside
- **WHEN** a character stands exactly 800 cm away
- **THEN** it is not offered at a radius of 800

### Requirement: Alarm trigger
Ringing an alarm zone SHALL arm it with its box centre and pass item unless another zone is armed and the player does not carry the armed zone's pass, which keeps the armed zone. The wake callback SHALL refuse a guardian whose statue byte is clear; it SHALL accept any other, wake it, and make its contact list the player alone at a concern of 10. A ring that woke any guardian SHALL count one more alarm.

#### Scenario: Another zone is refused
- **WHEN** the zone of pass 474 is armed and the player rings the zone of pass 476 without carrying token 474
- **THEN** the zone of pass 474 stays armed

#### Scenario: An exempt guardian is accepted
- **WHEN** a statue guardian whose body rules carry `0x100` is offered
- **THEN** it keeps its statue byte, its only contact is the player at concern 10 and the ring counts

### Requirement: Alarm return
While an alarm is armed, the alarm return SHALL offer the characters within 3000 cm of the player (x and z, strictly inside, AI-bit race words) in the player's room and the rooms behind its exits to the homeward callback, which SHALL refuse a guardian whose statue byte is set or whose condition carries the dying bit `0x800` and send any other home. The alarm SHALL stay armed.

#### Scenario: Every guardian asleep
- **WHEN** the alarm is armed and every guardian near the player is a statue
- **THEN** none goes home, the player stays where it fell and no draw is taken

#### Scenario: A dying guardian stays
- **WHEN** an awake guardian's condition carries `0x800`
- **THEN** the homeward callback refuses it

### Requirement: Homeward callback
A guardian sent home SHALL have its home filled from the body's place and angle when the home's x is exactly 0, its tone, stance factor and a third word set to 1 and its condition 0, ask for Stand 2, be put at its home facing its home angle and fall asleep.

#### Scenario: An unset home
- **WHEN** a guardian whose home's x is 0 is sent home
- **THEN** its home becomes where it stands, facing its angle

### Requirement: Player post after the alarm return
When the alarm return sent a guardian home, the first `Alarm_` node whose box holds the armed point (squared distance under 0.1) SHALL place the player 200 cm past the box's face along its third axis, facing the axis's heading plus π plus a draw less one half, asking for motion `0x1d`, with its condition dead and its tone and stance factor 0.

#### Scenario: The post past the box
- **WHEN** the alarm box is centred at (100, 50, 0) with a third half extent of 25 along z and the draw is 0.5
- **THEN** the player is put at (100, 10, 225) facing π

#### Scenario: The exanima06 gate
- **WHEN** the gate of pass 474 is armed, two awake golems stand within 3000 cm of the knocked-out player and the draw is 0.75
- **THEN** both go home asleep and the player is put at (700, 10, 6900) cm facing the third axis's heading −π/2 plus π plus 0.25

### Requirement: Alarm return in the story revive
The story revive of a knocked-out player (five seconds after its body died with red health of at least 0.01) SHALL run the alarm return after its prelude closes and before its save and the body's revive. A guardian sent home SHALL keep its pose about its root when it stands on its controller. With its statue byte set again, a guardian's sense SHALL clear its concern for the player, so it leaves combat.

#### Scenario: Golems sent home in play
- **WHEN** the player walks into the gate of pass 474 without its token, the two golems that wake fight it, and it falls with red health left
- **THEN** five seconds later both golems stand at their homes as statues at rest and the player lies 200 cm past the gate box's third-axis face

#### Scenario: The revive's order
- **WHEN** the story revive runs
- **THEN** the alarm return comes after the prelude closes and before the save and the body's revive

### Requirement: Alarm reset
A level start SHALL disarm the alarm, put its point 1.1·10⁶ cm over the origin and zero the alarm count.

#### Scenario: A new level
- **WHEN** a level starts
- **THEN** no zone is armed and the alarm count is 0

### Requirement: Alarm zones in the level
A level SHALL yield as alarm zones its model-less placed objects whose record's volume section has class bits `0xF00` equal to `0x100` and low byte 2, each a box centred on the object's origin with the section's three half extents and the placement's rotation rows as axes, its radius the extents' length, and its pass item the section's word at `0x10`. Every shipped level SHALL hold none but `exanima06`, which holds 14, passed by the items 474 to 479.

#### Scenario: The exanima06 gate
- **WHEN** `exanima06`'s alarm zones are read from the installation
- **THEN** there are 14, and the one centred at (1050, 100, 6900) cm has half extents (170, 100, 150), its first axis along +z and pass 474

### Requirement: Zone check
Each frame, once the player has moved more than 3 cm (squared distance strictly over 9) from the point its zones were last checked at, the level's alarm zones SHALL be checked in placement order and the point SHALL move to the player. A zone SHALL be entered when the player is not farther from its centre than its radius, not more than 0.1 (squared, in double) from its box, and the checked point was not under 0.1 from it; a NaN SHALL pass each comparison.

#### Scenario: Under three centimetres
- **WHEN** the player steps 2 cm from its checked point across the gate's face
- **THEN** the zones are not checked until it has moved past 3 cm

### Requirement: Crossing an alarm zone
An entered alarm zone SHALL end the check while the player's state flags carry `0x20000`, SHALL be crossed when the player carries its pass item in the inventory or in a carried container at any depth, and SHALL ring the alarm otherwise.

#### Scenario: Walking into the gate
- **WHEN** the player, carrying no token, walks from 6600 to 6750 cm along z into the gate of pass 474 with a statue golem 618 cm from its centre and another 900 cm away
- **THEN** the first golem wakes with the player as its only contact at concern 10 and its home where it stands, the second stays a statue, and the alarm is armed at the gate's centre with one ring

#### Scenario: The token lets the player through
- **WHEN** the player carries token 474 and walks into its gate
- **THEN** no golem wakes and the alarm stays disarmed

### Requirement: Golem bodies
The game SHALL build a simulated body for a placed character whose race byte is of the human frame (0 to 2) or a golem's (bit `0x10`, not the skeleton or the wraith), from its race model's bones and the shared skeleton's links, with its race's stature and the character init's muscles; a golem SHALL get a mind and a duellist like a human and SHALL wear no human apparel.

#### Scenario: The golem races are physical
- **WHEN** the race bytes 16 to 20 are tested
- **THEN** each is built physical, and none is dressed

#### Scenario: Other races stay static
- **WHEN** the ogre (byte 3) or the gobbler (byte 8) is tested
- **THEN** neither is built physical

### Requirement: Guardian start motion
A placed character without a saved motion SHALL start in Stand 2 (motion 0x19) when its record's race word carries the AI bit `0x1000`, and in Idle (motion 0x13) otherwise.

#### Scenario: A golem stands guard
- **WHEN** a character whose race word is `0x1000` with the statue bit is placed
- **THEN** it starts in Stand 2

#### Scenario: A human idles
- **WHEN** a character whose race word lacks the AI bit is placed
- **THEN** it starts in Idle

### Requirement: Sentry rest
Each tick a brain stands sentry out of combat, a statue whose body plays the motion it asks for SHALL rest (pose word 2, its body counting its sleep down by one a tick with the dormant retention share), any other body SHALL stand (pose word 1), and a body not asking for Stand 2 SHALL ask for it.

#### Scenario: A statue rests in Stand 2
- **WHEN** a statue standing sentry plays Stand 2
- **THEN** it rests

#### Scenario: A woken guardian stands
- **WHEN** a guardian whose statue byte is clear stands sentry playing Idle
- **THEN** it does not rest and it asks for Stand 2

### Requirement: Statue sleep scales
The first sentry tick of a statue whose state flags lack bit `0x400` SHALL set the bit, make every part's contact mobility 0.05 of its base and then each held body's 0.02 of that, both in double precision. While the bit is set, blows on the body SHALL be turned.

#### Scenario: The scales come once
- **WHEN** a statue runs the sentry tick twice with its latch clear at first
- **THEN** its latch is set, its pelvis's contact mobility is 0.05 of its base and the second tick changes nothing

### Requirement: Statue wake
A wake SHALL end a statue's rest at once and, while its bit `0x400` is set, clear the bit, give every part its contact mobility base back and multiply each held body's by 50.

#### Scenario: A woken statue stands
- **WHEN** a resting, latched statue is woken by an alarm
- **THEN** it no longer rests, its latch is clear, its pelvis takes its base back and its fist fifty times its base
