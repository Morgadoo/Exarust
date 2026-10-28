## MODIFIED Requirements

### Requirement: Attaching the brain
Without the race word's AI flag no contact sense SHALL be installed. With it, the sense SHALL be the standard one for race class `0x12xx` and the ally sense otherwise; a dead character SHALL get the effect at 0 and its roles SHALL NOT be applied; a statue race SHALL stand sentry with the effect at 0; any other SHALL get the effect at 1.

#### Scenario: Ally, dead and statue
- **WHEN** brains are attached to a class-`0x12xx` body, a class-`0x04xx` body, a dead body and a statue body, all with the AI flag
- **THEN** the first has the standard sense and the effect at 1, the second the ally sense, the third the effect at 0 and no roles applied, and the fourth is a statue with the effect at 0

### Requirement: Perception fills the brain's contacts
Once a body tick each brain SHALL clear its contacts' concern targets, closeness, view and noticed flag, then perceive, through the sense it holds, every character within the largest perception reach that the level's sight test lets it see and every character it already knows, and SHALL write the perception's sighting, closeness, attitude, threat and concern target into its contact for that character. A second frame on the same body tick SHALL perceive nothing.

#### Scenario: Visible and unseen
- **WHEN** the player stands two metres ahead, then is made unseen
- **THEN** the contact's closeness is 1 − 200/1200, then its sighting and closeness are 0

#### Scenario: Once a tick
- **WHEN** a contact's noticeability is cleared after a perception and the frame repeats on the same body tick, then the tick advances
- **THEN** it stays 0 on the same tick and is written again on the next

### Requirement: The AI tick
Once a body tick after the perception each brain SHALL decay its group alert, stop there when its body is out of the fight (condition bit `0x800`), and otherwise drift its morale, integrate its contacts and run the target pass over every character seen this tick, each contact's way out answered from the brain's room, its exits and its fled rooms away from that contact; it SHALL keep the report on the brain and copy its concern and hostility to the combatant's contacts.

#### Scenario: One pass a tick
- **WHEN** a brain holds a seen contact with hostility 5 for the player
- **THEN** one tick's pass enters combat at level 3 and attends the player, and a cooled contact the next tick leaves it at level 0

## ADDED Requirements

### Requirement: The sight test
A brain SHALL see a character in a level without a grid always; in a grid level when both stand in the same room, or when a door between their rooms, open in the room tracker, has its point (¾ of the way into the cell behind it) within 2 m of the other character. A position in no room SHALL be taken as seen.

#### Scenario: Through a door
- **WHEN** a brain in a two-room grid looks at a character in its own room, one 1.5 m from the door's point behind it, one two cells past the door, and the first again with the door shut
- **THEN** it sees the first two, not the third, and not the second once the door is shut

### Requirement: The level room
The AI tick SHALL read the brain's level room from the level's cell-to-room table. A cell without a room SHALL set the room to −1 and keep the previous room; a new room SHALL make the old one the previous room only when the old one was a room.

#### Scenario: Through a roomless cell
- **WHEN** a brain enters room 2, a roomless cell, room 5 and room 3
- **THEN** the previous room stays −1 until room 3, which remembers 5

### Requirement: The out-of-combat look chain
Out of combat, a body stepping or attacking SHALL copy its look into its look smoothing and heading. Otherwise the look yaw SHALL turn a tenth of the gap to the eased offset (held within a quarter turn), the pitch ease 1 % toward the target pitch, the smoothing ease from the facing by 0.4 of the gap (0.64 of its sign past 1.6 rad), and the gait helper SHALL rewrite the body's heading when it lags the eased look by more than 30°.

#### Scenario: A tenth and 0.4
- **WHEN** a body looking along yaw 0 with facing 0 has an eased offset 0.3 and a target pitch 0.5
- **THEN** the look yaw becomes 0.03, the pitch 0.005 and the smoothing yaw 0.012
