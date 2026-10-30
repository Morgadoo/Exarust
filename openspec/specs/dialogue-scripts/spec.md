# dialogue-scripts Specification

## Purpose
The role and dialogue script interpreter with its classes and globals, roles and relations, topic memory, the conversation engine and reply panel, speech bubbles, and scripted objects.

## Requirements

### Requirement: Script statements
The system SHALL run a role or dialogue script in one pass over its text: statements ending with `;`, terms with an optional `[n]` actor slot, `.member` calls on the `Actor` and `Role` classes, global calls, `result`, locals and decimal literals, and `? cond : stmt / stmt` with brace blocks. No text SHALL make it panic or hang.

#### Scenario: Both branches of an encounter script
- **WHEN** `? Actor[1].HasRole($E1D1) :` then a CRLF, `AddConcern(Actor[1], -100);`, a CRLF, `/`, a CRLF and `AddConcern(Actor[1], 10);` runs with concern 3
- **THEN** the concern is 13 without the role, then 15 (clamped), and 0 once the partner holds role 57809

### Requirement: Script conditions
Conditions SHALL compare with `=`, `!` (not equal), `<`, `>` or a bare value above 0, chain left to right with `&` and `|`, and compare actor references by identity; `!` SHALL flip a value's low bit.

#### Scenario: A chain
- **WHEN** `result = (1 = 1 & 2 = 3);` and `result = (1 = 2 | 2 = 2);` run
- **THEN** they give 0 and 1

### Requirement: The original's walk
The walk SHALL keep the original's behaviour where it differs from a parser: a false branch steps one byte past its `;`, the byte after a closing `)` is consumed, and a member name stops only at a space, `;`, `.` or `(`.

#### Scenario: A member name ends at a space
- **WHEN** `result = (Actor[1].Awake);` runs for an awake partner
- **THEN** it gives 0, since `Awake)` names no method

### Requirement: Role objects
`Actor[n].Role(id)` SHALL give the variables of that role on that actor, written back at once when assigned; `Role.` SHALL give the running role's variables on `Actor[0]`, written back when the run ends; an actor without the role SHALL skip the rest of the term.

#### Scenario: A chained role variable
- **WHEN** `result = (Actor[0].Role($2E).Activity ! 4);` runs with that variable at 2, then `Actor[0].Role($2E).Activity = 4;`, then the condition again
- **THEN** the condition gives 1, the variable is stored as 4 at once, and the condition then gives 0

### Requirement: Actor built-ins
The `Actor` class SHALL offer its 22 methods (`Role` … `HasItem`), the writable `Activity` and `IdleActivity` (read back as 0) and the text members `Name`, `ContextPhrase`, `ContextObj`. `Health`, `Trust` and `Concern` SHALL round to nearest even; `AddConcern` SHALL keep concern within 0 to 15; `Trust` of oneself SHALL be 100.

#### Scenario: Health and concern
- **WHEN** the pool is 0.125 of its maximum, and a concern of 13 gets 10 added
- **THEN** `Health` gives 12 and the concern becomes 15

#### Scenario: Trust on a tie
- **WHEN** the attitude toward the partner is −30.5 and the concern 2.5
- **THEN** `Trust` gives −30 and `Concern` 2

### Requirement: Script globals
The globals SHALL be `GameEvent` (hex), `CheckpointSave`, `StartDialogue`, `ChangeRole` (reported as missing), `EndDialogue`, `TopicUsed`, `WorldTime`, `ActivateTopic` and `WitnessRole`. `{…}` groups in a line SHALL be replaced by the text their member gives.

#### Scenario: A name in a line
- **WHEN** `{Actor[0].Name}, I need your help.` is spoken by Derrin
- **THEN** the line reads `Derrin, I need your help.`

### Requirement: Roles and relations
Adding a role SHALL walk its parent chain while the role met inherits: a held role asked for becomes explicit, a held parent gains a reference, new entries go in front. Removing an explicit role SHALL drop it and walk the chain, dropping parents held once and lowering the others.

#### Scenario: A golem sentry
- **WHEN** Golem Sentry (inherits Golem, which inherits Construct) is added, then Golem, then Golem Sentry is removed
- **THEN** the list is Sentry, Golem, Construct with one reference each; Golem turns explicit and Construct has two; only Construct with one reference is left

