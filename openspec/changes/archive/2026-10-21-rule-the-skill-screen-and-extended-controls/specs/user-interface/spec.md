## MODIFIED Requirements

### Requirement: Skill ring position
The angle SHALL be `(1 − turn) + (slot − 3.5) × 2π / count`, summed in that order. The x offset SHALL be the radius times the unit times the engine's sine times the horizontal stretch, the y offset the radius times the unit times the cosine, each rounded to the nearest even integer and multiplied by the turn, and added to the centre. The centre x SHALL lose one unit with the inset-layout flag on a positive page; the centre y SHALL lose three units on page 0.

#### Scenario: A node on the ring
- **WHEN** the count is 8, the radius 10, the stretch 5, the turn 1, the size 200 by 100, the page 0, the unit 4 and the slot 3
- **THEN** the position is 23, 75
- **WHEN** the page is 1 and the inset-layout flag is set
- **THEN** the position is 19, 87

### Requirement: Skill satellite
The satellite SHALL start from the stored slot's ring point at `(1 − turn) + (stored − 2.5) × 2π / count`, then add the pentagon offset of `sample − 1`: `18.3 × unit` times the sine across and minus the cosine down, each rounded in double precision and multiplied by the turn.

#### Scenario: Two samples
- **WHEN** the count is 8, the radius 10, the stretch 5, the turn 1, the size 200 by 100, the page 0, the unit 4 and the stored slot 3
- **THEN** sample 1 is 177, 2
- **AND** sample 3 is 220, 134
- **WHEN** the page is 1 and the inset-layout flag is set
- **THEN** sample 1 is 173, 14

### Requirement: Skill point nodes
A node awaiting the forget confirmation SHALL be tested first: a point six to ten units below it SHALL give 1 for a rounded horizontal offset between 13 and 4 units to the left and 2 for the same to the right, else −1. A node whose learning row is open SHALL give, for a point in the eight units above it, the slot `round(offset / (10 × unit) + 4.5)` half to even when it is 1 through 8, else −1. Without either the fallback SHALL run.

#### Scenario: High, low and fallback
- **WHEN** the forget node is at 100, 50, the unit 4 and the point 80, 80
- **THEN** the result is 1
- **WHEN** the point is 120, 80
- **THEN** the result is 2
- **WHEN** the learning node is at 100, 40 and the point 120, 20
- **THEN** the slot is 5
- **WHEN** the point is 100, 20
- **THEN** the slot is 4
- **WHEN** no selection exists
- **THEN** the fallback result is returned

### Requirement: Trackbar geometry
The system SHALL map a cursor to `(cursor−x−pad−span/2)/travel` in double precision, set a value below 0 to 0 and above 1 to 1 while a NaN passes both, quantise by the step count with a 64-bit half-to-even rounding (a NaN rounding to the indefinite integer), place the thumb at `x+pad+rint(travel·value)` and centre the caption in the knob slot.

#### Scenario: Half value
- **WHEN** the cursor is 50 in a 100-wide track with pad 2 and span 20 at x 0
- **THEN** the value is 0.5 and the thumb left is 40
- **WHEN** the value is 0.6447 with four steps
- **THEN** the value is 0.75
- **WHEN** the travel is 0 and the cursor is centred
- **THEN** the value is NaN

### Requirement: Control label horizontal alignment
The system SHALL align a label by the style's horizontal field: 1 left of the control, 3 centred in the available width, 4 ending before its right edge, 5 after it; any other value, 2 included, SHALL take the default half-unit inset.

#### Scenario: Four styles
- **WHEN** x is 100, the text width 20, the unit 4 and the style 0x1000
- **THEN** the label x is 76
- **WHEN** the style is 0x2000 with pad 2 and available width 100
- **THEN** the label x is 104
- **WHEN** the style is 0x3000
- **THEN** the label x is 145
- **WHEN** the style is 0x4000
- **THEN** the label x is 184
- **WHEN** the style is 0x5000
- **THEN** the label x is 204

