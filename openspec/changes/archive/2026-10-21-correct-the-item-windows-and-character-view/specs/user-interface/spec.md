## MODIFIED Requirements

### Requirement: Item-panel relaxation
The system SHALL place a new entry at a quarter of the panel plus a draw below half of it from the engine's shared generator and push it out over six passes of two rounds, an axis pushing only when its overlap (slack 2) is above the pass number by a signed compare and the entry moving only when both axes push.

#### Scenario: One entry against another
- **WHEN** a new entry is placed with the generator at 0 against another at (100, 100), both half 80, in a 400×400 panel
- **THEN** it lands at (100, 106), moves to (262, 262), the other stays, and the generator has advanced twice

#### Scenario: A far entry
- **WHEN** the new entry lands at (100, 106) and the other entry, half 10, is at (390, 390)
- **THEN** both overlaps are negative and the new entry stays

### Requirement: Item-panel hit test and drop
The system SHALL hit-test from the last entry backwards over the cell box, skipping held entries, first by the icon's own pixels and then, when nothing was hit and the GUI's switch is on, by the icon cache's second test. A drop SHALL subtract the grab offset and keep the entry inside by its icon's half extents, right, left, top, then bottom. Removal SHALL shift the tail left.

#### Scenario: Overlapping cells
- **WHEN** two cells are at (0, 0) and a point lies inside both
- **THEN** the hit is the higher index

#### Scenario: Top before bottom
- **WHEN** an entry with half extents 40 is dropped at y −60 in a panel 60 tall with a 160 cell
- **THEN** the top lifts it to −40 and the bottom then sets it to −60

### Requirement: Item-panel accept and positions
The system SHALL reject a drag on the reject mask, the container's own id, a missing model, an oversized item, a capacity overflow (unless from the linked panel or this one), or a required-mask mismatch; `only_own` SHALL require an entry still marked dragged with the payload's icon. List positions SHALL be sixteenths of a unit, y also divided by the vertical scale, and record positions eightieths.

#### Scenario: Capacity and codec
- **WHEN** the capacity is 10, used 6 and the item bulk 5
- **THEN** the drag is rejected, unless it comes from the linked panel
- **WHEN** a position of (140, 20) px is saved with unit 4 and vertical scale 2
- **THEN** the list value is (560, 40) and the record x value 2800

### Requirement: Item-panel mouse intent
The system SHALL map a mouse event to no action, a drag start with the cell-centre grab, use, open or card; the right-button bit SHALL pass to the base form, nothing SHALL happen without the press bit, and a drag start SHALL require drag permission and a hit.

#### Scenario: Drag, use and card
- **WHEN** the 0x4000 and press bits are set with drag permission and a hit
- **THEN** the intent is a drag start with grab (80, 80) for a 160 cell
- **WHEN** the 0x8000 and press bits are set on a container item
- **THEN** the intent is open
- **WHEN** the 0x2000 and press bits are set with a hit
- **THEN** the intent is card
- **WHEN** the 0x4000 bit is set without the press bit
- **THEN** the intent is no action

## ADDED Requirements

### Requirement: Item-panel grid relaxation
The system SHALL jitter each ideal grid centre by up to an eighth of a cell from the panel's own seeded generator, give the first slot to the last entry, and relax over five passes dividing the overlap by 4 to 8, each pass ending with every centre kept inside the panel.

#### Scenario: No jitter
- **WHEN** three entries with no extent are laid out in a 400×300 panel with a 4-pixel cell
- **THEN** the last entry's cell is at (98, 73) and the first's at (198, 223)

### Requirement: Item-panel scatter
The system SHALL scatter every entry over the middle half of the panel from the engine's shared generator, x then y for each, and relax over six passes dividing by 1, 1, 2, 2, 3 and 3, with slack 2 when the other entry is further along and 1 otherwise, both entries moving only when both axes overlap, each pass ending with the clamp.

#### Scenario: Five entries
- **WHEN** five entries of half 80 are scattered in a 400×400 panel
- **THEN** the generator advances ten times and every centre lies within 80 to 320

### Requirement: Item-panel drop sources
A drop from the linked panel SHALL show its held entry of the item again and a drop from the panel itself SHALL move the entry still marked dragged with the payload's icon, either being raised to the end; any other drop SHALL append the item, adding its bulk unless it came from the linked panel; a payload without an icon SHALL append nothing.

#### Scenario: From the linked panel
- **WHEN** an item the panel does not hold as held is dropped from the linked panel
- **THEN** it is appended and the used bulk does not change

### Requirement: Item-panel position rounding and records
Position saves SHALL round half to even and keep the low 16 bits, and loads SHALL round half to even. A record load SHALL take the container's id; a record SHALL be saved only for a loaded panel showing a database copy.

#### Scenario: Halves round to even
- **WHEN** list values 2 and 6 and record values 10 and 30 are loaded at unit 4
- **THEN** they give 0, 2, 0 and 2

### Requirement: Item card request
A card request with an item under the pointer and no request pending SHALL ask for a delayed card when the item's use type is 0 or 1, run the game's use first for type 2, and build the card at once otherwise. A delayed card SHALL come due 200 ms of real time after the request, a new request SHALL replace a pending one, and a close request SHALL cancel a pending timer before closing an open card.

#### Scenario: A delayed card
- **WHEN** a card is requested at 1000 ms for an item of use type 0
- **THEN** it is not due at 1199 ms, is built once at 1200 ms, and a cancelled later request builds nothing

### Requirement: Character view camera
The character view SHALL place its camera 410 cm from the figure at (0, 100, 0) towards (−200, 60, 360) with a 30° field of view, and choose its first super-sampling level as 1 for a setting at most 0, the setting plus 2 for 1 to 3 and 4 above. Its task SHALL ease the figure's race share up to the race's value by 0.003 per frame millisecond, never past it.

#### Scenario: The camera
- **WHEN** the view is built with a super-sampling setting of 3
- **THEN** the camera is 410 cm from the figure on the (−200, 60, 360) side and the level is 5

### Requirement: Character view background
The background SHALL be the skin scheme's bytes over 240, decoded from sRGB when sRGB targets are on and brightened by 1.4 when they are off, washed to red 0.38 and green 0.4 of the green and blue 0.45 of the blue, as bytes of 255 at the top and 280 at the bottom rounded half to even.

#### Scenario: The wash
- **WHEN** a scheme of green 240 and blue 200 is washed without sRGB targets
- **THEN** the top colour is (136, 143, 134)

### Requirement: Figure drag acceptance
The figure SHALL accept apparel only for a race that wears it in a view that allows changes, a withheld weapon or shield only with the body's matching permission, and an item with neither section only as the AL crystal on a race whose core word takes it while nothing is shown. A drop SHALL wear apparel, take a weapon, take a shield or take the body's core crystal, in that order.

#### Scenario: The AL crystal
- **WHEN** item 500 is dragged over a body whose race core word takes it, first with nothing shown and then with an item shown
- **THEN** the first is accepted and the second refused

### Requirement: Figure drag start and release
A frozen view SHALL start drags only from a held item, and an item in a locked view flagged 0x80000000 SHALL be dragged as a payload no panel takes. A release SHALL take a held item out of its hand, take the shown item off when the dragged icon is the shown one, and otherwise take the dragged item off, charging the pack its bulk when the pack did not receive it.

#### Scenario: Releasing over the figure
- **WHEN** drags end over hand 2, over the shown item, and elsewhere with the pack open
- **THEN** the first takes hand 1 (0-based) out, the second takes the shown item off, and the third takes the dragged item off and charges the pack