### Requirement: Topic cooldowns
A used node SHALL wait its cooldown class: 0 never again, 1 a year, 2 thirty days, 3 a week, 4 three days, 5 a day, 6 three hours, 7 an hour, 8 thirty minutes, 9 ten minutes, 10 five minutes, 11 a minute, 12 thirty seconds, 13 ten seconds, 14 none, 15 always.

#### Scenario: Ten minutes
- **WHEN** a class-9 node used at 1000 s is asked at 1599 s and 1600 s
- **THEN** it is closed, then open

### Requirement: Topic activation
An activated node SHALL need its hits (or distinct witnesses), open after its delay class (0, 2, 5, 10, 30, 300, 1800 s) and stay open for its window class. A conversation's memory SHALL be its count word followed by one word per node, node n at slot n. A stored block whose count differs SHALL load shifted: slot 1 gets the stored count, slot j the stored node word j − 1, the rest zero.

#### Scenario: Activation
- **WHEN** a node needing two hits with a 5 s delay and a 60 s window is hit at 100 and 101 s
- **THEN** it opens at 106 s and closes at 166 s

#### Scenario: A grown dialogue
- **WHEN** words 7, 8, 9 are stored and loaded for five nodes, then for two
- **THEN** the buffers are 5, 3, 7, 8, 0, 0, then 2, 3, 7

### Requirement: Dialogue search
A dialogue for owner A and partner B SHALL be sought through A's roles against B's roles, then A's relation roles toward B, then the entry aimed at 0, then the dialogues of A's relation roles, A→B before B→A, the first the purpose accepts winning.

#### Scenario: Derrin's topics
- **WHEN** the player, holding Subordinate toward Derrin, talks to him while he follows
- **THEN** Explorer's topics toward a subordinate are offered, except "Follow me"

### Requirement: Running a conversation
A conversation SHALL start at an entry node the calm filter and memory allow (or a named node), speak the first branch with a candidate line (one at random per speaker), take it, step half the bubble's duration after it is due, follow a branch's link when its replies run out, and end with the node recorded as used when the memory entry exists.

#### Scenario: A greeting
- **WHEN** a guard whose role aims a two-line greeting at Human greets a traveller at clock 0, and the clock reaches 626 and then 1502
- **THEN** the guard says "Halt." (due again at 625), the traveller answers at 626 (next step at 1501), and the conversation ends

#### Scenario: A remembered greeting
- **WHEN** the guard holds the memory entry, the greeting runs to its end at world time 1005, and the guard greets again at 1604 and 1605
- **THEN** the second greeting is refused and the third goes ahead

### Requirement: The reply panel and the narrator
The panel SHALL list the player's replies, then the topics, with `[Other]` between them when both exist; a pick SHALL show the player's line and take the reply or start the topic. The narrator SHALL speak a named node of its graph and offer the player's replies.

#### Scenario: The narrator
- **WHEN** the narrator starts at Corlian and the player picks the second reply
- **THEN** three replies are offered with the player's name substituted, the narrator's bubble closes, and game event 0xC10EC2 is raised

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

### Requirement: Bubble placement
A bubble SHALL be wrapped at 320 pixels and be `lines × line height − 2` tall. A narrator bubble SHALL be centred 75 % down the screen; a character bubble SHALL be centred on the head lifted 120 cm, taken through the rigid inverse view and the viewport projection, where behind the camera an ordered negative component picks the low edge and anything else the high edge. Both SHALL keep four units inside the screen.

#### Scenario: Projection and edges
- **WHEN** an identity view with the eye at z = 5 and scales 2, 2 projects (0, −120, 0), (0, −120, 10) and an all-NaN point on an 800 × 600 screen
- **THEN** the points are (400, 14700), (800, 600) and (800, 0)

### Requirement: Bubble drawing
A frame SHALL draw every showing bubble's box (radius 3, alpha 128 × fade, the narrator's grown by four units) before any text (white, alpha 160 × fade); waiting bubbles SHALL not be drawn.

#### Scenario: Draw order
- **WHEN** two bubbles are showing
- **THEN** one box batch with both boxes comes before both text layers

