## MODIFIED Requirements

### Requirement: Skill node hit
For each node within 23 units of its centre (an unordered compare: a NaN distance passes) the result SHALL become that node's group code `(group + 1) << 8`; five samples SHALL sit at 18.3 units around the node, 0.2 of a turn apart, with `x + round(sin) × turn` and `y − round(cos) × turn`, and a sample within 5 units SHALL OR its index into the code and add one. A later node in reach SHALL replace an earlier one's code. The result SHALL start at 0 on page 2 and −1 otherwise.

#### Scenario: A sample and a miss
- **WHEN** page is 2, the unit is 4, the turn is 1, the node at 100, 100 shows group 1 (0-based) and the mouse is at 100, 27
- **THEN** the result is 0x201
- **WHEN** the mouse is at 170, 77
- **THEN** the result is 0x202
- **WHEN** the mouse is at the node's centre
- **THEN** the result is 0x200
- **WHEN** the page is 0 and the mouse is far
- **THEN** the result is −1

### Requirement: Skill hover
Setting the hover SHALL store a positive index or current and return 3. The candidate SHALL be the index when bits 8–10 are set and it is a group or its grid byte is set, else 0. An unchanged candidate SHALL leave the stamps; a new positive candidate with a different last positive SHALL stamp the clock, else the stamp SHALL take the unsigned elapsed time with the under-200 correction. The candidate SHALL become previous and, if positive, last positive; with both non-positive the base runs.

#### Scenario: Hover, miss and stamp
- **WHEN** the index is 0, the current is 0 and the base is 7
- **THEN** the result is 7 and the base ran
- **WHEN** the index is 0x0203, grid byte 20 is set, previous is 0, stamp 900 and clock 1000
- **THEN** the candidate and last positive are 0x0203
- **AND** the stamp stays 900
- **WHEN** the last positive is 0x0101 instead
- **THEN** the stamp becomes 1000
- **WHEN** the candidate equals the previous
- **THEN** the stamps do not change
- **WHEN** the grid byte is clear
- **THEN** the previous stays 0
- **WHEN** the index is the group code 0x0300 with an empty grid and the stamp 1100 is ahead of the clock 1000
- **THEN** the candidate is 0x0300 and the stamp becomes 1000

### Requirement: Skill popups and tooltips
The learning row and the forget dialog SHALL open over 300 ms (alpha `t(2 − t)`, width eased again) and close over 200 ms (alpha `(remaining × 0.005)²`), clearing the pending choice after. The learning row SHALL be 82 by 28 units four under its level's satellite with eight icons ten apart; the forget dialog 82 by 20 units two under the satellite with Yes 12 left and No 6 right; a ring node's tip 82 by 14 units 13 under its satellite and clamped into the window.

#### Scenario: Half way open
- **WHEN** an opening popup has 150 ms left
- **THEN** the alpha is 0.75 and the width scale 0.9375

## ADDED Requirements

### Requirement: Skill screen in play
In play the skills key (through the hotkeys' permission gate) SHALL open the skill screen as a ring over the window at the interface unit, with the player's record attached, its atlases cut at that unit and `SkillShow_01` played. While it is open the key SHALL not reach it again. Closing SHALL put the technique word of the record's skills on the player's body.

#### Scenario: Open, choose and close
- **WHEN** K is pressed in exanima01 and the level-1 satellite of Close Combat is clicked
- **THEN** the learning row opens over it
- **WHEN** its first slot is clicked
- **THEN** the record's Close Combat holds Remise at level 1
- **WHEN** the right button is pressed
- **THEN** the screen closes and the body takes the technique word

### Requirement: Skill screen frame
Each frame the open screen SHALL step the draw's fades (a finished hide closes it), the popup's fade (a finished close clears the pending choice), the point hit test and the hover filter.

#### Scenario: The popup closes
- **WHEN** a learning row's 200 ms close ends
- **THEN** no learning row is pending

### Requirement: Skill pointer
A right button SHALL close the ring screen unless it is hiding or on a page. With nothing under the pointer a pending learning row or forget dialog SHALL start its 200 ms close if its popup is settled. The page-2 background SHALL show a hidden screen again or start the hide. A locked screen SHALL take no other click; a click SHALL press the node with nothing pending, else answer the pending choice.

#### Scenario: Forget a level
- **WHEN** a learnt level is clicked
- **THEN** the forget dialog opens with a 300 ms popup
- **WHEN** Yes is clicked with the popup settled
- **THEN** the level is unlearnt and the popup starts its 200 ms close

### Requirement: Skill icon cells upright
An icon cell quad SHALL go top-left, bottom-left, bottom-right, top-right, the top edge sampling the cell's far row, so the icon stands upright.

#### Scenario: A cell
- **WHEN** icon 6 of a 32-pixel grid is drawn at 10, 20
- **THEN** the corners are 10,20 then 10,52 then 42,52 then 42,20 and the first samples the higher row

### Requirement: Skill boxes and labels in order
Each glow box and label SHALL be drawn after the batches emitted before it, so the learning row's icons sit on its box; a glow box SHALL be the skin's rounded box in black at its alpha, grown by eight units at the unit.

#### Scenario: The learning row
- **WHEN** the learning row is open
- **THEN** its box is drawn before its icons and its labels after them
