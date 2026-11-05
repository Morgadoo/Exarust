# Spec Delta

## MODIFIED Requirements

### Requirement: Screens from the constructors' words
The journal SHALL paint with the attribute and frame-style words its constructor writes. The inventory SHALL paint through the game skin's form draw with the words its show commits: the window as a top-level form, the character view and the pack as held children. The pack panel SHALL use attributes 0x130032 and style 0x80 as constructed. Control records SHALL be laid out by the two-pass layout inside the client.

#### Scenario: Inventory records
- **WHEN** the inventory at unit 4 is laid out
- **THEN** the Encumbrance label is at client (4, 386), Primary is 108 by 24 at (4, 416) and Secondary 108 by 24 at (116, 416)

#### Scenario: Journal records
- **WHEN** the journal is laid out
- **THEN** New is 104 by 24 at client (4, 20) and Erase is 104 by 24 at (120, 20)

#### Scenario: Title bit
- **WHEN** a frame is painted without the title bit
- **THEN** no title text is drawn

## ADDED Requirements

### Requirement: Committed form words
Before a form's first draw its words SHALL be committed: a form whose owner is not the task SHALL be held; with attribute 0x10000 a held form SHALL lose its backdrop and a top-level form SHALL gain the corner frame style and the title and shadow attributes.

#### Scenario: The inventory's forms
- **WHEN** the inventory and its character view and pack are committed
- **THEN** the window has a shadow, a backdrop, a title and the corner frame, and the view and the pack are held with no backdrop

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
