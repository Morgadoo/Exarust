# gui-draw-path Specification

## Purpose
How the interface's skin batches reach the screen: the blend word, the skin programs, the pixel rules of the CPU rasteriser, the texture rows, the GUI faces and the font fallback, and which screens are drawn through it.

## Requirements

### Requirement: Blend word
The draw path SHALL read a blend word's high-byte low nibble as the source factor and its low nibble as the destination factor from the original's ten-entry table, SHALL draw with blending off when the word under the mask 0x3f0f is 0x100, and SHALL draw a word of 0 as 0x203.

#### Scenario: Steady state
- **WHEN** a primitive is drawn with word 0x203
- **THEN** its source factor is source alpha and its destination factor is one minus source alpha

#### Scenario: Blending off
- **WHEN** a primitive is drawn with word 0x100
- **THEN** it replaces the pixels under it

### Requirement: Skin programs
Solid primitives SHALL draw the vertex ink; textured primitives SHALL mix the sampled texel by `TexCol` and multiply by the ink; rounded primitives SHALL cut the alpha with the `Scale` uniform; mode 0x10 SHALL draw what the skin's subclass answers, the flat program for the line skin.

#### Scenario: Line skin subclass
- **WHEN** a mode 0x10 primitive is drawn by the line skin
- **THEN** it draws its vertex ink and samples no texture

### Requirement: Shared quad edge
A pixel on the diagonal two triangles of one quad share SHALL be drawn once.

#### Scenario: Half-transparent quad
- **WHEN** a quad of alpha 128 is drawn
- **THEN** every covered pixel is blended exactly once

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

### Requirement: Item panel icons
The inventory screen SHALL draw each visible pack entry whose icon is in the panel's atlas through the `HltCol` program at its cell inside the pack panel's client area, clipped to that area, and SHALL draw no icon layer for an empty pack.

#### Scenario: One entry
- **WHEN** the pack holds one entry with a cached icon at cell (10, 20) and the inventory opens
- **THEN** the frame holds one `HltCol` layer clipped to the pack's client area with its quad 10 pixels in from the clip's left

### Requirement: HUD compass
The HUD SHALL draw the compass while the player carries a usable item of kind 4, sort 11, as a square of half extent `12·unit` centred `14·unit` from the view's left and bottom, at an opacity of `round((1 − fade) · 192)` of 255, turned by the frame of the compass frame requirement.

#### Scenario: Shown with the item
- **WHEN** the player's pack holds an item whose usable section is kind 4, sort 11 and the HUD is active
- **THEN** the compass is drawn, and at fade 0.5 its opacity is 96 of 255

### Requirement: HUD compass frame
The compass frame's axis `d` SHALL be the camera's backward axis laid flat with x negated and normalised, its first axis `d × up`, and its origin the centre plus `−3·d`; the local corners `(−e,−e)`, `(e,−e)`, `(e,e)`, `(−e,e)` SHALL take texture coordinates `(1,1)`, `(0,1)`, `(0,0)`, `(1,0)`.

#### Scenario: Camera looking down −z
- **WHEN** the GUI unit is 4, the view is 900 pixels tall and the camera's backward axis is (0, 0, 1)
- **THEN** the corner with texture (1,1) is at about (104, 793) and the picture's top-left texel is painted near (9, 794)

#### Scenario: A quarter turn
- **WHEN** the backward axis is (1, 0, 0)
- **THEN** the corners lie about (107, 892), (107, 796), (11, 796), (11, 892) and the picture's top-left texel is painted at the square's top right

### Requirement: Form client spans
Setting a form's controls SHALL raise its border to the sum of the frame's stretches (style bits 0x30 and 0xc0) when that sum is larger, otherwise remember the first border and restore it on later calls; the client's right and bottom edges SHALL be the width and height less the new border and the skin's scroll inset for each scrollbar attribute, and the title SHALL be the skin's height plus 2 (attribute bits 0x20 without 0x800000) or the top stretch, or the new border without a title.

