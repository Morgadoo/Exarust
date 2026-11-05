# Spec Delta

## MODIFIED Requirements

### Requirement: Screens from the constructors' words
The inventory, the journal, the loot window, the container popup, the item card and the hub's windows SHALL paint through the game skin's form draw with the words their show commits from their constructors' words: each window as a top-level form, its character views and item panels as held children. The pack panel SHALL use attributes 0x130032 and style 0x80 as constructed. Control records SHALL be laid out by the two-pass layout inside the client.

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

### Requirement: Texture rows and font fallback
A texture coordinate SHALL run up the image, so a texture stored top-down SHALL be sampled from its bottom row, and a text quad's top edge SHALL name the cell's far row. The interface's five level faces SHALL be read from the installed executable's raw-data resources named `<face>_rfi`, without regard to case, after the packs. When a level's font texture is still not installed, the text SHALL keep the level font's glyph quad, advances and line height and SHALL sample the base font's atlas cells.

#### Scenario: Striped texture
- **WHEN** a quad's top edge is at v 0.75 and its bottom edge at v 0 over a texture of four rows
- **THEN** its top pixel shows the third row from the bottom and its bottom pixel shows the bottom row

#### Scenario: Missing level font
- **WHEN** the level font texture is missing and the base font texture is installed
- **THEN** the screen's glyph quads have the level font's size and sample steps of one base glyph

#### Scenario: Faces from the executable
- **WHEN** the installation is opened and `Rayform10R` is asked for
- **THEN** no pack holds it and the image comes from the executable's resource `RAYFORM10R_RFI`

#### Scenario: A malformed executable
- **WHEN** the executable's resource tree points outside its section or is cut short
- **THEN** the parser returns an error and no resource is read
