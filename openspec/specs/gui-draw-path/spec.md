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

### Requirement: Form base input
The game's windows SHALL take the pointer through the form base class's rules over the line skin's frame codes: a press on the title band drags a window, a release over its close mark closes it, and a pressed window comes to the front by the layer rule.

#### Scenario: Frame codes
- **WHEN** the pointer is off a form's client
- **THEN** the close corner and the title band four units down, and right of the client the upper arrow, the scroll rail and the lower arrow, answer it

#### Scenario: Moving the inventory
- **WHEN** the pointer presses the inventory's title band and moves 400 pixels right and 50 down before letting go
- **THEN** the inventory, its view, pack and records are 400 pixels right and 50 down and stay there until it closes

#### Scenario: Closing by the close mark
- **WHEN** the left button is pressed and released over a window's close mark
- **THEN** the window closes

#### Scenario: Raising
- **WHEN** a press on its title holds a window that is under another
- **THEN** that window draws above the other

### Requirement: Scroll bar input
A form with the vertical scroll bar SHALL scroll by its arrows, its rail and thumb, and the wheel, which goes to the focused form and moves it by whole notches of its step.

#### Scenario: Thumb
- **WHEN** the rail is pressed off the thumb and dragged
- **THEN** the thumb centres on the pointer and follows it, a snapping form keeping its fraction on its steps

#### Scenario: Held arrow
- **WHEN** the manual page's lower arrow is held
- **THEN** the page scrolls a line at the press, the next line about 700 ms later, then a line each tick until its end

#### Scenario: Wheel
- **WHEN** the wheel turns three notches down over the manual's page
- **THEN** the page's fraction moves three of its steps toward the end

### Requirement: Frame hover inks
The line skin SHALL draw the frame mark under the pointer in 255, 204, 153 at the GUI's alpha: the scroll bar's upper arrow, lower arrow or thumb in place of their idle ink, and the close mark drawn again after the frame.

#### Scenario: Hovered close mark
- **WHEN** the pointer rests on the inventory's close mark
- **THEN** the close mark draws in the hover ink after the frame

### Requirement: Default tooltip
A hovered record that asks for its tooltip SHALL show the default tooltip once the pointer has rested 400 clock units on it: a button shows its record's tooltip string and asks only when it has one, any other record its text; the box is the text's width plus four units, fades in over 256 units, stays 2500 and fades out, and a press hides it.

#### Scenario: Captioned buttons
- **WHEN** the pointer rests on the inventory's Primary button, which has no tooltip string
- **THEN** no tooltip shows

### Requirement: Title client top
A titled top-level form whose attribute word's 0x800030 bits are exactly 0x20 SHALL start its client at two units plus the border metric 2 under the line skin; the inventory's character view and records SHALL sit that far below its top, its pack staying at the client's bottom.

#### Scenario: Inventory at unit 4
- **WHEN** the inventory is laid out at unit 4
- **THEN** its client starts 10 pixels down, its character view at (8, 10) and its pack at (8, 456)

### Requirement: Extended controls on table-built forms
A table-built form SHALL report a landed release of an extended-class record (text field, combo box, colour swatch, spinner, image) to its owner without changing its on bit or radio group. A combo box record SHALL show its 1-based selected item, and its report SHALL open the drop-down list under it, whose pick writes the row back.

#### Scenario: A combo box's list
- **WHEN** a combo box over three items is released on and the third row taken
- **THEN** the list opens under it and the box shows the third item

### Requirement: The colour picker form
The colour picker form SHALL be built from its 17 records (0x192 × 0x11a, "Colour"): the RGB and HSV trackbars send their channel's value, the spinners their value over 255, the image its point as hue and saturation, the old swatch and Reset revert, and Close finishes; each refresh SHALL be written back into the trackbars, spinners and swatches.

#### Scenario: A spinner step
- **WHEN** the picker opens on red and the green spinner's up button is pressed
- **THEN** the colour and the edited swatch become 255, 1, 0

