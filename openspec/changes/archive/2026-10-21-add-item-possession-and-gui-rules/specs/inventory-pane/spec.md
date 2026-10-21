## ADDED Requirements

### Requirement: Inventory window
The inventory window SHALL be 244 by 644 units (628 compact, 590 inside the arsenal) with an 8-unit border, its doll filling all but 260 units of the height above a 228 by 180 pack of capacity 400 and items up to 50. The pack's used capacity SHALL count every listed item, held ones too, and everything worn. A drag out of the pack SHALL be refused while the player swaps its weapon sets. A right-button release SHALL close the window.

#### Scenario: The normal window
- **WHEN** the window is built in the normal layout
- **THEN** it is 244 by 644 units and its doll 228 by 384

### Requirement: Arsenal
The arsenal SHALL show as many 236-unit columns as fit after a 12-unit margin and 61 units for a shown inventory, 60 units apart and at most five, with the scroll clamped to the columns left. An item dragged out of the columns SHALL stay in its column marked 0x200; one dropped back from elsewhere SHALL clear its copy's mark when its set is the current one, and otherwise go back to its owning set.

#### Scenario: Four columns
- **WHEN** a 1280-pixel arsenal at unit 4 shows its inventory and asks for a scroll of 3
- **THEN** four columns show, the scroll is 1, the left arrow is on and the right arrow off

### Requirement: Icon cache
An icon SHALL be cached per item and variant; a lit model SHALL keep its framing bit and condition byte and gain the lit bit, any other only its framing bit. A match SHALL add a reference; the last release SHALL free the slot for reuse; a release past it SHALL report "Item reference underflow.".

#### Scenario: Shared and freed
- **WHEN** an item's icon is asked for twice and released twice, then another item is asked for
- **THEN** the second request renders nothing new, the second release frees the slot, and the new item takes it

### Requirement: Item GUI controls
A crystal dropped into a socket SHALL replace the one there, which goes back with its charge; a drag from the same socket SHALL be refused. Rating dots SHALL be whole up to the value less a quarter, with a half dot for a remainder above a quarter. An icon's halo SHALL make each inner pixel's alpha the clamped sum of the five-pixel cross around it. A popup SHALL open centred on the cursor and kept on the screen, its bottom 8 units below the cursor.

#### Scenario: Three and a half dots
- **WHEN** a rating of 3.5 is drawn at unit 4
- **THEN** three dots go 12 pixels apart and a half dot follows

### Requirement: Panel entries
A panel entry SHALL not be drawn while dragged, held or without an icon; it SHALL be tinted green by the green mask, yellow at half opacity by the yellow mask alone, green at half opacity by both, else black. A looted body's worn panel SHALL scatter its items over the middle half of the panel and push them apart over four rounds, keeping every centre inside.

#### Scenario: The tints
- **WHEN** entries flagged with the green mask, both masks, the yellow mask and neither are drawn, and a held one
- **THEN** they are green, half-opaque green, half-opaque yellow and black, and the held one is not drawn
