# Spec Delta

## MODIFIED Requirements

### Requirement: Screens from the constructors' words
Every game window SHALL paint through the game skin's form draw with the words their show commits from their constructors' words: each window as a top-level form, its views, panels, tables and pages as held children. The pack panel SHALL use attributes 0x130032 and style 0x80 as constructed. Control records SHALL be laid out by the two-pass layout inside the client.

#### Scenario: Inventory records
- **WHEN** the inventory at unit 4 is laid out
- **THEN** the Encumbrance label is at client (4, 386), Primary is 108 by 24 at (4, 416) and Secondary 108 by 24 at (116, 416)

#### Scenario: Journal records
- **WHEN** the journal is laid out
- **THEN** New is 104 by 24 at client (4, 20) and Erase is 104 by 24 at (120, 20)

#### Scenario: Title bit
- **WHEN** a frame is painted without the title bit
- **THEN** no title text is drawn

#### Scenario: The shown forms' words
- **WHEN** the journal, the loot window, the item card, the container popup and the trade window are committed as top-level forms
- **THEN** their style and attribute words are (0x3010, 0x38063), (0x3010, 0x30063), (0x3090, 0x310f1), (0x3010, 0x830063) and (0xb810, 0x830063)

#### Scenario: The loot window's panels
- **WHEN** the loot window at unit 4 is painted
- **THEN** its shadow and backdrop come first, its two item panels draw only their frames, and the window's frame comes last

#### Scenario: Manage Company's tables
- **WHEN** Manage Company is painted
- **THEN** its roster and report tables are held with style 0x88 and attributes 0x30c32, draw no backdrop and are framed on their edges

#### Scenario: The Controls page
- **WHEN** Settings is painted on its Controls page
- **THEN** the page is held with style 0x88 and attributes 0x302a2 and draws its scroll bar, and the Settings page draws no frame

### Requirement: Line skin form draw
A game form SHALL draw a drop shadow, a solid backdrop in the form group's fourth shade, its centred title, and then its frame from the skin atlas: a top-level form's ornamented corners, close mark and edge bars split around the title over a split strip; a free panel's two-unit frame two units in; a held panel's frame on its edges with joints at the corners flush with its owner's client and curled corners elsewhere. Every sample SHALL take its record's own eight texture coordinates.

#### Scenario: The pack's frame
- **WHEN** the pack, held at the bottom of the inventory's client at unit 4, is drawn
- **THEN** its top corners are curled pieces reaching two units outside and its bottom corners are joints

#### Scenario: The title's gap
- **WHEN** a 244-wide window at unit 4 with a 60-pixel title is drawn
- **THEN** its top edge bars run from 26 to 82 and from 162 to 218, with caps at the gap

#### Scenario: An edge bar's sample
- **WHEN** an edge bar takes the sample whose first corners are (0.3, 0.1) and (0.3, 0.2)
- **THEN** its other corners sample (0.5, 0.2) and (0.5, 0.1)

## ADDED Requirements

### Requirement: Panel scroll bar
A panel with the scroll bar attribute SHALL draw its two arrows past its client's right edge, its thumb at the line skin's scroll refresh's place, its right bar beside the bar, a rail outside a right edge not flush with its owner's client, and the scroll bar's own corner pieces on the right.

#### Scenario: A held page's scroll bar
- **WHEN** a 400 × 416 page held at its owner's client's top-left, client right 380, top 2, bottom 414, thumb at 40, is drawn at unit 4
- **THEN** its arrows are 8 pixels square at x 385, y 10 and y 398, its thumb 16 pixels square at (381, 36), and its right bar runs at x 384 to 392

### Requirement: Item card records
The item card SHALL be built from its builders' template records: a stats card's rows as a caption ending a unit left of x 156 and its value (rating dots, or the weight slider) at x 152, one anchor row apart, and the rows a stats card adds (Sockets, Rank, Points, Price) at x 120 from an absolute row. The records SHALL be laid out at unit 4 and rescaled with the card.

#### Scenario: A weapon card's rows
- **WHEN** a weapon card with two striking rows and a price is laid out at unit 4
- **THEN** its captions read Weight, Impact, Balance, Slash, Pierce and Price, the Weight caption's record is at (168, 22), the slider at (164, 20) and the card is 268 × 232

### Requirement: Item card text and size
The item's name SHALL be the card's centred title, in the builder's tint when one is set, its colour bytes read red first. The description SHALL be drawn in the small font in a box as high as its lines and a unit; a plain card's box SHALL start five units under its icon area, and a plain card with more than 200 bytes of description SHALL be 100 units wide. The card SHALL be two units and two borders taller than its body and SHALL be placed over the pointer it was asked at.

#### Scenario: A plain card's text
- **WHEN** a plain card with a one-line description and a 120-pixel icon is laid out at unit 4
- **THEN** its text box is 236 wide at (16, 152) and the card is 196 high

#### Scenario: The tint
- **WHEN** the tint word is 0xc060d4ff and the card is half faded in
- **THEN** the title's ink is (255, 212, 96) at alpha 96