#### Scenario: Stretches above the border
- **WHEN** a 302 × 402 form with border 2, style 0xc0, both scrollbars and a title sets its controls with stretches 8 and 9, title 20 and inset 7
- **THEN** the border is 9, the right edge 286, the bottom 386 and the title 29

### Requirement: Counter segments
The counter control SHALL draw `segments + 1` backs `block − 1` wide and `block + 1` apart from one pixel in, where `block = round(width / (segments + 1) − 1)`, each with a shade two pixels in at 0.8, and fills from two pixels in that end at the first share at or below zero.

#### Scenario: Four segments at half
- **WHEN** a counter of 4 segments at 0.5 is drawn at (10, 20), 100 × 16
- **THEN** the backs start at 11, 36, 61 and 86, 23 wide, and the fills are (12, 21, 21, 14) and (37, 21, 21, 14)

### Requirement: HUD power marker and skill-ready alert
The HUD SHALL draw the power marker at `(W/2 + 53·unit, H − 8·unit)` and the skill-ready alert at `(W/2 − 60·unit, H − 8·unit)` as the skill-icon atlas's first cell, 8 units square, each over a rounded black halo filling the square from 3 to 5 units into the icon grown by 8 units at alpha `round(0.6 · pulse · 255)`.

#### Scenario: Marker at half pulse
- **WHEN** the GUI unit is 4, the marker sits at (100, 200) and the pulse is 0.5
- **THEN** the halo's quad spans x 80 to 152 in black at alpha 76 and the icon is one 32-pixel quad from (100, 200) tinted (140, 160, 255, 128)

### Requirement: HUD status icon inks
The marker's icon SHALL be tinted (140, 160, 255) at alpha `CVTSS2SI(pulse · 255)`, and the alert's icon SHALL be drawn outlined at the pulse, its face in the skin's seventh label ink and its four one-pixel copies in the third highlight ink.

#### Scenario: Alert at full pulse
- **WHEN** the alert is drawn at (40, 60) with the pulse 1
- **THEN** the face quad starts at x 40 in the seventh label ink and the first outline copy at x 39 in the third highlight ink

### Requirement: HUD status pulse clock
The marker and the alert SHALL share one pulse clock: the alert's first frame stamps it with the clock, the marker reads it as it is (zero before any stamp), a frame without a player draws neither and keeps the stamp, and only the HUD switched off clears it.

#### Scenario: The alert stamps the clock
- **WHEN** the marker draws alone at 1500 ms and the alert first shows at 3000 ms
- **THEN** the marker's pulse is 0.5 at 1500 ms and 0 at 3000 ms, the alert's pulse at 3000 ms is 1, and at 4000 ms it is 0

### Requirement: Compass interface fade
The compass's fade SHALL be the open power tree's window fade, and 0 while the tree is shut.

#### Scenario: Tree open at a quarter
- **WHEN** the power tree is open with a window fade of 0.25
- **THEN** the compass's fade is 0.25, an opacity of `round(0.75 · 192)` = 144

### Requirement: Form close cascade
Closing a form SHALL restore the controller's action set for style bit 0x20 even when already closing; a form not yet closing SHALL mark itself when it closes once; the task's active child SHALL run its callback, clear the task's active child and close the task; any other form SHALL clear the task's remembered child when it is this form, close the task's children it owns that are not closing, then its own children not closing, then hide, run its callback and redraw.

#### Scenario: Owned popup and child
- **WHEN** a window owning one popup in its task and having one child not closing is closed
- **THEN** the popup is closed and hidden first, then the child, then the window hides, runs its callback and redraws

### Requirement: Form maximise and restore steps
A maximise SHALL show the form, save its record, tell the window-state hook word 2 before the form changes, write the place, set the state's low bits to 2, resize, run the layout-changed virtual and redraw; a restore SHALL show the form and, with a record, write it back, resize, free it and refresh the control spans, then tell the hook word 0 with or without a record, clear the state's low bits, run the layout-changed virtual and redraw.

#### Scenario: Restore without a record
- **WHEN** a form without a maximise record is restored with a hook installed
- **THEN** it is shown, the hook is told 0, the state's low bits clear and the form redraws without moving

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
