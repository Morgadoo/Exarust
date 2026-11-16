## ADDED Requirements

### Requirement: Open pack drains its crystals
While the player's own inventory is open, the inventory crystal-drain source SHALL drain the crystal stacks of the inventory's pack panel instead of the record, resolve each drained entry's icon again for its new flags, and give the carried entry it was loaded from the same flags.

#### Scenario: A crystal in the open pack
- **WHEN** a game second passes with an AR crystal stack of charge 10 in the open pack
- **THEN** the pack entry and the carried entry it came from both carry charge 8

### Requirement: View shows the worn core crystal
When the inventory's character view is made for a body, it SHALL show the first of the body's worn items that is one of the crystals A, AL, B, C and AR, and show nothing for a body that wears none; the shown item SHALL go when the view closes.

#### Scenario: A golem wearing its core
- **WHEN** the inventory opens on a body that wears crystal A
- **THEN** the view shows crystal A with its flags

### Requirement: Shown item draw and hover
The view SHALL draw the shown item's icon at its lower-left corner (the icon placed at one unit from the left and one unit above the bottom by the icon draw's origin rule), clipped to the view, and its hover pick SHALL answer the shown item first when the pointer is on an opaque pixel of that icon.

#### Scenario: The pointer on the shown icon
- **WHEN** the pointer presses on the shown crystal's icon
- **THEN** the pick answers the crystal and a drag of it starts

### Requirement: Core crystal drop and take-off
A drop of an item the body takes as its core SHALL be accepted only while the view shows nothing; it SHALL leave its source and be shown, without entering the record's lists. A release of a drag of the shown item SHALL take it off the worn list and show nothing.

#### Scenario: The AL crystal dropped on a golem's figure
- **WHEN** an AL crystal is dragged from the pack onto the figure of a body whose race takes it
- **THEN** it leaves the pack and is shown, and a second one is refused

### Requirement: View image supersampling
The inventory's character view SHALL render its image at the supersampling level its constructor takes from the settings: two image pixels a view pixel on each axis with super sampling on (4x), one with it off.

#### Scenario: Super sampling off
- **WHEN** the inventory opens with super sampling off
- **THEN** the view's image is the view's own size
