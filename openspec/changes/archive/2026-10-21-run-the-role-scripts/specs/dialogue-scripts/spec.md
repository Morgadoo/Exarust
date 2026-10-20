## ADDED Requirements

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
The `Actor` class SHALL offer its 22 methods (`Role` … `HasItem`), the writable `Activity` and `IdleActivity` (read back as 0) and the text members `Name`, `ContextPhrase`, `ContextObj`. `Health` SHALL round pool/maximum × 100 to nearest even; `AddConcern` SHALL keep concern within 0 to 15; `Trust` of oneself SHALL be 100.

#### Scenario: Health and concern
- **WHEN** the pool is 0.125 of its maximum, and a concern of 13 gets 10 added
- **THEN** `Health` gives 12 and the concern becomes 15

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
An activated node SHALL need its hits (or distinct witnesses), open after its delay class (0, 2, 5, 10, 30, 300, 1800 s) and stay open for its window class. A stored block whose count differs from the dialogue's SHALL load with its count word in the first slot.

#### Scenario: Activation
- **WHEN** a node needing two hits with a 5 s delay and a 60 s window is hit at 100 and 101 s
- **THEN** it opens at 106 s and closes at 166 s

#### Scenario: A grown dialogue
- **WHEN** words 7, 8, 9 are stored and loaded for five nodes
- **THEN** the words are 3, 7, 8, 0, 0

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
A line SHALL show for 1000 ms + 50 ms per byte, stretched by (2 − d/10 000) for the narrator under 10 s and halved for the player; a speaker's next line SHALL wait 375 ms past its previous one's end; a character's line SHALL be cut at a CR; a bubble SHALL fade in over 300 ms and out over 1.5 s.

#### Scenario: Durations
- **WHEN** ten-byte lines are said by a character, the narrator and the player
- **THEN** they last 1500, 2775 and 750 ms

### Requirement: Scripted objects and crystals
Using a scripted object SHALL run its script at most once per 2.4 s across all such objects, with a 1.2 s glow pulse up and down; a charged crystal SHALL lose 2 charge per game second and a spent source SHALL leave the list.

#### Scenario: Use interval
- **WHEN** objects are used at 5000 ms, 7399 ms and 7400 ms
- **THEN** the first and third run their scripts and the second is refused