### Requirement: Separator line frame
A separator record SHALL draw each of its line rectangles as the skin's four-ink line loop, one-pixel lines half a pixel in: the top in the hot-form group's first shade, the left side in its second, the bottom in its seventh and the right side in its fifth, or with style 0x40 its eighth, sixth, first and second.

#### Scenario: A Settings rule
- **WHEN** the Settings form draws one of its rules
- **THEN** the rule is a batch of eight line vertices: two in each of the four inks, top, left, bottom, right

### Requirement: Line-skinned label fronts
A label record on a line-skinned form SHALL draw no back for a frame style and SHALL pick its front as the label class does: style 0x20000000 the block draw, its line fitted to the record less the border on both sides and cut with an ellipsis (or, right-aligned, its tail kept), a record with its own colour the caller's-ink draw, else the plain draw.

#### Scenario: A key cell
- **WHEN** the Controls page draws a key cell (style 0x20003380)
- **THEN** its name is drawn through the block draw in the label group's seventh shade

### Requirement: Hovered label and button inks
A hovered record (0x10000 in its 0xf0000 bits) SHALL draw its caption through the skin's glowing text when it is a plain label or a line-skinned button (a pixel lower while lit), and in the highlight group's eighth shade at the GUI's alpha when it is a block label.

#### Scenario: The pointer on Apply
- **WHEN** the pointer rests on the Settings form's Apply button
- **THEN** its caption draws in the label group's eighth shade and four copies a pixel off in the highlight group's first shade under the additive blend

### Requirement: Line skin combo box back
A combo box on a line-skinned form SHALL draw its back as the line skin's rail: a track along the box's bottom from two units outside its left edge to two units past its right, its caps four units wide, and a thumb two units down from one to three units short of its right edge, in the disabled or idle control ink, the thumb in the highlight group's eighth shade at the GUI's alpha while hovered.

#### Scenario: The loadout's fighter list
- **WHEN** the arena loadout form draws its fighter list at unit 4
- **THEN** the rail's thumb runs from four to twelve pixels short of the list's right edge, eight pixels below its top

### Requirement: Line skin text field back
A text field on a line-skinned form SHALL draw no frame of its own; with a frame bit of its style (0xf0) it SHALL draw the line skin's capped span along its bottom less two pixels, from one pixel in on each side, in the idle control ink.

#### Scenario: A framed field
- **WHEN** a framed text field at (10, 20), 100 by 24, draws at unit 4
- **THEN** its left cap starts at (3, 34) and its middle bar at (27, 42)

### Requirement: Line skin image button
An image button on a line-skinned form SHALL draw no frame and SHALL draw its icon through the line skin's centred icon: centred in the record, in the label group's fourth shade while disabled, its seventh with the alpha raised by 0x40 while lit and its sixth otherwise for a style with bit 0x80000000, outlined while hovered.

#### Scenario: The Flip weapon button
- **WHEN** the item card draws its Flip weapon button lit
- **THEN** only the grip icon draws, in the label group's seventh shade with its alpha raised by 0x40

### Requirement: Rounded label boxes through the skin
The domain wheel's label box and the power tree's shaded boxes SHALL be the skin's rounded fills: the wheel's grown by twice four units at `0.6 · fade`, the tree's by its style times four units at its alpha byte.

#### Scenario: A wheel label
- **WHEN** the wheel's hovered domain shows its label at fade 1
- **THEN** one rounded batch draws in black at alpha byte 153

### Requirement: Dim link fade per row
A dim tree link SHALL take one alpha byte per spline row, `alpha · (2·row / segments) · 0.5` rounded to even, on both of the row's vertices, `segments` being the count the link's spline began with.

#### Scenario: The second row of a three-segment link
- **WHEN** a dim link of alpha 200 is built over three segments
- **THEN** both vertices of its second row take alpha 67

