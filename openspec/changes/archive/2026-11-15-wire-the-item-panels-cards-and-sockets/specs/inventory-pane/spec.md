## MODIFIED Requirements

### Requirement: Drag and hit
Ending a drag SHALL remove the icon. The hit SHALL be the top-most entry, held ones skipped, whose cell holds the point and whose icon's alpha there is above 0; with none, the top-most whose icon has an opaque pixel within a GUI unit of the point. An icon not yet rendered SHALL answer by its whole cell.

#### Scenario: Top icon
- **WHEN** two overlapping icons are dropped and the point lies in both, then the lower icon's drag ends
- **THEN** the hit was the upper icon, and the lower icon is gone

#### Scenario: A transparent corner
- **WHEN** the left button is pressed on a rendered icon's transparent corner, then on its opaque middle
- **THEN** the first press starts no drag and the second drags the entry

### Requirement: Crystal given back and the row committed
A crystal already in the socket a crystal is dropped on SHALL go back to the player: into the open pack by its centre add and place pass, else onto the carried list. After a drop or a drag out the row SHALL be committed into the item's weapon section (byte 0x48), else its apparel body (byte 0x218).

#### Scenario: Swapping crystals
- **WHEN** crystal A is dropped on a socket holding crystal C with charge 0x12
- **THEN** the socket holds A and crystal C with flags 0x120000 is back in the player's pack

## ADDED Requirements

### Requirement: Container clamp on open
A container popup SHALL keep every child its record loads inside its panel, each by its icon's half extents.

#### Scenario: A child stored outside
- **WHEN** a container whose child is stored far right of and above its panel opens
- **THEN** the child's icon centre is its half extents from the panel's right and top edges

### Requirement: Pack move kept
A move inside a pack SHALL write the entry's new cell, in sixteenths of a GUI unit, into the owner's carried list and put the entry last, so the pack opens again with it there.

#### Scenario: A move inside the pack
- **WHEN** an entry is dragged to another place of the same pack
- **THEN** the carried list's last entry is that item at the new cell, and binding the pack again puts it there

### Requirement: Place pass after a centre add
Every item added at a panel's centre SHALL then take a random spot in the middle half of the panel and be pushed out of the other entries, drawing from the item GUI's generator; a carried copy SHALL take the cell it ends at.

#### Scenario: A crystal given back
- **WHEN** a socket's crystal goes back into the open pack
- **THEN** its entry is not at the panel's centre and its carried copy has its cell

### Requirement: Pack item held in place
A weapon or shield taken from the inventory's own pack into a hand SHALL stay in the pack at its cell, marked held; a hand item the take-in frees SHALL show again in the pack.

#### Scenario: Swapping shields
- **WHEN** a shield is dragged from the pack onto the figure while another is in the off hand
- **THEN** the new shield's entry is held at its cell and the old one's entry is shown

### Requirement: Crystal drain sources of the panels
Item 499 dropped into a container's panel SHALL file drain source kind 2 with the container's id, into an owned pack kind 3; a committed socket row holding it SHALL file kind 4 with the item. A kind-2 source SHALL drain the container record's crystal stacks by 2 a game second.

#### Scenario: The AR crystal into a container
- **WHEN** the AR crystal with charge 0x30 is dropped from the pack into a container and a game second passes
- **THEN** the container is a source and its record's stack has charge 0x2E

### Requirement: Card gates from the game
The card's rank and points rows SHALL follow the mode switch (on in the arena, off in the story, a menu leaving them), the value row SHALL show only while the trade window is open, and the pronouns SHALL follow the player record's sex.

#### Scenario: Story then arena
- **WHEN** a female player is in the story, then the arena
- **THEN** the gates are off with female pronouns, then the rank and points rows are on

### Requirement: Socket crystal card
A right press on a socket of the shown card that holds a crystal SHALL open the crystal's card with its charge in the flags.

#### Scenario: Crystal B in the first socket
- **WHEN** the right button is pressed on a first socket holding crystal B with charge 0x30
- **THEN** crystal B's card shows with flags 0x300000

### Requirement: Use before the card
A card request on an item whose usable section's first byte is 2 SHALL run the game's object use first, by the section's second byte, then delay the card.

#### Scenario: The Proctor's Seal
- **WHEN** the right button is pressed on the Proctor's Seal in the pack
- **THEN** the once-a-game use is spent and the card waits on its timer

### Requirement: Dot size by GUI level
The rating dots, the slider's knob and the inventory's encumbrance dots SHALL be cut and spaced at the GUI level's large-icon cell: 16, 20, 28, 32, 42 or 64 pixels.

#### Scenario: Unit 4
- **WHEN** the dots are cut at GUI unit 4 for a 28-pixel row
- **THEN** they are 20 pixels and draw 4 below the row's top