### Requirement: Control label vertical alignment
The system SHALL lift by half a unit (1), centre by the line height with a shift halving (3), base-align (4), place below (5), or use the half-unit default for any other value, 2 included.

#### Scenario: Vertical styles
- **WHEN** y is 100, the unit 4 and the style 0x100
- **THEN** the label y is 98
- **WHEN** the style is 0x200
- **THEN** the label y is 102
- **WHEN** the style is 0x300 with height 20 and line height 16
- **THEN** the label y is 119
- **WHEN** the style is 0x400 with base 14
- **THEN** the label y is 133

## ADDED Requirements

### Requirement: Skill learning row requirements
The learning row's mask SHALL mark the nodes of the five written techniques chosen, the slots the tree has no node for absent (over a chosen mark), then every still-clear slot missing when either of its two prerequisites is not among the techniques the points fund, read through the record's byte layout.

#### Scenario: Close Combat's mask
- **WHEN** Close Combat has 35 000 points and Remise, Feint, Fend and Riposte
- **THEN** the mask is chosen for the first four, absent for the fifth to seventh and clear for Dual Wield
- **WHEN** Riposte is not chosen
- **THEN** Dual Wield is missing
- **WHEN** Ranged Combat is asked
- **THEN** every slot is absent

### Requirement: Skill node press state
A ring node SHALL be learnt when the points fund its level; the next level SHALL be open while a level is ready and its technique byte is clear, and, with a technique chosen there ahead of the points, learnt once the skill is more than ten points past its funded levels, else open; any other level SHALL be locked.

#### Scenario: One funded level
- **WHEN** Close Combat has 5000 points and Remise
- **THEN** level 1 is learnt, level 2 is open with a ready level and locked without, and level 3 is locked
- **WHEN** Feint is chosen ahead at 5010 points
- **THEN** level 2 is open, and learnt at 5011 points

### Requirement: Skill unlearning
Forgetting a level SHALL move the points by where the level sits, in 16-bit arithmetic: at or below the committed levels both the committed and the record's points lose 5000; one past them the committed points round down to their levels and the record's lose 5000 while it holds more levels, else take the committed points; further out the record's lose 5000 when they fund the level, else round down.

#### Scenario: A wrapped count
- **WHEN** the committed points are 5000, the record's 0 and level 1 is forgotten
- **THEN** the record holds 60 536 points

### Requirement: Skill unlearning cascade
The forgotten level's technique SHALL leave, the later written slots move down, and from the last funded level down every technique listing the removed one as a prerequisite SHALL be unlearned in turn.

#### Scenario: Armour's chain
- **WHEN** Armour holds I, II and III at 15 000 committed points and I is forgotten
- **THEN** II and III go with it and both point counts are 0