### Requirement: Wide line forms
A line-skinned form with the attribute bit 0x40_0000 SHALL draw, for the corner style, the wide window frame (on a held form its own ink and a top bar once more unless it sits within a unit of a titled owner's top; side bars four units from the corners, two on a held form; corner arms two units outside and six in), and for the panel style twelve quads straddling its edges a unit either side, the bars from two units in.

#### Scenario: A combo list
- **WHEN** a line-skinned combo opens its drop-down list at unit 4
- **THEN** the list's window frame's top-left corner arm starts eight pixels above and left of the list

### Requirement: Popups take the line skin
The drop-down list and the palette swatch popup SHALL draw through the line skin's form draw with their own words: the shadow, the backdrop, their content, then the wide window frame. A list opened from a line-skinned combo SHALL lose its panel style bit and rise by a unit less one.

#### Scenario: The merchant's Look for list
- **WHEN** the merchant's Look for combo opens its list at unit 4
- **THEN** the list's style word is 0x18 and it starts three pixels above the combo's bottom edge

### Requirement: Draw skin frame marks
A form that keeps the draw skin's own instance SHALL keep the frame mark under the pointer, the mark a press went down on and the button, from the draw skin's hit test on its default background's mark rectangles; the close mark SHALL draw its hot box under the pointer, its pressed box while held, and a release still over it SHALL close the form.

#### Scenario: The picker's close mark
- **WHEN** the pointer presses and releases on the colour picker's close mark
- **THEN** the picker closes

### Requirement: Default background mark icons
The default background's mark and arrow icons SHALL draw from the small glyph atlas, an active mark through the atlas's outlined draw at scale 1 in the skin's icon inks and any other plainly in the label group's seventh shade.

#### Scenario: An idle close mark
- **WHEN** the colour picker draws with the pointer away from its close mark
- **THEN** the close glyph draws in the label group's seventh shade

### Requirement: Wide title band
The line skin's hit test in the running game SHALL take the six-unit title band.

#### Scenario: The close corner six units deep
- **WHEN** a titled, closable window at unit 4 is hit 22 pixels down and two pixels from its right edge, above its client
- **THEN** the hit answers the close mark (-4)

### Requirement: Swatch backs
A colour swatch SHALL draw its back through its form's skin's control fill: on the line skin the shade (a cap, a bar and a cap a unit either side of half a unit above the top and below the bottom, in the on ink or the highlight ink under the pointer) and the colour a unit in; on the draw skin's own instance the style frames and the colour inset by the margin; both at the GUI's alpha.

#### Scenario: An emblem swatch
- **WHEN** the emblem form draws its first swatch at unit 4
- **THEN** its colour fills the swatch four pixels in from each edge

### Requirement: Table header line
A line-skinned table view SHALL draw one span bar along its header's foot, two units thick at the row height less half a unit and one, across its client at its scroll, its caps the skin's first side length long, in the idle control ink.

#### Scenario: Manage Company
- **WHEN** Manage Company draws its two tables
- **THEN** each draws one header bar two units thick

### Requirement: Skill tooltip boxes
The skill screen's tooltip boxes SHALL draw through the skin's rounded fill at radius 2.

#### Scenario: Unchanged pixels
- **WHEN** a tooltip box is drawn on the skill screen's canvas
- **THEN** its pixels equal those of the screen's former quad

### Requirement: Text box line count
The text box's line count SHALL be one for a text under two bytes and SHALL add a line when the byte before the last is a carriage return.

#### Scenario: Empty text
- **WHEN** the text is empty
- **THEN** it counts one line

### Requirement: Forget dialog lines centred
The skill screen's forget dialog SHALL draw its title and question centred in its text width and clipped after the glyph that passes the width's end.

#### Scenario: A narrow title
- **WHEN** the title is narrower than the dialog's text width
- **THEN** it starts at the rounded centre offset

### Requirement: GUI reselect on mode switch
A game mode switch SHALL force the game GUI's reselect under the pointer.

#### Scenario: Start of play
- **WHEN** the Character mode starts
- **THEN** the GUI's active control becomes the reselect code and the pointer is read again
