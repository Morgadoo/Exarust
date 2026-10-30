# Spec Delta

## ADDED Requirements

### Requirement: The sense the perception runs
The perception pass SHALL run, for each brain, the sense its race installed: the base perception (the role database's attitude, the group blend, the threat estimate and the familiarity drift) for a race without the AI bit, the standard sense for an AI race of class `0x12xx`, and the ally sense for the other AI races.

#### Scenario: A Golem Suit sees the player
- **WHEN** a brain holding the standard sense, with concern 2 for the player, perceives the player standing 200 cm ahead and looking away
- **THEN** the contact is tracked with attitude 1 and concern 0, closeness 1 − 200/1200 and noticeability `2·(2c⁴ + (1 − c²)c)`

#### Scenario: A human keeps the base perception
- **WHEN** a brain is placed from a human record
- **THEN** it installs no sense and perceives through the base perception

### Requirement: An out-of-fight brain senses nothing
A brain whose body is out of the fight (condition bit `0x800`) SHALL perceive nothing in the perception pass.

#### Scenario: Knocked out
- **WHEN** a brain whose body is dead faces the player two metres ahead, then stands again
- **THEN** it holds no contact for the player, then tracks the player

### Requirement: The target pass classifies before the drift
Within the AI tick, the brain SHALL take its aim point and freshness, then run the target pass on the contacts as the perception left them, and only then drift its contacts' hostility, concern, noticed level and attention, with the level, attention and stress the pass wrote.

#### Scenario: Classified before the drift
- **WHEN** a brain with no tolerance and no stress holds a seen contact with hostility 2.0015
- **THEN** the pass engages at level 3 and the drift then leaves the hostility under 2

### Requirement: The standard sense's statue observer
The standard sense SHALL take its lying branch for an observer standing as a statue, and SHALL NOT take it for an observer that is merely asleep.

#### Scenario: A statue observer
- **WHEN** an observer standing as a statue sees a live body facing it at 600 cm with noticed level 1, and an asleep observer that is no statue sees it
- **THEN** the first contact's noticed level is 0.5 and its noticeability 0, and the second keeps noticed level 1 with noticeability 2.25

## MODIFIED Requirements

### Requirement: The race word picks the per-contact sense
A brain placed from a character record SHALL read the record's profession word as its race word. A race word with the AI bit `0x1000` SHALL install the standard sense when its class byte is `0x12`, and the ally sense otherwise; a race word without it SHALL install no sense and keep the base perception.

#### Scenario: Guard, suit and human
- **WHEN** brains are placed from records of race byte 16, race byte 18 and race byte 0
- **THEN** the first takes the ally sense, the second the standard sense and the third the base perception

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

### Requirement: The standard sense's sighting
A dead or asleep body, or any body seen by an observer standing as a statue, SHALL get noticeability 0, its noticed level capped at 0.5 and its concern, hostility and selected character cleared. A down body SHALL get 24·closeness, its concern times 0.99 in double while knocked out. A live one SHALL get `((look·u·0.5 + 0.5)²·0.5 + 1)·((facing·u·0.5 + 0.5) + 2)·(2c⁴ + (1 − c²)c)`, 10 more while it attacks. The view SHALL be `(look·u·−0.5 + 0.5)²·0.25 + (forward·u·−0.5 + 0.5)·0.75`.

#### Scenario: A body facing the observer at 600 cm
- **WHEN** a live body 600 cm away looks and faces straight at an observer that looks straight at it
- **THEN** the closeness is 0.5, the view 1 and the noticeability 2.25