### Requirement: Calm filters
The greeting filter SHALL take calm class 0 only; the start and reply filter SHALL refuse class 0 and take a class from 1 while the stored arousal is at or under its threshold, an unordered arousal passing.

#### Scenario: Class zero
- **WHEN** a class-0 node and a class-1 node are tested at arousal 0, 10, 10.5 and NaN
- **THEN** the greeting takes only the class-0 node, and the start filter refuses it and takes the class-1 node at 0, 10 and NaN

### Requirement: Heard speech
A character's line SHALL give each side's contact of the other a stimulus of (bytes / 2 + 20) / 4: louder than the level, it SHALL take the level and clear the decay; otherwise louder than level − decay, it SHALL set the decay to level − amount. Unordered values SHALL change nothing.

#### Scenario: Floor
- **WHEN** a contact at level 5.5 and decay 4 hears 10
- **THEN** the decay becomes 3

### Requirement: Context object
`ContextObj` SHALL be "a " plus the item's object database name with A–Z lowered for a weapon (`kind & 0x7000 == 0x4000`) whose name is found, else "nothing".

#### Scenario: A sword
- **WHEN** a character holds a weapon named "Short Sword"
- **THEN** `ContextObj` is "a short sword"

### Requirement: Glow tasks and crystal walks
A used scripted object SHALL schedule a 1200 ms fade, 0 ms hold glow task; each frame a task SHALL write its level into the object's emissive mesh, emissive children and lights (base × level), and on the frame it ends it SHALL write 0 and be removed. A crystal stack SHALL lose 2 charge down to 0, reporting a change only when it was above 2; a speaker's candidate lines SHALL be the first six.

#### Scenario: The last glow frame
- **WHEN** an object used at 5000 ms is ticked at 5600, 7400 and 7401 ms
- **THEN** it glows at 0.5, then 0, then writes 0 once more and the task is gone

### Requirement: Dialogue keys
While a dialogue panel is open the digits 1 to 9 SHALL pick reply 0 to 8 and the Talk key SHALL end every conversation the player is in; otherwise the Talk key SHALL start talking to the focused character, and send nothing when none is focused.

#### Scenario: Picking and leaving
- **WHEN** the panel is open and the player presses 1, then the Talk key
- **THEN** reply 0 is picked, then the player's conversations end

### Requirement: Reply panel sync
The reply panel SHALL be rebuilt from the engine's entries only when they or the narrator flag change: each entry measured at the panel width less eight units, the `[Other]` toggle (topic 0xFFFF) appended with zero lines, then laid out; an emptied engine list SHALL clear the panel's rows and keep it shown, and a closed engine panel SHALL hide and clear it.

#### Scenario: An unchanged list keeps its animation
- **WHEN** a one-entry panel is synced at 1000 ms and again unchanged at 1200 ms
- **THEN** the height animation still ends at 1500 ms

#### Scenario: A conversation step
- **WHEN** the engine's list is emptied while its panel stays open
- **THEN** the panel stays shown with no rows

### Requirement: Script cast mirror
Each frame every live character SHALL reach the script cast with its name, roles, health as the red pool over the maximum, arousal (0 without a brain) and the collapsing/dead state bit, keeping its relations, topic memory and role values; a role list a script changed SHALL be written back to the AI's roles first and its change flag cleared.

#### Scenario: A script adds a role
- **WHEN** a character holds role 5 and a script adds role 7 in front of it
- **THEN** after the next frames both the AI and the cast hold roles 7 then 5

#### Scenario: Script health
- **WHEN** a body has red health 0.2, yellow 0.1 and maximum 0.25
- **THEN** the cast health is (0.2, 0.25), so `Health` gives 80

### Requirement: Talk candidates
On the frame the Talk key is pressed with no dialogue open, the candidates SHALL be the player's nearby characters: those in the player's room or a room behind one of its exits, seen from the player, within 30 m, the sixteen nearest; collapsing and dead ones (condition bit 0x800) SHALL be skipped.

#### Scenario: Sixteen within 30 m
- **WHEN** twenty characters stand 1 to 20 m away and one at 30.01 m, with no level rooms
- **THEN** the sixteen nearest are candidates and the far one is not