### Requirement: Skill learning commit
A press on a ring node with nothing pending SHALL open the forget dialog for a learnt node or the learning row (with the group's mask) for an open one, starting a 300 ms popup opening. A second press SHALL, in the forget dialog, unlearn on Yes; in the learning row, on a slot with a clear mask byte, unlearn the record's surplus choice and write that slot's technique into the pending level. Either SHALL refresh the ready flag and start the popup's 200 ms close unless it is fading.

#### Scenario: Riposte at level two
- **WHEN** Close Combat has 5000 points and Remise, level 2 is pressed and then slot 4
- **THEN** the techniques are Remise and Riposte
- **WHEN** slot 8 is pressed instead
- **THEN** nothing is written, Dual Wield's prerequisites being missing

### Requirement: Skill experience pass
With global experience the screen SHALL spend its worth: the last group with a partial committed level first, at the missing points times a weight of four plus the committed levels, quartered toward zero, or the budget's share `budget × 4 / weight` when short; then each level between a group's committed and record levels at `weight × 5000 / 4`, the weight rising after each; and the rest, compared unsigned, on the record's surplus choice, a full level or the share on top of the level below.

#### Scenario: A surplus choice takes the rest
- **WHEN** Close Combat's committed and record points are 2500 with Remise and Feint chosen and the worth is 5000
- **THEN** the record holds 7000 points and nothing is left

### Requirement: Skill ready flag
A locked screen SHALL never be ready. While experience is left over a level SHALL be ready. Otherwise the scan SHALL stop at the first skill whose next technique byte is set, which must be no more than ten points past its levels, and the levels counted to there SHALL stay below fifteen.

#### Scenario: The scan stops early
- **WHEN** every skill has 15 000 points and two techniques
- **THEN** no level is ready
- **WHEN** Close Combat instead has 10 000 points with a third technique chosen
- **THEN** a level is ready

### Requirement: Skill group icons and orbits
A group SHALL show one icon per funded level, plus the next while a technique is chosen ahead (at half alpha) or a level is ready with fewer than five funded, on the pentagon about a point four units left of and above the group; the first empty level SHALL draw the choose icon and end the row. Its orbit bars SHALL span 20.3 to 26.3 units out and 14 either side, the last filling by the points past the funded levels when below 0.99 in double precision.

#### Scenario: Two levels
- **WHEN** Close Combat has 10 000 points with Remise and Feint at 100, 100 and the unit 4
- **THEN** the icons stand at 84, 11 and 154, 61

### Requirement: Skill popups and tooltips
The learning row and the forget dialog SHALL open over 300 ms (alpha `t(2 − t)`, width eased again) and close over 200 ms (alpha `(remaining × 0.005)²`), clearing the pending choice after. The learning row SHALL be 82 by 28 units four units under its node with eight icons ten units apart; the forget dialog 82 by 20 units two under it with Yes 12 units left and No 6 right; a ring node's tip 82 by 14 units 13 under it, clamped into the window, naming the technique chosen at that level.

#### Scenario: Half way open
- **WHEN** an opening popup has 150 ms left
- **THEN** the alpha is 0.75 and the width scale 0.9375

### Requirement: Extended control classes
The unit SHALL register the text field, combo box, colour swatch, spinner, progress bar, separator, image, texture view, trackbar, caption and scroll bar with their default sizes, hooks for the text field, combo box, spinner, trackbar and scroll bar, and a unit hook caching the spinner button width `3·unit | 1`. The trackbar's hook SHALL default the knob width to 20 and the pad to 2.

#### Scenario: Defaults
- **WHEN** the unit is 4
- **THEN** the spinner button is 13 wide and a trackbar with no knob width gets 20

### Requirement: Spinner rules
A spinner commit SHALL set a value below the minimum to it and above the maximum to it, letting a NaN pass, and change the value only when it moves by more than 1e-8 in double precision. A press left of the button column SHALL place the caret, on the buttons step up above half the height and down below. Typed bytes SHALL pass as digits, a dot (stepping over a dot at the anchor) or a minus while the anchor or caret is 0.

#### Scenario: A NaN commit
- **WHEN** a spinner at 50 commits a NaN
- **THEN** the value stays 50 and the text is marked dirty

### Requirement: Extended control geometry
The image SHALL pick `(x − pad)/(w − pad)` and `1 − (y − pad)/(h − pad)` from the message's coordinates, the scroll bar map `(x − left − pad − width/2)/travel` with a `4·unit + 1` thumb, both clamped with a NaN passing; the progress bar SHALL fill `w × value` half to even; a separator SHALL leave its text between two `(w − gap)/2 − 1` lines, or keep only the right part when right-aligned; a drop-down list that does not fit SHALL open above or be cut to the rows below by its row counts.

#### Scenario: Zero travel
- **WHEN** a scroll bar 21 wide with pad 2 and unit 4 maps x 10
- **THEN** the value is NaN
