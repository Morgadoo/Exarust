# Spec Delta

## ADDED Requirements

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