### Requirement: Talk target
The character talked to SHALL be the first candidate whose chest point, projected through the camera's rigid inverse view and viewport projection, lies at a squared pixel distance from the cursor below every earlier one's and below 3.4e38; an unordered distance SHALL never win and there SHALL be no facing test.

#### Scenario: Nearest on screen
- **WHEN** chests show at (900, 360), (700, 400), a collapsing one at (640, 360), a dead one at (641, 360) and an unordered one, with the cursor at (640, 360)
- **THEN** the one at (700, 400) is talked to

#### Scenario: No range on screen
- **WHEN** the only chest shows at (−5000, 9000)
- **THEN** it is still talked to

### Requirement: Reply panel form
The reply panel SHALL be made on the first talk, 400 by 120 pixels, centred across its parent and 120 + 32 pixels above the parent's bottom, and reused afterwards; it SHALL be shown while the engine's panel is open, also with an empty list, and hidden when that closes. The engine's pick turning on SHALL hold the picked entry for 1000 ms.

#### Scenario: First talk
- **WHEN** the player first talks on a 1280 × 720 window and the list needs 224 pixels
- **THEN** the panel sits at x 440, top 568, and its height eases from 120 toward 224, reaching 172 half way through the 500 ms

### Requirement: Reply panel pointer
The pointer over the shown panel SHALL hover the entry whose band (from one unit down, each `lines × 4 + 1` units tall) lies under it, clear the hover off the panel, count as over the GUI, and a left click released over a hovered band SHALL pick that entry.

#### Scenario: Clicking the second band
- **WHEN** a two-entry panel at top 568 is clicked at 30 pixels below its top
- **THEN** entry 1 (the second) is picked

### Requirement: Dialogue paint
After the open screens, every showing bubble SHALL be placed and drawn, a character's over its body position lifted 120 cm, then the shown reply panel, both in the main GUI font of the window's level; nothing SHALL run while there is no bubble and the panel is hidden.

#### Scenario: A reply bubble
- **WHEN** the partner answers while the panel is shown
- **THEN** one frame draws the bubble's box and text, then the panel

### Requirement: Reply panel draw
The panel's draw SHALL ease its height, fill a rounded box of radius 2 at alpha 160, and write each row's "N." one unit in and its wrapped text six units in: the hovered or held row in the highlight group's ninth ink with its alpha raised by half again, the others in the label group's ninth (its fifth while a pick is held).

#### Scenario: A hovered row
- **WHEN** a two-row panel is drawn with the first row hovered and the highlight ink's alpha 0x64
- **THEN** the box is 400 × 120 at alpha 160, "1." starts at the panel's x + 4 with its bottom 20 pixels below the top, and the first row draws at alpha 0x96

### Requirement: Session player
The player body's character id SHALL be the session's player, and a level character sharing that id SHALL stay out of the script cast and out of the Talk target.

#### Scenario: A shared id
- **WHEN** a level character and the player body both carry id 1
- **THEN** the cast's entry 1 holds the player's name and roles

### Requirement: Story companion
A new story in `exanima01` SHALL place the unknown background's companion, the `xac_derrin` actor template, at the story start's companion point (5808, 0, −4075) cm, under the character id after the level's records.

#### Scenario: Derrin greets the unknown
- **WHEN** a new player (Player, Human, XA_Unknown) talks to Derrin (Human, Activity Wait, XA_Derrin_Meet)
- **THEN** "Who are you?" is offered, and picking it brings Derrin's "My name's Derrin. I'm just a farm hand." and two replies

### Requirement: CheckpointSave call
`CheckpointSave(n)` SHALL read its argument and drop the value, and SHALL report whether the script's first actor slot is the player; it SHALL answer 1 when that slot is filled, 0 otherwise.

#### Scenario: The zone's routine
- **WHEN** `CheckpointSave(1);` runs with the player in the first slot, then with a guard
- **THEN** the first reports the player and the second does not

### Requirement: Zone routine
A script zone's routine SHALL run with the entering character as the one actor slot, no role, and the local `ZoneID` holding the zone's id.

#### Scenario: A routine reading its zone
- **WHEN** `? ZoneID = 4 : { GameEvent(A1); } / { GameEvent(B2); };` runs for zone 4 and for zone 3
- **THEN** the first raises event 0xA1 and the second 0xB2
