## MODIFIED Requirements

### Requirement: Form scrolling
The system SHALL set the horizontal scroll step to 10/overflow (0 without one), the thumb to `round(track · client / content)` (ties to even) at least 9 and at most the track, and the thumb x to `border + rail + round((track − thumb) · f)`; setting the fraction SHALL clamp it with ordered compares (a NaN passes), snap it when the attribute word asks, store 0 when the content fits, and place the thumb from the snapped fraction unless the thumb itself is held.

#### Scenario: A long form
- **WHEN** the width is 404, content 1200, border 2 and rail 19
- **THEN** the step is 10/796, the thumb 121 and the x at f 0 is 21, at f 1 is 262
- **WHEN** the fraction 0.55 is set with a step of 0.1 and snapping
- **THEN** the stored fraction is 0.6 and the thumb x 166, or 154 while the thumb is held
- **WHEN** the content fits
- **THEN** the stored fraction is 0 and the thumb stays where it was asked

## ADDED Requirements

### Requirement: Form sibling layers
The system SHALL raise a child by the attribute words: a lower-layer (0x2000) child to the first child outside that layer (one less when that is above it), searched from the first child among the root's children and from the second in other parents; an upper-layer (0x1000) child to the top; any other just below the upper layer; and the children below it that it owns after it.

#### Scenario: The search start
- **WHEN** the middle of three children is in the lower layer and is raised
- **THEN** it goes to index 0 among the root's children and to index 1 in another parent

#### Scenario: Owned popups
- **WHEN** a form whose popup sits below it is raised
- **THEN** the popup ends above it

### Requirement: Form active test
The system SHALL treat a form as its task's active one when it is the last child outside the upper layer, or, when its attribute word is exactly 0x1000, when it is the first child, without looking at visibility.

#### Scenario: The exact word
- **WHEN** the first of two children has the attribute word 0x1000
- **THEN** it is active

### Requirement: Form unset place
The system SHALL place an unset x at `(right − left − width) / 2` (truncating) when centred, `right − width − 4` when right-anchored and 4 otherwise, and an unset y at `(bottom − top − height) >> 1` (a floor) when centred, `bottom − height − 4` when bottom-anchored and 36 otherwise, in the parent's client rectangle; showing SHALL re-centre both axes truncating.

#### Scenario: Negative spans
- **WHEN** a 901 × 701 centred form is placed in a client 2…798 by 24…600
- **THEN** it is at (−52, −63), and showing re-centres it to (−52, −62)

### Requirement: Form maximise and restore
The system SHALL maximise a form to 0, 0 at the screen's size with the four anchor bits, a free form (attribute 0x40000) losing its border, spacing, attributes 0x4f and style 0xf0, and SHALL restore each saved coordinate and size divided by the saved unit (truncating) times the current one, the border rounded half to even.

#### Scenario: Doubling the unit
- **WHEN** a form saved at 10, 20, 302 × 402 with border 2 at unit 4 is restored at unit 8
- **THEN** it is at 16, 40, 600 × 800 with border 4

### Requirement: Form fade
The system SHALL fade a hidden form in over 256 GUI ticks at alpha `1 − left/256` and out at `left/256` in mode 0, SHALL set no alpha in mode 1, SHALL not fade in a visible form, and SHALL close a form whose hide has ended without drawing it.

#### Scenario: Half-way
- **WHEN** a mode 0 show started at 1000 is drawn at 1128
- **THEN** the alpha is 0.5
- **WHEN** a mode 1 show is drawn half-way
- **THEN** no alpha is set

### Requirement: Form hit test
The system SHALL answer the skin's frame code off the client (with the resize cursor unless the skin's cursor is −2), try the visible hit-taking children last first where the content point is strictly inside, and answer −1 for a client without a record, or 0 for a form without attribute bits 0x30.

#### Scenario: A child's edge
- **WHEN** the content point is on a child's left edge
- **THEN** the parent answers

### Requirement: Form record hit
The system SHALL try the enabled, hit-taking records that do not pass the point through first on their rectangles with the edges and then within two units, answering a record's 1-based index with its 0x7f0000 bits (0x10000 when none) and its cursor.

