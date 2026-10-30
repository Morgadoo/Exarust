# Spec Delta

## ADDED Requirements

### Requirement: The race word picks the per-contact sense
A brain placed from a character record SHALL read the record's profession word as its race word. A race word with the AI bit `0x1000` SHALL install the standard sense when its class byte is `0x12`, and the ally sense otherwise; a race word without it SHALL keep the behaviour base's perception.

#### Scenario: Guard, suit and human
- **WHEN** brains are placed from records of race byte 16, race byte 18 and race byte 0
- **THEN** the first takes the ally sense, the second the standard sense and the third the default

### Requirement: Statues and fellow AIs
The statue bit `0x08000000` with the AI bit SHALL mark the brain a statue, whose ally sense clears the contact's noticed level and concern and ends. The ally sense SHALL take an observed character for a fellow AI only when its race word carries the AI bit.

#### Scenario: A statue guard and a human
- **WHEN** brains are placed from a record of race byte 16 with the statue bit and from a human record
- **THEN** the first is a statue carrying the AI mark and the second carries no AI mark

### Requirement: The standard sense's range
The standard sense SHALL clear the contact's threat word, end with 0 for a contact that is not a character, and end with −1 for a character that cannot be perceived. Out of 1200 cm (1800 cm while tracked, a strict squared compare ignoring height differences under 150 cm) it SHALL clear the tracked flag and end with −1. In range it SHALL set the tracked and noticed flags and write the closeness `1 − d/1200`, at most 0.99 compared in double.

#### Scenario: Exactly at the range
- **WHEN** an untracked character stands exactly 1200 cm away
- **THEN** the sense ends with −1 and the contact's flags stay 0

### Requirement: The standard sense's sighting
A dead or asleep body, or an asleep observer, SHALL get noticeability 0, its noticed level capped at 0.5 and its concern, hostility and selected character cleared. A down body SHALL get 24·closeness, its concern times 0.99 in double while knocked out. A live one SHALL get `((look·u·0.5 + 0.5)²·0.5 + 1)·((facing·u·0.5 + 0.5) + 2)·(2c⁴ + (1 − c²)c)`, 10 more while it attacks. The view SHALL be `(look·u·−0.5 + 0.5)²·0.25 + (forward·u·−0.5 + 0.5)·0.75`.

#### Scenario: A body facing the observer at 600 cm
- **WHEN** a live body 600 cm away looks and faces straight at an observer that looks straight at it
- **THEN** the closeness is 0.5, the view 1 and the noticeability 2.25

### Requirement: The standard sense's threat
The player's body SHALL clear the concern and set the attitude to 1. For any other live body the sense SHALL weigh each character contact of the observed, softened by `min((concern + hostility)·0.15, 1)`. The heaviest SHALL select the observer's own contact at that entry's position. Only a third party that is the observer (times the multiplier and 0.5) or the player (times the multiplier) SHALL raise the concern target to the largest such value.

#### Scenario: A threat to the observer
- **WHEN** the observed body's only contact is the observer, at closeness 0.5, view 1 and concern 2, with multiplier 8
- **THEN** the concern target is 0.102589 and the selected character is the observer's contact at position 0

### Requirement: The ally sense's entry
The ally sense SHALL clear the contact's hostility and threat words before anything else.

#### Scenario: A non-character contact
- **WHEN** the ally sense runs on a contact with hostility 2 that is not a character
- **THEN** its hostility and threat are 0 and the sense ends with 0
