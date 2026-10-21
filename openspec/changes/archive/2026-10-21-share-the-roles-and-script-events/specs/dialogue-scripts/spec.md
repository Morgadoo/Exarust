## MODIFIED Requirements

### Requirement: Actor built-ins
The `Actor` class SHALL offer its 22 methods (`Role` … `HasItem`), the writable `Activity` and `IdleActivity` (read back as 0) and the text members `Name`, `ContextPhrase`, `ContextObj`. `Health`, `Trust` and `Concern` SHALL round to nearest even; `AddConcern` SHALL keep concern within 0 to 15; `Trust` of oneself SHALL be 100.

#### Scenario: Health and concern
- **WHEN** the pool is 0.125 of its maximum, and a concern of 13 gets 10 added
- **THEN** `Health` gives 12 and the concern becomes 15

#### Scenario: Trust on a tie
- **WHEN** the attitude toward the partner is −30.5 and the concern 2.5
- **THEN** `Trust` gives −30 and `Concern` 2

### Requirement: Topic activation
An activated node SHALL need its hits (or distinct witnesses), open after its delay class (0, 2, 5, 10, 30, 300, 1800 s) and stay open for its window class. A stored block whose count differs from the dialogue's SHALL load from its count word: node 0 gets the stored count, node i the stored word i − 1, as many words as the block holds.

#### Scenario: Activation
- **WHEN** a node needing two hits with a 5 s delay and a 60 s window is hit at 100 and 101 s
- **THEN** it opens at 106 s and closes at 166 s

#### Scenario: A grown dialogue
- **WHEN** words 7, 8, 9 are stored and loaded for five nodes, then for two
- **THEN** the words are 3, 7, 8, 0, 0, then 3, 7

### Requirement: Speech bubbles
A line SHALL show for 1000 ms + 50 ms per byte, stretched by (2 − d/10 000) for the narrator under 10 s (rounded to nearest) and halved for the player; a speaker's next line SHALL wait 375 ms past its previous one's end; a character's line SHALL be cut at a CR; a bubble SHALL fade in over 300 ms and out over 1.5 s.

#### Scenario: Durations
- **WHEN** ten-byte lines are said by a character, the narrator and the player, and a one-byte line by the narrator
- **THEN** they last 1500, 2775, 750 and 1990 ms

### Requirement: Scripted objects and crystals
Using a scripted object SHALL run its script with the player as its one actor slot, at most once per 2.4 s across all such objects, with a 1.2 s glow pulse up and down; a socketed crystal SHALL lose 2 charge per game second down to 0 and its source SHALL leave the list once the socket no longer carries the charge flag.

#### Scenario: Use interval
- **WHEN** objects are used at 5000 ms, 7399 ms and 7400 ms
- **THEN** the first and third run their scripts and the second is refused

## ADDED Requirements

### Requirement: Mood tests
The mood of A toward B SHALL be a hostility code (hostile contact 0x600; an engaged contact 0x600 with concern above 4, 0x200 above 1.2) and an attitude band (above 70, above 30, below −30, else: 0x10, 0x20, 0x80, 0x40), the attitude first scaled by ((10 − x)/10)² unless the contact value x is above 10, which zeroes it. An unordered value SHALL fail every compare. The request's mood code SHALL use the same bands (0x1…, 0x2…, 0x4…, 0x3… in the top nibble) after the engaged and hostile codes.

#### Scenario: Unordered values
- **WHEN** the attitude is NaN, then an engaged contact's concern is NaN
- **THEN** the mood is 0x40 with code 0x30000000, then no hostility code is set

### Requirement: Topic events
`ActivateTopicN` SHALL activate the topic in the actor's memory of each contact holding the aimed-at role (for a relation role, held by the actor toward the contact); a witnessed event SHALL need the actor to hold the target (unless it is a relation role) and write each contact holding the dialogue's role. Both SHALL file under entry = target, key = role; a broadcast witness SHALL key a role-to-role block by its role.

#### Scenario: The memory flash
- **WHEN** the player (holding Player) raises the memory flash and a contact holds XA_Derrin
- **THEN** the contact's memory of the player has "XAU_Memory" activated under entry Player, key XA_Derrin

### Requirement: StartDialogue
`StartDialogue(topic)` SHALL start the named node between the script's first two actors, refused while the first actor's arousal is above 10 or it fills an actor slot of any active conversation, whatever that conversation's state.

#### Scenario: Already talking
- **WHEN** a script asks to start a dialogue for an actor that is in an ended but still active conversation
- **THEN** no dialogue starts

### Requirement: Role-set check
After a topic event stores a word, or an event node runs a branch, the actor's held roles SHALL be looked up in the role database and an unknown one reported ("Invalid role identifier.", or "Actor n invalid role identifier." for the event node's slot n).

#### Scenario: An unknown role
- **WHEN** a near activation stores a word for an actor holding a role the database lacks
- **THEN** "Near Activation: Invalid role identifier." is reported

### Requirement: Zone and locale tests
`InZone` and `InLocale` SHALL compare the locale and zone ids with the argument without wrapping, so a negative argument never matches. A zone marker SHALL match when the character's squared distance to it is within its radius squared and it stands at most 0.1 (squared) outside the marker's box.

#### Scenario: A negative zone
- **WHEN** `InZone` asks for zone −1 and a marker's id is all ones
- **THEN** it does not match