#### Scenario: Exact before slop
- **WHEN** a point is one pixel past one record and on the next one
- **THEN** the next record answers

### Requirement: Form close band and wheel
The system SHALL treat −21, −20, −14, −13, −10, −9 and −6…−4 (and −8 with the skin's first flag) as the close band and SHALL turn the wheel by `round(delta / 120)` vertical steps on a form with a vertical scrollbar, else by `round(−delta / 120)` horizontal steps, both reducing the fraction.

#### Scenario: One notch
- **WHEN** a form with a vertical bar at 0.5 and step 0.25 gets a 120 turn
- **THEN** its fraction is asked to be 0.25
- **WHEN** a form with only a horizontal bar at 0.3 and step 0.1 gets a 120 turn
- **THEN** its fraction is asked to be 0.2
- **WHEN** the code is −19
- **THEN** it is not in the close band

### Requirement: Form snap slots
The system SHALL cut a snap container's client, between its edges and its visible snapping children's client rectangles widened by their spacing, into slots: from each free cell the longer free run (the row on a tie) SHALL become a slot one cell thick.

#### Scenario: One child
- **WHEN** a 100 × 100 container holds one 30 × 30 child at 20, 20
- **THEN** the slots are 0,0–100,20; 0,20–20,100; 50,20–100,100 and 20,50–50,100

### Requirement: In-game screen tags and places
The system SHALL tag the inventory 0xce1c2010, a container popup 0xce1c2013, the journal 0xce1c2030 and the loot window 0xce1c2040, open the inventory at (8, 8) 244 wide and 644 high (628 compact, 590 in the arsenal), put a filled inventory at its remembered place for the player and at the screen's right less its width and four units for another character, and open the journal 244 × 518 at its remembered place when set.

#### Scenario: Another character's inventory
- **WHEN** a non-player's inventory fills on a 1600-wide screen at unit 4
- **THEN** its x is 1340

### Requirement: In-game show rules
The system SHALL refuse the inventory and journal unless the player is ready and not dead, leave a filling inventory, close an active inventory when the call toggles and activate an inactive one, close an active journal and activate an inactive one, and refuse the loot window for a character whose flag word's second byte is 3.

#### Scenario: Toggling
- **WHEN** the player's active inventory is asked for again with the toggle
- **THEN** it closes

### Requirement: In-game HUD extras
The system SHALL draw the status marker 53 units right of the centre and 8 above the bottom while no status entry has 0x2000, the skill-ready alert 60 units left while the six skills' levels sum below 15 and no skill has more techniques among its first `level + 1` bytes than its level, both pulsing at `ease(|1000 − (now − start) mod 2000| / 1000)` with a glow of 0.6 of it; the level curtain SHALL fall to black over three seconds at `round(t · 255)` with the world at `1 − t`.

#### Scenario: The pulse and curtain
- **WHEN** 1500 ms have passed
- **THEN** the pulse is 0.5 and its glow 0.3
- **WHEN** the curtain is 1.5 s in
- **THEN** its alpha is 128 and the world's brightness 0.5

### Requirement: In-game counter control
The system SHALL turn a negative counter value into 0 (−0 and NaN kept), caption it `value × 100` with one decimal and `%`, and fill blocks `round(width / (n + 1) − 1) − 3` wide, `block + 1` apart from two pixels in, the share past a whole block rounded.

#### Scenario: Three and a bit
- **WHEN** a four-segment counter 100 wide is at 0.6
- **THEN** its fills are 21, 21 and 8 wide at 2, 27 and 52

### Requirement: HUD bars show values
The system SHALL compare the yellow health and focus with the values taken in the last frame that showed the bars.

#### Scenario: A hidden rise
- **WHEN** the focus rises while the bars are hidden and then drops by less than the rise
- **THEN** the bars do not ride up

### Requirement: In-game screens in the running game
The running game SHALL open the inventory and journal from the I and J hotkeys through the hotkey handler's permissions, register the inventory as an open container, keep the journal's entries for the session, and close the focused screen on Escape.

#### Scenario: Two screens
- **WHEN** I and then J are pressed and then Escape
- **THEN** the journal closes and the inventory takes the focus
