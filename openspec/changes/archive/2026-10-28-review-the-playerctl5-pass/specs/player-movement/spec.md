## ADDED Requirements

### Requirement: Movement word persistence
The movement word SHALL be built with A as 2, D as 8, W as 1 and S as 4, A and D cancelling and W and S cancelling; a word pressed for less than 200 ms SHALL be kept until 200 ms after the press, and a word held longer SHALL clear on release. The analog stick SHALL replace the keys only when its squared length is over 0.01 in double precision, and a held word SHALL be flagged a tap after 200 ms only while the crouch is under 0.1.

#### Scenario: A tap and opposing keys
- **WHEN** W is pressed at 1000 ms and released at 1010 ms, and later A and D are held together
- **THEN** the word stays forward until 1200 ms and is clear at 1201 ms, and A with D gives no word

### Requirement: Camera follow step
When the camera follows the body, a camera yaw more than 0.1π from the body's yaw (the gap wrapped into ±π in double precision) SHALL become the body's yaw put back 0.1π on the camera's side, wrapped into ±π; within 0.1π it SHALL stay.

#### Scenario: Camera one radian off
- **WHEN** the camera's yaw is 1.0 and the body's 0.0
- **THEN** the camera's yaw becomes 0.1π

### Requirement: Level placement marker match
A level placement marker SHALL match an item whose object id, or its model's id, equals the marker's eight name characters read as hexadecimal up to the first other character; the cursor's segment (to the pick, or 10 000 cm without one) SHALL meet the marker's sphere, the item SHALL have an object id, and the ray SHALL not climb.

#### Scenario: Hexadecimal name
- **WHEN** a marker is named `00001A2f` and the held item's object id is 0x1a2f
- **THEN** a descending segment through the marker's sphere snaps the item onto it
