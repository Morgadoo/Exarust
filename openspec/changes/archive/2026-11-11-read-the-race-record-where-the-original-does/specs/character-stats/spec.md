## ADDED Requirements

### Requirement: Core crystal race lookup
The body equipment panel's core test SHALL read the first flag word of the race whose id is the record's race byte plus one, and the built-in record's 0 when the table lacks it: `0x2000` takes only Power Crystal AL, `0x1000` only crystal A, neither while one of the five crystals is worn.

#### Scenario: Shipped golems
- **WHEN** crystal A and crystal AL are offered to bodies of race bytes 16, 17, 18, 19 and 20
- **THEN** bytes 16, 19 and 20 take only crystal A and bytes 17 and 18 only crystal AL, and race byte 0 takes neither

### Requirement: Dressing by the race look word
A character's worn items SHALL be put on its body only when its race's look word has bit `0x4`; a race the table lacks (look 0) SHALL not be dressed.

#### Scenario: Shipped races
- **WHEN** the shipped race table is read
- **THEN** race bytes 0, 1, 2, 3 and 9 (human, skeleton, ancient, ogre, manipulator) are dressed and no other race is

### Requirement: Race mind word
A character's mind kind for the Mind casts SHALL be its race record's word at `0xEC` (1 or 2 living, 4 or 5 undead, anything else no mind), and the built-in record's 0 for a race the table lacks.

#### Scenario: Calm on other races
- **WHEN** Calm is cast at a skeleton, a ghoul and a golem guard
- **THEN** the skeleton is calmed, the ghoul answers 0 (spent, no effect) and the golem refuses

### Requirement: Underwear by race
The underwear rule SHALL put the underwear on a marked body only when its race byte is 0; a body without a race is a human's.

#### Scenario: A marked ogre
- **WHEN** a marked body of race byte 3 wears a shirt
- **THEN** no underwear is put on
