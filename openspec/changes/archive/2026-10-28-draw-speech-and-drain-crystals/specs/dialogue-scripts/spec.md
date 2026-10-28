## MODIFIED Requirements

### Requirement: Topic activation
An activated node SHALL need its hits (or distinct witnesses), open after its delay class (0, 2, 5, 10, 30, 300, 1800 s) and stay open for its window class. A conversation's memory SHALL be its count word followed by one word per node, node n at slot n. A stored block whose count differs SHALL load shifted: slot 1 gets the stored count, slot j the stored node word j − 1, the rest zero.

#### Scenario: Activation
- **WHEN** a node needing two hits with a 5 s delay and a 60 s window is hit at 100 and 101 s
- **THEN** it opens at 106 s and closes at 166 s

#### Scenario: A grown dialogue
- **WHEN** words 7, 8, 9 are stored and loaded for five nodes, then for two
- **THEN** the buffers are 5, 3, 7, 8, 0, 0, then 2, 3, 7

## ADDED Requirements

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
